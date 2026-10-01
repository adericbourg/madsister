# M3 · Step 2 — Click to seek

## Goal
F-PB-2: clicking a bar (or a keyboard shortcut on the cursor bar) seeks the audio to its `startSec`.

## Spec refs
F-PB-2.

## Depends on
M3-1.

## Decisions
- **Click = place the edit cursor and seek**, when the bar has a `startSec`; bars without one (inserted by hand) only get the
  cursor. A click never starts or stops playback: paused stays paused (the green marker moves to the bar), playing keeps
  playing from the new position.
- **`Shift+Space` = play from the cursor bar** (seek, then play if paused), not `Mod+Space`: `⌘Space` is Spotlight on macOS
  and `Ctrl+Space` switches the input source, so the webview never sees them. On a bar without `startSec` it does nothing
  and says so in the status notice. Listed in the help dialog.
- `usePlayer` gains `seek(sec)`: sets the element's `currentTime` and the hook's `time` (rounded like the rAF loop), so the
  highlight moves even while paused (the rAF loop only runs while playing).

## Tests
- `keyToCommand`: Shift+Space → `playFromCursor`.
- `Editor` with a mocked audio element (`currentTime` getter/setter): a click while paused seeks and moves the highlight without
  playing; a click on a bar without `startSec` doesn't seek; Shift+Space plays from the cursor bar; a click while playing keeps playing.

## Check by hand
- `pnpm tauri dev`, a transcribed song: click bars while paused and while playing; Shift+Space on a bar; Shift+Space on a manually
  inserted bar does nothing.

## Verify
`cd app && pnpm test && pnpm build`

## Commit message
`feat(app): seek audio from the grid`
