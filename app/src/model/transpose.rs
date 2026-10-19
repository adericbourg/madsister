//! Transposing a song (F-ED-9).

use super::chord::{leading_note, parse_harte, semitone};
use super::display::note_name;
use super::song::Song;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spelling {
    Sharp,
    Flat,
}

// Respells the leading note of a chord or key ("F#:7/3", "Am", "A:min"); the rest is relative to it, so it stays.
fn transpose_root(text: &str, semitones: i32, spelling: Spelling) -> String {
    match leading_note(text) {
        Some(root) => note_name(semitone(root) + semitones, spelling == Spelling::Flat).to_string() + &text[root.len()..],
        None => text.to_string(),
    }
}

pub fn transpose_chord(harte: &str, semitones: i32, spelling: Spelling) -> String {
    if parse_harte(harte).is_some() { transpose_root(harte, semitones, spelling) } else { harte.to_string() }
}

// Keeps `confidence`: transposing isn't a correction of the chord.
pub fn transpose_song(song: &Song, semitones: i32, spelling: Spelling) -> Song {
    let mut song = song.clone();
    if let Some(key) = &mut song.meta.key {
        *key = transpose_root(key, semitones, spelling);
    }
    for slot in song.sections.iter_mut().flat_map(|s| &mut s.bars).flat_map(|b| &mut b.chords) {
        slot.chord = transpose_chord(&slot.chord, semitones, spelling);
    }
    song
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::song::parse_song;

    #[test]
    fn transpose_chord_moves_the_root() {
        for (harte, semitones, spelling, expected) in [
            ("C:maj7", 2, Spelling::Sharp, "D:maj7"),
            ("B:min", 1, Spelling::Flat, "C:min"),
            ("F#:7/3", -1, Spelling::Flat, "F:7/3"),
            ("C#:min", 0, Spelling::Flat, "Db:min"),
            ("Bb:maj/5", -13, Spelling::Sharp, "A:maj/5"),
            ("N", 3, Spelling::Sharp, "N"),
            ("%", 3, Spelling::Flat, "%"),
        ] {
            // Given / When / Then
            assert_eq!(transpose_chord(harte, semitones, spelling), expected, "{harte} by {semitones}");
        }
    }

    fn song() -> Song {
        parse_song(
            r#"{
                "version": 1,
                "meta": { "title": "Song", "key": "Am", "meter": { "beats": 4, "unit": 4 } },
                "sections": [{
                    "id": "s1",
                    "label": "Verse",
                    "bars": [
                        { "chords": [{ "chord": "A:min", "beats": 2, "confidence": 0.4 }, { "chord": "F#:7/3", "beats": 2 }] },
                        { "chords": [{ "chord": "N", "beats": 4 }] }
                    ]
                }]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn transpose_song_by_twelve_with_original_spelling_returns_original_song() {
        // Given / When / Then
        assert_eq!(transpose_song(&song(), 12, Spelling::Sharp), song());
    }

    #[test]
    fn transpose_song_by_three_flat_transposes_chords_and_key_and_keeps_confidence() {
        // Given / When
        let transposed = transpose_song(&song(), 3, Spelling::Flat);

        // Then
        assert_eq!(transposed.meta.key.as_deref(), Some("Cm"));
        let chords = &transposed.sections[0].bars[0].chords;
        assert_eq!((chords[0].chord.as_str(), chords[0].beats, chords[0].confidence), ("C:min", 2, Some(0.4)));
        assert_eq!((chords[1].chord.as_str(), chords[1].beats, chords[1].confidence), ("A:7/3", 2, None));
        assert_eq!(transposed.sections[0].bars[1].chords[0].chord, "N");
    }
}
