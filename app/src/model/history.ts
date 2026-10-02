import type { Song } from "./song";

// ponytail: full Song snapshots instead of inverse commands (cheap thanks to structural sharing);
// revisit only if memory becomes a problem.
export type History = { readonly past: readonly Song[]; readonly present: Song; readonly future: readonly Song[] };

// Net semitones a snapshot is transposed by, kept beside the song (not in it, so it never reaches the file)
// and inherited by edits, so undo/redo always see the right value.
const transpositions = new WeakMap<Song, number>();
export const transposition = (song: Song): number => transpositions.get(song) ?? 0;
export const setTransposition = (song: Song, semitones: number): Song => (transpositions.set(song, semitones), song);

export const reset = (song: Song): History => ({ past: [], present: song, future: [] });

export const apply = (h: History, fn: (s: Song) => Song): History => {
  const next = fn(h.present);
  if (!transpositions.has(next)) transpositions.set(next, transposition(h.present));
  return next === h.present ? h : { past: [...h.past, h.present], present: next, future: [] };
};

export const canUndo = (h: History): boolean => h.past.length > 0;
export const canRedo = (h: History): boolean => h.future.length > 0;

export const undo = (h: History): History =>
  canUndo(h) ? { past: h.past.slice(0, -1), present: h.past[h.past.length - 1], future: [h.present, ...h.future] } : h;

export const redo = (h: History): History =>
  canRedo(h) ? { past: [...h.past, h.present], present: h.future[0], future: h.future.slice(1) } : h;
