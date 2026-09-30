# M2 · Step 3 — Force the meter before transcription

> Coarse step. Refine before starting.

## Goal
F-IN-5: the user may force 3/4, 4/4 or 6/8 before transcribing. Default: auto.

## Spec refs
F-IN-5, §3.2 (`--meter 4|3`), §4 (`meta.meter` 6/8).

## Depends on
M2-2.

## Tasks
- Import dialog: meter select (Auto, 4/4, 3/4, 6/8), passed to the engine.
- Engine: extend `--meter` to accept `6/8`. 6/8 = 2 dotted-quarter beats per bar for the trackers, or 6 eighth-note beats? Decide from
  what the trackers output (probably 2 beats per bar at the dotted-quarter pulse) and write `meta.meter = {beats: 6, unit: 8}` with slot
  beats counted in eighths (×3). Document the choice in the spec §3.2 (short line) since the CLI contract changes.

## Tests
- Engine unit test for the 6/8 conversion in quantization; UI test that the chosen meter reaches `transcribe`.

## Verify
`cd engine && uv run pytest -m "not slow"`; `cd app && pnpm test`

## Commit message
`feat: let the user force the meter before transcription`
