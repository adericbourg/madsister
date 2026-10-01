// Engine bridge (spec §3.2, F-IN-4): thin wrapper over the Rust `transcribe`/`cancel` commands (src-tauri/src/engine.rs).
import { Channel, invoke } from "@tauri-apps/api/core";

/** Progress events, then exactly one of result/error/cancelled. */
export type EngineEvent =
  | { type: "progress"; stage: string; pct: number }
  | { type: "result"; path: string }
  | { type: "error"; message: string; stderr: string }
  | { type: "cancelled" };

/** Starts a transcription and returns its job id. Rejects when the engine can't be started. `sections`: all-in-one (slow). */
export const transcribe = (
  audioPath: string,
  outPath: string,
  meter: 3 | 4 | null,
  sections: boolean,
  onEvent: (event: EngineEvent) => void,
): Promise<number> => {
  const channel = new Channel<EngineEvent>();
  channel.onmessage = onEvent;
  return invoke<number>("transcribe", { audioPath, outPath, meter, sections, onEvent: channel });
};

export const cancel = (jobId: number): Promise<void> => invoke("cancel", { jobId });
