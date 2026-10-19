# Contributing to madsister

## Prerequisites

Linux is the main development platform (CI runs there); macOS works too, and nothing ties the toolchain to either. Versions
below were last checked on macOS (Apple silicon):

- Rust stable (1.98) and a C++ toolchain (`build-essential` on Debian/Ubuntu, Xcode Command Line Tools on macOS): the
  app, and madmom, built from its git source.
- [uv](https://docs.astral.sh/uv/) (0.12): it installs Python 3.11 (`engine/.python-version`) by itself.
- ffmpeg on `PATH` (the engine and the app's player decode every input with it).
- Linux: the packages Slint's winit backend (fonts, keyboard layouts) and cpal's ALSA output build against, same list as
  `.github/workflows/app.yml` (Debian/Ubuntu names; other distributions have equivalents):
  `libfontconfig1-dev libxkbcommon-dev libasound2-dev`.

## Engine setup (once)

```sh
cd engine
uv sync --group beats-madmom --group chords-cnnlstm --group beats-allinone --group fetch --group record  # drop beats-allinone to skip section detection
```

Each `uv sync` installs only the groups it names, and a plain `uv sync` or `uv run` removes them: use `uv run --no-sync`
afterwards. Weights and model repos download on first use into `MADSISTER_MODELS_DIR` (default
`~/.cache/madsister/models`). On Linux, torch comes from the PyTorch CPU index. More in [engine/README.md](engine/README.md).

## Run

```sh
cd app
cargo run                    # the app
cargo run -- print FILE...   # `madsister print`, see the README
```

The app runs the engine with `uv run --project <repo>/engine --no-sync madsister-engine`; set `MADSISTER_ENGINE` to another
command line (split on whitespace) to override it.

## Test

| Command | Where | CI workflow |
|---|---|---|
| `uv run --no-sync pytest -m "not slow"` | `engine/` | `engine` (Linux) |
| `uv run --no-sync pytest -m slow` | `engine/` | `engine-models` (Linux; macOS and Windows weekly) |
| `cargo test` then `cargo clippy --all-targets -- -D warnings` | `app/` | `app` (Linux; Windows nightly) |

The slow tests need every model group (`engine/README.md` has the sync line) and download weights on the first run.
The model benchmark is described in [bench/README.md](bench/README.md).

## Package locally

[app/README.md](app/README.md#packaging) builds the deb and the AppImage with cargo-packager, the `uv` sidecar included.
The dmg and the Windows installer come from `.github/workflows/release.yml` (its `macos` and `windows` jobs show the
steps): on Windows, ffmpeg is a sidecar too, since nothing else installs it (a static build, BtbN's `win64-gpl`: a shared
one needs its DLLs), added by `app/release-config.py`.

On first launch (and after each update) the packaged app installs its engine into its data dir (`engine-env`) with
`app/setup-engine.py` (run by the bundled uv's managed Python), then downloads the models.

## Releases

`.github/workflows/release.yml` builds the Linux deb and AppImage (on Ubuntu 26.04, with the madmom wheel and the `uv`
sidecar) and the macOS arm64 dmg (ad-hoc signed), installs each on a clean system to set up its engine and transcribe a clip,
then publishes (and the `homebrew` job commits the matching cask to
[adericbourg/homebrew-tap](https://github.com/adericbourg/homebrew-tap); the `HOMEBREW_TAP_TOKEN` secret is a fine-grained
token with Contents read/write on that repository only):

- every night (03:00 UTC), if `main` got a commit in the last 25 hours: a pre-release `v<last vX.Y.Z tag or 0.0.0>-snapshot.<run number>`;
  the previous snapshot (release and tag) is deleted once the new one is published, so only the latest is kept;
- a manual run on `main` (Actions > release > Run workflow) with a `patch`, `minor` or `major` bump of the last `vX.Y.Z` tag:
  the release `X.Y.Z`, kept, and its tag is created on the run's commit. The version is injected with
  `cargo packager --config` (`app/release-config.py`), so `Cargo.toml` isn't bumped.

A release is the package `madsister`, a snapshot the package `madsister-snapshot`: the workflow overrides the product
name, the binary's name, the identifier and the sidecar name (`<package>-uv`) at build time (`app/release-config.py`), so
the two share no file and no data dir.
The `apt` job then rebuilds the signed apt repository on GitHub Pages (a single suite, `stable`, with the latest release and
the latest snapshot) with `apt-ftparchive`, signed by the key in the `APT_GPG_PRIVATE_KEY` secret. Pages must be set to the
"GitHub Actions" source.

Releases are on the [releases page](https://github.com/adericbourg/madsister/releases).
