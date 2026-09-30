# M0 · Step 8 — Chord model: Chord-CNN-LSTM

## Goal
Adapter `madsister_engine/chords/cnnlstm.py` with the same `recognize(wav_path) -> list[ChordSegment]` signature as BTC.

## Spec refs
§2.1 (Chord-CNN-LSTM row), §3.3 step 3, §4.1 Q10 (inversions).

## Depends on
M0-7 (`models.py` helpers, `ChordSegment` merge helper).

## Tasks
- Clone `music-x-lab/ISMIR2019-Large-Vocabulary-Chord-Recognition` via `ensure_repo` at a pinned sha. Read its README and
  `chord_recognition.py` to find the inference entry point and the pretrained models it ships.
- The code is old (little maintained). Prefer running it **in-process** by importing its modules. If its dependency pins conflict with
  ours, run it as a subprocess in its own `uv venv` under `models_dir()` (note it in a `ponytail:` comment) and parse its `.lab` output.
- Choose the chord dictionary whose vocabulary is closest to §4.1 (e.g. `submission`/`ismir2017`-style list with inversions).
  Map its labels to Harte: keep the inversions (`/3`, `/5`, `/b7`) → resolves the Q10 "model-dependent" VERIFY item for this model.
  Qualities outside §4.1 (e.g. `9`, `11`, `maj9`, `min9`, `13`) → degrade to the nearest §4.1 quality (`9`→`7`, `maj9`→`maj7`,
  `min9`/`min11`→`min7`, `sus4(b7)`→`7`… keep it a small explicit table); keep the bass. `X` → `N`.
- Confidence: if the model exposes per-frame probabilities, use max prob; otherwise leave `None` and log it in `m0-results.md` later
  (the F-ED-8 flags then won't work with this model; that counts against it).
- Optional group `chords-cnnlstm`. `THIRD_PARTY.md` entry.

## Tests
- Unit: label mapping table (inversion kept, degradation, `X`→`N`).
- `@pytest.mark.slow`: same loose synthetic-clip check as M0-7.

## Verify
`cd engine && uv sync --group chords-cnnlstm && uv run pytest -k cnnlstm`

## Commit message
`feat(engine): add Chord-CNN-LSTM chord recognizer`

## Out of scope
ChordFormer: only its availability is checked, in M0-14 (research only, no adapter unless code + weights are public **and** it
installs within the time-box; otherwise record it).

## If blocked
Log it; the bench runs with BTC only.
