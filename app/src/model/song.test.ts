import { describe, expect, test } from "vitest";
import { barBeats, emptySong, parseSong, serializeSong } from "./song";

const validFile = () => ({
  version: 1,
  producedBy: "engine 0.1",
  meta: { title: "Song", artist: "Someone", key: "A:min", tempoBpm: 120, meter: { beats: 4, unit: 4 }, mood: "sad" },
  audio: { path: "song.mp3", sha256: "abc" },
  sections: [
    {
      id: "s1",
      label: "Verse",
      repeat: 2,
      color: "red",
      bars: [
        { startSec: 0.5, chords: [{ chord: "A:min", beats: 4, confidence: 0.9, source: "btc" }], lyric: "la" },
        { meter: { beats: 2, unit: 4 }, chords: [{ chord: "C:maj/b6", beats: 1 }, { chord: "G:add2/2", beats: 1 }] },
      ],
    },
  ],
});

describe("parseSong", () => {
  test("parseSong_ofFileWithUnknownFieldsAndBarMeterOverride_roundTripsThroughSerializeSong", () => {
    // Given a file with unknown fields at every level and a 2/4 bar in a 4/4 song
    const file = validFile();

    // When
    const serialized = serializeSong(parseSong(file));

    // Then
    expect(JSON.parse(serialized)).toEqual(file);
  });

  test.each([
    ["wrong version", (f: any) => (f.version = 2), "unsupported version 2"],
    ["beat sum mismatch", (f: any) => (f.sections[0].bars[0].chords[0].beats = 3), "sections[0].bars[0]"],
    ["beat sum mismatch against the override", (f: any) => (f.sections[0].bars[1].chords[0].beats = 3), "sections[0].bars[1]"],
    ["non-integer beats", (f: any) => (f.sections[0].bars[0].chords[0].beats = 2.5), "sections[0].bars[0].chords[0].beats"],
    ["zero beats", (f: any) => (f.sections[0].bars[1].chords[1].beats = 0), "sections[0].bars[1].chords[1].beats"],
    ["empty chord", (f: any) => (f.sections[0].bars[0].chords[0].chord = ""), "sections[0].bars[0].chords[0].chord"],
    ["missing sections", (f: any) => delete f.sections, "sections"],
  ])("parseSong_whenInvalid_throwsWithPath (%s)", (_, mutate, message) => {
    // Given
    const file = validFile();
    mutate(file);

    // When / Then
    expect(() => parseSong(file)).toThrow(message);
  });
});

describe("emptySong", () => {
  test("emptySong_returnsAValidSongWithOneVerseOfFourEmptyBars", () => {
    // Given / When
    const song = emptySong({ beats: 3, unit: 4 });

    // Then
    expect(parseSong(JSON.parse(serializeSong(song)))).toEqual(song);
    expect(song.meta).toEqual({ title: "Untitled", meter: { beats: 3, unit: 4 } });
    expect(song.sections).toHaveLength(1);
    expect(song.sections[0].label).toBe("Verse");
    expect(song.sections[0].id).toMatch(/^[0-9a-f]{8}$/);
    expect(song.sections[0].bars).toEqual(Array(4).fill({ chords: [{ chord: "N", beats: 3 }] }));
    expect(barBeats(song, song.sections[0].bars[0])).toBe(3);
    expect(emptySong().meta.meter).toEqual({ beats: 4, unit: 4 });
  });

  test("emptySong_ofNashville_hasTheNotationAndNoKey", () => {
    // Given / When
    const song = emptySong({ beats: 4, unit: 4 }, "nashville");

    // Then
    expect(song.meta.notation).toBe("nashville");
    expect(song.meta.key).toBeUndefined();
    expect(parseSong(JSON.parse(serializeSong(song)))).toEqual(song);
    expect(emptySong().meta.notation).toBeUndefined();
  });
});

describe("parseSong notation", () => {
  test("parseSong_ofFileWithoutNotation_leavesItUndefined", () => {
    // Given / When / Then
    expect(parseSong(validFile()).meta.notation).toBeUndefined();
  });

  test("parseSong_ofUnknownNotation_throwsWithPath", () => {
    // Given
    const file = validFile();
    (file.meta as Record<string, unknown>).notation = "roman";

    // When / Then
    expect(() => parseSong(file)).toThrow("meta.notation");
  });
});
