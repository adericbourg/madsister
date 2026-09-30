# M4b · Step 3 — Mode picker and re-run

> Coarse step. Refine before starting.

## Goal
F-IN-7: pick the mode with its estimated duration; re-run `accurate` on an existing song without silently losing manual edits.

## Spec refs
F-IN-7.

## Depends on
M4b-1, M4b-2, M2-2.

## Tasks
- Import dialog: mode radio group with the estimated duration for this file.
- "Re-transcribe (accurate)" on a song with audio: run into a temporary output, then show a confirmation ("Replace the current grid?
  N manual edits will be lost" — count = slots without `confidence` that differ, cheap approximation). Replacing = one undoable history entry.

## Tests
- Component tests with the engine mocked: estimate shown; cancel on the confirmation keeps the song; confirm replaces and undo restores.

## Verify
`cd app && pnpm test`

## Commit message
`feat(app): choose transcription mode and re-run safely`
