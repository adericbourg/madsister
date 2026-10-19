//! The window driven like a user would: key events and clicks on a headless Slint backend.

use i_slint_backend_testing::ElementHandle;
use slint::Model;
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
    run_deferred();
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
    let sha256 = "6ed8919ce20490a5e3ad8630a4fab69475297abd07db73918dd5f36fcfaeb11b"; // printf audio | sha256sum
    song.audio = Some(madsister::model::song::Audio { path: "gone.mp3".into(), sha256: sha256.into(), extra });
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

    // When the transcribed audio file is located next to the song (what the dialog's pick does)
    let found = dir.join("found.mp3");
    std::fs::write(&found, "audio").unwrap();
    state.borrow_mut().tab_mut().audio.locating = Some((found.clone(), hash_in_background(found.clone())));
    settle();

    // Then the song points to it, relative to the song, keeping the unknown field, and the notice gives way to the player
    let audio = state.borrow().tab().editor.as_ref().unwrap().song().audio.clone().unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!((audio.path.as_str(), audio.extra["future"].as_i64()), ("found.mp3", Some(1)));
    assert_eq!(audio.sha256, sha256);
    assert!(app.get_confirm_text().is_empty() && find(&app, notice).is_none() && find(&app, "Playback").is_some());
}

#[test]
fn locating_another_audio_file_than_the_transcribed_one_asks_first() {
    // Given a song file whose audio file isn't there
    let (app, state) = start_screen();
    let mut song = empty_song(Meter { beats: 4, unit: 4, extra: Default::default() }, None);
    song.audio = Some(madsister::model::song::Audio { path: "gone.mp3".into(), sha256: "abc".into(), extra: Default::default() });
    let dir = temp_dir("other-audio");
    let path = dir.join("song.madsister.json");
    std::fs::write(&path, madsister::model::song::serialize_song(&song)).unwrap();
    let settle = || {
        while state.borrow().tab().audio.hashing.is_some() || state.borrow().tab().audio.locating.is_some() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            poll_audio(&app, &mut state.borrow_mut());
        }
        tick(&app, &state.borrow());
    };
    assert!(open(&app, &mut state.borrow_mut(), &path));
    settle();
    let other = dir.join("other.mp3");
    std::fs::write(&other, "audio").unwrap();
    let locate = || {
        state.borrow_mut().tab_mut().audio.locating = Some((other.clone(), hash_in_background(other.clone())));
        settle();
    };
    let audio = || state.borrow().tab().editor.as_ref().unwrap().song().audio.clone().unwrap();

    // When locating a file with another sha256, Then it asks, and Cancel keeps the song as it was
    locate();
    assert_eq!(app.get_confirm_text(), "other.mp3 isn't the file that was transcribed. Use it anyway?");
    click(&app, &find(&app, "Cancel").unwrap());
    assert_eq!((audio().path.as_str(), audio().sha256.as_str()), ("gone.mp3", "abc"));

    // When locating it again and accepting, Then the song points to it, keeping the transcribed file's sha256, so the
    // "changed" notice shows with the player
    locate();
    click(&app, &find(&app, "Use it").unwrap());
    settle();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(app.get_confirm_text(), "");
    assert_eq!((audio().path.as_str(), audio().sha256.as_str()), ("other.mp3", "abc"));
    assert!(find(&app, "Audio file has changed since the transcription: other.mp3").is_some());
    assert!(find(&app, "Playback").is_some());
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

#[test]
fn saving_writes_the_song_file_and_edits_are_autosaved_after_two_seconds() {
    // Given a song file opened, switched to edit mode, with a chord typed
    let (app, state) = start_screen();
    let dir = temp_dir("save");
    let path = song_file(&dir, "Blues");
    assert!(open(&app, &mut state.borrow_mut(), &path));
    with_ctrl(&app, "e");
    run_deferred();
    type_keys(&app, "C\n");
    assert_eq!(tab_labels(&app), ["•Blues"]);

    // When saving with Ctrl+S
    with_ctrl(&app, "s");
    run_deferred();

    // Then the file has the chord and the tab is clean
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("\"C:maj\""));
    assert_eq!(parse_song(&saved).unwrap(), *state.borrow().tab().editor.as_ref().unwrap().song());
    assert_eq!(tab_labels(&app), ["Blues"]);
    assert_eq!(app.get_window_title(), "Blues — madsister");

    // When typing another chord, Then it's written 2 s after the last edit, not before
    type_keys(&app, "G\n");
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(1990));
    assert!(!std::fs::read_to_string(&path).unwrap().contains("\"G:maj\""));
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(20));
    let autosaved = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(autosaved.contains("\"G:maj\""));
    assert_eq!(tab_labels(&app), ["Blues"]);
}

#[test]
fn a_failed_save_says_why_and_keeps_the_changes() {
    // Given an edited song whose folder is gone
    let (app, state) = window();
    type_keys(&app, "C\n");
    let path = std::env::temp_dir().join(format!("madsister-gone-{}", std::process::id())).join("song.madsister.json");
    state.borrow_mut().tab_mut().path = Some(path.clone());

    // When saving
    with_ctrl(&app, "s");
    run_deferred();

    // Then the error is shown and the tab still has unsaved changes
    assert!(app.get_error().starts_with(&format!("Can't save {}", path.display())));
    assert!(find(&app, &app.get_error()).is_some());
    assert_eq!(tab_labels(&app), ["•Untitled"]);
}

#[test]
fn saving_a_copy_in_another_folder_keeps_its_audio_file() {
    // Given a song whose audio is next to it
    let (app, state) = window();
    let dir = temp_dir("save-copy");
    std::fs::create_dir_all(dir.join("copies")).unwrap();
    {
        let mut state = state.borrow_mut();
        let tab = state.tab_mut();
        let mut song = tab.editor.as_ref().unwrap().song().clone();
        song.audio = Some(madsister::model::song::Audio { path: "a.mp3".into(), sha256: "abc".into(), extra: Default::default() });
        tab.editor.as_mut().unwrap().history.apply(song);
        tab.path = Some(dir.join("song.madsister.json"));
    }

    // When saving it in a subfolder
    let copy = dir.join("copies").join("song.madsister.json");
    save_to(state.borrow_mut().tab_mut(), copy.clone());
    refresh(&app, &mut state.borrow_mut());

    // Then the copy points to the same audio file
    let saved = parse_song(&std::fs::read_to_string(&copy).unwrap()).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(saved.audio.unwrap().path, "../a.mp3");
    assert_eq!(tab_labels(&app), ["Untitled"]);
}

#[test]
fn a_failed_save_as_keeps_the_audio_path_of_the_song_file() {
    // Given a song whose audio is next to it
    let (_app, state) = window();
    let dir = temp_dir("failed-copy");
    let mut state = state.borrow_mut();
    let tab = state.tab_mut();
    let mut song = tab.editor.as_ref().unwrap().song().clone();
    song.audio = Some(madsister::model::song::Audio { path: "a.mp3".into(), sha256: "abc".into(), extra: Default::default() });
    tab.editor.as_mut().unwrap().history.apply(song);
    tab.path = Some(dir.join("song.madsister.json"));

    // When saving it in a folder that doesn't exist
    save_to(tab, dir.join("gone").join("song.madsister.json"));

    // Then the song and its file are unchanged
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(tab.error.starts_with("Can't save"));
    assert_eq!(tab.editor.as_ref().unwrap().song().audio.as_ref().unwrap().path, "a.mp3");
    assert_eq!(tab.path, Some(dir.join("song.madsister.json")));
}

#[test]
fn opened_files_are_listed_as_recent_and_saved_in_the_config_folder() {
    // Given two song files of the same name in two folders, and a config folder
    let (app, state) = start_screen();
    let dir = temp_dir("recent");
    state.borrow_mut().config = Some(dir.join("config"));
    let (a, b) = (dir.join("a"), dir.join("b"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let (first, second) = (song_file(&a, "Blues"), song_file(&b, "Blues"));

    // When opening both
    assert!(open(&app, &mut state.borrow_mut(), &first));
    assert_eq!(app.get_recent().iter().collect::<Vec<_>>(), ["Blues.madsister.json"]);
    assert!(open(&app, &mut state.borrow_mut(), &second));

    // Then the menu tells them apart, most recent first, and recent.json lists them as the Tauri app wrote it
    assert_eq!(app.get_recent().iter().collect::<Vec<_>>(), ["b/Blues.madsister.json", "a/Blues.madsister.json"]);
    let saved = std::fs::read_to_string(dir.join("config").join("recent.json")).unwrap();
    assert_eq!(saved, serde_json::to_string(&[second.display().to_string(), first.display().to_string()]).unwrap());

    // When the second file is gone, closed and reopened from Open recent
    with_ctrl(&app, "w");
    run_deferred();
    std::fs::remove_file(&second).unwrap();
    app.invoke_open_recent(0);
    run_deferred();

    // Then the error is shown and it leaves the recent files
    assert!(app.get_error().starts_with(&format!("Can't open {}", second.display())));
    assert_eq!(app.get_recent().iter().collect::<Vec<_>>(), ["Blues.madsister.json"]);

    // When opening the first one from Open recent, Then it comes to the front and the error goes
    app.invoke_open_recent(0);
    run_deferred();
    assert_eq!((tab_labels(&app), app.get_error()), (vec!["Blues".to_string()], "".into()));

    // When clearing the recent files, Then the list is empty, on disk too
    app.invoke_app_action(AppAction::ClearRecent);
    run_deferred();
    let saved = std::fs::read_to_string(dir.join("config").join("recent.json")).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!((app.get_recent().row_count(), saved.as_str()), (0, "[]"));
}

#[test]
fn opening_an_invalid_song_file_says_why_and_keeps_the_open_song() {
    // Given an open song, and a file that isn't one
    let (app, state) = window();
    let dir = temp_dir("invalid");
    let bad = dir.join("bad.madsister.json");
    std::fs::write(&bad, r#"{"version": 1}"#).unwrap();

    // When opening it
    let opened = open(&app, &mut state.borrow_mut(), &bad);

    // Then the error is shown in the window and the song stays
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!opened);
    assert!(app.get_error().contains("meta"));
    assert_eq!(tab_labels(&app), ["Untitled"]);
    assert!(app.get_has_song());
}

/// The start screen with the given config folder, its previous session restored.
fn relaunch(config: &Path, reopen_tabs: bool) -> (AppWindow, Rc<RefCell<State>>) {
    let (app, state) = start_screen();
    let mut state_mut = state.borrow_mut();
    state_mut.config = Some(config.to_path_buf());
    state_mut.settings.reopen_tabs = reopen_tabs;
    restore_session(&mut state_mut);
    show_tab(&app, &mut state_mut);
    drop(state_mut);
    (app, state)
}

#[test]
fn at_launch_the_tabs_of_the_previous_session_that_still_have_their_file_come_back() {
    // Given a session of three files, the second one active, and the third gone
    let dir = temp_dir("session");
    let (blues, waltz) = (song_file(&dir, "Blues"), song_file(&dir, "Waltz"));
    let gone = dir.join("gone.madsister.json");
    let path = |p: &Path| p.display().to_string();
    std::fs::write(dir.join("session.json"), serde_json::json!({ "paths": [path(&blues), path(&waltz), path(&gone)], "active": 1 }).to_string())
        .unwrap();

    // When the app starts
    let (app, state) = relaunch(&dir, true);

    // Then the two remaining songs are back in their tabs, the second in front, without any error
    assert_eq!((tab_labels(&app), app.get_active_tab()), (vec!["Blues".to_string(), "Waltz".to_string()], 1));
    assert!(app.get_has_song() && app.get_error().is_empty());

    // When adding a new grid, then coming back to the first song
    with_ctrl(&app, "n");
    run_deferred();
    click(&app, &find(&app, "Create grid").unwrap());
    click(&app, &find(&app, "Blues").unwrap());

    // Then the session keeps the songs that have a file, the first one active
    let session: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("session.json")).unwrap()).unwrap();
    drop(state);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(session, serde_json::json!({ "paths": [path(&blues), path(&waltz)], "active": 0 }));
}

#[test]
fn at_launch_with_reopen_tabs_off_the_start_screen_shows_without_tabs() {
    // Given a previous session, and the setting turned off
    let dir = temp_dir("session-off");
    let blues = song_file(&dir, "Blues");
    std::fs::write(dir.join("session.json"), serde_json::json!({ "paths": [blues.display().to_string()], "active": 0 }).to_string()).unwrap();

    // When the app starts
    let (app, _state) = relaunch(&dir, false);

    // Then
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(tab_labels(&app), ["New tab"]);
    assert!(!app.get_has_song());
}

#[test]
fn each_export_writes_its_format_and_a_failure_shows_on_the_tab() {
    // Given a song with a chord
    let (app, state) = window();
    type_keys(&app, "C\n");
    let dir = temp_dir("export");

    // When exporting it in each format
    let mut state = state.borrow_mut();
    let state = &mut *state;
    let song = state.tabs[0].editor.as_ref().unwrap().song().clone();
    let starts = [
        (ExportFormat::ChordPro, "cho", &b"{title: Untitled}"[..]),
        (ExportFormat::MusicXml, "musicxml", b"<?xml"),
        (ExportFormat::Midi, "mid", b"MThd"),
        (ExportFormat::Pdf, "pdf", b"%PDF"),
    ];
    for (format, extension, start) in starts {
        let (_, ext, contents) = export_file(format, &song, &state.settings).unwrap();
        let path = dir.join(format!("Untitled.{ext}"));
        save_export(&mut state.tabs[0], &path, &contents);

        // Then each file is written with its format's extension and start
        assert_eq!(ext, extension);
        assert!(std::fs::read(&path).unwrap().starts_with(start), "{ext}");
    }

    // When the folder is gone, Then the tab says why
    std::fs::remove_dir_all(&dir).unwrap();
    save_export(&mut state.tabs[0], &dir.join("Untitled.cho"), b"");
    refresh(&app, state);
    assert!(app.get_error().starts_with("Can't export"));
}

#[test]
fn the_exports_are_locked_for_a_nashville_song_without_a_key() {
    // Given a new Nashville grid
    let (app, _state) = start_screen();
    assert!(!app.get_has_song());
    app.invoke_create_grid(4, 4, true);

    // Then it can't be exported until it has a key
    assert!(app.get_is_export_locked());
    app.invoke_meta_committed(MetaChoice::Key, "A".into());
    assert!(!app.get_is_export_locked());
}

#[test]
fn the_edit_menu_undoes_in_edit_mode_only_and_the_view_menu_switches_modes() {
    // Given a chord typed in a new grid (edit mode)
    let (app, state) = window();
    type_keys(&app, "C\n");
    assert!(app.get_is_editing());

    // When undoing from the Edit menu, then redoing, Then the chord goes and comes back
    app.invoke_app_action(AppAction::Undo);
    run_deferred();
    assert_eq!(chords(&state, 0), ["N"]);
    app.invoke_app_action(AppAction::Redo);
    run_deferred();
    assert_eq!(chords(&state, 0), ["C:maj"]);

    // When switching to view mode from the menu, Then undo does nothing
    app.invoke_app_action(AppAction::EditMode);
    run_deferred();
    assert!(!app.get_is_editing() && !app.global::<Grid>().get_is_editable());
    app.invoke_app_action(AppAction::Undo);
    run_deferred();
    assert_eq!(chords(&state, 0), ["C:maj"]);

    // When the menu toggles its checkmark while a question is open, Then the action is ignored and the mark comes back
    state.borrow_mut().pending = Some(Pending::CloseWindow);
    app.set_is_editing(true);
    app.invoke_app_action(AppAction::EditMode);
    run_deferred();
    assert!(!app.get_is_editing());
}

#[test]
fn quitting_with_unsaved_changes_asks_first() {
    // Given an unsaved chord
    let (app, _state) = window();
    type_keys(&app, "C\n");

    // When quitting from the menu, Then it asks, and discarding closes the window
    with_ctrl(&app, "q");
    run_deferred();
    assert_eq!(app.get_confirm_text(), "Discard unsaved changes?");
    click(&app, &find(&app, "Discard").unwrap());
    assert!(!app.window().is_visible());
}

#[test]
fn the_preferences_are_saved_keeping_unknown_keys_and_applied_at_once() {
    // Given a song of 4 bars, and settings.json with a key this app doesn't know
    let (app, state) = window();
    let dir = temp_dir("preferences");
    std::fs::write(dir.join("settings.json"), r#"{"barsPerRow": 4, "theme": "dark"}"#).unwrap();
    state.borrow_mut().config = Some(dir.clone());
    assert_eq!(app.get_chart().sections.row_data(0).unwrap().rows.row_count(), 1);

    // When opening the preferences with Ctrl+, then closing them with Escape
    with_ctrl(&app, ",");
    run_deferred();
    assert!(find(&app, "Bars per row").is_some());
    press(&app, Key::Escape);

    // Then nothing is saved
    assert!(find(&app, "Bars per row").is_none());
    assert!(!std::fs::read_to_string(dir.join("settings.json")).unwrap().contains("font"));

    // When the dialog saves 2 bars per row and Kalam
    with_ctrl(&app, ",");
    run_deferred();
    let view = PreferencesView { bars_per_row: 2, font: 0, ..app.get_preferences() };
    app.invoke_preferences_saved(view);

    // Then the dialog closes, the chart has 2 rows of 2 bars in Kalam, and settings.json has them with the unknown key
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(app.get_dialog(), Dialog::None);
    assert_eq!(app.get_chart().sections.row_data(0).unwrap().rows.row_count(), 2);
    assert_eq!(app.global::<ChartFont>().get_family(), "Kalam");
    assert_eq!((&saved["barsPerRow"], &saved["font"], &saved["theme"]), (&2.into(), &"kalam".into(), &"dark".into()));
    assert_eq!(Settings::parse(&saved), state.borrow().settings);
}

#[test]
fn the_menu_shortcuts_wait_while_a_dialog_is_open() {
    // Given a song, the preferences open
    let (app, _state) = window();
    with_ctrl(&app, ",");
    run_deferred();

    // When pressing Ctrl+N, Then no tab opens
    with_ctrl(&app, "n");
    run_deferred();
    assert_eq!(app.get_tabs().row_count(), 1);
}

#[test]
fn about_opens_from_the_help_menu_and_closes() {
    // Given the start screen
    let (app, _state) = start_screen();

    // When opening Help > About
    app.invoke_app_action(AppAction::About);
    run_deferred();

    // Then it credits the author, with the models' links as buttons; Close closes it
    assert!(find(&app, "Alban Dericbourg").is_some());
    assert_eq!(find(&app, "madmom").and_then(|e| e.accessible_role()), Some(i_slint_backend_testing::AccessibleRole::Button));
    click(&app, &find(&app, "Close").unwrap());
    assert_eq!(app.get_dialog(), Dialog::None);
}

#[test]
fn the_sidebar_collapses_to_a_rail_remembered_in_the_settings() {
    // Given a song in edit mode, and a config folder
    let (app, state) = window();
    let dir = temp_dir("sidebar");
    state.borrow_mut().config = Some(dir.clone());
    let is_collapsed_saved = || {
        let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
        saved["isSidebarCollapsed"].as_bool().unwrap()
    };

    // When hiding the parameters
    click(&app, &find(&app, "Hide parameters").unwrap());
    run_deferred();

    // Then the panels give way to the rail's buttons, every panel's in edit mode, and it's saved
    assert!(!app.get_is_sidebar_shown() && is_collapsed_saved());
    assert!(ElementHandle::find_by_accessible_label(&app, "Title").next().is_none());
    assert!(find(&app, "Tempo").is_some() && find(&app, "Show parameters").is_some());
    assert!(find(&app, "Show parameters").unwrap().size().width < 60.0, "the chart takes the room");

    // When clicking the rail's Transpose button, Then the sidebar is back, saved too
    click(&app, &find(&app, "Transpose").unwrap());
    assert!(app.get_is_sidebar_shown() && !is_collapsed_saved());
    assert!(find(&app, "+1 semitone").is_some());

    // When View > Show sidebar unchecks it, then the chart's title is clicked, Then the sidebar expands on its field
    app.invoke_app_action(AppAction::Sidebar);
    run_deferred();
    assert!(!app.get_is_sidebar_shown());
    app.global::<Grid>().invoke_meta_clicked(MetaChoice::Title);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(app.get_is_sidebar_shown());
}

#[test]
fn the_ribbon_opens_tabs_switches_modes_and_opens_the_preferences() {
    // Given the start screen, Then the song actions are disabled and there's no mode switch
    let (app, state) = start_screen();
    assert!(!find(&app, "Save").unwrap().accessible_enabled().unwrap());
    assert!(find(&app, "Edit mode").is_none());

    // When creating a grid, then switching to view mode from the ribbon
    click(&app, &find(&app, "Create grid").unwrap());
    assert!(find(&app, "Save").unwrap().accessible_enabled().unwrap());
    click(&app, &find(&app, "Edit mode").unwrap());
    run_deferred();

    // Then the song is read-only
    assert!(!state.borrow().tab().editor.as_ref().unwrap().is_editable);
    assert_eq!(find(&app, "Edit mode").and_then(|e| e.accessible_checked()), Some(false));

    // When clicking New, then Preferences, Then a tab opens, then the dialog
    click(&app, &find(&app, "New").unwrap());
    run_deferred();
    assert_eq!(app.get_tabs().row_count(), 2);
    click(&app, &find(&app, "Preferences").unwrap());
    run_deferred();
    assert_eq!(app.get_dialog(), Dialog::Preferences);
}
