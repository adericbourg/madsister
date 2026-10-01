// Pure editing commands (spec F-ED-3/4/5/6/8): each returns a new Song and shares every untouched object with the input.
import { newSectionId, type Bar, type ChordSlot, type Meter, type Section, type Song } from "./song";

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

/** Sets the bar's meter (F-ED-11): slots are cut from the end, or the last slot grows. `null` or the song meter removes the override. */
export const setBarMeter = (song: Song, ref: BarRef, meter: Meter | null): Song =>
  updateBar(song, ref, ({ meter: _, ...bar }) => {
    const target = meter ?? song.meta.meter;
    let left = target.beats;
    const chords = bar.chords.flatMap((slot) => {
      const beats = Math.min(slot.beats, left);
      left -= beats;
      return beats > 0 ? [{ ...slot, beats }] : [];
    });
    const last = chords.length - 1;
    chords[last] = { ...chords[last], beats: chords[last].beats + left };
    const isSongMeter = target.beats === song.meta.meter.beats && target.unit === song.meta.meter.unit;
    return isSongMeter ? { ...bar, chords } : { ...bar, meter: target, chords };
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

const updateSection = (song: Song, index: number, fn: (section: Section) => Section): Song => {
  at(song.sections, index, "section");
  return { ...song, sections: song.sections.map((section, i) => (i === index ? fn(section) : section)) };
};

const newSection = (song: Song, label: string): Section => ({
  id: newSectionId(),
  label,
  bars: [{ chords: [{ chord: "N", beats: song.meta.meter.beats }] }],
});

export const addSection = (song: Song, index: number, label: string): Song => ({
  ...song,
  sections: spliced(song.sections, index, 0, newSection(song, label)),
});

export const renameSection = (song: Song, index: number, label: string): Song =>
  updateSection(song, index, (section) => ({ ...section, label }));

/** Moves the section so that it ends up at index `to`. */
export const moveSection = (song: Song, from: number, to: number): Song => {
  const section = at(song.sections, from, "section");
  at(song.sections, to, "section");
  return { ...song, sections: spliced(spliced(song.sections, from, 1), to, 0, section) };
};

// A song always keeps at least one section.
export const deleteSection = (song: Song, index: number): Song => {
  at(song.sections, index, "section");
  const sections = spliced(song.sections, index, 1);
  return { ...song, sections: sections.length > 0 ? sections : [newSection(song, "Song")] };
};

/** Bars from `ref` onward go to a new section labelled `<label> (2)`, inserted right after; the repeat count stays on the first part. */
export const splitSection = (song: Song, ref: BarRef): Song => {
  barAt(song, ref);
  const section = song.sections[ref.section];
  const { repeat: _, ...rest } = section;
  const tail = { ...rest, id: newSectionId(), label: `${section.label} (2)`, bars: section.bars.slice(ref.bar) };
  const head = { ...section, bars: section.bars.slice(0, ref.bar) };
  return { ...song, sections: spliced(song.sections, ref.section, 1, head, tail) };
};

/** `n <= 1` removes the repeat count. */
export const setRepeat = (song: Song, index: number, n: number): Song =>
  updateSection(song, index, ({ repeat: _, ...section }) => (n > 1 ? { ...section, repeat: n } : section));

/** Bars `from..to` (inclusive) of one section. */
export const copyBars = (song: Song, from: BarRef, to: BarRef): Bar[] => {
  if (from.section !== to.section || to.bar < from.bar) throw new Error("copy range must be ordered and within one section");
  barAt(song, from);
  barAt(song, to);
  return song.sections[from.section].bars.slice(from.bar, to.bar + 1);
};

/** Pasted bars lose `startSec`; a bar whose beats don't match the song meter gets a `meter` override. */
export const pasteBars = (song: Song, ref: BarRef, bars: readonly Bar[], position: "before" | "after"): Song => {
  at(song.sections, ref.section, "section");
  // A pasted chord is the user's decision, like a typed one: it loses its low-confidence flag.
  const pasted = bars.map(({ startSec: _, ...copied }): Bar => {
    const bar = { ...copied, chords: copied.chords.map(({ confidence: _, ...slot }) => slot) };
    const beats = bar.chords.reduce((sum, slot) => sum + slot.beats, 0);
    return bar.meter || beats === song.meta.meter.beats ? bar : { ...bar, meter: { beats, unit: song.meta.meter.unit } };
  });
  return updateBars(song, ref, (existing) => spliced(existing, position === "before" ? ref.bar : ref.bar + 1, 0, ...pasted));
};
