import { expect, test } from "vitest";
import type { Bar, Song } from "../song";
import { toMidi } from "./midi";

const bar = (...chords: [string, number][]): Bar => ({ chords: chords.map(([chord, beats]) => ({ chord, beats })) });

const hex = (bytes: Uint8Array): string => Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join(" ");

const tempos = (bytes: Uint8Array): number[] =>
  bytes.reduce<number[]>(
    (found, b, i) =>
      b === 0xff && bytes[i + 1] === 0x51 && bytes[i + 2] === 3 ? [...found, (bytes[i + 3] << 16) | (bytes[i + 4] << 8) | bytes[i + 5]] : found,
    [],
  );

test("toMidi_ofATwoBarSong_matchesTheGoldenBytes", () => {
  // Given a 2-bar 4/4 song without tempo: C, then G/B and N for 2 beats each
  const song: Song = {
    version: 1,
    meta: { title: "Tiny", meter: { beats: 4, unit: 4 } },
    sections: [{ id: "a", label: "Verse", bars: [bar(["C:maj", 4]), bar(["G:maj/3", 2], ["N", 2])] }],
  };

  // When
  const midi = toMidi(song);

  // Then: type 0, 480 PPQ, 4/4 at 120 BPM, C3+C4 E4 G4 for a whole note, B3+G4 B4 D5 for a half note, a half rest
  expect(hex(midi)).toBe(
    [
      "4d 54 68 64 00 00 00 06 00 00 00 01 01 e0",
      "4d 54 72 6b 00 00 00 56",
      "00 ff 58 04 04 02 18 08",
      "00 ff 51 03 07 a1 20",
      "00 90 30 50 00 90 3c 50 00 90 40 50 00 90 43 50",
      "8f 00 80 30 00 00 80 3c 00 00 80 40 00 00 80 43 00",
      "00 90 3b 50 00 90 43 50 00 90 47 50 00 90 4a 50",
      "87 40 80 3b 00 00 80 43 00 00 80 47 00 00 80 4a 00",
      "87 40 ff 2f 00",
    ].join(" "),
  );
});

test("toMidi_whenBarsHaveStartSec_followsTheRecordingAfterALeadingRest", () => {
  // Given bars at 1 s, 3 s and 4 s, the second one in 2/4, in a section played twice
  const song: Song = {
    version: 1,
    meta: { title: "Timed", tempoBpm: 90, meter: { beats: 4, unit: 4 } },
    sections: [
      {
        id: "a",
        label: "Verse",
        repeat: 2,
        bars: [
          { ...bar(["A:min", 4]), startSec: 1 },
          { ...bar(["%", 2]), startSec: 3, meter: { beats: 2, unit: 4 } },
          { ...bar(["N", 4]), startSec: 4 },
        ],
      },
    ],
  };

  // When
  const midi = toMidi(song);

  // Then the header and track chunks are consistent and the track ends with end-of-track
  expect(hex(midi.slice(0, 14))).toBe("4d 54 68 64 00 00 00 06 00 00 00 01 01 e0");
  expect(hex(midi.slice(14, 18))).toBe("4d 54 72 6b");
  expect(new DataView(midi.buffer).getUint32(18)).toBe(midi.length - 22);
  expect(hex(midi.slice(-3))).toBe("ff 2f 00");
  // a 1 s rest bar (4 quarters → 250 ms each), then 2 s per 4/4 bar, 1 s per 2/4 bar, the last bars keep the previous tempo
  expect(tempos(midi)).toEqual([250000, 500000]);
  // the time signature switches to 2/4 and back to 4/4 on each pass
  expect(hex(midi).match(/ff 58 04 0[24] 02/g)).toEqual([
    "ff 58 04 04 02", "ff 58 04 02 02", "ff 58 04 04 02", "ff 58 04 02 02", "ff 58 04 04 02",
  ]);
});
