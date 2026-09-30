# M3 · Step 2 — Click to seek

> Coarse step. Refine before starting.

## Goal
F-PB-2: clicking a bar (or a keyboard shortcut on the cursor bar) seeks the audio to its `startSec`.

## Spec refs
F-PB-2.

## Depends on
M3-1.

## Tasks
- Click on a bar = place the edit cursor **and** seek (if it has `startSec`). Keyboard: `Mod+Space` = play from the cursor bar.

## Tests
- Component test with a mocked audio element: `currentTime` set to the bar's `startSec`.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): seek audio from the grid`
