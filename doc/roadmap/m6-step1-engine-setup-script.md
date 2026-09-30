# M6 · Step 1 — Engine setup

> Coarse step. Refine before starting (depends on which optional groups M0 kept).

## Goal
One command installs the engine environment and downloads the weights once (D4, D5, NF-1, NF-6).

## Spec refs
D4, D5, NF-1, NF-6, §11.4.

## Depends on
M0-14, M4b.

## Tasks
- `madsister-engine setup`: fetches every model/repo/weight the chosen presets need into `models_dir()`, with progress events; idempotent.
  Afterwards the engine must run with the network disabled (test by setting `HF_HUB_OFFLINE=1`/`TORCH_HOME` and no proxy).
- Decide the packaged engine shape: `uv` bundled + `uv sync --frozen` into the app data dir at first launch (smallest package), vs.
  a frozen venv in the package (~1–2 GB). Recommend the first; confirm with the user.
- Re-check the license aggregation note (§11.4) and `THIRD_PARTY.md` before anything is published.

## Tests
- Unit: idempotence (second run downloads nothing) with the downloader mocked.

## Verify
Fresh `MADSISTER_MODELS_DIR` → `setup` → transcribe offline.

## Commit message
`feat(engine): add one-shot setup for models`
