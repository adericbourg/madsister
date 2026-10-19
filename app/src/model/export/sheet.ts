// The printed chart (spec F-EX-1) as spelled text, for pdf.rs to lay out and draw: the GUI and `madsister print` share it.
import { displayChord, displayKey, type DisplayStyle } from "../display";
import { displayNashville, tonicPc, type MinorConvention } from "../nashville";
import type { Song } from "../song";

/** pdf.rs `Sheet`. */
export type Sheet = {
  title: string;
  artist?: string;
  facts: string[];
  font: string;
  compact: boolean;
  barsPerRow: number;
  sections: { label: string; repeat?: number; bars: { meter?: string; slots: { text: string; beats: number }[] }[] }[];
};

export type SheetOptions = { style: DisplayStyle; minorConvention: MinorConvention; font: string; compactPrint: boolean; barsPerRow: number };

export const toSheet = (song: Song, { style, minorConvention, font, compactPrint, barsPerRow }: SheetOptions): Sheet => {
  const { title, artist, key, tempoBpm, notation } = song.meta;
  // As the grid shows it: Nashville degrees of the key's tonic, or chord names.
  const tonic = notation === "nashville" ? tonicPc(key, minorConvention) : undefined;
  const show = (harte: string) => (tonic === undefined ? displayChord(harte, style) : displayNashville(harte, tonic, style));
  return {
    title,
    artist,
    facts: [key !== undefined && `Key ${displayKey(key, style)}`, tempoBpm !== undefined && `♩ = ${tempoBpm}`].filter((f) => f !== false),
    font,
    compact: compactPrint,
    barsPerRow,
    sections: song.sections.map(({ label, repeat, bars }) => ({
      label,
      repeat: repeat !== undefined && repeat > 1 ? repeat : undefined,
      bars: bars.map((bar) => ({
        meter: bar.meter && `${bar.meter.beats}/${bar.meter.unit}`,
        slots: bar.chords.map((slot) => ({ text: show(slot.chord), beats: slot.beats })),
      })),
    })),
  };
};
