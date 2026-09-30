# M0 · Step 6 — Beat tracker + structure: all-in-one

## Goal
Adapter `madsister_engine/beats/allinone_tracker.py`: `track(wav_path, meter) -> BeatResult(beats, downbeats, segments)`.

## Spec refs
§2.1 (all-in-one row), §3.3 steps 2 and 6, F-ST-1.

## Depends on
M0-3, M0-4. Shares the madmom install with M0-5 (all-in-one depends on madmom from git).

## Tasks
- Check the install instructions with Context7 / the `mir-aidj/all-in-one` README first. It needs PyTorch, NATTEN, and madmom from git.
  **Known risk:** NATTEN wheels target Linux/CUDA. On macOS arm64 it must build from source, or an older CPU-capable NATTEN version
  compatible with the pinned torch must be used. Try in that order: (1) the README's recommended version, (2) a NATTEN version
  known to support CPU, (3) `uv pip install --no-build-isolation natten==<ver>`. 3 attempts, then blocker.
- Optional dependency group `beats-allinone`.
- `allin1.analyze(path, ...)` → `beats`, `downbeats`, `segments` (start, end, label). Keep `beats`/`downbeats` as float lists.
  all-in-one runs Demucs internally; point its stems/cache dirs at `MADSISTER_MODELS_DIR` subfolders, not at the cwd.
- `meter` is not a parameter of all-in-one. If the user forces a meter, keep the downbeats as detected (note it in a docstring).
- `THIRD_PARTY.md`: all-in-one (MIT code; record the weights license found — this resolves a VERIFY item), NATTEN, Demucs.

## Tests
- `@pytest.mark.slow`: same beat/downbeat tolerances as M0-5 on the synthetic clip; `segments` is a non-empty list of
  (float, float, str).

## Verify
`cd engine && uv sync --group beats-allinone && uv run pytest -m slow -k allinone`

## Commit message
`feat(engine): add all-in-one beat and structure tracker`

## Out of scope
Using all-in-one's tempo output.

## If blocked
Log it. Sections then fall back to a single "Song" section (already handled by M0-3), and the bench runs with madmom only.
If both M0-5 and M0-6 are blocked, M0-11 onwards are blocked too: stop M0 and go to M1.
