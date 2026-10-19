//! ChordPro 6 grid export (spec F-EX-2): one {start_of_grid} per section, one cell per beat.

use crate::model::display::{DisplayStyle, display_chord};
use crate::model::song::{Bar, Song};

const BARS_PER_LINE: usize = 4;

// International spelling, except the two names outside ChordPro's built-in extension list.
fn chord_name(harte: &str) -> String {
    display_chord(harte, DisplayStyle::Intl).replace("m(maj7)", "mmaj7").replace("ø7", "m7b5")
}

// shortcut: a bar gets as many cells as its own beats, so a meter-override bar is narrower than the grid shape says.
fn cells(bar: &Bar) -> String {
    let cells: Vec<String> = bar
        .chords
        .iter()
        .flat_map(|slot| std::iter::once(chord_name(&slot.chord)).chain(std::iter::repeat_n(".".into(), slot.beats as usize - 1)))
        .collect();
    cells.join(" ")
}

pub fn to_chord_pro(song: &Song) -> String {
    let meta = &song.meta;
    let mut header = vec![format!("{{title: {}}}", meta.title)];
    header.extend(meta.artist.as_ref().filter(|a| !a.is_empty()).map(|a| format!("{{artist: {a}}}")));
    header.extend(meta.key.as_ref().filter(|k| !k.is_empty()).map(|k| format!("{{key: {k}}}")));
    header.extend(meta.tempo_bpm.map(|t| format!("{{tempo: {}}}", t.round())));
    header.push(format!("{{time: {}/{}}}", meta.meter.beats, meta.meter.unit));
    let grids = song.sections.iter().map(|section| {
        let mut lines: Vec<String> = section
            .bars
            .chunks(BARS_PER_LINE)
            .map(|bars| format!("| {} |", bars.iter().map(cells).collect::<Vec<_>>().join(" | ")))
            .collect();
        // The section ends on a double bar; its repeat count goes in the right margin.
        if let Some(last) = lines.last_mut() {
            last.push('|');
            if let Some(repeat) = section.repeat {
                last.push_str(&format!(" x{repeat}"));
            }
        }
        let start = format!("{{start_of_grid shape=\"1+{BARS_PER_LINE}x{}+1\" label=\"{}\"}}", meta.meter.beats, section.label);
        [vec![start], lines, vec!["{end_of_grid}".into()]].concat().join("\n")
    });
    let blocks: Vec<String> = std::iter::once(header.join("\n")).chain(grids).collect();
    format!("{}\n", blocks.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::song::parse_song;

    #[test]
    fn to_chord_pro_of_a_song_matches_the_golden_file() {
        // Given a song with a repeated section, a split bar, a slash chord, a repeat bar, N, and a 2-beat bar
        let song = parse_song(
            r#"{
                "version": 1,
                "meta": { "title": "Song 1", "artist": "The Fixtures", "key": "C", "tempoBpm": 119.6, "meter": { "beats": 4, "unit": 4 } },
                "sections": [
                    { "id": "a", "label": "Verse", "repeat": 2, "bars": [
                        { "chords": [{ "chord": "C:maj7", "beats": 4 }] },
                        { "chords": [{ "chord": "A:min", "beats": 2 }, { "chord": "G:maj/3", "beats": 2 }] },
                        { "chords": [{ "chord": "%", "beats": 4 }] },
                        { "chords": [{ "chord": "N", "beats": 4 }] },
                        { "chords": [{ "chord": "F:maj", "beats": 4 }] }
                    ] },
                    { "id": "b", "label": "Chorus", "bars": [
                        { "chords": [{ "chord": "B:hdim7", "beats": 4 }] },
                        { "meter": { "beats": 2, "unit": 4 }, "chords": [{ "chord": "E:minmaj7", "beats": 2 }] },
                        { "chords": [{ "chord": "D:min7", "beats": 4 }] }
                    ] }
                ]
            }"#,
        )
        .unwrap();

        // When / Then
        assert_eq!(to_chord_pro(&song), include_str!("golden/song1.cho"));
    }
}
