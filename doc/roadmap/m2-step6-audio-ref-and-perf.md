# M2 · Step 6 — Audio reference and performance check

## Goal
The Song references its audio (`audio.path` + `sha256`), and M2's "done when" (< 2 min for an mp3 of ~4 min) is measured.

## Spec refs
§4 `audio`, NF-2, §8 M2.

## Depends on
M2-2.

## Tasks
- Engine (`pipeline.transcribe`): writes `audio: {path, sha256}`: the absolute path of the input file and its sha256, streamed
  with `hashlib.file_digest`.
- Engine (`pipeline.to_song`, shared with the bench): fewer than 2 beats from the tracker (all-in-one returned 1 beat on a
  GuitarSet take, m0-results) → `ValueError("could not find beats in this audio")`, which the CLI turns into an `error` line
  (was a `StatisticsError` crash).
- Rust: `audio_sha256(path) -> Option<String>` Tauri command (`sha2`, already in `Cargo.lock`), streamed, run off the main
  thread (`#[tauri::command(async)]`); `None` when the file can't be read. Hashing in Rust keeps the fs plugin scope as is
  (`$APPCONFIG/*`, `*.madsister.json`): the webview never reads audio files.
- App: whenever `song.audio` changes (open, import, locate, undo), hash the file asynchronously after the song is shown. Missing
  → `Audio file not found: <path>`, different hash → `Audio file has changed since the transcription: <path>`, as a
  non-blocking `role="status"` notice with a "Locate audio…" button: dialog (audio filters) → hash → `audio.path` + `sha256`
  updated as one history entry. A stale result (song changed meanwhile) is ignored.
- Measure `transcribe` wall time on a ~4-min mp3 (GuitarSet concatenation, the user has no file yet) with the default combo;
  record it in `m0-results.md`. If > 2 min, log a blocker with the stage breakdown (the progress events give it).

## Tests
- Engine: `test_main_of_transcribe_…` asserts the `audio` field (path + sha256); `test_main_when_adapter_fails_finds_no_beats_…`
  covers the 1-beat tracker output.
- Rust: `audio_sha256` on a known vector, and on a missing file.
- App: `App_whenOpeningASongWithMissingOrChangedAudio_showsANoticeAndLocatesTheAudio` (missing notice, locate updates
  path + hash, undo restores the old reference, changed-hash notice).

## Check by hand
- Open a transcribed song, move its mp3: the notice appears; "Locate audio…" on the moved file clears it and marks the song dirty.

## Verify
`cd engine && uv run --no-sync pytest -m "not slow"`;
`cd app && pnpm test && pnpm build && (cd src-tauri && cargo test && cargo clippy -- -D warnings)`

## Commit message
`fix(engine): report an error when no beats are found`, then `feat: reference source audio from the song file`
