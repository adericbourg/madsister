# M2 · Step 5 — Per-bar meter override

> Coarse step. Refine before starting.

## Goal
F-ED-11: set a single bar to another meter (e.g. one 2/4 bar in a 4/4 song).

## Spec refs
F-ED-11, §4 `Bar.meter`, §11.3.

## Depends on
M1-6, M1-10.

## Tasks
- Command `setBarMeter(song, barRef, meter | null)`: resizes the bar's slots to the new beat count (truncate from the end, or extend the
  last slot); `null` or a meter equal to the song's removes the override.
- UI: shortcut + a small control in the bar's context; the bar shows its meter (e.g. `2/4`) when overridden, also in print.

## Tests
- Command tests (shrink, grow, reset); render shows the meter label.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): per-bar meter override`
