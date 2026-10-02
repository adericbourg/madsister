# Blockers

Append-only log. Format per entry:

```
## mX-stepY — <short title> (YYYY-MM-DD)
- Symptom: <exact error>
- Tried: <1. … 2. … 3. …>
- Decision needed: <question for the user>
```

## Decision: torch 2.5.1 pin vs security advisories (2026-09-30) — RESOLVED 2026-10-01
- Symptom: GitHub's Dependabot security-update runs ("uv in /engine for torch - Update") fail on every push. They are not
  our CI (the `engine` workflow is green); they come from Dependabot security updates enabled in the repo settings.
- Cause: the `beats-allinone` and `chords-btc` groups pin `torch==2.5.1`, because all-in-one needs `natten==0.17.4`
  (the last release with the API allin1 calls), and NATTEN builds against that torch. torch < 2.6 has known advisories
  (e.g. `torch.load` with `weights_only=True` is still exploitable: CVE-2025-32434). We only load weights from pinned
  upstream repos / Hugging Face, so the exposure is limited to those sources.
- Not tried: bumping torch would likely break the NATTEN 0.17.4 build and therefore all-in-one (M0-6).
- Decision needed: accept the pin for the M0 spike (and dismiss the Dependabot alert), or drop/replace all-in-one if M0-14
  shows it isn't worth it (it's also the slowest tracker: ~30 s for a 16 s clip).
- Resolution (user, 2026-10-01): keep all-in-one as a slow opt-in for section detection, so the torch 2.5.1 pin and
  its Dependabot alerts are accepted for now.
- Superseded 2026-10-02: NATTEN is replaced by a PyTorch reimplementation of the ops all-in-one calls, torch/torchaudio
  moved to 2.11.0 (see `doc/torch-2.11-migration.md`). Section detection kept.
