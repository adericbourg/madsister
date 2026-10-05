import { describe, expect, test } from "vitest";
import { parseChord, QUALITIES } from "./chord";
import { displayChord, displayKey, speakChord } from "./display";

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
  ["C:minadd2", "Cmadd2", "Cmadd2"],
  ["C:minadd4", "Cmadd4", "Cmadd4"],
  ["C:7sus4", "C7sus4", "C7sus4"],
  ["C:9sus4", "C9sus4", "C9sus4"],
  ["C:69", "C69", "C69"],
  ["C:min69", "Cm69", "Cm69"],
  ["C:min11", "Cm11", "Cm11"],
  ["C:maj13", "C7M13", "Cmaj13"],
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

  test.each([
    ["C:maj", "Do"],
    ["D:min", "Rém"],
    ["E:7", "Mi7"],
    ["F:maj7", "Fa7M"],
    ["G:dim", "Sol°"],
    ["A:min7", "Lam7"],
    ["B:hdim7", "Sim7b5"],
    ["Db:maj7/2", "Réb7M/Mib"],
    ["F#:min7/b7", "Fa#m7/Mi"],
    ["C:maj/3", "Do/Mi"],
    ["N", "N.C."],
    ["%", "%"],
  ])("displayChord_of%s_returnsLatinRootsWithFrenchSuffixes", (harte, latin) => {
    // Given / When / Then
    expect(displayChord(harte, "latin")).toBe(latin);
  });

  test.each(rows)("displayChord_of%s_parsesBackToSameHarte", (harte, fr, intl) => {
    // Given / When / Then
    expect(parseChord(fr)).toEqual({ ok: true, harte });
    expect(parseChord(intl)).toEqual({ ok: true, harte });
  });
});

describe("speakChord", () => {
  test("speakChord_ofHarte_spellsItForScreenReaders", () => {
    // Given / When / Then
    expect(speakChord("A:min7")).toBe("A minor 7");
    expect(speakChord("Bb:maj7/3")).toBe("B flat major 7 over D");
    expect(speakChord("F#:hdim7")).toBe("F sharp half-diminished 7");
    expect(speakChord("C:maj")).toBe("C");
    expect(speakChord("N")).toBe("no chord");
    expect(speakChord("%")).toBe("repeat previous bar");
  });
});

describe("displayKey", () => {
  test.each([
    ["F#m", "latin", "Fa#m"],
    ["F#m", "fr", "F#m"],
    ["Bb", "latin", "Sib"],
    ["Am", "intl", "Am"],
    ["C", "fr", "C"],
    ["C:min", "fr", "Cm"],
  ] as const)("displayKey_of%s_%s_returns%s", (key, style, expected) => {
    // Given / When / Then
    expect(displayKey(key, style)).toBe(expected);
  });
});
