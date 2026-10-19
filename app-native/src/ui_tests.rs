//! The window driven like a user would: key events and clicks on a headless Slint backend.

use i_slint_backend_testing::ElementHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};

use super::*;
use madsister::model::song::{Meter, empty_song};

fn window() -> (AppWindow, Rc<RefCell<State>>) {
    let (app, state) = start_screen();
    let song = empty_song(Meter { beats: 4, unit: 4, extra: Default::default() }, None);
    load(&app, &mut state.borrow_mut(), Editor::new(song), None);
    (app, state)
}

/// The window as it opens: no song yet.
fn start_screen() -> (AppWindow, Rc<RefCell<State>>) {
    i_slint_backend_testing::init_no_event_loop();
    let app = AppWindow::new().unwrap();
    // Tall enough for every panel: the element search skips what the sidebar's scroll view clips.
    app.window().set_size(slint::LogicalSize::new(1000.0, 1800.0));
    app.show().unwrap();
    let state = Rc::new(RefCell::new(State::new(Settings::parse(&serde_json::Value::Null))));
    wire(&app, &state);
    refresh(&app, &mut state.borrow_mut());
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
    state.tab().editor.as_ref().unwrap().song().sections[0].bars[bar].chords.iter().map(|c| c.chord.clone()).collect()
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
    assert_eq!(state.borrow().tab().editor.as_ref().unwrap().song().meta.title, "Blue");

    // When the tempo isn't a number, Then the field says so and nothing changes
    let tempo = field(&app, "Tempo (BPM)");
    click(&app, &tempo);
    tempo.set_accessible_value("fast");
    press(&app, Key::Return);
    assert_eq!(app.get_tempo_error(), "the tempo must be a positive number");
    assert_eq!(state.borrow().tab().editor.as_ref().unwrap().song().meta.tempo_bpm, None);
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
    assert_eq!(state.borrow().tab().editor.as_ref().unwrap().song().sections[0].label, "Song");
}


#[test]
fn the_start_screen_creates_an_editable_grid_of_the_chosen_meter() {
    // Given the start screen
    let (app, state) = start_screen();
    assert!(!app.get_has_song());

    // When creating a grid with the defaults
    click(&app, &find(&app, "Create grid").unwrap());

    // Then a 4/4 song opens in edit mode
    let state = state.borrow();
    let editor = state.tab().editor.as_ref().unwrap();
    assert!(app.get_has_song() && editor.is_editable);
    assert_eq!((editor.song().meta.meter.beats, editor.song().meta.meter.unit), (4, 4));
}

#[test]
fn the_importer_refuses_a_url_that_isnt_http() {
    // Given a file URL in the URL tab
    let (app, _state) = start_screen();
    let import = app.global::<Import>();
    import.set_source(1);
    import.set_url("file:///etc/passwd".into());

    // When running, Then it's refused and nothing runs
    import.invoke_run();
    assert!(import.get_failure().contains("http"));
    assert!(!import.get_is_running());
}

#[test]
fn a_song_whose_audio_file_is_missing_says_so_until_it_is_located() {
    // Given a song file whose audio file isn't there, with a field this version doesn't know in its audio
    let (app, state) = start_screen();
    let mut song = empty_song(Meter { beats: 4, unit: 4, extra: Default::default() }, None);
    let extra = serde_json::json!({ "future": 1 }).as_object().unwrap().clone();
    song.audio = Some(madsister::model::song::Audio { path: "gone.mp3".into(), sha256: "abc".into(), extra });
    let dir = std::env::temp_dir().join(format!("madsister-missing-audio-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("song.madsister.json");
    std::fs::write(&path, madsister::model::song::serialize_song(&song)).unwrap();
    let settle = || {
        while state.borrow().tab().audio.hashing.is_some() || state.borrow().tab().audio.locating.is_some() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            poll_audio(&app, &mut state.borrow_mut());
        }
        tick(&app, &state.borrow());
    };

    // When opening it, once its audio file is checked
    assert!(open(&app, &mut state.borrow_mut(), &path));
    settle();

    // Then the notice names the file and offers to locate it, and there's nothing to play
    let notice = "Audio file not found next to the song: gone.mp3.\nThe song can't be played until you locate it.";
    assert!(find(&app, notice).is_some());
    assert!(find(&app, "Locate audio…").is_some() && find(&app, "Show song file").is_some());
    assert!(find(&app, "Playback").is_none() && matches!(state.borrow().tab().player, Ok(None)));

    // When an audio file next to the song is located (what the dialog's pick does)
    let found = dir.join("found.mp3");
    std::fs::write(&found, "audio").unwrap();
    state.borrow_mut().tab_mut().audio.locating = Some((found.clone(), hash_in_background(found.clone())));
    settle();

    // Then the song points to it, relative to the song, keeping the unknown field, and the notice gives way to the player
    let audio = state.borrow().tab().editor.as_ref().unwrap().song().audio.clone().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!((audio.path.as_str(), audio.extra["future"].as_i64()), ("found.mp3", Some(1)));
    assert_eq!(audio.sha256, "6ed8919ce20490a5e3ad8630a4fab69475297abd07db73918dd5f36fcfaeb11b"); // printf audio | sha256sum
    assert!(find(&app, notice).is_none() && find(&app, "Playback").is_some());
}

/// Runs the timers due now: the File menu's actions are deferred.
fn run_deferred() {
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(1));
}

/// The tab bar's labels, `•` marking unsaved changes.
fn tab_labels(app: &AppWindow) -> Vec<String> {
    app.get_tabs().iter().map(|tab| format!("{}{}", if tab.is_dirty { "•" } else { "" }, tab.title)).collect()
}

/// A folder of its own for the test's files.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("madsister-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes a song titled `title` to `dir/<title>.madsister.json`.
fn song_file(dir: &Path, title: &str) -> PathBuf {
    let mut song = empty_song(Meter { beats: 4, unit: 4, extra: Default::default() }, None);
    song.meta.title = title.into();
    let path = dir.join(format!("{title}.madsister.json"));
    std::fs::write(&path, madsister::model::song::serialize_song(&song)).unwrap();
    path
}

#[test]
fn a_new_tab_opens_on_the_start_screen_and_each_tab_keeps_its_song() {
    // Given a song with an unsaved chord
    let (app, state) = window();
    type_keys(&app, "C\n");
    assert_eq!(tab_labels(&app), ["•Untitled"]);
    assert_eq!(app.get_window_title(), "• Untitled — madsister");

    // When opening a new tab with Ctrl+N
    with_ctrl(&app, "n");
    run_deferred();

    // Then it shows the start screen, in front
    assert_eq!(tab_labels(&app), ["•Untitled", "New tab"]);
    assert_eq!(app.get_active_tab(), 1);
    assert!(!app.get_has_song() && find(&app, "Create grid").is_some());
    assert_eq!(app.get_window_title(), "Untitled — madsister");

    // When going back to the first tab, Then its song and its unsaved chord are back
    click(&app, &find(&app, "Untitled (unsaved changes)").unwrap());
    assert_eq!(app.get_active_tab(), 0);
    assert!(app.get_has_song());
    assert_eq!(chords(&state, 0), ["C:maj"]);
}

#[test]
fn closing_a_tab_with_unsaved_changes_asks_first() {
    // Given an unsaved chord in the only tab
    let (app, state) = window();
    type_keys(&app, "C\n");

    // When closing it and cancelling, Then it stays
    click(&app, &find(&app, "Close Untitled").unwrap());
    assert_eq!(app.get_confirm_text(), "Discard unsaved changes?");
    click(&app, &find(&app, "Cancel").unwrap());
    assert_eq!(tab_labels(&app), ["•Untitled"]);

    // When closing it from the keyboard and discarding, Then the start screen takes its place, without tabs
    with_ctrl(&app, "w");
    run_deferred();
    click(&app, &find(&app, "Discard").unwrap());
    assert_eq!(tab_labels(&app), ["New tab"]);
    assert!(!app.get_has_song() && state.borrow().tab().editor.is_none());
    assert!(find(&app, "Close New tab").is_none());
}

#[test]
fn closing_the_window_with_unsaved_changes_asks_first() {
    // Given an unsaved chord
    let (app, _state) = window();
    type_keys(&app, "C\n");

    // When the window is asked to close and the user cancels, Then it stays open
    app.window().dispatch_event(WindowEvent::CloseRequested);
    assert_eq!(app.get_confirm_text(), "Discard unsaved changes?");
    click(&app, &find(&app, "Cancel").unwrap());
    assert!(app.window().is_visible());

    // When asked again and discarding, Then it closes
    app.window().dispatch_event(WindowEvent::CloseRequested);
    click(&app, &find(&app, "Discard").unwrap());
    assert!(!app.window().is_visible());
}

#[test]
fn opening_a_file_that_is_open_already_shows_its_tab() {
    // Given two song files, the first opened from the start screen, the second in a tab of its own
    let (app, state) = start_screen();
    let dir = temp_dir("open-twice");
    let (blues, waltz) = (song_file(&dir, "Blues"), song_file(&dir, "Waltz"));
    assert!(open(&app, &mut state.borrow_mut(), &blues));
    assert_eq!(tab_labels(&app), ["Blues"]);
    assert!(open(&app, &mut state.borrow_mut(), &waltz));
    assert_eq!((tab_labels(&app), app.get_active_tab()), (vec!["Blues".to_string(), "Waltz".to_string()], 1));

    // When opening the first one again
    assert!(open(&app, &mut state.borrow_mut(), &blues));

    // Then its tab comes to the front, no tab added
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!((app.get_tabs().row_count(), app.get_active_tab()), (2, 0));
    assert_eq!(app.get_window_title(), "Blues — madsister");
}
