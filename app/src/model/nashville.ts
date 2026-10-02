// Nashville numbers: chords are stored absolute (Harte) and shown as degrees of the song's tonic.
// A Nashville song without a key keeps its chords relative to C, so C is degree 1.
import { DEGREES, parseChord, parseHarte, parseKey, semitone, type ParseResult } from "./chord";
import { noteName, SPOKEN, SUFFIXES, type DisplayStyle } from "./display";
import type { Spelling } from "./transpose";

export type MinorConvention = "relative" | "tonic";

const MAJOR_SCALE = [0, 2, 4, 5, 7, 9, 11];
const SHARP_TONICS = [2, 4, 6, 7, 9, 11];
const mod12 = (n: number): number => ((n % 12) + 12) % 12;

/** Pitch class of degree 1: C without a key; a minor key's relative major with the "relative" convention (Am → C). */
export const tonicPc = (key: string | undefined, convention: MinorConvention): number => {
  if (key === undefined) return 0;
  const { root, isMinor } = parseKey(key);
  return isMinor && convention === "relative" ? mod12(semitone(root) + 3) : semitone(root);
};

export const spellingFor = (tonic: number): Spelling => (SHARP_TONICS.includes(mod12(tonic)) ? "sharp" : "flat");

const degreeOf = (pc: number, tonic: number): string => DEGREES[mod12(pc - tonic)];

export const displayNashville = (harte: string, tonic: number, style: DisplayStyle): string => {
  if (harte === "N") return "N.C.";
  const parsed = parseHarte(harte);
  if (!parsed) return harte;
  const root = semitone(parsed.root);
  const chord = degreeOf(root, tonic) + SUFFIXES[style][parsed.quality];
  return parsed.bass === undefined ? chord : `${chord}/${degreeOf(root + DEGREES.indexOf(parsed.bass), tonic)}`;
};

// "b3" → the note a minor third above the tonic, spelled with the key's accidentals unless the degree says otherwise.
const degreeNote = (accidental: string, degree: string, tonic: number): string => {
  const pc = tonic + MAJOR_SCALE[Number(degree) - 1] + (accidental === "b" ? -1 : accidental === "#" ? 1 : 0);
  const isFlat = accidental === "b" || (accidental === "" && spellingFor(tonic) === "flat");
  return noteName(pc, isFlat);
};

export const parseNashville = (input: string, tonic: number): ParseResult => {
  const text = input.trim().replace(/♭/g, "b").replace(/♯/g, "#");
  if (text === "N" || text === "N.C." || text === "NC" || text === "%") return parseChord(text);
  const match = /^([b#]?)([1-7])([^/]*)(?:\/([b#]?)([1-7]))?$/.exec(text);
  if (!match) return { ok: false, error: `"${input}" isn't a degree (1–7, optional b or #)` };
  const [, accidental, degree, alias, bassAccidental, bassDegree] = match;
  const bass = bassDegree === undefined ? "" : `/${degreeNote(bassAccidental, bassDegree, tonic)}`;
  return parseChord(`${degreeNote(accidental, degree, tonic)}${alias}${bass}`);
};

const speakDegree = (degree: string): string => degree.replace("b", "flat ").replace("#", "sharp ");

/** Spells a Nashville chord for screen readers, e.g. "Bb:maj7" in C → "flat 7 major 7". */
export const speakNashville = (harte: string, tonic: number): string => {
  if (harte === "N") return "no chord";
  if (harte === "%") return "repeat previous bar";
  const parsed = parseHarte(harte);
  if (!parsed) return harte;
  const [, bass] = displayNashville(harte, tonic, "intl").split("/");
  return speakDegree(degreeOf(semitone(parsed.root), tonic)) + SPOKEN[parsed.quality] + (bass === undefined ? "" : ` over ${speakDegree(bass)}`);
};
