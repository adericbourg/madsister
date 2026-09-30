import { describe, expect, test } from "vitest";
import { parseSong } from "./song";
import { transposeChord, transposeSong } from "./transpose";

describe("transposeChord", () => {
  test.each([
    ["C:maj7", 2, "sharp", "D:maj7"],
    ["B:min", 1, "flat", "C:min"],
    ["F#:7/3", -1, "flat", "F:7/3"],
    ["C#:min", 0, "flat", "Db:min"],
    ["Bb:maj/5", -13, "sharp", "A:maj/5"],
    ["N", 3, "sharp", "N"],
    ["%", 3, "flat", "%"],
  ] as const)("transposeChord_of%s_by%i_%s_returns%s", (harte, semitones, spelling, expected) => {
    // Given / When / Then
    expect(transposeChord(harte, semitones, spelling)).toBe(expected);
  });
});

describe("transposeSong", () => {
  const song = parseSong({
    version: 1,
    meta: { title: "Song", key: "Am", meter: { beats: 4, unit: 4 } },
    sections: [
      {
        id: "s1",
        label: "Verse",
        bars: [
          { chords: [{ chord: "A:min", beats: 2, confidence: 0.4 }, { chord: "F#:7/3", beats: 2 }] },
          { chords: [{ chord: "N", beats: 4 }] },
        ],
      },
    ],
  });

  test("transposeSong_byTwelveWithOriginalSpelling_returnsOriginalSong", () => {
    // Given / When / Then
    expect(transposeSong(song, 12, "sharp")).toEqual(song);
  });

  test("transposeSong_byThreeFlat_transposesChordsAndKeyAndKeepsConfidence", () => {
    // Given / When
    const transposed = transposeSong(song, 3, "flat");

    // Then
    expect(transposed.meta.key).toBe("Cm");
    expect(transposed.sections[0].bars[0].chords).toEqual([
      { chord: "C:min", beats: 2, confidence: 0.4 },
      { chord: "A:7/3", beats: 2 },
    ]);
    expect(transposed.sections[0].bars[1].chords[0].chord).toBe("N");
  });
});
