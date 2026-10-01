// MusicXML 4.0 export (spec F-EX-3): one measure per bar, a <harmony> per slot over one slash note per beat.
import { parseHarte } from "../chord";
import { displayChord } from "../display";
import { barBeats, type Bar, type Meter, type Song } from "../song";

const KINDS: Record<string, string> = {
  maj: "major", min: "minor", aug: "augmented", dim: "diminished", sus2: "suspended-second", sus4: "suspended-fourth",
  add2: "major", add4: "major", "7": "dominant", maj7: "major-seventh", min7: "minor-seventh", minmaj7: "major-minor",
  maj6: "major-sixth", min6: "minor-sixth", dim7: "diminished-seventh", hdim7: "half-diminished",
  "9": "dominant-ninth", maj9: "major-ninth", min9: "minor-ninth", "11": "dominant-11th", "13": "dominant-13th",
};
// Spelled like the app ("Bbadd2"), not as the equivalent add9/add11.
const ADDED: Record<string, string> = { add2: "2", add4: "4" };

const DIVISIONS = 2; // per quarter note, so an eighth-note beat is 1

const escape = (text: string): string =>
  text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

const pitch = (element: "root" | "bass", note: string): string =>
  `<${element}><${element}-step>${note[0]}</${element}-step>` +
  (note[1] === undefined ? "" : `<${element}-alter>${note[1] === "#" ? 1 : -1}</${element}-alter>`) +
  `</${element}>`;

const harmony = (harte: string): string | undefined => {
  if (harte === "N") return '<harmony><root><root-step text="">C</root-step></root><kind text="N.C.">none</kind></harmony>';
  const parsed = parseHarte(harte);
  if (!parsed) return undefined;
  const [, bass] = displayChord(harte, "intl").split("/");
  const added = ADDED[parsed.quality];
  return (
    `<harmony>${pitch("root", parsed.root)}<kind>${KINDS[parsed.quality]}</kind>` +
    (bass === undefined ? "" : pitch("bass", bass)) +
    (added === undefined
      ? ""
      : `<degree><degree-value>${added}</degree-value><degree-alter>0</degree-alter><degree-type>add</degree-type></degree>`) +
    "</harmony>"
  );
};

const slash = (unit: 4 | 8): string =>
  `<note><pitch><step>B</step><octave>4</octave></pitch><duration>${(DIVISIONS * 4) / unit}</duration><voice>1</voice>` +
  `<type>${unit === 4 ? "quarter" : "eighth"}</type><stem>none</stem><notehead>slash</notehead></note>`;

const time = (meter: Meter): string => `<time><beats>${meter.beats}</beats><beat-type>${meter.unit}</beat-type></time>`;

export const toMusicXml = (song: Song): string => {
  const { title, artist } = song.meta;
  // Repeated sections are written out, each pass under its own rehearsal mark.
  const bars = song.sections.flatMap((section) =>
    Array.from({ length: section.repeat ?? 1 }, () =>
      section.bars.map((bar, i) => ({ bar, label: i === 0 ? section.label : undefined })),
    ).flat(),
  );
  let previous: Bar | undefined;
  const measures = bars.map(({ bar, label }, i) => {
    const meter = bar.meter ?? song.meta.meter;
    // ponytail: a lone "%" copies the previous bar's chords; a "%" slot inside a split bar is left without a harmony.
    const isRepeatBar = bar.chords.length === 1 && bar.chords[0].chord === "%";
    const previousMeter = previous && (previous.meter ?? song.meta.meter);
    const chords = isRepeatBar && previous && barBeats(song, previous) === barBeats(song, bar) ? previous.chords : bar.chords;
    previous = { ...bar, chords };
    const lines = [];
    if (previousMeter === undefined) {
      lines.push(`<attributes><divisions>${DIVISIONS}</divisions>${time(meter)}<clef><sign>G</sign><line>2</line></clef></attributes>`);
    } else if (meter.beats !== previousMeter.beats || meter.unit !== previousMeter.unit) {
      lines.push(`<attributes>${time(meter)}</attributes>`);
    }
    if (label !== undefined) {
      lines.push(`<direction placement="above"><direction-type><rehearsal>${escape(label)}</rehearsal></direction-type></direction>`);
    }
    for (const slot of chords) {
      const chord = harmony(slot.chord);
      if (chord !== undefined) lines.push(chord);
      for (let beat = 0; beat < slot.beats; beat++) lines.push(slash(meter.unit));
    }
    return [`    <measure number="${i + 1}">`, ...lines.map((line) => `      ${line}`), "    </measure>"].join("\n");
  });
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">',
    '<score-partwise version="4.0">',
    `  <work><work-title>${escape(title)}</work-title></work>`,
    ...(artist ? [`  <identification><creator type="composer">${escape(artist)}</creator></identification>`] : []),
    '  <part-list><score-part id="P1"><part-name>Chords</part-name></score-part></part-list>',
    '  <part id="P1">',
    ...measures,
    "  </part>",
    "</score-partwise>",
    "",
  ].join("\n");
};
