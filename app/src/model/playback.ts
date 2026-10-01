import type { BarRef } from "./commands";
import type { Song } from "./song";

/**
 * The bar playing at `t` seconds (spec F-PB-1): the one with the latest `startSec` at or before `t`; bars without `startSec`
 * (added by hand) are skipped, null before the first timed bar. A full scan: moved sections leave `startSec` out of order.
 */
export const barAtTime = (song: Song, t: number): BarRef | null => {
  let found: BarRef | null = null;
  let foundSec = -Infinity;
  song.sections.forEach((section, s) =>
    section.bars.forEach(({ startSec }, b) => {
      if (startSec !== undefined && startSec <= t && startSec > foundSec) {
        found = { section: s, bar: b };
        foundSec = startSec;
      }
    }),
  );
  return found;
};
