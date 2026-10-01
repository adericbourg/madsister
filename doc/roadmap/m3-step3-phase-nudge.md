# M3 · Step 3 — Downbeat phase nudge

> Coarse step. Refine before starting.

## Goal
F-PB-3: shift the bar phase by ±1 beat for the whole song, in one undoable action.

## Spec refs
F-PB-3, §2.1 known limits, §8 M3 "done when".

## Depends on
M3-1.

## Tasks
- Pure `shiftPhase(song, delta: 1 | -1)`: flatten all slots into a beat sequence with times (beat times interpolated inside each bar from
  its `startSec` to the next bar's `startSec`), shift the bar boundaries by one beat, re-group into bars of the song meter (same slot
  grouping rule as the engine: consecutive equal chords), recompute `startSec`. Sections: keep each section's first bar anchored to the
  shifted boundary. Meter overrides and bars without `startSec`: define the behaviour explicitly (probably: refuse with a message if the song
  has overrides — `ponytail:` comment).
- Toolbar buttons "Phase −1 / +1 beat".
- Also offer the shift **from the cursor bar onward** (not only the whole song): after an undetected 2-beat break bar the
  phase is wrong only from that point (see `BACKLOG.md`).

## Tests
- A song whose chords change on beat 2 of every bar → +1/−1 fixes it; round-trip −1 then +1 returns the original.
- "Done when" check on a bench song with a known phase error (take a GuitarSet take and shift its reference by one beat).

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): fix the downbeat phase in one action`
