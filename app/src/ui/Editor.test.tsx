import { confirm } from "@tauri-apps/plugin-dialog";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { emptySong, type Song } from "../model/song";
import { Editor } from "./Editor";
import { useHistory } from "./useHistory";

vi.mock("@tauri-apps/plugin-dialog", () => ({ confirm: vi.fn() }));

afterEach(cleanup);

let song: Song | undefined;
const Harness = ({ from = emptySong }: { from?: () => Song }) => {
  const [initial] = useState(from);
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

  // When merging its first slot with the previous one (impossible), Then the refusal is announced politely
  await user.keyboard("{Backspace}");
  expect(screen.getByRole("status").textContent).toBe("Not possible here");
  await user.keyboard("{ArrowRight}");
  expect(screen.getByRole("status").textContent).toBe("");
});

test("Editor_whenUsingTheSectionControls_editsTheCursorSection", async () => {
  // Given an empty intro (cursor there) and a verse with a chord
  const user = userEvent.setup();
  render(
    <Harness
      from={() => ({
        version: 1,
        meta: { title: "Song", meter: { beats: 4, unit: 4 } },
        sections: [
          { id: "i", label: "Intro", bars: [bar(["N", 4])] },
          { id: "v", label: "Verse", bars: [bar(["C:maj", 4])] },
        ],
      })}
    />,
  );

  // When renaming the intro, repeating it 3 times and moving it down
  const name = screen.getByRole("textbox", { name: "Name" });
  await user.clear(name);
  await user.type(name, "Outro{Enter}");
  fireEvent.change(screen.getByRole("spinbutton", { name: "Repeat" }), { target: { value: "3" } });
  await user.click(screen.getByRole("button", { name: "Move down" }));

  // Then the order changes and the controls follow the moved section
  expect(song?.sections.map((s) => [s.label, s.repeat])).toEqual([["Verse", undefined], ["Outro", 3]]);
  expect(screen.getByRole("button", { name: "Move down" }).hasAttribute("disabled")).toBe(true);

  // When deleting the outro (no chords), Then it goes without confirmation
  await user.click(screen.getByRole("button", { name: "Delete section" }));
  expect(confirm).not.toHaveBeenCalled();
  expect(song?.sections.map((s) => s.label)).toEqual(["Verse"]);

  // When deleting the verse (it has chords), declining then accepting the confirmation
  vi.mocked(confirm).mockResolvedValueOnce(false);
  await user.click(screen.getByRole("button", { name: "Delete section" }));
  expect(confirm).toHaveBeenCalledTimes(1);
  expect(song?.sections.map((s) => s.label)).toEqual(["Verse"]);
  vi.mocked(confirm).mockResolvedValueOnce(true);
  await user.click(screen.getByRole("button", { name: "Delete section" }));

  // Then only the accepted deletion happens
  await vi.waitFor(() => expect(song?.sections.map((s) => s.label)).toEqual(["Song"]));
});
