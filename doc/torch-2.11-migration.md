# torch 2.5.1 → 2.11.0 migration (drop NATTEN)

## Why
`torch==2.5.1` was pinned only because `natten==0.17.4` is the last release with the legacy ops all-in-one calls
(`natten1dqkrpb`, `natten1dav`, `natten2dqkrpb`, `natten2dav`), and NATTEN builds against that torch. torch < 2.6 has
open advisories (Renovate PR #3, closed). NATTEN ≥ 0.20 removed relative position bias (RPB) and the split QK/AV ops,
and all-in-one's pretrained weights were trained with RPB, so a newer NATTEN cannot reproduce the model.
Section detection (all-in-one) is a must-have: dropping it was not an option.

## Design
- `engine/madsister_engine/beats/neighborhood_attention.py`: the four ops in plain PyTorch (window gather with NATTEN's
  border clamping, dilation, RPB added to the scores). Same signatures as NATTEN 0.17.
- `beats/allinone_tracker.py` registers it as `natten.functional` in `sys.modules` before `import allin1`, so allin1 is
  used unmodified.
- `engine/pyproject.toml`: `natten` removed, `[tool.uv] exclude-dependencies = ["natten"]` (allin1 requires it on macOS),
  every `torch`/`torchaudio` pin at 2.11.0, one lockfile, same CPU index on Linux.

## Why 2.11.0, not 2.13.0
torchaudio has no release after 2.11.0, and 2.11.0 pins `torch==2.11.0`. demucs and allin1 need torchaudio, so 2.11.0 is
the highest consistent pair. It fixes every torch advisory that has a patch except GHSA-rrmf-rvhw-rf47 (low, fixed in
2.13.0); the critical one (`torch.load`, GHSA-53q9-r3pm-6pq6) was fixed in 2.6.0. Renovate is capped at `<2.12` for
both packages (`renovate.json`) until torchaudio ships again.

## Verification (done 2026-10-02, Linux x86_64)
- Golden outputs recorded from real NATTEN 0.17.4 / torch 2.5.1 (`tests/data/natten_golden.npz`: 1D and 2D, dilation
  1-3, windows at the sequence size). The pure-torch ops match exactly (`tests/test_neighborhood_attention.py`).
- `allin1.analyze` on the synthetic clip, before vs after: beats and downbeats identical; one segment boundary moves by
  20 ms (torch 2.5 → 2.11 numerics). 45 s → 11.5 s.
- All 71 tests pass locally with every model group, including the 27 slow ones with real weights.
- Not run locally: macOS arm64, covered by the `engine-models` workflow on the PR.
