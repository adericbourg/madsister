# M5 · Step 1 — Engine `fetch` (yt-dlp)

> Coarse step. Refine before starting.

## Goal
`madsister-engine fetch <url> --out-dir <dir>` downloads the audio and emits its path as the `result` (F-IN-2).

## Spec refs
F-IN-2, §3.2, D5, §10 (yt-dlp breakage).

## Depends on
M0-1.

## Tasks
- Dependency `yt-dlp` (optional group `fetch`), used as a library (`yt_dlp.YoutubeDL`) with `format: bestaudio`, output template in
  `out-dir`, progress hook → `progress` events (`stage: "download"`). No ffmpeg post-processing: decode handles every format.
- Validate the URL scheme (`http`/`https` only) at this boundary.

## Tests
- Unit with `YoutubeDL` mocked (progress mapping, result path, error propagation). No network in tests.

## Verify
`cd engine && uv run pytest -m "not slow"`; one manual fetch of a CC-licensed video URL.

## Commit message
`feat(engine): fetch audio from a URL with yt-dlp`
