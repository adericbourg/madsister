# M6 · Step 2 — Linux package

> Coarse step. Refine before starting. Needs a Linux machine or CI runner: build in GitHub Actions.

## Goal
M6 "done when" on Linux: AppImage/deb that installs on a clean machine and sets up the engine on first launch.

## Spec refs
§8 M6, D4, NF-3.

## Depends on
M6-1.

## Tasks
- `tauri build` with `deb` and `appimage` targets in a `release.yml` workflow. Release rules (user request, 2026-10-01):
  - every build of `main` publishes a **snapshot pre-release**; only the latest snapshot is kept (delete the previous one and its tag);
  - every **versioned build** (tag) publishes a release that is kept;
  - versions are **semver**. Proposed (the user had no preference, confirm before implementing): the version comes from tag `vX.Y.Z`,
    injected at build time; snapshots are `<last tag or 0.0.0>-snapshot.<run number>`. The platform list is decided with M6-3.
- Engine resolution in the packaged app (the M2-1 resolver's third branch): bundled `uv` binary as a Tauri sidecar + the `engine/` sources as
  resources; first launch runs `setup` with a progress screen.
- Notes from M6-1 ([packaged engine shape](m6-step1-engine-setup-script.md#packaged-engine-shape-recommendation-for-m6-2)):
  first launch = `uv sync --frozen` (default groups: `beats-madmom`, `chords-cnnlstm`) into the app data dir, then
  `madsister-engine setup` (`--sections` when the user opts in to all-in-one), reading its JSONL progress. madmom (git source +
  Cython) and NATTEN (macOS, sections) build from source: ship CI-built wheels instead, or first launch needs git and a C
  compiler. Add `uv` (MIT OR Apache-2.0) to `THIRD_PARTY.md`.
- Runtime deps: ffmpeg (deb `Depends`; AppImage: document or bundle a static ffmpeg — decide by size).

## Verify
Install the `.deb` in a clean container/VM (`ubuntu:24.04` + a desktop session, or at least launch the binary headless and run `setup`).

## Commit message
`build: package the app for Linux`
