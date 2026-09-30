export type ParseResult = { ok: true; harte: string } | { ok: false; error: string };
export type ParsedHarte = { root: string; quality: string; bass?: string };

const ALIASES: Record<string, string> = {
  "": "maj", maj: "maj", M: "maj",
  m: "min", min: "min", "-": "min",
  "+": "aug", aug: "aug",
  "°": "dim", o: "dim", dim: "dim",
  sus: "sus4", sus4: "sus4", sus2: "sus2",
  add2: "add2", add9: "add2", add4: "add4", add11: "add4",
  "7": "7",
  "7M": "maj7", maj7: "maj7", M7: "maj7", "Δ": "maj7", "Δ7": "maj7", ma7: "maj7",
  m7: "min7", min7: "min7", "-7": "min7",
  m7M: "minmaj7", mM7: "minmaj7", mmaj7: "minmaj7", minmaj7: "minmaj7", "-Δ": "minmaj7",
  "6": "maj6", m6: "min6", min6: "min6", "-6": "min6",
  "°7": "dim7", o7: "dim7", dim7: "dim7",
  m7b5: "hdim7", "ø": "hdim7", "ø7": "hdim7", "-7b5": "hdim7", hdim7: "hdim7",
  "9": "9", maj9: "maj9", "7M9": "maj9", m9: "min9", "11": "11", "13": "13",
};

export const QUALITIES: readonly string[] = [...new Set(Object.values(ALIASES))];

const DEGREES = ["1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7"];
const NATURALS: Record<string, number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };
const NOTE = /^[A-G][#b]?$/;

const semitone = (note: string): number =>
  NATURALS[note[0]] + (note[1] === "#" ? 1 : note[1] === "b" ? -1 : 0);

export const parseChord = (input: string): ParseResult => {
  const text = input.trim().replace(/♭/g, "b").replace(/♯/g, "#");
  if (text === "N" || text === "N.C." || text === "NC") return { ok: true, harte: "N" };
  if (text === "%") return { ok: true, harte: "%" };
  const match = /^([A-G][#b]?)([^/]*)(?:\/(.*))?$/.exec(text);
  if (!match) return { ok: false, error: `"${input}" doesn't start with a note (A–G, optional # or b)` };
  const [, root, alias, bassNote] = match;
  const quality = ALIASES[alias];
  if (quality === undefined) {
    const isMinorAdd = /^(m|min|-)\(?add/.test(alias);
    return {
      ok: false,
      error: isMinorAdd ? `minor add chords ("${alias}") aren't supported yet` : `unknown chord quality "${alias}"`,
    };
  }
  if (bassNote === undefined) return { ok: true, harte: `${root}:${quality}` };
  if (!NOTE.test(bassNote)) return { ok: false, error: `bass "${bassNote}" isn't a note (A–G, optional # or b)` };
  const degree = DEGREES[(semitone(bassNote) - semitone(root) + 12) % 12];
  return { ok: true, harte: `${root}:${quality}/${degree}` };
};

// Returns null for anything that isn't a "Root:quality[/degree]" chord, including "N" and "%".
export const parseHarte = (harte: string): ParsedHarte | null => {
  const match = /^([^:]+):([^/]+)(?:\/(.+))?$/.exec(harte);
  if (!match) return null;
  const [, root, quality, bass] = match;
  if (!NOTE.test(root) || !QUALITIES.includes(quality) || (bass !== undefined && !DEGREES.includes(bass))) return null;
  return bass === undefined ? { root, quality } : { root, quality, bass };
};
