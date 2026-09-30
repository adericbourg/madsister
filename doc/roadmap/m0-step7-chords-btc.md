# M0 · Step 7 — Chord model: BTC (large vocabulary)

## Goal
Adapter `madsister_engine/chords/btc.py`: `recognize(wav_path) -> list[ChordSegment]` with Harte labels and confidences.

## Spec refs
§2.1 (BTC row, VERIFY weight availability), §3.3 step 3, §4.1.

## Depends on
M0-3, M0-4.

## Tasks
- `madsister_engine/models.py`: `models_dir()` = `$MADSISTER_MODELS_DIR` or `~/.cache/madsister/models`; `ensure_repo(name, url, sha)`
  that runs `git clone` + `git checkout <sha>` into `models_dir()/name` if missing. (Shared by M0-8.)
- BTC repo `jayg996/BTC-ISMIR19`: pin a commit. Check that the large-vocab weights file (`test/btc_model_large_voca.pt`) and
  `run_config.yaml` exist → this resolves the VERIFY item. Import its model/feature code by adding the cloned dir to `sys.path`
  (MIT, so vendoring is allowed too, but cloning keeps the repo clean). Reproduce the inference path of its `test.py`:
  CQT features, normalization with the saved mean/std, windows of `timestep` frames, softmax → argmax + max prob per frame.
- Frames → segments: merge consecutive equal labels; confidence = mean of max-prob over the segment.
- BTC label names (from `idx2voca_chord`, e.g. `C:min7`, `C#:maj6`, `N`, `X`) → Harte: mostly identity. Map `X` → `N`.
  Quality names must be exactly the §4.1 set (`maj, min, dim, aug, maj6, min6, 7, maj7, min7, minmaj7, dim7, hdim7, sus2, sus4`).
  Unit-test the mapping table. Keep the mapping and the frame→segment merge in torch-free code (`chords/labels.py`) and import torch
  lazily inside `recognize`, so these unit tests run in CI without the model group.
- Dependencies: torch (CPU), librosa, pyyaml, as the repo requires. Optional group `chords-btc`. PyTorch on macOS arm64 = default wheel.
- `THIRD_PARTY.md`: BTC (MIT code, weights license as found in the repo).

## Tests
- Unit: label mapping (`X`→`N`, `C#:maj6` unchanged, no unknown quality survives).
- Unit: frame→segment merge with confidences.
- `@pytest.mark.slow`: on the synthetic clip, the majority label per bar matches `C:maj G:maj A:min F:maj` on ≥ 6 of 8 bars
  (synthetic sine tones are an unusual input, so the tolerance is loose; if it fails badly, inspect the output before calling it a bug).

## Verify
`cd engine && uv sync --group chords-btc && uv run pytest -k btc` (includes slow)

## Commit message
`feat(engine): add BTC large-vocabulary chord recognizer`

## Out of scope
The majmin BTC variant.
