// Standard MIDI File export (spec F-EX-4): type 0, one track of chord pads (close voicing + bass) following the recording.
import { DEGREES, parseHarte, semitone } from "../chord";
import type { Bar, Meter, Song } from "../song";
import { playedBars } from "./bars";

const PPQ = 480; // ticks per quarter note
const VELOCITY = 80;

// Semitones above the root, root position, close voicing (11 drops its 3rd, 13 its 5th and 11th).
const VOICINGS: Record<string, number[]> = {
  maj: [0, 4, 7], min: [0, 3, 7], aug: [0, 4, 8], dim: [0, 3, 6], sus2: [0, 2, 7], sus4: [0, 5, 7],
  add2: [0, 2, 4, 7], add4: [0, 4, 5, 7], minadd2: [0, 2, 3, 7], minadd4: [0, 3, 5, 7],
  "7sus4": [0, 5, 7, 10], "9sus4": [0, 5, 10, 14], "69": [0, 4, 7, 9, 14], min69: [0, 3, 7, 9, 14],
  min11: [0, 3, 10, 14, 17], maj13: [0, 4, 11, 14, 21],
  "7": [0, 4, 7, 10], maj7: [0, 4, 7, 11], min7: [0, 3, 7, 10],
  minmaj7: [0, 3, 7, 11], maj6: [0, 4, 7, 9], min6: [0, 3, 7, 9], dim7: [0, 3, 6, 9], hdim7: [0, 3, 6, 10],
  "9": [0, 4, 7, 10, 14], maj9: [0, 4, 7, 11, 14], min9: [0, 3, 7, 10, 14], "11": [0, 7, 10, 14, 17], "13": [0, 4, 10, 14, 21],
};

/** The MIDI notes of a chord: the bass (root, or slash degree) in C3–B3, then the chord from its root in C4–B4. */
const notes = (harte: string): number[] => {
  const parsed = parseHarte(harte);
  if (!parsed) return []; // N, and a "%" slot left in a split bar
  const root = (semitone(parsed.root) + 12) % 12;
  const bass = 48 + ((root + DEGREES.indexOf(parsed.bass ?? "1")) % 12);
  return [bass, ...VOICINGS[parsed.quality].map((interval) => 60 + root + interval)];
};

const vlq = (value: number): number[] => {
  const bytes = [value & 0x7f];
  while ((value >>= 7) > 0) bytes.unshift((value & 0x7f) | 0x80);
  return bytes;
};

const bytes32 = (value: number): number[] => [value >>> 24, (value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];

const ascii = (text: string): number[] => [...text].map((c) => c.charCodeAt(0));

export const toMidi = (song: Song): Uint8Array => {
  const bars = playedBars(song).map(({ bar }) => bar);
  const meterOf = (bar: Bar): Meter => bar.meter ?? song.meta.meter;
  const quarters = (bar: Bar): number => (meterOf(bar).beats * 4) / meterOf(bar).unit;

  const track: number[] = [];
  let last = 0;
  let now = 0;
  const emit = (tick: number, ...event: number[]): void => {
    track.push(...vlq(tick - last), ...event);
    last = tick;
  };
  let meter: Meter | undefined;
  const timeSignature = (next: Meter): void => {
    if (meter?.beats === next.beats && meter.unit === next.unit) return;
    meter = next;
    emit(now, 0xff, 0x58, 4, next.beats, Math.log2(next.unit), 24, 8);
  };
  let tempo: number | undefined;
  const setTempo = (usPerQuarter: number): void => {
    // ponytail: clamped to MIDI's 24-bit limit (16.7 s per quarter); only a very long leading rest gets there.
    const next = Math.min(Math.round(usPerQuarter), 0xffffff);
    if (next === tempo) return;
    tempo = next;
    emit(now, 0xff, 0x51, 3, next >> 16, (next >> 8) & 0xff, next & 0xff);
  };

  // tempoBpm counts the pulse: quarters, or dotted quarters in x/8 (spec §4).
  let barTempo = 60e6 / ((song.meta.tempoBpm ?? 120) * (song.meta.meter.unit === 8 ? 1.5 : 1));
  // A leading rest bar lasting until the first bar's startSec, so the MIDI and the audio line up when both start at 0.
  const first = bars[0];
  if (first?.startSec !== undefined && first.startSec > 0) {
    timeSignature(meterOf(first));
    setTempo((first.startSec * 1e6) / quarters(first));
    now += quarters(first) * PPQ;
  }
  bars.forEach((bar, i) => {
    // The bar's real duration sets its tempo; otherwise (untimed bar, last bar, back to a repeat) the tempo carries on.
    const next = bars[i + 1];
    if (bar.startSec !== undefined && next?.startSec !== undefined && next.startSec > bar.startSec) {
      barTempo = ((next.startSec - bar.startSec) * 1e6) / quarters(bar);
    }
    timeSignature(meterOf(bar));
    setTempo(barTempo);
    for (const slot of bar.chords) {
      const ticks = (slot.beats * PPQ * 4) / meterOf(bar).unit;
      const chord = notes(slot.chord);
      for (const note of chord) emit(now, 0x90, note, VELOCITY);
      for (const note of chord) emit(now + ticks, 0x80, note, 0);
      now += ticks;
    }
  });
  emit(now, 0xff, 0x2f, 0);

  return new Uint8Array([
    ...ascii("MThd"), ...bytes32(6), 0, 0, 0, 1, PPQ >> 8, PPQ & 0xff,
    ...ascii("MTrk"), ...bytes32(track.length), ...track,
  ]);
};
