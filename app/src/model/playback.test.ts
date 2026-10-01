import { expect, test } from "vitest";
import { barAtTime } from "./playback";
import type { Song } from "./song";

const bar = (startSec?: number) => ({ startSec, chords: [{ chord: "C:maj", beats: 4 }] });

test("barAtTime_returnsTheBarWithTheLatestStartAtOrBeforeTheTime", () => {
  // Given two sections moved out of time order, and a manually added bar without startSec
  const song: Song = {
    version: 1,
    meta: { title: "t", meter: { beats: 4, unit: 4 } },
    sections: [
      { id: "b", label: "Chorus", bars: [bar(10), bar(12)] },
      { id: "a", label: "Verse", bars: [bar(2), bar(), bar(4)] },
    ],
  };

  // Then nothing before the first bar
  expect(barAtTime(song, 1.9)).toBeNull();
  // And a bar from its start until the next timed bar, skipping the untimed one
  expect(barAtTime(song, 2)).toEqual({ section: 1, bar: 0 });
  expect(barAtTime(song, 3.99)).toEqual({ section: 1, bar: 0 });
  expect(barAtTime(song, 4)).toEqual({ section: 1, bar: 2 });
  expect(barAtTime(song, 11)).toEqual({ section: 0, bar: 0 });
  // And the last bar after the end
  expect(barAtTime(song, 99)).toEqual({ section: 0, bar: 1 });
  // And nothing in a song without timing
  expect(barAtTime({ ...song, sections: [{ id: "c", label: "x", bars: [bar()] }] }, 5)).toBeNull();
});
