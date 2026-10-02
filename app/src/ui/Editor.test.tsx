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
const Harness = ({ from = emptySong, mode }: { from?: () => Song; mode?: "view" | "edit" }) => {
  const [initial] = useState(from);
  const history = useHistory(initial);
  song = history.song;
  return <Editor history={history} mode={mode} barsPerRow={4} style="fr" minorConvention="relative" lowConfidenceThreshold={0.5} audioPath={initial.audio?.path} />;
};

const bar = (...chords: [string, number][]) => ({ chords: chords.map(([chord, beats]) => ({ chord, beats })) });

test("Editor_inViewMode_navigatesWithoutEditing", async () => {
  // Given a song starting with C then G, in view mode
  const user = userEvent.setup();
  render(<Harness mode="view" from={() => ({ ...emptySong(), sections: [{ id: "v", label: "Verse", bars: [bar(["C:maj", 4]), bar(["G:maj", 4])] }] })} />);
  const before = song;
  expect(screen.queryByRole("group", { name: "Section" })).toBeNull();

  // When clicking a bar, Then the cursor moves there and no input opens
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G" }));
  expect(screen.queryByRole("textbox", { name: "Chord" })).toBeNull();
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G" }));

  // When typing, using an edit shortcut or undoing, Then the song is untouched and the refusal is announced
  await user.keyboard("A");
  expect(screen.queryByRole("textbox", { name: "Chord" })).toBeNull();
  await user.keyboard("{Control>}d{/Control}");
  await user.keyboard("{Control>}z{/Control}");
  expect(song).toBe(before);
  expect(screen.getByRole("status").textContent).toMatch(/^Read-only/);

  // When moving with the arrows, Then navigation still works
  await user.keyboard("{ArrowLeft}");
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 1, beat 1: C" }));
});

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

  // When clicking a cell and cancelling its edition, Then it keeps the cursor
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G" }));
  await user.keyboard("{Escape}");
  expect(document.activeElement).toBe(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G" }));

  // When merging its first slot with the previous one (impossible), Then the refusal is announced politely
  await user.keyboard("{Backspace}");
  expect(screen.getByRole("status").textContent).toBe("Not possible here");
  await user.keyboard("{ArrowRight}");
  expect(screen.getByRole("status").textContent).toBe("");
});

test("Editor_whenClickingASlot_editsItInPlaceUntilEnterBlurOrEscape", async () => {
  // Given a song starting with C then G
  const user = userEvent.setup();
  render(<Harness />);
  await user.keyboard("C{Enter}G{Enter}");

  // When clicking the first slot, Then it is edited in place with its whole text selected
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 1, beat 1: C" }));
  const input = screen.getByRole("textbox", { name: "Chord" }) as HTMLInputElement;
  expect(input.value).toBe("C");
  expect([input.selectionStart, input.selectionEnd]).toEqual([0, 1]);
  expect(input.closest(".slot")?.classList.contains("is-editing")).toBe(true);

  // When clicking inside the input, Then the same input stays open
  await user.click(input);
  expect(screen.getByRole("textbox", { name: "Chord" })).toBe(input);

  // When replacing the text and clicking another slot, Then it is saved and the other slot is edited
  await user.clear(input);
  await user.keyboard("D");
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G" }));
  expect(song?.sections[0].bars[0].chords[0].chord).toBe("D:maj");
  expect((screen.getByRole("textbox", { name: "Chord" }) as HTMLInputElement).value).toBe("G");

  // When typing then pressing Escape, Then nothing is saved
  await user.clear(screen.getByRole("textbox", { name: "Chord" }));
  await user.keyboard("A{Escape}");
  expect(screen.queryByRole("textbox", { name: "Chord" })).toBeNull();
  expect(song?.sections[0].bars[1].chords[0].chord).toBe("G:maj");

  // When typing then clicking outside the grid, Then it is saved
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: G" }));
  await user.clear(screen.getByRole("textbox", { name: "Chord" }));
  await user.keyboard("Em");
  await user.click(document.body);
  expect(screen.queryByRole("textbox", { name: "Chord" })).toBeNull();
  expect(song?.sections[0].bars[1].chords[0].chord).toBe("E:min");

  // When leaving a slot unchanged, Then the song is untouched (no undo entry)
  const before = song;
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 1, beat 1: D" }));
  await user.click(document.body);
  expect(song).toBe(before);

  // When leaving with an invalid chord, Then the edition is dropped
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 1, beat 1: D" }));
  await user.clear(screen.getByRole("textbox", { name: "Chord" }));
  await user.keyboard("Gx");
  await user.click(document.body);
  expect(screen.queryByRole("textbox", { name: "Chord" })).toBeNull();
  expect(song).toBe(before);
});

test("Editor_whenPressingModSlashInTheChordInput_setsTheChordAndSplitsTheSlot", async () => {
  // Given a typed C in the first slot of an empty song
  const user = userEvent.setup();
  render(<Harness />);
  await user.keyboard("C");

  // When pressing Ctrl+/ and typing G, Then the slot is split 2+2 and the new half is edited with the chord preselected
  await user.keyboard("{Control>}/{/Control}");
  const input = screen.getByRole("textbox", { name: "Chord" }) as HTMLInputElement;
  expect(input.value).toBe("C");
  expect([input.selectionStart, input.selectionEnd]).toEqual([0, 1]);
  await user.keyboard("G{Enter}");
  expect(song?.sections[0].bars[0]).toEqual(bar(["C:maj", 2], ["G:maj", 2]));

  // When undoing, Then C and the split are one step (the split copies C to both halves), after the last G
  await user.keyboard("{Control>}z{/Control}");
  expect(song?.sections[0].bars[0]).toEqual(bar(["C:maj", 2], ["C:maj", 2]));
  await user.keyboard("{Control>}z{/Control}");
  expect(song?.sections[0].bars[0]).toEqual(bar(["N", 4]));

  // When the chord is invalid, Then the error is shown and nothing is split
  await user.keyboard("Gx{Control>}/{/Control}");
  expect(screen.getByRole("alert").textContent).toBe('unknown chord quality "x"');
  expect(song?.sections[0].bars[0]).toEqual(bar(["N", 4]));

  // When the slot is a single beat, Then the split is refused and the input stays open
  await user.clear(screen.getByRole("textbox", { name: "Chord" }));
  await user.keyboard("C{Control>}/{/Control}A{Control>}/{/Control}"); // C 2 | A 1 | A 1, editing the last A
  await user.keyboard("{Control>}/{/Control}");
  expect(screen.getByRole("textbox", { name: "Chord" })).toBeDefined();
  expect(screen.getByRole("status").textContent).toBe("Not possible here");
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

test("Editor_whenShiftingThePhase_movesTheBarLinesInOneUndoableStep", async () => {
  // Given a song whose chords change on beat 2
  const user = userEvent.setup();
  const original: Song = {
    version: 1,
    meta: { title: "Song", meter: { beats: 4, unit: 4 } },
    sections: [{ id: "v", label: "Verse", bars: [bar(["A:min", 1], ["C:maj", 3]), bar(["C:maj", 1], ["G:maj", 3]), bar(["G:maj", 4])] }],
  };
  render(<Harness from={() => original} />);
  const chords = () => song?.sections[0].bars.map((b) => b.chords.map((c) => `${c.chord}:${c.beats}`).join(" "));

  // When moving the whole song's bar lines one beat later, Then each chord starts a bar
  await user.click(screen.getByRole("button", { name: "Whole song +1 beat" }));
  expect(chords()).toEqual(["A:min:1", "C:maj:4", "G:maj:4", "G:maj:3"]);

  // When undoing, Then the song is back in one step
  screen.getByRole("gridcell", { name: /^Verse, bar 1 in 1\/4/ }).focus();
  await user.keyboard("{Control>}z{/Control}");
  expect(song).toBe(original);

  // When moving the bar lines one beat earlier from bar 2, Then bar 1 is untouched
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 2, beat 1: C" }));
  await user.click(screen.getByRole("button", { name: "From this bar −1 beat" }));
  expect(chords()).toEqual(["A:min:1 C:maj:3", "C:maj:1 G:maj:2", "G:maj:4", "G:maj:1"]);

  // When a bar inside the range has its own meter, Then the shift is refused and says why
  await user.click(screen.getByRole("button", { name: "Whole song +1 beat" }));
  expect(screen.getByRole("status").textContent).toBe("the phase can't be shifted across a bar with its own meter");
});

test("Editor_whenFixingTheTempo_mergesOrSplitsBarsInOneUndoableStep", async () => {
  // Given a 120 BPM song of 4 bars
  const user = userEvent.setup();
  const original: Song = {
    version: 1,
    meta: { title: "Song", tempoBpm: 120, meter: { beats: 4, unit: 4 } },
    sections: [{ id: "v", label: "Verse", bars: [bar(["A:min", 4]), bar(["C:maj", 4]), bar(["G:maj", 4]), bar(["F:maj", 4])] }],
  };
  render(<Harness from={() => original} />);
  const chords = () => song?.sections[0].bars.map((b) => b.chords.map((c) => `${c.chord}:${c.beats}`).join(" "));

  // When halving the whole song's tempo, Then bars are merged two by two at 60 BPM
  await user.click(screen.getByRole("button", { name: "Whole song half tempo" }));
  expect(chords()).toEqual(["A:min:2 C:maj:2", "G:maj:2 F:maj:2"]);
  expect(song?.meta.tempoBpm).toBe(60);

  // When undoing, Then the song is back in one step
  screen.getByRole("gridcell", { name: /^Verse, bar 1, beat 1/ }).focus();
  await user.keyboard("{Control>}z{/Control}");
  expect(song).toBe(original);

  // When doubling the tempo from bar 4, Then only bar 4 is split
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 4, beat 1: F" }));
  await user.click(screen.getByRole("button", { name: "From this bar double tempo" }));
  expect(chords()).toEqual(["A:min:4", "C:maj:4", "G:maj:4", "F:maj:4", "F:maj:4"]);
  expect(song?.meta.tempoBpm).toBe(120);
  expect(screen.getByRole("button", { name: "Whole song double tempo" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "From this bar half tempo" })).toBeTruthy();
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
        sections: [{ id: "v", label: "Verse", bars: [timed(0), timed(2), timed(4.37), timed(), timed(6)] }],
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
  expect(currentTime).toBe(4.37);
  expect(barOf(3).classList).toContain("is-playing");
  expect(screen.getByRole("button", { name: "Play" })).toBeDefined();

  // When clicking the manual bar 4, Then only the edit cursor moves; Shift+Space there doesn't play
  await user.click(screen.getByRole("gridcell", { name: "Verse, bar 4, beat 1: C" }));
  await user.keyboard("{Escape}{Shift>} {/Shift}");
  expect(currentTime).toBe(4.37);
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

test("Editor_whenPickingASectionByName_editsThatSection", async () => {
  // Given a song with an intro (cursor there) and a verse
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

  // When picking the verse in the panel and renaming it
  await user.selectOptions(screen.getByRole("combobox", { name: "Section" }), "Verse");
  const name = screen.getByRole("textbox", { name: "Name" });
  expect((name as HTMLInputElement).value).toBe("Verse");
  await user.clear(name);
  await user.type(name, "Chorus{Enter}");

  // Then only the verse changed
  expect(song?.sections.map((s) => s.label)).toEqual(["Intro", "Chorus"]);

  // When clicking the intro's name in the grid, Then the panel follows
  await user.click(screen.getByText("Intro", { selector: ".section-label" }));
  expect((screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement).value).toBe("Intro");
});
