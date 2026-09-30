# M5 · Step 3 — URL and recording in the app

> Coarse step. Refine before starting.

## Goal
M5 "done when": a URL and a recording both yield a grid.

## Spec refs
F-IN-2, F-IN-3, F-IN-4, §8 M5.

## Depends on
M5-1, M5-2, M2-2.

## Tasks
- "Import from URL…": input + Fetch → progress → then the normal transcription flow on the downloaded file (stored in the app data dir
  `sources/`). Cancel works in both stages.
- "Record…": Start/Stop with an elapsed-time display; Stop writes `stop` to the engine's stdin (bridge needs a `send(job_id, line)`), then
  transcribe. Recording files in `sources/` too.

## Tests
- Component tests with the engine mocked for both flows, including cancel.

## Verify
`cd app && pnpm test`; manual end-to-end in `pnpm tauri dev`.

## Commit message
`feat(app): import from URL and from the microphone`
