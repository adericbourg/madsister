# M6 · Step 2 — Linux package

> Coarse step. Refine before starting. Needs a Linux machine or CI runner: build in GitHub Actions.

## Goal
M6 "done when" on Linux: AppImage/deb that installs on a clean machine and sets up the engine on first launch.

## Spec refs
§8 M6, D4, NF-3.

## Depends on
M6-1.

## Tasks
- `tauri build` with `deb` and `appimage` targets in a `release.yml` workflow (on tag), artifacts uploaded to the run.
- Engine resolution in the packaged app (the M2-1 resolver's third branch): bundled `uv` binary as a Tauri sidecar + the `engine/` sources as
  resources; first launch runs `setup` with a progress screen.
- Runtime deps: ffmpeg (deb `Depends`; AppImage: document or bundle a static ffmpeg — decide by size).

## Verify
Install the `.deb` in a clean container/VM (`ubuntu:24.04` + a desktop session, or at least launch the binary headless and run `setup`).

## Commit message
`build: package the app for Linux`
