//! The chord parser (F-ED-3): common spellings in, canonical Harte out.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHarte<'a> {
    pub root: &'a str,
    pub quality: &'a str,
    pub bass: Option<&'a str>,
}

#[rustfmt::skip]
const ALIASES: &[(&str, &str)] = &[
    ("", "maj"), ("maj", "maj"), ("M", "maj"),
    ("m", "min"), ("min", "min"), ("-", "min"),
    ("+", "aug"), ("aug", "aug"),
    ("°", "dim"), ("o", "dim"), ("dim", "dim"),
    ("sus", "sus4"), ("sus4", "sus4"), ("sus2", "sus2"),
    ("add2", "add2"), ("add9", "add2"), ("add4", "add4"), ("add11", "add4"),
    ("madd2", "minadd2"), ("madd9", "minadd2"), ("m(add2)", "minadd2"), ("m(add9)", "minadd2"), ("-add9", "minadd2"),
    ("madd4", "minadd4"), ("madd11", "minadd4"), ("m(add4)", "minadd4"), ("m(add11)", "minadd4"),
    ("7sus4", "7sus4"), ("7sus", "7sus4"), ("9sus4", "9sus4"), ("9sus", "9sus4"),
    ("69", "69"), ("6add9", "69"), ("6(9)", "69"), ("m69", "min69"), ("m6add9", "min69"), ("m6(9)", "min69"),
    ("m11", "min11"), ("min11", "min11"), ("-11", "min11"),
    ("maj13", "maj13"), ("7M13", "maj13"), ("M13", "maj13"), ("Δ13", "maj13"),
    ("7", "7"),
    ("7M", "maj7"), ("maj7", "maj7"), ("M7", "maj7"), ("Δ", "maj7"), ("Δ7", "maj7"), ("ma7", "maj7"),
    ("m7", "min7"), ("min7", "min7"), ("-7", "min7"),
    ("m7M", "minmaj7"), ("m(maj7)", "minmaj7"), ("mM7", "minmaj7"), ("mmaj7", "minmaj7"), ("minmaj7", "minmaj7"),
    ("-Δ", "minmaj7"),
    ("6", "maj6"), ("m6", "min6"), ("min6", "min6"), ("-6", "min6"),
    ("°7", "dim7"), ("o7", "dim7"), ("dim7", "dim7"),
    ("m7b5", "hdim7"), ("ø", "hdim7"), ("ø7", "hdim7"), ("-7b5", "hdim7"), ("hdim7", "hdim7"),
    ("9", "9"), ("maj9", "maj9"), ("7M9", "maj9"), ("m9", "min9"), ("11", "11"), ("13", "13"),
];

/// Every Harte quality, in the order the aliases first name them.
pub fn qualities() -> Vec<&'static str> {
    let mut seen = Vec::new();
    for &(_, quality) in ALIASES {
        if !seen.contains(&quality) {
            seen.push(quality);
        }
    }
    seen
}

fn is_quality(quality: &str) -> bool {
    ALIASES.iter().any(|&(_, q)| q == quality)
}

pub const DEGREES: [&str; 12] = ["1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7"];

pub fn degree_index(degree: &str) -> Option<i32> {
    DEGREES.iter().position(|&d| d == degree).map(|i| i as i32)
}

/// The leading note of `text` ("F#" of "F#m7"), if it starts with one.
pub fn leading_note(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if !matches!(bytes.first(), Some(b'A'..=b'G')) {
        return None;
    }
    Some(if matches!(bytes.get(1), Some(b'#' | b'b')) { &text[..2] } else { &text[..1] })
}

fn is_note(text: &str) -> bool {
    leading_note(text) == Some(text)
}

/// Pitch class of a note, unbounded ("Cb" is -1).
pub fn semitone(note: &str) -> i32 {
    let natural = match note.as_bytes()[0] {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        _ => 11,
    };
    natural
        + match note.as_bytes().get(1) {
            Some(b'#') => 1,
            Some(b'b') => -1,
            _ => 0,
        }
}

pub struct Key<'a> {
    pub root: &'a str,
    pub is_minor: bool,
}

/// A key is "C", "Am", "F#m"; Harte ("A:min") is read too.
pub fn parse_key(key: &str) -> Key<'_> {
    Key { root: leading_note(key).unwrap_or("C"), is_minor: key.ends_with('m') || key.ends_with(":min") }
}

/// Unicode ♭ and ♯ typed as b and #.
pub fn normalize_input(input: &str) -> String {
    input.trim().replace('♭', "b").replace('♯', "#")
}

pub fn parse_chord(input: &str) -> Result<String, String> {
    let text = normalize_input(input);
    if text == "N" || text == "N.C." || text == "NC" {
        return Ok("N".into());
    }
    if text == "%" {
        return Ok("%".into());
    }
    let Some(root) = leading_note(&text) else {
        return Err(format!("\"{input}\" doesn't start with a note (A–G, optional # or b)"));
    };
    let rest = &text[root.len()..];
    let (alias, bass_note) = match rest.split_once('/') {
        Some((alias, bass)) => (alias, Some(bass)),
        None => (rest, None),
    };
    let Some(&(_, quality)) = ALIASES.iter().find(|&&(a, _)| a == alias) else {
        return Err(format!("unknown chord quality \"{alias}\""));
    };
    let Some(bass_note) = bass_note else {
        return Ok(format!("{root}:{quality}"));
    };
    if !is_note(bass_note) {
        return Err(format!("bass \"{bass_note}\" isn't a note (A–G, optional # or b)"));
    }
    let degree = DEGREES[(semitone(bass_note) - semitone(root)).rem_euclid(12) as usize];
    Ok(format!("{root}:{quality}/{degree}"))
}

/// None for anything that isn't a "Root:quality[/degree]" chord, including "N" and "%".
pub fn parse_harte(harte: &str) -> Option<ParsedHarte<'_>> {
    let (root, rest) = harte.split_once(':')?;
    let (quality, bass) = match rest.split_once('/') {
        Some((quality, bass)) => (quality, Some(bass)),
        None => (rest, None),
    };
    let valid = is_note(root) && is_quality(quality) && bass.is_none_or(|b| degree_index(b).is_some());
    valid.then_some(ParsedHarte { root, quality, bass })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCEPTED: &[(&str, &str)] = &[
        // F-ED-3 examples
        ("Am7", "A:min7"),
        ("A-7", "A:min7"),
        ("Amin7", "A:min7"),
        ("C7M", "C:maj7"),
        ("Cmaj7", "C:maj7"),
        ("CΔ", "C:maj7"),
        ("C/E", "C:maj/3"),
        ("Csus", "C:sus4"),
        ("C°", "C:dim"),
        // §4.1 Q1–Q11
        ("C", "C:maj"),
        ("Cm", "C:min"),
        ("C+", "C:aug"),
        ("Cdim", "C:dim"),
        ("Csus2", "C:sus2"),
        ("Csus4", "C:sus4"),
        ("Cadd9", "C:add2"),
        ("Cadd2", "C:add2"),
        ("Cadd11", "C:add4"),
        ("Cadd4", "C:add4"),
        ("Amadd9", "A:minadd2"),
        ("Am(add2)", "A:minadd2"),
        ("Amadd11", "A:minadd4"),
        ("Am(add4)", "A:minadd4"),
        ("G7sus4", "G:7sus4"),
        ("G9sus", "G:9sus4"),
        ("C69", "C:69"),
        ("C6(9)", "C:69"),
        ("Cm69", "C:min69"),
        ("Cm11", "C:min11"),
        ("Cmaj13", "C:maj13"),
        ("C7", "C:7"),
        ("CmM7", "C:minmaj7"),
        ("C-Δ", "C:minmaj7"),
        ("Cm(maj7)", "C:minmaj7"),
        ("C6", "C:maj6"),
        ("Cm6", "C:min6"),
        ("Co7", "C:dim7"),
        ("Cm7b5", "C:hdim7"),
        ("Cø", "C:hdim7"),
        ("C/G", "C:maj/5"),
        ("C/Bb", "C:maj/b7"),
        ("Am/G", "A:min/b7"),
        ("N.C.", "N"),
        ("NC", "N"),
        ("N", "N"),
        ("%", "%"),
        // extensions (manual only)
        ("C9", "C:9"),
        ("C7M9", "C:maj9"),
        ("Cm9", "C:min9"),
        ("C11", "C:11"),
        ("C13", "C:13"),
        // accidentals, spelling kept, bass degrees beyond /3 /5 /b7
        ("F#m", "F#:min"),
        ("Bbmaj7", "Bb:maj7"),
        ("E♭", "Eb:maj"),
        ("D♯m7♭5", "D#:hdim7"),
        ("Db/Eb", "Db:maj/2"),
        ("C/Eb", "C:maj/b3"),
        ("C/Ab", "C:maj/b6"),
        ("C/F#", "C:maj/b5"),
        ("G/F#", "G:maj/7"),
        ("B/C", "B:maj/b2"),
        (" Cadd2/D ", "C:add2/2"),
    ];

    #[test]
    fn parse_chord_of_accepted_spellings_returns_harte() {
        for &(input, harte) in ACCEPTED {
            // Given / When / Then
            assert_eq!(parse_chord(input), Ok(harte.to_string()), "{input}");
        }
    }

    #[test]
    fn parse_chord_of_invalid_input_returns_error() {
        for input in ["H", "Cx7", "C/", "C/H", "", "Cmaj7/E/G", "c", "Cm(add13)"] {
            // Given / When
            let result = parse_chord(input);

            // Then
            assert!(result.as_ref().is_err_and(|e| !e.trim().is_empty()), "{input}: {result:?}");
        }
    }

    #[test]
    fn parse_harte_of_parsed_chord_round_trips() {
        for &(_, harte) in ACCEPTED.iter().filter(|(_, h)| h.contains(':')) {
            // Given / When
            let parsed = parse_harte(harte).unwrap_or_else(|| panic!("{harte}"));

            // Then
            let bass = parsed.bass.map(|b| format!("/{b}")).unwrap_or_default();
            assert_eq!(format!("{}:{}{bass}", parsed.root, parsed.quality), harte);
        }
    }

    #[test]
    fn parse_harte_of_engine_bass_degree_parses() {
        for harte in ["C:maj/b3", "C:min/2", "C:maj/b6", "A:7/b5"] {
            // Given / When / Then
            assert_eq!(parse_harte(harte).and_then(|p| p.bass), harte.split('/').nth(1), "{harte}");
        }
    }

    #[test]
    fn parse_harte_of_non_chords_returns_none() {
        for harte in ["N", "%", "C", "C:foo", "H:maj", "C:maj/#9", "C:maj/3/5"] {
            // Given / When / Then
            assert_eq!(parse_harte(harte), None, "{harte}");
        }
    }

    #[test]
    fn parse_harte_of_slash_chord_splits_root_quality_and_bass() {
        // Given / When / Then
        assert_eq!(parse_harte("Db:min7/b7"), Some(ParsedHarte { root: "Db", quality: "min7", bass: Some("b7") }));
        assert_eq!(parse_harte("A:min"), Some(ParsedHarte { root: "A", quality: "min", bass: None }));
    }
}
