# M4b · Step 1 — Engine `--mode` presets

> Coarse step. Depends on the (user-validated) M0 decisions. Refine before starting.

## Goal
D7: `--mode fast|accurate` maps to two fixed presets over the pipeline options.

## Spec refs
D7, §3.2, F-IN-7, NF-2.

## Depends on
M0-14 (and the user's own bench run), M0-11.

## Tasks
- A `PRESETS = {"fast": {...}, "accurate": {...}}` dict in `pipeline.py` from the recorded M0 results. If M0 concluded "single mode",
  skip M4b entirely and mark its steps `done (not needed)`.
- `--mode` default `fast`. The hidden bench flags override the preset.
- `madsister-engine estimate <audio> --mode <m>` **or** a `durationSec` field in a first progress event, to let the UI show the estimated
  duration (F-IN-7). Prefer the cheapest: a per-mode seconds-per-audio-second factor measured by the bench, stored next to the presets.

## Tests
- Unit: the preset resolution and override order; estimate computation.

## Verify
`cd engine && uv run pytest -m "not slow"`

## Commit message
`feat(engine): add fast and accurate transcription modes`
