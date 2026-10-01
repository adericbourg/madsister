// Audio import (spec F-IN-1, F-IN-4, NF-4): dialog or drag & drop → engine job with progress and cancel.
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useRef, useState } from "react";
import { cancel, transcribe, type EngineEvent } from "./engine";
import { AUDIO_EXTENSIONS, canWrite, pickAudioPath } from "./fileActions";

type Job = { id: number | null; stage: string; pct: number };
type Failure = { message: string; stderr: string };

export const Importer = ({ onResult }: { onResult: (path: string) => void }) => {
  const [job, setJob] = useState<Job | null>(null);
  const [failure, setFailure] = useState<Failure | null>(null);
  const [shouldDetectSections, setShouldDetectSections] = useState(false);

  const start = async (audioPath: string) => {
    setFailure(null);
    const match = /^(.*)\.([^./\\]+)$/.exec(audioPath);
    if (match === null || !AUDIO_EXTENSIONS.includes(match[2].toLowerCase())) {
      setFailure({ message: `Can't import ${audioPath}: use an mp3, wav, flac, m4a or ogg file.`, stderr: "" });
      return;
    }
    const outPath = `${match[1]}.madsister.json`;
    if (!(await canWrite(outPath))) return;
    const onEvent = (event: EngineEvent) => {
      if (event.type === "progress") {
        setJob((j) => j && { ...j, stage: event.stage, pct: event.pct });
        return;
      }
      setJob(null);
      if (event.type === "result") latest.current.onResult(event.path);
      if (event.type === "error") setFailure(event);
    };
    setJob({ id: null, stage: "Starting…", pct: 0 });
    try {
      const id = await transcribe(audioPath, outPath, null, shouldDetectSections, onEvent);
      setJob((j) => j && { ...j, id });
    } catch (e) {
      setJob(null);
      setFailure({ message: String(e), stderr: "" });
    }
  };

  // The drop listener and the engine callback outlive renders: they go through the latest props and state.
  const latest = useRef({ onResult, start, isRunning: job !== null });
  latest.current = { onResult, start, isRunning: job !== null };
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "drop" && payload.paths.length > 0 && !latest.current.isRunning) void latest.current.start(payload.paths[0]);
    });
    return () => void unlisten.then((f) => f());
  }, []);

  const importAudio = async () => {
    const path = await pickAudioPath();
    if (path !== null) await start(path);
  };

  return (
    <section aria-label="Import audio" className="importer">
      <button type="button" onClick={importAudio} disabled={job !== null}>
        Import audio…
      </button>{" "}
      <label>
        <input type="checkbox" checked={shouldDetectSections} onChange={(e) => setShouldDetectSections(e.target.checked)} /> Detect
        sections (slow: ≈ 6 min for a 4-min song, instead of ≈ 20 s)
      </label>{" "}
      <span>or drop an audio file on the window.</span>
      {/* Always rendered: a live region that appears with its content isn't reliably announced. */}
      <span role="status">{job?.stage}</span>
      {job !== null && (
        <>
          {" "}
          <progress aria-label={job.stage} value={job.pct} max={100} />{" "}
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
