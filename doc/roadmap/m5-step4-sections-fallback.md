# M5 · Step 4 — Section detection (if not done yet)

> Coarse step. Probably already covered by M0-6 + M0-3. Check first; if so, mark it `done (covered by M0)`.

## Goal
F-ST-1: auto-detected labelled sections, with the single-section fallback.

## Spec refs
F-ST-1, §3.3 step 6.

## Depends on
M0-6.

## Tasks
- Only if all-in-one was blocked in M0: evaluate another structure option (e.g. a novelty-curve segmentation with librosa, labels
  "Section A/B" by clustering) and add it behind `--sections`. Otherwise nothing to do.

## Verify
Bench segmentation F on the user's bench (sections annotated).

## Commit message
`feat(engine): detect song sections` (only if work was needed)
