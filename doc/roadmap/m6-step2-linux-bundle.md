# M6 · Step 2 — Linux package

> Refined 2026-10-01 (second unattended run). Built in GitHub Actions (`.github/workflows/release.yml`).

## Goal
M6 "done when" on Linux: a deb and an AppImage that install on a clean machine and set up the engine on first launch.

## Spec refs
§8 M6, D4, D5, NF-3, NF-6, §11.4.

## Depends on
M6-1.

## Decisions
- **Packaged engine** (M6-1's recommendation): `uv` is a Tauri sidecar (`bundle.externalBin`, `binaries/madsister-uv-<triple>`,
  installed next to the app's binary; `<package>-uv` (`madsister-uv`, `madsister-snapshot-uv` for snapshots) so the debs never
  clash with a `/usr/bin/uv` nor with each other), and the `engine/`
  project (`pyproject.toml`, `uv.lock`, `.python-version`, `madsister_engine/`) a resource. Both live in
  `app/src-tauri/tauri.bundle.conf.json`, merged only when packaging (`--config`): in `tauri.conf.json` they would make every
  dev build and the `app` CI need the sidecar binary.
- **Resolution** (`engine.rs`, M2-1's third branch): `MADSISTER_ENGINE`, then the packaged engine
  `<app data>/engine-env/bin/madsister-engine` (only when built by `tauri build`: `!tauri::is_dev()`), then the dev
  `uv run --project <repo>/engine --no-sync`.
- **First launch** (and after each update): the env is stamped with the app version (`engine-env/.madsister-version`); when the
  stamp differs, the app shows the engine setup instead of the importer (editing stays available) and runs
  `setup-engine.sh` (embedded in the binary, run with `sh`, cancelled with the app): a synthetic `install` progress, then
  `uv sync --frozen --no-dev --no-editable` with the default groups (`beats-madmom`, `chords-cnnlstm`, `fetch`, `record`)
  into `UV_PROJECT_ENVIRONMENT=<app data>/engine-env`, then `madsister-engine setup` (its JSONL progress), then the stamp.
  Errors show uv's output and a Retry button. `--no-editable`: an AppImage mounts at a new path on every launch.
  `UV_PYTHON_PREFERENCE=only-managed`: uv's own CPython 3.11 (in `~/.local/share/uv/python`), the same everywhere. uv's cache
  stays in `~/.cache/uv`.
- **No compiler on first launch**: madmom (git source + Cython) is the only default dependency without a wheel. The release
  workflow builds it (`uv build --wheel` of the pinned sha, with its weights submodule) and bundles it as
  `engine/wheels/`; the script then syncs with `--no-install-package madmom` and installs the wheel with `uv pip install
  --no-deps` (the lock stays untouched). A local `tauri build` without that wheel builds madmom from git on first launch
  (needs git and a C compiler). NATTEN only matters for the all-in-one opt-in, which is installed on demand (see "Detect sections").
- **Runtime dependencies**: deb `Depends`: `ca-certificates` (HTTPS downloads at setup), `ffmpeg`, `gstreamer1.0-plugins-base`,
  `gstreamer1.0-plugins-good`, `gstreamer1.0-libav` (playback, M3-1), `libportaudio2` (recording, M5-2). The AppImage bundles
  GStreamer (`bundleMediaFramework`, +15–35 MB per Tauri's docs: playback is the AppImage's point) but not ffmpeg (a static one
  is ~80 MB, every distribution packages it) nor PortAudio: documented in the release notes.
- **Releases** (README "Second unattended run"): the version comes from tag `vX.Y.Z`, or `<last vX.Y.Z tag or 0.0.0>-snapshot.<run
  number>` on `main`, checked against semver and injected with `--config '{"version":…}'` (Cargo.toml keeps 0.1.0). A snapshot is
  a pre-release; the previous snapshots (release and tag) are deleted right *after* the new one is published, so a failed
  publication never leaves zero snapshots. Tag filter `v[0-9]+.[0-9]+.[0-9]+`: snapshot tags never trigger a build (and tags
  pushed with `GITHUB_TOKEN` never trigger workflows anyway). Built on `ubuntu-22.04` (oldest glibc). The workflow runs on
  every push without waiting for `app`/`engine`: its clean-install job gates the publication, and the next push replaces a bad
  snapshot. Never push a version tag from an agent: the user decides versions.
- **Clean-install check** (`linux-clean-install` job): installs the deb in `ubuntu:24.04` (no compiler, no git), runs
  `setup-engine.sh` as a non-root user against the installed (read-only) project, then transcribes a synthetic C major clip
  (ffmpeg `aevalsrc`) and checks it reads `C:maj`.

## Tests
- Rust (`engine.rs`): resolution order (override, packaged, dev); `setup_command` against a fake `uv` that logs its calls and
  writes a fake engine: events (install → setup → result), the stamp, `sync --frozen … --no-install-package madmom` then
  `pip install` of the bundled wheel.
- TS: `EngineSetup.test.tsx` (ready → children; first launch → progress per stage → children; failure → stderr + Retry),
  `engine.test.ts` (commands), `App.test.tsx` (mock).

## Verify
- `cd app && pnpm test && pnpm build && (cd src-tauri && cargo test && cargo clippy -- -D warnings)`.
- Local (macOS): `setup-engine.sh` with the real uv against a read-only copy of the project and a CI-like madmom wheel, then a
  transcription of the synthetic clip (`C:maj` everywhere); a packaged `.app` (`--config src-tauri/tauri.bundle.conf.json`)
  launched with no engine env sets it up by itself (stamp written after 20 s with a warm uv cache).
- CI: `release.yml` publishes the snapshot with the `.deb` and the `.AppImage`, the clean-install job passes, and a second push
  leaves a single snapshot.

## Results (2026-10-01, run 1, `36cb421`)
- `release.yml` green on the first push: snapshot pre-release `v0.0.0-snapshot.1` with `madsister_0.0.0-snapshot.1_amd64.deb`
  (46.7 MB: app, uv sidecar, engine sources, the 23 MB madmom wheel with its weights) and
  `madsister_0.0.0-snapshot.1_amd64.AppImage` (173 MB: WebKitGTK and GStreamer bundled).
- Durations: build job 9 min (Tauri build 7 min, madmom wheel 39 s); clean install in `ubuntu:24.04`: `apt install` of the deb
  42 s, then engine setup (CPython, `uv sync`, Chord-CNN-LSTM download) and the transcription together 32 s on the runner's
  network, transcription reading `C:maj`. The env takes ~680 MB (measured on macOS with the same groups).
- The next push (the commit recording these results) must leave a single snapshot: checked on the releases page.

## Detect sections
The default env has no all-in-one (NATTEN builds from source, torch is large). Ticking "Detect sections" runs
`setup-engine.sh ... sections` once (`setup_sections`): same script plus `--group beats-allinone` and `setup --sections`, marked by
`<env>/.madsister-sections` (= the app version). `uv sync` is exact, so the plain setup after an update drops the group, and the
marker with it: the next use reinstalls.

## Not done
- Check by hand (needs a desktop, a compiler and cmake): tick "Detect sections" in a packaged app, watch the one-time install, then
  the sections transcription.
- Check by hand (needs a desktop): install the deb on a desktop Ubuntu, launch, watch the setup screen, import an mp3, play it;
  same with the AppImage (with ffmpeg and libportaudio2 installed).

## Commit message
`feat(app): run the bundled engine on first launch`, `build: package the app for Linux`, `ci: publish snapshot and tagged releases`
