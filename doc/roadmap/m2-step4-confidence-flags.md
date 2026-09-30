# M2 · Step 4 — Low-confidence flags

> Coarse step. Refine before starting.

## Goal
F-ED-8 end to end: engine confidences flagged in the grid, cleared on edit, and quick navigation between flags.

## Spec refs
F-ED-8, §3.3 step 3 (confidence), D6 (heuristic caps confidence).

## Depends on
M2-2 (M1-9 already styles flags; M1-6 already clears them on `setChord`).

## Tasks
- Threshold setting (default 0.5) in the preferences.
- `F8` / `Shift+F8`: jump to the next/previous flagged slot (the core of "correct the draft fast"). Status line: "N chords to review".
- Check the flag disappears on any chord edit, including paste over it.

## Tests
- Component: next/previous flag navigation, counter updates after an edit.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): navigate and review low-confidence chords`
