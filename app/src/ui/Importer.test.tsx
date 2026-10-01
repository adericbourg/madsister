import { confirm, open } from "@tauri-apps/plugin-dialog";
import { exists } from "@tauri-apps/plugin-fs";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cancel, engineHasSections, fetchAudio, record, setupSections, stop, transcribe, type EngineEvent } from "./engine";
import { Importer } from "./Importer";

vi.mock("./engine", () => ({ engineHasSections: vi.fn(), setupSections: vi.fn(), transcribe: vi.fn(), fetchAudio: vi.fn(), record: vi.fn(), stop: vi.fn(), cancel: vi.fn() }));
vi.mock("@tauri-apps/api/path", () => ({ appDataDir: async () => "/data", join: async (...parts: string[]) => parts.join("/") }));
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
/** Sends an event to the engine callback of the last `transcribe` call (or of the given callback). */
const emit = (event: EngineEvent, onEvent = () => vi.mocked(transcribe).mock.lastCall![4]) => act(() => onEvent()(event));
const fetchEvent = () => vi.mocked(fetchAudio).mock.lastCall![2];
const recordEvent = () => vi.mocked(record).mock.lastCall![1];

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(engineHasSections).mockResolvedValue(true);
  vi.mocked(setupSections).mockResolvedValue(9);
  vi.mocked(transcribe).mockResolvedValue(42);
  vi.mocked(fetchAudio).mockResolvedValue(7);
  vi.mocked(record).mockResolvedValue(8);
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

test("Importer_whenSectionsAreNotInstalled_installsThemBeforeTranscribing", async () => {
  // Given "Detect sections" checked while all-in-one isn't installed
  const user = userEvent.setup();
  vi.mocked(engineHasSections).mockResolvedValue(false);
  render(<Importer onResult={onResult} />);
  await user.click(screen.getByRole("checkbox", { name: /Detect sections \(slow/ }));

  // When dropping an mp3
  await dropFile("/music/song.mp3");

  // Then the install runs first and nothing is transcribed yet
  expect(setupSections).toHaveBeenCalledOnce();
  expect(transcribe).not.toHaveBeenCalled();

  // When the install ends, Then the transcription starts with all-in-one
  vi.mocked(engineHasSections).mockResolvedValue(true);
  await emit({ type: "result", path: "/models" }, () => vi.mocked(setupSections).mock.lastCall![0]);
  expect(transcribe).toHaveBeenCalledWith("/music/song.mp3", "/music/song.madsister.json", null, true, expect.any(Function));
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

test("Importer_whenFetchingAUrl_downloadsThenTranscribesTheFile", async () => {
  // Given the importer
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);

  // When fetching a URL
  await user.type(screen.getByRole("textbox", { name: "Audio URL" }), "https://example.org/v?id=1");
  await user.click(screen.getByRole("button", { name: "Fetch" }));

  // Then it is downloaded into the app data dir, with its progress shown and nothing else startable
  expect(fetchAudio).toHaveBeenCalledWith("https://example.org/v?id=1", "/data/sources", expect.any(Function));
  await emit({ type: "progress", stage: "download", pct: 30 }, fetchEvent);
  expect(screen.getByRole("status").textContent).toBe("download");
  expect(screen.getByRole("progressbar", { name: "download" })).toHaveProperty("value", 30);
  expect(screen.getByRole("button", { name: "Record" })).toHaveProperty("disabled", true);

  // When the download ends, Then the downloaded file (any format) is transcribed next to it
  await emit({ type: "result", path: "/data/sources/My song.webm" }, fetchEvent);
  await vi.waitFor(() =>
    expect(transcribe).toHaveBeenCalledWith("/data/sources/My song.webm", "/data/sources/My song.madsister.json", null, false, expect.any(Function)),
  );

  // When the transcription ends, Then its result is handed over
  await emit({ type: "result", path: "/data/sources/My song.madsister.json" });
  expect(onResult).toHaveBeenCalledWith("/data/sources/My song.madsister.json");
});

test("Importer_whenCancellingAFetchOrItsTranscription_stopsThere", async () => {
  // Given a download
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);
  await user.type(screen.getByRole("textbox", { name: "Audio URL" }), "https://example.org/v");
  await user.click(screen.getByRole("button", { name: "Fetch" }));

  // When cancelling it, Then the download job is killed and nothing is transcribed
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  await emit({ type: "cancelled" }, fetchEvent);
  expect(cancel).toHaveBeenCalledWith(7);
  expect(transcribe).not.toHaveBeenCalled();
  expect(screen.queryByRole("progressbar")).toBeNull();

  // When fetching again, then cancelling during the transcription
  await user.click(screen.getByRole("button", { name: "Fetch" }));
  await emit({ type: "result", path: "/data/sources/v.m4a" }, fetchEvent);
  await vi.waitFor(() => expect(transcribe).toHaveBeenCalled());
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  await emit({ type: "cancelled" });

  // Then the transcription job is killed and nothing is reported
  expect(cancel).toHaveBeenLastCalledWith(42);
  expect(onResult).not.toHaveBeenCalled();
});

test("Importer_ofANonHttpUrl_doesNotFetch", async () => {
  // Given / When fetching a local path
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);
  await user.type(screen.getByRole("textbox", { name: "Audio URL" }), "file:///etc/passwd");
  await user.click(screen.getByRole("button", { name: "Fetch" }));

  // Then it is rejected
  expect(screen.getByRole("alert").textContent).toContain("http");
  expect(fetchAudio).not.toHaveBeenCalled();
});

test("Importer_whenRecording_showsTheElapsedTimeThenTranscribesOnStop", async () => {
  // Given the importer
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);

  // When recording for 65 s
  await user.click(screen.getByRole("button", { name: "Record" }));
  await emit({ type: "progress", stage: "record", pct: 0, elapsedSec: 65 }, recordEvent);

  // Then a WAV is recorded into the app data dir and the elapsed time is shown
  expect(record).toHaveBeenCalledWith(expect.stringMatching(/^\/data\/sources\/Recording .+\.wav$/), expect.any(Function));
  expect(screen.getByRole("status").textContent).toBe("Recording 1:05");

  // When stopping, then the engine writes the file, Then it is transcribed
  await user.click(screen.getByRole("button", { name: "Stop" }));
  expect(stop).toHaveBeenCalledWith(8);
  await emit({ type: "result", path: "/data/sources/Recording x.wav" }, recordEvent);
  await vi.waitFor(() =>
    expect(transcribe).toHaveBeenCalledWith("/data/sources/Recording x.wav", "/data/sources/Recording x.madsister.json", null, false, expect.any(Function)),
  );
  expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
});

test("Importer_whenCancellingARecording_doesNotTranscribe", async () => {
  // Given a recording
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);
  await user.click(screen.getByRole("button", { name: "Record" }));
  await emit({ type: "progress", stage: "record", pct: 0, elapsedSec: 2 }, recordEvent);

  // When cancelling it
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  await emit({ type: "cancelled" }, recordEvent);

  // Then the recording job is killed, not stopped, and nothing follows
  expect(cancel).toHaveBeenCalledWith(8);
  expect(stop).not.toHaveBeenCalled();
  expect(transcribe).not.toHaveBeenCalled();
  expect(screen.queryByRole("status")?.textContent).toBe("");
});
