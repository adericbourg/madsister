// Key → command mapping for the grid (spec F-ED-3..7), pure so it's testable without the DOM.
import {
  addSection,
  copyBars,
  deleteBar,
  deleteSection,
  doubleTempo,
  duplicateBar,
  halveTempo,
  insertBar,
  mergeSlotWithNext,
  moveSection,
  pasteBars,
  resizeSlot,
  setBarMeter,
  setRepeat,
  shiftPhase,
  splitSection,
  splitSlot,
  type BarRef,
  type SlotRef,
} from "../model/commands";
import type { Bar, Song } from "../model/song";

export type KeyInput = { key: string; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean; altKey: boolean };
export type EditorState = {
  song: Song;
  cursor: SlotRef;
  anchor: BarRef | null;
  clipboard: readonly Bar[];
  barsPerRow: number;
  lowConfidenceThreshold: number;
};
export type Command =
  | { kind: "move"; cursor: SlotRef; anchor: BarRef | null }
  | { kind: "edit"; song: Song; cursor: SlotRef }
  | { kind: "copy"; bars: Bar[] }
  | { kind: "type"; text: string }
  | { kind: "rename" | "undo" | "redo" | "help" | "play" | "playFromCursor" | "refused" };

export const MOD_LABEL = /Mac|iP/.test(navigator.platform) ? "⌘" : "Ctrl";

export const SHORTCUTS: readonly [keys: string, what: string][] = [
  ["← → ↑ ↓", "Move between slots and rows"],
  ["Home / End", "First / last slot of the section"],
  ["Type a chord, Enter or Tab", "Set the chord and go to the next slot (Escape cancels)"],
  ["F8 / Shift+F8", "Next / previous low-confidence chord"],
  ["/", "Split the slot"],
  ["Mod+/", "While typing a chord: set it and split the slot"],
  ["Backspace", "Merge the slot with the previous one"],
  ["Alt+← / Alt+→", "Shrink / grow the slot by one beat"],
  ["Mod+Enter / Mod+Shift+Enter", "Insert a bar after / before"],
  ["Mod+Backspace", "Delete the bar"],
  ["Mod+D", "Duplicate the bar"],
  ["Mod+B", "Make the bar a short break bar (2/4, or 3/8 in 6/8) / back to the song meter"],
  ["< / >", "Move the bar lines one beat earlier / later, from the cursor bar on (twice after a missed 2-beat break)"],
  ["- / +", "Half tempo (merge bars two by two) / double tempo (split each bar), from the cursor bar on"],
  ["Shift+← / Shift+→", "Select bars"],
  ["Mod+C / Mod+V", "Copy the selected bars / paste after the cursor bar"],
  ["Mod+Z / Mod+Shift+Z", "Undo / redo"],
  ["Mod+K", "Split the section at the cursor bar"],
  ["Mod+Shift+K", "Add a section after this one"],
  ["Mod+Shift+Backspace", "Delete the section"],
  ["F2", "Rename the section"],
  ["Alt+↑ / Alt+↓", "Repeat the section one more / one less time"],
  ["Alt+Shift+↑ / Alt+Shift+↓", "Move the section up / down"],
  ["Space", "Play / pause the source audio"],
  ["Shift+Space", "Play from the cursor bar"],
  ["Mod+N / Mod+O", "New song / open a file"],
  ["Mod+S / Mod+Shift+S", "Save / save as"],
  ["Mod+P", "Print"],
  ["?", "Show this help"],
];

export const clampCursor = (song: Song, { section: s, bar: b, slot }: SlotRef): SlotRef => {
  const section = Math.min(s, song.sections.length - 1);
  const bars = song.sections[section].bars;
  if (bars.length === 0) return { section, bar: 0, slot: 0 };
  const bar = Math.min(b, bars.length - 1);
  return { section, bar, slot: Math.min(slot, bars[bar].chords.length - 1) };
};

// Every cursor position in reading order; an empty section has one, on its "No bars yet" placeholder.
const positions = (song: Song): SlotRef[] =>
  song.sections.flatMap((section, s) =>
    section.bars.length === 0
      ? [{ section: s, bar: 0, slot: 0 }]
      : section.bars.flatMap((bar, b) => bar.chords.map((_, slot) => ({ section: s, bar: b, slot }))),
  );

const isFlagged = (song: Song, { section, bar, slot }: SlotRef, threshold: number) => {
  const confidence = song.sections[section].bars[bar]?.chords[slot].confidence;
  return confidence !== undefined && confidence < threshold;
};

/** Whether any chord comes with an engine confidence, i.e. the song went through chord recognition. */
export const hasConfidence = (song: Song): boolean => song.sections.some((s) => s.bars.some((b) => b.chords.some((c) => c.confidence !== undefined)));

/** Number of slots whose engine confidence is below the threshold (spec F-ED-8). */
export const countFlagged = (song: Song, threshold: number): number => positions(song).filter((p) => isFlagged(song, p, threshold)).length;

/** The next (or previous) flagged slot after the cursor, wrapping around the song. */
const nextFlagged = (song: Song, cursor: SlotRef, threshold: number, step: 1 | -1): SlotRef | undefined => {
  const all = step === 1 ? positions(song) : positions(song).reverse();
  const i = all.findIndex((p) => p.section === cursor.section && p.bar === cursor.bar && p.slot === cursor.slot);
  return [...all.slice(i + 1), ...all.slice(0, i + 1)].find((p) => isFlagged(song, p, threshold));
};

export const nextSlot = (song: Song, cursor: SlotRef, step: 1 | -1): SlotRef => {
  const all = positions(song);
  const i = all.findIndex((p) => p.section === cursor.section && p.bar === cursor.bar && p.slot === cursor.slot);
  return all[i + step] ?? cursor;
};

const nextRow = (song: Song, cursor: SlotRef, barsPerRow: number, step: 1 | -1): SlotRef => {
  const rows = song.sections.flatMap((section, s) =>
    Array.from({ length: Math.max(1, Math.ceil(section.bars.length / barsPerRow)) }, (_, r) => ({ section: s, start: r * barsPerRow })),
  );
  const i = rows.findIndex((r) => r.section === cursor.section && cursor.bar >= r.start && cursor.bar < r.start + barsPerRow);
  const target = rows[i + step];
  return target ? clampCursor(song, { ...cursor, section: target.section, bar: target.start + cursor.bar - rows[i].start }) : cursor;
};

/** The selected bar range: anchor..cursor, or just the cursor bar. */
export const selectedBars = ({ cursor, anchor }: Pick<EditorState, "cursor" | "anchor">) => {
  const from = anchor?.bar ?? cursor.bar;
  return { section: cursor.section, from: Math.min(from, cursor.bar), to: Math.max(from, cursor.bar) };
};

const commandFor = (e: KeyInput, state: EditorState): Command | null => {
  const { song, cursor, anchor, clipboard } = state;
  const bar: BarRef = { section: cursor.section, bar: cursor.bar };
  const section = cursor.section;
  const move = (to: SlotRef): Command => ({ kind: "move", cursor: to, anchor: null });
  const edit = (fn: (s: Song) => Song, to: SlotRef = cursor): Command => {
    const next = fn(song);
    return { kind: "edit", song: next, cursor: clampCursor(next, to) };
  };
  const isUp = e.key === "ArrowUp";

  // Either modifier is accepted as Mod; the help shows the platform's one.
  if (e.metaKey || e.ctrlKey) {
    switch (e.key.toLowerCase()) {
      case "z":
        return { kind: e.shiftKey ? "redo" : "undo" };
      case "enter":
        return e.shiftKey
          ? edit((s) => insertBar(s, bar, "before"), { ...bar, slot: 0 })
          : edit((s) => insertBar(s, bar, "after"), { ...bar, bar: bar.bar + 1, slot: 0 });
      case "backspace":
        return e.shiftKey ? edit((s) => deleteSection(s, section)) : edit((s) => deleteBar(s, bar));
      case "d":
        return edit((s) => duplicateBar(s, bar));
      case "b": {
        const unit = song.meta.meter.unit;
        const isOverridden = song.sections[section].bars[bar.bar]?.meter !== undefined;
        return edit((s) => setBarMeter(s, bar, isOverridden ? null : { beats: unit === 8 ? 3 : 2, unit }));
      }
      case "k":
        return edit(
          (s) => (e.shiftKey ? addSection(s, section + 1, "New section") : splitSection(s, bar)),
          { section: section + 1, bar: 0, slot: 0 },
        );
      case "c": {
        const range = selectedBars(state);
        return { kind: "copy", bars: copyBars(song, { section, bar: range.from }, { section, bar: range.to }) };
      }
      case "v":
        return clipboard.length > 0 ? edit((s) => pasteBars(s, bar, clipboard, "after")) : null;
    }
    return null;
  }
  if (e.altKey) {
    switch (e.key) {
      case "ArrowLeft":
      case "ArrowRight":
        return edit((s) => resizeSlot(s, cursor, e.key === "ArrowRight" ? 1 : -1));
      case "ArrowUp":
      case "ArrowDown": {
        if (!e.shiftKey) return edit((s) => setRepeat(s, section, (s.sections[section].repeat ?? 1) + (isUp ? 1 : -1)));
        const to = section + (isUp ? -1 : 1);
        return edit((s) => moveSection(s, section, to), { ...cursor, section: to });
      }
    }
    return null;
  }
  if (e.shiftKey && (e.key === "ArrowLeft" || e.key === "ArrowRight")) {
    const b = cursor.bar + (e.key === "ArrowRight" ? 1 : -1);
    if (b < 0 || b >= song.sections[section].bars.length) return null;
    return { kind: "move", cursor: { section, bar: b, slot: 0 }, anchor: anchor ?? bar };
  }
  switch (e.key) {
    case "ArrowLeft":
    case "ArrowRight":
      return move(nextSlot(song, cursor, e.key === "ArrowRight" ? 1 : -1));
    case "ArrowUp":
    case "ArrowDown":
      return move(nextRow(song, cursor, state.barsPerRow, isUp ? -1 : 1));
    case "Home":
    case "End": {
      const inSection = positions(song).filter((p) => p.section === section);
      return move(inSection[e.key === "Home" ? 0 : inSection.length - 1]);
    }
    case "/":
      return edit((s) => splitSlot(s, cursor));
    case "<":
    case ">":
      return edit((s) => shiftPhase(s, e.key === ">" ? 1 : -1, bar), { ...bar, slot: 0 });
    case "-":
    case "+":
      return edit((s) => (e.key === "+" ? doubleTempo(s, bar) : halveTempo(s, bar)), { ...bar, slot: 0 });
    case "Backspace":
      return edit((s) => mergeSlotWithNext(s, { ...cursor, slot: cursor.slot - 1 }), { ...cursor, slot: cursor.slot - 1 });
    case "?":
      return { kind: "help" };
    case " ":
      return { kind: e.shiftKey ? "playFromCursor" : "play" };
    case "F2":
      return { kind: "rename" };
    case "F8": {
      const target = nextFlagged(song, cursor, state.lowConfidenceThreshold, e.shiftKey ? -1 : 1);
      if (target === undefined) throw new Error("no low-confidence chord"); // refused
      return move(target);
    }
  }
  const isPrintable = e.key.length === 1 && e.key.trim() !== "";
  return isPrintable && song.sections[section].bars.length > 0 ? { kind: "type", text: e.key } : null;
};

/** Commands throw on impossible edits (e.g. merging the first slot): the key is then refused. */
export const keyToCommand = (e: KeyInput, state: EditorState): Command | null => {
  try {
    return commandFor(e, state);
  } catch {
    return { kind: "refused" };
  }
};
