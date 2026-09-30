import { parseHarte, semitone } from "./chord";
import { noteName } from "./display";
import type { Song } from "./song";

export type Spelling = "sharp" | "flat";

// Respells the leading note of a chord or key ("F#:7/3", "Am", "A:min"); the rest is relative to it, so it stays.
const transposeRoot = (text: string, semitones: number, spelling: Spelling): string =>
  text.replace(/^[A-G][#b]?/, (root) => noteName(semitone(root) + semitones, spelling === "flat"));

export const transposeChord = (harte: string, semitones: number, spelling: Spelling): string =>
  parseHarte(harte) ? transposeRoot(harte, semitones, spelling) : harte;

// Keeps `confidence`: transposing isn't a correction of the chord.
export const transposeSong = (song: Song, semitones: number, spelling: Spelling): Song => ({
  ...song,
  meta: { ...song.meta, key: song.meta.key && transposeRoot(song.meta.key, semitones, spelling) },
  sections: song.sections.map((section) => ({
    ...section,
    bars: section.bars.map((bar) => ({
      ...bar,
      chords: bar.chords.map((slot) => ({ ...slot, chord: transposeChord(slot.chord, semitones, spelling) })),
    })),
  })),
});
