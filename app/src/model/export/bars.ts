import { barBeats, type Bar, type Song } from "../song";

/**
 * The bars as played, for score-like exports: repeated sections written out (each pass keeps its section label on its
 * first bar), and a lone "%" replaced by the previous bar's chords when both bars have the same beat count.
 */
export const playedBars = (song: Song): { bar: Bar; label?: string }[] => {
  let previous: Bar | undefined;
  return song.sections
    .flatMap((section) =>
      Array.from({ length: section.repeat ?? 1 }, () =>
        section.bars.map((bar, i) => ({ bar, label: i === 0 ? section.label : undefined })),
      ).flat(),
    )
    .map(({ bar, label }) => {
      // ponytail: a "%" slot inside a split bar is left as is (no harmony, silence in MIDI).
      const isRepeatBar = bar.chords.length === 1 && bar.chords[0].chord === "%";
      const chords = isRepeatBar && previous && barBeats(song, previous) === barBeats(song, bar) ? previous.chords : bar.chords;
      previous = { ...bar, chords };
      return { bar: previous, label };
    });
};
