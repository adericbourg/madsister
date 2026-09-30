# M0 · Step 9 — Harmonic stem (Demucs)

## Goal
`madsister_engine/separate.py`: `harmonic_stem(wav_path, out_dir) -> Path`, the mix without vocals and drums (bass + other),
mono 44.1 kHz. Used as an optional pre-processing before chord recognition, and by the add2/add4 heuristic.

## Spec refs
§2.1 (Demucs row), §3.3 step 5b, D7.

## Depends on
M0-4.

## Tasks
- Dependency `demucs` (optional group `separate`). Check the current Python API with Context7 (`demucs.api.Separator` exists in
  4.x; if unavailable, call `python -m demucs -n htdemucs -o <dir>` as a subprocess).
- Model `htdemucs`, CPU unless CUDA is available (MPS optional; if it errors, use CPU). Weights go to `models_dir()` (set `TORCH_HOME`).
- Sum `bass + other`, downmix to mono, write WAV (stdlib `wave` or `torchaudio`, whichever is already installed).
- Record its wall time on the synthetic clip in the test output (`-s`); the bench measures it properly.
- `THIRD_PARTY.md`: Demucs (MIT code/weights).

## Tests
- `@pytest.mark.slow`: on the synthetic clip, the output exists, is mono 44.1 kHz, same duration ±0.1 s, and has lower energy
  than the input (drums/clicks removed).

## Verify
`cd engine && uv sync --group separate && uv run pytest -k separate`

## Commit message
`feat(engine): add Demucs harmonic stem separation`

## Out of scope
Any caching of stems between runs (the bench may reuse them within one run by path).
