import { expect, test } from "vitest";
import type { Bar, Song } from "../song";
import { toChordPro } from "./chordpro";
import golden from "./__golden__/song1.cho?raw";

const bar = (...chords: [string, number][]): Bar => ({ chords: chords.map(([chord, beats]) => ({ chord, beats })) });

test("toChordPro_ofASong_matchesTheGoldenFile", () => {
  // Given a song with a repeated section, a split bar, a slash chord, a repeat bar, N, and a 2-beat bar
  const song: Song = {
    version: 1,
    meta: { title: "Song 1", artist: "The Fixtures", key: "C", tempoBpm: 119.6, meter: { beats: 4, unit: 4 } },
    sections: [
      {
        id: "a",
        label: "Verse",
        repeat: 2,
        bars: [bar(["C:maj7", 4]), bar(["A:min", 2], ["G:maj/3", 2]), bar(["%", 4]), bar(["N", 4]), bar(["F:maj", 4])],
      },
      {
        id: "b",
        label: "Chorus",
        bars: [bar(["B:hdim7", 4]), { ...bar(["E:minmaj7", 2]), meter: { beats: 2, unit: 4 } }, bar(["D:min7", 4])],
      },
    ],
  };

  // When
  const cho = toChordPro(song);

  // Then
  expect(cho).toBe(golden);
});
