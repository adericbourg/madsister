//! How chords are written on screen (F-DS-1, F-DS-2) and spoken to screen readers.

use super::chord::{degree_index, parse_harte, parse_key, semitone};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayStyle {
    Fr,
    Intl,
    Latin,
}

/// The quality's suffix after the root: "maj7" → "7M" in French, "maj7" in the international style.
pub fn suffix(style: DisplayStyle, quality: &str) -> &'static str {
    if style == DisplayStyle::Intl {
        match quality {
            "maj7" => return "maj7",
            "minmaj7" => return "m(maj7)",
            "dim" => return "dim",
            "dim7" => return "dim7",
            "hdim7" => return "ø7",
            "maj9" => return "maj9",
            "maj13" => return "maj13",
            _ => {}
        }
    }
    match quality {
        "maj" => "",
        "min" => "m",
        "aug" => "+",
        "dim" => "°",
        "sus2" => "sus2",
        "sus4" => "sus4",
        "add2" => "add2",
        "add4" => "add4",
        "minadd2" => "madd2",
        "minadd4" => "madd4",
        "7sus4" => "7sus4",
        "9sus4" => "9sus4",
        "69" => "69",
        "min69" => "m69",
        "min11" => "m11",
        "maj13" => "7M13",
        "7" => "7",
        "maj7" => "7M",
        "min7" => "m7",
        "minmaj7" => "m7M",
        "maj6" => "6",
        "min6" => "m6",
        "dim7" => "°7",
        "hdim7" => "m7b5",
        "9" => "9",
        "maj9" => "7M9",
        "min9" => "m9",
        "11" => "11",
        "13" => "13",
        _ => unreachable!("not a Harte quality: {quality}"),
    }
}

/// The key as written in the chart header: "F#m" → "Fa#m" in the latin style.
pub fn display_key(key: &str, style: DisplayStyle) -> String {
    let key = parse_key(key);
    display_chord(&format!("{}:{}", key.root, if key.is_minor { "min" } else { "maj" }), style)
}

const SHARPS: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const FLATS: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];

pub fn note_name(pitch_class: i32, is_flat: bool) -> &'static str {
    (if is_flat { FLATS } else { SHARPS })[pitch_class.rem_euclid(12) as usize]
}

fn latin(note: &str) -> String {
    let name = match note.as_bytes()[0] {
        b'C' => "Do",
        b'D' => "Ré",
        b'E' => "Mi",
        b'F' => "Fa",
        b'G' => "Sol",
        b'A' => "La",
        _ => "Si",
    };
    format!("{name}{}", &note[1..])
}

pub fn display_chord(harte: &str, style: DisplayStyle) -> String {
    if harte == "N" {
        return "N.C.".into();
    }
    let Some(parsed) = parse_harte(harte) else {
        return harte.into();
    };
    let spell = |note: &str| if style == DisplayStyle::Latin { latin(note) } else { note.to_string() };
    let chord = spell(parsed.root) + suffix(style, parsed.quality);
    let Some(bass) = parsed.bass else {
        return chord;
    };
    let is_flat = parsed.root.as_bytes().get(1) == Some(&b'b') || bass.starts_with('b');
    let bass_note = note_name(semitone(parsed.root) + degree_index(bass).expect("parse_harte checked it"), is_flat);
    format!("{chord}/{}", spell(bass_note))
}

/// The quality as spoken after the root, e.g. " minor 7".
pub fn spoken(quality: &str) -> &'static str {
    match quality {
        "maj" => "",
        "min" => " minor",
        "aug" => " augmented",
        "dim" => " diminished",
        "sus2" => " suspended 2",
        "sus4" => " suspended 4",
        "add2" => " add 2",
        "add4" => " add 4",
        "minadd2" => " minor add 2",
        "minadd4" => " minor add 4",
        "7sus4" => " 7 suspended 4",
        "9sus4" => " 9 suspended 4",
        "69" => " 6 9",
        "min69" => " minor 6 9",
        "min11" => " minor 11",
        "maj13" => " major 13",
        "7" => " 7",
        "maj7" => " major 7",
        "min7" => " minor 7",
        "minmaj7" => " minor major 7",
        "maj6" => " 6",
        "min6" => " minor 6",
        "dim7" => " diminished 7",
        "hdim7" => " half-diminished 7",
        "9" => " 9",
        "maj9" => " major 9",
        "min9" => " minor 9",
        "11" => " 11",
        "13" => " 13",
        _ => unreachable!("not a Harte quality: {quality}"),
    }
}

fn speak_note(note: &str) -> String {
    let accidental = match note.as_bytes().get(1) {
        Some(b'#') => " sharp",
        Some(b'b') => " flat",
        _ => "",
    };
    format!("{}{accidental}", &note[..1])
}

/// Spells a chord for screen readers, e.g. "Bb:maj7/3" → "B flat major 7 over D".
pub fn speak_chord(harte: &str) -> String {
    match harte {
        "N" => return "no chord".into(),
        "%" => return "repeat previous bar".into(),
        _ => {}
    }
    let Some(parsed) = parse_harte(harte) else {
        return harte.into();
    };
    let displayed = display_chord(harte, DisplayStyle::Intl);
    let over = displayed.split_once('/').map(|(_, bass)| format!(" over {}", speak_note(bass))).unwrap_or_default();
    format!("{}{}{over}", speak_note(parsed.root), spoken(parsed.quality))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::chord::{parse_chord, qualities};

    // (harte, fr, intl)
    const ROWS: &[(&str, &str, &str)] = &[
        ("C:maj", "C", "C"),
        ("C:min", "Cm", "Cm"),
        ("C:aug", "C+", "C+"),
        ("C:dim", "C°", "Cdim"),
        ("C:sus2", "Csus2", "Csus2"),
        ("C:sus4", "Csus4", "Csus4"),
        ("C:add2", "Cadd2", "Cadd2"),
        ("C:add4", "Cadd4", "Cadd4"),
        ("C:minadd2", "Cmadd2", "Cmadd2"),
        ("C:minadd4", "Cmadd4", "Cmadd4"),
        ("C:7sus4", "C7sus4", "C7sus4"),
        ("C:9sus4", "C9sus4", "C9sus4"),
        ("C:69", "C69", "C69"),
        ("C:min69", "Cm69", "Cm69"),
        ("C:min11", "Cm11", "Cm11"),
        ("C:maj13", "C7M13", "Cmaj13"),
        ("C:7", "C7", "C7"),
        ("C:maj7", "C7M", "Cmaj7"),
        ("C:min7", "Cm7", "Cm7"),
        ("C:minmaj7", "Cm7M", "Cm(maj7)"),
        ("C:maj6", "C6", "C6"),
        ("C:min6", "Cm6", "Cm6"),
        ("C:dim7", "C°7", "Cdim7"),
        ("B:hdim7", "Bm7b5", "Bø7"),
        ("C:9", "C9", "C9"),
        ("C:maj9", "C7M9", "Cmaj9"),
        ("C:min9", "Cm9", "Cm9"),
        ("C:11", "C11", "C11"),
        ("C:13", "C13", "C13"),
        ("C:maj/3", "C/E", "C/E"),
        ("A:min/b7", "Am/G", "Am/G"),
        ("Eb:maj/5", "Eb/Bb", "Eb/Bb"),
        ("C:maj/b6", "C/Ab", "C/Ab"),
        ("C:maj/b5", "C/Gb", "C/Gb"),
        ("G:maj/7", "G/F#", "G/F#"),
        ("F#:min7/b7", "F#m7/E", "F#m7/E"),
        ("Db:maj7/2", "Db7M/Eb", "Dbmaj7/Eb"),
        ("N", "N.C.", "N.C."),
        ("%", "%", "%"),
    ];

    #[test]
    fn rows_cover_every_quality() {
        // Given / When
        let covered: Vec<&str> = ROWS.iter().filter_map(|(harte, ..)| harte.split([':', '/']).nth(1)).collect();

        // Then
        assert_eq!(qualities().into_iter().filter(|q| !covered.contains(q)).collect::<Vec<_>>(), Vec::<&str>::new());
    }

    #[test]
    fn every_quality_has_a_suffix_in_every_style_and_a_spoken_name() {
        for quality in qualities() {
            // Given / When / Then: no unreachable! panic
            for style in [DisplayStyle::Fr, DisplayStyle::Intl, DisplayStyle::Latin] {
                suffix(style, quality);
            }
            spoken(quality);
        }
    }

    #[test]
    fn display_chord_returns_fr_and_intl_spellings() {
        for &(harte, fr, intl) in ROWS {
            // Given / When / Then
            assert_eq!(display_chord(harte, DisplayStyle::Fr), fr, "{harte}");
            assert_eq!(display_chord(harte, DisplayStyle::Intl), intl, "{harte}");
        }
    }

    #[test]
    fn display_chord_returns_latin_roots_with_french_suffixes() {
        for (harte, latin) in [
            ("C:maj", "Do"),
            ("D:min", "Rém"),
            ("E:7", "Mi7"),
            ("F:maj7", "Fa7M"),
            ("G:dim", "Sol°"),
            ("A:min7", "Lam7"),
            ("B:hdim7", "Sim7b5"),
            ("Db:maj7/2", "Réb7M/Mib"),
            ("F#:min7/b7", "Fa#m7/Mi"),
            ("C:maj/3", "Do/Mi"),
            ("N", "N.C."),
            ("%", "%"),
        ] {
            // Given / When / Then
            assert_eq!(display_chord(harte, DisplayStyle::Latin), latin, "{harte}");
        }
    }

    #[test]
    fn display_chord_parses_back_to_same_harte() {
        for &(harte, fr, intl) in ROWS {
            // Given / When / Then
            assert_eq!(parse_chord(fr), Ok(harte.to_string()), "{fr}");
            assert_eq!(parse_chord(intl), Ok(harte.to_string()), "{intl}");
        }
    }

    #[test]
    fn speak_chord_spells_it_for_screen_readers() {
        // Given / When / Then
        assert_eq!(speak_chord("A:min7"), "A minor 7");
        assert_eq!(speak_chord("Bb:maj7/3"), "B flat major 7 over D");
        assert_eq!(speak_chord("F#:hdim7"), "F sharp half-diminished 7");
        assert_eq!(speak_chord("C:maj"), "C");
        assert_eq!(speak_chord("N"), "no chord");
        assert_eq!(speak_chord("%"), "repeat previous bar");
    }

    #[test]
    fn display_key_writes_the_key_in_the_style() {
        for (key, style, expected) in [
            ("F#m", DisplayStyle::Latin, "Fa#m"),
            ("F#m", DisplayStyle::Fr, "F#m"),
            ("Bb", DisplayStyle::Latin, "Sib"),
            ("Am", DisplayStyle::Intl, "Am"),
            ("C", DisplayStyle::Fr, "C"),
            ("C:min", DisplayStyle::Fr, "Cm"),
        ] {
            // Given / When / Then
            assert_eq!(display_key(key, style), expected, "{key} {style:?}");
        }
    }
}
