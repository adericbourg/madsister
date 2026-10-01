import { confirm, open } from "@tauri-apps/plugin-dialog";
import { exists } from "@tauri-apps/plugin-fs";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cancel, transcribe, type EngineEvent } from "./engine";
import { Importer } from "./Importer";

vi.mock("./engine", () => ({ transcribe: vi.fn(), cancel: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), confirm: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({ exists: vi.fn() }));
const drop = vi.hoisted(() => ({ handler: (_: { payload: unknown }) => {} }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: async (handler: typeof drop.handler) => {
      drop.handler = handler;
      return () => {};
    },
  }),
}));

const onResult = vi.fn();
const dropFile = (path: string) => act(() => drop.handler({ payload: { type: "drop", paths: [path] } }));
/** The engine callback of the last `transcribe` call. */
const emit = (event: EngineEvent) => act(() => vi.mocked(transcribe).mock.lastCall![4](event));

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(transcribe).mockResolvedValue(42);
  vi.mocked(exists).mockResolvedValue(false);
});

test("Importer_whenDroppingAnAudioFile_showsProgressThenReportsTheResult", async () => {
  // Given the importer with "Detect sections" checked and the meter forced to 6/8
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);
  await user.click(screen.getByRole("checkbox", { name: /Detect sections \(slow/ }));
  await user.selectOptions(screen.getByRole("combobox", { name: "Meter" }), "6/8");

  // When dropping an mp3
  await dropFile("/music/song.mp3");

  // Then the engine starts on it, writing next to it, with that meter and all-in-one
  expect(transcribe).toHaveBeenCalledWith("/music/song.mp3", "/music/song.madsister.json", "6/8", true, expect.any(Function));
  expect(screen.getByRole("status").textContent).toBe("Starting…");
  expect(screen.getByRole("button", { name: "Import audio…" })).toHaveProperty("disabled", true);

  // When the engine reports progress, Then the stage and the bar follow
  await emit({ type: "progress", stage: "chords", pct: 60 });
  expect(screen.getByRole("status").textContent).toBe("chords");
  expect(screen.getByRole("progressbar", { name: "chords" })).toHaveProperty("value", 60);

  // When the engine reports its result, Then it is handed over and the panel goes away
  await emit({ type: "result", path: "/music/song.madsister.json" });
  expect(onResult).toHaveBeenCalledWith("/music/song.madsister.json");
  expect(screen.queryByRole("progressbar")).toBeNull();
});

test("Importer_whenCancelling_cancelsTheJob", async () => {
  // Given a transcription started from the dialog, without sections
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);
  vi.mocked(open).mockResolvedValueOnce("/music/take.wav");
  await user.click(screen.getByRole("button", { name: "Import audio…" }));
  expect(open).toHaveBeenCalledWith(expect.objectContaining({ filters: [expect.objectContaining({ extensions: ["mp3", "wav", "flac", "m4a", "ogg"] })] }));
  expect(transcribe).toHaveBeenCalledWith("/music/take.wav", "/music/take.madsister.json", null, false, expect.any(Function));

  // When clicking Cancel, then the engine reports it was cancelled
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  await emit({ type: "cancelled" });

  // Then the job is killed, the panel goes away and nothing is reported
  expect(cancel).toHaveBeenCalledWith(42);
  expect(screen.queryByRole("progressbar")).toBeNull();
  expect(screen.queryByRole("alert")).toBeNull();
  expect(onResult).not.toHaveBeenCalled();
});

test("Importer_whenTheEngineFails_showsTheMessageAndStderr", async () => {
  // Given a running transcription
  render(<Importer onResult={onResult} />);
  await dropFile("/music/song.flac");

  // When the engine fails
  await emit({ type: "error", message: "no beats installed", stderr: "Traceback: boom" });

  // Then the message is shown, with stderr behind a disclosure, and the panel goes away
  expect(screen.getByRole("alert").textContent).toContain("no beats installed");
  expect(screen.getByText("Traceback: boom").closest("details")).not.toBeNull();
  expect(screen.queryByRole("progressbar")).toBeNull();
  expect(onResult).not.toHaveBeenCalled();

  // When the engine can't even start
  vi.mocked(transcribe).mockRejectedValueOnce("can't start the engine (uv): not found");
  await dropFile("/music/song.flac");

  // Then that is shown too
  await vi.waitFor(() => expect(screen.getByRole("alert").textContent).toContain("can't start the engine"));
  expect(screen.queryByRole("progressbar")).toBeNull();
});

test("Importer_ofUnsupportedOrDeclinedFiles_doesNotStart", async () => {
  // Given the importer
  render(<Importer onResult={onResult} />);

  // When dropping a text file, Then it is rejected
  await dropFile("/music/notes.txt");
  expect(screen.getByRole("alert").textContent).toMatch(/notes\.txt.*mp3, wav, flac, m4a or ogg/);

  // When dropping an mp3 whose song file exists, and declining to overwrite it
  vi.mocked(exists).mockResolvedValueOnce(true);
  vi.mocked(confirm).mockResolvedValueOnce(false);
  await dropFile("/music/song.MP3");

  // Then the user was asked about that file and nothing started
  expect(confirm).toHaveBeenCalledWith(expect.stringContaining("/music/song.madsister.json"), expect.anything());
  expect(transcribe).not.toHaveBeenCalled();
});
