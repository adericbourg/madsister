# M5 · Step 3 — URL and recording in the app

> Refined on 2026-10-01 from the M5-1/M5-2 contracts and the M2 bridge.

## Goal
M5 "done when": a URL and a recording both yield a grid.

## Spec refs
F-IN-2, F-IN-3, F-IN-4, §3.2, §8 M5.

## Depends on
M5-1, M5-2, M2-2.

## Tasks
- Bridge (`engine.rs`, `lib.rs`): one Tauri command per engine verb, sharing `Jobs::spawn`:
  - `fetch(url, outDir, onEvent)` → `fetch <url> --out-dir <dir>`.
  - `record(outPath, onEvent)` → `record --out <file.wav>`, spawned with a piped stdin (a null stdin is an EOF: the engine
    would stop at once). Creates the parent directory first (the engine doesn't).
  - `stop(jobId)`: writes `stop\n` to the job's stdin: the engine stops gracefully and writes the WAV (then `result`).
    `cancel(jobId)` stays a SIGKILL of the process group: fetch, transcribe, and aborting a recording without keeping it
    (the killed WAV has no valid header; it stays in `sources/`).
  - `Progress` gets an optional `elapsedSec` (record), serialized only when present.
- Sources live in the app data dir (`appDataDir()/sources`, macOS `~/Library/Application Support/<id>/sources`, Linux
  `~/.local/share/<id>/sources`); the Song JSON goes next to the audio, `<stem>.madsister.json`. Recordings are named
  `Recording <UTC date>T<time>.wav`. fs scope: `$APPDATA/sources/*.madsister.json` added to read/write/exists
  (`$HOME/**` doesn't match the dot dir `~/.local` on Linux).
- `ui/Importer.tsx`:
  - "Audio URL" input + "Fetch": only `http(s)://` URLs (else an alert, nothing started) → `download` progress → on result,
    the normal transcription flow on the downloaded file (meter, sections, overwrite confirm). The downloaded file isn't
    checked against the import extensions (yt-dlp keeps the original format, e.g. webm; the engine decodes it).
  - "Record": status "Recording m:ss" from `elapsedSec`, a Stop button (→ `stop`), then the transcription of the WAV.
  - Cancel at every stage (download, recording, transcription) kills the current job; nothing follows.
  - All the start buttons are disabled while a job runs. Status in the existing `role="status"` live region.

## Tests
- Rust: `parse_line` with `elapsedSec`; `fetch_args`/`record_args`; integration (`tests/fake-engine.sh` `record`
  scenario): `stop` → the fake reads `stop` on stdin → result; spawn without stdin unchanged.
- vitest: `engine.ts` (`fetchAudio`, `record`, `stop`); `Importer.test.tsx` with the engine and path mocked: URL flow
  (download → transcription → result), cancel during download and during transcription, non-http URL rejected; record flow
  (elapsed time, Stop → transcription), cancel during recording.

## Verify
`cd app && pnpm test && pnpm build && (cd src-tauri && cargo test && cargo clippy -- -D warnings)`; one real fetch through the
bridge (`https://commons.wikimedia.org/wiki/File:Example.ogg`).

## Check by hand
With the dev env synced (root README "Development", including `--group fetch --group record`), `cd app && pnpm tauri dev`:
- [ ] Paste a YouTube URL, Fetch → `download` progress, then the transcription stages, then the grid; the files are in
      `~/Library/Application Support/<id>/sources/` and the song is in Recent.
- [ ] Fetch the same URL again → the overwrite confirm appears.
- [ ] Cancel during the download, and during the transcription → the panel goes away, no engine process left.
- [ ] Record → macOS asks for the microphone (for the terminal running `pnpm tauri dev`); the timer counts; Stop → the
      transcription runs, the grid appears, the WAV plays back.
- [ ] Record then Cancel → nothing is transcribed, no engine process left.
- [ ] VoiceOver announces "Recording 0:05"-style updates and the stages.
- [ ] A fetched webm/opus file transcribes, but playback only works for mp3/wav/flac/m4a/ogg (`read_audio` list).

## Commit message
`feat(app): import from URL and from the microphone`
