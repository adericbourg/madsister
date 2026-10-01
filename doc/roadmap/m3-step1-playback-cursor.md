# M3 · Step 1 — Playback with bar cursor

## Goal
F-PB-1: play/pause the source audio; the bar being played is highlighted.

## Spec refs
F-PB-1, §4 `startSec`, §10 (WebKitGTK `<audio>`: VERIFY), NF-5.

## Depends on
M2-6.

## Decisions
- **No asset protocol.** WebKitGTK plays media through GStreamer, which has no source element for Tauri's `asset://` scheme:
  `<audio src={convertFileSrc(path)}>` fails on Linux with `NotSupportedError` (see `m0-results.md` VERIFY). Blob URLs work on
  both webviews. So a Rust command `read_audio(path)` returns the file's bytes (`tauri::ipc::Response`, raw `ArrayBuffer`), the
  app wraps them in a `Blob` typed from the extension (AVFoundation wants a MIME type) and plays its object URL. No
  `assetProtocol`, no `protocol-asset` feature, no scope, CSP unchanged (`null`). `read_audio` reads outside the fs scope, so it
  only accepts the audio extensions of the "Locate audio…" dialog.
  `ponytail:` the whole file sits in memory (~10 MB for a 4-min mp3, ~40 MB as wav); stream it if long recordings matter.
- **Space** = play/pause, from the grid only. It was unmapped (`keyToCommand` treats it as non-printable) and the chord input
  stops key propagation, so typing in a cell is unaffected. Outside the grid, Space keeps its native meaning (press the focused
  button). Listed in the help dialog.
- **`barAtTime(song, t)`** (pure, `model/playback.ts`): the bar with the greatest `startSec <= t`, bars without `startSec`
  skipped, `null` before the first timed bar. A linear scan rather than a binary search: moving sections leaves `startSec`
  out of order, and a song has ~100 bars.
- Player state lives in `ui/usePlayer.ts` (used by `Editor`): `{ audio element props, isReady, isPlaying, time, error, toggle }`.
  `time` is refreshed by a `requestAnimationFrame` loop while playing, rounded to 0.1 s so the grid re-renders ~10×/s, not every frame.
- Transport: a "Playback" toolbar (only for songs with `audio`): Play/Pause button (its text is its name), current time `m:ss`,
  an error text when the file can't be loaded or played (`play()` rejection, `<audio>` `error`: missing GStreamer plugins).
- Highlight: `.bar.is-playing` = a thick green bar under the bar (shape, not colour only; distinct from the focus outline and
  the selection background), removed in print. The edit cursor doesn't move.

## Tests
- `barAtTime`: before the first bar, inside/at a bar start, after the last bar, bars without `startSec`, unordered sections.
- `keyToCommand`: Space → `play`.
- `Editor` with a mocked audio element: Play toggles to Pause, the highlighted bar follows the mocked `currentTime`, Space pauses,
  a song without audio has no transport.
- Rust: `read_audio` reads an audio file, refuses other extensions.

## Linux runtime dependencies (for M6 packaging)
`<audio>` in WebKitGTK needs GStreamer: `gstreamer1.0-plugins-base` (wav, ogg), `gstreamer1.0-plugins-good` (mp3, flac),
`gstreamer1.0-libav` (m4a/AAC); plus an audio sink (`autoaudiosink` from -good). The AppImage needs
`bundle.linux.appimage.bundleMediaFramework: true`; deb packages should depend on these plugins.

## Check by hand
- macOS (`pnpm tauri dev`): import or open a transcribed song; play an mp3, an m4a and a wav: sound plays, the green bar marker
  follows the music bar by bar, Pause stops it, the edit cursor stays where it was.
- Linux: same with the GStreamer plugins above installed; without `gstreamer1.0-libav`, an m4a shows the error text instead of failing silently.
- Space in the grid toggles playback; Space while typing a chord does nothing special; Space on a focused toolbar button presses it.
- Manually inserted bars (no `startSec`) are skipped by the marker; moving the audio file then "Locate audio…" reloads it.
- Print preview shows no playback marker.

## Verify
`cd app && pnpm test && pnpm build && (cd src-tauri && cargo test && cargo clippy -- -D warnings)`

## Commit message
`feat(app): play source audio with a bar cursor`
