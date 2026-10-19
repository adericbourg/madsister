use serde_json::json;

use super::*;
use crate::model::commands::{delete_bar, insert_bar, move_section, paste_bars, set_bar_meter, set_repeat, split_slot};
use crate::model::song::parse_song;

fn song_of(file: serde_json::Value) -> Song {
    parse_song(&file.to_string()).unwrap()
}

/// Verse: C | F G | Am; Chorus x2: N; Bridge: empty.
fn song() -> Song {
    song_of(json!({
        "version": 1,
        "meta": { "title": "t", "meter": { "beats": 4, "unit": 4 } },
        "sections": [
            { "id": "a", "label": "Verse", "bars": [
                { "chords": [{ "chord": "C:maj", "beats": 4 }] },
                { "chords": [{ "chord": "F:maj", "beats": 2 }, { "chord": "G:maj", "beats": 2 }] },
                { "chords": [{ "chord": "A:min", "beats": 4 }] }
            ] },
            { "id": "b", "label": "Chorus", "repeat": 2, "bars": [{ "chords": [{ "chord": "N", "beats": 4 }] }] },
            { "id": "c", "label": "Bridge", "bars": [] }
        ]
    }))
}

const fn at(section: usize, bar: usize, slot: usize) -> SlotRef {
    SlotRef { section, bar, slot }
}

const fn bar_at(section: usize, bar: usize) -> BarRef {
    BarRef { section, bar }
}

fn state<'a>(song: &'a Song, cursor: SlotRef) -> EditorState<'a> {
    EditorState { song, cursor, anchor: None, clipboard: &[], bars_per_row: 2, low_confidence_threshold: 0.5 }
}

fn key(k: &str) -> KeyInput<'_> {
    KeyInput { key: k, ..Default::default() }
}

fn shift(k: &str) -> KeyInput<'_> {
    KeyInput { shift: true, ..key(k) }
}

fn ctrl(k: &str) -> KeyInput<'_> {
    KeyInput { ctrl: true, ..key(k) }
}

fn alt(k: &str) -> KeyInput<'_> {
    KeyInput { alt: true, ..key(k) }
}

/// Where a key moves the cursor, or the command when it isn't a move.
fn move_to(k: KeyInput, s: &EditorState) -> Result<SlotRef, Option<Command>> {
    match key_to_command(k, s) {
        Some(Command::Move { cursor, .. }) => Ok(cursor),
        other => Err(other),
    }
}

fn edited(song: Song, cursor: SlotRef) -> Option<Command> {
    Some(Command::Edit { song: Box::new(song), cursor })
}

#[test]
fn key_to_command_of_navigation_keys_moves_the_cursor() {
    // Given a 3-section song laid out 2 bars per row (Verse: 2 rows, Chorus: 1 row, Bridge: empty)
    let song = song();
    let m = |k, cursor| move_to(k, &state(&song, cursor));

    // Then ←/→ walk slots across bars and sections, including the empty section, and stop at both ends
    assert_eq!(m(key("ArrowRight"), at(0, 0, 0)), Ok(at(0, 1, 0)));
    assert_eq!(m(key("ArrowRight"), at(0, 1, 1)), Ok(at(0, 2, 0)));
    assert_eq!(m(key("ArrowRight"), at(0, 2, 0)), Ok(at(1, 0, 0)));
    assert_eq!(m(key("ArrowRight"), at(1, 0, 0)), Ok(at(2, 0, 0)));
    assert_eq!(m(key("ArrowRight"), at(2, 0, 0)), Ok(at(2, 0, 0)));
    assert_eq!(m(key("ArrowLeft"), at(0, 1, 0)), Ok(at(0, 0, 0)));
    assert_eq!(m(key("ArrowLeft"), at(0, 0, 0)), Ok(at(0, 0, 0)));

    // And ↑/↓ go to the same column of the previous/next row, clamped to the row and the bar's slots
    assert_eq!(m(key("ArrowDown"), at(0, 1, 1)), Ok(at(0, 2, 0)));
    assert_eq!(m(key("ArrowDown"), at(0, 2, 0)), Ok(at(1, 0, 0)));
    assert_eq!(m(key("ArrowDown"), at(1, 0, 0)), Ok(at(2, 0, 0)));
    assert_eq!(m(key("ArrowUp"), at(0, 2, 0)), Ok(at(0, 0, 0)));
    assert_eq!(m(key("ArrowUp"), at(0, 0, 0)), Ok(at(0, 0, 0)));

    // And Home/End go to the first/last slot of the section
    assert_eq!(m(key("Home"), at(0, 1, 1)), Ok(at(0, 0, 0)));
    assert_eq!(m(key("End"), at(0, 0, 0)), Ok(at(0, 2, 0)));

    // And a plain move clears the bar selection
    let selecting = EditorState { anchor: Some(bar_at(0, 0)), ..state(&song, at(0, 0, 0)) };
    assert!(matches!(key_to_command(key("ArrowRight"), &selecting), Some(Command::Move { anchor: None, .. })));
}

fn flagged() -> Song {
    let mut song = song();
    song.sections[0].bars[0].chords[0].confidence = Some(0.5);
    song.sections[0].bars[1].chords[1].confidence = Some(0.2);
    song.sections[1].bars[0].chords[0].confidence = Some(0.4);
    song
}

#[test]
fn key_to_command_of_f8_jumps_to_the_next_or_previous_low_confidence_slot_wrapping_around() {
    // Given low-confidence slots at Verse bar 2 slot 2 (0.2) and Chorus bar 1 (0.4), and a confident one (0.5) at Verse bar 1
    let song = flagged();
    let f8 = |cursor, back: bool, threshold| {
        let s = EditorState { low_confidence_threshold: threshold, ..state(&song, cursor) };
        move_to(if back { shift("F8") } else { key("F8") }, &s)
    };

    // Then F8 goes forward, Shift+F8 backward, both wrapping around the song
    assert_eq!(f8(at(0, 0, 0), false, 0.5), Ok(at(0, 1, 1)));
    assert_eq!(f8(at(0, 1, 1), false, 0.5), Ok(at(1, 0, 0)));
    assert_eq!(f8(at(2, 0, 0), false, 0.5), Ok(at(0, 1, 1)));
    assert_eq!(f8(at(0, 1, 1), true, 0.5), Ok(at(1, 0, 0)));
    assert_eq!(f8(at(1, 0, 0), true, 0.5), Ok(at(0, 1, 1)));

    // And the threshold decides what is flagged; with nothing flagged, the key is refused
    assert_eq!(f8(at(1, 0, 0), false, 0.3), Ok(at(0, 1, 1)));
    assert_eq!(f8(at(0, 0, 0), false, 0.1), Err(Some(Command::Refused)));
}

#[test]
fn next_flagged_bar_when_the_bar_has_several_flagged_slots_skips_to_the_next_bar() {
    // Given both slots of Verse bar 2 flagged, and Chorus bar 1 flagged
    let mut song = flagged();
    song.sections[0].bars[0].chords[0].confidence = None;
    song.sections[0].bars[1].chords[0].confidence = Some(0.1);

    // Then forward from either slot of bar 2 lands on the Chorus, and backward from the Chorus on bar 2, wrapping around
    assert_eq!(next_flagged_bar(&song, at(0, 1, 0), 0.5, Step::Forward), Some(at(1, 0, 0)));
    assert_eq!(next_flagged_bar(&song, at(1, 0, 0), 0.5, Step::Forward), Some(at(0, 1, 0)));
    assert_eq!(next_flagged_bar(&song, at(0, 1, 1), 0.5, Step::Back), Some(at(1, 0, 0)));
    assert_eq!(next_flagged_bar(&song, at(1, 0, 0), 0.5, Step::Back), Some(at(0, 1, 1)));
    assert_eq!(next_flagged_bar(&song, at(0, 0, 0), 0.05, Step::Forward), None);
}

#[test]
fn count_flagged_and_has_confidence_follow_the_threshold() {
    // Given / When / Then
    assert_eq!(count_flagged(&flagged(), 0.5), 2);
    assert_eq!(count_flagged(&flagged(), 0.3), 1);
    assert!(has_confidence(&flagged()));
    assert!(!has_confidence(&song()));
}

#[test]
fn key_to_command_of_shift_arrows_extends_the_bar_selection_within_the_section() {
    let song = song();

    // Given no selection, When Shift+→, Then the cursor bar becomes the anchor and the cursor moves to the next bar
    assert_eq!(
        key_to_command(shift("ArrowRight"), &state(&song, at(0, 0, 0))),
        Some(Command::Move { cursor: at(0, 1, 0), anchor: Some(bar_at(0, 0)) })
    );

    // Given an anchor, When Shift+←, Then the anchor stays
    let selecting = EditorState { anchor: Some(bar_at(0, 2)), ..state(&song, at(0, 2, 0)) };
    assert_eq!(
        key_to_command(shift("ArrowLeft"), &selecting),
        Some(Command::Move { cursor: at(0, 1, 0), anchor: Some(bar_at(0, 2)) })
    );

    // And the selection doesn't leave the section
    assert_eq!(key_to_command(shift("ArrowRight"), &state(&song, at(0, 2, 0))), None);
}

#[test]
fn key_to_command_of_edit_shortcuts_applies_the_command_and_places_the_cursor() {
    // Given the cursor on the G slot (bar 2, beat 3)
    let song = song();
    let g = at(0, 1, 1);
    let bar = bar_at(0, 1);
    let edit = |k| key_to_command(k, &state(&song, g));

    // Then slot shortcuts
    assert_eq!(edit(key("/")), edited(split_slot(&song, g).unwrap(), g));
    assert_eq!(edit(key("Backspace")), edited(merge_slot_with_next(&song, at(0, 1, 0)).unwrap(), at(0, 1, 0)));
    assert_eq!(edit(alt("ArrowRight")), edited(resize_slot(&song, g, Step::Forward).unwrap(), g));
    assert_eq!(edit(alt("ArrowLeft")), edited(resize_slot(&song, g, Step::Back).unwrap(), g));

    // And bar shortcuts (Mod is Ctrl or Cmd)
    assert_eq!(edit(ctrl("Enter")), edited(insert_bar(&song, bar, Position::After).unwrap(), at(0, 2, 0)));
    let meta_shift_enter = KeyInput { meta: true, shift: true, ..key("Enter") };
    assert_eq!(edit(meta_shift_enter), edited(insert_bar(&song, bar, Position::Before).unwrap(), at(0, 1, 0)));
    assert_eq!(edit(ctrl("Backspace")), edited(delete_bar(&song, bar).unwrap(), at(0, 1, 0)));
    assert_eq!(edit(ctrl("d")), edited(duplicate_bar(&song, bar).unwrap(), g));
    let clipboard = [song.sections[0].bars[0].clone()];
    let with_clipboard = EditorState { clipboard: &clipboard, ..state(&song, g) };
    assert_eq!(
        key_to_command(ctrl("v"), &with_clipboard),
        edited(paste_bars(&song, bar, &clipboard, Position::After).unwrap(), g)
    );
    assert_eq!(edit(ctrl("v")), None);

    // And Mod+B makes the bar a short break bar (2/4; 3/8 in a 6/8 song), or back to the song meter
    let two_four = Meter { beats: 2, unit: 4, extra: Default::default() };
    let half = set_bar_meter(&song, bar, Some(two_four)).unwrap();
    assert_eq!(edit(ctrl("b")), edited(half.clone(), at(0, 1, 0)));
    assert_eq!(
        key_to_command(ctrl("b"), &state(&half, at(0, 1, 0))),
        edited(set_bar_meter(&half, bar, None).unwrap(), at(0, 1, 0))
    );
    // And < / > shift the bar lines one beat earlier / later from the cursor bar on
    assert_eq!(edit(key("<")), edited(shift_phase(&song, Step::Back, bar).unwrap(), at(0, 1, 0)));
    assert_eq!(edit(shift(">")), edited(shift_phase(&song, Step::Forward, bar).unwrap(), at(0, 1, 0)));
    // And - / + halve / double the tempo from the cursor bar on
    assert_eq!(edit(key("-")), edited(halve_tempo(&song, bar).unwrap(), at(0, 1, 0)));
    assert_eq!(edit(shift("+")), edited(double_tempo(&song, bar).unwrap(), at(0, 1, 0)));
    let jig = song_of(json!({
        "version": 1,
        "meta": { "title": "t", "meter": { "beats": 6, "unit": 8 } },
        "sections": [{ "id": "j", "label": "A", "bars": [{ "chords": [{ "chord": "D:maj", "beats": 6 }] }] }]
    }));
    let Some(Command::Edit { song: jig_edited, .. }) = key_to_command(ctrl("b"), &state(&jig, at(0, 0, 0))) else { panic!() };
    let jig_bar = &jig_edited.sections[0].bars[0];
    assert_eq!((jig_bar.meter.as_ref().map(|m| (m.beats, m.unit)), jig_bar.chords[0].beats), (Some((3, 8)), 3));

    // And section shortcuts
    let labels = |k| match edit(k) {
        Some(Command::Edit { song, cursor }) => (song.sections.iter().map(|s| s.label.clone()).collect::<Vec<_>>(), cursor),
        other => panic!("{other:?}"),
    };
    assert_eq!(labels(ctrl("k")), (vec!["Verse".into(), "Verse (2)".into(), "Chorus".into(), "Bridge".into()], at(1, 0, 0)));
    let ctrl_shift_k = KeyInput { shift: true, ..ctrl("K") };
    assert_eq!(labels(ctrl_shift_k), (vec!["Verse".into(), "New section".into(), "Chorus".into(), "Bridge".into()], at(1, 0, 0)));
    let ctrl_shift_backspace = KeyInput { shift: true, ..ctrl("Backspace") };
    assert_eq!(labels(ctrl_shift_backspace), (vec!["Chorus".into(), "Bridge".into()], at(0, 0, 0)));
    assert_eq!(edit(alt("ArrowUp")), edited(set_repeat(&song, 0, 2).unwrap(), g));
    assert_eq!(
        key_to_command(alt("ArrowDown"), &state(&song, at(1, 0, 0))),
        edited(set_repeat(&song, 1, 1).unwrap(), at(1, 0, 0))
    );
    let alt_shift_down = KeyInput { shift: true, ..alt("ArrowDown") };
    assert_eq!(edit(alt_shift_down), edited(move_section(&song, 0, 1).unwrap(), at(1, 1, 1)));

    // And impossible edits are refused instead of failing
    assert_eq!(key_to_command(key("Backspace"), &state(&song, at(0, 1, 0))), Some(Command::Refused));
    let alt_shift_up = KeyInput { shift: true, ..alt("ArrowUp") };
    assert_eq!(key_to_command(alt_shift_up, &state(&song, at(0, 0, 0))), Some(Command::Refused));
    assert_eq!(key_to_command(ctrl("k"), &state(&song, at(2, 0, 0))), Some(Command::Refused));
    assert_eq!(key_to_command(ctrl("b"), &state(&song, at(2, 0, 0))), Some(Command::Refused));
}

#[test]
fn key_to_command_of_other_keys_returns_the_matching_action() {
    let song = song();
    let first = state(&song, at(0, 0, 0));

    // Then history, help, rename, typing, play/pause and play from the cursor bar
    assert_eq!(key_to_command(ctrl("z"), &first), Some(Command::Undo));
    assert_eq!(key_to_command(KeyInput { shift: true, ..ctrl("Z") }, &first), Some(Command::Redo));
    assert_eq!(key_to_command(shift("?"), &first), Some(Command::Help));
    assert_eq!(key_to_command(key("F2"), &first), Some(Command::Rename));
    assert_eq!(key_to_command(shift("A"), &first), Some(Command::Type("A".into())));
    assert_eq!(key_to_command(key(" "), &first), Some(Command::Play));
    assert_eq!(key_to_command(shift(" "), &first), Some(Command::PlayFromCursor));

    // And copy takes the selected bars, or the cursor bar without a selection
    let selecting = EditorState { anchor: Some(bar_at(0, 0)), ..state(&song, at(0, 1, 1)) };
    assert_eq!(key_to_command(ctrl("c"), &selecting), Some(Command::Copy(song.sections[0].bars[..2].to_vec())));
    assert_eq!(key_to_command(ctrl("c"), &state(&song, at(0, 2, 0))), Some(Command::Copy(vec![song.sections[0].bars[2].clone()])));

    // And unmapped keys (F5 comes as a private-use character), reserved shortcuts and typing into an empty section do nothing
    assert_eq!(key_to_command(key("\u{F704}"), &first), None);
    assert_eq!(key_to_command(ctrl("s"), &first), None);
    assert_eq!(key_to_command(key("C"), &state(&song, at(2, 0, 0))), None);
}

#[test]
fn clamp_cursor_keeps_the_cursor_inside_the_song() {
    // Given / When / Then
    let song = song();
    assert_eq!(clamp_cursor(&song, at(9, 9, 9)), at(2, 0, 0));
    assert_eq!(clamp_cursor(&song, at(0, 1, 5)), at(0, 1, 1));
    assert_eq!(clamp_cursor(&song, at(0, 7, 0)), at(0, 2, 0));
}
