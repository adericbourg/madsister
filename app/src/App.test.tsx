import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { exists, readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import App from "./App";
import { serializeSong, type Song } from "./model/song";
import { transcribe } from "./ui/engine";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn(), confirm: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({ readTextFile: vi.fn(), writeTextFile: vi.fn(), mkdir: vi.fn(), exists: vi.fn() }));
vi.mock("./ui/engine", () => ({ transcribe: vi.fn(), cancel: vi.fn() }));
const drop = vi.hoisted(() => ({ handler: (_: { payload: unknown }) => {} }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: async (handler: typeof drop.handler) => {
      drop.handler = handler;
      return () => {};
    },
  }),
}));
const setTitle = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ setTitle }) }));
vi.mock("@tauri-apps/api/path", () => ({ appConfigDir: async () => "/config", join: async (...p: string[]) => p.join("/") }));

const song: Song = {
  version: 1,
  meta: { title: "Blues", meter: { beats: 4, unit: 4 } },
  sections: [{ id: "a", label: "Chorus", bars: [{ chords: [{ chord: "C:7", beats: 4 }] }] }],
};
const files: Record<string, string> = {};

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
  for (const k of Object.keys(files)) delete files[k];
  vi.mocked(readTextFile).mockImplementation(async (path) => {
    const text = files[String(path)];
    if (text === undefined) throw new Error(`not found: ${String(path)}`);
    return text;
  });
  vi.mocked(writeTextFile).mockImplementation(async (path, text) => {
    files[String(path)] = String(text);
  });
  vi.mocked(confirm).mockResolvedValue(true);
  vi.mocked(exists).mockResolvedValue(false);
  vi.mocked(transcribe).mockResolvedValue(42);
});

test("App_rendersAnEmptySongGrid", () => {
  // Given / When
  render(<App />);

  // Then
  expect(screen.getByRole("heading", { name: "Untitled" })).toBeDefined();
  expect(screen.getAllByRole("gridcell", { name: /^Verse, bar \d, beat 1: no chord$/ })).toHaveLength(4);
});

test("App_whenOpeningFiles_loadsValidOnesAndReportsInvalidOnes", async () => {
  // Given a valid and an invalid song file
  const user = userEvent.setup();
  files["/blues.madsister.json"] = serializeSong(song);
  files["/bad.madsister.json"] = '{"version": 1}';
  render(<App />);

  // When opening the valid file with Mod+O
  vi.mocked(open).mockResolvedValueOnce("/blues.madsister.json");
  await user.keyboard("{Control>}o{/Control}");

  // Then the song is loaded and the file is listed as recent
  expect(await screen.findByRole("heading", { name: "Blues" })).toBeDefined();
  expect(screen.getByRole("button", { name: "/blues.madsister.json" })).toBeDefined();
  expect(files["/config/recent.json"]).toBe('["/blues.madsister.json"]');

  // When opening the invalid file from the toolbar
  vi.mocked(open).mockResolvedValueOnce("/bad.madsister.json");
  await user.click(screen.getByRole("button", { name: "Open…" }));

  // Then the error is shown and the current song is kept
  expect((await screen.findByRole("alert")).textContent).toMatch(/meta/);
  expect(screen.getByRole("heading", { name: "Blues" })).toBeDefined();

  // When the recent file has gone missing and is reopened
  delete files["/blues.madsister.json"];
  await user.click(screen.getByRole("button", { name: "/blues.madsister.json" }));

  // Then it is dropped from the recent list
  await vi.waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/not found/));
  expect(screen.queryByRole("button", { name: "/blues.madsister.json" })).toBeNull();

  // When starting a new song with Mod+N
  await user.keyboard("{Control>}n{/Control}");

  // Then an empty song replaces the loaded one
  expect(await screen.findByRole("heading", { name: "Untitled" })).toBeDefined();
});

test("App_whenImportingAudio_loadsTheResultAndKeepsItOnError", async () => {
  // Given the app, and the song the engine will write
  render(<App />);
  files["/music/blues.madsister.json"] = serializeSong(song);
  const engineSays = (event: Parameters<Parameters<typeof transcribe>[4]>[0]) =>
    act(() => vi.mocked(transcribe).mock.lastCall![4](event));

  // When dropping the mp3 and the engine reports its result
  await act(() => drop.handler({ payload: { type: "drop", paths: ["/music/blues.mp3"] } }));
  await engineSays({ type: "result", path: "/music/blues.madsister.json" });

  // Then the song is loaded and listed as recent
  expect(await screen.findByRole("heading", { name: "Blues" })).toBeDefined();
  expect(screen.getByRole("button", { name: "/music/blues.madsister.json" })).toBeDefined();

  // When a second import fails
  await act(() => drop.handler({ payload: { type: "drop", paths: ["/music/other.mp3"] } }));
  await engineSays({ type: "error", message: "boom", stderr: "" });

  // Then the error is shown and the song is kept
  expect(screen.getByRole("alert").textContent).toContain("boom");
  expect(screen.getByRole("heading", { name: "Blues" })).toBeDefined();
});

test("App_whenSaving_writesTheSerializedSongThenAutosavesEdits", async () => {
  // Given a new song
  vi.useFakeTimers({ shouldAdvanceTime: true });
  const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
  render(<App />);
  expect(setTitle).toHaveBeenLastCalledWith("Untitled — madsister");

  // When saving it with Mod+S (no path yet: Save As)
  vi.mocked(save).mockResolvedValueOnce("/untitled.madsister.json");
  await user.keyboard("{Control>}s{/Control}");

  // Then Save As proposes a name from the title and writes the song there
  expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "Untitled.madsister.json" }));
  await vi.waitFor(() => expect(files["/untitled.madsister.json"]).toBeDefined());
  expect(JSON.parse(files["/untitled.madsister.json"])).toMatchObject({ meta: { title: "Untitled" } });

  // When editing the song, then waiting 2 s
  await user.keyboard("C{Enter}");
  expect(setTitle).toHaveBeenLastCalledWith("• Untitled — madsister");
  await act(() => vi.advanceTimersByTimeAsync(2000));

  // Then the edit is autosaved
  expect(files["/untitled.madsister.json"]).toContain('"C:maj"');
  expect(setTitle).toHaveBeenLastCalledWith("Untitled — madsister");
  vi.useRealTimers();
});

test("App_whenUsingTheToolbar_restylesTransposesEditsMetadataAndPersistsSettings", async () => {
  // Given saved settings (international style, 2 bars per row) and Cmaj7 typed in the first slot
  const user = userEvent.setup();
  files["/config/settings.json"] = '{"style":"intl","barsPerRow":2}';
  render(<App />);
  await vi.waitFor(() => expect(screen.getAllByRole("row")).toHaveLength(2));
  await user.keyboard("Cmaj7{Enter}");
  const firstSlot = () => screen.getByRole("gridcell", { name: /^Verse, bar 1, beat 1/ });
  expect(firstSlot().textContent).toBe("Cmaj7");

  // When switching to the French style, Then the chord is re-rendered and the setting saved
  await user.selectOptions(screen.getByRole("combobox", { name: "Chord style" }), "fr");
  expect(firstSlot().textContent).toBe("C7M");
  expect(JSON.parse(files["/config/settings.json"])).toEqual({ style: "fr", barsPerRow: 2, lowConfidenceThreshold: 0.5 });

  // When raising the low-confidence threshold, Then the setting is saved; an out-of-range value is ignored
  const threshold = screen.getByRole("spinbutton", { name: "Review chords below confidence" });
  fireEvent.change(threshold, { target: { value: "0.7" } });
  fireEvent.change(threshold, { target: { value: "2" } });
  await vi.waitFor(() => expect(JSON.parse(files["/config/settings.json"])).toEqual({ style: "fr", barsPerRow: 2, lowConfidenceThreshold: 0.7 }));
  expect((threshold as HTMLInputElement).value).toBe("0.7");

  // When transposing up twice with flats, then undoing once
  await user.selectOptions(screen.getByRole("combobox", { name: "Spelling" }), "flat");
  await user.click(screen.getByRole("button", { name: "+1 semitone" }));
  await user.click(screen.getByRole("button", { name: "+1 semitone" }));
  expect(firstSlot().textContent).toBe("D7M");
  expect(document.activeElement).toBe(screen.getByRole("button", { name: "+1 semitone" }));
  await user.click(firstSlot());
  await user.keyboard("{Control>}z{/Control}");

  // Then each click is one history entry
  expect(firstSlot().textContent).toBe("Db7M");

  // When filling the metadata, with an invalid key first
  const key = screen.getByRole("textbox", { name: "Key" });
  await user.type(key, "H{Enter}");
  expect(screen.getByRole("alert").textContent).toMatch(/note/);
  expect(key.getAttribute("aria-invalid")).toBe("true");
  await user.clear(key);
  await user.type(key, "Bbm{Enter}");
  const title = screen.getByRole("textbox", { name: "Title" });
  await user.clear(title);
  await user.type(title, "Blues{Tab}");
  await user.type(screen.getByRole("spinbutton", { name: "Tempo (BPM)" }), "96{Enter}");

  // Then the chart header shows them and the error is gone
  expect(screen.getByRole("heading", { name: "Blues" })).toBeDefined();
  expect(screen.getByText("Bbm")).toBeDefined();
  expect(screen.getByText("♩ = 96")).toBeDefined();
  expect(screen.queryByRole("alert")).toBeNull();

  // When starting a new 3/4 song and saving it
  await user.selectOptions(screen.getByRole("combobox", { name: "New song meter" }), "3/4");
  await user.click(screen.getByRole("button", { name: "New" }));
  vi.mocked(save).mockResolvedValueOnce("/waltz.madsister.json");
  await user.click(screen.getByRole("button", { name: "Save as…" }));

  // Then the new song has that meter
  await vi.waitFor(() => expect(files["/waltz.madsister.json"]).toBeDefined());
  expect(JSON.parse(files["/waltz.madsister.json"]).meta.meter).toEqual({ beats: 3, unit: 4 });
});

test("App_whenPrintingOrExporting_printsFromTheButtonAndModPAndWritesChordPro", async () => {
  // Given the app
  const user = userEvent.setup();
  const print = vi.spyOn(window, "print").mockImplementation(() => undefined);
  render(<App />);

  // When clicking Print, then pressing Mod+P from the grid
  await user.click(screen.getByRole("button", { name: "Print…" }));
  await user.click(screen.getByRole("gridcell", { name: /^Verse, bar 1, beat 1/ }));
  await user.keyboard("{Meta>}p{/Meta}");

  // Then the chart is printed twice
  expect(print).toHaveBeenCalledTimes(2);
  print.mockRestore();

  // When exporting ChordPro to a picked path
  vi.mocked(save).mockResolvedValueOnce("/out/untitled.cho");
  await user.click(screen.getByRole("button", { name: "ChordPro…" }));

  // Then the dialog proposes a .cho named after the title and the grid is written there
  expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "Untitled.cho" }));
  await vi.waitFor(() => expect(files["/out/untitled.cho"]).toContain('{start_of_grid shape="1+4x4+1" label="Verse"}'));
});

test("App_whenTabbingThroughTheControls_reachesEveryEnabledControl", async () => {
  // Given the app, with a recent file so that every control is present
  const user = userEvent.setup();
  files["/config/recent.json"] = '["/blues.madsister.json"]';
  render(<App />);
  await screen.findByText("Recent");

  // When tabbing forward past the end
  const reached = new Set<Element | null>();
  for (let i = 0; i < 40; i++) {
    await user.tab();
    reached.add(document.activeElement);
  }

  // Then every enabled control and the grid's single tab stop were focused
  const controls = document.querySelectorAll('button:not([disabled]), input, select, summary, [tabindex="0"]');
  expect(controls.length).toBeGreaterThan(15);
  for (const control of controls) expect(reached).toContain(control);
});
