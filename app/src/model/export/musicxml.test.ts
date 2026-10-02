import { expect, test } from "vitest";
import type { Bar, Song } from "../song";
import { toMusicXml } from "./musicxml";
import golden from "./__golden__/song1.musicxml?raw";

const bar = (...chords: [string, number][]): Bar => ({ chords: chords.map(([chord, beats]) => ({ chord, beats })) });

test("toMusicXml_ofASong_matchesTheGoldenFile", () => {
  // Given a song with a repeated section, a split bar, a slash chord, a repeat bar, N, add2, and a 2-beat bar
  const song: Song = {
    version: 1,
    meta: { title: "Song <1> & Co", artist: "The Fixtures", meter: { beats: 4, unit: 4 } },
    sections: [
      {
        id: "a",
        label: "Verse",
        repeat: 2,
        bars: [bar(["C:maj7", 4]), bar(["A:min", 2], ["G:maj/3", 2]), bar(["%", 4]), bar(["N", 4])],
      },
      {
        id: "b",
        label: "Chorus",
        bars: [bar(["Bb:add2", 4]), { ...bar(["E:minmaj7", 2]), meter: { beats: 2, unit: 4 } }, bar(["B:hdim7", 4])],
      },
    ],
  };

  // When
  const xml = toMusicXml(song);

  // Then
  expect(xml).toBe(golden);
});

test("toMusicXml_ofAHalfNoteUnitMeter_writesHalfNoteSlashes", () => {
  // Given a 3/2 song with one bar
  const song: Song = {
    version: 1,
    meta: { title: "T", meter: { beats: 3, unit: 2 } },
    sections: [{ id: "a", label: "A", bars: [{ chords: [{ chord: "C:maj", beats: 3 }] }] }],
  } as Song;

  // When
  const xml = toMusicXml(song);

  // Then
  expect(xml).toContain("<beats>3</beats><beat-type>2</beat-type>");
  expect(xml.match(/<type>half<\/type>/g)).toHaveLength(3);
});
