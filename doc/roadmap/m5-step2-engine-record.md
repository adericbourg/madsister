# M5 · Step 2 — Engine `record` (microphone)

> Coarse step. Refine before starting.

## Goal
`madsister-engine record --out <file.wav>` records from the default input until SIGINT or `stop` on stdin (F-IN-3, D3).

## Spec refs
F-IN-3, D3, §3.2.

## Depends on
M0-1.

## Tasks
- Dependency `sounddevice` (optional group `record`; needs PortAudio: bundled in the macOS/Windows wheels, `libportaudio2` on Linux —
  document it, and add it to CI only if a non-slow test needs it).
- `InputStream` callback → queue → writer thread with stdlib `wave` (mono 44.1 kHz, 16-bit). Progress events every second with
  `stage: "record"` and elapsed seconds (the `pct` field stays absent or 0: extend the event with `elapsedSec`, and document it in §3.2).
- Stop on SIGINT or a `stop` line on stdin; then `result` with the path.
- macOS: microphone permission prompt applies to the parent app (M6 packaging adds `NSMicrophoneUsageDescription`).

## Tests
- Unit with a fake stream: stop via stdin writes a valid WAV of the expected length.

## Verify
`cd engine && uv run pytest -m "not slow"`; a manual 5-second recording.

## Commit message
`feat(engine): record from the microphone`
