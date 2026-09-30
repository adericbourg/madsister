# M3 · Step 1 — Playback with bar cursor

> Coarse step. Refine before starting.

## Goal
F-PB-1: play/pause the source audio; the bar being played is highlighted.

## Spec refs
F-PB-1, §10 (WebKitGTK `<audio>`: VERIFY), NF-5.

## Depends on
M2-6.

## Tasks
- `<audio>` with `convertFileSrc(audio.path)` (asset protocol; enable and scope it in the Tauri v2 config/capabilities).
  VERIFY playback of mp3/m4a in WKWebView and WebKitGTK (GStreamer codecs); record the result.
- Space = play/pause (only when not editing a cell). Current bar = last bar with `startSec <= currentTime` (binary search over the
  flattened bars, pure function). Highlight via a CSS class; `requestAnimationFrame` loop while playing. The edit cursor doesn't move.
- Bars without `startSec` (manually added) are skipped by the cursor.

## Tests
- Pure: `barAtTime(song, t)` including bars without `startSec` and times before the first bar.

## Verify
`cd app && pnpm test`; manual check in `pnpm tauri dev`.

## Commit message
`feat(app): play source audio with a bar cursor`
