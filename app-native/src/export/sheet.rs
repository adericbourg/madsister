//! The printed chart (spec F-EX-1) as spelled text, for `pdf.rs` to lay out and draw: the GUI and `madsister print`
//! share it.

use crate::model::display::{display_chord, display_key};
use crate::model::nashville::{display_nashville, tonic_pc};
use crate::model::song::{Notation, Song, whole};
use crate::settings::{ChartFont, Settings};

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub title: String,
    pub artist: Option<String>,
    pub facts: Vec<String>,
    pub font: ChartFont,
    pub compact: bool,
    pub bars_per_row: usize,
    pub sections: Vec<SheetSection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SheetSection {
    pub label: String,
    pub repeat: Option<u32>,
    pub bars: Vec<SheetBar>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SheetBar {
    pub meter: Option<String>,
    pub slots: Vec<SheetSlot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SheetSlot {
    pub text: String,
    pub beats: u32,
}

/// Pitch class of degree 1 when the song is in Nashville notation.
pub fn nashville_tonic(song: &Song, settings: &Settings) -> Option<i32> {
    let meta = &song.meta;
    (meta.notation == Some(Notation::Nashville)).then(|| tonic_pc(meta.key.as_deref(), settings.minor_convention))
}

/// A chord as the grid shows it: Nashville degrees of the key's tonic, or its name.
pub fn chord_text(harte: &str, tonic: Option<i32>, settings: &Settings) -> String {
    match tonic {
        Some(tonic) => display_nashville(harte, tonic, settings.style),
        None => display_chord(harte, settings.style),
    }
}

pub fn to_sheet(song: &Song, options: &Settings) -> Sheet {
    let meta = &song.meta;
    let tonic = nashville_tonic(song, options);
    let show = |harte: &str| chord_text(harte, tonic, options);
    let mut facts = Vec::new();
    facts.extend(meta.key.as_ref().map(|key| format!("Key {}", display_key(key, options.style))));
    facts.extend(meta.tempo_bpm.map(|tempo| format!("♩ = {}", whole(tempo).map_or(tempo.to_string(), |w| w.to_string()))));
    Sheet {
        title: meta.title.clone(),
        artist: meta.artist.clone(),
        facts,
        font: options.font,
        compact: options.compact_print,
        bars_per_row: options.bars_per_row,
        sections: song
            .sections
            .iter()
            .map(|section| SheetSection {
                label: section.label.clone(),
                repeat: section.repeat.filter(|&n| n > 1),
                bars: section
                    .bars
                    .iter()
                    .map(|bar| SheetBar {
                        meter: bar.meter.as_ref().map(|m| format!("{}/{}", m.beats, m.unit)),
                        slots: bar.chords.iter().map(|slot| SheetSlot { text: show(&slot.chord), beats: slot.beats }).collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::display::DisplayStyle;
    use crate::model::song::parse_song;

    fn options() -> Settings {
        Settings {
            style: DisplayStyle::Intl,
            font: ChartFont::Kalam,
            compact_print: true,
            bars_per_row: 4,
            ..Settings::parse(&serde_json::Value::Null)
        }
    }

    fn slot(text: &str, beats: u32) -> SheetSlot {
        SheetSlot { text: text.into(), beats }
    }

    #[test]
    fn to_sheet_of_a_song_spells_its_chords_and_header() {
        // Given a song with a repeated section, a split bar, a meter override and an empty section
        let song = parse_song(
            r#"{ "version": 1, "meta": { "title": "Song 1", "artist": "The Fixtures", "key": "D", "tempoBpm": 120, "meter": { "beats": 4, "unit": 4 } }, "sections": [
                { "id": "a", "label": "Verse", "repeat": 2, "bars": [
                    { "chords": [{ "chord": "C:maj7", "beats": 4 }] },
                    { "meter": { "beats": 2, "unit": 4 }, "chords": [{ "chord": "A:min", "beats": 1 }, { "chord": "N", "beats": 1 }] }
                ] },
                { "id": "b", "label": "Outro", "repeat": 1, "bars": [] }
            ] }"#,
        )
        .unwrap();

        // When
        let sheet = to_sheet(&song, &options());

        // Then
        assert_eq!(
            sheet,
            Sheet {
                title: "Song 1".into(),
                artist: Some("The Fixtures".into()),
                facts: vec!["Key D".into(), "♩ = 120".into()],
                font: ChartFont::Kalam,
                compact: true,
                bars_per_row: 4,
                sections: vec![
                    SheetSection {
                        label: "Verse".into(),
                        repeat: Some(2),
                        bars: vec![
                            SheetBar { meter: None, slots: vec![slot("Cmaj7", 4)] },
                            SheetBar { meter: Some("2/4".into()), slots: vec![slot("Am", 1), slot("N.C.", 1)] },
                        ],
                    },
                    SheetSection { label: "Outro".into(), repeat: None, bars: vec![] },
                ],
            }
        );
    }

    #[test]
    fn to_sheet_of_a_nashville_song_spells_degrees_of_the_key() {
        // Given a Nashville song in G without tempo or artist
        let song = parse_song(
            r#"{ "version": 1, "meta": { "title": "N", "key": "G", "notation": "nashville", "meter": { "beats": 4, "unit": 4 } }, "sections": [
                { "id": "a", "label": "A", "bars": [{ "chords": [{ "chord": "G:maj", "beats": 2 }, { "chord": "E:min", "beats": 2 }] }] }
            ] }"#,
        )
        .unwrap();

        // When
        let sheet = to_sheet(&song, &options());

        // Then
        assert_eq!(sheet.facts, ["Key G"]);
        assert_eq!(sheet.sections[0].bars[0].slots, [slot("1", 2), slot("6m", 2)]);
    }

    #[test]
    fn to_sheet_writes_a_fractional_tempo_as_is() {
        // Given / When
        let mut song = parse_song(
            r#"{ "version": 1, "meta": { "title": "T", "meter": { "beats": 4, "unit": 4 } }, "sections": [{ "id": "a", "label": "A", "bars": [] }] }"#,
        )
        .unwrap();
        song.meta.tempo_bpm = Some(119.6);

        // Then
        assert_eq!(to_sheet(&song, &options()).facts, ["♩ = 119.6"]);
    }
}
