use serde_json::{Value, json};

use super::*;
use crate::model::song::{empty_song, parse_song, serialize_song};

fn song_of(file: Value) -> Song {
    parse_song(&file.to_string()).unwrap()
}

// 4/4. Verse: bar 0 = A:min 3 + E:7 1 (aligned to audio), bar 1 = C:maj 4. Chorus x2: G:maj 4, F:maj 4.
fn fixture() -> Song {
    song_of(json!({
        "version": 1,
        "meta": { "title": "Song", "meter": { "beats": 4, "unit": 4 } },
        "sections": [
            {
                "id": "s1",
                "label": "Verse",
                "bars": [
                    { "startSec": 0, "chords": [{ "chord": "A:min", "beats": 3, "confidence": 0.4 }, { "chord": "E:7", "beats": 1 }] },
                    { "startSec": 2, "chords": [{ "chord": "C:maj", "beats": 4, "confidence": 0.9 }] }
                ]
            },
            {
                "id": "s2",
                "label": "Chorus",
                "repeat": 2,
                "bars": [
                    { "startSec": 4, "chords": [{ "chord": "G:maj", "beats": 4 }] },
                    { "startSec": 6, "chords": [{ "chord": "F:maj", "beats": 4 }] }
                ]
            }
        ]
    }))
}

/// Every command must keep the file valid (beat sums).
fn valid(edit: Edit) -> Song {
    let song = edit.unwrap();
    parse_song(&serialize_song(&song)).unwrap();
    song
}

fn labels(song: &Song) -> Vec<&str> {
    song.sections.iter().map(|s| s.label.as_str()).collect()
}

fn chords(song: &Song, section: usize) -> Vec<String> {
    song.sections[section]
        .bars
        .iter()
        .map(|b| b.chords.iter().map(|s| s.chord.as_str()).collect::<Vec<_>>().join(" "))
        .collect()
}

fn slots(bar: &Bar) -> Vec<(&str, u32)> {
    bar.chords.iter().map(|s| (s.chord.as_str(), s.beats)).collect()
}

fn beats(bar: &Bar) -> Vec<u32> {
    bar.chords.iter().map(|s| s.beats).collect()
}

fn starts(bars: &[Bar]) -> Vec<Option<f64>> {
    bars.iter().map(|b| b.start_sec).collect()
}

fn meter(beats: u32, unit: u32) -> Meter {
    Meter { beats, unit, extra: Map::new() }
}

fn shape(song: &Song) -> Vec<Vec<String>> {
    song.sections
        .iter()
        .map(|s| {
            s.bars
                .iter()
                .map(|b| {
                    let start = b.start_sec.map_or("-".to_string(), |t| t.to_string());
                    let slots: Vec<String> = b.chords.iter().map(|c| format!("{}:{}", c.chord, c.beats)).collect();
                    format!("{start}|{}", slots.join(" "))
                })
                .collect()
        })
        .collect()
}

fn shapes(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter().map(|row| row.iter().map(|s| s.to_string()).collect()).collect()
}

const fn at(section: usize, bar: usize) -> BarRef {
    BarRef { section, bar }
}

const fn slot_at(section: usize, bar: usize, slot: usize) -> SlotRef {
    SlotRef { section, bar, slot }
}

#[test]
fn set_chord_replaces_the_chord_and_clears_confidence() {
    // Given
    let song = fixture();

    // When
    let result = valid(set_chord(&song, slot_at(0, 0, 0), "D:min7"));

    // Then
    let bars = &result.sections[0].bars;
    assert_eq!((bars[0].chords[0].chord.as_str(), bars[0].chords[0].beats, bars[0].chords[0].confidence), ("D:min7", 3, None));
    assert_eq!(bars[0].start_sec, Some(0.0));
    assert_eq!(bars[1], song.sections[0].bars[1]);
}

mod set_key {
    use super::*;

    // Nashville "1 4/3 5⁷": in G by default, or relative to C when it has no key.
    fn nashville(key: Option<&str>) -> Song {
        let [one, four, five] = if key.is_none() { ["C:maj", "F:maj/3", "G:7"] } else { ["G:maj", "C:maj/3", "D:7"] };
        let mut meta = json!({ "title": "Song", "notation": "nashville", "meter": { "beats": 4, "unit": 4 } });
        if let Some(key) = key {
            meta["key"] = json!(key);
        }
        song_of(json!({
            "version": 1,
            "meta": meta,
            "sections": [{ "id": "s1", "label": "Verse", "bars": [{ "chords": [
                { "chord": one, "beats": 2 }, { "chord": four, "beats": 1 }, { "chord": five, "beats": 1 }
            ] }] }]
        }))
    }

    fn chords(song: &Song) -> Vec<&str> {
        song.sections[0].bars[0].chords.iter().map(|s| s.chord.as_str()).collect()
    }

    #[test]
    fn set_key_in_chords_notation_only_sets_the_key() {
        // Given
        let song = fixture();

        // When
        let result = valid(Ok(set_key(&song, Some("Am"), MinorConvention::Relative)));

        // Then
        assert_eq!(result.meta.key.as_deref(), Some("Am"));
        assert_eq!(result.sections, song.sections);
    }

    #[test]
    fn set_key_in_nashville_from_no_key_transposes_from_c_to_the_new_tonic() {
        // Given the numbers 1 4 5 stored relative to C
        let song = nashville(None);

        // When
        let result = set_key(&song, Some("G"), MinorConvention::Relative);

        // Then the numbers stay, the chords move
        assert_eq!(result.meta.key.as_deref(), Some("G"));
        assert_eq!(chords(&result), ["G:maj", "C:maj/3", "D:7"]);
    }

    #[test]
    fn set_key_in_nashville_from_another_key_keeps_the_numbers() {
        // Given 1 4 5 in G
        let song = nashville(Some("G"));

        // When
        let result = set_key(&song, Some("Bb"), MinorConvention::Relative);

        // Then 1 4 5 in Bb
        assert_eq!(chords(&result), ["Bb:maj", "Eb:maj/3", "F:7"]);
        assert_eq!(result.meta.key.as_deref(), Some("Bb"));
    }

    #[test]
    fn set_key_in_nashville_to_no_key_transposes_back_to_c() {
        // Given
        let song = nashville(Some("G"));

        // When
        let result = set_key(&song, None, MinorConvention::Relative);

        // Then
        assert_eq!(chords(&result), ["C:maj", "F:maj/3", "G:7"]);
        assert_eq!(result.meta.key, None);
    }

    #[test]
    fn set_key_in_nashville_with_minor_key_follows_the_minor_convention() {
        // Given 1 4 5 relative to C
        let song = nashville(None);

        // When Am with the relative convention, degree 1 is C
        let relative = set_key(&song, Some("Am"), MinorConvention::Relative);
        // And with the tonic convention, degree 1 is A
        let tonic = set_key(&song, Some("Am"), MinorConvention::Tonic);

        // Then
        assert_eq!(chords(&relative), ["C:maj", "F:maj/3", "G:7"]);
        assert_eq!(chords(&tonic), ["A:maj", "D:maj/3", "E:7"]);
    }
}

#[test]
fn set_notation_with_a_key_switches_without_touching_the_chords() {
    // Given
    let song = set_key(&fixture(), Some("Am"), MinorConvention::Relative);

    // When
    let result = valid(set_notation(&song, Notation::Nashville));

    // Then
    assert_eq!(result.meta.notation, Some(Notation::Nashville));
    assert_eq!(result.sections, song.sections);
    assert_eq!(set_notation(&result, Notation::Chords).unwrap().meta.notation, Some(Notation::Chords));
}

#[test]
fn set_notation_without_key_fails() {
    // Given / When / Then
    assert!(set_notation(&fixture(), Notation::Nashville).unwrap_err().contains("a key is required"));
}

#[test]
fn split_slot_of_three_beats_returns_two_plus_one_copying_the_chord() {
    // Given
    let song = fixture();

    // When
    let even = valid(split_slot(&song, slot_at(0, 1, 0)));
    let odd = valid(split_slot(&song, slot_at(0, 0, 0)));

    // Then
    assert_eq!(slots(&even.sections[0].bars[1]), [("C:maj", 2), ("C:maj", 2)]);
    assert_eq!(slots(&odd.sections[0].bars[0]), [("A:min", 2), ("A:min", 1), ("E:7", 1)]);
}

#[test]
fn split_slot_of_one_beat_fails() {
    // Given / When / Then
    assert!(split_slot(&fixture(), slot_at(0, 0, 1)).is_err());
}

#[test]
fn merge_slot_with_next_adds_beats_and_keeps_the_first_chord() {
    // Given
    let song = fixture();

    // When
    let result = valid(merge_slot_with_next(&song, slot_at(0, 0, 0)));

    // Then
    assert_eq!(slots(&result.sections[0].bars[0]), [("A:min", 4)]);
    assert!(merge_slot_with_next(&song, slot_at(0, 0, 1)).is_err());
}

#[test]
fn remove_slot_gives_the_beats_to_the_previous_slot_or_the_next_for_the_first() {
    // Given
    let song = fixture();

    // When / Then
    let last = valid(remove_slot(&song, slot_at(0, 0, 1)));
    assert_eq!(slots(&last.sections[0].bars[0]), [("A:min", 4)]);
    let first = valid(remove_slot(&song, slot_at(0, 0, 0)));
    assert_eq!(slots(&first.sections[0].bars[0]), [("E:7", 4)]);
}

#[test]
fn remove_slot_turns_the_only_slot_into_n() {
    // Given
    let song = fixture();

    // When
    let result = valid(remove_slot(&song, slot_at(0, 1, 0)));

    // Then
    assert_eq!(slots(&result.sections[0].bars[1]), [("N", 4)]);
    assert_eq!(result.sections[0].bars[1].chords[0].confidence, None);
}

#[test]
fn resize_slot_takes_or_gives_a_beat_from_the_next_or_previous_slot() {
    // Given
    let song = fixture();
    let first = slot_at(0, 0, 0);
    let last = slot_at(0, 0, 1);

    // When
    let shrunk = valid(resize_slot(&song, first, Step::Back));
    let grown_last = valid(resize_slot(&song, last, Step::Forward));
    let grown_first = valid(resize_slot(&song, first, Step::Forward));
    let last_removed = valid(resize_slot(&song, last, Step::Back));

    // Then
    assert_eq!(beats(&shrunk.sections[0].bars[0]), [2, 2]);
    assert_eq!(beats(&grown_last.sections[0].bars[0]), [2, 2]);
    assert_eq!(slots(&grown_first.sections[0].bars[0]), [("A:min", 4)]);
    assert_eq!(slots(&last_removed.sections[0].bars[0]), [("A:min", 4)]);
}

#[test]
fn resize_slot_of_the_only_slot_fails() {
    // Given / When / Then
    assert!(resize_slot(&fixture(), slot_at(0, 1, 0), Step::Forward).is_err());
}

#[test]
fn insert_bar_inserts_an_empty_bar_and_keeps_neighbours_start_sec() {
    // Given
    let song = fixture();

    // When
    let before = valid(insert_bar(&song, at(0, 0), Position::Before));
    let after = valid(insert_bar(&song, at(0, 0), Position::After));

    // Then
    assert_eq!(starts(&before.sections[0].bars), [None, Some(0.0), Some(2.0)]);
    assert_eq!(before.sections[0].bars[0], empty_bar(&song));
    assert_eq!(slots(&before.sections[0].bars[0]), [("N", 4)]);
    assert_eq!(starts(&after.sections[0].bars), [Some(0.0), None, Some(2.0)]);
}

#[test]
fn delete_bar_removes_the_bar_and_allows_an_empty_section() {
    // Given
    let song = fixture();

    // When
    let deleted = valid(delete_bar(&song, at(0, 0)));
    let emptied = valid(delete_bar(&deleted, at(0, 0)));

    // Then
    assert_eq!(deleted.sections[0].bars, [song.sections[0].bars[1].clone()]);
    assert!(emptied.sections[0].bars.is_empty());
}

#[test]
fn duplicate_bar_inserts_a_copy_after_without_start_sec() {
    // Given
    let song = fixture();

    // When
    let result = valid(duplicate_bar(&song, at(0, 0)));

    // Then
    let bars = &result.sections[0].bars;
    assert_eq!(starts(bars), [Some(0.0), None, Some(2.0)]);
    assert_eq!(bars[1].chords, song.sections[0].bars[0].chords);
}

mod sections {
    use super::*;

    #[test]
    fn add_section_inserts_a_section_with_one_empty_bar() {
        // Given
        let song = fixture();

        // When
        let result = valid(Ok(add_section(&song, 1, "Bridge")));

        // Then
        assert_eq!(labels(&result), ["Verse", "Bridge", "Chorus"]);
        assert_eq!(result.sections[1].bars, [empty_bar(&song)]);
        let id = &result.sections[1].id;
        assert!(id.len() == 8 && id.chars().all(|c| c.is_ascii_hexdigit()), "{id}");
    }

    #[test]
    fn rename_section_move_section_set_repeat_update_the_sections() {
        // Given
        let song = fixture();

        // When
        let renamed = valid(rename_section(&song, 1, "Refrain"));
        let moved = valid(move_section(&song, 0, 1));
        let repeated = valid(set_repeat(&song, 0, 3));
        let unrepeated = valid(set_repeat(&song, 1, 1));

        // Then
        assert_eq!(labels(&renamed), ["Verse", "Refrain"]);
        assert_eq!(labels(&moved), ["Chorus", "Verse"]);
        assert_eq!(moved.sections[1], song.sections[0]);
        assert_eq!(repeated.sections[0].repeat, Some(3));
        assert_eq!(unrepeated.sections[1].repeat, None);
    }

    #[test]
    fn delete_section_removes_it_and_leaves_an_empty_song_section_when_it_was_the_last() {
        // Given
        let song = fixture();

        // When
        let deleted = valid(delete_section(&song, 0));
        let emptied = valid(delete_section(&deleted, 0));

        // Then
        assert_eq!(labels(&deleted), ["Chorus"]);
        assert_eq!(labels(&emptied), ["Song"]);
        assert_eq!(emptied.sections[0].bars, [empty_bar(&song)]);
    }

    #[test]
    fn split_section_moves_bars_from_the_ref_onward_to_a_new_section_right_after() {
        // Given
        let song = fixture();

        // When
        let at_last = valid(split_section(&song, at(0, 1)));
        let at_first = valid(split_section(&song, at(0, 0)));

        // Then
        assert_eq!(labels(&at_last), ["Verse", "Verse (2)", "Chorus"]);
        assert_eq!(chords(&at_last, 0), ["A:min E:7"]);
        assert_eq!(chords(&at_last, 1), ["C:maj"]);
        assert_ne!(at_last.sections[1].id, song.sections[0].id);
        assert!(chords(&at_first, 0).is_empty());
        assert_eq!(chords(&at_first, 1), ["A:min E:7", "C:maj"]);
    }

    #[test]
    fn split_section_keeps_the_repeat_count_on_the_first_part() {
        // Given / When
        let result = valid(split_section(&fixture(), at(1, 1)));

        // Then
        assert_eq!((result.sections[1].repeat, result.sections[2].repeat), (Some(2), None));
    }

    #[test]
    fn merge_with_previous_appends_the_bars_and_keeps_the_previous_label_and_repeat() {
        // Given Verse (2 bars) and Chorus x2 (2 bars)
        let song = fixture();

        // When
        let result = valid(merge_with_previous(&song, 1));

        // Then
        assert_eq!(labels(&result), ["Verse"]);
        assert_eq!(chords(&result, 0), ["A:min E:7", "C:maj", "G:maj", "F:maj"]);
        assert_eq!(result.sections[0].repeat, song.sections[0].repeat);
    }

    #[test]
    fn merge_with_previous_after_split_section_restores_the_bars() {
        // Given
        let song = fixture();

        // When
        let result = merge_with_previous(&split_section(&song, at(0, 1)).unwrap(), 1).unwrap();

        // Then
        assert_eq!(result.sections[0].bars, song.sections[0].bars);
    }

    #[test]
    fn merge_with_previous_on_the_first_section_fails() {
        assert!(merge_with_previous(&fixture(), 0).unwrap_err().contains("no previous section"));
    }
}

#[test]
fn paste_bars_of_two_copied_bars_inserts_them_in_order_without_start_sec_or_confidence() {
    // Given
    let song = fixture();
    let bars = copy_bars(&song, at(0, 0), at(0, 1)).unwrap();

    // When
    let after = valid(paste_bars(&song, at(1, 1), &bars, Position::After));
    let before = valid(paste_bars(&song, at(1, 0), &bars, Position::Before));

    // Then
    assert_eq!(chords(&after, 1), ["G:maj", "F:maj", "A:min E:7", "C:maj"]);
    assert_eq!(starts(&after.sections[1].bars), [Some(4.0), Some(6.0), None, None]);
    // A pasted chord is the user's decision: it is no longer flagged as low-confidence
    assert_eq!(after.sections[1].bars[2].chords[0].confidence, None);
    assert_eq!(chords(&before, 1), ["A:min E:7", "C:maj", "G:maj", "F:maj"]);
}

#[test]
fn paste_bars_when_beats_differ_from_the_song_meter_adds_a_meter_override() {
    // Given a 3/4 target song and 4-beat bars
    let song = fixture();
    let target = empty_song(meter(3, 4), None);
    let bars = copy_bars(&song, at(0, 1), at(0, 1)).unwrap();

    // When
    let result = valid(paste_bars(&target, at(0, 0), &bars, Position::Before));

    // Then
    assert_eq!(result.sections[0].bars[0].meter, Some(meter(4, 4)));
    assert_eq!(result.sections[0].bars[1].meter, None);
}

#[test]
fn copy_bars_of_an_unordered_range_fails() {
    // Given / When / Then
    assert!(copy_bars(&fixture(), at(0, 1), at(0, 0)).is_err());
    assert!(copy_bars(&fixture(), at(0, 0), at(1, 0)).is_err());
}

#[test]
fn set_bar_meter_resizes_the_slots_and_removes_the_override_at_the_song_meter() {
    // Given bar 0 = A:min 3 + E:7 1 in a 4/4 song
    let song = fixture();
    let bar = at(0, 0);

    // When shrinking it to 2/4, growing it to 5/4, then setting it back to 4/4 or to None
    let shrunk = valid(set_bar_meter(&song, bar, Some(meter(2, 4))));
    let grown = valid(set_bar_meter(&song, bar, Some(meter(5, 4))));
    let back = valid(set_bar_meter(&shrunk, bar, Some(meter(4, 4))));
    let reset = valid(set_bar_meter(&song, bar, None));

    // Then slots are truncated from the end, or the last slot grows; start_sec and the other bars are kept
    let shrunk_bar = &shrunk.sections[0].bars[0];
    assert_eq!((shrunk_bar.start_sec, &shrunk_bar.meter), (Some(0.0), &Some(meter(2, 4))));
    assert_eq!(slots(shrunk_bar), [("A:min", 2)]);
    assert_eq!(shrunk_bar.chords[0].confidence, Some(0.4));
    assert_eq!(slots(&grown.sections[0].bars[0]), [("A:min", 3), ("E:7", 2)]);
    assert_eq!(grown.sections[0].bars[0].meter, Some(meter(5, 4)));
    assert_eq!(shrunk.sections[0].bars[1], song.sections[0].bars[1]);
    // And the song meter removes the override
    assert_eq!(back.sections[0].bars[0].meter, None);
    assert_eq!(slots(&back.sections[0].bars[0]), [("A:min", 4)]);
    assert_eq!(back.sections[0].bars[0].chords[0].confidence, Some(0.4));
    assert_eq!(reset.sections[0].bars[0], song.sections[0].bars[0]);
}

#[test]
fn set_bar_meter_of_a_half_bar_in_a_68_song_counts_in_eighths() {
    // Given a 6/8 song whose bar is 4 + 2 eighths
    let song = song_of(json!({
        "version": 1,
        "meta": { "title": "Jig", "meter": { "beats": 6, "unit": 8 } },
        "sections": [{ "id": "s", "label": "A", "bars": [{ "chords": [{ "chord": "D:maj", "beats": 4 }, { "chord": "A:maj", "beats": 2 }] }] }]
    }));

    // When setting it to 3/8
    let result = valid(set_bar_meter(&song, at(0, 0), Some(meter(3, 8))));

    // Then it keeps 3 eighths of the first chord
    assert_eq!(result.sections[0].bars[0].meter, Some(meter(3, 8)));
    assert_eq!(slots(&result.sections[0].bars[0]), [("D:maj", 3)]);
}

#[test]
fn set_bar_meter_of_zero_beats_or_an_unreadable_unit_fails() {
    // Given / When / Then
    assert!(set_bar_meter(&fixture(), at(0, 0), Some(meter(0, 4))).is_err());
    assert!(set_bar_meter(&fixture(), at(0, 0), Some(meter(5, 16))).is_err());
}

mod shift_phase {
    use super::*;

    fn four_four(sections: Value) -> Song {
        song_of(json!({ "version": 1, "meta": { "title": "t", "meter": { "beats": 4, "unit": 4 } }, "sections": sections }))
    }

    #[test]
    fn shift_phase_when_chords_change_on_beat_2_moves_the_bar_lines_to_the_changes() {
        // Given a song whose chords all change on beat 2 (the bar lines are one beat early)
        let song = four_four(json!([{
            "id": "v",
            "label": "Verse",
            "bars": [
                { "startSec": 0, "chords": [{ "chord": "A:min", "beats": 1 }, { "chord": "C:maj", "beats": 3 }] },
                { "startSec": 2, "chords": [{ "chord": "C:maj", "beats": 1 }, { "chord": "G:maj", "beats": 3 }] },
                { "startSec": 4, "chords": [{ "chord": "G:maj", "beats": 1 }, { "chord": "F:maj", "beats": 3 }] },
                { "startSec": 6, "chords": [{ "chord": "F:maj", "beats": 4 }] }
            ]
        }]));

        // When shifting the phase by one beat forward
        let fixed = valid(shift_phase(&song, Step::Forward, WHOLE_SONG));

        // Then a 1-beat pickup comes first, each chord fills a bar, and the last bar is cut short
        assert_eq!(shape(&fixed), shapes(&[&["0|A:min:1", "0.5|C:maj:4", "2.5|G:maj:4", "4.5|F:maj:4", "6.5|F:maj:3"]]));
        assert_eq!(fixed.sections[0].bars[0].meter, Some(meter(1, 4)));
        assert_eq!(fixed.sections[0].bars[1].meter, None);
        assert_eq!(fixed.sections[0].bars[4].meter, Some(meter(3, 4)));
        // And Back undoes Forward, Forward undoes Back
        assert_eq!(valid(shift_phase(&fixed, Step::Back, WHOLE_SONG)), song);
        let back = valid(shift_phase(&song, Step::Back, WHOLE_SONG));
        assert_eq!(shift_phase(&back, Step::Forward, WHOLE_SONG).unwrap(), song);
    }

    #[test]
    fn shift_phase_from_a_bar_leaves_earlier_bars_and_anchors_sections_to_the_shifted_bar_lines() {
        // Given a Verse and a Chorus of two bars each, the third bar without a time
        let song = four_four(json!([
            { "id": "v", "label": "Verse", "bars": [
                { "startSec": 0, "chords": [{ "chord": "C:maj", "beats": 4 }] },
                { "startSec": 2, "chords": [{ "chord": "D:min", "beats": 4 }] }
            ] },
            { "id": "c", "label": "Chorus", "bars": [
                { "chords": [{ "chord": "G:maj", "beats": 4 }] },
                { "startSec": 6, "chords": [{ "chord": "F:maj", "beats": 4 }] }
            ] }
        ]));
        let from = at(0, 1);

        // When shifting one beat back from the second bar
        let shifted = valid(shift_phase(&song, Step::Back, from));

        // Then the first bar is kept, the Chorus starts one beat earlier, and bars starting in the untimed bar have no time
        assert_eq!(shifted.sections[0].bars[0], song.sections[0].bars[0]);
        assert_eq!(
            shape(&shifted),
            shapes(&[&["0|C:maj:4", "2|D:min:3"], &["3.5|D:min:1 G:maj:3", "-|G:maj:1 F:maj:3", "7.5|F:maj:1"]])
        );
        assert_eq!((shifted.sections[1].id.as_str(), shifted.sections[1].label.as_str()), ("c", "Chorus"));

        // When shifting twice forward from the second bar, Then it becomes a 2-beat break bar
        let once = valid(shift_phase(&song, Step::Forward, from));
        let twice = valid(shift_phase(&once, Step::Forward, from));
        assert_eq!(
            shape(&twice),
            shapes(&[&["0|C:maj:4", "2|D:min:2", "3|D:min:2 G:maj:2"], &["-|G:maj:2 F:maj:2", "7|F:maj:2"]])
        );
        assert_eq!(twice.sections[0].bars[1].meter, Some(meter(2, 4)));
    }

    #[test]
    fn shift_phase_across_a_bar_with_its_own_meter_keeps_its_length_and_moves_it() {
        // Given a 4/4 song with a 2/4 bar in the middle
        let song = four_four(json!([{
            "id": "v",
            "label": "Verse",
            "bars": [
                { "chords": [{ "chord": "C:maj", "beats": 4 }] },
                { "meter": { "beats": 2, "unit": 4 }, "chords": [{ "chord": "D:min", "beats": 2 }] },
                { "chords": [{ "chord": "G:maj", "beats": 4 }] }
            ]
        }]));

        // When shifting the whole song one beat forward
        let shifted = valid(shift_phase(&song, Step::Forward, WHOLE_SONG));

        // Then the 2/4 bar moves one beat later and stays 2/4
        assert_eq!(shape(&shifted), shapes(&[&["-|C:maj:1", "-|C:maj:3 D:min:1", "-|D:min:1 G:maj:1", "-|G:maj:3"]]));
        assert_eq!(shifted.sections[0].bars[2].meter, Some(meter(2, 4)));
        // And Back undoes it
        assert_eq!(shift_phase(&shifted, Step::Back, WHOLE_SONG).unwrap(), song);
    }

    #[test]
    fn shift_phase_of_a_guitarset_reference_shifted_by_one_beat_restores_it() {
        // Given the first 12 bars of GuitarSet 00_Rock1-130-A_comp (130 BPM), with its bar phase off by one beat
        let chords = ["A:maj", "A:maj", "A:maj", "A:maj", "D:maj/5", "D:maj/5", "A:maj", "A:maj", "E:maj", "D:sus2/2", "A:maj/5", "A:maj/5"];
        let bars: Vec<Value> = chords
            .iter()
            .enumerate()
            .map(|(i, chord)| json!({ "startSec": (i * 4 * 60) as f64 / 130.0, "chords": [{ "chord": chord, "beats": 4 }] }))
            .collect();
        let reference = four_four(json!([{ "id": "s", "label": "Song", "bars": bars }]));
        let wrong = shift_phase(&reference, Step::Forward, WHOLE_SONG).unwrap();

        // When fixing the phase in one action
        let fixed = shift_phase(&wrong, Step::Back, WHOLE_SONG).unwrap();

        // Then every bar is back on its reference chord and time
        assert_ne!(shape(&wrong), shape(&reference));
        for (bar, expected) in fixed.sections[0].bars.iter().zip(&reference.sections[0].bars) {
            assert_eq!(bar.chords, expected.chords);
            assert!((bar.start_sec.unwrap() - expected.start_sec.unwrap()).abs() < 5e-10);
        }
        assert_eq!(fixed.sections[0].bars.len(), reference.sections[0].bars.len());
    }
}

mod tempo {
    use super::*;

    fn song_at(tempo_bpm: f64, sections: Value) -> Song {
        song_of(json!({
            "version": 1,
            "meta": { "title": "t", "tempoBpm": tempo_bpm, "meter": { "beats": 4, "unit": 4 } },
            "sections": sections
        }))
    }

    /// A bar's start and its (chord, beats) slots.
    type BarSpec<'a> = (Option<f64>, &'a [(&'a str, u32)]);

    fn bars(specs: &[BarSpec]) -> Value {
        specs
            .iter()
            .map(|(start, slots)| {
                let chords: Vec<Value> = slots.iter().map(|(chord, beats)| json!({ "chord": chord, "beats": beats })).collect();
                match start {
                    Some(start) => json!({ "startSec": start, "chords": chords }),
                    None => json!({ "chords": chords }),
                }
            })
            .collect()
    }

    #[test]
    fn halve_tempo_of_a_song_tracked_at_double_tempo_merges_bars_two_by_two_within_each_section() {
        // Given a 120 BPM Verse of 5 bars (a chord change on an odd beat in bar 3) and a Chorus of 2 bars
        let verse = bars(&[
            (Some(0.0), &[("A:maj", 4)]),
            (Some(1.0), &[("A:maj", 2), ("B:maj", 2)]),
            (Some(2.0), &[("C:maj", 3), ("D:maj", 1)]),
            (Some(3.0), &[("E:maj", 4)]),
            (Some(4.0), &[("F:maj", 4)]),
        ]);
        let chorus = bars(&[(Some(5.0), &[("G:maj", 4)]), (Some(6.0), &[("G:maj", 2), ("A:min", 2)])]);
        let song = song_at(
            120.0,
            json!([{ "id": "v", "label": "Verse", "bars": verse }, { "id": "c", "label": "Chorus", "bars": chorus }]),
        );

        // When halving the tempo of the whole song
        let halved = valid(halve_tempo(&song, WHOLE_SONG));

        // Then each pair of tracked beats is one beat (the first one's chord), the odd last bar of the Verse is a 2/4 bar,
        // the Chorus still starts on its own bar, and the tempo is halved
        assert_eq!(shape(&halved), shapes(&[&["0|A:maj:3 B:maj:1", "2|C:maj:2 E:maj:2", "4|F:maj:2"], &["5|G:maj:3 A:min:1"]]));
        assert_eq!(halved.sections[0].bars[2].meter, Some(meter(2, 4)));
        assert_eq!(halved.sections[0].bars[0].meter, None);
        assert_eq!(halved.meta.tempo_bpm, Some(60.0));

        // When halving from bar 2, Then bar 1 is kept, pairs start on bar 2 and the song tempo is kept
        let from_bar_2 = valid(halve_tempo(&song, at(0, 1)));
        assert_eq!(from_bar_2.sections[0].bars[0], song.sections[0].bars[0]);
        assert_eq!(
            shape(&from_bar_2),
            shapes(&[&["0|A:maj:4", "1|A:maj:1 B:maj:1 C:maj:2", "3|E:maj:2 F:maj:2"], &["5|G:maj:3 A:min:1"]])
        );
        assert_eq!(from_bar_2.meta.tempo_bpm, Some(120.0));
    }

    #[test]
    fn double_tempo_of_a_song_tracked_at_half_tempo_splits_each_bar_at_its_midpoint() {
        // Given a 60 BPM song: two timed bars, then an untimed bar and a 3/4 bar
        let verse = bars(&[(Some(0.0), &[("A:maj", 3), ("B:maj", 1)]), (Some(2.0), &[("C:maj", 4)])]);
        let mut chorus = bars(&[(None, &[("G:maj", 4)])]);
        chorus.as_array_mut().unwrap().push(json!({ "meter": { "beats": 3, "unit": 4 }, "chords": [{ "chord": "D:min", "beats": 3, "confidence": 0.3 }] }));
        let song = song_at(
            60.0,
            json!([{ "id": "v", "label": "Verse", "bars": verse }, { "id": "c", "label": "Chorus", "bars": chorus }]),
        );

        // When doubling the tempo
        let doubled = valid(double_tempo(&song, WHOLE_SONG));

        // Then each bar is two bars, the second one starting at the midpoint (the last period carries on), untimed bars
        // stay untimed, the 3/4 bar is two 3/4 bars, and the tempo is doubled
        assert_eq!(
            shape(&doubled),
            shapes(&[
                &["0|A:maj:4", "1|A:maj:2 B:maj:2", "2|C:maj:4", "3|C:maj:4"],
                &["-|G:maj:4", "-|G:maj:4", "-|D:min:3", "-|D:min:3"],
            ])
        );
        let last = &doubled.sections[1].bars[3];
        assert_eq!((&last.meter, last.chords[0].confidence), (&Some(meter(3, 4)), Some(0.3)));
        assert_eq!(doubled.meta.tempo_bpm, Some(120.0));

        // And halving undoes doubling, for the whole song or from a bar
        assert_eq!(halve_tempo(&doubled, WHOLE_SONG).unwrap(), song);
        let from = at(0, 1);
        let doubled_from = double_tempo(&song, from).unwrap();
        assert_eq!(doubled_from.sections[0].bars[0], song.sections[0].bars[0]);
        assert_eq!(halve_tempo(&doubled_from, from).unwrap(), song);
    }

    #[test]
    fn halve_tempo_of_a_guitarset_reference_tracked_at_double_tempo_restores_it() {
        // Given the first 12 bars of GuitarSet 00_Rock1-130-A_comp (130 BPM), tracked at 260 BPM: each bar cut in two
        let chords = ["A:maj", "A:maj", "A:maj", "A:maj", "D:maj/5", "D:maj/5", "A:maj", "A:maj", "E:maj", "D:sus2/2", "A:maj/5", "A:maj/5"];
        let bar_sec = (4 * 60) as f64 / 130.0;
        let reference: Vec<Value> = chords
            .iter()
            .enumerate()
            .map(|(i, chord)| json!({ "startSec": i as f64 * bar_sec, "chords": [{ "chord": chord, "beats": 4 }] }))
            .collect();
        let wrong: Vec<Value> = chords
            .iter()
            .enumerate()
            .flat_map(|(i, chord)| {
                [0.0, 0.5].map(|half| json!({ "startSec": (i as f64 + half) * bar_sec, "chords": [{ "chord": chord, "beats": 4 }] }))
            })
            .collect();
        let reference = song_at(130.0, json!([{ "id": "s", "label": "Song", "bars": reference }]));
        let wrong = song_at(260.0, json!([{ "id": "s", "label": "Song", "bars": wrong }]));

        // When fixing the tempo in one action
        let fixed = halve_tempo(&wrong, WHOLE_SONG).unwrap();

        // Then every bar is back on its reference chord and time, at the reference tempo
        assert_eq!(fixed.sections[0].bars.len(), reference.sections[0].bars.len());
        for (bar, expected) in fixed.sections[0].bars.iter().zip(&reference.sections[0].bars) {
            assert_eq!(bar.chords, expected.chords);
            assert!((bar.start_sec.unwrap() - expected.start_sec.unwrap()).abs() < 5e-10);
        }
        assert_eq!(fixed.meta.tempo_bpm, Some(130.0));
    }
}
