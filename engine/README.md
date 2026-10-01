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
