// Engine bridge (spec §3.2, F-IN-4): thin wrapper over the Rust engine commands (src-tauri/src/engine.rs, lib.rs).
import { Channel, invoke } from "@tauri-apps/api/core";

/** Progress events (`elapsedSec`: `record` only), then exactly one of result/error/cancelled. */
export type EngineEvent =
  | { type: "progress"; stage: string; pct: number; elapsedSec?: number }
  | { type: "result"; path: string }
  | { type: "error"; message: string; stderr: string }
  | { type: "cancelled" };

/** A forced meter, as `madsister-engine transcribe --meter` takes it; null = auto. */
export type ForcedMeter = "3" | "4" | "6/8";

/** Starts an engine command and returns its job id. Rejects when the engine can't be started. */
const start = (command: string, args: Record<string, unknown>, onEvent: (event: EngineEvent) => void): Promise<number> => {
  const channel = new Channel<EngineEvent>();
  channel.onmessage = onEvent;
  return invoke<number>(command, { ...args, onEvent: channel });
};

/** `sections`: all-in-one (slow). */
export const transcribe = (
  audioPath: string,
  outPath: string,
  meter: ForcedMeter | null,
  sections: boolean,
  onEvent: (event: EngineEvent) => void,
): Promise<number> => start("transcribe", { audioPath, outPath, meter, sections }, onEvent);

/** Downloads the URL's audio into `outDir`; the result is the file's path. */
export const fetchAudio = (url: string, outDir: string, onEvent: (event: EngineEvent) => void): Promise<number> =>
  start("fetch", { url, outDir }, onEvent);

/** Records the microphone into `outPath` (a WAV) until `stop`. */
export const record = (outPath: string, onEvent: (event: EngineEvent) => void): Promise<number> =>
  start("record", { outPath }, onEvent);

/** Ends a recording and keeps it (its result follows); `cancel` kills any job, a recording is then lost. */
export const stop = (jobId: number): Promise<void> => invoke("stop", { jobId });

export const cancel = (jobId: number): Promise<void> => invoke("cancel", { jobId });

/** True on a packaged app's first launch (and after an update): `setupEngine` must run before the commands above. */
export const engineNeedsSetup = (): Promise<boolean> => invoke("engine_needs_setup");

/** Installs the packaged engine: an `install` progress (uv), then `setup`'s per model, then a result. `sections`: also all-in-one (slow, once). */
export const setupEngine = (sections: boolean, onEvent: (event: EngineEvent) => void): Promise<number> =>
  start("setup_engine", { sections }, onEvent);

/** True when "Detect sections" can run (always outside a packaged app). */
export const engineHasSections = (): Promise<boolean> => invoke("engine_has_sections");
