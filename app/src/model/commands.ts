// Pure editing commands (spec F-ED-3/4/5/8): each returns a new Song and shares every untouched object with the input.
import type { Bar, ChordSlot, Song } from "./song";

export type BarRef = { readonly section: number; readonly bar: number };
export type SlotRef = BarRef & { readonly slot: number };

const at = <T>(items: readonly T[], index: number, what: string): T => {
  if (index < 0 || index >= items.length) throw new Error(`no ${what} at index ${index}`);
  return items[index];
};

// Array.prototype.toSpliced is ES2023; tsconfig targets ES2020.
const spliced = <T>(items: readonly T[], start: number, deleteCount: number, ...inserted: T[]): T[] => [
  ...items.slice(0, start),
  ...inserted,
  ...items.slice(start + deleteCount),
];

/** Replaces the bars of `ref.section` with `fn(bars)`. */
const updateBars = (song: Song, ref: BarRef, fn: (bars: readonly Bar[]) => readonly Bar[]): Song => ({
  ...song,
  sections: song.sections.map((section, i) =>
    i === ref.section ? { ...section, bars: fn(section.bars) } : section,
  ),
});

const barAt = (song: Song, ref: BarRef): Bar => at(at(song.sections, ref.section, "section").bars, ref.bar, "bar");

export const updateBar = (song: Song, ref: BarRef, fn: (bar: Bar) => Bar): Song => {
  barAt(song, ref);
  return updateBars(song, ref, (bars) => bars.map((bar, i) => (i === ref.bar ? fn(bar) : bar)));
};

const updateSlots = (song: Song, ref: SlotRef, fn: (slots: readonly ChordSlot[], slot: ChordSlot) => readonly ChordSlot[]): Song =>
  updateBar(song, ref, (bar) => ({ ...bar, chords: fn(bar.chords, at(bar.chords, ref.slot, "slot")) }));

export const setChord = (song: Song, ref: SlotRef, harte: string): Song =>
  updateSlots(song, ref, (slots, { confidence: _, ...rest }) => spliced(slots, ref.slot, 1, { ...rest, chord: harte }));

export const splitSlot = (song: Song, ref: SlotRef): Song =>
  updateSlots(song, ref, (slots, slot) => {
    if (slot.beats < 2) throw new Error("cannot split a 1-beat slot");
    const half = Math.floor(slot.beats / 2);
    return spliced(slots, ref.slot, 1, { ...slot, beats: slot.beats - half }, { ...slot, beats: half });
  });

export const mergeSlotWithNext = (song: Song, ref: SlotRef): Song =>
  updateSlots(song, ref, (slots, slot) => {
    const next = at(slots, ref.slot + 1, "next slot");
    return spliced(slots, ref.slot, 2, { ...slot, beats: slot.beats + next.beats });
  });

/** Moves one beat between the slot and its next neighbour (previous one for the last slot); a slot left with 0 beats is removed. */
export const resizeSlot = (song: Song, ref: SlotRef, delta: 1 | -1): Song =>
  updateSlots(song, ref, (slots) => {
    if (slots.length < 2) throw new Error("cannot resize the only slot of a bar");
    const other = ref.slot === slots.length - 1 ? ref.slot - 1 : ref.slot + 1;
    const beats = slots.map((s, i) => (i === ref.slot ? s.beats + delta : i === other ? s.beats - delta : s.beats));
    return slots.map((s, i) => ({ ...s, beats: beats[i] })).filter((_, i) => beats[i] > 0);
  });

// Only the section is checked, so that `{bar: 0}` inserts into an empty section.
export const insertBar = (song: Song, ref: BarRef, position: "before" | "after"): Song => {
  at(song.sections, ref.section, "section");
  const bar: Bar = { chords: [{ chord: "N", beats: song.meta.meter.beats }] };
  return updateBars(song, ref, (bars) => spliced(bars, position === "before" ? ref.bar : ref.bar + 1, 0, bar));
};

export const deleteBar = (song: Song, ref: BarRef): Song => {
  barAt(song, ref);
  return updateBars(song, ref, (bars) => spliced(bars, ref.bar, 1));
};

export const duplicateBar = (song: Song, ref: BarRef): Song => {
  const { startSec: _, ...copy } = barAt(song, ref);
  return updateBars(song, ref, (bars) => spliced(bars, ref.bar + 1, 0, copy));
};
