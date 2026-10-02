// The Song file format (spec §4). Unknown fields are kept on every object so a load/save never drops them (NF-7).

export type Meter = { readonly beats: number; readonly unit: 2 | 4 | 8 };
export type SongMeter = Meter;

export type ChordSlot = {
  readonly chord: string; // Harte, e.g. "C:maj7/3", "A:min", "N"; syntax is checked by the chord parser, not here
  readonly beats: number;
  readonly confidence?: number;
};

export type Bar = {
  readonly startSec?: number;
  readonly meter?: Meter;
  readonly chords: readonly ChordSlot[];
};

export type Section = {
  readonly id: string;
  readonly label: string;
  readonly repeat?: number;
  readonly bars: readonly Bar[];
};

export type Notation = "chords" | "nashville";

export type Song = {
  readonly version: 1;
  readonly meta: {
    readonly title: string;
    readonly artist?: string;
    readonly key?: string;
    readonly notation?: Notation; // absent = "chords"; a Nashville song without a key keeps its chords relative to C
    readonly tempoBpm?: number;
    readonly meter: SongMeter;
  };
  readonly audio?: { readonly path: string; readonly sha256: string };
  readonly sections: readonly Section[];
};

type Raw = Record<string, unknown>;

const fail = (path: string, expected: string, value: unknown): never => {
  throw new Error(`${path}: expected ${expected}, got ${JSON.stringify(value)}`);
};

const object = (value: unknown, path: string): Raw =>
  typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Raw) : fail(path, "an object", value);

const array = (value: unknown, path: string): unknown[] => (Array.isArray(value) ? value : fail(path, "an array", value));

const string = (value: unknown, path: string): string => (typeof value === "string" ? value : fail(path, "a string", value));

const optional = <T>(value: unknown, check: (v: unknown) => T): T | undefined => (value === undefined ? undefined : check(value));

const number = (value: unknown, path: string): number =>
  typeof value === "number" && Number.isFinite(value) ? value : fail(path, "a number", value);

const positiveInt = (value: unknown, path: string): number =>
  Number.isInteger(value) && (value as number) > 0 ? (value as number) : fail(path, "a positive integer", value);

const oneOf = <T>(value: unknown, allowed: readonly T[], path: string): T =>
  allowed.includes(value as T) ? (value as T) : fail(path, `one of ${allowed.join(", ")}`, value);

const parseMeter = (value: unknown, path: string): Meter => {
  const raw = object(value, path);
  return { ...raw, beats: positiveInt(raw.beats, `${path}.beats`), unit: oneOf(raw.unit, [2, 4, 8] as const, `${path}.unit`) };
};

const parseSlot = (value: unknown, path: string): ChordSlot => {
  const raw = object(value, path);
  const chord = string(raw.chord, `${path}.chord`);
  if (chord === "") fail(`${path}.chord`, "a non-empty string", chord);
  const confidence = optional(raw.confidence, (v) => number(v, `${path}.confidence`));
  if (confidence !== undefined && (confidence < 0 || confidence > 1)) fail(`${path}.confidence`, "a number in [0, 1]", confidence);
  return { ...raw, chord, beats: positiveInt(raw.beats, `${path}.beats`), confidence };
};

const parseBar = (value: unknown, path: string, songMeter: SongMeter): Bar => {
  const raw = object(value, path);
  const chords = array(raw.chords, `${path}.chords`).map((slot, i) => parseSlot(slot, `${path}.chords[${i}]`));
  const bar: Bar = {
    ...raw,
    startSec: optional(raw.startSec, (v) => number(v, `${path}.startSec`)),
    meter: optional(raw.meter, (v) => parseMeter(v, `${path}.meter`)),
    chords,
  };
  const expected = (bar.meter ?? songMeter).beats;
  const actual = chords.reduce((sum, slot) => sum + slot.beats, 0);
  if (actual !== expected) throw new Error(`${path}: slot beats sum to ${actual}, meter expects ${expected}`);
  return bar;
};

const parseSection = (value: unknown, path: string, songMeter: SongMeter): Section => {
  const raw = object(value, path);
  return {
    ...raw,
    id: string(raw.id, `${path}.id`),
    label: string(raw.label, `${path}.label`),
    repeat: optional(raw.repeat, (v) => positiveInt(v, `${path}.repeat`)),
    bars: array(raw.bars, `${path}.bars`).map((bar, i) => parseBar(bar, `${path}.bars[${i}]`, songMeter)),
  };
};

/** Validates a loaded file and returns it as a Song; throws an Error whose message starts with the offending path. */
export const parseSong = (json: unknown): Song => {
  const raw = object(json, "song");
  if (raw.version !== 1) throw new Error(`unsupported version ${JSON.stringify(raw.version)}`);
  const rawMeta = object(raw.meta, "meta");
  const rawMeter = object(rawMeta.meter, "meta.meter");
  const meter: SongMeter = {
    ...rawMeter,
    beats: positiveInt(rawMeter.beats, "meta.meter.beats"),
    unit: oneOf(rawMeter.unit, [2, 4, 8] as const, "meta.meter.unit"),
  };
  const audio = optional(raw.audio, (v) => {
    const rawAudio = object(v, "audio");
    return { ...rawAudio, path: string(rawAudio.path, "audio.path"), sha256: string(rawAudio.sha256, "audio.sha256") };
  });
  return {
    ...raw,
    version: 1,
    meta: {
      ...rawMeta,
      title: string(rawMeta.title, "meta.title"),
      artist: optional(rawMeta.artist, (v) => string(v, "meta.artist")),
      key: optional(rawMeta.key, (v) => string(v, "meta.key")),
      notation: optional(rawMeta.notation, (v) => oneOf(v, ["chords", "nashville"] as const, "meta.notation")),
      tempoBpm: optional(rawMeta.tempoBpm, (v) => number(v, "meta.tempoBpm")),
      meter,
    },
    audio,
    sections: array(raw.sections, "sections").map((section, i) => parseSection(section, `sections[${i}]`, meter)),
  };
};

// Absent optional fields are `undefined` on parsed objects; JSON.stringify omits them, like the engine does.
export const serializeSong = (song: Song): string => JSON.stringify(song, null, 2);

export const newSectionId = (): string => crypto.randomUUID().slice(0, 8);

export const emptySong = (meter: SongMeter = { beats: 4, unit: 4 }, notation?: Notation): Song => ({
  version: 1,
  meta: { title: "Untitled", notation, meter },
  sections: [
    {
      id: newSectionId(),
      label: "Verse",
      bars: Array.from({ length: 4 }, () => ({ chords: [{ chord: "N", beats: meter.beats }] })),
    },
  ],
});

export const barBeats = (song: Song, bar: Bar): number => (bar.meter ?? song.meta.meter).beats;
