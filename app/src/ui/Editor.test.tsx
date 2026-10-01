import { invoke } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { emptySong, type Song } from "../model/song";
import { Editor } from "./Editor";
import { useHistory } from "./useHistory";

vi.mock("@tauri-apps/plugin-dialog", () => ({ confirm: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

afterEach(cleanup);

let song: Song | undefined;
const Harness = ({ from = emptySong }: { from?: () => Song }) => {
  const [initial] = useState(from);
  const history = useHistory(initial);
  song = history.song;
  return <Editor history={history} barsPerRow={4} style="fr" lowConfidenceThreshold={0.5} />;
};

const bar = (...chords: [string, number][]) => ({ chords: chords.map(([chord, beats]) => ({ chord, beats })) });

test("Editor_whenTypingAChartByKeyboard_buildsTheSong", async () => {
  // Given an empty song (Verse, 4 empty bars) with the cursor on the first slot
  const user = userEvent.setup();
  render(<Harness />);
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 1, beat 1: no chord" }));
  expect(screen.queryByRole("group", { name: "Playback" })).toBeNull(); // no audio, no transport

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

test("Editor_whenReviewingLowConfidenceChords_jumpsToThemAndCountsDown", async () => {
  // Given a transcription with two low-confidence chords (bars 2 and 4) and the cursor on bar 1
  const user = userEvent.setup();
  const flagged = (chord: string, confidence: number) => ({ chords: [{ chord, beats: 4, confidence }] });
  render(
    <Harness
      from={() => ({
        version: 1,
        meta: { title: "Song", meter: { beats: 4, unit: 4 } },
        sections: [{ id: "v", label: "Verse", bars: [flagged("C:maj", 0.9), flagged("G:maj", 0.3), bar(["A:min", 4]), flagged("F:maj", 0.1)] }],
      })}
    />,
  );
  const review = screen.getByText("2 chords to review");
  expect(review.getAttribute("aria-live")).toBe("polite");

  // When pressing F8, Then the cursor jumps to the first flagged chord
  await user.keyboard("{F8}");
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G, low confidence" }));

  // When confirming it by typing the same chord, Then its flag is cleared and the counter updates
  await user.keyboard("G{Enter}");
  expect(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G" })).toBeDefined();
  expect(review.textContent).toBe("1 chord to review");

  // When pressing Shift+F8 from bar 3, Then it goes back to the previous flagged chord, wrapping around
  await user.keyboard("{Shift>}{F8}{/Shift}");
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 4, beat 1: F, low confidence" }));

  // When correcting it, Then nothing is left to review and F8 is refused
  await user.keyboard("Fmaj7{Enter}");
  expect(review.textContent).toBe("No chords to review");
  await user.keyboard("{F8}");
  expect(screen.getByRole("status").textContent).toBe("Not possible here");
});

test("Editor_whenSettingTheBarMeter_overridesTheCursorBar", async () => {
  // Given an empty 4/4 song with the cursor on bar 1
  const user = userEvent.setup();
  render(<Harness />);
  const beats = screen.getByRole("spinbutton", { name: "Beats in this bar" });
  expect((beats as HTMLInputElement).value).toBe("4");

  // When pressing Mod+B, Then the bar becomes a 2/4 break bar
  await user.keyboard("{Control>}b{/Control}");
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 1 in 2/4, beat 1: no chord" }));
  expect((beats as HTMLInputElement).value).toBe("2");

  // When typing 3 then 4 beats in the bar control, Then the bar goes 3/4, then back to the song meter
  fireEvent.change(beats, { target: { value: "3" } });
  expect(song?.sections[0].bars[0]).toEqual({ meter: { beats: 3, unit: 4 }, chords: [{ chord: "N", beats: 3 }] });
  fireEvent.change(beats, { target: { value: "4" } });
  expect(song?.sections[0].bars[0]).toEqual({ chords: [{ chord: "N", beats: 4 }] });
});

test("Editor_whenPlayingTheAudio_highlightsTheBarBeingPlayed", async () => {
  // Given a transcribed song (bars at 0, 2, 4, 6 s plus a manually added bar) and a fake media element (jsdom has none)
  const user = userEvent.setup();
  let currentTime = 0;
  vi.spyOn(HTMLMediaElement.prototype, "currentTime", "get").mockImplementation(() => currentTime);
  vi.spyOn(HTMLMediaElement.prototype, "currentTime", "set").mockImplementation((t) => (currentTime = t));
  vi.spyOn(HTMLMediaElement.prototype, "play").mockImplementation(function (this: HTMLMediaElement) {
    this.dispatchEvent(new Event("play"));
    return Promise.resolve();
  });
  vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(function (this: HTMLMediaElement) {
    this.dispatchEvent(new Event("pause"));
  });
  URL.createObjectURL = vi.fn(() => "blob:song");
  URL.revokeObjectURL = vi.fn();
  vi.mocked(invoke).mockResolvedValue(new ArrayBuffer(8));
  const timed = (startSec?: number) => ({ startSec, chords: [{ chord: "C:maj", beats: 4 }] });
  render(
    <Harness
      from={() => ({
        version: 1,
        meta: { title: "Song", meter: { beats: 4, unit: 4 } },
        audio: { path: "/music/song.mp3", sha256: "x" },
        sections: [{ id: "v", label: "Verse", bars: [timed(0), timed(2), timed(4), timed(), timed(6)] }],
      })}
    />,
  );
  const barOf = (n: number) => screen.getByRole("gridcell", { name: `Verse, bar ${n}, beat 1: C` }).parentElement!;
  const transport = screen.getByRole("group", { name: "Playback" }) as HTMLFieldSetElement;
  await vi.waitFor(() => expect(transport.disabled).toBe(false));
  expect(invoke).toHaveBeenCalledWith("read_audio", { path: "/music/song.mp3" });

  // When pressing Play with the audio at 2.5 s
  currentTime = 2.5;
  await user.click(screen.getByRole("button", { name: "Play" }));

  // Then only bar 2 is highlighted and the time shows
  await vi.waitFor(() => expect(barOf(2).classList).toContain("is-playing"));
  expect(document.querySelectorAll(".is-playing")).toHaveLength(1);
  expect(transport.textContent).toContain("0:02");

  // When the audio reaches 6.1 s, Then the cursor skips the manual bar 4 and goes to bar 5
  currentTime = 6.1;
  await vi.waitFor(() => expect(barOf(5).classList).toContain("is-playing"));
  expect(barOf(4).classList).not.toContain("is-playing");

  // When pressing Space in the grid, Then it pauses and the edit cursor stays on bar 1
  barOf(1).querySelector<HTMLElement>('[tabindex="0"]')!.focus();
  await user.keyboard(" ");
  expect(screen.getByRole("button", { name: "Play" })).toBeDefined();
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 1, beat 1: C" }));

  // When the webview can't play it (e.g. missing codecs), Then it says so
  vi.mocked(HTMLMediaElement.prototype.play).mockRejectedValueOnce(new DOMException("The operation is not supported.", "NotSupportedError"));
  await user.keyboard(" ");
  expect((await screen.findByRole("alert")).textContent).toBe("Can't play this audio: NotSupportedError: The operation is not supported.");

  // When clicking bar 3 while paused, Then the audio seeks to its start and the highlight moves, without playing
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 3, beat 1: C" }));
  expect(currentTime).toBe(4);
  expect(barOf(3).classList).toContain("is-playing");
  expect(screen.getByRole("button", { name: "Play" })).toBeDefined();

  // When clicking the manual bar 4, Then only the edit cursor moves; Shift+Space there doesn't play
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 4, beat 1: C" }));
  await user.keyboard("{Shift>} {/Shift}");
  expect(currentTime).toBe(4);
  expect(screen.getByRole("button", { name: "Play" })).toBeDefined();

  // When pressing Shift+Space on bar 5, Then it plays from that bar
  await user.keyboard("{ArrowRight}{Shift>} {/Shift}");
  expect(currentTime).toBe(6);
  expect(screen.getByRole("button", { name: "Pause" })).toBeDefined();

  // When clicking bar 1 while playing, Then it keeps playing from there
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 1, beat 1: C" }));
  expect(currentTime).toBe(0);
  expect(screen.getByRole("button", { name: "Pause" })).toBeDefined();
});
