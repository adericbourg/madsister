# madsister

Offline desktop app that turns a song into an editable, printable chord grid.

## Development

### Prerequisites

Checked on macOS (Apple silicon) with these versions:

- Node.js LTS (26.10) and pnpm 12 (12.8; CI pins 12).
- Rust stable (1.98) and a C++ toolchain (Xcode Command Line Tools / `build-essential`): Tauri, and NATTEN
  (section detection), which is built from source on both OSes.
- [uv](https://docs.astral.sh/uv/) (0.12): it installs Python 3.11 (`engine/.python-version`) by itself.
- ffmpeg on `PATH` (the engine decodes every input with it).
- Linux only: the [Tauri system packages](https://v2.tauri.app/start/prerequisites/#linux), same list as
  `.github/workflows/app.yml`:
  `libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev build-essential curl wget file`;
  for audio playback, `gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav` (see
  [M3-1](doc/roadmap/m3-step1-playback-cursor.md); not checked here).

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
The engine isn't bundled yet (M6): the built app still runs the engine of this checkout through `uv`.
