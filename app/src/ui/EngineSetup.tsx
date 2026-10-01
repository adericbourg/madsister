// Packaged app's first launch (M6-2): installs the engine, then shows its children (the importer).
import { useEffect, useState, type ReactNode } from "react";
import { engineNeedsSetup, setupEngine, type EngineEvent } from "./engine";

const STAGES: Record<string, string> = {
  install: "Installing Python and the engine's libraries…",
  setup: "Downloading the models…",
};

type Failure = { message: string; stderr: string };

export const EngineSetup = ({ children }: { children: ReactNode }) => {
  const [isReady, setIsReady] = useState<boolean | null>(null);
  const [progress, setProgress] = useState<{ stage: string; pct: number } | null>(null);
  const [failure, setFailure] = useState<Failure | null>(null);

  const run = () => {
    setFailure(null);
    setProgress({ stage: "install", pct: 0 });
    const onEvent = (event: EngineEvent) => {
      if (event.type === "progress") return setProgress(event);
      setProgress(null);
      if (event.type === "result") setIsReady(true);
      else setFailure(event.type === "error" ? event : { message: "The setup was cancelled.", stderr: "" });
    };
    setupEngine(onEvent).catch((e) => onEvent({ type: "error", message: String(e), stderr: "" }));
  };

  useEffect(() => {
    void engineNeedsSetup().then((needsSetup) => {
      setIsReady(!needsSetup);
      if (needsSetup) run();
    });
  }, []);

  // While checking too: the check only reads a file, and dev builds never need a setup.
  if (isReady !== false) return children;
  return (
    <section aria-label="Engine setup">
      <p>
        First launch: madsister installs its transcription engine (Python, its libraries and the models, several hundred MB
        to download, once). This takes a few minutes; you can edit songs meanwhile.
      </p>
      {/* Always rendered: a live region that appears with its content isn't reliably announced. */}
      <span role="status">{progress === null ? "" : (STAGES[progress.stage] ?? progress.stage)}</span>{" "}
      {progress !== null && (
        <progress aria-label="Engine setup" value={progress.stage === "install" ? undefined : progress.pct} max={100} />
      )}
      {failure !== null && (
        <div role="alert">
          The engine setup failed: {failure.message}{" "}
          <button type="button" onClick={run}>
            Retry
          </button>
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
