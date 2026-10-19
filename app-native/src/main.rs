use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{Model, SharedString, VecModel};

use madsister::editor::{DraftKind, Editor, MOD_LABEL};
use madsister::keymap::{KeyInput, SHORTCUTS};
use madsister::model::commands::SlotRef;
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
    if draft.is_none() && !editor.is_help_open {
        app.invoke_focus_grid();
    }
}

/// Opens the song file at `path`; false when it can't be opened.
fn open(app: &AppWindow, state: &mut State, path: &Path) -> bool {
    match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|text| parse_song(&text)) {
        Ok(song) => {
            state.editor = Some(Editor::new(song));
            state.shown = None;
            refresh(app, state);
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

/// Connects the window's callbacks to the editor.
fn wire(app: &AppWindow, state: &Rc<RefCell<State>>) {
    let weak = app.as_weak();
    let grid_state = state.clone();
    app.on_grid_key(move |event| {
        let e = key_input(&event);
        with_editor(&weak, &grid_state, |editor, settings| {
            // The inline input appears at the next layout: keys typed before then still belong to it.
            if editor.draft.is_some() {
                if editor.draft_key(e, settings) {
                    return true;
                }
                if let Some(draft) = &editor.draft
                    && !(e.ctrl || e.meta || e.alt)
                    && e.key.chars().count() == 1
                    && !e.key.chars().all(char::is_control)
                {
                    let text = format!("{}{}", draft.text, e.key);
                    editor.set_draft_text(&text);
                    return true;
                }
            }
            if (e.ctrl || e.meta) && e.key.eq_ignore_ascii_case("e") {
                editor.is_editable = !editor.is_editable;
                editor.draft = None;
                editor.notice.clear();
                return true;
            }
            // shortcut: playback comes with step 9; until then there's no audio to play.
            match editor.key(e, settings) {
                madsister::editor::Outcome::Ignored => false,
                madsister::editor::Outcome::Handled => true,
                _ => {
                    editor.notice = "No audio to play".into();
                    true
                }
            }
        })
    });
    let weak = app.as_weak();
    let help_state = state.clone();
    app.on_close_help(move || with_editor(&weak, &help_state, |editor, _| editor.is_help_open = false));

    let grid = app.global::<Grid>();
    let (weak, grid_state) = (app.as_weak(), state.clone());
    grid.on_slot_clicked(move |section, bar, slot| {
        let at = SlotRef { section: section as usize, bar: bar as usize, slot: slot as usize };
        with_editor(&weak, &grid_state, |editor, settings| editor.click(at, settings));
    });
    let (weak, grid_state) = (app.as_weak(), state.clone());
    grid.on_section_clicked(move |index| with_editor(&weak, &grid_state, |editor, _| editor.select_section(index as usize)));
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
        with_editor(&weak, &grid_state, |editor, settings| editor.draft_key(e, settings))
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
    let state = Rc::new(RefCell::new(State { settings: Settings::load(), editor: None, shown: None }));
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

    let weak = app.as_weak();
    app.on_open_recent(move |path| {
        if let Some(app) = weak.upgrade() {
            open(&app, &mut state.borrow_mut(), Path::new(path.as_str()));
        }
    });
    // Deferred: macOS native menus break if their structure changes while an item's activation is being handled.
    app.on_clear_recent(move || {
        let recent = recent.clone();
        slint::Timer::single_shot(std::time::Duration::ZERO, move || recent.set_vec(Vec::new()));
    });

    app.run()
}
