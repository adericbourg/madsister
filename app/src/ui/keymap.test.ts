import { expect, test } from "vitest";
import {
  deleteBar,
  duplicateBar,
  insertBar,
  mergeSlotWithNext,
  moveSection,
  pasteBars,
  resizeSlot,
  setRepeat,
  splitSlot,
  type SlotRef,
} from "../model/commands";
import type { Song } from "../model/song";
import { keyToCommand, type EditorState } from "./keymap";

const song: Song = {
  version: 1,
  meta: { title: "t", meter: { beats: 4, unit: 4 } },
  sections: [
    {
      id: "a",
      label: "Verse",
      bars: [
        { chords: [{ chord: "C:maj", beats: 4 }] },
        { chords: [{ chord: "F:maj", beats: 2 }, { chord: "G:maj", beats: 2 }] },
        { chords: [{ chord: "A:min", beats: 4 }] },
      ],
    },
    { id: "b", label: "Chorus", repeat: 2, bars: [{ chords: [{ chord: "N", beats: 4 }] }] },
    { id: "c", label: "Bridge", bars: [] },
  ],
};

const state = (cursor: SlotRef, more: Partial<EditorState> = {}): EditorState => ({
  song,
  cursor,
  anchor: null,
  clipboard: [],
  barsPerRow: 2,
  ...more,
});

type Mods = { mod?: boolean; shift?: boolean; alt?: boolean; meta?: boolean };
const key = (k: string, { mod = false, shift = false, alt = false, meta = false }: Mods = {}) => ({
  key: k,
  ctrlKey: mod,
  metaKey: meta,
  shiftKey: shift,
  altKey: alt,
});
const at = (section: number, bar: number, slot = 0): SlotRef => ({ section, bar, slot });
const moveTo = (s: EditorState, k: ReturnType<typeof key>) => {
  const command = keyToCommand(k, s);
  return command?.kind === "move" ? command.cursor : command;
};

test("keyToCommand_ofNavigationKeys_movesTheCursor", () => {
  // Given a 3-section song laid out 2 bars per row (Verse: 2 rows, Chorus: 1 row, Bridge: empty)

  // Then ←/→ walk slots across bars and sections, including the empty section, and stop at both ends
  expect(moveTo(state(at(0, 0)), key("ArrowRight"))).toEqual(at(0, 1, 0));
  expect(moveTo(state(at(0, 1, 1)), key("ArrowRight"))).toEqual(at(0, 2));
  expect(moveTo(state(at(0, 2)), key("ArrowRight"))).toEqual(at(1, 0));
  expect(moveTo(state(at(1, 0)), key("ArrowRight"))).toEqual(at(2, 0));
  expect(moveTo(state(at(2, 0)), key("ArrowRight"))).toEqual(at(2, 0));
  expect(moveTo(state(at(0, 1, 0)), key("ArrowLeft"))).toEqual(at(0, 0));
  expect(moveTo(state(at(0, 0)), key("ArrowLeft"))).toEqual(at(0, 0));

  // And ↑/↓ go to the same column of the previous/next row, clamped to the row and the bar's slots
  expect(moveTo(state(at(0, 1, 1)), key("ArrowDown"))).toEqual(at(0, 2));
  expect(moveTo(state(at(0, 2)), key("ArrowDown"))).toEqual(at(1, 0));
  expect(moveTo(state(at(1, 0)), key("ArrowDown"))).toEqual(at(2, 0));
  expect(moveTo(state(at(0, 2)), key("ArrowUp"))).toEqual(at(0, 0));
  expect(moveTo(state(at(0, 0)), key("ArrowUp"))).toEqual(at(0, 0));

  // And Home/End go to the first/last slot of the section
  expect(moveTo(state(at(0, 1, 1)), key("Home"))).toEqual(at(0, 0));
  expect(moveTo(state(at(0, 0)), key("End"))).toEqual(at(0, 2));

  // And a plain move clears the bar selection
  expect(keyToCommand(key("ArrowRight"), state(at(0, 0), { anchor: { section: 0, bar: 0 } }))).toMatchObject({ anchor: null });
});

test("keyToCommand_ofShiftArrows_extendsTheBarSelectionWithinTheSection", () => {
  // Given no selection, When Shift+→, Then the cursor bar becomes the anchor and the cursor moves to the next bar
  expect(keyToCommand(key("ArrowRight", { shift: true }), state(at(0, 0)))).toEqual({
    kind: "move",
    cursor: at(0, 1),
    anchor: { section: 0, bar: 0 },
  });

  // Given an anchor, When Shift+←, Then the anchor stays
  expect(keyToCommand(key("ArrowLeft", { shift: true }), state(at(0, 2), { anchor: { section: 0, bar: 2 } }))).toEqual({
    kind: "move",
    cursor: at(0, 1),
    anchor: { section: 0, bar: 2 },
  });

  // And the selection doesn't leave the section
  expect(keyToCommand(key("ArrowRight", { shift: true }), state(at(0, 2)))).toBeNull();
});

test("keyToCommand_ofEditShortcuts_appliesTheCommandAndPlacesTheCursor", () => {
  // Given the cursor on the G slot (bar 2, beat 3)
  const g = at(0, 1, 1);
  const bar = { section: 0, bar: 1 };
  const edit = (k: ReturnType<typeof key>, s = state(g)) => keyToCommand(k, s);
  const labels = (k: ReturnType<typeof key>) => {
    const command = edit(k);
    return command?.kind === "edit" ? { labels: command.song.sections.map((s) => s.label), cursor: command.cursor } : command;
  };

  // Then slot shortcuts
  expect(edit(key("/"))).toEqual({ kind: "edit", song: splitSlot(song, g), cursor: g });
  expect(edit(key("Backspace"))).toEqual({ kind: "edit", song: mergeSlotWithNext(song, at(0, 1, 0)), cursor: at(0, 1, 0) });
  expect(edit(key("ArrowRight", { alt: true }))).toEqual({ kind: "edit", song: resizeSlot(song, g, 1), cursor: g });
  expect(edit(key("ArrowLeft", { alt: true }))).toEqual({ kind: "edit", song: resizeSlot(song, g, -1), cursor: g });

  // And bar shortcuts (Mod is Ctrl or Cmd)
  expect(edit(key("Enter", { mod: true }))).toEqual({ kind: "edit", song: insertBar(song, bar, "after"), cursor: at(0, 2) });
  expect(edit(key("Enter", { meta: true, shift: true }))).toEqual({ kind: "edit", song: insertBar(song, bar, "before"), cursor: at(0, 1) });
  expect(edit(key("Backspace", { mod: true }))).toEqual({ kind: "edit", song: deleteBar(song, bar), cursor: at(0, 1, 0) });
  expect(edit(key("d", { mod: true }))).toEqual({ kind: "edit", song: duplicateBar(song, bar), cursor: g });
  const clipboard = [song.sections[0].bars[0]];
  expect(edit(key("v", { mod: true }), state(g, { clipboard }))).toEqual({
    kind: "edit",
    song: pasteBars(song, bar, clipboard, "after"),
    cursor: g,
  });
  expect(edit(key("v", { mod: true }))).toBeNull();

  // And section shortcuts
  expect(labels(key("k", { mod: true }))).toEqual({ labels: ["Verse", "Verse (2)", "Chorus", "Bridge"], cursor: at(1, 0) });
  expect(labels(key("K", { mod: true, shift: true }))).toEqual({ labels: ["Verse", "New section", "Chorus", "Bridge"], cursor: at(1, 0) });
  expect(labels(key("Backspace", { mod: true, shift: true }))).toEqual({ labels: ["Chorus", "Bridge"], cursor: at(0, 0) });
  expect(edit(key("ArrowUp", { alt: true }))).toEqual({ kind: "edit", song: setRepeat(song, 0, 2), cursor: g });
  expect(edit(key("ArrowDown", { alt: true }), state(at(1, 0)))).toEqual({ kind: "edit", song: setRepeat(song, 1, 1), cursor: at(1, 0) });
  expect(edit(key("ArrowDown", { alt: true, shift: true }))).toEqual({ kind: "edit", song: moveSection(song, 0, 1), cursor: at(1, 1, 1) });

  // And impossible edits are refused instead of throwing
  expect(edit(key("Backspace"), state(at(0, 1, 0)))).toEqual({ kind: "refused" });
  expect(edit(key("ArrowUp", { alt: true, shift: true }), state(at(0, 0)))).toEqual({ kind: "refused" });
  expect(edit(key("k", { mod: true }), state(at(2, 0)))).toEqual({ kind: "refused" });
});

test("keyToCommand_ofOtherKeys_returnsTheMatchingAction", () => {
  // Then history, help, rename and typing
  expect(keyToCommand(key("z", { mod: true }), state(at(0, 0)))).toEqual({ kind: "undo" });
  expect(keyToCommand(key("Z", { mod: true, shift: true }), state(at(0, 0)))).toEqual({ kind: "redo" });
  expect(keyToCommand(key("?", { shift: true }), state(at(0, 0)))).toEqual({ kind: "help" });
  expect(keyToCommand(key("F2"), state(at(0, 0)))).toEqual({ kind: "rename" });
  expect(keyToCommand(key("A", { shift: true }), state(at(0, 0)))).toEqual({ kind: "type", text: "A" });

  // And copy takes the selected bars, or the cursor bar without a selection
  expect(keyToCommand(key("c", { mod: true }), state(at(0, 1, 1), { anchor: { section: 0, bar: 0 } }))).toEqual({
    kind: "copy",
    bars: song.sections[0].bars.slice(0, 2),
  });
  expect(keyToCommand(key("c", { mod: true }), state(at(0, 2)))).toEqual({ kind: "copy", bars: [song.sections[0].bars[2]] });

  // And unmapped keys, reserved shortcuts and typing into an empty section do nothing
  expect(keyToCommand(key(" "), state(at(0, 0)))).toBeNull();
  expect(keyToCommand(key("s", { mod: true }), state(at(0, 0)))).toBeNull();
  expect(keyToCommand(key("C"), state(at(2, 0)))).toBeNull();
});
