# M2 · Step 2 — Import audio UI

## Goal
Drop an audio file (or pick it) → progress by stage → editable grid (F-IN-1, F-IN-4).

## Spec refs
F-IN-1, F-IN-4, NF-4, §8 M2.

## Depends on
M2-1.

## Tasks
- `ui/Importer.tsx` (rendered by `App`, under the file toolbar):
  - "Import audio…" button (dialog filtered on mp3, wav, flac, m4a, ogg) and drag & drop on the window
    (`getCurrentWebview().onDragDropEvent`, `drop` payload; only the first dropped file is used). Other extensions are
    rejected with a message. A drop while a job runs is ignored (the button is disabled).
  - "Detect sections (slow)" checkbox, off by default (user decision: all-in-one is a slow opt-in). It passes
    `--beats allinone` (≈ 6 min for a 4-min song, vs ≈ 20 s with the default madmom + CNN-LSTM, which yields one "Song" section).
    Rust: the `transcribe` command gets a `sections: bool`; the argv is built by `engine::transcribe_args` (unit-tested).
  - Progress panel: stage name in a `role="status"` live region, `<progress>` labelled by the stage, Cancel button
    (calls `cancel(jobId)`). Nothing else is blocked while it runs (NF-4). "Starting…" until the first progress event.
  - Errors (engine `error` event, or the engine couldn't start): message in a `role="alert"`, stderr in a `<details>`.
    The current song is kept.
- Output path: `<audio dir>/<stem>.madsister.json`. If it exists, a confirm asks before overwriting; "no" cancels the import
  (the user can rename the existing file). Needs `fs:allow-exists` on `$HOME/**/*.madsister.json`.
- On result: `App.openSong(path)` (same path as Open: "Discard unsaved changes?" if dirty, `parseSong`, history reset,
  current path, recent list). The confirm is asked at result time, so edits made during the transcription are covered too.
- Meter: always `null` (auto) here; forcing it is M2-3.

## Tests
- `Importer.test.tsx` (engine module, dialog, fs and webview mocked): drop → transcribe args (with the sections flag) →
  progress shown → result reported; cancel calls `cancel`; error shows message + stderr, start failure shown;
  unsupported extension rejected; existing output + "no" doesn't start.
- `App.test.tsx`: a result loads the song; a later error keeps it.
- Rust: `transcribe_args` with/without meter and sections.

## Verify
`cd app && pnpm test && pnpm build && (cd src-tauri && cargo test && cargo clippy -- -D warnings)`.

## Check by hand
With the dev env synced (`uv sync --project engine --group beats-madmom --group chords-cnnlstm`, `+ --group beats-allinone`
for sections), `cd app && pnpm tauri dev`:
- [ ] Drag a real mp3 from Finder onto the window → the stage and the bar move, the grid appears (< 2 min), the title shows
      the song, `<stem>.madsister.json` is next to the mp3 and in Recent.
- [ ] Drop it again → the overwrite confirm appears; "Cancel" doesn't start anything.
- [ ] Drop a `.txt` → rejected with a message.
- [ ] "Import audio…" with "Detect sections (slow)" checked → several sections in the grid.
- [ ] Cancel during a run → the panel disappears, no engine process is left (`ps aux | grep madsister-engine`).
- [ ] Edit the grid while it runs (NF-4); with unsaved edits, the result asks "Discard unsaved changes?".
- [ ] VoiceOver announces the stage changes.
- [ ] Audio outside `$HOME` (e.g. `/Volumes/…`): the JSON can't be checked/read (fs scope) — an error is shown, nothing crashes.

## Commit message
`feat(app): import audio and transcribe it`
