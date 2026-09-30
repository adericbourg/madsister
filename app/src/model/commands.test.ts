import { describe, expect, test } from "vitest";
import {
  deleteBar,
  duplicateBar,
  insertBar,
  mergeSlotWithNext,
  resizeSlot,
  setChord,
  splitSlot,
} from "./commands";
import { parseSong, type Song } from "./song";

const deepFreeze = <T>(value: T): T => {
  if (typeof value === "object" && value !== null) Object.values(value).forEach(deepFreeze);
  return Object.freeze(value);
};

// One 4/4 section: bar 0 = A:min 3 + E:7 1 (aligned to audio), bar 1 = C:maj 4.
const fixture = (): Song =>
  deepFreeze(
    parseSong({
      version: 1,
      meta: { title: "Song", meter: { beats: 4, unit: 4 } },
      sections: [
        {
          id: "s1",
          label: "Verse",
          bars: [
            { startSec: 0, chords: [{ chord: "A:min", beats: 3, confidence: 0.4 }, { chord: "E:7", beats: 1 }] },
            { startSec: 2, chords: [{ chord: "C:maj", beats: 4, confidence: 0.9 }] },
          ],
        },
      ],
    }),
  );

const snapshot = fixture();

// Every command must keep the file valid (beat sums) and leave the frozen input untouched.
const check = (song: Song, result: Song) => {
  expect(song).toEqual(snapshot);
  expect(() => parseSong(JSON.parse(JSON.stringify(result)))).not.toThrow();
  return result.sections[0].bars;
};

describe("setChord", () => {
  test("setChord_replacesTheChordAndClearsConfidence", () => {
    // Given
    const song = fixture();

    // When
    const bars = check(song, setChord(song, { section: 0, bar: 0, slot: 0 }, "D:min7"));

    // Then
    expect(bars[0].chords[0]).toStrictEqual({ chord: "D:min7", beats: 3 });
    expect(bars[0].startSec).toBe(0);
    expect(bars[1]).toBe(song.sections[0].bars[1]);
  });
});

describe("splitSlot", () => {
  test("splitSlot_ofThreeBeats_returnsTwoPlusOneCopyingTheChord", () => {
    // Given
    const song = fixture();

    // When
    const bars = check(song, splitSlot(song, { section: 0, bar: 1, slot: 0 }));
    const barsOfOdd = check(song, splitSlot(song, { section: 0, bar: 0, slot: 0 }));

    // Then
    expect(bars[1].chords.map((s) => [s.chord, s.beats])).toEqual([["C:maj", 2], ["C:maj", 2]]);
    expect(barsOfOdd[0].chords.map((s) => [s.chord, s.beats])).toEqual([["A:min", 2], ["A:min", 1], ["E:7", 1]]);
  });

  test("splitSlot_ofOneBeat_throws", () => {
    // Given / When / Then
    expect(() => splitSlot(fixture(), { section: 0, bar: 0, slot: 1 })).toThrow();
  });
});

describe("mergeSlotWithNext", () => {
  test("mergeSlotWithNext_addsBeatsAndKeepsTheFirstChord", () => {
    // Given
    const song = fixture();

    // When
    const bars = check(song, mergeSlotWithNext(song, { section: 0, bar: 0, slot: 0 }));

    // Then
    expect(bars[0].chords.map((s) => [s.chord, s.beats])).toEqual([["A:min", 4]]);
    expect(() => mergeSlotWithNext(song, { section: 0, bar: 0, slot: 1 })).toThrow();
  });
});

describe("resizeSlot", () => {
  test("resizeSlot_takesOrGivesABeatFromTheNextOrPreviousSlot", () => {
    // Given
    const song = fixture();
    const first = { section: 0, bar: 0, slot: 0 };
    const last = { section: 0, bar: 0, slot: 1 };

    // When
    const shrunk = check(song, resizeSlot(song, first, -1));
    const grownLast = check(song, resizeSlot(song, last, 1));
    const grownFirst = check(song, resizeSlot(song, first, 1));
    const lastRemoved = check(song, resizeSlot(song, last, -1));

    // Then
    expect(shrunk[0].chords.map((s) => s.beats)).toEqual([2, 2]);
    expect(grownLast[0].chords.map((s) => s.beats)).toEqual([2, 2]);
    expect(grownFirst[0].chords.map((s) => [s.chord, s.beats])).toEqual([["A:min", 4]]);
    expect(lastRemoved[0].chords.map((s) => [s.chord, s.beats])).toEqual([["A:min", 4]]);
  });

  test("resizeSlot_ofTheOnlySlot_throws", () => {
    // Given / When / Then
    expect(() => resizeSlot(fixture(), { section: 0, bar: 1, slot: 0 }, 1)).toThrow();
  });
});

describe("insertBar", () => {
  test("insertBar_insertsAnEmptyBarAndKeepsNeighboursStartSec", () => {
    // Given
    const song = fixture();

    // When
    const before = check(song, insertBar(song, { section: 0, bar: 0 }, "before"));
    const after = check(song, insertBar(song, { section: 0, bar: 0 }, "after"));

    // Then
    expect(before.map((b) => b.startSec)).toEqual([undefined, 0, 2]);
    expect(before[0]).toStrictEqual({ chords: [{ chord: "N", beats: 4 }] });
    expect(after.map((b) => b.startSec)).toEqual([0, undefined, 2]);
  });
});

describe("deleteBar", () => {
  test("deleteBar_removesTheBarAndAllowsAnEmptySection", () => {
    // Given
    const song = fixture();

    // When
    const bars = check(song, deleteBar(song, { section: 0, bar: 0 }));
    const emptied = check(song, deleteBar(deleteBar(song, { section: 0, bar: 0 }), { section: 0, bar: 0 }));

    // Then
    expect(bars).toEqual([song.sections[0].bars[1]]);
    expect(emptied).toEqual([]);
  });
});

describe("duplicateBar", () => {
  test("duplicateBar_insertsACopyAfterWithoutStartSec", () => {
    // Given
    const song = fixture();

    // When
    const bars = check(song, duplicateBar(song, { section: 0, bar: 0 }));

    // Then
    expect(bars.map((b) => b.startSec)).toEqual([0, undefined, 2]);
    expect(bars[1]).toEqual({ chords: song.sections[0].bars[0].chords });
    expect(bars[1]).not.toHaveProperty("startSec");
  });
});
