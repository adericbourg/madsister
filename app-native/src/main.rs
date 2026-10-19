use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::rc::Rc;

use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{Model, SharedString, VecModel};

use madsister::audio::{FileCheck, Player as AudioPlayer, Status, audio_path, check_file, relative_audio_path};
use madsister::editor::{DraftKind, Editor, MOD_LABEL, MenuAction, MetaField, Outcome};
use madsister::engine::{EngineEvent, Jobs};
use madsister::importer::{ForcedMeter, Importer, Next, Request, Source, recording_name};
use madsister::keymap::{count_flagged, has_confidence};
use madsister::model::playback::bar_at_time;
use madsister::keymap::{KeyInput, SHORTCUTS};
use madsister::model::commands::{BarRef, SlotRef, Step};
use madsister::model::song::{Meter, Notation, empty_song};
use madsister::model::transpose::Spelling;
use madsister::model::song::parse_song;
use madsister::settings::Settings;

mod view;

#[cfg(test)]
mod ui_tests;

slint::include_modules!();

/// What the window shows: the settings, and the open song's editor.
struct State {
    settings: Settings,
    editor: Option<Editor>,
    /// The open song's file; None for a new grid.
    path: Option<PathBuf>,
    audio: AudioFile,
    /// The song the sheet was last built from: rebuilding it recreates every slot, the inline input included.
    shown: Option<madsister::model::song::Song>,
    /// The panels' last field errors.
    errors: Errors,
    /// What the open confirmation dialog would do.
    pending: Option<Pending>,
    /// The song's audio, when it has one; or why it can't play.
    player: Result<Option<AudioPlayer>, String>,
    importer: Importer,
    jobs: Jobs,
    /// The running engine job's events, sent from its thread, read by the ticker.
    engine_events: (std::sync::mpsc::Sender<EngineEvent>, std::sync::mpsc::Receiver<EngineEvent>),
}

/// The checks of the song's audio file.
#[derive(Default)]
struct AudioFile {
    /// The audio reference last checked; None until the open song's is.
    checked: Option<Option<madsister::model::song::Audio>>,
    check: Option<FileCheck>,
    /// The audio file's sha256 being computed for the check.
    hashing: Option<Receiver<Option<String>>>,
    /// The file picked by "Locate audio…", and its sha256 being computed.
    locating: Option<(PathBuf, Receiver<Option<String>>)>,
}

#[derive(Default)]
struct Errors {
    title: String,
    key: String,
    tempo: String,
    section: String,
}

enum Pending {
    DeleteSection,
    /// The import would write over this song file.
    Overwrite(std::path::PathBuf),
}

/// Slint's key text as the keymap names it: special keys come as private-use characters.
fn key_name(text: &str) -> &str {
    use slint::platform::Key;
    let named = [
        (Key::LeftArrow, "ArrowLeft"),
        (Key::RightArrow, "ArrowRight"),
        (Key::UpArrow, "ArrowUp"),
        (Key::DownArrow, "ArrowDown"),
        (Key::Home, "Home"),
        (Key::End, "End"),
        (Key::Backspace, "Backspace"),
        (Key::Return, "Enter"),
        (Key::Tab, "Tab"),
        (Key::Escape, "Escape"),
        (Key::F2, "F2"),
        (Key::F8, "F8"),
    ];
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => named.iter().find(|(key, _)| char::from(*key) == c).map_or(text, |(_, name)| name),
        _ => text,
    }
}

fn key_input(event: &slint::language::KeyEvent) -> KeyInput<'_> {
    KeyInput {
        key: key_name(&event.text),
        ctrl: event.modifiers.control,
        meta: event.modifiers.meta,
        shift: event.modifiers.shift,
        alt: event.modifiers.alt,
    }
}

/// Shows the editor's state.
fn refresh(app: &AppWindow, state: &mut State) {
    check_audio(state);
    let Some(editor) = &state.editor else { return };
    if state.shown.as_ref() != Some(editor.song()) {
        app.set_chart(view::chart_view(editor.song(), &state.settings));
        state.shown = Some(editor.song().clone());
    }
    app.set_has_song(true);
    app.set_notice(editor.notice.as_str().into());
    app.set_is_help_open(editor.is_help_open);
    let grid = app.global::<Grid>();
    let SlotRef { section, bar, slot } = editor.cursor();
    grid.set_cursor_section(section as i32);
    grid.set_cursor_bar(bar as i32);
    grid.set_cursor_slot(slot as i32);
    grid.set_is_editable(editor.is_editable);
    let draft = editor.draft.as_ref();
    grid.set_is_drafting_chord(draft.is_some_and(|d| d.kind == DraftKind::Chord));
    grid.set_is_drafting_label(draft.is_some_and(|d| d.kind == DraftKind::Label));
    grid.set_draft_text(draft.map(|d| d.text.as_str()).unwrap_or_default().into());
    grid.set_draft_select_all(draft.is_some_and(|d| d.select_all));
    grid.set_draft_error(draft.and_then(|d| d.error.as_deref()).unwrap_or_default().into());
    let selection = editor.selection();
    grid.set_selection_section(selection.map_or(-1, |s| s.section as i32));
    grid.set_selection_from(selection.map_or(0, |s| s.from as i32));
    grid.set_selection_to(selection.map_or(0, |s| s.to as i32));
    app.set_panels(view::panels_view(editor));
    let player = app.global::<Player>();
    player.set_has_review(has_confidence(editor.song()));
    player.set_flagged(count_flagged(editor.song(), state.settings.low_confidence_threshold) as i32);
    let name = audio_file(state).and_then(|f| f.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_default();
    let notice = match state.audio.check {
        Some(FileCheck::Missing) => {
            format!("Audio file not found next to the song: {name}.\nThe song can't be played until you locate it.")
        }
        Some(FileCheck::Changed) => format!("Audio file has changed since the transcription: {name}"),
        None => String::new(),
    };
    player.set_audio_notice(notice.into());
    app.set_title_error(state.errors.title.as_str().into());
    app.set_key_error(state.errors.key.as_str().into());
    app.set_tempo_error(state.errors.tempo.as_str().into());
    app.set_section_error(state.errors.section.as_str().into());
    show_question(app, state);
}

/// The confirmation dialog, when a question is pending.
fn show_question(app: &AppWindow, state: &State) {
    let (question, action) = match &state.pending {
        Some(Pending::DeleteSection) => {
            let label = state.editor.as_ref().map(|e| e.song().sections[e.cursor().section].label.as_str()).unwrap_or_default();
            (format!("Delete the section \"{label}\" and its chords?"), "Delete")
        }
        Some(Pending::Overwrite(path)) => (format!("{} already exists. Overwrite it?", path.display()), "Overwrite"),
        None => (String::new(), ""),
    };
    app.set_confirm_text(question.into());
    app.set_confirm_action(action.into());
}

/// Where downloads and recordings go: the app's local data folder, as the Tauri app had it.
fn sources_dir() -> std::path::PathBuf {
    dirs::data_local_dir().unwrap_or_default().join("dev.dericbourg.madsister").join("sources")
}

// shortcut: no packaged engine yet (step 12), so every feature counts as installed, as in dev.
fn engine_has(_feature: &str) -> bool {
    true
}

/// Starts an engine job; its events go to the ticker.
fn start_job(state: &State, request: Request) -> Result<u32, String> {
    let mut argv = madsister::engine::engine_command(std::env::var("MADSISTER_ENGINE").ok(), None);
    if let Some(hint) = madsister::engine::missing_dev_engine(&argv) {
        return Err(hint);
    }
    let has_stdin = matches!(request, Request::Record { .. });
    let path = |p: &Path| p.display().to_string();
    match request {
        Request::Setup(_) => return Err("the engine isn't packaged".into()),
        Request::Transcribe { audio, out, meter, sections, mode } => {
            argv.extend(madsister::engine::transcribe_args(path(&audio), path(&out), meter, sections, mode));
        }
        Request::Fetch { url, out_dir } => argv.extend(["fetch".into(), url, "--out-dir".into(), path(&out_dir)]),
        Request::Record { out } => {
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("can't create {}: {e}", dir.display()))?;
            }
            argv.extend(["record".into(), "--out".into(), path(&out)]);
        }
    }
    let events = state.engine_events.0.clone();
    state
        .jobs
        .spawn(&argv, has_stdin, move |event| _ = events.send(event))
        .map_err(|e| format!("can't start the engine ({}): {e}", argv[0]))
}

/// Does what the importer asks for next.
fn follow(app: &AppWindow, state: &mut State, next: Next) {
    match next {
        Next::Nothing => {}
        Next::Start(request) => {
            let started = start_job(state, request);
            state.importer.started(started);
        }
        Next::ConfirmOverwrite(path) => state.pending = Some(Pending::Overwrite(path)),
        Next::Open(path) => {
            open(app, state, &path);
        }
    }
    show_import(app, state);
    show_question(app, state);
}

/// Shows the importer's state.
fn show_import(app: &AppWindow, state: &State) {
    let importer = &state.importer;
    let ui = app.global::<Import>();
    let file_name = importer.audio_path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
    ui.set_file_name(file_name.unwrap_or_default().into());
    ui.set_is_running(importer.is_running());
    ui.set_is_recording(importer.is_recording());
    ui.set_status(importer.status().into());
    ui.set_pct(importer.job.as_ref().map_or(0, |j| i32::from(j.pct)));
    ui.set_failure(importer.failure.as_ref().map(|f| f.message.as_str()).unwrap_or_default().into());
    ui.set_stderr(importer.failure.as_ref().map(|f| f.stderr.as_str()).unwrap_or_default().into());
}

/// Reads the engine's events that arrived since the last tick.
fn drain_engine_events(app: &AppWindow, state: &mut State) {
    while let Ok(event) = state.engine_events.1.try_recv() {
        let next = state.importer.event(event, engine_has, Path::exists);
        follow(app, state, next);
    }
}

/// Connects the start screen (F-IN-1..6).
fn wire_start(app: &AppWindow, state: &Rc<RefCell<State>>) {
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_create_grid(move |beats, unit, is_nashville| {
        let Some(app) = weak.upgrade() else { return };
        let meter = Meter { beats: beats as u32, unit: unit as u32, extra: Default::default() };
        let notation = is_nashville.then_some(Notation::Nashville);
        let mut state = st.borrow_mut();
        state.editor = Some(Editor::new(empty_song(meter, notation)));
        state.path = None;
        state.audio = AudioFile::default();
        state.shown = None;
        state.errors = Errors::default();
        refresh(&app, &mut state);
        tick(&app, &state);
        focus_grid_when_idle(&app, &state);
    });
    let import = app.global::<Import>();
    let (weak, st) = (app.as_weak(), state.clone());
    import.on_pick_file(move || {
        let (weak, st) = (weak.clone(), st.clone());
        let dialog = audio_dialog();
        let picking = slint::spawn_local(async move {
            let Some(file) = dialog.pick_file().await else { return };
            let Some(app) = weak.upgrade() else { return };
            let mut state = st.borrow_mut();
            state.importer.select_file(file.path());
            show_import(&app, &state);
        });
        if let Err(e) = picking {
            eprintln!("can't open the file dialog: {e}");
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    import.on_run(move || {
        let Some(app) = weak.upgrade() else { return };
        let ui = app.global::<Import>();
        let mut state = st.borrow_mut();
        let importer = &mut state.importer;
        importer.source = match ui.get_source() {
            1 => Source::Url,
            2 => Source::Record,
            _ => Source::File,
        };
        importer.url = ui.get_url().into();
        importer.meter = match ui.get_meter() {
            1 => Some(ForcedMeter::Four),
            2 => Some(ForcedMeter::Three),
            3 => Some(ForcedMeter::SixEight),
            _ => None,
        };
        importer.detect_sections = ui.get_detect_sections();
        importer.accurate = ui.get_accurate();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let next = importer.run(&sources_dir(), &recording_name(now), engine_has, Path::exists);
        follow(&app, &mut state, next);
    });
    let (weak, st) = (app.as_weak(), state.clone());
    import.on_cancel(move || {
        let state = st.borrow();
        if let Some(id) = state.importer.job_id() {
            state.jobs.cancel(id);
        }
        if let Some(app) = weak.upgrade() {
            show_import(&app, &state);
        }
    });
    let st = state.clone();
    import.on_stop(move || {
        let state = st.borrow();
        if let Some(id) = state.importer.job_id() {
            state.jobs.stop(id);
        }
    });
}

/// After an event from the grid: the keyboard goes back to it unless an input or a dialog holds it.
fn focus_grid_when_idle(app: &AppWindow, state: &State) {
    let is_idle = state.editor.as_ref().is_some_and(|e| e.draft.is_none() && !e.is_help_open) && state.pending.is_none();
    if is_idle {
        app.invoke_focus_grid();
    }
}

/// `with_editor` for the grid's own events, which give it the keyboard back.
fn with_grid<R: Default>(app: &slint::Weak<AppWindow>, state: &Rc<RefCell<State>>, f: impl FnOnce(&mut Editor, &Settings) -> R) -> R {
    let result = with_editor(app, state, f);
    if let Some(window) = app.upgrade() {
        focus_grid_when_idle(&window, &state.borrow());
    }
    result
}

/// The open song's audio file.
fn audio_file(state: &State) -> Option<PathBuf> {
    let audio = state.editor.as_ref()?.song().audio.as_ref()?;
    Some(audio_path(state.path.as_deref()?, &audio.path))
}

/// The file's sha256, computed on its own thread: the song shows first.
fn hash_in_background(path: PathBuf) -> Receiver<Option<String>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || _ = sender.send(madsister::audio::sha256(&path)));
    receiver
}

/// Checks the song's audio file when its reference changed (opened, located, undone); the player waits for the result.
fn check_audio(state: &mut State) {
    let audio = state.editor.as_ref().and_then(|e| e.song().audio.clone());
    if state.audio.checked.as_ref() == Some(&audio) {
        return;
    }
    state.player = Ok(None);
    state.audio.check = None;
    state.audio.hashing = audio_file(state).map(hash_in_background);
    state.audio.checked = Some(audio);
}

/// Takes the audio file hashes that are ready: the check's result opens the player, a located file becomes the song's audio.
fn poll_audio(app: &AppWindow, state: &mut State) {
    if let Some(Ok(sha256)) = state.audio.hashing.as_ref().map(Receiver::try_recv) {
        state.audio.hashing = None;
        let expected = state.editor.as_ref().and_then(|e| e.song().audio.as_ref()).map(|a| a.sha256.clone()).unwrap_or_default();
        state.audio.check = check_file(&expected, sha256.as_deref());
        // A missing file would only fail to load, repeating the notice.
        if state.audio.check != Some(FileCheck::Missing) {
            state.player = audio_file(state).map(|file| AudioPlayer::open(&file)).transpose();
        }
        refresh(app, state);
    }
    if let Some((file, Ok(sha256))) = state.audio.locating.as_ref().map(|(file, hashing)| (file.clone(), hashing.try_recv())) {
        state.audio.locating = None;
        let (Some(editor), Some(song_path)) = (state.editor.as_mut(), state.path.as_deref()) else { return };
        match sha256 {
            None => editor.notice = format!("Can't read {}", file.display()),
            Some(sha256) => {
                let mut song = editor.song().clone();
                let extra = song.audio.take().map(|a| a.extra).unwrap_or_default();
                song.audio = Some(madsister::model::song::Audio { path: relative_audio_path(song_path, &file), sha256, extra });
                editor.history.apply(song);
            }
        }
        refresh(app, state);
    }
}

/// The native dialog that picks an audio file.
fn audio_dialog() -> rfd::AsyncFileDialog {
    rfd::AsyncFileDialog::new().add_filter("Audio", &madsister::importer::AUDIO_EXTENSIONS)
}

fn minutes(sec: f64) -> String {
    format!("{:02}:{:02}", (sec / 60.0) as u32, (sec % 60.0) as u32)
}

/// Shows the player's state: the transport, and the bar being played (F-PB-1).
fn tick(app: &AppWindow, state: &State) {
    let Some(editor) = &state.editor else { return };
    let ui = app.global::<Player>();
    let (player, error) = match &state.player {
        Ok(player) => (player.as_ref(), String::new()),
        Err(e) => (None, e.clone()),
    };
    let status = player.map(AudioPlayer::status);
    let error = match &status {
        Some(Status::Failed(e)) => e.clone(),
        _ => error,
    };
    let (time, duration) = player.map_or((0.0, 0.0), |p| (p.time(), p.duration()));
    ui.set_has_audio(editor.song().audio.is_some() && state.audio.check != Some(FileCheck::Missing));
    ui.set_is_ready(status == Some(Status::Ready));
    ui.set_is_playing(player.is_some_and(AudioPlayer::is_playing));
    ui.set_time(time as f32);
    ui.set_duration(duration as f32);
    ui.set_time_text(format!("{}/{}", minutes(time), minutes(duration)).into());
    ui.set_error(error.into());
    let playing = player.and_then(|_| bar_at_time(editor.song(), time));
    let grid = app.global::<Grid>();
    grid.set_playing_section(playing.map_or(-1, |at| at.section as i32));
    grid.set_playing_bar(playing.map_or(0, |at| at.bar as i32));
}

/// Plays from `sec`, or says why it can't.
fn play_from(state: &mut State, sec: f64) {
    let started = matches!(&state.player, Ok(Some(player)) if player.seek(sec) && (player.is_playing() || player.toggle()));
    if !started && let Some(editor) = &mut state.editor {
        editor.notice = "No audio to play".into();
    }
}

/// Opens the song file at `path`; false when it can't be opened.
fn open(app: &AppWindow, state: &mut State, path: &Path) -> bool {
    match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|text| parse_song(&text)) {
        Ok(song) => {
            state.path = Some(path.to_path_buf());
            state.audio = AudioFile::default();
            // Opened files start in view mode, as in the Tauri app; a new grid starts in edit mode.
            let mut editor = Editor::new(song);
            editor.is_editable = false;
            state.editor = Some(editor);
            state.shown = None;
            state.errors = Errors::default();
            refresh(app, state);
            focus_grid_when_idle(app, state);
            true
        }
        // shortcut: errors go to stderr until the app shell (step 11) shows them.
        Err(e) => {
            eprintln!("can't open {}: {e}", path.display());
            false
        }
    }
}

/// Runs `f` on the open editor with the settings, then shows the result.
fn with_editor<R: Default>(app: &slint::Weak<AppWindow>, state: &Rc<RefCell<State>>, f: impl FnOnce(&mut Editor, &Settings) -> R) -> R {
    let Some(app) = app.upgrade() else { return R::default() };
    let mut state = state.borrow_mut();
    let state = &mut *state;
    let Some(editor) = &mut state.editor else { return R::default() };
    let result = f(editor, &state.settings);
    refresh(&app, state);
    result
}

/// Like `with_editor`, with the whole state (field errors, pending confirmation).
fn with_state(app: &slint::Weak<AppWindow>, state: &Rc<RefCell<State>>, f: impl FnOnce(&mut State)) {
    let Some(app) = app.upgrade() else { return };
    let mut state = state.borrow_mut();
    if state.editor.is_some() {
        f(&mut state);
        refresh(&app, &mut state);
    }
}

/// Connects the panels' callbacks to the editor (F-UI-4).
fn wire_panels(app: &AppWindow, state: &Rc<RefCell<State>>) {
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_meta_committed(move |field, text| {
        with_state(&weak, &st, |state| {
            let field = match field {
                MetaChoice::Title => MetaField::Title,
                MetaChoice::Artist => MetaField::Artist,
                MetaChoice::Key => MetaField::Key,
                MetaChoice::Tempo => MetaField::Tempo,
            };
            let editor = state.editor.as_mut().expect("with_state checked it");
            let error = editor.set_meta(field, &text, &state.settings).err().unwrap_or_default();
            match field {
                MetaField::Title => state.errors.title = error,
                MetaField::Key => state.errors.key = error,
                MetaField::Tempo => state.errors.tempo = error,
                MetaField::Artist => {}
            }
        });
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_notation_picked(move |is_nashville| {
        let notation = if is_nashville { Notation::Nashville } else { Notation::Chords };
        with_editor(&weak, &st, |editor, _| editor.set_notation(notation));
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_transpose(move |semitones, is_flat| {
        let spelling = if is_flat { Spelling::Flat } else { Spelling::Sharp };
        with_editor(&weak, &st, |editor, _| editor.transpose(semitones, spelling));
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_section_picked(move |index| with_editor(&weak, &st, |editor, _| editor.select_section(index as usize)));
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_section_renamed(move |text| {
        with_state(&weak, &st, |state| {
            let editor = state.editor.as_mut().expect("with_state checked it");
            state.errors.section = editor.rename_cursor_section(&text).err().unwrap_or_default();
        });
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_repeat_set(move |n| with_editor(&weak, &st, |editor, _| editor.set_cursor_repeat(n.max(1) as u32)));
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_section_moved(move |delta| {
        let step = if delta < 0 { Step::Back } else { Step::Forward };
        with_editor(&weak, &st, |editor, _| editor.move_cursor_section(step));
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_section_added(move || with_editor(&weak, &st, |editor, _| editor.add_section_after_cursor()));
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_section_deleted(move || {
        with_state(&weak, &st, |state| {
            let editor = state.editor.as_mut().expect("with_state checked it");
            if editor.cursor_section_has_chords() {
                state.pending = Some(Pending::DeleteSection);
            } else {
                editor.delete_cursor_section();
            }
        });
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_confirm_answered(move |is_yes| {
        let Some(app) = weak.upgrade() else { return };
        let mut state = st.borrow_mut();
        match state.pending.take() {
            Some(Pending::DeleteSection) if is_yes => {
                if let Some(editor) = state.editor.as_mut() {
                    editor.delete_cursor_section();
                }
                refresh(&app, &mut state);
            }
            Some(Pending::Overwrite(_)) if is_yes => {
                let next = state.importer.transcribe_confirmed(engine_has);
                follow(&app, &mut state, next);
            }
            _ => {}
        }
        show_question(&app, &state);
        focus_grid_when_idle(&app, &state);
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_bar_beats_set(move |n| with_editor(&weak, &st, |editor, _| editor.set_cursor_bar_beats(n.max(1) as u32)));
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_shifted(move |is_later, from_cursor| {
        let step = if is_later { Step::Forward } else { Step::Back };
        with_editor(&weak, &st, |editor, _| editor.shift(step, from_cursor));
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_tempo_fixed(move |is_half, from_cursor| with_editor(&weak, &st, |editor, _| editor.fix_tempo(is_half, from_cursor)));

    let player = app.global::<Player>();
    let (weak, st) = (app.as_weak(), state.clone());
    player.on_toggle(move || {
        if let Ok(Some(player)) = &st.borrow().player {
            player.toggle();
        }
        if let Some(app) = weak.upgrade() {
            tick(&app, &st.borrow());
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    player.on_seek(move |sec| {
        if let Ok(Some(player)) = &st.borrow().player {
            player.seek(f64::from(sec));
        }
        if let Some(app) = weak.upgrade() {
            tick(&app, &st.borrow());
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    player.on_review(move |is_forward| {
        let step = if is_forward { Step::Forward } else { Step::Back };
        let start = with_grid(&weak, &st, |editor, settings| editor.review(step, settings));
        if let (Some(start), Ok(Some(player))) = (start, &st.borrow().player) {
            player.seek(start);
        }
    });

    let st = state.clone();
    player.on_locate_audio(move || {
        let (st, dialog) = (st.clone(), audio_dialog());
        let picking = slint::spawn_local(async move {
            let Some(file) = dialog.pick_file().await else { return };
            let file = file.path().to_path_buf();
            st.borrow_mut().audio.locating = Some((file.clone(), hash_in_background(file)));
        });
        if let Err(e) = picking {
            eprintln!("can't open the file dialog: {e}");
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    player.on_show_song_file(move || {
        with_state(&weak, &st, |state| {
            if let (Some(path), Some(editor)) = (&state.path, &mut state.editor)
                && let Err(e) = opener::reveal(path)
            {
                editor.notice = format!("Can't show {}: {e}", path.display());
            }
        });
    });

    let grid = app.global::<Grid>();
    let weak = app.as_weak();
    grid.on_meta_clicked(move |field| {
        if let Some(app) = weak.upgrade() {
            app.invoke_focus_meta(field);
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    grid.on_meter_clicked(move |section, bar| {
        let at = SlotRef { section: section as usize, bar: bar as usize, slot: 0 };
        with_editor(&weak, &st, |editor, _| editor.select_bar(at));
        if let Some(app) = weak.upgrade() {
            app.invoke_focus_bar_beats();
        }
    });
}

/// Connects the window's callbacks to the editor.
fn wire(app: &AppWindow, state: &Rc<RefCell<State>>) {
    wire_panels(app, state);
    wire_start(app, state);
    let weak = app.as_weak();
    let grid_state = state.clone();
    app.on_grid_key(move |event| {
        let e = key_input(&event);
        let outcome = with_grid(&weak, &grid_state, |editor, settings| {
            // The inline input appears at the next layout: keys typed before then still belong to it.
            if editor.draft.is_some() {
                if editor.draft_key(e, settings) {
                    return Outcome::Handled;
                }
                if let Some(draft) = &editor.draft
                    && !(e.ctrl || e.meta || e.alt)
                    && e.key.chars().count() == 1
                    && !e.key.chars().all(char::is_control)
                {
                    let text = format!("{}{}", draft.text, e.key);
                    editor.set_draft_text(&text);
                    return Outcome::Handled;
                }
            }
            if (e.ctrl || e.meta) && e.key.eq_ignore_ascii_case("e") {
                editor.is_editable = !editor.is_editable;
                editor.draft = None;
                editor.notice.clear();
                return Outcome::Handled;
            }
            editor.key(e, settings)
        });
        let mut state = grid_state.borrow_mut();
        match outcome {
            Outcome::Ignored => return false,
            Outcome::Handled => return true,
            Outcome::Play => {
                let is_toggled = matches!(&state.player, Ok(Some(player)) if player.toggle());
                if !is_toggled && let Some(editor) = &mut state.editor {
                    editor.notice = "No audio to play".into();
                }
            }
            Outcome::PlayFrom(sec) => play_from(&mut state, sec),
        }
        if let Some(app) = weak.upgrade() {
            refresh(&app, &mut state);
            tick(&app, &state);
        }
        true
    });
    let weak = app.as_weak();
    let help_state = state.clone();
    app.on_close_help(move || with_grid(&weak, &help_state, |editor, _| editor.is_help_open = false));

    let grid = app.global::<Grid>();
    let (weak, grid_state) = (app.as_weak(), state.clone());
    grid.on_slot_clicked(move |section, bar, slot| {
        let at = SlotRef { section: section as usize, bar: bar as usize, slot: slot as usize };
        // F-PB-2: the audio goes to the bar's start, playing or not.
        let start = with_grid(&weak, &grid_state, |editor, settings| {
            editor.click(at, settings);
            editor.bar_start(at)
        });
        if let (Some(start), Ok(Some(player))) = (start, &grid_state.borrow().player) {
            player.seek(start);
        }
    });
    let (weak, grid_state) = (app.as_weak(), state.clone());
    grid.on_menu(move |choice, section, bar, slot| {
        let at = SlotRef { section: section as usize, bar: bar as usize, slot: slot as usize };
        let bar = BarRef { section: at.section, bar: at.bar };
        let action = match choice {
            MenuChoice::Split => MenuAction::Split(at),
            MenuChoice::AddBarBefore => MenuAction::AddBarBefore(bar),
            MenuChoice::AddBarAfter => MenuAction::AddBarAfter(bar),
            MenuChoice::RemoveBar => MenuAction::RemoveBar(bar),
            MenuChoice::SplitSectionBefore => MenuAction::SplitSectionBefore(bar),
            MenuChoice::SplitSectionAfter => MenuAction::SplitSectionAfter(bar),
            MenuChoice::AddBarAt => MenuAction::AddBarAt(bar),
            MenuChoice::SplitSectionAt => MenuAction::SplitSectionAt(bar),
            MenuChoice::MergeWithPrevious => MenuAction::MergeWithPrevious(at.section),
        };
        with_grid(&weak, &grid_state, |editor, _| editor.run(action));
    });
    let (weak, grid_state) = (app.as_weak(), state.clone());
    grid.on_section_clicked(move |index| with_grid(&weak, &grid_state, |editor, _| editor.select_section(index as usize)));
    let (weak, grid_state) = (app.as_weak(), state.clone());
    // Only the editor's text: refreshing would reset the input's caret on every keystroke.
    grid.on_draft_edited(move |text| {
        if let Some(editor) = &mut grid_state.borrow_mut().editor {
            editor.set_draft_text(&text);
        }
        if let Some(app) = weak.upgrade() {
            app.global::<Grid>().set_draft_error(SharedString::new());
        }
    });
    let (weak, grid_state) = (app.as_weak(), state.clone());
    grid.on_draft_key(move |event| {
        let e = key_input(&event);
        with_grid(&weak, &grid_state, |editor, settings| editor.draft_key(e, settings))
    });
    let (weak, grid_state) = (app.as_weak(), state.clone());
    grid.on_draft_blurred(move || {
        // An input closed by its own commit or cancel has nothing left to save.
        if grid_state.borrow().editor.as_ref().is_none_or(|editor| editor.draft.is_none()) {
            return;
        }
        // Deferred: the input is being removed or another slot was clicked; both settle first. A new input that took the
        // focus meanwhile (the sheet was rebuilt under it) means the draft goes on.
        let (weak, grid_state) = (weak.clone(), grid_state.clone());
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if weak.upgrade().is_some_and(|app| !app.global::<Grid>().get_draft_focused()) {
                // Not with_grid: the focus went where the user clicked.
                with_editor(&weak, &grid_state, |editor, settings| editor.commit(settings, true));
            }
        });
    });
    let weak = app.as_weak();
    grid.on_reveal(move |y, height| {
        if let Some(app) = weak.upgrade() {
            app.invoke_reveal(y, height);
        }
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(("print", rest)) = args.split_first().map(|(command, rest)| (command.as_str(), rest)) {
        std::process::exit(madsister::print::run(rest, Settings::load()));
    }

    // The winit backend gives access to OS file drops (F-IN-1), which Slint's own drag & drop doesn't receive yet.
    let selector = slint::BackendSelector::new().backend_name("winit".into());
    // shortcut: winit has no Wayland drag & drop, so Linux runs under XWayland when it's there (no DISPLAY: native
    // Wayland without drops); go native Wayland once winit supports it.
    #[cfg(target_os = "linux")]
    let selector = if std::env::var_os("DISPLAY").is_some() {
        use slint::winit_030::winit::platform::x11::EventLoopBuilderExtX11;
        let mut builder = slint::winit_030::winit::event_loop::EventLoop::with_user_event();
        builder.with_x11();
        selector.with_winit_event_loop_builder(builder)
    } else {
        selector
    };
    selector.select()?;

    let app = AppWindow::new()?;
    let state = Rc::new(RefCell::new(State {
        settings: Settings::load(),
        editor: None,
        path: None,
        audio: AudioFile::default(),
        shown: None,
        errors: Errors::default(),
        pending: None,
        player: Ok(None),
        importer: Importer::default(),
        jobs: Jobs::default(),
        engine_events: std::sync::mpsc::channel(),
    }));
    {
        let settings = &state.borrow().settings;
        let font = app.global::<ChartFont>();
        font.set_family(settings.font.family().into());
        font.set_has_bold(settings.font.has_bold());
    }
    let shortcuts: Vec<Shortcut> = SHORTCUTS
        .iter()
        .map(|(keys, what)| Shortcut { keys: keys.replace("Mod", MOD_LABEL).into(), what: (*what).into() })
        .collect();
    app.set_shortcuts(Rc::new(VecModel::from(shortcuts)).into());
    if let Some(path) = args.first() {
        open(&app, &mut state.borrow_mut(), Path::new(path));
    }

    wire(&app, &state);

    let recent = Rc::new(VecModel::<SharedString>::default());
    app.set_recent(recent.clone().into());
    let weak = app.as_weak();
    let dropped_into = recent.clone();
    let drop_state = state.clone();
    app.window().on_winit_window_event(move |_, event| {
        let Some(app) = weak.upgrade() else { return EventResult::Propagate };
        let WindowEvent::DroppedFile(path) = event else { return EventResult::Propagate };
        let mut state = drop_state.borrow_mut();
        let is_song = path.to_string_lossy().ends_with(".json");
        // On the start screen, an audio file is for the importer (F-IN-1); a song file opens.
        if !is_song && state.editor.is_none() {
            if !state.importer.is_running() {
                state.importer.select_file(path);
                show_import(&app, &state);
            }
            return EventResult::Propagate;
        }
        if open(&app, &mut state, path) {
            let path = SharedString::from(path.display().to_string());
            if let Some(i) = dropped_into.iter().position(|p| p == path) {
                dropped_into.remove(i);
            }
            dropped_into.insert(0, path);
        }
        EventResult::Propagate
    });

    let (weak, recent_state) = (app.as_weak(), state.clone());
    app.on_open_recent(move |path| {
        if let Some(app) = weak.upgrade() {
            open(&app, &mut recent_state.borrow_mut(), Path::new(path.as_str()));
        }
    });
    // Deferred: macOS native menus break if their structure changes while an item's activation is being handled.
    app.on_clear_recent(move || {
        let recent = recent.clone();
        slint::Timer::single_shot(std::time::Duration::ZERO, move || recent.set_vec(Vec::new()));
    });

    let ticker = slint::Timer::default();
    let weak = app.as_weak();
    let ticking = state.clone();
    ticker.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(100), move || {
        if let Some(app) = weak.upgrade() {
            let mut state = ticking.borrow_mut();
            drain_engine_events(&app, &mut state);
            poll_audio(&app, &mut state);
            tick(&app, &state);
        }
    });

    let result = app.run();
    // The engine's jobs run in their own process groups: they'd outlive the app.
    state.borrow().jobs.cancel_all();
    result
}
