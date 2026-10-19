//! Nashville numbers: chords are stored absolute (Harte) and shown as degrees of the song's tonic.
//! A Nashville song without a key keeps its chords relative to C, so C is degree 1.

use super::chord::{DEGREES, degree_index, normalize_input, parse_chord, parse_harte, parse_key, semitone};
use super::display::{DisplayStyle, note_name, spoken, suffix};
use super::transpose::Spelling;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinorConvention {
    Relative,
    Tonic,
}

const MAJOR_SCALE: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const SHARP_TONICS: [i32; 6] = [2, 4, 6, 7, 9, 11];

/// Pitch class of degree 1: C without a key; a minor key's relative major with the "relative" convention (Am → C).
pub fn tonic_pc(key: Option<&str>, convention: MinorConvention) -> i32 {
    let Some(key) = key else { return 0 };
    let key = parse_key(key);
    if key.is_minor && convention == MinorConvention::Relative {
        (semitone(key.root) + 3).rem_euclid(12)
    } else {
        semitone(key.root)
    }
}

pub fn spelling_for(tonic: i32) -> Spelling {
    if SHARP_TONICS.contains(&tonic.rem_euclid(12)) { Spelling::Sharp } else { Spelling::Flat }
}

fn degree_of(pc: i32, tonic: i32) -> &'static str {
    DEGREES[(pc - tonic).rem_euclid(12) as usize]
}

pub fn display_nashville(harte: &str, tonic: i32, style: DisplayStyle) -> String {
    if harte == "N" {
        return "N.C.".into();
    }
    let Some(parsed) = parse_harte(harte) else {
        return harte.into();
    };
    let root = semitone(parsed.root);
    let chord = format!("{}{}", degree_of(root, tonic), suffix(style, parsed.quality));
    match parsed.bass {
        None => chord,
        Some(bass) => {
            format!("{chord}/{}", degree_of(root + degree_index(bass).expect("parse_harte checked it"), tonic))
        }
    }
}

// "b3" → the note a minor third above the tonic, spelled with the key's accidentals unless the degree says otherwise.
fn degree_note(accidental: &str, degree: u8, tonic: i32) -> &'static str {
    let shift = match accidental {
        "b" => -1,
        "#" => 1,
        _ => 0,
    };
    let pc = tonic + MAJOR_SCALE[usize::from(degree - 1)] + shift;
    note_name(pc, accidental == "b" || (accidental.is_empty() && spelling_for(tonic) == Spelling::Flat))
}

/// Splits a leading "b" or "#" and a degree 1–7 off `text`: "b7maj7" → ("b", 7, "maj7").
fn split_degree(text: &str) -> Option<(&str, u8, &str)> {
    let accidental = if text.starts_with(['b', '#']) { &text[..1] } else { "" };
    let rest = &text[accidental.len()..];
    let degree = rest.bytes().next().filter(|d| (b'1'..=b'7').contains(d))? - b'0';
    Some((accidental, degree, &rest[1..]))
}

pub fn parse_nashville(input: &str, tonic: i32) -> Result<String, String> {
    let text = normalize_input(input);
    if matches!(text.as_str(), "N" | "N.C." | "NC" | "%") {
        return parse_chord(&text);
    }
    let error = || format!("\"{input}\" isn't a degree (1–7, optional b or #)");
    let (accidental, degree, rest) = split_degree(&text).ok_or_else(error)?;
    let (alias, bass) = match rest.split_once('/') {
        None => (rest, String::new()),
        Some((alias, bass)) => match split_degree(bass) {
            Some((bass_accidental, bass_degree, "")) => {
                (alias, format!("/{}", degree_note(bass_accidental, bass_degree, tonic)))
            }
            _ => return Err(error()),
        },
    };
    parse_chord(&format!("{}{alias}{bass}", degree_note(accidental, degree, tonic)))
}

fn speak_degree(degree: &str) -> String {
    degree.replacen('b', "flat ", 1).replacen('#', "sharp ", 1)
}

/// Spells a Nashville chord for screen readers, e.g. "Bb:maj7" in C → "flat 7 major 7".
pub fn speak_nashville(harte: &str, tonic: i32) -> String {
    match harte {
        "N" => return "no chord".into(),
        "%" => return "repeat previous bar".into(),
        _ => {}
    }
    let Some(parsed) = parse_harte(harte) else {
        return harte.into();
    };
    let displayed = display_nashville(harte, tonic, DisplayStyle::Intl);
    let over = displayed.split_once('/').map(|(_, bass)| format!(" over {}", speak_degree(bass))).unwrap_or_default();
    format!("{}{}{over}", speak_degree(degree_of(semitone(parsed.root), tonic)), spoken(parsed.quality))
}

#[cfg(test)]
mod tests {
    use super::*;
    use MinorConvention::{Relative, Tonic};

    #[test]
    fn tonic_pc_of_key_with_convention() {
        for (key, convention, expected) in [
            (None, Relative, 0),
            (None, Tonic, 0),
            (Some("G"), Relative, 7),
            (Some("Bb"), Tonic, 10),
            (Some("Am"), Relative, 0),
            (Some("Am"), Tonic, 9),
            (Some("F#m"), Relative, 9),
            (Some("F#m"), Tonic, 6),
            (Some("C:min"), Relative, 3),
            (Some("C:min"), Tonic, 0),
        ] {
            // Given / When / Then
            assert_eq!(tonic_pc(key, convention), expected, "{key:?} {convention:?}");
        }
    }

    #[test]
    fn spelling_for_sharp_keys_returns_sharp_and_otherwise_flat() {
        // Given / When / Then
        assert!([7, 2, 9, 6].into_iter().all(|t| spelling_for(t) == Spelling::Sharp));
        assert!([0, 5, 10, 3].into_iter().all(|t| spelling_for(t) == Spelling::Flat));
    }

    #[test]
    fn display_nashville_shows_degrees_of_the_tonic() {
        for (harte, tonic, style, expected) in [
            ("G:maj", 7, DisplayStyle::Intl, "1"),
            ("C:maj", 7, DisplayStyle::Intl, "4"),
            ("E:min", 7, DisplayStyle::Intl, "6m"),
            ("F:7", 0, DisplayStyle::Intl, "47"),
            ("Bb:maj7", 0, DisplayStyle::Intl, "b7maj7"),
            ("Bb:maj7", 0, DisplayStyle::Fr, "b77M"),
            ("C:maj/3", 0, DisplayStyle::Intl, "1/3"),
            ("C:maj/3", 7, DisplayStyle::Intl, "4/6"),
            ("N", 7, DisplayStyle::Intl, "N.C."),
            ("%", 7, DisplayStyle::Intl, "%"),
        ] {
            // Given / When / Then
            assert_eq!(display_nashville(harte, tonic, style), expected, "{harte} in {tonic}");
        }
    }

    #[test]
    fn parse_nashville_returns_harte() {
        for (input, tonic, harte) in [
            ("1", 7, "G:maj"),
            ("4", 7, "C:maj"),
            ("6m", 7, "E:min"),
            ("47", 0, "F:7"),
            ("b7maj7", 0, "Bb:maj7"),
            ("♭3m", 0, "Eb:min"),
            ("#4°", 0, "F#:dim"),
            ("7", 7, "F#:maj"),
            ("4/6", 7, "C:maj/3"),
            ("N", 7, "N"),
            ("%", 7, "%"),
        ] {
            // Given / When / Then
            assert_eq!(parse_nashville(input, tonic), Ok(harte.to_string()), "{input} in {tonic}");
        }
    }

    #[test]
    fn parse_nashville_of_invalid_input_returns_an_error() {
        for input in ["Am", "8", "0m", "4xyz", "4/x", ""] {
            // Given / When / Then
            assert!(parse_nashville(input, 0).is_err(), "{input}");
        }
    }

    #[test]
    fn parse_nashville_of_letter_chord_explains_degrees_are_expected() {
        // Given / When / Then
        assert_eq!(parse_nashville("Am", 0), Err("\"Am\" isn't a degree (1–7, optional b or #)".to_string()));
    }

    #[test]
    fn parse_nashville_then_display_nashville_round_trips_every_degree() {
        // Given every pitch class as root, in every key
        for tonic in 0..12 {
            for degree in DEGREES {
                // When
                let parsed = parse_nashville(&format!("{degree}m7/5"), tonic).unwrap();

                // Then the bass is counted from the key, like the root
                assert_eq!(display_nashville(&parsed, tonic, DisplayStyle::Intl), format!("{degree}m7/5"));
            }
        }
    }

    #[test]
    fn speak_nashville_spells_the_degrees_for_screen_readers() {
        // Given / When / Then
        assert_eq!(speak_nashville("E:min", 7), "6 minor");
        assert_eq!(speak_nashville("Bb:maj7", 0), "flat 7 major 7");
        assert_eq!(speak_nashville("C:maj/3", 7), "4 over 6");
        assert_eq!(speak_nashville("N", 7), "no chord");
        assert_eq!(speak_nashville("%", 7), "repeat previous bar");
    }
}
