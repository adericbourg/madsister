# madsister

Offline desktop app that turns a song into an editable, printable chord grid.

## Install (Debian, Ubuntu)

Add the apt repository once, and `apt upgrade` then keeps madsister up to date:

```sh
sudo curl -fsSLo /etc/apt/keyrings/madsister.asc https://www.dericbourg.net/madsister/key.asc
sudo tee /etc/apt/sources.list.d/madsister.sources <<'EOF'
Types: deb
URIs: https://www.dericbourg.net/madsister
Suites: stable
Components: main
Signed-By: /etc/apt/keyrings/madsister.asc
EOF
sudo apt update && sudo apt install madsister
```

- Two packages: `madsister` follows the releases; `madsister-snapshot` follows every push to `main` (unreleased, may be
  broken). Install either or both: the snapshot has its own command (`madsister-snapshot`), launcher and data dir, so
  it never touches the stable app's settings, and it downloads its own engine.
- amd64 only. The first launch downloads the engine (Python, its libraries and the models), once, and again after an update.
- The `.deb` and the AppImage are also on the [releases page](https://github.com/adericbourg/madsister/releases).
- For non-commercial use only: the engine includes madmom's CC BY-NC-SA 4.0 model weights (see `THIRD_PARTY.md`).
- Uninstall: `sudo apt remove madsister` (or `madsister-snapshot`); to remove the repository, delete
  `/etc/apt/sources.list.d/madsister.sources` and `/etc/apt/keyrings/madsister.asc`.

## Install (macOS, Apple silicon)

Download `madsister_<version>_aarch64.dmg` from the [releases page](https://github.com/adericbourg/madsister/releases) and
copy the app to `/Applications`. It isn't signed or notarized: run `xattr -dr com.apple.quarantine /Applications/madsister.app`
once, or Gatekeeper blocks it.

- `madsister-snapshot_<version>_aarch64.dmg` (the latest pre-release) installs next to `madsister`, with its own data dir.
- Same first launch as on Linux (the engine and the models download once); ffmpeg must be on `PATH` (`brew install ffmpeg`).
  Section detection builds NATTEN from source on demand, which needs the Xcode Command Line Tools.

## Development

### Prerequisites

Linux is the main development platform (CI runs there); macOS works too, and nothing ties the toolchain to either. Versions
below were last checked on macOS (Apple silicon):

- Node.js LTS (26.10) and pnpm 12 (12.8; CI pins 12).
- Rust stable (1.98) and a C++ toolchain (`build-essential` on Debian/Ubuntu, Xcode Command Line Tools on macOS): Tauri,
  and NATTEN (section detection), which is built from source on every OS.
- [uv](https://docs.astral.sh/uv/) (0.12): it installs Python 3.11 (`engine/.python-version`) by itself.
- ffmpeg on `PATH` (the engine decodes every input with it).
- Linux: the [Tauri system packages](https://v2.tauri.app/start/prerequisites/#linux), same list as
  `.github/workflows/app.yml` (Debian/Ubuntu names; other distributions have equivalents):
  `libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev build-essential curl wget file`;
  for audio playback, `gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav` (see
  [M3-1](doc/roadmap/m3-step1-playback-cursor.md)).

### Engine setup (once)

```sh
cd engine
uv sync --group beats-madmom --group chords-cnnlstm --group beats-allinone --group fetch --group record  # drop beats-allinone to skip section detection
```

Each `uv sync` installs only the groups it names, and a plain `uv sync` or `uv run` removes them: use `uv run --no-sync`
afterwards. Weights and model repos download on first use into `MADSISTER_MODELS_DIR` (default
`~/.cache/madsister/models`). On Linux, torch comes from the PyTorch CPU index. More in [engine/README.md](engine/README.md).

### Run

```sh
cd app
pnpm install
pnpm tauri dev
```

The app runs the engine with `uv run --project <repo>/engine --no-sync madsister-engine`; set `MADSISTER_ENGINE` to another
command line (split on whitespace) to override it.

### Test

| Command | Where | CI workflow |
|---|---|---|
| `uv run --no-sync pytest -m "not slow"` | `engine/` | `engine` (Linux) |
| `uv run --no-sync pytest -m slow` | `engine/` | `engine-models` (Linux + macOS), weekly too |
| `pnpm test` then `pnpm build` (type check) | `app/` | `app` |
| `cargo test` then `cargo clippy -- -D warnings` | `app/src-tauri/` | `app` |

The slow tests need every model group (`engine/README.md` has the sync line) and download weights on the first run.
The model benchmark is described in [bench/README.md](bench/README.md).

### Package locally

```sh
cd app
pnpm tauri build
```

On macOS this produces `app/src-tauri/target/release/bundle/macos/madsister.app` (~4.5 MB) and
`bundle/dmg/madsister_<version>_aarch64.dmg`. The DMG step drives Finder through AppleScript and fails ("AppleEvent timed
out") when the terminal lacks the Automation permission; `pnpm tauri build --bundles app` builds the `.app` alone.
Such a build still runs the engine of this checkout through `uv`, like `pnpm tauri dev`.

To bundle the engine as the releases do, add the `uv` sidecar (any uv ≥ 0.12, named after the Rust target triple) and merge
`src-tauri/tauri.bundle.conf.json`:

```sh
cd app
mkdir -p src-tauri/binaries
cp "$(which uv)" "src-tauri/binaries/madsister-uv-$(rustc -vV | sed -n 's/^host: //p')"
pnpm tauri build --config src-tauri/tauri.bundle.conf.json      # Linux: --bundles deb,appimage
```

On first launch (and after each update) the packaged app installs its engine into its data dir (`engine-env`) with
`src-tauri/setup-engine.sh`, then downloads the models. Without the madmom wheel the release workflow bundles, that first
launch builds madmom from git: it needs git and a C compiler. The deb depends on ffmpeg, the GStreamer plugins and
`libportaudio2`; the AppImage bundles GStreamer but needs ffmpeg and `libportaudio2` on the system.

### Releases

`.github/workflows/release.yml` builds the Linux deb and AppImage (on Ubuntu 22.04, with the madmom wheel and the `uv`
sidecar) and the macOS arm64 dmg (ad-hoc signed), installs each on a clean system to set up its engine and transcribe a clip,
then publishes:

- every push to `main`: a pre-release `v<last vX.Y.Z tag or 0.0.0>-snapshot.<run number>`; the previous snapshot (release and
  tag) is deleted once the new one is published, so only the latest is kept;
- a manual run on `main` (Actions > release > Run workflow) with a `patch`, `minor` or `major` bump of the last `vX.Y.Z` tag:
  the release `X.Y.Z`, kept, and its tag is created on the run's commit. The version is injected with `tauri build --config`,
  so `tauri.conf.json` and `Cargo.toml` aren't bumped.

A release is the package `madsister`, a snapshot the package `madsister-snapshot`: the workflow overrides `productName`,
`mainBinaryName`, `identifier` and the sidecar name (`<package>-uv`) at build time, so the two share no file and no data dir.
The `apt` job then rebuilds the signed apt repository on GitHub Pages (a single suite, `stable`, with the latest release and
the latest snapshot) with `apt-ftparchive`, signed by the key in the `APT_GPG_PRIVATE_KEY` secret. Pages must be set to the
"GitHub Actions" source.

Releases are on the [releases page](https://github.com/adericbourg/madsister/releases).
