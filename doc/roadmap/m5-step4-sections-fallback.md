# M5 · Step 4 — Section detection (if not done yet)

> Checked 2026-10-01: covered, no work needed. all-in-one (M0-6) gives labelled sections, `quantize.sections_from_segments`
> (M0-3) snaps them to bars and falls back to a single "Song" section, and the import dialog's "Detect sections (slow)"
> checkbox (M2-2) exposes it as the opt-in the user chose. A cheaper detector stays possible later if the slow path hurts.

## Goal
F-ST-1: auto-detected labelled sections, with the single-section fallback.

## Spec refs
F-ST-1, §3.3 step 6.

## Depends on
M0-6.

## Tasks
- M0-14: all-in-one works but is in no preset (too slow for NF-2), so the default gives a single section. Decide with the user:
  all-in-one as a slow opt-in, or the cheaper option below.
- Only if all-in-one was blocked in M0: evaluate another structure option (e.g. a novelty-curve segmentation with librosa, labels
  "Section A/B" by clustering) and add it behind `--sections`. Otherwise nothing to do.

## Verify
Bench segmentation F on the user's bench (sections annotated).

## Commit message
`feat(engine): detect song sections` (only if work was needed)
