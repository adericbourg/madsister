use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{Model, SharedString, VecModel};

use madsister::audio::{Player as AudioPlayer, Status};
use madsister::editor::{DraftKind, Editor, MOD_LABEL, MenuAction, MetaField, Outcome};
use madsister::keymap::{count_flagged, has_confidence};
use madsister::model::playback::bar_at_time;
use madsister::keymap::{KeyInput, SHORTCUTS};
use madsister::model::commands::{BarRef, SlotRef, Step};
use madsister::model::song::Notation;
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
    /// The song the sheet was last built from: rebuilding it recreates every slot, the inline input included.
    shown: Option<madsister::model::song::Song>,
    /// The panels' last field errors.
    errors: Errors,
    /// What the open confirmation dialog would do.
    pending: Option<Pending>,
    /// The song's audio, when it has one; or why it can't play.
    player: Result<Option<AudioPlayer>, String>,
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
    app.set_title_error(state.errors.title.as_str().into());
    app.set_key_error(state.errors.key.as_str().into());
    app.set_tempo_error(state.errors.tempo.as_str().into());
    app.set_section_error(state.errors.section.as_str().into());
    let question = match state.pending {
        Some(Pending::DeleteSection) => {
            format!("Delete the section \"{}\" and its chords?", editor.song().sections[editor.cursor().section].label)
        }
        None => String::new(),
    };
    app.set_confirm_text(question.into());
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

/// The song's audio file: a relative `audio.path` is relative to the song file's folder.
fn audio_path(song_path: &Path, audio: &str) -> std::path::PathBuf {
    let audio = Path::new(audio);
    if audio.is_absolute() { audio.to_path_buf() } else { song_path.parent().unwrap_or(Path::new("")).join(audio) }
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
    ui.set_has_audio(editor.song().audio.is_some());
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
            state.player = match &song.audio {
                None => Ok(None),
                Some(audio) => AudioPlayer::open(&audio_path(path, &audio.path)).map(Some),
            };
            state.editor = Some(Editor::new(song));
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
        with_state(&weak, &st, |state| {
            let editor = state.editor.as_mut().expect("with_state checked it");
            match state.pending.take() {
                Some(Pending::DeleteSection) if is_yes => editor.delete_cursor_section(),
                _ => {}
            }
        });
        if let Some(app) = weak.upgrade() {
            focus_grid_when_idle(&app, &st.borrow());
        }
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
        shown: None,
        errors: Errors::default(),
        pending: None,
        player: Ok(None),
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
        if let WindowEvent::DroppedFile(path) = event
            && let Some(app) = weak.upgrade()
            && open(&app, &mut drop_state.borrow_mut(), path)
        {
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
    ticker.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(100), move || {
        if let Some(app) = weak.upgrade() {
            tick(&app, &state.borrow());
        }
    });

    app.run()
}
