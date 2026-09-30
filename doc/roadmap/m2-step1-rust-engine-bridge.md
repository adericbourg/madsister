# M2 · Step 1 — Rust ↔ engine bridge

> Coarse step (written before M0 results). Re-read `m0-results.md` and refine before starting.

## Goal
The Tauri shell spawns `madsister-engine`, streams its JSON Lines to the UI as events, and can cancel it (D2, F-IN-4, NF-4).

## Spec refs
§3, §3.2, D2, F-IN-4, NF-4.

## Depends on
M0-11, M1-11.

## Tasks
- Engine command resolution: env `MADSISTER_ENGINE` (a command line) or, in dev, `uv run --project <repo>/engine madsister-engine`.
  (Packaged resolution is M6.)
- Rust command `transcribe(audio_path, out_path, meter?) -> job_id`: `std::process::Command` (or `tauri-plugin-shell`, check which is
  simpler in v2), read stdout line by line on a thread, parse with serde into `Progress | Result | Error`, emit a Tauri event per line
  (`engine://<job_id>`). Keep stderr in a bounded buffer and attach it to the error. Non-zero exit without an error line → synthetic error.
- `cancel(job_id)`: kill the child (and its process group: the engine may spawn ffmpeg/Demucs children).
- TS side: `ui/engine.ts` with `transcribe(...)` returning an async iterator or callbacks, plus `cancel`.

## Tests
- Rust unit tests on the line parser (valid, invalid JSON, unknown type).
- Rust integration test with a fake engine script (a shell script echoing JSONL) for success, error, and cancel.

## Verify
`cd app/src-tauri && cargo test && cargo clippy -- -D warnings`; `cd app && pnpm test`

## Commit message
`feat(app): spawn the engine and stream its progress`
