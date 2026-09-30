# M0 · Step 5 — Beat tracker: madmom

## Goal
Adapter `madsister_engine/beats/madmom_tracker.py`: `track(wav_path, meter: int | None) -> BeatResult(beats, downbeats, segments=None)`.

## Spec refs
§2.1 (madmom row), §3.3 step 2, §10 (madmom risk).

## Depends on
M0-3 (types), M0-4.

## Tasks
- Dependency: `madmom @ git+https://github.com/CPJKU/madmom` (PyPI 0.16.1 is broken). Pin to a commit sha in `pyproject.toml`.
  It needs Cython/numpy at build time. If the build fails, try `uv add --no-build-isolation` with the build deps pre-installed.
  Use an optional dependency group `beats-madmom` so the core install stays light.
- `RNNDownBeatProcessor()(wav)` → `DBNDownBeatTrackingProcessor(beats_per_bar=[meter] if meter else [3, 4], fps=100)`.
  Output rows `(time, position)`: beats = all times, downbeats = rows with position == 1.
- Define `BeatResult` in `madsister_engine/beats/__init__.py` (a dataclass; no abstract base class, since one shape is enough).
- `THIRD_PARTY.md`: madmom — BSD code / CC BY-NC-SA 4.0 weights.

## Tests
- `@pytest.mark.slow`: on the synthetic clip, the beat period is within 5% of 0.5 s and ≥ 75% of the downbeats are within 70 ms
  of a true downbeat (every 2.0 s).

## Verify
`cd engine && uv sync --group beats-madmom && uv run pytest -m slow -k madmom`

## Commit message
`feat(engine): add madmom beat/downbeat tracker`

## Out of scope
Key estimation (madmom key), which is optional (§3.3 step 7) and skipped in M0.

## If blocked
Log it in BLOCKERS.md. The all-in-one tracker (M0-6) alone is enough to continue; the bench then has 1 beat tracker.
