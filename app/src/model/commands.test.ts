import { describe, expect, test } from "vitest";
import {
  addSection,
  copyBars,
  deleteBar,
  deleteSection,
  duplicateBar,
  insertBar,
  mergeSlotWithNext,
  moveSection,
  pasteBars,
  renameSection,
  resizeSlot,
  setBarMeter,
  setChord,
  setRepeat,
  shiftPhase,
  splitSection,
  splitSlot,
} from "./commands";
import { emptySong, parseSong, type Song } from "./song";

const deepFreeze = <T>(value: T): T => {
  if (typeof value === "object" && value !== null) Object.values(value).forEach(deepFreeze);
  return Object.freeze(value);
};

// 4/4. Verse: bar 0 = A:min 3 + E:7 1 (aligned to audio), bar 1 = C:maj 4. Chorus x2: G:maj 4, F:maj 4.
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
        {
          id: "s2",
          label: "Chorus",
          repeat: 2,
          bars: [
            { startSec: 4, chords: [{ chord: "G:maj", beats: 4 }] },
            { startSec: 6, chords: [{ chord: "F:maj", beats: 4 }] },
          ],
        },
      ],
    }),
  );

const snapshot = fixture();

// Every command must keep the file valid (beat sums) and leave the frozen input untouched.
const checkSong = (song: Song, result: Song) => {
  expect(song).toEqual(snapshot);
  expect(() => parseSong(JSON.parse(JSON.stringify(result)))).not.toThrow();
  return result;
};
const check = (song: Song, result: Song) => checkSong(song, result).sections[0].bars;
const labels = (song: Song) => song.sections.map((s) => s.label);
const chords = (song: Song, section: number) => song.sections[section].bars.map((b) => b.chords.map((s) => s.chord).join(" "));

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

describe("section commands", () => {
  test("addSection_insertsASectionWithOneEmptyBar", () => {
    // Given
    const song = fixture();

    // When
    const result = checkSong(song, addSection(song, 1, "Bridge"));

    // Then
    expect(labels(result)).toEqual(["Verse", "Bridge", "Chorus"]);
    expect(result.sections[1].bars).toEqual([{ chords: [{ chord: "N", beats: 4 }] }]);
    expect(result.sections[1].id).toMatch(/^[0-9a-f]{8}$/);
  });

  test("renameSection_moveSection_setRepeat_updateTheSections", () => {
    // Given
    const song = fixture();

    // When
    const renamed = checkSong(song, renameSection(song, 1, "Refrain"));
    const moved = checkSong(song, moveSection(song, 0, 1));
    const repeated = checkSong(song, setRepeat(song, 0, 3));
    const unrepeated = checkSong(song, setRepeat(song, 1, 1));

    // Then
    expect(labels(renamed)).toEqual(["Verse", "Refrain"]);
    expect(labels(moved)).toEqual(["Chorus", "Verse"]);
    expect(moved.sections[1]).toBe(song.sections[0]);
    expect(repeated.sections[0].repeat).toBe(3);
    expect(unrepeated.sections[1]).not.toHaveProperty("repeat");
  });

  test("deleteSection_removesItAndLeavesAnEmptySongSectionWhenItWasTheLast", () => {
    // Given
    const song = fixture();

    // When
    const deleted = checkSong(song, deleteSection(song, 0));
    const emptied = checkSong(song, deleteSection(deleted, 0));

    // Then
    expect(labels(deleted)).toEqual(["Chorus"]);
    expect(labels(emptied)).toEqual(["Song"]);
    expect(emptied.sections[0].bars).toEqual([{ chords: [{ chord: "N", beats: 4 }] }]);
  });

  test("splitSection_movesBarsFromTheRefOnwardToANewSectionRightAfter", () => {
    // Given
    const song = fixture();

    // When
    const atLast = checkSong(song, splitSection(song, { section: 0, bar: 1 }));
    const atFirst = checkSong(song, splitSection(song, { section: 0, bar: 0 }));

    // Then
    expect(labels(atLast)).toEqual(["Verse", "Verse (2)", "Chorus"]);
    expect(chords(atLast, 0)).toEqual(["A:min E:7"]);
    expect(chords(atLast, 1)).toEqual(["C:maj"]);
    expect(atLast.sections[1].id).not.toBe(song.sections[0].id);
    expect(chords(atFirst, 0)).toEqual([]);
    expect(chords(atFirst, 1)).toEqual(["A:min E:7", "C:maj"]);
  });
});

describe("copyBars / pasteBars", () => {
  test("pasteBars_ofTwoCopiedBars_insertsThemInOrderWithoutStartSecOrConfidence", () => {
    // Given
    const song = fixture();
    const bars = copyBars(song, { section: 0, bar: 0 }, { section: 0, bar: 1 });

    // When
    const result = checkSong(song, pasteBars(song, { section: 1, bar: 1 }, bars, "after"));
    const before = checkSong(song, pasteBars(song, { section: 1, bar: 0 }, bars, "before"));

    // Then
    expect(chords(result, 1)).toEqual(["G:maj", "F:maj", "A:min E:7", "C:maj"]);
    expect(result.sections[1].bars.map((b) => b.startSec)).toEqual([4, 6, undefined, undefined]);
    expect(result.sections[1].bars[2]).not.toHaveProperty("startSec");
    // A pasted chord is the user's decision: it is no longer flagged as low-confidence
    expect(result.sections[1].bars[2].chords[0]).not.toHaveProperty("confidence");
    expect(chords(before, 1)).toEqual(["A:min E:7", "C:maj", "G:maj", "F:maj"]);
  });

  test("pasteBars_whenBeatsDifferFromTheSongMeter_addsAMeterOverride", () => {
    // Given a 3/4 target song and 4-beat bars
    const song = fixture();
    const target = emptySong({ beats: 3, unit: 4 });
    const bars = copyBars(song, { section: 0, bar: 1 }, { section: 0, bar: 1 });

    // When
    const result = pasteBars(target, { section: 0, bar: 0 }, bars, "before");

    // Then
    expect(() => parseSong(JSON.parse(JSON.stringify(result)))).not.toThrow();
    expect(result.sections[0].bars[0].meter).toEqual({ beats: 4, unit: 4 });
    expect(result.sections[0].bars[1]).not.toHaveProperty("meter");
  });
});

describe("setBarMeter", () => {
  test("setBarMeter_resizesTheSlotsAndRemovesTheOverrideAtTheSongMeter", () => {
    // Given bar 0 = A:min 3 + E:7 1 in a 4/4 song
    const song = fixture();
    const ref = { section: 0, bar: 0 };

    // When shrinking it to 2/4, growing it to 5/4, then setting it back to 4/4 or to null
    const shrunk = check(song, setBarMeter(song, ref, { beats: 2, unit: 4 }));
    const grown = check(song, setBarMeter(song, ref, { beats: 5, unit: 4 }));
    const back = checkSong(song, setBarMeter(setBarMeter(song, ref, { beats: 2, unit: 4 }), ref, { beats: 4, unit: 4 }));
    const reset = check(song, setBarMeter(song, ref, null));

    // Then slots are truncated from the end, or the last slot grows; startSec and the other bars are kept
    expect(shrunk[0]).toEqual({ startSec: 0, meter: { beats: 2, unit: 4 }, chords: [{ chord: "A:min", beats: 2, confidence: 0.4 }] });
    expect(grown[0].chords.map((s) => [s.chord, s.beats])).toEqual([["A:min", 3], ["E:7", 2]]);
    expect(grown[0].meter).toEqual({ beats: 5, unit: 4 });
    expect(shrunk[1]).toBe(song.sections[0].bars[1]);
    // And the song meter removes the override
    expect(back.sections[0].bars[0]).not.toHaveProperty("meter");
    expect(back.sections[0].bars[0].chords).toEqual([{ chord: "A:min", beats: 4, confidence: 0.4 }]);
    expect(reset[0]).toEqual(song.sections[0].bars[0]);
  });

  test("setBarMeter_ofAHalfBarInA68Song_countsInEighths", () => {
    // Given a 6/8 song whose bar is 4 + 2 eighths
    const song = deepFreeze(
      parseSong({
        version: 1,
        meta: { title: "Jig", meter: { beats: 6, unit: 8 } },
        sections: [{ id: "s", label: "A", bars: [{ chords: [{ chord: "D:maj", beats: 4 }, { chord: "A:maj", beats: 2 }] }] }],
      }),
    );

    // When setting it to 3/8
    const result = setBarMeter(song, { section: 0, bar: 0 }, { beats: 3, unit: 8 });

    // Then it keeps 3 eighths of the first chord
    expect(() => parseSong(JSON.parse(JSON.stringify(result)))).not.toThrow();
    expect(result.sections[0].bars[0]).toEqual({ meter: { beats: 3, unit: 8 }, chords: [{ chord: "D:maj", beats: 3 }] });
  });
});

describe("shiftPhase", () => {
  const frozen = (sections: unknown[]): Song => deepFreeze(parseSong({ version: 1, meta: { title: "t", meter: { beats: 4, unit: 4 } }, sections }));
  const valid = (song: Song) => {
    expect(() => parseSong(JSON.parse(JSON.stringify(song)))).not.toThrow();
    return song;
  };
  const shape = (song: Song) =>
    song.sections.map((s) => s.bars.map((b) => `${b.startSec ?? "-"}|${b.chords.map((c) => `${c.chord}:${c.beats}`).join(" ")}`));

  test("shiftPhase_whenChordsChangeOnBeat2_movesTheBarLinesToTheChanges", () => {
    // Given a song whose chords all change on beat 2 (the bar lines are one beat early)
    const song = frozen([
      {
        id: "v",
        label: "Verse",
        bars: [
          { startSec: 0, chords: [{ chord: "A:min", beats: 1 }, { chord: "C:maj", beats: 3 }] },
          { startSec: 2, chords: [{ chord: "C:maj", beats: 1 }, { chord: "G:maj", beats: 3 }] },
          { startSec: 4, chords: [{ chord: "G:maj", beats: 1 }, { chord: "F:maj", beats: 3 }] },
          { startSec: 6, chords: [{ chord: "F:maj", beats: 4 }] },
        ],
      },
    ]);

    // When shifting the phase by +1 beat
    const fixed = valid(shiftPhase(song, 1));

    // Then a 1-beat pickup comes first, each chord fills a bar, and the last bar is cut short
    expect(shape(fixed)).toEqual([["0|A:min:1", "0.5|C:maj:4", "2.5|G:maj:4", "4.5|F:maj:4", "6.5|F:maj:3"]]);
    expect(fixed.sections[0].bars[0].meter).toEqual({ beats: 1, unit: 4 });
    expect(fixed.sections[0].bars[1]).not.toHaveProperty("meter");
    expect(fixed.sections[0].bars[4].meter).toEqual({ beats: 3, unit: 4 });
    // And -1 undoes +1, +1 undoes -1
    expect(valid(shiftPhase(fixed, -1))).toEqual(song);
    expect(shiftPhase(valid(shiftPhase(song, -1)), 1)).toEqual(song);
  });

  test("shiftPhase_fromABar_leavesEarlierBarsAndAnchorsSectionsToTheShiftedBarLines", () => {
    // Given a Verse and a Chorus of two bars each, the third bar without a time
    const song = frozen([
      { id: "v", label: "Verse", bars: [{ startSec: 0, chords: [{ chord: "C:maj", beats: 4 }] }, { startSec: 2, chords: [{ chord: "D:min", beats: 4 }] }] },
      { id: "c", label: "Chorus", bars: [{ chords: [{ chord: "G:maj", beats: 4 }] }, { startSec: 6, chords: [{ chord: "F:maj", beats: 4 }] }] },
    ]);
    const from = { section: 0, bar: 1 };

    // When shifting by -1 beat from the second bar
    const shifted = valid(shiftPhase(song, -1, from));

    // Then the first bar is the same object, the Chorus starts one beat earlier, and bars starting in the untimed bar have no time
    expect(shifted.sections[0].bars[0]).toBe(song.sections[0].bars[0]);
    expect(shape(shifted)).toEqual([
      ["0|C:maj:4", "2|D:min:3"],
      ["3.5|D:min:1 G:maj:3", "-|G:maj:1 F:maj:3", "7.5|F:maj:1"],
    ]);
    expect(shifted.sections[1]).toMatchObject({ id: "c", label: "Chorus" });

    // When shifting twice by +1 from the second bar, Then it becomes a 2-beat break bar
    const twice = valid(shiftPhase(shiftPhase(song, 1, from), 1, from));
    expect(shape(twice)).toEqual([
      ["0|C:maj:4", "2|D:min:2", "3|D:min:2 G:maj:2"],
      ["-|G:maj:2 F:maj:2", "7|F:maj:2"],
    ]);
    expect(twice.sections[0].bars[1].meter).toEqual({ beats: 2, unit: 4 });
  });

  test("shiftPhase_acrossABarWithItsOwnMeter_throws", () => {
    // Given a 4/4 song with a 2/4 bar in the middle
    const song = frozen([
      {
        id: "v",
        label: "Verse",
        bars: [
          { chords: [{ chord: "C:maj", beats: 4 }] },
          { meter: { beats: 2, unit: 4 }, chords: [{ chord: "C:maj", beats: 2 }] },
          { chords: [{ chord: "C:maj", beats: 4 }] },
        ],
      },
    ]);

    // When / Then shifting across it is refused, shifting after it isn't
    expect(() => shiftPhase(song, 1)).toThrow(/own meter/);
    expect(() => shiftPhase(song, 1, { section: 0, bar: 2 })).not.toThrow();
  });

  test("shiftPhase_ofAGuitarSetReferenceShiftedByOneBeat_restoresIt", () => {
    // Given the first 12 bars of GuitarSet 00_Rock1-130-A_comp (130 BPM), with its bar phase off by one beat
    const chords = ["A:maj", "A:maj", "A:maj", "A:maj", "D:maj/5", "D:maj/5", "A:maj", "A:maj", "E:maj", "D:sus2/2", "A:maj/5", "A:maj/5"];
    const reference = frozen([
      { id: "s", label: "Song", bars: chords.map((chord, i) => ({ startSec: (i * 4 * 60) / 130, chords: [{ chord, beats: 4 }] })) },
    ]);
    const wrong = shiftPhase(reference, 1);

    // When fixing the phase in one action
    const fixed = shiftPhase(wrong, -1);

    // Then every bar is back on its reference chord and time
    expect(shape(wrong)).not.toEqual(shape(reference));
    expect(fixed.sections[0].bars.map((b) => b.chords)).toEqual(reference.sections[0].bars.map((b) => b.chords));
    fixed.sections[0].bars.forEach((b, i) => expect(b.startSec).toBeCloseTo(reference.sections[0].bars[i].startSec ?? NaN, 9));
  });
});
