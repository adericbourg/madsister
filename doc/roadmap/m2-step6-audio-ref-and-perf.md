# M2 · Step 6 — Audio reference and performance check

> Coarse step. Refine before starting.

## Goal
The Song references its audio (`audio.path` + `sha256`), and M2's "done when" (< 2 min for an mp3 of ~4 min) is measured.

## Spec refs
§4 `audio`, NF-2, §8 M2.

## Depends on
M2-2.

## Tasks
- Engine writes `audio: {path, sha256}` (sha256 of the original input file, streamed with `hashlib`).
- App: on open, if the audio file is missing or its hash differs, show a non-blocking notice with "Locate audio…" (updates the path).
  (Hash check async so opening stays fast.)
- M0-14 already measured 19 s for a 4 min 16 s GuitarSet concatenation with the default combo ([m0-results](m0-results.md)):
  re-measure on a real ~4-min mp3 only.
- Measure `transcribe` wall time on a ~4-min song (a GuitarSet concatenation or a user file) with the default combo; record it in
  `m0-results.md`. If > 2 min, log a blocker with the stage breakdown (the progress events give it).

## Tests
- Engine unit test for the `audio` field; app test for the missing-audio notice.

## Verify
`cd engine && uv run pytest -m "not slow"`; `cd app && pnpm test`

## Commit message
`feat: reference source audio from the song file`
