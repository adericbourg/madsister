# M4b · Step 2 — Benchmark both modes

> Coarse step. Refine before starting.

## Goal
M4b "done when": both modes run on `bench/`; `accurate` is kept only if it is measurably better in "edits needed".

## Spec refs
§8 M4b, §7.

## Depends on
M4b-1, M0-13.

## Tasks
- `bench/run.py --modes fast,accurate` on the user's bench (and GuitarSet). Record in `m0-results.md` (a "Modes" section) with the margin.
- If `accurate` isn't better: remove it from `PRESETS`, update D7 in the spec, and mark M4b-3 as "single mode: no picker, estimate only".

## Verify
The results table exists; `uv run pytest -m "not slow"` passes.

## Commit message
`docs: benchmark fast and accurate modes`
