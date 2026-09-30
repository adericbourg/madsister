import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { expect, test } from "vitest";
import { emptySong, type Song } from "../model/song";
import { Editor } from "./Editor";
import { useHistory } from "./useHistory";

let song: Song | undefined;
const Harness = () => {
  const [initial] = useState(emptySong);
  const history = useHistory(initial);
  song = history.song;
  return <Editor history={history} barsPerRow={4} style="fr" />;
};

const bar = (...chords: [string, number][]) => ({ chords: chords.map(([chord, beats]) => ({ chord, beats })) });

test("Editor_whenTypingAChartByKeyboard_buildsTheSong", async () => {
  // Given an empty song (Verse, 4 empty bars) with the cursor on the first slot
  const user = userEvent.setup();
  render(<Harness />);
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 1, beat 1: no chord" }));

  // When typing the verse, splitting the last bar 2+2
  await user.keyboard("C{Enter}G{Enter}Am{Enter}/F{Enter}Gx{Enter}");

  // Then the invalid chord is rejected inline and nothing changes
  const input = screen.getByRole("textbox", { name: "Chord" });
  expect(input.getAttribute("aria-invalid")).toBe("true");
  expect(screen.getByRole("alert").textContent).toBe('unknown chord quality "x"');
  expect(input.getAttribute("aria-describedby")).toBe(screen.getByRole("alert").id);
  expect(screen.getByRole("gridcell", { name: "Verse, bar 4, beat 3: no chord" })).toBeDefined();

  // When correcting it, then adding 4 bars, going back to the first new one and splitting the section there
  await user.clear(input);
  await user.keyboard("G{Enter}");
  await user.keyboard("{Control>}{Enter}{Enter}{Enter}{Enter}{/Control}{ArrowLeft}{ArrowLeft}{ArrowLeft}{Control>}k{/Control}");

  // And typing the chorus, undoing and redoing the last chord
  await user.keyboard("F{Enter}C{Enter}G{Tab}G{Enter}{Control>}z{/Control}");
  expect(screen.getByRole("gridcell", { name: "Verse (2), bar 4, beat 1: no chord" })).toBeDefined();
  await user.keyboard("{Control>}{Shift>}z{/Shift}{/Control}");

  // And renaming the section and repeating it twice
  await user.keyboard("{F2}");
  expect((screen.getByRole("textbox", { name: "Section name" }) as HTMLInputElement).value).toBe("Verse (2)");
  await user.keyboard("Chorus{Enter}{Alt>}{ArrowUp}{/Alt}");

  // Then the song is the expected chart
  expect(song).toEqual({
    version: 1,
    meta: { title: "Untitled", meter: { beats: 4, unit: 4 } },
    sections: [
      { id: expect.any(String), label: "Verse", bars: [bar(["C:maj", 4]), bar(["G:maj", 4]), bar(["A:min", 4]), bar(["F:maj", 2], ["G:maj", 2])] },
      { id: expect.any(String), label: "Chorus", repeat: 2, bars: [bar(["F:maj", 4]), bar(["C:maj", 4]), bar(["G:maj", 4]), bar(["G:maj", 4])] },
    ],
  });
  const cursorCell = screen.getByRole("gridcell", { name: "Chorus, bar 4, beat 1: G" });
  expect(document.activeElement).toBe(cursorCell);

  // When opening the help then pressing Escape, Then it closes and focus returns to the cursor cell
  await user.keyboard("?");
  expect(screen.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeDefined();
  await user.keyboard("{Escape}");
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(document.activeElement).toBe(cursorCell);

  // When clicking a cell, Then it gets the cursor
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G" }));
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G" }));
});
