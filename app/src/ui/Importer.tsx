// Audio import (spec F-IN-1..4, NF-4): pick a file (dialog or drag & drop), a URL or the microphone, set the options, then Run → engine jobs with progress and cancel.
import { appDataDir, join } from "@tauri-apps/api/path";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useId, useRef, useState } from "react";
import { cancel, engineHas, FEATURES, fetchAudio, record, setupEngine, stop, transcribe, type EngineEvent, type Feature, type ForcedMeter } from "./engine";
import { AUDIO_EXTENSIONS, canWrite, pickAudioPath } from "./fileActions";

type Job = { id: number | null; stage: string; pct: number; elapsedSec?: number };
type Failure = { message: string; stderr: string };

type Props = {
  /** Only the active tab's importer takes a dropped file. */
  isActive?: boolean;
  onRunningChange?: (isRunning: boolean) => void;
  onResult: (path: string) => void;
};

export const Importer = ({ isActive = true, onRunningChange, onResult }: Props) => {
  const id = useId();
  const [job, setJob] = useState<Job | null>(null);
  const [failure, setFailure] = useState<Failure | null>(null);
  const [shouldDetectSections, setShouldDetectSections] = useState(false);
  const [isAccurate, setIsAccurate] = useState(false);
  const [meter, setMeter] = useState<ForcedMeter | null>(null);
  const [url, setUrl] = useState("");
  const [tab, setTab] = useState<"file" | "url" | "record">("file");
  const [audioPath, setAudioPath] = useState<string | null>(null);
  // A feature counts as installed until the engine says it isn't.
  const [installed, setInstalled] = useState<Record<Feature, boolean>>({ sections: true, separate: true });
  const isUnmounted = useRef(false);

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
      if (isUnmounted.current) return void cancel(id); // the tab closed while the job was starting
      setJob((j) => j && { ...j, id });
    } catch (e) {
      setJob(null);
      setFailure({ message: String(e), stderr: "" });
    }
  };

  /** Transcribes into `<stem>.madsister.json` next to the audio. */
  const transcribeFile = async (audioPath: string) => {
    const wanted = FEATURES.filter((f) => (f === "sections" ? shouldDetectSections : isAccurate));
    const isInstalled = await Promise.all(wanted.map(engineHas));
    const missing = wanted.filter((_, i) => !isInstalled[i]);
    if (missing.length > 0) {
      await run((onEvent) => setupEngine(missing, onEvent), () => {
        setInstalled((current) => ({ ...current, ...Object.fromEntries(missing.map((f) => [f, true])) }));
        void latest.current.transcribeFile(audioPath); // one-time install, then back here
      });
      return;
    }
    const outPath = `${audioPath.replace(/\.[^./\\]+$/, "")}.madsister.json`;
    if (!(await canWrite(outPath))) return;
    const mode = isAccurate ? "accurate" : "fast";
    await run((onEvent) => transcribe(audioPath, outPath, meter, shouldDetectSections, mode, onEvent), (path) => latest.current.onResult(path));
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
  const latest = useRef({ onResult, selectFile, transcribeFile, isRunning: job !== null, isActive, jobId: job?.id ?? null });
  latest.current = { onResult, selectFile, transcribeFile, isRunning: job !== null, isActive, jobId: job?.id ?? null };
  useEffect(() => {
    isUnmounted.current = false;
    for (const f of FEATURES) void engineHas(f).then((has) => setInstalled((current) => ({ ...current, [f]: has })));
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "drop" && payload.paths.length > 0 && latest.current.isActive && !latest.current.isRunning) latest.current.selectFile(payload.paths[0]);
    });
    return () => {
      void unlisten.then((f) => f());
      // A closed tab's job would run on unseen (a recording, until the app quits).
      isUnmounted.current = true;
      if (latest.current.jobId !== null) void cancel(latest.current.jobId);
    };
  }, []);
  useEffect(() => onRunningChange?.(job !== null), [job !== null]);

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
    else if (tab === "url") await fetchUrl();
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
          <button type="button" role="tab" id={`${id}-import-tab-file`} aria-selected={tab === "file"} aria-controls={`${id}-import-panel-file`} onClick={() => setTab("file")}>
            From file
          </button>
          <button type="button" role="tab" id={`${id}-import-tab-url`} aria-selected={tab === "url"} aria-controls={`${id}-import-panel-url`} onClick={() => setTab("url")}>
            From URL
          </button>
          <button type="button" role="tab" id={`${id}-import-tab-record`} aria-selected={tab === "record"} aria-controls={`${id}-import-panel-record`} onClick={() => setTab("record")}>
            Record
          </button>
        </div>
        {/* The panels share one grid cell, so the box keeps the height of the tallest one; the inactive ones are hidden. */}
        <div className="tabpanels">
          <div role="tabpanel" id={`${id}-import-panel-file`} aria-labelledby={`${id}-import-tab-file`} aria-hidden={tab !== "file"}>
            <button type="button" onClick={importAudio} disabled={job !== null}>
              Import audio…
            </button>{" "}
            <span>{audioPath !== null ? /[^/\\]+$/.exec(audioPath)?.[0] : "or drop an audio file on the window."}</span>
          </div>
          <div role="tabpanel" id={`${id}-import-panel-url`} aria-labelledby={`${id}-import-tab-url`} aria-hidden={tab !== "url"}>
            <label>
              Audio URL <input type="url" value={url} onChange={(e) => setUrl(e.target.value)} />
            </label>
          </div>
          <div role="tabpanel" id={`${id}-import-panel-record`} aria-labelledby={`${id}-import-tab-record`} aria-hidden={tab !== "record"}>
            Records the default audio input. Press Run, then Stop.
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
            <input type="checkbox" aria-describedby={`${id}-sections-hint`} checked={shouldDetectSections} onChange={(e) => setShouldDetectSections(e.target.checked)} /> Detect
            sections
          </label>
          <span id={`${id}-sections-hint`} className="hint">
            <span>Slower (~6 min for a 4-min song)</span>
            {!installed.sections && <span>Installed on first use, requires a network connection</span>}
          </span>
        </p>
        <p>
          <label>
            <input type="checkbox" aria-describedby={`${id}-accurate-hint`} checked={isAccurate} onChange={(e) => setIsAccurate(e.target.checked)} /> Accurate
            (slower)
          </label>
          <span id={`${id}-accurate-hint`} className="hint">
            <span>Separates the instruments first (~+1 min for a 4-min song)</span>
            {!installed.separate && <span>Installed on first use, requires a network connection</span>}
          </span>
        </p>
        <button type="submit" className="primary" disabled={job !== null || (tab === "file" && audioPath === null) || (tab === "url" && url === "")}>
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
