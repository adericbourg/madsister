# madsister engine

Python CLI that turns audio into a chord grid (`*.madsister.json`). Contract: spec §3.2 — JSON Lines on stdout, exit code 0 on success.

```
uv sync
uv run madsister-engine transcribe song.mp3 --out song.madsister.json
uv run pytest -m "not slow"   # fast tests (what CI runs)
uv run pytest -m slow         # needs the model dependency groups and weights
```
