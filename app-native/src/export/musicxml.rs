//! MusicXML 4.0 export (spec F-EX-3): one measure per bar, a <harmony> per slot over one slash note per beat.

use super::bars::played_bars;
use crate::model::chord::parse_harte;
use crate::model::display::{DisplayStyle, display_chord};
use crate::model::song::{Meter, Song};

fn kind(quality: &str) -> &'static str {
    match quality {
        "maj" | "add2" | "add4" => "major",
        "min" | "minadd2" | "minadd4" => "minor",
        "aug" => "augmented",
        "dim" => "diminished",
        "sus2" => "suspended-second",
        "sus4" | "7sus4" | "9sus4" => "suspended-fourth",
        "69" | "maj6" => "major-sixth",
        "min69" | "min6" => "minor-sixth",
        "min11" => "minor-11th",
        "maj13" => "major-13th",
        "7" => "dominant",
        "maj7" => "major-seventh",
        "min7" => "minor-seventh",
        "minmaj7" => "major-minor",
        "dim7" => "diminished-seventh",
        "hdim7" => "half-diminished",
        "9" => "dominant-ninth",
        "maj9" => "major-ninth",
        "min9" => "minor-ninth",
        "11" => "dominant-11th",
        "13" => "dominant-13th",
        _ => unreachable!("not a Harte quality: {quality}"),
    }
}

// Spelled like the app ("Bbadd2"), not as the equivalent add9/add11: (value, alter) of each added degree.
fn added(quality: &str) -> &'static [(u8, i8)] {
    match quality {
        "add2" | "minadd2" => &[(2, 0)],
        "add4" | "minadd4" => &[(4, 0)],
        "7sus4" => &[(7, -1)],
        "9sus4" => &[(7, -1), (9, 0)],
        "69" | "min69" => &[(9, 0)],
        _ => &[],
    }
}

const DIVISIONS: u32 = 2; // per quarter note, so an eighth-note beat is 1

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn pitch(element: &str, note: &str) -> String {
    let alter = match note.as_bytes().get(1) {
        Some(b'#') => format!("<{element}-alter>1</{element}-alter>"),
        Some(_) => format!("<{element}-alter>-1</{element}-alter>"),
        None => String::new(),
    };
    format!("<{element}><{element}-step>{}</{element}-step>{alter}</{element}>", &note[..1])
}

fn harmony(harte: &str) -> Option<String> {
    if harte == "N" {
        return Some(r#"<harmony><root><root-step text="">C</root-step></root><kind text="N.C.">none</kind></harmony>"#.into());
    }
    let parsed = parse_harte(harte)?;
    let displayed = display_chord(harte, DisplayStyle::Intl);
    let bass = displayed.split_once('/').map(|(_, bass)| pitch("bass", bass)).unwrap_or_default();
    let degrees: String = added(parsed.quality)
        .iter()
        .map(|(value, alter)| {
            format!("<degree><degree-value>{value}</degree-value><degree-alter>{alter}</degree-alter><degree-type>add</degree-type></degree>")
        })
        .collect();
    Some(format!("<harmony>{}<kind>{}</kind>{bass}{degrees}</harmony>", pitch("root", parsed.root), kind(parsed.quality)))
}

fn slash(unit: u32) -> String {
    let kind = match unit {
        2 => "half",
        4 => "quarter",
        _ => "eighth",
    };
    format!(
        "<note><pitch><step>B</step><octave>4</octave></pitch><duration>{}</duration><voice>1</voice><type>{kind}</type><stem>none</stem><notehead>slash</notehead></note>",
        DIVISIONS * 4 / unit
    )
}

fn time(meter: &Meter) -> String {
    format!("<time><beats>{}</beats><beat-type>{}</beat-type></time>", meter.beats, meter.unit)
}

pub fn to_music_xml(song: &Song) -> String {
    let bars = played_bars(song);
    // Repeated sections are written out, each pass under its own rehearsal mark.
    let meter_of = |i: usize| bars[i].bar.meter.as_ref().unwrap_or(&song.meta.meter);
    let measures = bars.iter().enumerate().map(|(i, played)| {
        let meter = meter_of(i);
        let mut lines = Vec::new();
        if i == 0 {
            lines.push(format!("<attributes><divisions>{DIVISIONS}</divisions>{}<clef><sign>G</sign><line>2</line></clef></attributes>", time(meter)));
        } else if (meter.beats, meter.unit) != (meter_of(i - 1).beats, meter_of(i - 1).unit) {
            lines.push(format!("<attributes>{}</attributes>", time(meter)));
        }
        if let Some(label) = played.label {
            lines.push(format!(r#"<direction placement="above"><direction-type><rehearsal>{}</rehearsal></direction-type></direction>"#, escape(label)));
        }
        for slot in &played.bar.chords {
            lines.extend(harmony(&slot.chord));
            lines.extend(std::iter::repeat_n(slash(meter.unit), slot.beats as usize));
        }
        let body: Vec<String> = lines.iter().map(|line| format!("      {line}")).collect();
        [vec![format!("    <measure number=\"{}\">", i + 1)], body, vec!["    </measure>".into()]].concat().join("\n")
    });
    let mut out = vec![
        r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string(),
        r#"<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">"#.into(),
        r#"<score-partwise version="4.0">"#.into(),
        format!("  <work><work-title>{}</work-title></work>", escape(&song.meta.title)),
    ];
    if let Some(artist) = song.meta.artist.as_ref().filter(|a| !a.is_empty()) {
        out.push(format!(r#"  <identification><creator type="composer">{}</creator></identification>"#, escape(artist)));
    }
    out.push(r#"  <part-list><score-part id="P1"><part-name>Chords</part-name></score-part></part-list>"#.into());
    out.push(r#"  <part id="P1">"#.into());
    out.extend(measures);
    out.extend(["  </part>".into(), "</score-partwise>".into(), String::new()]);
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::chord::qualities;
    use crate::model::song::parse_song;

    fn one_section(meter: &str, bars: &[String]) -> Song {
        parse_song(&format!(
            r#"{{ "version": 1, "meta": {{ "title": "T", "meter": {meter} }}, "sections": [{{ "id": "a", "label": "A", "bars": [{}] }}] }}"#,
            bars.join(",")
        ))
        .unwrap()
    }

    fn bar(chord: &str, beats: u32) -> String {
        format!(r#"{{ "chords": [{{ "chord": "{chord}", "beats": {beats} }}] }}"#)
    }

    const FOUR_FOUR: &str = r#"{ "beats": 4, "unit": 4 }"#;

    #[test]
    fn to_music_xml_of_a_song_matches_the_golden_file() {
        // Given a song with a repeated section, a split bar, a slash chord, a repeat bar, N, add2, and a 2-beat bar
        let song = parse_song(
            r#"{
                "version": 1,
                "meta": { "title": "Song <1> & Co", "artist": "The Fixtures", "meter": { "beats": 4, "unit": 4 } },
                "sections": [
                    { "id": "a", "label": "Verse", "repeat": 2, "bars": [
                        { "chords": [{ "chord": "C:maj7", "beats": 4 }] },
                        { "chords": [{ "chord": "A:min", "beats": 2 }, { "chord": "G:maj/3", "beats": 2 }] },
                        { "chords": [{ "chord": "%", "beats": 4 }] },
                        { "chords": [{ "chord": "N", "beats": 4 }] }
                    ] },
                    { "id": "b", "label": "Chorus", "bars": [
                        { "chords": [{ "chord": "Bb:add2", "beats": 4 }] },
                        { "meter": { "beats": 2, "unit": 4 }, "chords": [{ "chord": "E:minmaj7", "beats": 2 }] },
                        { "chords": [{ "chord": "B:hdim7", "beats": 4 }] }
                    ] }
                ]
            }"#,
        )
        .unwrap();

        // When / Then
        assert_eq!(to_music_xml(&song), include_str!("golden/song1.musicxml"));
    }

    #[test]
    fn to_music_xml_of_a_half_note_unit_meter_writes_half_note_slashes() {
        // Given a 3/2 song with one bar
        let song = one_section(r#"{ "beats": 3, "unit": 2 }"#, &[bar("C:maj", 3)]);

        // When
        let xml = to_music_xml(&song);

        // Then
        assert!(xml.contains("<beats>3</beats><beat-type>2</beat-type>"));
        assert_eq!(xml.matches("<type>half</type>").count(), 3);
    }

    #[test]
    fn to_music_xml_of_every_quality_writes_a_harmony_each() {
        // Given one bar per quality
        let bars: Vec<String> = qualities().iter().map(|q| bar(&format!("C:{q}"), 4)).collect();

        // When: no unreachable! panic
        let xml = to_music_xml(&one_section(FOUR_FOUR, &bars));

        // Then
        assert_eq!(xml.matches("<harmony>").count(), qualities().len());
    }

    #[test]
    fn to_music_xml_of_a_7sus4_adds_the_flat_seventh() {
        // Given / When / Then
        assert!(to_music_xml(&one_section(FOUR_FOUR, &[bar("G:7sus4", 4)])).contains(
            "<kind>suspended-fourth</kind><degree><degree-value>7</degree-value><degree-alter>-1</degree-alter><degree-type>add</degree-type></degree>"
        ));
    }
}
