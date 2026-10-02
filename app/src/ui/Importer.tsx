// Audio import (spec F-IN-1..4, NF-4): pick a file (dialog or drag & drop), a URL or the microphone, set the options, then Run → engine jobs with progress and cancel.
import { appDataDir, join } from "@tauri-apps/api/path";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useRef, useState } from "react";
import { cancel, engineHasSections, fetchAudio, record, setupEngine, stop, transcribe, type EngineEvent, type ForcedMeter } from "./engine";
import { AUDIO_EXTENSIONS, canWrite, pickAudioPath } from "./fileActions";

type Job = { id: number | null; stage: string; pct: number; elapsedSec?: number };
type Failure = { message: string; stderr: string };

export const Importer = ({ onResult }: { onResult: (path: string) => void }) => {
  const [job, setJob] = useState<Job | null>(null);
  const [failure, setFailure] = useState<Failure | null>(null);
  const [shouldDetectSections, setShouldDetectSections] = useState(false);
  const [meter, setMeter] = useState<ForcedMeter | null>(null);
  const [url, setUrl] = useState("");
  const [tab, setTab] = useState<"file" | "url">("file");
  const [audioPath, setAudioPath] = useState<string | null>(null);
  const [urlAction, setUrlAction] = useState<"fetch" | "record">("fetch");
  const [hasSections, setHasSections] = useState(true); // false only once the engine says sections aren't installed

  /** Runs one engine job; its result goes to `onDone`. */
  const run = async (begin: (onEvent: (event: EngineEvent) => void) => Promise<number>, onDone: (path: string) => void) => {
    setFailure(null);
    const onEvent = (event: EngineEvent) => {
      if (event.type === "progress") {
        setJob((j) => j && { ...j, stage: event.stage, pct: event.pct, elapsedSec: event.elapsedSec });
        return;
      }
      setJob(null);
      if (event.type === "result") onDone(event.path);
      if (event.type === "error") setFailure(event);
    };
    setJob({ id: null, stage: "Starting…", pct: 0 });
    try {
      const id = await begin(onEvent);
      setJob((j) => j && { ...j, id });
    } catch (e) {
      setJob(null);
      setFailure({ message: String(e), stderr: "" });
    }
  };

  /** Transcribes into `<stem>.madsister.json` next to the audio. */
  const transcribeFile = async (audioPath: string) => {
    if (shouldDetectSections && !(await engineHasSections())) {
      await run((onEvent) => setupEngine(true, onEvent), () => {
        setHasSections(true);
        void latest.current.transcribeFile(audioPath); // one-time install, then back here
      });
      return;
    }
    const outPath = `${audioPath.replace(/\.[^./\\]+$/, "")}.madsister.json`;
    if (!(await canWrite(outPath))) return;
    await run((onEvent) => transcribe(audioPath, outPath, meter, shouldDetectSections, onEvent), (path) => latest.current.onResult(path));
  };

  const selectFile = (path: string) => {
    const extension = /\.([^./\\]+)$/.exec(path)?.[1];
    if (extension === undefined || !AUDIO_EXTENSIONS.includes(extension.toLowerCase())) {
      setFailure({ message: `Can't import ${path}: use an mp3, wav, flac, m4a or ogg file.`, stderr: "" });
      return;
    }
    setFailure(null);
    setAudioPath(path);
    setTab("file");
  };

  // The drop listener and the engine callbacks outlive renders: they go through the latest props and state.
  const latest = useRef({ onResult, selectFile, transcribeFile, isRunning: job !== null });
  latest.current = { onResult, selectFile, transcribeFile, isRunning: job !== null };
  useEffect(() => {
    void engineHasSections().then(setHasSections);
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "drop" && payload.paths.length > 0 && !latest.current.isRunning) latest.current.selectFile(payload.paths[0]);
    });
    return () => void unlisten.then((f) => f());
  }, []);

  const importAudio = async () => {
    const path = await pickAudioPath();
    if (path !== null) selectFile(path);
  };

  // Downloads and recordings go to the app data dir; the engine keeps a download's format (e.g. webm), which it decodes.
  const sourcesDir = async () => join(await appDataDir(), "sources");
  const thenTranscribe = (path: string) => void latest.current.transcribeFile(path);

  const fetchUrl = async () => {
    if (!/^https?:\/\//i.test(url)) {
      setFailure({ message: `Can't fetch ${url}: use an http(s) URL.`, stderr: "" });
      return;
    }
    const outDir = await sourcesDir();
    await run((onEvent) => fetchAudio(url, outDir, onEvent), thenTranscribe);
  };

  const startRecording = async () => {
    const outPath = await join(await sourcesDir(), `Recording ${new Date().toISOString().slice(0, 19).replace(/:/g, "-")}.wav`);
    await run((onEvent) => record(outPath, onEvent), thenTranscribe);
  };

  const runSelected = async () => {
    if (tab === "file") await transcribeFile(audioPath!);
    else if (urlAction === "fetch") await fetchUrl();
    else await startRecording();
  };

  const isRecording = job?.stage === "record";
  const elapsed = job?.elapsedSec ?? 0;
  const status = isRecording ? `Recording ${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, "0")}` : job?.stage;

  return (
    <section aria-label="Import audio" className="importer">
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void runSelected();
        }}
      >
        <div role="tablist" aria-label="Audio source">
          <button type="button" role="tab" id="import-tab-file" aria-selected={tab === "file"} aria-controls="import-panel-file" onClick={() => setTab("file")}>
            From file
          </button>
          <button type="button" role="tab" id="import-tab-url" aria-selected={tab === "url"} aria-controls="import-panel-url" onClick={() => setTab("url")}>
            From URL
          </button>
        </div>
        {/* Both panels share one grid cell, so the box keeps the height of the taller one; the inactive one is hidden. */}
        <div className="tabpanels">
          <div role="tabpanel" id="import-panel-file" aria-labelledby="import-tab-file" aria-hidden={tab !== "file"}>
            <button type="button" onClick={importAudio} disabled={job !== null}>
              Import audio…
            </button>{" "}
            <span>{audioPath !== null ? /[^/\\]+$/.exec(audioPath)?.[0] : "or drop an audio file on the window."}</span>
          </div>
          <div role="tabpanel" id="import-panel-url" aria-labelledby="import-tab-url" aria-hidden={tab !== "url"}>
            <label>
              Audio URL <input type="url" value={url} onChange={(e) => setUrl(e.target.value)} disabled={urlAction === "record"} />
            </label>{" "}
            <span role="radiogroup" aria-label="Audio URL action">
              <label>
                <input type="radio" name="url-action" checked={urlAction === "fetch"} onChange={() => setUrlAction("fetch")} disabled={job !== null} /> Fetch
              </label>{" "}
              <label>
                <input type="radio" name="url-action" checked={urlAction === "record"} onChange={() => setUrlAction("record")} disabled={job !== null} /> Record
              </label>
            </span>
          </div>
        </div>
        <p>
          <label>
            Meter{" "}
            <select value={meter ?? ""} onChange={(e) => setMeter((e.target.value || null) as ForcedMeter | null)}>
              <option value="">Auto</option>
              <option value="4">4/4</option>
              <option value="3">3/4</option>
              <option value="6/8">6/8</option>
            </select>
          </label>
        </p>
        <p>
          <label>
            <input type="checkbox" aria-describedby="sections-hint" checked={shouldDetectSections} onChange={(e) => setShouldDetectSections(e.target.checked)} /> Detect
            sections
          </label>
          <span id="sections-hint" className="hint">
            <span>Slower (~6 min for a 4-min song)</span>
            {!hasSections && <span>Installed on first use, requires a network connection</span>}
          </span>
        </p>
        <button type="submit" disabled={job !== null || (tab === "file" && audioPath === null) || (tab === "url" && urlAction === "fetch" && url === "")}>
          Run
        </button>
      </form>
      {/* Always rendered: a live region that appears with its content isn't reliably announced. */}
      <span role="status">{status}</span>
      {job !== null && (
        <>
          {" "}
          {isRecording ? (
            <button type="button" onClick={() => job.id !== null && void stop(job.id)}>
              Stop
            </button>
          ) : (
            <progress aria-label={job.stage} value={job.pct} max={100} />
          )}{" "}
          <button type="button" onClick={() => job.id !== null && void cancel(job.id)}>
            Cancel
          </button>
        </>
      )}
      {failure !== null && (
        <div role="alert">
          {failure.message}
          {failure.stderr !== "" && (
            <details>
              <summary>Engine output</summary>
              <pre>{failure.stderr}</pre>
            </details>
          )}
        </div>
      )}
    </section>
  );
};
