//! The window driven like a user would: key events and clicks on a headless Slint backend.

use i_slint_backend_testing::ElementHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};

use super::*;
use madsister::model::song::{Meter, empty_song};

fn window() -> (AppWindow, Rc<RefCell<State>>) {
    i_slint_backend_testing::init_no_event_loop();
    let app = AppWindow::new().unwrap();
    // Tall enough for every panel: the element search skips what the sidebar's scroll view clips.
    app.window().set_size(slint::LogicalSize::new(1000.0, 1800.0));
    app.show().unwrap();
    let song = empty_song(Meter { beats: 4, unit: 4, extra: Default::default() }, None);
    let state = Rc::new(RefCell::new(State { settings: Settings::parse(&serde_json::Value::Null), editor: Some(Editor::new(song)), shown: None, errors: Errors::default(), pending: None, player: Ok(None) }));
    wire(&app, &state);
    refresh(&app, &mut state.borrow_mut());
    focus_grid_when_idle(&app, &state.borrow());
    (app, state)
}

fn press(app: &AppWindow, text: impl Into<SharedString>) {
    let text = text.into();
    app.window().dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    app.window().dispatch_event(WindowEvent::KeyReleased { text });
}

fn with_ctrl(app: &AppWindow, text: &str) {
    let control = SharedString::from(Key::Control);
    app.window().dispatch_event(WindowEvent::KeyPressed { text: control.clone() });
    press(app, text);
    app.window().dispatch_event(WindowEvent::KeyReleased { text: control });
}

/// Types each character; `\n` is Enter.
fn type_keys(app: &AppWindow, keys: &str) {
    for c in keys.chars() {
        if c == '\n' {
            press(app, Key::Return);
        } else {
            press(app, c.to_string());
        }
    }
}

fn chords(state: &Rc<RefCell<State>>, bar: usize) -> Vec<String> {
    let state = state.borrow();
    state.editor.as_ref().unwrap().song().sections[0].bars[bar].chords.iter().map(|c| c.chord.clone()).collect()
}

/// A left click in the middle of the element.
fn click(app: &AppWindow, element: &ElementHandle) {
    let (at, size) = (element.absolute_position(), element.size());
    let position = slint::LogicalPosition::new(at.x + size.width / 2.0, at.y + size.height / 2.0);
    let button = PointerEventButton::Left;
    app.window().dispatch_event(WindowEvent::PointerPressed { position, button });
    app.window().dispatch_event(WindowEvent::PointerReleased { position, button });
}

fn find(app: &AppWindow, label: &str) -> Option<ElementHandle> {
    ElementHandle::find_by_accessible_label(app, label).next()
}

/// A text field by its label (the label's own text has the same accessible name).
fn field(app: &AppWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(app, label)
        .find(|e| e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::TextInput))
        .unwrap()
}

#[test]
fn typing_on_the_grid_sets_chords_and_moves_on() {
    // Given an empty song, the grid focused
    let (app, state) = window();

    // When typing two chords, the second one invalid first
    type_keys(&app, "C\nGx\n");

    // Then the first is set, and the input shows the second one's error
    assert_eq!(chords(&state, 0), ["C:maj"]);
    assert_eq!(app.global::<Grid>().get_draft_error(), "unknown chord quality \"x\"");
    assert!(find(&app, "Chord").is_some());

    // When fixing it by retyping, Then it's set and the input closes on the next slot
    press(&app, Key::Backspace);
    press(&app, Key::Return);
    assert_eq!(chords(&state, 1), ["G:maj"]);
    assert!(find(&app, "Chord").is_none());
    assert_eq!(app.global::<Grid>().get_cursor_bar(), 2);
}

#[test]
fn clicking_a_slot_opens_its_chord_for_editing() {
    // Given C then G
    let (app, state) = window();
    type_keys(&app, "C\nG\n");

    // When clicking the first slot
    click(&app, &find(&app, "Verse, bar 1, beat 1: C").unwrap());

    // Then its chord is in the input, and typing over it then Escape keeps the song
    assert_eq!(find(&app, "Chord").and_then(|e| e.accessible_value()).as_deref(), Some("C"));
    type_keys(&app, "D");
    press(&app, Key::Escape);
    assert!(find(&app, "Chord").is_none());
    assert_eq!(chords(&state, 0), ["C:maj"]);
}

#[test]
fn shortcuts_edit_undo_and_announce_refusals() {
    // Given C in the first bar
    let (app, state) = window();
    type_keys(&app, "C\n");
    press(&app, Key::LeftArrow);

    // When splitting it, then undoing
    press(&app, "/");
    assert_eq!(chords(&state, 0), ["C:maj", "C:maj"]);
    with_ctrl(&app, "z");
    assert_eq!(chords(&state, 0), ["C:maj"]);

    // When merging the first slot with a previous one, Then the refusal is shown
    press(&app, Key::Backspace);
    assert_eq!(app.get_notice(), "Not possible here");

    // When switching to view mode and typing, Then it's read-only
    with_ctrl(&app, "e");
    press(&app, "A");
    assert!(find(&app, "Chord").is_none());
    assert!(app.get_notice().starts_with("Read-only"));
}

#[test]
fn the_help_opens_with_a_question_mark_and_closes_with_escape() {
    // Given / When
    let (app, _state) = window();
    press(&app, "?");

    // Then
    assert!(app.get_is_help_open());
    assert!(find(&app, "Keyboard shortcuts").is_some());
    press(&app, Key::Escape);
    assert!(!app.get_is_help_open());
}

#[test]
fn mod_slash_while_typing_splits_the_slot_and_keeps_editing() {
    // Given C typed in the first slot
    let (app, state) = window();
    type_keys(&app, "C");

    // When pressing Ctrl+/ (the sheet is rebuilt under the input)
    with_ctrl(&app, "/");

    // Then the bar is split and the new half is being edited with C preselected
    assert_eq!(chords(&state, 0), ["C:maj", "C:maj"]);
    assert_eq!(find(&app, "Chord").and_then(|e| e.accessible_value()).as_deref(), Some("C"));
    assert_eq!(app.global::<Grid>().get_cursor_slot(), 1);
}

#[test]
fn the_song_panel_sets_the_title_and_shows_errors() {
    // Given
    let (app, state) = window();
    let title = field(&app, "Title");

    // When typing a new title in its field and pressing Enter
    click(&app, &title);
    title.set_accessible_value("Blue");
    press(&app, Key::Return);

    // Then the song is renamed
    assert_eq!(state.borrow().editor.as_ref().unwrap().song().meta.title, "Blue");

    // When the tempo isn't a number, Then the field says so and nothing changes
    let tempo = field(&app, "Tempo (BPM)");
    click(&app, &tempo);
    tempo.set_accessible_value("fast");
    press(&app, Key::Return);
    assert_eq!(app.get_tempo_error(), "the tempo must be a positive number");
    assert_eq!(state.borrow().editor.as_ref().unwrap().song().meta.tempo_bpm, None);
}

#[test]
fn deleting_a_section_with_chords_asks_first() {
    // Given a chord in the only section
    let (app, state) = window();
    type_keys(&app, "C\n");

    // When deleting the section, Then it asks, and Cancel keeps it
    click(&app, &find(&app, "Delete section").unwrap());
    assert!(app.get_confirm_text().starts_with("Delete the section \"Verse\""));
    click(&app, &find(&app, "Cancel").unwrap());
    assert_eq!(chords(&state, 0), ["C:maj"]);

    // When accepting, Then an empty section replaces it
    click(&app, &find(&app, "Delete section").unwrap());
    click(&app, &find(&app, "Delete").unwrap());
    assert_eq!(app.get_confirm_text(), "");
    assert_eq!(state.borrow().editor.as_ref().unwrap().song().sections[0].label, "Song");
}

