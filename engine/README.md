# madsister engine

Python CLI that turns audio into a chord grid (`*.madsister.json`). Contract: spec §3.2 — JSON Lines on stdout, exit code 0 on success.

```
uv sync --group beats-allinone --group chords-btc   # a beat tracker and a chord model (one command: --group replaces)
uv run madsister-engine transcribe song.mp3 --out song.madsister.json
```
The app (dev build) runs `uv run --project engine --no-sync madsister-engine` (plain `uv run` re-syncs and drops the model
groups), so sync once with `uv sync --group beats-madmom --group chords-cnnlstm` (+ `--group beats-allinone` for sections),
or set `MADSISTER_ENGINE` to another command line.
```
uv run pytest -m "not slow"   # fast tests (what CI runs)
uv run pytest -m slow         # needs the model dependency groups and weights
```
The `engine-models` CI workflow runs the slow tests with every model group, on Linux and macOS. Same locally:
```
uv sync --locked --group beats-madmom --group beats-allinone --group chords-btc --group chords-cnnlstm --group separate
uv run --no-sync pytest -m slow tests/test_model_imports.py   # seconds: each heavy dep imports and runs one native call
uv run --no-sync pytest -m slow                               # minutes: downloads weights on the first run
```
On Linux, torch and torchaudio come from the PyTorch CPU index (PyPI's are the CUDA builds), and NATTEN is built from
source (needs a C++ compiler).
