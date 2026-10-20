use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::rc::Rc;

use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{SharedString, VecModel};

use madsister::audio::{FileCheck, Player as AudioPlayer, Status, audio_path, check_file, located_sha256, relative_audio_path};
use madsister::editor::{DraftKind, Editor, MOD_LABEL, MenuAction, MetaField, Outcome};
use madsister::engine::{EngineEvent, Jobs, Packaged};
use madsister::export::chordpro::to_chord_pro;
use madsister::export::midi::to_midi;
use madsister::export::musicxml::to_music_xml;
use madsister::export::sheet::to_sheet;
use madsister::files::{parse_paths, push_recent, shortest_unique_paths, write_atomic};
use madsister::importer::{EngineSetup, ForcedMeter, Importer, Next, Request, Source, recording_name};
use madsister::keymap::{count_flagged, has_confidence};
use madsister::model::playback::bar_at_time;
use madsister::keymap::{KeyInput, SHORTCUTS};
use madsister::model::commands::{BarRef, SlotRef, Step};
use madsister::model::song::{Meter, Notation, Song, empty_song};
use madsister::model::transpose::Spelling;
use madsister::model::song::{parse_song, serialize_song};
use madsister::model::display::DisplayStyle;
use madsister::model::nashville::MinorConvention;
use madsister::settings::Settings;

mod view;

#[cfg(test)]
mod ui_tests;

slint::include_modules!();

/// What the window shows: the settings, and the open songs' tabs.
struct State {
    settings: Settings,
    /// Never empty: closing the last tab opens a start screen.
    tabs: Vec<Tab>,
    active: usize,
    /// The song the sheet was last built from: rebuilding it recreates every slot, the inline input included.
    shown: Option<madsister::model::song::Song>,
    /// What the open confirmation dialog would do.
    pending: Option<Pending>,
    jobs: Jobs,
    /// The packaged engine; None in a dev build and when `MADSISTER_ENGINE` overrides it.
    engine: Option<Packaged>,
    setup: EngineSetup,
    /// The setup job's events, read by the ticker.
    setup_events: (std::sync::mpsc::Sender<EngineEvent>, std::sync::mpsc::Receiver<EngineEvent>),
    last_tab_id: u32,
    /// Saves the songs that have a file 2 s after the last edit; restarted by every refresh while one has changes.
    autosave: slint::Timer,
    /// Where recent.json and session.json are kept; None in tests.
    config: Option<PathBuf>,
    /// The recent song files, most recent first.
    recent: Vec<String>,
    /// session.json as last written: the tabs that have a file, and the active one among them (F-UI-8).
    session: String,
}

impl State {
    fn new(settings: Settings) -> State {
        State {
            settings,
            tabs: vec![Tab::new(0)],
            active: 0,
            shown: None,
            pending: None,
            jobs: Jobs::default(),
            engine: None,
            setup: EngineSetup::default(),
            setup_events: std::sync::mpsc::channel(),
            last_tab_id: 0,
            autosave: slint::Timer::default(),
            config: None,
            recent: Vec::new(),
            session: String::new(),
        }
    }

    /// Adds a start screen tab at the end; returns its index.
    fn push_tab(&mut self) -> usize {
        self.last_tab_id += 1;
        self.tabs.push(Tab::new(self.last_tab_id));
        self.tabs.len() - 1
    }

    /// The tab a dialog was opened for, if it's still open: tabs can change while it is shown.
    fn tab_by_id(&mut self, id: u32) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }

    fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }
}

/// One open song (F-UI-7), or the start screen until a grid is created or imported.
struct Tab {
    id: u32,
    editor: Option<Editor>,
    /// The song as last opened or saved: the tab has unsaved changes when the editor's differs.
    saved: Option<madsister::model::song::Song>,
    /// The song's file; None for a new grid.
    path: Option<PathBuf>,
    audio: AudioFile,
    /// The panels' last field errors.
    errors: Errors,
    /// Why the last save failed.
    error: String,
    /// The song's audio, when it has one; or why it can't play. Each tab keeps its own, paused when another tab is shown.
    player: Result<Option<AudioPlayer>, String>,
    importer: Importer,
    /// The tab's engine job events, sent from its thread, read by the ticker while the tab is shown: a hidden tab's job
    /// keeps running, its events wait here.
    engine_events: (std::sync::mpsc::Sender<EngineEvent>, std::sync::mpsc::Receiver<EngineEvent>),
}

impl Tab {
    fn new(id: u32) -> Tab {
        Tab {
            id,
            editor: None,
            saved: None,
            path: None,
            audio: AudioFile::default(),
            errors: Errors::default(),
            error: String::new(),
            player: Ok(None),
            importer: Importer::default(),
            engine_events: std::sync::mpsc::channel(),
        }
    }

    fn is_dirty(&self) -> bool {
        self.editor.as_ref().is_some_and(|e| self.saved.as_ref() != Some(e.song()))
    }

    /// An untouched start screen, which an opened song takes over.
    fn is_idle(&self) -> bool {
        self.editor.is_none() && !self.importer.is_running()
    }
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
    /// Closing this tab drops its unsaved changes.
    CloseTab(usize),
    /// Closing the window drops unsaved changes.
    CloseWindow,
    /// The located audio file, and the sha256 to keep, isn't the one that was transcribed.
    OtherAudio(PathBuf, String),
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

/// Shows the tabs and the active tab's editor.
fn refresh(app: &AppWindow, state: &mut State) {
    check_audio(state);
    show_question(app, state);
    let tabs: Vec<TabView> = state
        .tabs
        .iter()
        .map(|tab| TabView {
            title: tab.editor.as_ref().map_or("New tab", |e| e.song().meta.title.as_str()).into(),
            is_dirty: tab.is_dirty(),
        })
        .collect();
    app.set_tabs(Rc::new(VecModel::from(tabs)).into());
    app.set_active_tab(state.active as i32);
    save_session(state);
    let tab = &state.tabs[state.active];
    app.set_error(tab.error.as_str().into());
    app.set_has_song(tab.editor.is_some());
    app.set_is_editing(tab.editor.as_ref().is_some_and(|e| e.is_editable));
    app.set_is_sidebar_shown(tab.editor.is_some() && !state.settings.is_sidebar_collapsed);
    // The exports write real notes, which a key-less Nashville song (chords relative to C) doesn't have: same lock as transposing.
    app.set_is_export_locked(tab.editor.as_ref().is_some_and(Editor::is_transpose_locked));
    // shortcut: a new song without a file isn't autosaved (no recovery file); add one if users lose work.
    if tab.is_dirty() && tab.path.is_some() {
        state.autosave.restart();
    }
    let Some(editor) = &tab.editor else { return };
    if state.shown.as_ref() != Some(editor.song()) {
        app.set_chart(view::chart_view(editor.song(), &state.settings));
        state.shown = Some(editor.song().clone());
    }
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
    let name = audio_file(tab).and_then(|f| f.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_default();
    let notice = match tab.audio.check {
        Some(FileCheck::Missing) => {
            format!("Audio file not found next to the song: {name}.\nThe song can't be played until you locate it.")
        }
        Some(FileCheck::Changed) => format!("Audio file has changed since the transcription: {name}"),
        None => String::new(),
    };
    player.set_audio_notice(notice.into());
    app.set_title_error(tab.errors.title.as_str().into());
    app.set_key_error(tab.errors.key.as_str().into());
    app.set_tempo_error(tab.errors.tempo.as_str().into());
    app.set_section_error(tab.errors.section.as_str().into());
}

/// The confirmation dialog, when a question is pending.
fn show_question(app: &AppWindow, state: &State) {
    let (question, action) = match &state.pending {
        Some(Pending::DeleteSection) => {
            let editor = state.tab().editor.as_ref();
            let label = editor.map(|e| e.song().sections[e.cursor().section].label.as_str()).unwrap_or_default();
            (format!("Delete the section \"{label}\" and its chords?"), "Delete")
        }
        Some(Pending::Overwrite(path)) => (format!("{} already exists. Overwrite it?", path.display()), "Overwrite"),
        Some(Pending::CloseTab(_) | Pending::CloseWindow) => ("Discard unsaved changes?".into(), "Discard"),
        Some(Pending::OtherAudio(file, _)) => {
            let name = file.file_name().unwrap_or_default().to_string_lossy();
            (format!("{name} isn't the file that was transcribed. Use it anyway?"), "Use it")
        }
        None => (String::new(), ""),
    };
    app.set_confirm_text(question.into());
    app.set_confirm_action(action.into());
}

/// Where downloads and recordings go: the app's local data folder, as the Tauri app had it.
fn sources_dir() -> std::path::PathBuf {
    dirs::data_local_dir().unwrap_or_default().join("dev.dericbourg.madsister").join("sources")
}

/// A feature ("Detect sections", "Accurate") can run: always in dev or with `MADSISTER_ENGINE`, else once installed.
fn engine_has(engine: Option<&Packaged>, feature: &str) -> bool {
    engine.is_none_or(|p| p.has(feature))
}

/// Starts the first-launch engine setup unless it runs; its events go to the ticker.
fn run_setup(state: &mut State) {
    let Some(packaged) = &state.engine else { return };
    if !state.setup.run() {
        return;
    }
    let argv = packaged.setup_command(&[]);
    let events = state.setup_events.0.clone();
    if let Err(e) = state.jobs.spawn(&argv, false, move |event| _ = events.send(event)) {
        let message = format!("can't start the engine setup ({}): {e}", argv[0]);
        state.setup.event(EngineEvent::Error { message, stderr: String::new() });
    }
}

/// Starts an engine job; its events go to the ticker.
fn start_job(state: &State, request: Request) -> Result<u32, String> {
    // A sync is exact: two installs of the same env would each remove what the other adds.
    let is_other_installing = state.tabs.iter().enumerate().any(|(i, tab)| i != state.active && tab.importer.is_installing());
    if matches!(request, Request::Setup(_)) && is_other_installing {
        return Err("Another install is running: wait for it to finish.".into());
    }
    let mut argv = madsister::engine::engine_command(std::env::var("MADSISTER_ENGINE").ok(), state.engine.as_ref());
    if let Some(hint) = madsister::engine::missing_dev_engine(&argv) {
        return Err(hint);
    }
    let has_stdin = matches!(request, Request::Record { .. });
    let path = |p: &Path| p.display().to_string();
    match request {
        Request::Setup(features) => {
            argv = state.engine.as_ref().ok_or("the engine isn't packaged")?.setup_command(&features);
        }
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
    let events = state.tab().engine_events.0.clone();
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
            state.tab_mut().importer.started(started);
        }
        Next::ConfirmOverwrite(path) => state.pending = Some(Pending::Overwrite(path)),
        Next::Open(path) => match read_song(&path) {
            Ok(song) => {
                // A tab still on the overwritten file would autosave its old song over the transcription: it becomes an
                // unsaved copy.
                for tab in state.tabs.iter_mut().filter(|tab| tab.path.as_deref() == Some(path.as_path())) {
                    tab.path = None;
                    tab.saved = None;
                }
                load(app, state, viewer(song), Some(path.clone()));
                remember(app, state, &path);
            }
            Err(e) => {
                state.tab_mut().error = format!("Can't open {}: {e}", path.display());
                refresh(app, state);
            }
        },
    }
    show_import(app, state);
    show_question(app, state);
}

/// Shows the importer's state.
fn show_import(app: &AppWindow, state: &State) {
    let importer = &state.tab().importer;
    let ui = app.global::<Import>();
    let file_name = importer.audio_path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
    ui.set_file_name(file_name.unwrap_or_default().into());
    ui.set_is_running(importer.is_running());
    ui.set_is_recording(importer.is_recording());
    ui.set_status(importer.status().into());
    ui.set_pct(importer.job.as_ref().map_or(0, |j| i32::from(j.pct)));
    ui.set_failure(importer.failure.as_ref().map(|f| f.message.as_str()).unwrap_or_default().into());
    ui.set_stderr(importer.failure.as_ref().map(|f| f.stderr.as_str()).unwrap_or_default().into());
    ui.set_sections_installed(engine_has(state.engine.as_ref(), "sections"));
    ui.set_accurate_installed(engine_has(state.engine.as_ref(), "separate"));
    let setup = &state.setup;
    ui.set_needs_setup(setup.is_needed);
    ui.set_setup_status(setup.status().into());
    ui.set_is_setting_up(setup.progress.is_some());
    ui.set_setup_pct(match &setup.progress {
        Some((stage, _)) if stage == "install" => -1,
        Some((_, pct)) => i32::from(*pct),
        None => 0,
    });
    ui.set_setup_failure(setup.failure.as_ref().map(|f| f.message.as_str()).unwrap_or_default().into());
    ui.set_setup_stderr(setup.failure.as_ref().map(|f| f.stderr.as_str()).unwrap_or_default().into());
}

/// Reads the engine's events that arrived since the last tick.
fn drain_engine_events(app: &AppWindow, state: &mut State) {
    while let Ok(event) = state.tab().engine_events.1.try_recv() {
        let tab = &mut state.tabs[state.active];
        let next = tab.importer.event(event, |f| engine_has(state.engine.as_ref(), f), Path::exists);
        follow(app, state, next);
    }
    let mut has_setup_events = false;
    while let Ok(event) = state.setup_events.1.try_recv() {
        state.setup.event(event);
        has_setup_events = true;
    }
    if has_setup_events {
        show_import(app, state);
    }
}

/// Connects the start screen (F-IN-1..6).
fn wire_start(app: &AppWindow, state: &Rc<RefCell<State>>) {
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_create_grid(move |beats, unit, is_nashville| {
        let Some(app) = weak.upgrade() else { return };
        let meter = Meter { beats: beats as u32, unit: unit as u32, extra: Default::default() };
        let notation = is_nashville.then_some(Notation::Nashville);
        load(&app, &mut st.borrow_mut(), Editor::new(empty_song(meter, notation)), None);
    });
    let import = app.global::<Import>();
    let (weak, st) = (app.as_weak(), state.clone());
    import.on_pick_file(move || {
        let (weak, st) = (weak.clone(), st.clone());
        let (dialog, id) = (audio_dialog(), st.borrow().tab().id);
        let picking = slint::spawn_local(async move {
            let Some(file) = dialog.pick_file().await else { return };
            let Some(app) = weak.upgrade() else { return };
            let mut state = st.borrow_mut();
            let Some(tab) = state.tab_by_id(id) else { return };
            tab.importer.select_file(file.path());
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
        let state = &mut *state; // borrows the tab and the engine apart
        let importer = &mut state.tabs[state.active].importer;
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
        let has = |f: &str| engine_has(state.engine.as_ref(), f);
        let next = importer.run(&sources_dir(), &recording_name(now), has, Path::exists);
        follow(&app, state, next);
    });
    let (weak, st) = (app.as_weak(), state.clone());
    import.on_cancel(move || {
        let state = st.borrow();
        if let Some(id) = state.tab().importer.job_id() {
            state.jobs.cancel(id);
        }
        if let Some(app) = weak.upgrade() {
            show_import(&app, &state);
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    import.on_retry_setup(move || {
        let Some(app) = weak.upgrade() else { return };
        let mut state = st.borrow_mut();
        run_setup(&mut state);
        show_import(&app, &state);
    });
    let st = state.clone();
    import.on_stop(move || {
        let state = st.borrow();
        if let Some(id) = state.tab().importer.job_id() {
            state.jobs.stop(id);
        }
    });
}

/// A question or a dialog is open over the window: the menu's shortcuts, which still fire, wait.
fn is_asking(app: &AppWindow, state: &State) -> bool {
    state.pending.is_some() || app.get_dialog() != Dialog::None
}

/// After an event from the grid: the keyboard goes back to it unless an input or a dialog holds it.
fn focus_grid_when_idle(app: &AppWindow, state: &State) {
    let is_idle = state.tab().editor.as_ref().is_some_and(|e| e.draft.is_none() && !e.is_help_open) && !is_asking(app, state);
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

/// The tab's audio file.
fn audio_file(tab: &Tab) -> Option<PathBuf> {
    let audio = tab.editor.as_ref()?.song().audio.as_ref()?;
    Some(audio_path(tab.path.as_deref()?, &audio.path))
}

/// The file's sha256, computed on its own thread: the song shows first.
fn hash_in_background(path: PathBuf) -> Receiver<Option<String>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || _ = sender.send(madsister::audio::sha256(&path)));
    receiver
}

/// Checks the song's audio file when its reference changed (opened, located, undone); the player waits for the result.
fn check_audio(state: &mut State) {
    let tab = state.tab_mut();
    let audio = tab.editor.as_ref().and_then(|e| e.song().audio.clone());
    if tab.audio.checked.as_ref() == Some(&audio) {
        return;
    }
    tab.player = Ok(None);
    tab.audio.check = None;
    tab.audio.hashing = audio_file(tab).map(hash_in_background);
    tab.audio.checked = Some(audio);
}

/// Takes the audio file hashes that are ready: the check's result opens the player, a located file becomes the song's audio.
fn poll_audio(app: &AppWindow, state: &mut State) {
    let tab = state.tab_mut();
    if let Some(Ok(sha256)) = tab.audio.hashing.as_ref().map(Receiver::try_recv) {
        tab.audio.hashing = None;
        let expected = tab.editor.as_ref().and_then(|e| e.song().audio.as_ref()).map(|a| a.sha256.clone()).unwrap_or_default();
        tab.audio.check = check_file(&expected, sha256.as_deref());
        // A missing file would only fail to load, repeating the notice.
        if tab.audio.check != Some(FileCheck::Missing) {
            tab.player = audio_file(tab).map(|file| AudioPlayer::open(&file)).transpose();
        }
        refresh(app, state);
    }
    let asking = state.pending.is_some();
    let tab = state.tab_mut();
    // An open question waits to be answered before the located file can ask its own.
    if !asking && let Some((file, Ok(sha256))) = tab.audio.locating.as_ref().map(|(file, hashing)| (file.clone(), hashing.try_recv())) {
        tab.audio.locating = None;
        let Some(editor) = tab.editor.as_mut() else { return };
        match sha256 {
            None => editor.notice = format!("Can't read {}", file.display()),
            Some(sha256) => {
                let recorded = editor.song().audio.as_ref().map(|a| a.sha256.as_str());
                match located_sha256(recorded, &sha256) {
                    (sha256, true) => state.pending = Some(Pending::OtherAudio(file, sha256)),
                    (sha256, false) => link_audio(tab, &file, sha256),
                }
            }
        }
        refresh(app, state);
    }
}

/// Makes the file the song's audio, relative to the song file, as an undoable edit keeping the audio's unknown fields.
fn link_audio(tab: &mut Tab, file: &Path, sha256: String) {
    let (Some(editor), Some(song_path)) = (tab.editor.as_mut(), tab.path.as_deref()) else { return };
    let mut song = editor.song().clone();
    let extra = song.audio.take().map(|a| a.extra).unwrap_or_default();
    song.audio = Some(madsister::model::song::Audio { path: relative_audio_path(song_path, file), sha256, extra });
    editor.history.apply(song);
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
    let tab = state.tab();
    let Some(editor) = &tab.editor else { return };
    let ui = app.global::<Player>();
    let (player, error) = match &tab.player {
        Ok(player) => (player.as_ref(), String::new()),
        Err(e) => (None, e.clone()),
    };
    let status = player.map(AudioPlayer::status);
    let error = match &status {
        Some(Status::Failed(e)) => e.clone(),
        _ => error,
    };
    let (time, duration) = player.map_or((0.0, 0.0), |p| (p.time(), p.duration()));
    ui.set_has_audio(editor.song().audio.is_some() && tab.audio.check != Some(FileCheck::Missing));
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
fn play_from(tab: &mut Tab, sec: f64) {
    let started = matches!(&tab.player, Ok(Some(player)) if player.seek(sec) && (player.is_playing() || player.toggle()));
    if !started && let Some(editor) = &mut tab.editor {
        editor.notice = "No audio to play".into();
    }
}

fn read_song(path: &Path) -> Result<madsister::model::song::Song, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|text| parse_song(&text))
}

/// An editor for an opened song: opened files start in view mode, as in the Tauri app; a new grid starts in edit mode.
fn viewer(song: madsister::model::song::Song) -> Editor {
    let mut editor = Editor::new(song);
    editor.is_editable = false;
    editor
}

/// Puts the song in the active tab, as saved at `path`.
fn load(app: &AppWindow, state: &mut State, editor: Editor, path: Option<PathBuf>) {
    let tab = state.tab_mut();
    tab.saved = Some(editor.song().clone());
    tab.editor = Some(editor);
    tab.path = path;
    tab.audio = AudioFile::default();
    tab.errors = Errors::default();
    tab.error.clear();
    show_tab(app, state);
}

/// Writes the tab's song to `path` (F-ED-10), which becomes its file; false when it couldn't.
fn save_to(tab: &mut Tab, path: PathBuf) -> bool {
    let Some(editor) = &mut tab.editor else { return false };
    let mut song = editor.song().clone();
    // The audio path is relative to the song file: a copy in another folder needs it rebased.
    if let (Some(old), Some(audio)) = (tab.path.as_deref(), song.audio.as_mut())
        && old != path
    {
        audio.path = relative_audio_path(&path, &audio_path(old, &audio.path));
    }
    match write_atomic(&path, serialize_song(&song)) {
        Ok(()) => {
            if song != *editor.song() {
                editor.history.apply(song.clone());
            }
            tab.saved = Some(song);
            tab.path = Some(path);
            tab.error.clear();
            true
        }
        Err(e) => {
            tab.error = format!("Can't save {}: {e}", path.display());
            false
        }
    }
}

/// Asks where to save the active tab's song, then saves it there.
fn save_as(app: &AppWindow, state: &Rc<RefCell<State>>) {
    let (id, title) = {
        let state = state.borrow();
        let Some(editor) = &state.tab().editor else { return };
        (state.tab().id, editor.song().meta.title.clone())
    };
    // shortcut: "json" rather than "madsister.json", macOS dialogs don't handle multi-dot extensions (as in the Tauri app).
    let dialog = rfd::AsyncFileDialog::new().set_file_name(format!("{title}.madsister.json")).add_filter("madsister song", &["json"]);
    let (weak, st) = (app.as_weak(), state.clone());
    let picking = slint::spawn_local(async move {
        let Some(file) = dialog.save_file().await else { return };
        let Some(app) = weak.upgrade() else { return };
        let mut state = st.borrow_mut();
        let Some(tab) = state.tab_by_id(id) else { return };
        if save_to(tab, file.path().to_path_buf()) {
            remember(&app, &mut state, file.path());
        }
        refresh(&app, &mut state);
    });
    if let Err(e) = picking {
        eprintln!("can't open the file dialog: {e}");
    }
}

/// File ▸ Export (F-EX-1..4): the dialog's filter name, the extension, and the file's contents.
fn export_file(format: ExportFormat, song: &Song, settings: &Settings) -> Result<(&'static str, &'static str, Vec<u8>), String> {
    Ok(match format {
        ExportFormat::ChordPro => ("ChordPro", "cho", to_chord_pro(song).into_bytes()),
        ExportFormat::MusicXml => ("MusicXML", "musicxml", to_music_xml(song).into_bytes()),
        ExportFormat::Midi => ("MIDI", "mid", to_midi(song)),
        ExportFormat::Pdf => ("PDF", "pdf", madsister::pdf::render(&to_sheet(song, settings))?),
    })
}

/// Writes an export where the user picked; a failure shows on the tab.
fn save_export(tab: &mut Tab, path: &Path, contents: &[u8]) {
    tab.error = match write_atomic(path, contents) {
        Ok(()) => String::new(),
        Err(e) => format!("Can't export {}: {e}", path.display()),
    };
}

/// Asks where to export the active tab's song as `<title>.<extension>`, then writes it there.
fn export(app: &AppWindow, state: &Rc<RefCell<State>>, format: ExportFormat) {
    let (id, title, exported) = {
        let state = state.borrow();
        let Some(editor) = &state.tab().editor else { return };
        (state.tab().id, editor.song().meta.title.clone(), export_file(format, editor.song(), &state.settings))
    };
    let (name, extension, contents) = match exported {
        Ok(exported) => exported,
        Err(e) => {
            let mut state = state.borrow_mut();
            state.tab_mut().error = format!("Can't export: {e}");
            refresh(app, &mut state);
            return;
        }
    };
    let dialog = rfd::AsyncFileDialog::new().set_file_name(format!("{title}.{extension}")).add_filter(name, &[extension]);
    let (weak, st) = (app.as_weak(), state.clone());
    let picking = slint::spawn_local(async move {
        let Some(file) = dialog.save_file().await else { return };
        let Some(app) = weak.upgrade() else { return };
        let mut state = st.borrow_mut();
        let Some(tab) = state.tab_by_id(id) else { return };
        save_export(tab, file.path(), &contents);
        refresh(&app, &mut state);
    });
    if let Err(e) = picking {
        eprintln!("can't open the file dialog: {e}");
    }
}

/// Puts the song file first in the recent files.
fn remember(app: &AppWindow, state: &mut State, path: &Path) {
    let recent = push_recent(&state.recent, &path.to_string_lossy());
    set_recent(app, state, recent);
}

/// Shows the recent files in the File menu.
fn show_recent(app: &AppWindow, state: &State) {
    let labels: Vec<SharedString> = shortest_unique_paths(&state.recent).into_iter().map(SharedString::from).collect();
    app.set_recent(Rc::new(VecModel::from(labels)).into());
}

/// Changes the recent files, and saves them.
fn set_recent(app: &AppWindow, state: &mut State, recent: Vec<String>) {
    let json = serde_json::to_string(&recent).expect("strings always serialize");
    state.recent = recent;
    show_recent(app, state);
    if let Err(e) = write_config(state, "recent.json", &json) {
        state.tab_mut().error = format!("Can't save the recent files: {e}");
        refresh(app, state);
    }
}

/// Reads a file of the config folder; None when it's missing or unreadable (first run, tests).
fn read_config(state: &State, name: &str) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(state.config.as_ref()?.join(name)).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_config(state: &State, name: &str, json: &str) -> std::io::Result<()> {
    let Some(dir) = &state.config else { return Ok(()) };
    std::fs::create_dir_all(dir)?;
    write_atomic(&dir.join(name), json)
}

/// The chord styles in the Preferences dialog's order.
const STYLES: [DisplayStyle; 3] = [DisplayStyle::Fr, DisplayStyle::Intl, DisplayStyle::Latin];

/// The settings as the Preferences dialog shows them.
fn preferences_view(settings: &Settings) -> PreferencesView {
    PreferencesView {
        style: STYLES.iter().position(|s| *s == settings.style).unwrap_or_default() as i32,
        font: madsister::settings::ChartFont::ALL.iter().position(|f| *f == settings.font).unwrap_or_default() as i32,
        minor_from_tonic: settings.minor_convention == MinorConvention::Tonic,
        bars_per_row: settings.bars_per_row as i32,
        threshold_percent: (settings.low_confidence_threshold * 100.0).round() as i32,
        compact_print: settings.compact_print,
        reopen_tabs: settings.reopen_tabs,
    }
}

/// The settings the Preferences dialog saved; the ones it doesn't show stay.
fn settings_of(view: &PreferencesView, settings: &Settings) -> Settings {
    Settings {
        style: STYLES[view.style as usize],
        font: madsister::settings::ChartFont::ALL[view.font as usize],
        minor_convention: if view.minor_from_tonic { MinorConvention::Tonic } else { MinorConvention::Relative },
        bars_per_row: view.bars_per_row as usize,
        low_confidence_threshold: f64::from(view.threshold_percent) / 100.0,
        compact_print: view.compact_print,
        reopen_tabs: view.reopen_tabs,
        ..settings.clone()
    }
}

/// Writes settings.json, keeping the keys this app doesn't know.
fn save_settings(state: &mut State) {
    let json = state.settings.write_over(&read_config(state, "settings.json").unwrap_or_default()).to_string();
    if let Err(e) = write_config(state, "settings.json", &json) {
        state.tab_mut().error = format!("Can't save the preferences: {e}");
    }
}

/// Expands or collapses the sidebar (F-UI-4), remembered in settings.json.
fn set_sidebar_collapsed(app: &AppWindow, state: &mut State, is_collapsed: bool) {
    if state.settings.is_sidebar_collapsed != is_collapsed {
        state.settings.is_sidebar_collapsed = is_collapsed;
        save_settings(state);
    }
    refresh(app, state);
}

/// Shows the settings: the chart font, and the chart rebuilt with the others at the next refresh.
fn apply_settings(app: &AppWindow, state: &mut State) {
    let font = app.global::<ChartFont>();
    font.set_family(state.settings.font.family().into());
    font.set_has_bold(state.settings.font.has_bold());
    state.shown = None;
}

/// Writes session.json when the tabs that have a file, or the active one, changed.
fn save_session(state: &mut State) {
    let paths: Vec<String> = state.tabs.iter().filter_map(|tab| tab.path.as_ref()).map(|p| p.to_string_lossy().into_owned()).collect();
    let active = state.tab().path.as_ref().and_then(|p| paths.iter().position(|q| *q == p.to_string_lossy())).unwrap_or(0);
    let session = serde_json::json!({ "paths": paths, "active": active }).to_string();
    if session == state.session {
        return;
    }
    if let Err(e) = write_config(state, "session.json", &session) {
        state.tab_mut().error = format!("Can't save the open tabs: {e}");
    }
    state.session = session;
}

/// Reopens the tabs of the previous session that have a file still readable, unless the reopenTabs setting is off.
fn restore_session(state: &mut State) {
    let Some(json) = read_config(state, "session.json").filter(|_| state.settings.reopen_tabs) else { return };
    let paths = parse_paths(&json["paths"]);
    let active = json["active"].as_u64().and_then(|i| paths.get(i as usize)).map(PathBuf::from);
    let opened: Vec<_> = paths.iter().filter_map(|p| read_song(Path::new(p)).ok().map(|song| (PathBuf::from(p), song))).collect();
    if opened.is_empty() {
        return;
    }
    state.tabs.clear();
    for (path, song) in opened {
        let index = state.push_tab();
        let tab = &mut state.tabs[index];
        tab.saved = Some(song.clone());
        tab.editor = Some(viewer(song));
        tab.path = Some(path);
    }
    state.active = state.tabs.iter().position(|tab| tab.path == active).unwrap_or(0);
}

/// Asks for a song file to open.
fn pick_song(app: &AppWindow, state: &Rc<RefCell<State>>) {
    let dialog = rfd::AsyncFileDialog::new().add_filter("madsister song", &["json"]);
    let (weak, st) = (app.as_weak(), state.clone());
    let picking = slint::spawn_local(async move {
        let Some(file) = dialog.pick_file().await else { return };
        if let Some(app) = weak.upgrade() {
            open(&app, &mut st.borrow_mut(), file.path());
        }
    });
    if let Err(e) = picking {
        eprintln!("can't open the file dialog: {e}");
    }
}

/// Makes the tab at `index` the active one; the one it hides lets go of its decoded audio, checked and loaded again
/// when it's shown.
fn activate(state: &mut State, index: usize) {
    if index != state.active {
        let hidden = state.tab_mut();
        hidden.player = Ok(None);
        hidden.audio.checked = None;
    }
    state.active = index;
}

/// Shows the active tab after it changed.
fn show_tab(app: &AppWindow, state: &mut State) {
    state.shown = None;
    refresh(app, state);
    show_import(app, state);
    tick(app, state);
    if state.tab().editor.is_none() && !is_asking(app, state) {
        app.invoke_focus_shell();
    }
    focus_grid_when_idle(app, state);
}

/// Opens the song file at `path` (F-UI-7): its tab when it's open already, else in the untouched start screen or a new
/// tab. False when it can't be opened.
fn open(app: &AppWindow, state: &mut State, path: &Path) -> bool {
    if let Some(index) = state.tabs.iter().position(|tab| tab.path.as_deref() == Some(path)) {
        state.tab_mut().error.clear();
        activate(state, index);
        show_tab(app, state);
        return true;
    }
    match read_song(path) {
        Ok(song) => {
            state.tab_mut().error.clear();
            if !state.tab().is_idle() {
                let index = state.push_tab();
                activate(state, index);
            }
            load(app, state, viewer(song), Some(path.to_path_buf()));
            remember(app, state, path);
            true
        }
        Err(e) => {
            state.tab_mut().error = format!("Can't open {}: {e}", path.display());
            refresh(app, state);
            false
        }
    }
}

/// Closes the tab at `index`, asking first when it has unsaved changes.
fn close_tab(app: &AppWindow, state: &mut State, index: usize) {
    if state.tabs[index].is_dirty() {
        state.pending = Some(Pending::CloseTab(index));
        show_question(app, state);
    } else {
        close_tab_now(app, state, index);
    }
}

fn close_tab_now(app: &AppWindow, state: &mut State, index: usize) {
    let tab = state.tabs.remove(index);
    if let Some(id) = tab.importer.job_id() {
        state.jobs.cancel(id);
    }
    if state.tabs.is_empty() {
        state.push_tab();
    }
    // The active tab stays in front; closing it shows the next one, or the last.
    if index < state.active {
        state.active -= 1;
    }
    state.active = state.active.min(state.tabs.len() - 1);
    show_tab(app, state);
}

/// Runs `f` on the open editor with the settings, then shows the result.
fn with_editor<R: Default>(app: &slint::Weak<AppWindow>, state: &Rc<RefCell<State>>, f: impl FnOnce(&mut Editor, &Settings) -> R) -> R {
    let Some(app) = app.upgrade() else { return R::default() };
    let mut state = state.borrow_mut();
    let state = &mut *state;
    let Some(editor) = &mut state.tabs[state.active].editor else { return R::default() };
    let result = f(editor, &state.settings);
    refresh(&app, state);
    result
}

/// Like `with_editor`, with the whole state (field errors, pending confirmation).
fn with_state(app: &slint::Weak<AppWindow>, state: &Rc<RefCell<State>>, f: impl FnOnce(&mut State)) {
    let Some(app) = app.upgrade() else { return };
    let mut state = state.borrow_mut();
    if state.tab().editor.is_some() {
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
            let tab = &mut state.tabs[state.active];
            let editor = tab.editor.as_mut().expect("with_state checked it");
            let error = editor.set_meta(field, &text, &state.settings).err().unwrap_or_default();
            match field {
                MetaField::Title => tab.errors.title = error,
                MetaField::Key => tab.errors.key = error,
                MetaField::Tempo => tab.errors.tempo = error,
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
            let tab = state.tab_mut();
            let editor = tab.editor.as_mut().expect("with_state checked it");
            tab.errors.section = editor.rename_cursor_section(&text).err().unwrap_or_default();
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
            let editor = state.tab_mut().editor.as_mut().expect("with_state checked it");
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
                if let Some(editor) = state.tab_mut().editor.as_mut() {
                    editor.delete_cursor_section();
                }
                refresh(&app, &mut state);
            }
            Some(Pending::Overwrite(_)) if is_yes => {
                let state = &mut *state; // borrows the tab and the engine apart
                let next = state.tabs[state.active].importer.transcribe_confirmed(|f| engine_has(state.engine.as_ref(), f));
                follow(&app, state, next);
            }
            Some(Pending::CloseTab(index)) if is_yes => close_tab_now(&app, &mut state, index),
            Some(Pending::OtherAudio(file, sha256)) if is_yes => {
                link_audio(state.tab_mut(), &file, sha256);
                refresh(&app, &mut state);
            }
            Some(Pending::CloseWindow) if is_yes => {
                drop(state);
                close_window(&app);
                return;
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
        if let Ok(Some(player)) = &st.borrow().tab().player {
            player.toggle();
        }
        if let Some(app) = weak.upgrade() {
            tick(&app, &st.borrow());
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    player.on_seek(move |sec| {
        if let Ok(Some(player)) = &st.borrow().tab().player {
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
        if let (Some(start), Ok(Some(player))) = (start, &st.borrow().tab().player) {
            player.seek(start);
        }
    });

    let st = state.clone();
    player.on_locate_audio(move || {
        let (st, dialog) = (st.clone(), audio_dialog());
        let id = st.borrow().tab().id;
        let picking = slint::spawn_local(async move {
            let Some(file) = dialog.pick_file().await else { return };
            let file = file.path().to_path_buf();
            if let Some(tab) = st.borrow_mut().tab_by_id(id) {
                tab.audio.locating = Some((file.clone(), hash_in_background(file)));
            }
        });
        if let Err(e) = picking {
            eprintln!("can't open the file dialog: {e}");
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    player.on_show_song_file(move || {
        with_state(&weak, &st, |state| {
            let tab = state.tab_mut();
            if let (Some(path), Some(editor)) = (&tab.path, &mut tab.editor)
                && let Err(e) = opener::reveal(path)
            {
                editor.notice = format!("Can't show {}: {e}", path.display());
            }
        });
    });

    let (weak, st) = (app.as_weak(), state.clone());
    app.on_panel_shown(move |index| {
        if let Some(app) = weak.upgrade() {
            set_sidebar_collapsed(&app, &mut st.borrow_mut(), false);
            app.invoke_reveal_panel(index);
        }
    });

    // The chart's header and time signatures lead to their field in the panels, expanded first.
    let grid = app.global::<Grid>();
    let (weak, st) = (app.as_weak(), state.clone());
    grid.on_meta_clicked(move |field| {
        if let Some(app) = weak.upgrade() {
            set_sidebar_collapsed(&app, &mut st.borrow_mut(), false);
            app.invoke_focus_meta(field);
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    grid.on_meter_clicked(move |section, bar| {
        let at = SlotRef { section: section as usize, bar: bar as usize, slot: 0 };
        with_editor(&weak, &st, |editor, _| editor.select_bar(at));
        if let Some(app) = weak.upgrade() {
            set_sidebar_collapsed(&app, &mut st.borrow_mut(), false);
            app.invoke_focus_bar_beats();
        }
    });
}

/// Connects the tab bar, the File menu and the window's close button (F-UI-7).
fn wire_tabs(app: &AppWindow, state: &Rc<RefCell<State>>) {
    let (weak, st) = (app.as_weak(), state.clone());
    // The tabs stay as they are while a question is open: its answer applies to the tab it was asked for. A screen
    // reader can still reach the tab bar under it.
    let select = |app: &AppWindow, state: &mut State, index: usize| {
        if !is_asking(app, state) {
            activate(state, index);
            show_tab(app, state);
        }
    };
    app.on_tab_selected(move |index| {
        if let Some(app) = weak.upgrade() {
            select(&app, &mut st.borrow_mut(), index as usize);
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_tab_cycled(move |step| {
        let Some(app) = weak.upgrade() else { return };
        let mut state = st.borrow_mut();
        let index = (state.active as i32 + step).rem_euclid(state.tabs.len() as i32);
        select(&app, &mut state, index as usize);
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_tab_closed(move |index| {
        let Some(app) = weak.upgrade() else { return };
        let mut state = st.borrow_mut();
        if !is_asking(&app, &state) {
            close_tab(&app, &mut state, index as usize);
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_app_action(move |action| {
        let (weak, st) = (weak.clone(), st.clone());
        // Deferred: macOS native menus break if their structure changes while an item's activation is being handled.
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            let Some(app) = weak.upgrade() else { return };
            let mut state = st.borrow_mut();
            // The menu's shortcuts still fire under the dialogs; refreshing unchecks what the menu toggled.
            if is_asking(&app, &state) {
                refresh(&app, &mut state);
                return;
            }
            match action {
                AppAction::NewTab => {
                    let index = state.push_tab();
                    activate(&mut state, index);
                    show_tab(&app, &mut state);
                }
                AppAction::CloseTab => {
                    let active = state.active;
                    close_tab(&app, &mut state, active);
                }
                AppAction::Save => match state.tab().path.clone() {
                    Some(path) => {
                        save_to(state.tab_mut(), path);
                        refresh(&app, &mut state);
                    }
                    None => {
                        drop(state);
                        save_as(&app, &st);
                    }
                },
                AppAction::SaveAs => {
                    drop(state);
                    save_as(&app, &st);
                }
                AppAction::Open => {
                    drop(state);
                    pick_song(&app, &st);
                }
                AppAction::ClearRecent => set_recent(&app, &mut state, Vec::new()),
                AppAction::Quit => {
                    if !asks_before_closing(&app, &mut state) {
                        drop(state);
                        close_window(&app);
                    }
                }
                AppAction::Undo | AppAction::Redo => {
                    // Read-only in view mode, like the grid.
                    if let Some(editor) = state.tab_mut().editor.as_mut().filter(|e| e.is_editable) {
                        if action == AppAction::Undo {
                            editor.history.undo();
                        } else {
                            editor.history.redo();
                        }
                    }
                    refresh(&app, &mut state);
                }
                AppAction::EditMode => {
                    if let Some(editor) = state.tab_mut().editor.as_mut() {
                        editor.is_editable = !editor.is_editable;
                        editor.draft = None;
                        editor.notice.clear();
                    }
                    refresh(&app, &mut state);
                    focus_grid_when_idle(&app, &state);
                }
                AppAction::Preferences => {
                    app.set_preferences(preferences_view(&state.settings));
                    app.set_dialog(Dialog::Preferences);
                }
                AppAction::About => app.set_dialog(Dialog::About),
                AppAction::Sidebar => {
                    let is_collapsed = !state.settings.is_sidebar_collapsed;
                    set_sidebar_collapsed(&app, &mut state, is_collapsed);
                }
                AppAction::Minimize => app.window().set_minimized(true),
                AppAction::Zoom => app.window().set_maximized(!app.window().is_maximized()),
            }
        });
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_preferences_saved(move |view| {
        let Some(app) = weak.upgrade() else { return };
        let mut state = st.borrow_mut();
        state.settings = settings_of(&view, &state.settings);
        save_settings(&mut state);
        app.set_dialog(Dialog::None);
        apply_settings(&app, &mut state);
        refresh(&app, &mut state);
        focus_grid_when_idle(&app, &state);
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_open_url(move |url| {
        if let Err(e) = opener::open_browser(url.as_str())
            && let Some(app) = weak.upgrade()
        {
            let mut state = st.borrow_mut();
            state.tab_mut().error = format!("Can't open {url}: {e}");
            refresh(&app, &mut state);
        }
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_export(move |format| {
        let (weak, st) = (weak.clone(), st.clone());
        // Deferred, as the File menu's actions.
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let Some(app) = weak.upgrade()
                && !is_asking(&app, &st.borrow())
            {
                export(&app, &st, format);
            }
        });
    });
    let (weak, st) = (app.as_weak(), state.clone());
    app.on_open_recent(move |index| {
        let (weak, st) = (weak.clone(), st.clone());
        // Deferred, as the File menu's actions.
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            let Some(app) = weak.upgrade() else { return };
            let mut state = st.borrow_mut();
            let Some(path) = state.recent.get(index as usize).cloned() else { return };
            if !is_asking(&app, &state) && !open(&app, &mut state, Path::new(&path)) {
                let recent = state.recent.iter().filter(|p| **p != path).cloned().collect();
                set_recent(&app, &mut state, recent);
            }
        });
    });
    let (weak, st) = (app.as_weak(), Rc::downgrade(state));
    state.borrow().autosave.start(slint::TimerMode::SingleShot, std::time::Duration::from_secs(2), move || {
        let (Some(app), Some(st)) = (weak.upgrade(), st.upgrade()) else { return };
        let mut state = st.borrow_mut();
        for tab in state.tabs.iter_mut().filter(|tab| tab.is_dirty()) {
            if let Some(path) = tab.path.clone() {
                save_to(tab, path);
            }
        }
        refresh(&app, &mut state);
    });
    state.borrow().autosave.stop();
    let (weak, st) = (app.as_weak(), state.clone());
    app.window().on_close_requested(move || {
        let Some(app) = weak.upgrade() else { return slint::CloseRequestResponse::HideWindow };
        if asks_before_closing(&app, &mut st.borrow_mut()) {
            slint::CloseRequestResponse::KeepWindowShown
        } else {
            slint::CloseRequestResponse::HideWindow
        }
    });
}

/// Closing the window (or Quit) with unsaved songs asks first: true when it asked.
fn asks_before_closing(app: &AppWindow, state: &mut State) -> bool {
    if !state.tabs.iter().any(Tab::is_dirty) {
        return false;
    }
    state.pending = Some(Pending::CloseWindow);
    show_question(app, state);
    true
}

fn close_window(app: &AppWindow) {
    _ = app.hide();
    _ = slint::quit_event_loop();
}

/// Connects the window's callbacks to the editor.
fn wire(app: &AppWindow, state: &Rc<RefCell<State>>) {
    wire_panels(app, state);
    wire_start(app, state);
    wire_tabs(app, state);
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
            editor.key(e, settings)
        });
        let mut state = grid_state.borrow_mut();
        match outcome {
            Outcome::Ignored => return false,
            Outcome::Handled => return true,
            Outcome::Play => {
                let tab = state.tab_mut();
                let is_toggled = matches!(&tab.player, Ok(Some(player)) if player.toggle());
                if !is_toggled && let Some(editor) = &mut tab.editor {
                    editor.notice = "No audio to play".into();
                }
            }
            Outcome::PlayFrom(sec) => play_from(state.tab_mut(), sec),
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
        if let (Some(start), Ok(Some(player))) = (start, &grid_state.borrow().tab().player) {
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
        if let Some(editor) = &mut grid_state.borrow_mut().tab_mut().editor {
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
        if grid_state.borrow().tab().editor.as_ref().is_none_or(|editor| editor.draft.is_none()) {
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
    app.set_is_mac(cfg!(target_os = "macos"));
    app.set_version(env!("CARGO_PKG_VERSION").into());
    let mut state = State::new(Settings::load());
    state.engine = madsister::engine::packaged_engine();
    // At launch rather than on the start screen: it downloads for minutes, songs can be edited meanwhile.
    state.setup.is_needed = state.engine.as_ref().is_some_and(Packaged::needs_setup);
    if state.setup.is_needed {
        run_setup(&mut state);
    }
    state.config = madsister::settings::config_dir();
    state.recent = read_config(&state, "recent.json").map_or_else(Vec::new, |json| parse_paths(&json));
    show_recent(&app, &state);
    restore_session(&mut state);
    apply_settings(&app, &mut state);
    let state = Rc::new(RefCell::new(state));
    let shortcuts: Vec<Shortcut> = SHORTCUTS
        .iter()
        .map(|(keys, what)| Shortcut { keys: keys.replace("Mod", MOD_LABEL).into(), what: (*what).into() })
        .collect();
    app.set_shortcuts(Rc::new(VecModel::from(shortcuts)).into());
    match args.first() {
        Some(path) => _ = open(&app, &mut state.borrow_mut(), Path::new(path)),
        None => show_tab(&app, &mut state.borrow_mut()),
    }

    wire(&app, &state);

    let weak = app.as_weak();
    let drop_state = state.clone();
    app.window().on_winit_window_event(move |_, event| {
        let Some(app) = weak.upgrade() else { return EventResult::Propagate };
        let WindowEvent::DroppedFile(path) = event else { return EventResult::Propagate };
        let mut state = drop_state.borrow_mut();
        // As for the tab bar: the tabs stay as they are while a question is open.
        if is_asking(&app, &state) {
            return EventResult::Propagate;
        }
        let is_song = path.to_string_lossy().ends_with(".json");
        // On the start screen, an audio file is for the importer (F-IN-1); a song file opens.
        if !is_song && state.tab().editor.is_none() {
            if !state.tab().importer.is_running() {
                state.tab_mut().importer.select_file(path);
                show_import(&app, &state);
            }
            return EventResult::Propagate;
        }
        open(&app, &mut state, path);
        EventResult::Propagate
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
