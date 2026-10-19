use super::*;
use crate::model::song::{ChordSlot, Meter, empty_song, parse_song};

fn settings() -> Settings {
    Settings::parse(&serde_json::Value::Null)
}

fn editor() -> Editor {
    Editor::new(empty_song(Meter { beats: 4, unit: 4, extra: Default::default() }, None))
}

fn key(k: &str) -> KeyInput<'_> {
    KeyInput { key: k, ..Default::default() }
}

fn ctrl(k: &str) -> KeyInput<'_> {
    KeyInput { ctrl: true, ..key(k) }
}

/// What the UI does with a key: the inline input gets it while open (typed characters go in its text), else the grid.
fn press(editor: &mut Editor, k: KeyInput) {
    let settings = settings();
    if editor.draft.is_some() {
        if !editor.draft_key(k, &settings) && k.key.chars().count() == 1 {
            let text = format!("{}{}", editor.draft.as_ref().unwrap().text, k.key);
            editor.set_draft_text(&text);
        }
    } else {
        editor.key(k, &settings);
    }
}

/// Types each character, `\n` being Enter.
fn type_keys(editor: &mut Editor, keys: &str) {
    for c in keys.chars() {
        let k = c.to_string();
        press(editor, key(if c == '\n' { "Enter" } else { &k }));
    }
}

fn bar(editor: &Editor, section: usize, bar: usize) -> Vec<(String, u32)> {
    editor.song().sections[section].bars[bar].chords.iter().map(|c: &ChordSlot| (c.chord.clone(), c.beats)).collect()
}

fn chords(slots: &[(&str, u32)]) -> Vec<(String, u32)> {
    slots.iter().map(|(c, b)| (c.to_string(), *b)).collect()
}

const fn at(section: usize, bar: usize, slot: usize) -> SlotRef {
    SlotRef { section, bar, slot }
}

#[test]
fn typing_a_chart_by_keyboard_builds_the_song() {
    // Given an empty song (Verse, 4 empty bars) with the cursor on the first slot
    let mut ed = editor();

    // When typing the verse, splitting the last bar 2+2, with an invalid chord at the end
    type_keys(&mut ed, "C\nG\nAm\n/F\nGx\n");

    // Then the invalid chord is rejected inline and nothing changes
    let draft = ed.draft.clone().unwrap();
    assert_eq!(draft.error.as_deref(), Some("unknown chord quality \"x\""));
    assert_eq!(bar(&ed, 0, 3), chords(&[("F:maj", 2), ("N", 2)]));

    // When correcting it, then adding 4 bars, going back to the first new one and splitting the section there
    ed.set_draft_text("G");
    press(&mut ed, key("Enter"));
    for _ in 0..4 {
        press(&mut ed, ctrl("Enter"));
    }
    for _ in 0..3 {
        press(&mut ed, key("ArrowLeft"));
    }
    press(&mut ed, ctrl("k"));

    // And typing the chorus, undoing and redoing the last chord
    type_keys(&mut ed, "F\nC\nG");
    press(&mut ed, key("Tab"));
    type_keys(&mut ed, "G\n");
    press(&mut ed, ctrl("z"));
    assert_eq!(bar(&ed, 1, 3), chords(&[("N", 4)]));
    press(&mut ed, KeyInput { shift: true, ..ctrl("Z") });

    // And renaming the section and repeating it twice
    press(&mut ed, key("F2"));
    assert_eq!(ed.draft.as_ref().map(|d| (d.kind, d.text.as_str())), Some((DraftKind::Label, "Verse (2)")));
    ed.set_draft_text("Chorus");
    press(&mut ed, key("Enter"));
    press(&mut ed, KeyInput { alt: true, ..key("ArrowUp") });

    // Then the song is the expected chart
    let song = ed.song();
    let labels: Vec<(&str, Option<u32>)> = song.sections.iter().map(|s| (s.label.as_str(), s.repeat)).collect();
    assert_eq!(labels, [("Verse", None), ("Chorus", Some(2))]);
    assert_eq!(
        (0..4).map(|b| bar(&ed, 0, b)).collect::<Vec<_>>(),
        [chords(&[("C:maj", 4)]), chords(&[("G:maj", 4)]), chords(&[("A:min", 4)]), chords(&[("F:maj", 2), ("G:maj", 2)])]
    );
    assert_eq!(
        (0..4).map(|b| bar(&ed, 1, b)).collect::<Vec<_>>(),
        [chords(&[("F:maj", 4)]), chords(&[("C:maj", 4)]), chords(&[("G:maj", 4)]), chords(&[("G:maj", 4)])]
    );
    assert_eq!(ed.cursor(), at(1, 3, 0));

    // When opening the help, Then it is open
    press(&mut ed, KeyInput { shift: true, ..key("?") });
    assert!(ed.is_help_open);

    // When merging the first slot with the previous one (impossible), Then the refusal is announced, and cleared by a move
    press(&mut ed, key("Backspace"));
    assert_eq!(ed.notice, "Not possible here");
    press(&mut ed, key("ArrowLeft"));
    assert_eq!(ed.notice, "");
}

#[test]
fn in_view_mode_the_editor_navigates_without_editing() {
    // Given a song in view mode
    let mut ed = editor();
    type_keys(&mut ed, "C\nG\n");
    ed.is_editable = false;
    let before = ed.song().clone();

    // When clicking a bar, Then the cursor moves there and no input opens
    ed.click(at(0, 1, 0), &settings());
    assert_eq!((ed.cursor(), ed.draft.is_none()), (at(0, 1, 0), true));

    // When typing, using an edit shortcut or undoing, Then the song is untouched and the refusal is announced
    for k in [key("A"), ctrl("d"), ctrl("z")] {
        press(&mut ed, k);
    }
    assert_eq!(ed.song(), &before);
    assert!(ed.notice.starts_with("Read-only"), "{}", ed.notice);

    // When moving with the arrows, Then navigation still works
    press(&mut ed, key("ArrowLeft"));
    assert_eq!(ed.cursor(), at(0, 0, 0));
}

#[test]
fn clicking_a_slot_edits_it_in_place_until_enter_blur_or_escape() {
    // Given a song starting with C then G
    let mut ed = editor();
    type_keys(&mut ed, "C\nG\n");
    let s = settings();

    // When clicking the first slot, Then it is edited with its whole text selected
    ed.click(at(0, 0, 0), &s);
    assert_eq!(ed.draft.as_ref().map(|d| (d.text.as_str(), d.select_all)), Some(("C", true)));

    // When replacing the text and clicking another slot, Then it is saved and the other slot is edited
    ed.set_draft_text("D");
    ed.click(at(0, 1, 0), &s);
    assert_eq!(bar(&ed, 0, 0), chords(&[("D:maj", 4)]));
    assert_eq!(ed.draft.as_ref().map(|d| d.text.as_str()), Some("G"));

    // When typing then pressing Escape, Then nothing is saved
    ed.set_draft_text("A");
    press(&mut ed, key("Escape"));
    assert!(ed.draft.is_none());
    assert_eq!(bar(&ed, 0, 1), chords(&[("G:maj", 4)]));

    // When typing then leaving the input, Then it is saved and the cursor stays
    ed.click(at(0, 1, 0), &s);
    ed.set_draft_text("Em");
    ed.commit(&s, true);
    assert_eq!((bar(&ed, 0, 1), ed.cursor()), (chords(&[("E:min", 4)]), at(0, 1, 0)));

    // When leaving a slot unchanged, Then there's no undo entry
    let before = ed.song().clone();
    ed.click(at(0, 0, 0), &s);
    ed.commit(&s, true);
    ed.history.undo();
    assert_eq!(bar(&ed, 0, 1), chords(&[("G:maj", 4)]), "the undo went past the unchanged slot");
    ed.history.redo();
    assert_eq!(ed.song(), &before);

    // When leaving with an invalid chord, Then the edition is dropped
    ed.click(at(0, 0, 0), &s);
    ed.set_draft_text("Gx");
    ed.commit(&s, true);
    assert!(ed.draft.is_none());
    assert_eq!(ed.song(), &before);
}

#[test]
fn mod_slash_in_the_chord_input_sets_the_chord_and_splits_the_slot() {
    // Given a typed C in the first slot of an empty song
    let mut ed = editor();
    type_keys(&mut ed, "C");

    // When pressing Ctrl+/, Then the slot is split 2+2 and the new half is edited with the chord preselected
    press(&mut ed, ctrl("/"));
    assert_eq!(ed.draft.as_ref().map(|d| (d.text.as_str(), d.select_all)), Some(("C", true)));
    ed.set_draft_text("G");
    press(&mut ed, key("Enter"));
    assert_eq!(bar(&ed, 0, 0), chords(&[("C:maj", 2), ("G:maj", 2)]));

    // When undoing, Then C and the split are one step, after the last G
    press(&mut ed, ctrl("z"));
    assert_eq!(bar(&ed, 0, 0), chords(&[("C:maj", 2), ("C:maj", 2)]));
    press(&mut ed, ctrl("z"));
    assert_eq!(bar(&ed, 0, 0), chords(&[("N", 4)]));

    // When the chord is invalid, Then the error is shown and nothing is split
    ed.move_to(at(0, 0, 0));
    type_keys(&mut ed, "Gx");
    press(&mut ed, ctrl("/"));
    assert_eq!(ed.draft.as_ref().and_then(|d| d.error.as_deref()), Some("unknown chord quality \"x\""));
    assert_eq!(bar(&ed, 0, 0), chords(&[("N", 4)]));

    // When the slot is a single beat, Then the split is refused and the input stays open
    ed.set_draft_text("C");
    press(&mut ed, ctrl("/"));
    ed.set_draft_text("A");
    press(&mut ed, ctrl("/"));
    press(&mut ed, ctrl("/"));
    assert!(ed.draft.is_some());
    assert_eq!(ed.notice, "Not possible here");
}

#[test]
fn clearing_the_chord_of_a_split_slot_removes_the_subdivision() {
    // Given a bar split into C and G
    let mut ed = editor();
    type_keys(&mut ed, "C");
    press(&mut ed, ctrl("/"));
    ed.set_draft_text("G");
    press(&mut ed, key("Enter"));

    // When clearing the second chord and pressing Enter, Then the bar is whole again
    ed.click(at(0, 0, 1), &settings());
    ed.set_draft_text("");
    press(&mut ed, key("Enter"));
    assert_eq!(bar(&ed, 0, 0), chords(&[("C:maj", 4)]));
}

#[test]
fn typing_in_a_nashville_song_reads_degrees_of_the_key() {
    // Given a Nashville song in G
    let song = parse_song(
        r#"{ "version": 1, "meta": { "title": "t", "key": "G", "notation": "nashville", "meter": { "beats": 4, "unit": 4 } },
            "sections": [{ "id": "a", "label": "A", "bars": [{ "chords": [{ "chord": "N", "beats": 4 }] }] }] }"#,
    )
    .unwrap();
    let mut ed = Editor::new(song);

    // When typing 4, Then the chord is C, and clicking it shows 4 again
    type_keys(&mut ed, "4\n");
    assert_eq!(bar(&ed, 0, 0), chords(&[("C:maj", 4)]));
    ed.click(at(0, 0, 0), &settings());
    assert_eq!(ed.draft.as_ref().map(|d| d.text.as_str()), Some("4"));
}

mod menu {
    use super::*;

    /// Verse: C | G D (1 beat each); Chorus: Am.
    fn menu_editor() -> Editor {
        Editor::new(
            parse_song(
                r#"{ "version": 1, "meta": { "title": "t", "meter": { "beats": 2, "unit": 4 } }, "sections": [
                    { "id": "v", "label": "Verse", "bars": [
                        { "chords": [{ "chord": "C:maj", "beats": 2 }] },
                        { "chords": [{ "chord": "G:maj", "beats": 1 }, { "chord": "D:maj", "beats": 1 }] }
                    ] },
                    { "id": "c", "label": "Chorus", "bars": [{ "chords": [{ "chord": "A:min", "beats": 2 }] }] }
                ] }"#,
            )
            .unwrap(),
        )
    }

    fn labels(ed: &Editor) -> Vec<String> {
        ed.song().sections.iter().map(|s| format!("{}:{}", s.label, s.bars.len())).collect()
    }

    const fn bar_at(section: usize, bar: usize) -> BarRef {
        BarRef { section, bar }
    }

    #[test]
    fn bar_items_add_and_remove_bars_and_place_the_cursor() {
        // Given
        let mut ed = menu_editor();

        // When adding an empty bar after the first one, Then the section has one more bar, the cursor on it
        ed.run(MenuAction::AddBarAfter(bar_at(0, 0)));
        assert_eq!((labels(&ed), ed.cursor()), (vec!["Verse:3".into(), "Chorus:1".into()], at(0, 1, 0)));

        // When removing it, Then the section is back to two bars
        ed.run(MenuAction::RemoveBar(bar_at(0, 1)));
        assert_eq!(labels(&ed), ["Verse:2", "Chorus:1"]);

        // When splitting a 2-beat chord, Then the cursor is on its second half
        ed.run(MenuAction::Split(at(0, 0, 0)));
        assert_eq!((bar(&ed, 0, 0).len(), ed.cursor()), (2, at(0, 0, 1)));
    }

    #[test]
    fn line_and_bar_items_split_the_section() {
        // Given / When splitting on the line between the two Verse bars
        let mut ed = menu_editor();
        ed.run(MenuAction::SplitSectionAt(bar_at(0, 1)));

        // Then
        assert_eq!(labels(&ed), ["Verse:1", "Verse (2):1", "Chorus:1"]);
        assert_eq!(ed.cursor(), at(1, 0, 0));

        // When splitting before/after a bar and adding a bar at the closing line
        let mut ed = menu_editor();
        ed.run(MenuAction::SplitSectionAfter(bar_at(0, 0)));
        assert_eq!(labels(&ed), ["Verse:1", "Verse (2):1", "Chorus:1"]);
        let mut ed = menu_editor();
        ed.run(MenuAction::AddBarAt(bar_at(0, 2)));
        assert_eq!(labels(&ed), ["Verse:3", "Chorus:1"]);
    }

    #[test]
    fn merge_with_previous_joins_the_sections_and_a_refused_item_is_announced() {
        // Given / When merging the Chorus into the Verse
        let mut ed = menu_editor();
        ed.run(MenuAction::MergeWithPrevious(1));

        // Then the cursor is on the first merged bar
        assert_eq!((labels(&ed), ed.cursor()), (vec!["Verse:3".into()], at(0, 2, 0)));

        // When merging the first section, Then nothing changes and the reason is shown
        ed.run(MenuAction::MergeWithPrevious(0));
        assert_eq!(labels(&ed), ["Verse:3"]);
        assert!(!ed.notice.is_empty());
    }
}

mod panels {
    use super::*;
    use crate::model::song::Notation;
    use crate::model::transpose::Spelling;

    fn two_sections() -> Editor {
        Editor::new(
            parse_song(
                r#"{ "version": 1, "meta": { "title": "Song", "meter": { "beats": 4, "unit": 4 } }, "sections": [
                    { "id": "i", "label": "Intro", "bars": [{ "chords": [{ "chord": "N", "beats": 4 }] }] },
                    { "id": "v", "label": "Verse", "bars": [{ "chords": [{ "chord": "C:maj", "beats": 4 }] }] }
                ] }"#,
            )
            .unwrap(),
        )
    }

    fn labels(ed: &Editor) -> Vec<(String, Option<u32>)> {
        ed.song().sections.iter().map(|s| (s.label.clone(), s.repeat)).collect()
    }

    #[test]
    fn the_section_controls_edit_the_cursor_section() {
        // Given an empty intro (cursor there) and a verse with a chord
        let mut ed = two_sections();

        // When renaming the intro, repeating it 3 times and moving it down
        ed.rename_cursor_section("Outro").unwrap();
        ed.set_cursor_repeat(3);
        ed.move_cursor_section(Step::Forward);

        // Then the order changes and the cursor follows the moved section
        assert_eq!(labels(&ed), [("Verse".into(), None), ("Outro".into(), Some(3))]);
        assert_eq!(ed.cursor().section, 1);
        assert_eq!(ed.rename_cursor_section(" "), Err("a section needs a name".into()));

        // When deleting the outro (no chords), then the verse (chords: the UI asks first)
        assert!(!ed.cursor_section_has_chords());
        ed.delete_cursor_section();
        assert_eq!(labels(&ed), [("Verse".into(), None)]);
        assert!(ed.cursor_section_has_chords());
        ed.delete_cursor_section();
        assert_eq!(labels(&ed), [("Song".into(), None)]);
    }

    #[test]
    fn the_bar_control_overrides_the_cursor_bar_meter() {
        // Given an empty 4/4 song
        let mut ed = editor();
        assert_eq!(ed.cursor_bar_meter().map(|m| m.beats), Some(4));

        // When setting 3 then 4 beats, Then the bar goes 3/4, then back to the song meter
        ed.set_cursor_bar_beats(3);
        assert_eq!(ed.song().sections[0].bars[0].meter.as_ref().map(|m| m.beats), Some(3));
        ed.set_cursor_bar_beats(4);
        assert_eq!(ed.song().sections[0].bars[0].meter, None);
    }

    #[test]
    fn the_song_fields_are_checked_and_the_key_moves_nashville_chords() {
        // Given
        let mut ed = editor();
        let s = settings();

        // Then invalid values are refused with a message, valid ones set
        assert_eq!(ed.set_meta(MetaField::Title, " ", &s), Err("a song needs a title".into()));
        assert!(ed.set_meta(MetaField::Key, "H", &s).is_err());
        assert!(ed.set_meta(MetaField::Tempo, "-3", &s).is_err());
        ed.set_meta(MetaField::Title, "Blue", &s).unwrap();
        ed.set_meta(MetaField::Key, "F#m", &s).unwrap();
        ed.set_meta(MetaField::Tempo, "96", &s).unwrap();
        let meta = &ed.song().meta;
        assert_eq!((meta.title.as_str(), meta.key.as_deref(), meta.tempo_bpm), ("Blue", Some("F#m"), Some(96.0)));

        // And an empty optional field removes it
        ed.set_meta(MetaField::Tempo, "", &s).unwrap();
        assert_eq!(ed.song().meta.tempo_bpm, None);

        // And the notation needs the key
        ed.set_notation(Notation::Nashville);
        assert_eq!(ed.song().meta.notation, Some(Notation::Nashville));
    }

    #[test]
    fn transposing_counts_the_offset_and_undo_restores_it() {
        // Given C in the first bar
        let mut ed = editor();
        type_keys(&mut ed, "C\n");

        // When transposing up twice, Then D and an offset of 2
        ed.transpose(1, Spelling::Sharp);
        ed.transpose(1, Spelling::Sharp);
        assert_eq!((bar(&ed, 0, 0), ed.history.present().transposition), (chords(&[("D:maj", 4)]), 2));

        // When undoing, Then C# and 1
        ed.history.undo();
        assert_eq!((bar(&ed, 0, 0), ed.history.present().transposition), (chords(&[("C#:maj", 4)]), 1));
    }

    #[test]
    fn phase_and_tempo_buttons_act_from_the_cursor_or_on_the_whole_song() {
        // Given 4 bars at 120 BPM, the cursor on bar 4
        let mut ed = editor();
        type_keys(&mut ed, "Am\nC\nG\nF\n");
        ed.set_meta(MetaField::Tempo, "120", &settings()).unwrap();
        ed.click(at(0, 3, 0), &settings());
        ed.draft = None;

        // When doubling from the cursor bar, Then only bar 4 is split and the tempo stays
        ed.fix_tempo(false, true);
        assert_eq!((ed.song().sections[0].bars.len(), ed.song().meta.tempo_bpm), (5, Some(120.0)));

        // When halving the whole song, Then bars merge two by two and the tempo halves
        ed.history.undo();
        ed.fix_tempo(true, false);
        assert_eq!((ed.song().sections[0].bars.len(), ed.song().meta.tempo_bpm), (2, Some(60.0)));

        // When moving the whole song's bar lines one beat later, Then a pickup bar appears
        ed.history.undo();
        ed.shift(Step::Forward, false);
        assert_eq!(bar(&ed, 0, 0), chords(&[("A:min", 1)]));
    }
}
