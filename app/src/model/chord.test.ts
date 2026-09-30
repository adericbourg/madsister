import { describe, expect, test } from "vitest";
import { parseChord, parseHarte } from "./chord";

const accepted: [string, string][] = [
  // F-ED-3 examples
  ["Am7", "A:min7"],
  ["A-7", "A:min7"],
  ["Amin7", "A:min7"],
  ["C7M", "C:maj7"],
  ["Cmaj7", "C:maj7"],
  ["CΔ", "C:maj7"],
  ["C/E", "C:maj/3"],
  ["Csus", "C:sus4"],
  ["C°", "C:dim"],
  // §4.1 Q1–Q11
  ["C", "C:maj"],
  ["Cm", "C:min"],
  ["C+", "C:aug"],
  ["Cdim", "C:dim"],
  ["Csus2", "C:sus2"],
  ["Csus4", "C:sus4"],
  ["Cadd9", "C:add2"],
  ["Cadd2", "C:add2"],
  ["Cadd11", "C:add4"],
  ["Cadd4", "C:add4"],
  ["C7", "C:7"],
  ["CmM7", "C:minmaj7"],
  ["C-Δ", "C:minmaj7"],
  ["Cm(maj7)", "C:minmaj7"],
  ["C6", "C:maj6"],
  ["Cm6", "C:min6"],
  ["Co7", "C:dim7"],
  ["Cm7b5", "C:hdim7"],
  ["Cø", "C:hdim7"],
  ["C/G", "C:maj/5"],
  ["C/Bb", "C:maj/b7"],
  ["Am/G", "A:min/b7"],
  ["N.C.", "N"],
  ["NC", "N"],
  ["N", "N"],
  ["%", "%"],
  // extensions (manual only)
  ["C9", "C:9"],
  ["C7M9", "C:maj9"],
  ["Cm9", "C:min9"],
  ["C11", "C:11"],
  ["C13", "C:13"],
  // accidentals, spelling kept, bass degrees beyond /3 /5 /b7
  ["F#m", "F#:min"],
  ["Bbmaj7", "Bb:maj7"],
  ["E♭", "Eb:maj"],
  ["D♯m7♭5", "D#:hdim7"],
  ["Db/Eb", "Db:maj/2"],
  ["C/Eb", "C:maj/b3"],
  ["C/Ab", "C:maj/b6"],
  ["C/F#", "C:maj/b5"],
  ["G/F#", "G:maj/7"],
  ["B/C", "B:maj/b2"],
  [" Cadd2/D ", "C:add2/2"],
];

describe("parseChord", () => {
  test.each(accepted)("parseChord_of%s_returnsHarte", (input, harte) => {
    // Given / When
    const result = parseChord(input);

    // Then
    expect(result).toEqual({ ok: true, harte });
  });

  test.each([["H"], ["Cx7"], ["C/"], ["C/H"], [""], ["Cmaj7/E/G"], ["c"], ["Amadd9"]])(
    "parseChord_of%s_returnsError",
    (input) => {
      // Given / When
      const result = parseChord(input);

      // Then
      expect(result.ok).toBe(false);
      expect(!result.ok && result.error).toMatch(/\S/);
    },
  );

  test("parseChord_ofMinorAddChord_explainsItIsNotSupported", () => {
    // Given / When
    const result = parseChord("Am(add9)");

    // Then
    expect(!result.ok && result.error).toMatch(/minor add/i);
  });
});

describe("parseHarte", () => {
  test.each(accepted.filter(([, harte]) => harte.includes(":")))(
    "parseHarte_ofParsedChord_roundTrips (%s)",
    (_, harte) => {
      // Given / When
      const parsed = parseHarte(harte);

      // Then
      expect(parsed).not.toBeNull();
      const { root, quality, bass } = parsed!;
      expect(`${root}:${quality}${bass ? `/${bass}` : ""}`).toBe(harte);
    },
  );

  test.each([["C:maj/b3"], ["C:min/2"], ["C:maj/b6"], ["A:7/b5"]])("parseHarte_ofEngineBassDegree_%s_parses", (harte) => {
    // Given / When
    const parsed = parseHarte(harte);

    // Then
    expect(parsed?.bass).toBe(harte.split("/")[1]);
  });

  test.each([["N"], ["%"], ["C"], ["C:foo"], ["H:maj"], ["C:maj/#9"], ["C:maj/3/5"]])(
    "parseHarte_of%s_returnsNull",
    (harte) => {
      // Given / When / Then
      expect(parseHarte(harte)).toBeNull();
    },
  );

  test("parseHarte_ofSlashChord_splitsRootQualityAndBass", () => {
    // Given / When / Then
    expect(parseHarte("Db:min7/b7")).toEqual({ root: "Db", quality: "min7", bass: "b7" });
    expect(parseHarte("A:min")).toEqual({ root: "A", quality: "min" });
  });
});
