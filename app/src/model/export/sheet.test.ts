import { expect, test } from "vitest";
import type { Bar, Song } from "../song";
import { toSheet, type SheetOptions } from "./sheet";

const bar = (...chords: [string, number][]): Bar => ({ chords: chords.map(([chord, beats]) => ({ chord, beats })) });
const options: SheetOptions = { style: "intl", minorConvention: "relative", font: "kalam", compactPrint: true, barsPerRow: 4 };

test("toSheet_ofASong_spellsItsChordsAndHeader", () => {
  // Given a song with a repeated section, a split bar, a meter override and an empty section
  const song: Song = {
    version: 1,
    meta: { title: "Song 1", artist: "The Fixtures", key: "D", tempoBpm: 120, meter: { beats: 4, unit: 4 } },
    sections: [
      { id: "a", label: "Verse", repeat: 2, bars: [bar(["C:maj7", 4]), { ...bar(["A:min", 1], ["N", 1]), meter: { beats: 2, unit: 4 } }] },
      { id: "b", label: "Outro", repeat: 1, bars: [] },
    ],
  };

  // When
  const sheet = toSheet(song, options);

  // Then
  expect(sheet).toEqual({
    title: "Song 1",
    artist: "The Fixtures",
    facts: ["Key D", "♩ = 120"],
    font: "kalam",
    compact: true,
    barsPerRow: 4,
    sections: [
      {
        label: "Verse",
        repeat: 2,
        bars: [
          { meter: undefined, slots: [{ text: "Cmaj7", beats: 4 }] },
          { meter: "2/4", slots: [{ text: "Am", beats: 1 }, { text: "N.C.", beats: 1 }] },
        ],
      },
      { label: "Outro", repeat: undefined, bars: [] },
    ],
  });
});

test("toSheet_ofANashvilleSong_spellsDegreesOfTheKey", () => {
  // Given a Nashville song in G without tempo or artist
  const song: Song = {
    version: 1,
    meta: { title: "N", key: "G", notation: "nashville", meter: { beats: 4, unit: 4 } },
    sections: [{ id: "a", label: "A", bars: [bar(["G:maj", 2], ["E:min", 2])] }],
  };

  // When
  const sheet = toSheet(song, options);

  // Then
  expect(sheet.facts).toEqual(["Key G"]);
  expect(sheet.sections[0].bars[0].slots.map((s) => s.text)).toEqual(["1", "6m"]);
});
