import { describe, expect, test } from "vitest";
import { displayNashville, parseNashville, speakNashville, spellingFor, tonicPc } from "./nashville";

describe("tonicPc", () => {
  test.each([
    [undefined, "relative", 0],
    [undefined, "tonic", 0],
    ["G", "relative", 7],
    ["Bb", "tonic", 10],
    ["Am", "relative", 0],
    ["Am", "tonic", 9],
    ["F#m", "relative", 9],
    ["F#m", "tonic", 6],
    ["C:min", "relative", 3],
    ["C:min", "tonic", 0],
  ] as const)("tonicPc_ofKey%s_with%sConvention_returns%i", (key, convention, expected) => {
    // Given / When / Then
    expect(tonicPc(key, convention)).toBe(expected);
  });
});

describe("spellingFor", () => {
  test("spellingFor_ofSharpKeys_returnsSharpAndOtherwiseFlat", () => {
    // Given / When / Then
    expect([7, 2, 9, 6].map(spellingFor)).toEqual(["sharp", "sharp", "sharp", "sharp"]);
    expect([0, 5, 10, 3].map(spellingFor)).toEqual(["flat", "flat", "flat", "flat"]);
  });
});

describe("displayNashville", () => {
  test.each([
    ["G:maj", 7, "intl", "1"],
    ["C:maj", 7, "intl", "4"],
    ["E:min", 7, "intl", "6m"],
    ["F:7", 0, "intl", "47"],
    ["Bb:maj7", 0, "intl", "b7maj7"],
    ["Bb:maj7", 0, "fr", "b77M"],
    ["C:maj/3", 0, "intl", "1/3"],
    ["C:maj/3", 7, "intl", "4/6"],
    ["N", 7, "intl", "N.C."],
    ["%", 7, "intl", "%"],
  ] as const)("displayNashville_of%s_inTonic%i_%s_returns%s", (harte, tonic, style, expected) => {
    // Given / When / Then
    expect(displayNashville(harte, tonic, style)).toBe(expected);
  });
});

describe("parseNashville", () => {
  test.each([
    ["1", 7, "G:maj"],
    ["4", 7, "C:maj"],
    ["6m", 7, "E:min"],
    ["47", 0, "F:7"],
    ["b7maj7", 0, "Bb:maj7"],
    ["♭3m", 0, "Eb:min"],
    ["#4°", 0, "F#:dim"],
    ["7", 7, "F#:maj"],
    ["4/6", 7, "C:maj/3"],
    ["N", 7, "N"],
    ["%", 7, "%"],
  ] as const)("parseNashville_of%s_inTonic%i_returns%s", (input, tonic, harte) => {
    // Given / When / Then
    expect(parseNashville(input, tonic)).toEqual({ ok: true, harte });
  });

  test.each([["Am"], ["8"], ["0m"], ["4xyz"], ["4/x"], [""]])("parseNashville_of%s_returnsAnError", (input) => {
    // Given / When
    const result = parseNashville(input, 0);

    // Then
    expect(result.ok).toBe(false);
  });

  test("parseNashville_ofLetterChord_explainsDegreesAreExpected", () => {
    // Given / When
    const result = parseNashville("Am", 0);

    // Then
    expect(result).toEqual({ ok: false, error: '"Am" isn\'t a degree (1–7, optional b or #)' });
  });

  test("parseNashville_thenDisplayNashville_roundTripsEveryDegree", () => {
    // Given every pitch class as root, in every key
    for (let tonic = 0; tonic < 12; tonic++) {
      for (const degree of ["1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7"]) {
        // When
        const parsed = parseNashville(`${degree}m7/5`, tonic);

        // Then the bass is counted from the key, like the root
        expect(parsed.ok).toBe(true);
        if (parsed.ok) expect(displayNashville(parsed.harte, tonic, "intl")).toBe(`${degree}m7/5`);
      }
    }
  });
});

describe("speakNashville", () => {
  test("speakNashville_ofHarte_spellsTheDegreesForScreenReaders", () => {
    // Given / When / Then
    expect(speakNashville("E:min", 7)).toBe("6 minor");
    expect(speakNashville("Bb:maj7", 0)).toBe("flat 7 major 7");
    expect(speakNashville("C:maj/3", 7)).toBe("4 over 6");
    expect(speakNashville("N", 7)).toBe("no chord");
    expect(speakNashville("%", 7)).toBe("repeat previous bar");
  });
});
