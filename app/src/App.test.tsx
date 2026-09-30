import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import App from "./App";
import { serializeSong, type Song } from "./model/song";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn(), confirm: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({ readTextFile: vi.fn(), writeTextFile: vi.fn(), mkdir: vi.fn() }));
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
