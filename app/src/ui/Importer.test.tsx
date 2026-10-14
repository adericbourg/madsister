import { confirm, open } from "@tauri-apps/plugin-dialog";
import { exists } from "@tauri-apps/plugin-fs";
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cancel, engineHas, fetchAudio, record, setupEngine, stop, transcribe, type EngineEvent } from "./engine";
import { Importer } from "./Importer";

vi.mock("./engine", () => ({ FEATURES: ["sections", "separate"], engineHas: vi.fn(), setupEngine: vi.fn(), transcribe: vi.fn(), fetchAudio: vi.fn(), record: vi.fn(), stop: vi.fn(), cancel: vi.fn() }));
vi.mock("@tauri-apps/api/path", () => ({ appLocalDataDir: async () => "/data", join: async (...parts: string[]) => parts.join("/") }));
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
const emit = (event: EngineEvent, onEvent = () => vi.mocked(transcribe).mock.lastCall![5]) => act(() => onEvent()(event));
const fetchEvent = () => vi.mocked(fetchAudio).mock.lastCall![2];
const recordEvent = () => vi.mocked(record).mock.lastCall![1];

afterEach(cleanup);

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(engineHas).mockResolvedValue(true);
  vi.mocked(setupEngine).mockResolvedValue(9);
  vi.mocked(transcribe).mockResolvedValue(42);
  vi.mocked(fetchAudio).mockResolvedValue(7);
  vi.mocked(record).mockResolvedValue(8);
  vi.mocked(exists).mockResolvedValue(false);
});

test("Importer_whenDroppingAnAudioFile_waitsForRunThenShowsProgressAndReportsTheResult", async () => {
  // Given the importer
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);

  // Then "Detect sections" and "Accurate" warn about their duration, and not about an install since both are installed
  const hint = (name: string) => document.getElementById(screen.getByRole("checkbox", { name }).getAttribute("aria-describedby")!)!.textContent;
  expect(hint("Detect sections")).toContain("Slower (~6 min for a 4-min song)");
  expect(hint("Accurate (slower)")).toContain("Separates the instruments first (~+1 min for a 4-min song)");
  expect(screen.queryByText(/Installed on first use/)).toBeNull();

  // When dropping an mp3
  await dropFile("/music/song.mp3");

  // Then the file is selected and nothing starts
  expect(screen.getByText("song.mp3")).toBeDefined();
  expect(transcribe).not.toHaveBeenCalled();

  // When choosing the options then running
  await user.click(screen.getByRole("checkbox", { name: "Detect sections" }));
  await user.selectOptions(screen.getByRole("combobox", { name: "Meter" }), "6/8");
  await user.click(screen.getByRole("button", { name: "Run" }));

  // Then the engine starts on it, writing next to it, with that meter and all-in-one
  expect(transcribe).toHaveBeenCalledWith("/music/song.mp3", "/music/song.madsister.json", "6/8", true, "fast", expect.any(Function));
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

test("Importer_whenSectionsOrAccurateAreNotInstalled_installsThemBeforeTranscribing", async () => {
  // Given "Detect sections" and "Accurate" checked while neither all-in-one nor Demucs is installed
  const user = userEvent.setup();
  vi.mocked(engineHas).mockResolvedValue(false);
  render(<Importer onResult={onResult} />);
  await user.click(screen.getByRole("checkbox", { name: "Detect sections" }));
  await user.click(screen.getByRole("checkbox", { name: "Accurate (slower)" }));

  // Then the install requirement is announced for each
  expect(await screen.findAllByText("Installed on first use, requires a network connection")).toHaveLength(2);

  // When dropping an mp3 then running
  await dropFile("/music/song.mp3");
  await user.click(screen.getByRole("button", { name: "Run" }));

  // Then one install of both runs first and nothing is transcribed yet
  expect(engineHas).toHaveBeenCalledWith("sections");
  expect(engineHas).toHaveBeenCalledWith("separate");
  expect(setupEngine).toHaveBeenCalledExactlyOnceWith(["sections", "separate"], expect.any(Function));
  expect(transcribe).not.toHaveBeenCalled();

  // When the install ends, Then the transcription starts with all-in-one, in accurate mode
  vi.mocked(engineHas).mockResolvedValue(true);
  await emit({ type: "result", path: "/models" }, () => vi.mocked(setupEngine).mock.lastCall![1]);
  expect(screen.queryByText(/Installed on first use/)).toBeNull();
  expect(transcribe).toHaveBeenCalledWith("/music/song.mp3", "/music/song.madsister.json", null, true, "accurate", expect.any(Function));
});

test("Importer_whenCancelling_cancelsTheJob", async () => {
  // Given a file picked from the dialog, then run without sections
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);
  vi.mocked(open).mockResolvedValueOnce("/music/take.wav");
  await user.click(screen.getByRole("button", { name: "Import audio…" }));
  expect(open).toHaveBeenCalledWith(expect.objectContaining({ filters: [expect.objectContaining({ extensions: ["mp3", "wav", "flac", "m4a", "ogg", "webm"] })] }));
  expect(screen.getByText("take.wav")).toBeDefined();
  expect(transcribe).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Run" }));
  expect(transcribe).toHaveBeenCalledWith("/music/take.wav", "/music/take.madsister.json", null, false, "fast", expect.any(Function));

  // When clicking Cancel, then the engine reports it was cancelled
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  await emit({ type: "cancelled" });

  // Then the job is killed, the panel goes away and nothing is reported
  expect(cancel).toHaveBeenCalledWith(42);
  expect(screen.queryByRole("progressbar")).toBeNull();
  expect(screen.queryByRole("alert")).toBeNull();
  expect(onResult).not.toHaveBeenCalled();
});

test("Importer_whenUnmountedDuringAJob_cancelsIt", async () => {
  // Given a recording running, and a second importer whose recording is still starting
  const user = userEvent.setup();
  const running = render(<Importer onResult={onResult} />);
  await user.click(screen.getByRole("tab", { name: "Record" }));
  await user.click(screen.getByRole("button", { name: "Run" }));
  running.unmount();

  // Then closing its tab kills the running job
  expect(cancel).toHaveBeenCalledExactlyOnceWith(8);

  // When the second one is unmounted before its job id comes back
  let started = (_: number) => {};
  vi.mocked(record).mockReturnValueOnce(new Promise((resolve) => (started = resolve)));
  const starting = render(<Importer onResult={onResult} />);
  await user.click(screen.getByRole("tab", { name: "Record" }));
  await user.click(screen.getByRole("button", { name: "Run" }));
  starting.unmount();
  await act(async () => started(11));

  // Then the job is killed as soon as it has an id
  expect(cancel).toHaveBeenLastCalledWith(11);
});

test("Importer_whenTheEngineFails_showsTheMessageAndStderr", async () => {
  // Given a running transcription
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);
  await dropFile("/music/song.flac");
  await user.click(screen.getByRole("button", { name: "Run" }));

  // When the engine fails
  await emit({ type: "error", message: "no beats installed", stderr: "Traceback: boom" });

  // Then the message is shown, with stderr behind a disclosure, and the panel goes away
  expect(screen.getByRole("alert").textContent).toContain("no beats installed");
  expect(screen.getByText("Traceback: boom").closest("details")).not.toBeNull();
  expect(screen.queryByRole("progressbar")).toBeNull();
  expect(onResult).not.toHaveBeenCalled();

  // When the engine can't even start
  vi.mocked(transcribe).mockRejectedValueOnce("can't start the engine (uv): not found");
  await user.click(screen.getByRole("button", { name: "Run" }));

  // Then that is shown too
  await vi.waitFor(() => expect(screen.getByRole("alert").textContent).toContain("can't start the engine"));
  expect(screen.queryByRole("progressbar")).toBeNull();
});

test("Importer_whenTheSongFileCantBeChecked_showsWhy", async () => {
  // Given an mp3 whose song file is out of the fs scope
  const user = userEvent.setup();
  vi.mocked(exists).mockRejectedValueOnce("forbidden path: /media/usb/song.madsister.json");
  render(<Importer onResult={onResult} />);
  await dropFile("/media/usb/song.mp3");

  // When running
  await user.click(screen.getByRole("button", { name: "Run" }));

  // Then the reason is shown and nothing started
  await vi.waitFor(() => expect(screen.getByRole("alert").textContent).toContain("forbidden path"));
  expect(transcribe).not.toHaveBeenCalled();
});

test("Importer_ofUnsupportedOrDeclinedFiles_doesNotStart", async () => {
  // Given the importer
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);

  // When dropping a text file, Then it is rejected and there is nothing to run
  await dropFile("/music/notes.txt");
  expect(screen.getByRole("alert").textContent).toMatch(/notes\.txt.*mp3, wav, flac, m4a or ogg/);
  expect(screen.getByRole("button", { name: "Run" })).toHaveProperty("disabled", true);

  // When dropping an mp3 whose song file exists, and declining to overwrite it
  vi.mocked(exists).mockResolvedValueOnce(true);
  vi.mocked(confirm).mockResolvedValueOnce(false);
  await dropFile("/music/song.MP3");
  await user.click(screen.getByRole("button", { name: "Run" }));

  // Then the user was asked about that file and nothing started
  expect(confirm).toHaveBeenCalledWith(expect.stringContaining("/music/song.madsister.json"), expect.anything());
  expect(transcribe).not.toHaveBeenCalled();
});

test("Importer_whenFetchingAUrl_downloadsThenTranscribesTheFile", async () => {
  // Given the importer
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);

  // When fetching a URL from the URL tab
  expect(screen.getByRole("tab", { name: "From file" }).getAttribute("aria-selected")).toBe("true");
  await user.click(screen.getByRole("tab", { name: "From URL" }));
  expect(screen.getByRole("tab", { name: "From URL" }).getAttribute("aria-selected")).toBe("true");
  await user.type(screen.getByRole("textbox", { name: "Audio URL" }), "https://example.org/v?id=1");
  expect(fetchAudio).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "Run" }));

  // Then it is downloaded into the app data dir, with its progress shown and nothing else startable
  expect(fetchAudio).toHaveBeenCalledWith("https://example.org/v?id=1", "/data/sources", expect.any(Function));
  await emit({ type: "progress", stage: "download", pct: 30 }, fetchEvent);
  expect(screen.getByRole("status").textContent).toBe("download");
  expect(screen.getByRole("progressbar", { name: "download" })).toHaveProperty("value", 30);
  expect(screen.getByRole("button", { name: "Run" })).toHaveProperty("disabled", true);

  // When the download ends, Then the downloaded file (any format) is transcribed next to it
  await emit({ type: "result", path: "/data/sources/My song.webm" }, fetchEvent);
  await vi.waitFor(() =>
    expect(transcribe).toHaveBeenCalledWith("/data/sources/My song.webm", "/data/sources/My song.madsister.json", null, false, "fast", expect.any(Function)),
  );

  // When the transcription ends, Then its result is handed over
  await emit({ type: "result", path: "/data/sources/My song.madsister.json" });
  expect(onResult).toHaveBeenCalledWith("/data/sources/My song.madsister.json");
});

test("Importer_whenCancellingAFetchOrItsTranscription_stopsThere", async () => {
  // Given a download
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);
  await user.click(screen.getByRole("tab", { name: "From URL" }));
  await user.type(screen.getByRole("textbox", { name: "Audio URL" }), "https://example.org/v");
  await user.click(screen.getByRole("button", { name: "Run" }));

  // When cancelling it, Then the download job is killed and nothing is transcribed
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  await emit({ type: "cancelled" }, fetchEvent);
  expect(cancel).toHaveBeenCalledWith(7);
  expect(transcribe).not.toHaveBeenCalled();
  expect(screen.queryByRole("progressbar")).toBeNull();

  // When fetching again, then cancelling during the transcription
  await user.click(screen.getByRole("button", { name: "Run" }));
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
  await user.click(screen.getByRole("tab", { name: "From URL" }));
  await user.type(screen.getByRole("textbox", { name: "Audio URL" }), "file:///etc/passwd");
  await user.click(screen.getByRole("button", { name: "Run" }));

  // Then it is rejected
  expect(screen.getByRole("alert").textContent).toContain("http");
  expect(fetchAudio).not.toHaveBeenCalled();
});

test("Importer_whenRecording_showsTheElapsedTimeThenTranscribesOnStop", async () => {
  // Given the importer
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);

  // When recording for 65 s
  await user.click(screen.getByRole("tab", { name: "Record" }));
  expect(screen.getByRole("tab", { name: "Record" }).getAttribute("aria-selected")).toBe("true");
  await user.click(screen.getByRole("button", { name: "Run" }));
  await emit({ type: "progress", stage: "record", pct: 0, elapsedSec: 65 }, recordEvent);

  // Then a WAV is recorded into the app data dir and the elapsed time is shown
  expect(record).toHaveBeenCalledWith(expect.stringMatching(/^\/data\/sources\/Recording .+\.wav$/), expect.any(Function));
  expect(screen.getByRole("status").textContent).toBe("Recording 1:05");

  // When stopping, then the engine writes the file, Then it is transcribed
  await user.click(screen.getByRole("button", { name: "Stop" }));
  expect(stop).toHaveBeenCalledWith(8);
  await emit({ type: "result", path: "/data/sources/Recording x.wav" }, recordEvent);
  await vi.waitFor(() =>
    expect(transcribe).toHaveBeenCalledWith("/data/sources/Recording x.wav", "/data/sources/Recording x.madsister.json", null, false, "fast", expect.any(Function)),
  );
  expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
});

test("Importer_whenCancellingARecording_doesNotTranscribe", async () => {
  // Given a recording
  const user = userEvent.setup();
  render(<Importer onResult={onResult} />);
  await user.click(screen.getByRole("tab", { name: "Record" }));
  await user.click(screen.getByRole("button", { name: "Run" }));
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
