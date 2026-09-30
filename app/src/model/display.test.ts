import { describe, expect, test } from "vitest";
import { parseChord, QUALITIES } from "./chord";
import { displayChord } from "./display";

// [harte, fr, intl]
const rows: [string, string, string][] = [
  ["C:maj", "C", "C"],
  ["C:min", "Cm", "Cm"],
  ["C:aug", "C+", "C+"],
  ["C:dim", "C°", "Cdim"],
  ["C:sus2", "Csus2", "Csus2"],
  ["C:sus4", "Csus4", "Csus4"],
  ["C:add2", "Cadd2", "Cadd2"],
  ["C:add4", "Cadd4", "Cadd4"],
  ["C:7", "C7", "C7"],
  ["C:maj7", "C7M", "Cmaj7"],
  ["C:min7", "Cm7", "Cm7"],
  ["C:minmaj7", "Cm7M", "Cm(maj7)"],
  ["C:maj6", "C6", "C6"],
  ["C:min6", "Cm6", "Cm6"],
  ["C:dim7", "C°7", "Cdim7"],
  ["B:hdim7", "Bm7b5", "Bø7"],
  ["C:9", "C9", "C9"],
  ["C:maj9", "C7M9", "Cmaj9"],
  ["C:min9", "Cm9", "Cm9"],
  ["C:11", "C11", "C11"],
  ["C:13", "C13", "C13"],
  ["C:maj/3", "C/E", "C/E"],
  ["A:min/b7", "Am/G", "Am/G"],
  ["Eb:maj/5", "Eb/Bb", "Eb/Bb"],
  ["C:maj/b6", "C/Ab", "C/Ab"],
  ["C:maj/b5", "C/Gb", "C/Gb"],
  ["G:maj/7", "G/F#", "G/F#"],
  ["F#:min7/b7", "F#m7/E", "F#m7/E"],
  ["Db:maj7/2", "Db7M/Eb", "Dbmaj7/Eb"],
  ["N", "N.C.", "N.C."],
  ["%", "%", "%"],
];

describe("displayChord", () => {
  test("rows_coverEveryQuality", () => {
    // Given / When
    const covered = new Set(rows.map(([harte]) => harte.split(/[:/]/)[1]));

    // Then
    expect(QUALITIES.filter((quality) => !covered.has(quality))).toEqual([]);
  });

  test.each(rows)("displayChord_of%s_returnsFrAndIntlSpellings", (harte, fr, intl) => {
    // Given / When / Then
    expect(displayChord(harte, "fr")).toBe(fr);
    expect(displayChord(harte, "intl")).toBe(intl);
  });

  test.each(rows)("displayChord_of%s_parsesBackToSameHarte", (harte, fr, intl) => {
    // Given / When / Then
    expect(parseChord(fr)).toEqual({ ok: true, harte });
    expect(parseChord(intl)).toEqual({ ok: true, harte });
  });
});
