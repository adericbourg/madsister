// Pure editing commands (spec F-ED-3/4/5/6/8): each returns a new Song and shares every untouched object with the input.
import { spellingFor, tonicPc, type MinorConvention } from "./nashville";
import { newSectionId, type Bar, type ChordSlot, type Meter, type Notation, type Section, type Song } from "./song";
import { transposeSong } from "./transpose";

export type BarRef = { readonly section: number; readonly bar: number };
export type SlotRef = BarRef & { readonly slot: number };

const at = <T>(items: readonly T[], index: number, what: string): T => {
  if (index < 0 || index >= items.length) throw new Error(`no ${what} at index ${index}`);
  return items[index];
};

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

/** In Nashville notation the numbers stay and the chords move: degree 1 goes from the old tonic (C without a key) to the new one. */
export const setKey = (song: Song, key: string | undefined, convention: MinorConvention): Song => {
  if (song.meta.notation !== "nashville") return { ...song, meta: { ...song.meta, key } };
  const to = tonicPc(key, convention);
  const moved = transposeSong(song, to - tonicPc(song.meta.key, convention), spellingFor(to));
  return { ...moved, meta: { ...moved.meta, key } };
};

export const setNotation = (song: Song, notation: Notation): Song => {
  if (song.meta.key === undefined) throw new Error("a key is required to switch the notation");
  return { ...song, meta: { ...song.meta, notation } };
};

export const setChord = (song: Song, ref: SlotRef, harte: string): Song =>
  updateSlots(song, ref, (slots, { confidence: _, ...rest }) => slots.toSpliced(ref.slot, 1, { ...rest, chord: harte }));

export const splitSlot = (song: Song, ref: SlotRef): Song =>
  updateSlots(song, ref, (slots, slot) => {
    if (slot.beats < 2) throw new Error("cannot split a 1-beat slot");
    const half = Math.floor(slot.beats / 2);
    return slots.toSpliced(ref.slot, 1, { ...slot, beats: slot.beats - half }, { ...slot, beats: half });
  });

export const mergeSlotWithNext = (song: Song, ref: SlotRef): Song =>
  updateSlots(song, ref, (slots, slot) => {
    const next = at(slots, ref.slot + 1, "next slot");
    return slots.toSpliced(ref.slot, 2, { ...slot, beats: slot.beats + next.beats });
  });

/** Removes a slot, its beats going to the previous slot (the next one for the first slot); the only slot of a bar becomes `N`. */
export const removeSlot = (song: Song, ref: SlotRef): Song =>
  updateSlots(song, ref, (slots, { confidence: _, ...slot }) => {
    if (slots.length < 2) return [{ ...slot, chord: "N" }];
    const other = ref.slot > 0 ? ref.slot - 1 : 1;
    return slots.flatMap((s, i) => (i === ref.slot ? [] : i === other ? [{ ...s, beats: s.beats + slot.beats }] : [s]));
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
  return updateBars(song, ref, (bars) => bars.toSpliced(position === "before" ? ref.bar : ref.bar + 1, 0, bar));
};

export const deleteBar = (song: Song, ref: BarRef): Song => {
  barAt(song, ref);
  return updateBars(song, ref, (bars) => bars.toSpliced(ref.bar, 1));
};

export const duplicateBar = (song: Song, ref: BarRef): Song => {
  const { startSec: _, ...copy } = barAt(song, ref);
  return updateBars(song, ref, (bars) => bars.toSpliced(ref.bar + 1, 0, copy));
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
  sections: song.sections.toSpliced(index, 0, newSection(song, label)),
});

export const renameSection = (song: Song, index: number, label: string): Song =>
  updateSection(song, index, (section) => ({ ...section, label }));

/** Moves the section so that it ends up at index `to`. */
export const moveSection = (song: Song, from: number, to: number): Song => {
  const section = at(song.sections, from, "section");
  at(song.sections, to, "section");
  return { ...song, sections: song.sections.toSpliced(from, 1).toSpliced(to, 0, section) };
};

// A song always keeps at least one section.
export const deleteSection = (song: Song, index: number): Song => {
  at(song.sections, index, "section");
  const sections = song.sections.toSpliced(index, 1);
  return { ...song, sections: sections.length > 0 ? sections : [newSection(song, "Song")] };
};

/** Bars from `ref` onward go to a new section labelled `<label> (2)`, inserted right after; the repeat count stays on the first part. */
export const splitSection = (song: Song, ref: BarRef): Song => {
  barAt(song, ref);
  const section = song.sections[ref.section];
  const { repeat: _, ...rest } = section;
  const tail = { ...rest, id: newSectionId(), label: `${section.label} (2)`, bars: section.bars.slice(ref.bar) };
  const head = { ...section, bars: section.bars.slice(0, ref.bar) };
  return { ...song, sections: song.sections.toSpliced(ref.section, 1, head, tail) };
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
  return updateBars(song, ref, (existing) => existing.toSpliced(position === "before" ? ref.bar : ref.bar + 1, 0, ...pasted));
};

type Beat = { readonly slot: ChordSlot; readonly sec?: number; readonly section: number };
type TimedBar = { readonly bar: Bar; readonly s: number; readonly b: number; readonly beats: readonly Beat[]; readonly period?: number };

/**
 * The bars from `from` to the end of the song, each with its beats. Beat times are interpolated between bar starts; when
 * the next bar has no time (or for the last bar) the last timed period carries on. A bar without `startSec` has untimed beats.
 */
const barsFrom = (song: Song, from: BarRef): TimedBar[] => {
  barAt(song, from);
  const all = song.sections.flatMap((section, s) => section.bars.map((bar, b) => ({ bar, s, b })));
  let period: number | undefined;
  const timed = all.map(({ bar, s, b }, i): TimedBar => {
    const { startSec } = bar;
    const next = all[i + 1]?.bar.startSec;
    if (startSec !== undefined && next !== undefined) period = (next - startSec) / (bar.meter ?? song.meta.meter).beats;
    const beats = bar.chords.flatMap((slot) => Array.from({ length: slot.beats }, () => slot)).map((slot, j) => ({
      slot,
      section: s,
      sec: startSec === undefined || (j > 0 && period === undefined) ? undefined : startSec + j * (period ?? 0),
    }));
    return { bar, s, b, beats, period };
  });
  return timed.slice(timed.findIndex(({ s, b }) => s === from.section && b === from.bar));
};

/** Consecutive beats of the same chord make one slot; a merged slot keeps the lowest confidence (the most doubtful). */
const toSlots = (beats: readonly Beat[]): ChordSlot[] =>
  beats.reduce<ChordSlot[]>((slots, { slot }) => {
    const last = slots[slots.length - 1];
    if (last?.chord !== slot.chord) return [...slots, { ...slot, beats: 1 }];
    const levels = [last.confidence, slot.confidence].filter((c) => c !== undefined);
    const confidence = levels.length > 0 ? Math.min(...levels) : undefined;
    return [...slots.slice(0, -1), { ...last, beats: last.beats + 1, confidence }];
  }, []);

/** A bar made of `beats`, with a meter override when it isn't as long as the song meter. */
const toBar = (song: Song, beats: readonly Beat[]): Bar => {
  const bar: Bar = { startSec: beats[0].sec, chords: toSlots(beats) };
  const { beats: size, unit } = song.meta.meter;
  return beats.length === size ? bar : { ...bar, meter: { beats: beats.length, unit } };
};

/** Replaces the bars from `from` to the end of the song with `bars`, each going to its section. */
const replaceFrom = (song: Song, from: BarRef, bars: readonly { section: number; bar: Bar }[]): Song => ({
  ...song,
  sections: song.sections.map((section, s) =>
    s < from.section
      ? section
      : {
          ...section,
          bars: [...(s === from.section ? section.bars.slice(0, from.bar) : []), ...bars.filter((b) => b.section === s).map((b) => b.bar)],
        },
  ),
});

/**
 * Moves every bar line from bar `from` (default: the first bar) to the end of the song by `delta` beats (spec F-PB-3):
 * +1 = the downbeats come one beat later. Beats are slot beats (eighths in 6/8). The range is re-cut into bars of the song
 * meter: a short first bar (pickup) and a short last bar get a meter override. A short first bar sets the current phase,
 * so shifting twice from the bar after a missed break makes it a 2-beat bar. Beat times are interpolated between bar
 * starts (the last timed period carries on); a bar starting on an untimed beat has no `startSec`. Each section starts at
 * the bar line its first bar line moved to.
 */
export const shiftPhase = (song: Song, delta: 1 | -1, from: BarRef = { section: 0, bar: 0 }): Song => {
  const size = song.meta.meter.beats;
  const range = barsFrom(song, from);
  const lengths = range.map(({ bar }) => (bar.meter ?? song.meta.meter).beats);
  // ponytail: a bar with its own meter inside the range is refused rather than kept; handle it if break bars get detected.
  if (lengths.some((n, i) => n > size || (n < size && i > 0 && i < lengths.length - 1)))
    throw new Error("the phase can't be shifted across a bar with its own meter");

  const beats = range.flatMap((t) => t.beats);
  const phase = (((lengths[0] < size ? lengths[0] : 0) + delta) % size + size) % size;
  const starts = phase > 0 ? [0] : [];
  for (let q = phase; q < beats.length; q += size) starts.push(q);
  const bars = starts.map((q, i) => ({
    section: beats[Math.min(Math.max(q - delta, 0), beats.length - 1)].section,
    bar: toBar(song, beats.slice(q, starts[i + 1] ?? beats.length)),
  }));
  return replaceFrom(song, from, bars);
};

// The song tempo only changes when the whole song does.
const scaleTempo = (song: Song, from: BarRef, factor: number): Song => {
  const { tempoBpm } = song.meta;
  const isWholeSong = from.section === 0 && from.bar === 0;
  return isWholeSong && tempoBpm !== undefined ? { ...song, meta: { ...song.meta, tempoBpm: tempoBpm * factor } } : song;
};

/**
 * Fixes a double tempo (spec F-PB-4): from bar `from` (default: the first bar) on, bars are merged two by two within each
 * section and every two beats make one, taking the chord and time of the first. So a chord change on an odd beat moves to
 * the next beat, and a 1-beat chord there is dropped. A section's odd last bar becomes a half bar (meter override).
 */
export const halveTempo = (song: Song, from: BarRef = { section: 0, bar: 0 }): Song => {
  const range = barsFrom(song, from);
  const bars = range.flatMap(({ s, b, beats }, i) => {
    if ((b - (s === from.section ? from.bar : 0)) % 2 === 1) return [];
    const next = range[i + 1];
    const pair = next?.s === s ? [...beats, ...next.beats] : beats;
    return [{ section: s, bar: toBar(song, pair.filter((_, j) => j % 2 === 0)) }];
  });
  return scaleTempo(replaceFrom(song, from, bars), from, 0.5);
};

/** Fixes a half tempo (spec F-PB-4): from bar `from` on, each bar is split in two bars of its meter, every beat becoming two. */
export const doubleTempo = (song: Song, from: BarRef = { section: 0, bar: 0 }): Song => {
  const bars = barsFrom(song, from).flatMap(({ s, beats, period }) => {
    const halves = beats.flatMap((beat) => [beat, { ...beat, sec: beat.sec === undefined || period === undefined ? undefined : beat.sec + period / 2 }]);
    return [halves.slice(0, beats.length), halves.slice(beats.length)].map((part) => ({ section: s, bar: toBar(song, part) }));
  });
  return scaleTempo(replaceFrom(song, from, bars), from, 2);
};
