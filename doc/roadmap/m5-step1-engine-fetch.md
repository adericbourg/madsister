# M5 · Step 1 — Engine `fetch` (yt-dlp)

## Goal
`madsister-engine fetch <url> --out-dir <dir>` downloads the audio and emits its path as the `result` (F-IN-2).

## Spec refs
F-IN-2, §3.2, D5, NF-1, §10 (yt-dlp breakage).

## Depends on
M0-1.

## Contract (what M5-3 relies on)
- Args: `fetch <url> --out-dir <dir>`. Only `http`/`https` URLs: anything else → `error` `not an http(s) URL: <url>`, exit 1,
  nothing downloaded.
- Events: `progress` with `stage: "download"`, `pct` 0–100, one line per new percentage (none while the size is unknown);
  then `result` with the absolute path of the downloaded file, `<out-dir>/<title>.<ext>` (the stem becomes the Song title
  when it is transcribed). The file is the best audio stream as is (webm/m4a/ogg…): `decode` reads any of them.
- Errors: yt-dlp's message (e.g. `ERROR: Unsupported URL: …`) as the `error` line, exit 1. Without the `fetch` group:
  `downloader yt-dlp needs the \`fetch\` dependency group: \`uv sync --group fetch\``.
- A playlist URL downloads its single video (`noplaylist`); an existing file of the same name is reused, not re-downloaded.

## Tasks
- Dependency group `fetch = ["yt-dlp"]`, imported lazily (`pipeline._adapter`, like the model groups); add it to the
  `engine-models` workflow sync so the lock stays exercised. `THIRD_PARTY.md`: yt-dlp (Unlicense).
- `madsister_engine/fetch.py`: `yt_dlp.YoutubeDL` with `format: bestaudio/best`, `outtmpl` in `out-dir`, `quiet` +
  `noprogress` (fd 1 stays JSON Lines), progress hook `downloaded_bytes / (total_bytes or total_bytes_estimate)` →
  `progress("download", pct)`. Result = `requested_downloads[0]["filepath"]`. No ffmpeg post-processing.
- Validate the URL scheme at this boundary. Network access happens only here (D5, NF-1).

## Tests
- `tests/test_cli.py`, `yt_dlp` faked via `sys.modules` (no network, no package needed in the `engine` workflow): progress
  mapping (unknown total skipped, estimate used, repeated percentages deduplicated), options, result path, scheme rejection
  before any download, error propagation.

## Verify
- `cd engine && uv run --no-sync pytest -m "not slow"`, `uv lock --check`,
  `uvx --from actionlint-py actionlint ../.github/workflows/engine-models.yml`.
- Manual: `uv sync --group fetch` then a fetch of a public-domain file to a temp dir outside the repo
  (done: `https://commons.wikimedia.org/wiki/File:Example.ogg` → `Example.ogg`, 105 kB, progress 0…100, stdout pure JSON Lines).

## Commit message
`feat(engine): fetch audio from a URL with yt-dlp`
