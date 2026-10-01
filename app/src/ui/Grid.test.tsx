import { render, screen, within } from "@testing-library/react";
import { expect, test } from "vitest";
import type { Song } from "../model/song";
import { Grid } from "./Grid";

const song: Song = {
  version: 1,
  meta: { title: "Blue Bossa", artist: "Kenny Dorham", key: "C:min", tempoBpm: 120, meter: { beats: 4, unit: 4 } },
  sections: [
    {
      id: "a",
      label: "Verse",
      bars: [
        { chords: [{ chord: "C:min7", beats: 4 }] },
        { chords: [{ chord: "F:min7", beats: 3 }, { chord: "G:7", beats: 1, confidence: 0.3 }] },
        { chords: [{ chord: "A:min7", beats: 4, confidence: 0.9 }] },
        { meter: { beats: 2, unit: 4 }, chords: [{ chord: "D:7", beats: 2 }] },
      ],
    },
    { id: "b", label: "Chorus", repeat: 2, bars: [{ chords: [{ chord: "N", beats: 4 }] }] },
    { id: "c", label: "Bridge", bars: [] },
  ],
};

test("Grid_ofSong_rendersSectionedChart", () => {
  // Given a 3-section song, 2 bars per row, cursor on the 3+1 bar's second slot
  render(<Grid song={song} barsPerRow={2} style="fr" cursor={{ section: 0, bar: 1, slot: 1 }} />);

  // Then the header, section labels, repeat and empty placeholder are shown
  expect(screen.getByRole("heading", { name: "Blue Bossa" })).toBeDefined();
  expect(screen.getByText("Kenny Dorham")).toBeDefined();
  expect(screen.getByText("Cm")).toBeDefined();
  expect(screen.getByText("♩ = 120")).toBeDefined();
  expect(screen.getByText("Verse")).toBeDefined();
  expect(screen.getByText("x2")).toBeDefined();
  expect(screen.getByText("No bars yet")).toBeDefined();

  // And a bar with a meter override shows it, also to screen readers
  expect(screen.getByText("2/4")).toBeDefined();
  expect(screen.getByRole("gridcell", { name: "Verse, bar 4 in 2/4, beat 1: D 7" })).toBeDefined();

  // And bars are laid out in rows of barsPerRow: Verse takes 2 rows
  const verse = screen.getByRole("rowgroup", { name: "Verse" });
  expect(within(verse).getAllByRole("row")).toHaveLength(2);
  expect(screen.getByRole("rowgroup", { name: "Chorus, 2 times" })).toBeDefined();

  // And there is one cell per slot, named for screen readers
  const cells = within(screen.getByRole("grid")).getAllByRole("gridcell", { name: /, bar \d/ });
  expect(cells).toHaveLength(6);
  const f = screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: F minor 7" });
  const g = screen.getByRole("gridcell", { name: "Verse, bar 2, beat 4: G 7, low confidence" });
  expect(screen.getByRole("gridcell", { name: "Chorus, bar 1, beat 1: no chord" })).toBeDefined();

  // And a 3+1 bar splits by beats
  expect(f.style.flexGrow).toBe("3");
  expect(g.style.flexGrow).toBe("1");

  // And only the low-confidence slot carries the flag
  expect(g.classList.contains("is-low-confidence")).toBe(true);
  expect(screen.getByRole("gridcell", { name: "Verse, bar 3, beat 1: A minor 7" }).classList.contains("is-low-confidence")).toBe(false);

  // And the cursor cell is the only tab stop
  expect(cells.filter((cell) => cell.tabIndex === 0)).toEqual([g]);
  expect(cells.filter((cell) => cell.tabIndex === -1)).toHaveLength(5);
});
