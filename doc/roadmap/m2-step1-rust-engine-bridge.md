# M2 · Step 1 — Rust ↔ engine bridge

> Refined on 2026-10-01 from the M0 results and the M1 code.

## Goal
The Tauri shell spawns `madsister-engine`, streams its JSON Lines to the UI, and can cancel it (D2, F-IN-4, NF-4).

## Spec refs
§3, §3.2, D2, F-IN-4, NF-4.

## Depends on
M0-11, M1-11.

## Engine facts (M0)
- `madsister-engine transcribe <audio> --out <json> [--meter 3|4] [--no-sections]`. stdout = JSONL only (`progress{stage,pct}`,
  `result{path}`, `error{message}`; model libs' stdout goes to stderr), exit 0/1. Default combo = madmom + CNN-LSTM.
- `--no-sections` isn't exposed yet: sections come from all-in-one, a slow opt-in (M5-4 decides the UI).
- The engine may spawn children (ffmpeg, Demucs), and in dev it runs under `uv`: cancel kills the whole process group.

## Tasks
- Engine command resolution (`engine.rs`): env `MADSISTER_ENGINE` (a command line, split on whitespace) or, in dev,
  `uv run --project <repo>/engine --no-sync madsister-engine` (`<repo>` from `CARGO_MANIFEST_DIR`). `--no-sync` because a plain
  `uv run` re-syncs to the default groups and removes the model groups. The dev env is synced once with
  `uv sync --project engine --group beats-madmom --group chords-cnnlstm` (`+ --group beats-allinone` for sections).
  Packaged resolution is M6.
- Rust command `transcribe(audioPath, outPath, meter?, onEvent) -> jobId` (`jobId` = the child pid). Events go through a
  `tauri::ipc::Channel` passed by the caller rather than a global `engine://<id>` event: ordered, and no event is lost
  between the spawn and the `listen()`. `std::process::Command` (no `tauri-plugin-shell`: it would need a scope for an
  arbitrary command line, and `process_group` is std).
- Events (`#[serde(tag = "type")]`): `progress{stage,pct}` forwarded as they come; then exactly one terminal event:
  `result{path}`, `error{message,stderr}` (engine `error` line, or synthetic when the exit is non-zero without one, or exit 0
  without a `result`), or `cancelled`. `stderr` = the last 20 stderr lines. Unparsable stdout lines go to that tail.
- `cancel(jobId)`: `killpg(SIGKILL)` on the child's process group (spawned with `process_group(0)`). Only pids of live jobs
  are killed (a job leaves the set before it's reaped, so a reused pid is never hit). `libc` is already in `Cargo.lock`.
- TS side: `ui/engine.ts` with `transcribe(audioPath, outPath, meter, onEvent)` and `cancel(jobId)`.

## Tests
- Rust unit tests on the line parser (valid, invalid JSON, unknown type).
- Rust integration test (`tests/engine.rs`) with a fake engine (`tests/fake-engine.sh`): success, error with stderr,
  crash without an error line, cancel (the fake engine's own child must die too).
- vitest: `ui/engine.ts` with `@tauri-apps/api/core` mocked.
- CI runs `cargo test --locked`.

## Verify
`cd app && pnpm test && pnpm build && (cd src-tauri && cargo test && cargo clippy -- -D warnings)`; one real run of the
engine through the bridge on a GuitarSet take (or a short WAV).

## Commit message
`feat(app): spawn the engine and stream its progress`
