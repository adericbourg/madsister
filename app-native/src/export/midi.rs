//! Standard MIDI File export (spec F-EX-4): type 0, one track of chord pads (close voicing + bass) following the recording.

use super::bars::played_bars;
use crate::model::chord::{degree_index, parse_harte, semitone};
use crate::model::song::{Bar, Meter, Song};

const PPQ: u32 = 480; // ticks per quarter note
const VELOCITY: u8 = 80;

// Semitones above the root, root position, close voicing (11 drops its 3rd, 13 its 5th and 11th).
fn voicing(quality: &str) -> &'static [i32] {
    match quality {
        "maj" => &[0, 4, 7],
        "min" => &[0, 3, 7],
        "aug" => &[0, 4, 8],
        "dim" => &[0, 3, 6],
        "sus2" => &[0, 2, 7],
        "sus4" => &[0, 5, 7],
        "add2" => &[0, 2, 4, 7],
        "add4" => &[0, 4, 5, 7],
        "minadd2" => &[0, 2, 3, 7],
        "minadd4" => &[0, 3, 5, 7],
        "7sus4" => &[0, 5, 7, 10],
        "9sus4" => &[0, 5, 10, 14],
        "69" => &[0, 4, 7, 9, 14],
        "min69" => &[0, 3, 7, 9, 14],
        "min11" => &[0, 3, 10, 14, 17],
        "maj13" => &[0, 4, 11, 14, 21],
        "7" => &[0, 4, 7, 10],
        "maj7" => &[0, 4, 7, 11],
        "min7" => &[0, 3, 7, 10],
        "minmaj7" => &[0, 3, 7, 11],
        "maj6" => &[0, 4, 7, 9],
        "min6" => &[0, 3, 7, 9],
        "dim7" => &[0, 3, 6, 9],
        "hdim7" => &[0, 3, 6, 10],
        "9" => &[0, 4, 7, 10, 14],
        "maj9" => &[0, 4, 7, 11, 14],
        "min9" => &[0, 3, 7, 10, 14],
        "11" => &[0, 7, 10, 14, 17],
        "13" => &[0, 4, 10, 14, 21],
        _ => unreachable!("not a Harte quality: {quality}"),
    }
}

/// The MIDI notes of a chord: the bass (root, or slash degree) in C3–B3, then the chord from its root in C4–B4.
fn notes(harte: &str) -> Vec<u8> {
    // N, and a "%" slot left in a split bar, are silent.
    let Some(parsed) = parse_harte(harte) else { return Vec::new() };
    let root = semitone(parsed.root).rem_euclid(12);
    let bass = 48 + (root + degree_index(parsed.bass.unwrap_or("1")).expect("parse_harte checked it")) % 12;
    std::iter::once(bass).chain(voicing(parsed.quality).iter().map(|interval| 60 + root + interval)).map(|n| n as u8).collect()
}

fn vlq(mut value: u32) -> Vec<u8> {
    let mut bytes = vec![(value & 0x7f) as u8];
    value >>= 7;
    while value > 0 {
        bytes.insert(0, (value & 0x7f) as u8 | 0x80);
        value >>= 7;
    }
    bytes
}

struct Track<'a> {
    bytes: Vec<u8>,
    last: u32,
    now: u32,
    meter: Option<&'a Meter>,
    tempo: Option<u32>,
}

impl<'a> Track<'a> {
    fn emit(&mut self, tick: u32, event: &[u8]) {
        self.bytes.extend(vlq(tick - self.last));
        self.bytes.extend(event);
        self.last = tick;
    }

    fn time_signature(&mut self, next: &'a Meter) {
        if self.meter.is_some_and(|m| (m.beats, m.unit) == (next.beats, next.unit)) {
            return;
        }
        self.meter = Some(next);
        self.emit(self.now, &[0xff, 0x58, 4, next.beats as u8, next.unit.trailing_zeros() as u8, 24, 8]);
    }

    fn set_tempo(&mut self, us_per_quarter: f64) {
        // shortcut: clamped to MIDI's 24-bit limit (16.7 s per quarter); only a very long leading rest gets there.
        let next = (us_per_quarter.round() as u32).min(0xff_ffff);
        if self.tempo == Some(next) {
            return;
        }
        self.tempo = Some(next);
        self.emit(self.now, &[0xff, 0x51, 3, (next >> 16) as u8, (next >> 8) as u8, next as u8]);
    }
}

fn meter_of<'a>(song: &'a Song, bar: &'a Bar) -> &'a Meter {
    bar.meter.as_ref().unwrap_or(&song.meta.meter)
}

pub fn to_midi(song: &Song) -> Vec<u8> {
    let played = played_bars(song);
    let bars: Vec<&Bar> = played.iter().map(|p| p.bar.as_ref()).collect();
    let quarters = |bar: &Bar| f64::from(meter_of(song, bar).beats) * 4.0 / f64::from(meter_of(song, bar).unit);
    let ticks = |beats: u32, bar: &Bar| beats * PPQ * 4 / meter_of(song, bar).unit;

    let mut track = Track { bytes: Vec::new(), last: 0, now: 0, meter: None, tempo: None };
    // tempoBpm counts the pulse: quarters, or dotted quarters in x/8 (spec §4).
    let pulse = if song.meta.meter.unit == 8 { 1.5 } else { 1.0 };
    let mut bar_tempo = 60e6 / (song.meta.tempo_bpm.unwrap_or(120.0) * pulse);
    // A leading rest bar lasting until the first bar's start, so the MIDI and the audio line up when both start at 0.
    if let Some(first) = bars.first().filter(|b| b.start_sec.is_some_and(|s| s > 0.0)) {
        track.time_signature(meter_of(song, first));
        track.set_tempo(first.start_sec.unwrap() * 1e6 / quarters(first));
        track.now += ticks(meter_of(song, first).beats, first);
    }
    for (i, bar) in bars.iter().enumerate() {
        // The bar's real duration sets its tempo; otherwise (untimed bar, last bar, back to a repeat) the tempo carries on.
        if let (Some(start), Some(next)) = (bar.start_sec, bars.get(i + 1).and_then(|b| b.start_sec))
            && next > start
        {
            bar_tempo = (next - start) * 1e6 / quarters(bar);
        }
        track.time_signature(meter_of(song, bar));
        track.set_tempo(bar_tempo);
        for slot in &bar.chords {
            let length = ticks(slot.beats, bar);
            let chord = notes(&slot.chord);
            let now = track.now;
            for &note in &chord {
                track.emit(now, &[0x90, note, VELOCITY]);
            }
            for &note in &chord {
                track.emit(now + length, &[0x80, note, 0]);
            }
            track.now += length;
        }
    }
    track.emit(track.now, &[0xff, 0x2f, 0]);

    let mut file = b"MThd".to_vec();
    file.extend(6u32.to_be_bytes());
    file.extend([0, 0, 0, 1]);
    file.extend((PPQ as u16).to_be_bytes());
    file.extend(b"MTrk");
    file.extend((track.bytes.len() as u32).to_be_bytes());
    file.extend(track.bytes);
    file
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::chord::qualities;
    use crate::model::song::parse_song;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
    }

    fn tempos(bytes: &[u8]) -> Vec<u32> {
        bytes
            .windows(6)
            .filter(|w| w[..3] == [0xff, 0x51, 3])
            .map(|w| (u32::from(w[3]) << 16) | (u32::from(w[4]) << 8) | u32::from(w[5]))
            .collect()
    }

    #[test]
    fn to_midi_of_a_two_bar_song_matches_the_golden_bytes() {
        // Given a 2-bar 4/4 song without tempo: C, then G/B and N for 2 beats each
        let song = parse_song(
            r#"{ "version": 1, "meta": { "title": "Tiny", "meter": { "beats": 4, "unit": 4 } }, "sections": [{ "id": "a", "label": "Verse", "bars": [
                { "chords": [{ "chord": "C:maj", "beats": 4 }] },
                { "chords": [{ "chord": "G:maj/3", "beats": 2 }, { "chord": "N", "beats": 2 }] }
            ] }] }"#,
        )
        .unwrap();

        // When
        let midi = to_midi(&song);

        // Then: type 0, 480 PPQ, 4/4 at 120 BPM, C3+C4 E4 G4 for a whole note, B3+G4 B4 D5 for a half note, a half rest
        assert_eq!(
            hex(&midi),
            [
                "4d 54 68 64 00 00 00 06 00 00 00 01 01 e0",
                "4d 54 72 6b 00 00 00 56",
                "00 ff 58 04 04 02 18 08",
                "00 ff 51 03 07 a1 20",
                "00 90 30 50 00 90 3c 50 00 90 40 50 00 90 43 50",
                "8f 00 80 30 00 00 80 3c 00 00 80 40 00 00 80 43 00",
                "00 90 3b 50 00 90 43 50 00 90 47 50 00 90 4a 50",
                "87 40 80 3b 00 00 80 43 00 00 80 47 00 00 80 4a 00",
                "87 40 ff 2f 00",
            ]
            .join(" ")
        );
    }

    #[test]
    fn to_midi_when_bars_have_start_sec_follows_the_recording_after_a_leading_rest() {
        // Given bars at 1 s, 3 s and 4 s, the second one in 2/4, in a section played twice
        let song = parse_song(
            r#"{ "version": 1, "meta": { "title": "Timed", "tempoBpm": 90, "meter": { "beats": 4, "unit": 4 } }, "sections": [
                { "id": "a", "label": "Verse", "repeat": 2, "bars": [
                    { "startSec": 1, "chords": [{ "chord": "A:min", "beats": 4 }] },
                    { "startSec": 3, "meter": { "beats": 2, "unit": 4 }, "chords": [{ "chord": "%", "beats": 2 }] },
                    { "startSec": 4, "chords": [{ "chord": "N", "beats": 4 }] }
                ] }
            ] }"#,
        )
        .unwrap();

        // When
        let midi = to_midi(&song);

        // Then the header and track chunks are consistent and the track ends with end-of-track
        assert_eq!(hex(&midi[..14]), "4d 54 68 64 00 00 00 06 00 00 00 01 01 e0");
        assert_eq!(hex(&midi[14..18]), "4d 54 72 6b");
        assert_eq!(u32::from_be_bytes(midi[18..22].try_into().unwrap()) as usize, midi.len() - 22);
        assert_eq!(hex(&midi[midi.len() - 3..]), "ff 2f 00");
        // a 1 s rest bar (4 quarters → 250 ms each), then 2 s per 4/4 bar, 1 s per 2/4 bar, the last bars keep the previous tempo
        assert_eq!(tempos(&midi), [250_000, 500_000]);
        // the time signature switches to 2/4 and back to 4/4 on each pass
        let signatures: Vec<u8> = midi.windows(5).filter(|w| w[..3] == [0xff, 0x58, 4] && w[4] == 2).map(|w| w[3]).collect();
        assert_eq!(signatures, [4, 2, 4, 2, 4]);
    }

    #[test]
    fn to_midi_of_every_quality_voices_a_chord_from_the_root() {
        // Given one bar per quality
        let bars: Vec<String> =
            qualities().iter().map(|q| format!(r#"{{ "chords": [{{ "chord": "C:{q}", "beats": 4 }}] }}"#)).collect();
        let song = parse_song(&format!(
            r#"{{ "version": 1, "meta": {{ "title": "T", "meter": {{ "beats": 4, "unit": 4 }} }}, "sections": [{{ "id": "a", "label": "A", "bars": [{}] }}] }}"#,
            bars.join(",")
        ))
        .unwrap();

        // When / Then: no quality lacks a voicing (it would panic)
        assert!(!to_midi(&song).is_empty());
    }
}
