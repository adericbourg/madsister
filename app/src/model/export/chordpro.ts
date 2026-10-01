// ChordPro 6 grid export (spec F-EX-2): one {start_of_grid} per section, one cell per beat.
import { displayChord } from "../display";
import type { Bar, Song } from "../song";

const BARS_PER_LINE = 4;

// International spelling, except the two names outside ChordPro's built-in extension list.
const chordName = (harte: string): string => displayChord(harte, "intl").replace("m(maj7)", "mmaj7").replace("ø7", "m7b5");

// ponytail: a bar gets as many cells as its own beats, so a meter-override bar is narrower than the grid shape says.
const cells = (bar: Bar): string =>
  bar.chords.flatMap((slot) => [chordName(slot.chord), ...Array<string>(slot.beats - 1).fill(".")]).join(" ");

export const toChordPro = (song: Song): string => {
  const { title, artist, key, tempoBpm, meter } = song.meta;
  const header = [
    `{title: ${title}}`,
    artist && `{artist: ${artist}}`,
    key && `{key: ${key}}`,
    tempoBpm !== undefined && `{tempo: ${Math.round(tempoBpm)}}`,
    `{time: ${meter.beats}/${meter.unit}}`,
  ].filter(Boolean);
  const grids = song.sections.map((section) => {
    const lines = [];
    for (let i = 0; i < section.bars.length; i += BARS_PER_LINE) {
      lines.push(`| ${section.bars.slice(i, i + BARS_PER_LINE).map(cells).join(" | ")} |`);
    }
    // The section ends on a double bar; its repeat count goes in the right margin.
    if (lines.length > 0) lines[lines.length - 1] += `|${section.repeat === undefined ? "" : ` x${section.repeat}`}`;
    return [
      `{start_of_grid shape="1+${BARS_PER_LINE}x${meter.beats}+1" label="${section.label}"}`,
      ...lines,
      "{end_of_grid}",
    ].join("\n");
  });
  return `${[header.join("\n"), ...grids].join("\n\n")}\n`;
};
