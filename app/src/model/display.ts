import { DEGREES, parseHarte, semitone } from "./chord";

export type DisplayStyle = "fr" | "intl";

const FR: Record<string, string> = {
  maj: "", min: "m", aug: "+", dim: "°", sus2: "sus2", sus4: "sus4", add2: "add2", add4: "add4",
  "7": "7", maj7: "7M", min7: "m7", minmaj7: "m7M", maj6: "6", min6: "m6", dim7: "°7", hdim7: "m7b5",
  "9": "9", maj9: "7M9", min9: "m9", "11": "11", "13": "13",
};
const SUFFIXES: Record<DisplayStyle, Record<string, string>> = {
  fr: FR,
  intl: { ...FR, maj7: "maj7", minmaj7: "m(maj7)", dim: "dim", dim7: "dim7", hdim7: "ø7", maj9: "maj9" },
};

const SHARPS = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const FLATS = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];

export const noteName = (pitchClass: number, isFlat: boolean): string =>
  (isFlat ? FLATS : SHARPS)[((pitchClass % 12) + 12) % 12];

export const displayChord = (harte: string, style: DisplayStyle): string => {
  if (harte === "N") return "N.C.";
  const parsed = parseHarte(harte);
  if (!parsed) return harte;
  const { root, quality, bass } = parsed;
  const chord = root + SUFFIXES[style][quality];
  if (bass === undefined) return chord;
  const isFlat = root[1] === "b" || bass.startsWith("b");
  return `${chord}/${noteName(semitone(root) + DEGREES.indexOf(bass), isFlat)}`;
};
