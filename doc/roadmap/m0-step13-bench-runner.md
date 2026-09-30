# M0 · Step 13 — Benchmark runner

## Goal
`bench/run.py` evaluates every pipeline combination on the reference set and prints the §8 M0 table (metrics + time).

## Spec refs
§7 (metrics, "edits needed" is the primary metric), §8 M0, D6 (τ sweep), D7.

## Depends on
M0-11, M0-12.

## Tasks
- Combinations = {installed beat trackers} × {installed chord models} × {separate: no, yes}. Run each by calling `pipeline.transcribe`
  in-process (hidden options), with `has_sections=False` for GuitarSet. Wall time per file with `time.perf_counter()`
  (model loading included: that's the per-job cost of D2).
- Chord metrics: convert the predicted Song back to a timed `.lab` (bar `startSec` + beat times from the tracker output; keep those in
  memory rather than re-deriving them). Report `mir_eval.chord` WCSR for `majmin`,
  `sevenths`, `tetrads` on the quantized output (what the user gets).
- Downbeat F-measure: `mir_eval.beat.f_measure(ref_downbeats, est_downbeats)`.
- **Edits needed** (primary): for each reference beat, find the predicted slot sounding at that time and the reference slot; a reference
  slot counts as one edit if the predicted chord at its first beat differs (exact Harte match after normalization) or if it's
  split differently. Report the total and per 100 reference slots. Keep the definition in the docstring; it's the number the
  user feels. Unit-test it on two tiny hand-built Songs.
- Segmentation boundary F: N/A on GuitarSet (no sections); compute it only if the reference Song has > 1 section.
- τ sweep for add2/add4: `--add-tau 0.3,0.5,0.7,…`, reporting add* precision/recall at slot level. On GuitarSet it can only show
  false positives: report them.
- Output: `bench/results/<date>-<dataset>.csv` (one row per file × combo) and `.md` (mean per combo) — both committed (small).
- Resumable: skip (file, combo) rows already in the CSV (runs can take hours; the night may be interrupted).
- `--limit N` and `--combos` for quick runs.

## Tests
- Unit: edits-needed on hand-built Songs (0 edits for identical; 1 for one wrong chord; split difference counted).
- Smoke: `bench/run.py --limit 1 --combos <fastest>` completes.

## Verify
`uv run --project engine python bench/run.py --limit 1` then the unit tests.

## Commit message
`feat(bench): add benchmark runner with edits-needed metric`

## Out of scope
Plots.
