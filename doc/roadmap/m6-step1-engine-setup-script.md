# M6 · Step 1 — Engine setup

> Refined 2026-10-01 (second unattended run). Packaging itself (bundling `uv` and the engine in the app) is M6-2.

## Goal
One command downloads every model the engine needs, once; afterwards the engine runs with the network disabled
(D4, D5, NF-1, NF-6).

## Spec refs
D4, D5, NF-1, NF-6, §11.4.

## Depends on
M0-14. (M4b is skipped: single mode, D7.)

## Decisions
- What the models need (user decisions, README "Second unattended run"):
  - default combo madmom + Chord-CNN-LSTM: madmom ships its weights inside the package (nothing to download), Chord-CNN-LSTM
    needs its upstream repo (code + 5 weight files);
  - `--sections` (the all-in-one opt-in): all-in-one's 8-fold `harmonix-all` ensemble + htdemucs (all-in-one separates with it),
    both from Hugging Face;
  - `--all` adds the bench-only models: the BTC repo, and htdemucs for `--separate`.
- Each adapter exposes `prepare()` that downloads exactly what its `track`/`recognize` loads, using the same loader
  (`ensure_repo`, `allin1.models.load_pretrained_model`, `demucs.pretrained.get_model`): no separate list of files to drift.
  `pipeline.setup` picks the adapters through `_adapter`, so a missing dependency group fails with the `uv sync --group …` hint.
- Pinned repos are downloaded as GitHub archives (`<url>/archive/<sha>.tar.gz`, stdlib `urllib` + `tarfile`) instead of
  `git clone`: packaged installs don't need git for the models, and it is shorter than the clone + checkout it replaces.
  Existing clones under `models_dir()` are reused as is.
- Offline after setup needs nothing more: Hugging Face falls back to its cache when the network fails, and the default combo
  never touches the network. No `HF_HUB_OFFLINE` is forced (it would break `setup` itself).

## Contract
`madsister-engine setup [--sections] [--all]` → one `{"type":"progress","stage":"setup","pct":…}` line per model
(pct over the models), then `{"type":"result","path":"<models_dir()>"}`. Idempotent: a second run downloads nothing.

## Tests
- Fast (`test_cli.py`, no network): the default setup downloads the Chord-CNN-LSTM archive (a local `file://` archive) once,
  and a second run succeeds with the archive gone; `--sections` / `--all` call each adapter's `prepare()` in order, with one
  progress line per model. `test_models.py`: `ensure_repo` extracts the pinned archive without its top-level directory, once.
- CI (`engine-models`, Linux + macOS): `setup --all` runs before the slow tests, which then run behind an unreachable proxy.

## Verify
- `cd engine && uv run --no-sync pytest -m "not slow"`, `uv lock --check`.
- Fresh `MADSISTER_MODELS_DIR` → `setup` → `setup --all` → `setup --all` again → transcribe a GuitarSet take with
  `HTTPS_PROXY=http://127.0.0.1:9` (with and without `HF_HUB_OFFLINE=1`), default combo and all-in-one + BTC + `--separate`.

## Results (macOS arm64, 2026-10-01)
| Run | Downloads | Time |
|---|---|---|
| `setup` (default) | Chord-CNN-LSTM repo, 30 MB | 2 s |
| `setup --all` after it | + all-in-one ensemble and htdemucs (Hugging Face, 98 MB) + BTC repo (35 MB) | 16 s |
| `setup --all` again | nothing | 4 s (loads the models to check them) |

Total `models_dir()`: 163 MB. Offline transcription of `00_Rock1-130-A_comp_mic.wav` behind the unreachable proxy: default
combo OK (4 s), all-in-one + BTC + `--separate` OK (44 s), same with `HF_HUB_OFFLINE=1`. Control: with an empty models dir
the same proxy makes `setup` fail (`Connection refused`) and `hf_hub_download` raise `LocalEntryNotFoundError`, so the proxy
does block the network. The whole slow suite also passes behind it.

## Packaged engine shape (recommendation for M6-2)
Recommended: **bundled `uv` binary (Tauri sidecar) + `engine/` sources + `uv.lock` as resources; first launch runs
`uv sync --frozen` (default groups) into the app data dir, then `setup`**, with a progress screen. Smallest package
(uv ≈ 40 MB + sources), uv fetches a relocatable CPython, network needed once (consistent with D5).
Rejected: a frozen venv in the package: ~0.9 GB on disk with every group (measured `.venv`), per-platform, and venvs are
not relocatable by default.

**Blocker for a plain `uv sync` on a user's machine, to solve in M6-2:** two dependencies build from source.
madmom is a git dependency (pinned sha, weights in a git submodule) with Cython extensions → needs `git` and a C compiler;
NATTEN (macOS, `--sections` only) also builds with a C++ compiler. A clean desktop (and macOS without the Command Line Tools)
has neither. Fix: build those wheels in the release workflow per platform, ship them as resources (a few MB), and install
from `uv export --frozen` with those two entries pointing at the bundled wheels (`uv pip sync`); everything else is a wheel
from PyPI / the PyTorch CPU index.

Licenses (§11.4): with this shape the package bundles no weights at all (madmom's CC BY-NC-SA weights come with the madmom
wheel built in CI, so shipping that wheel *is* bundling them: aggregation under GPLv3 §5, keep the non-commercial notice in
`THIRD_PARTY.md` and the release notes). Everything `setup` downloads is listed in `THIRD_PARTY.md` (Chord-CNN-LSTM, BTC,
all-in-one, HTDemucs). M6-2 adds `uv` itself (MIT OR Apache-2.0).

## Commit message
`feat(engine): add one-shot setup for models`
