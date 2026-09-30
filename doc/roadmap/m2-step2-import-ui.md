# M2 · Step 2 — Import audio UI

> Coarse step. Refine before starting.

## Goal
Drop an audio file (or pick it) → progress by stage → editable grid (F-IN-1, F-IN-4).

## Spec refs
F-IN-1, F-IN-4, NF-4, §8 M2.

## Depends on
M2-1.

## Tasks
- Drag & drop (Tauri v2 drag-drop event on the window) + "Import audio…" dialog with the extensions mp3, wav, flac, m4a, ogg.
  Other extensions are rejected with a message.
- Progress panel: stage name + progress bar (`<progress>` with a label), Cancel button. The UI stays usable (NF-4).
- On result: the output JSON is written next to the audio as `<stem>.madsister.json` (if it exists, ask before overwriting, or choose
  `<stem> (2)...`); load it with `parseSong`, reset history, set the current path.
- Errors: show the engine message and keep the current song.

## Tests
- Component tests with the engine module mocked: progress displayed, cancel calls `cancel`, result loads the song, error keeps the song.

## Verify
`cd app && pnpm test && pnpm build`; manual `pnpm tauri dev` with a real mp3 if the engine works.

## Commit message
`feat(app): import audio and transcribe it`
