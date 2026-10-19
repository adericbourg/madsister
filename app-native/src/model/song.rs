//! The Song file format (spec §4). Unknown fields are kept on every object so a load/save never drops them (NF-7).
//! Known fields are written in the engine's order (`engine/madsister_engine/song.py`).

use std::hash::{BuildHasher, RandomState};

use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};

type Extra = Map<String, Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meter {
    pub beats: u32,
    pub unit: u32,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChordSlot {
    /// Harte, e.g. "C:maj7/3", "A:min", "N"; syntax is checked by the chord parser, not here.
    pub chord: String,
    pub beats: u32,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "js_number_opt")]
    pub confidence: Option<f64>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bar {
    pub chords: Vec<ChordSlot>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "js_number_opt")]
    pub start_sec: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meter: Option<Meter>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
    pub label: String,
    pub bars: Vec<Bar>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<u32>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Notation {
    Chords,
    Nashville,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub title: String,
    pub meter: Meter,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Absent = chords; a Nashville song without a key keeps its chords relative to C.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notation: Option<Notation>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "js_number_opt")]
    pub tempo_bpm: Option<f64>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Audio {
    pub path: String,
    pub sha256: String,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub version: u32,
    pub meta: Meta,
    pub sections: Vec<Section>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<Audio>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// The value as an integer when it is a whole number JavaScript writes without a fraction (`120`, not `120.0`).
pub fn whole(value: f64) -> Option<i64> {
    (value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0).then_some(value as i64)
}

/// Writes whole numbers without a fraction, as the TypeScript app saved them.
fn js_number_opt<S: Serializer>(value: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error> {
    match *value {
        Some(v) => match whole(v) {
            Some(w) => serializer.serialize_i64(w),
            None => serializer.serialize_f64(v),
        },
        None => serializer.serialize_none(),
    }
}

fn check_meter(meter: &Meter, path: &str) -> Result<(), String> {
    if meter.beats == 0 {
        return Err(format!("{path}.beats: expected a positive integer, got 0"));
    }
    if ![2, 4, 8].contains(&meter.unit) {
        return Err(format!("{path}.unit: expected one of 2, 4, 8, got {}", meter.unit));
    }
    Ok(())
}

fn check(song: &Song) -> Result<(), String> {
    if song.version != 1 {
        return Err(format!("unsupported version {}", song.version));
    }
    check_meter(&song.meta.meter, "meta.meter")?;
    // The editor always has a section to put its cursor in; deleting the last one leaves an empty one.
    if song.sections.is_empty() {
        return Err("sections: a song needs at least one section".into());
    }
    for (s, section) in song.sections.iter().enumerate() {
        if section.repeat == Some(0) {
            return Err(format!("sections[{s}].repeat: expected a positive integer, got 0"));
        }
        for (b, bar) in section.bars.iter().enumerate() {
            let path = format!("sections[{s}].bars[{b}]");
            if let Some(meter) = &bar.meter {
                check_meter(meter, &format!("{path}.meter"))?;
            }
            for (c, slot) in bar.chords.iter().enumerate() {
                let slot_path = format!("{path}.chords[{c}]");
                if slot.chord.is_empty() {
                    return Err(format!("{slot_path}.chord: expected a non-empty string"));
                }
                if slot.beats == 0 {
                    return Err(format!("{slot_path}.beats: expected a positive integer, got 0"));
                }
                if let Some(confidence) = slot.confidence.filter(|c| !(0.0..=1.0).contains(c)) {
                    return Err(format!("{slot_path}.confidence: expected a number in [0, 1], got {confidence}"));
                }
            }
            let expected = bar_beats(song, bar);
            let actual: u32 = bar.chords.iter().map(|slot| slot.beats).sum();
            if actual != expected {
                return Err(format!("{path}: slot beats sum to {actual}, meter expects {expected}"));
            }
        }
    }
    Ok(())
}

/// Validates a loaded file and returns it as a Song; the error message starts with the offending path.
pub fn parse_song(json: &str) -> Result<Song, String> {
    let deserializer = &mut serde_json::Deserializer::from_str(json);
    let song: Song = serde_path_to_error::deserialize(deserializer).map_err(|e| e.to_string())?;
    check(&song)?;
    Ok(song)
}

pub fn serialize_song(song: &Song) -> String {
    serde_json::to_string_pretty(song).expect("a Song always serializes")
}

pub fn new_section_id() -> String {
    format!("{:08x}", RandomState::new().hash_one(std::time::SystemTime::now()) as u32)
}

pub fn empty_song(meter: Meter, notation: Option<Notation>) -> Song {
    let bar = Bar {
        chords: vec![ChordSlot { chord: "N".into(), beats: meter.beats, confidence: None, extra: Extra::new() }],
        start_sec: None,
        meter: None,
        extra: Extra::new(),
    };
    Song {
        version: 1,
        meta: Meta {
            title: "Untitled".into(),
            meter,
            artist: None,
            key: None,
            notation,
            tempo_bpm: None,
            extra: Extra::new(),
        },
        sections: vec![Section {
            id: new_section_id(),
            label: "Verse".into(),
            bars: vec![bar; 4],
            repeat: None,
            extra: Extra::new(),
        }],
        audio: None,
        extra: Extra::new(),
    }
}

pub fn bar_beats(song: &Song, bar: &Bar) -> u32 {
    bar.meter.as_ref().unwrap_or(&song.meta.meter).beats
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn meter(beats: u32, unit: u32) -> Meter {
        Meter { beats, unit, extra: Extra::new() }
    }

    fn valid_file() -> Value {
        json!({
            "version": 1,
            "producedBy": "engine 0.1",
            "meta": { "title": "Song", "artist": "Someone", "key": "A:min", "tempoBpm": 120, "meter": { "beats": 4, "unit": 4 }, "mood": "sad" },
            "audio": { "path": "song.mp3", "sha256": "abc" },
            "sections": [{
                "id": "s1",
                "label": "Verse",
                "repeat": 2,
                "color": "red",
                "bars": [
                    { "startSec": 0.5, "chords": [{ "chord": "A:min", "beats": 4, "confidence": 0.9, "source": "btc" }], "lyric": "la" },
                    { "meter": { "beats": 2, "unit": 4, "compound": true }, "chords": [{ "chord": "C:maj/b6", "beats": 1 }, { "chord": "G:add2/2", "beats": 1 }] }
                ]
            }]
        })
    }

    fn parse(file: &Value) -> Result<Song, String> {
        parse_song(&file.to_string())
    }

    fn round_trip(song: &Song) -> Song {
        parse_song(&serialize_song(song)).unwrap()
    }

    #[test]
    fn parse_song_of_file_with_unknown_fields_and_bar_meter_override_round_trips_through_serialize_song() {
        // Given a file with unknown fields at every level and a 2/4 bar in a 4/4 song
        let file = valid_file();

        // When
        let serialized = serialize_song(&parse(&file).unwrap());

        // Then
        assert_eq!(serde_json::from_str::<Value>(&serialized).unwrap(), file);
    }

    #[test]
    fn serialize_song_of_an_engine_written_file_gives_it_back_byte_for_byte() {
        // Given a file in the engine's key order, with a whole tempo written the TypeScript way
        let file = r#"{
  "version": 1,
  "meta": {
    "title": "Song",
    "meter": {
      "beats": 4,
      "unit": 4
    },
    "key": "A:min",
    "tempoBpm": 120
  },
  "sections": [
    {
      "id": "s1",
      "label": "Verse",
      "bars": [
        {
          "chords": [
            {
              "chord": "A:min",
              "beats": 4,
              "confidence": 0.9
            }
          ],
          "startSec": 0.5
        }
      ]
    }
  ],
  "audio": {
    "path": "song.mp3",
    "sha256": "abc"
  }
}"#;

        // When / Then
        assert_eq!(serialize_song(&parse_song(file).unwrap()), file);
    }

    #[test]
    fn parse_song_when_invalid_returns_error_with_path() {
        type Mutation = fn(&mut Value);
        let cases: [(&str, Mutation, &str); 11] = [
            ("wrong version", |f| f["version"] = json!(2), "unsupported version 2"),
            ("beat sum mismatch", |f| f["sections"][0]["bars"][0]["chords"][0]["beats"] = json!(3), "sections[0].bars[0]"),
            (
                "beat sum mismatch against the override",
                |f| f["sections"][0]["bars"][1]["chords"][0]["beats"] = json!(3),
                "sections[0].bars[1]",
            ),
            (
                "non-integer beats",
                |f| f["sections"][0]["bars"][0]["chords"][0]["beats"] = json!(2.5),
                "sections[0].bars[0].chords[0].beats",
            ),
            (
                "zero beats",
                |f| f["sections"][0]["bars"][1]["chords"][1]["beats"] = json!(0),
                "sections[0].bars[1].chords[1].beats",
            ),
            (
                "empty chord",
                |f| f["sections"][0]["bars"][0]["chords"][0]["chord"] = json!(""),
                "sections[0].bars[0].chords[0].chord",
            ),
            ("missing sections", |f| _ = f.as_object_mut().unwrap().remove("sections"), "sections"),
            ("no section", |f| f["sections"] = json!([]), "sections: a song needs at least one section"),
            ("unsupported meter unit", |f| f["meta"]["meter"]["unit"] = json!(16), "meta.meter.unit"),
            ("zero meter beats", |f| f["meta"]["meter"]["beats"] = json!(0), "meta.meter.beats"),
            ("unknown notation", |f| f["meta"]["notation"] = json!("roman"), "meta.notation"),
        ];
        for (name, mutate, message) in cases {
            // Given
            let mut file = valid_file();
            mutate(&mut file);

            // When
            let error = parse(&file).unwrap_err();

            // Then
            assert!(error.contains(message), "{name}: {error:?} should mention {message:?}");
        }
    }

    #[test]
    fn parse_song_of_file_without_notation_leaves_it_none() {
        // Given / When / Then
        assert_eq!(parse(&valid_file()).unwrap().meta.notation, None);
    }

    #[test]
    fn empty_song_returns_a_valid_song_with_one_verse_of_four_empty_bars() {
        // Given / When
        let song = empty_song(meter(3, 4), None);

        // Then
        assert_eq!(round_trip(&song), song);
        assert_eq!(song.meta.title, "Untitled");
        assert_eq!(song.meta.meter, meter(3, 4));
        assert_eq!(song.sections.len(), 1);
        let section = &song.sections[0];
        assert_eq!(section.label, "Verse");
        assert!(section.id.len() == 8 && section.id.chars().all(|c| c.is_ascii_hexdigit()), "{}", section.id);
        assert_eq!(section.bars.len(), 4);
        assert!(section.bars.iter().all(|bar| bar.chords.len() == 1 && bar.chords[0].chord == "N" && bar.chords[0].beats == 3));
        assert_eq!(bar_beats(&song, &section.bars[0]), 3);
    }

    #[test]
    fn empty_song_of_an_uncommon_meter_round_trips_with_bars_summing_to_the_meter() {
        // Given / When
        let song = empty_song(meter(5, 2), None);

        // Then
        assert_eq!(round_trip(&song), song);
        assert_eq!(bar_beats(&song, &song.sections[0].bars[0]), 5);
    }

    #[test]
    fn empty_song_of_nashville_has_the_notation_and_no_key() {
        // Given / When
        let song = empty_song(meter(4, 4), Some(Notation::Nashville));

        // Then
        assert_eq!(song.meta.notation, Some(Notation::Nashville));
        assert_eq!(song.meta.key, None);
        assert_eq!(round_trip(&song), song);
        assert_eq!(empty_song(meter(4, 4), None).meta.notation, None);
    }

    #[test]
    fn new_section_id_differs_between_calls() {
        // Given / When / Then
        assert_ne!(new_section_id(), new_section_id());
    }
}
