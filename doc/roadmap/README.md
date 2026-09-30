# madsister implementation roadmap

Reader: Claude, running unattended. Spec: [`../product-brief.md`](../product-brief.md) (called "the spec").
**Start here after every context compaction:** read this file, then the first `todo` step in the status table.

## Execution protocol (per step)
1. Read the step file completely, plus the spec sections it references.
2. Check `Depends on`. If a dependency is `blocked`, mark this step `blocked (dep)` and move on.
3. TDD: write a failing behavioural test (red; a compile error doesn't count), then the minimal code to pass (green).
4. Run every command in the step's `Verify` section. Everything must pass.
5. Update the status table below (status + short sha of the step's own commit: fill it in the *next* commit, or write `this`).
6. Stage the step's files, run the `ponytail:ponytail-review` skill on the staged diff (the commit hook blocks otherwise), apply the
   relevant findings, re-stage. Then commit with the `development-tools:commit` skill: `git commit -s -S`, **no Co-Authored-By** (user rule), one commit per step,
   and the tests/docs go in the same commit as the code they cover. Use the step's suggested message.
7. `git push origin main`, then check CI (next section). **A red CI is priority #1: fix it before any other step.**

## Blocker policy
- Time-box: 3 distinct fix attempts (not 3 retries of the same thing). Use Context7 for library docs before giving up.
- If still blocked: append to [`BLOCKERS.md`](BLOCKERS.md) with *step · symptom (exact error) · what was tried · decision needed from the user*,
  mark the step `blocked` here, commit the log (`docs: log blocker for mX-stepY`), push, and continue with the next step
  that doesn't depend on it.
- If a step is partially done and the useful part stands alone (e.g. the adapter works but one option fails), commit that part
  and log the rest.
- Never commit weights, audio, or secrets. Never install system toolchains (brew/rustup/xcode). Package managers
  (pnpm, cargo, uv, including `uv python install`) and model/dataset downloads are allowed.
- If M0 downloads stall for more than ~30 min, switch to M1 and come back to M0 later.

## Conventions
- Repo layout per spec §3.1: `engine/` (Python, uv), `app/` (Tauri v2 + React + TS + Vite), `bench/`.
- Python 3.11, pinned via `engine/.python-version` + `requires-python`. pytest. Slow tests (models, audio) are marked `@pytest.mark.slow`.
  Default run: `uv run pytest -m "not slow"`.
- TS: `strict: true`, pnpm, vitest. Boolean variables prefixed with `is/has/should`.
- Tests: Given/When/Then comments; names follow `<method>_(when<cond>|of<input>)_<expected>` (Python: `test_` + same pattern in snake_case,
  e.g. `test_quantize_bars_when_chord_changes_mid_bar_splits_slot`). Enrich existing tests rather than multiplying near-duplicates.
- Model weights and third-party repos are cached in `${MADSISTER_MODELS_DIR:-~/.cache/madsister/models}`, never in the repo.
- `bench/data/` is gitignored (audio + downloaded annotations). Only scripts and small result files are committed.
- Chord canonical syntax = Harte (`C:maj7/3`, `A:min`, `N`) + the `add2`/`add4` extension (spec §4).
- Minimal code (no speculative abstractions). Mark a deliberate shortcut with a `ponytail:` comment naming its ceiling.
- CI = GitHub Actions, one workflow per project: `.github/workflows/engine.yml` (M0-1), `.github/workflows/app.yml` (M1-1).
  CI runs fast tests only (no model weights). Every step keeps CI green: if it adds a system dependency or a new test command,
  it updates the workflow in the same commit. Lint workflows with `uvx --from actionlint-py actionlint`. Pin actions to tags that exist
  (e.g. `astral-sh/setup-uv` has no floating major tag: use the exact version).
- Checking CI: don't use `gh` (logged into another account). The unauthenticated REST API allows only **60 requests/hour**,
  so never poll it in a loop. Poll the workflow badge instead (not rate-limited), every 60 s, for at most 10 min after the push:
  `curl -s https://github.com/adericbourg/madsister/actions/workflows/<engine|app>.yml/badge.svg | grep -oE '>(passing|failing)<'`.
  The badge shows the latest *completed* run on `main`: wait ≥ 3 min after the push before trusting it. Only when it's failing,
  spend 2 API calls: `…/repos/adericbourg/madsister/actions/runs?per_page=3` then `…/actions/runs/<id>/jobs` for the failing step.
  Logs need auth: reproduce the failing step locally.
- Dev machine is macOS arm64 (the spec targets Linux first). Don't add Linux-only code paths in M0/M1.

## Status

| Step | File | Status | Commit |
|---|---|---|---|
| M0-1 | [m0-step1-engine-scaffold](m0-step1-engine-scaffold.md) | done | 0a79b94 |
| M0-2 | [m0-step2-song-model](m0-step2-song-model.md) | done | a59e235 |
| M0-3 | [m0-step3-quantization](m0-step3-quantization.md) | done | fa8c54c |
| M0-4 | [m0-step4-decode](m0-step4-decode.md) | done | 42eb59e |
| M0-5 | [m0-step5-beats-madmom](m0-step5-beats-madmom.md) | done | b614ec1 |
| M0-6 | [m0-step6-beats-allinone](m0-step6-beats-allinone.md) | done | 37a5d14 |
| M0-7 | [m0-step7-chords-btc](m0-step7-chords-btc.md) | done | 6fdefd0 |
| M0-8 | [m0-step8-chords-cnnlstm](m0-step8-chords-cnnlstm.md) | done | 0e3b5bc |
| M0-9 | [m0-step9-demucs](m0-step9-demucs.md) | done | cf6cd7e |
| M0-10 | [m0-step10-add-heuristic](m0-step10-add-heuristic.md) | done | 527a9b1 |
| M0-11 | [m0-step11-transcribe-cli](m0-step11-transcribe-cli.md) | done | 91c127f |
| M0-12 | [m0-step12-bench-guitarset](m0-step12-bench-guitarset.md) | done | 4963df1 |
| M0-13 | [m0-step13-bench-runner](m0-step13-bench-runner.md) | done | 0e75b99 |
| M0-14 | [m0-step14-results-decisions](m0-step14-results-decisions.md) | done | c1e93db |
| M1-1 | [m1-step1-app-scaffold](m1-step1-app-scaffold.md) | done | a49966d |
| M1-2 | [m1-step2-song-model](m1-step2-song-model.md) | done | 9cabc31 |
| M1-3 | [m1-step3-chord-parser](m1-step3-chord-parser.md) | done | 775c9d7 |
| M1-4 | [m1-step4-chord-display](m1-step4-chord-display.md) | done | ba411b2 |
| M1-5 | [m1-step5-transpose](m1-step5-transpose.md) | done | 18d1b11 |
| M1-6 | [m1-step6-bar-slot-commands](m1-step6-bar-slot-commands.md) | done | 7b41a53 |
| M1-7 | [m1-step7-section-commands](m1-step7-section-commands.md) | done | 096e12a |
| M1-8 | [m1-step8-history](m1-step8-history.md) | done | 418e7ae |
| M1-9 | [m1-step9-grid-view](m1-step9-grid-view.md) | done | fc28933 |
| M1-10 | [m1-step10-keyboard-editing](m1-step10-keyboard-editing.md) | done | 62b481d |
| M1-11 | [m1-step11-file-io](m1-step11-file-io.md) | done | 3878597 |
| M1-12 | [m1-step12-settings-transpose-ui](m1-step12-settings-transpose-ui.md) | done | this |
| M1-13 | [m1-step13-print](m1-step13-print.md) | todo | |
| M2-1 | [m2-step1-rust-engine-bridge](m2-step1-rust-engine-bridge.md) | todo | |
| M2-2 | [m2-step2-import-ui](m2-step2-import-ui.md) | todo | |
| M2-3 | [m2-step3-meter-force](m2-step3-meter-force.md) | todo | |
| M2-4 | [m2-step4-confidence-flags](m2-step4-confidence-flags.md) | todo | |
| M2-5 | [m2-step5-bar-meter-override](m2-step5-bar-meter-override.md) | todo | |
| M2-6 | [m2-step6-audio-ref-and-perf](m2-step6-audio-ref-and-perf.md) | todo | |
| M3-1 | [m3-step1-playback-cursor](m3-step1-playback-cursor.md) | todo | |
| M3-2 | [m3-step2-click-seek](m3-step2-click-seek.md) | todo | |
| M3-3 | [m3-step3-phase-nudge](m3-step3-phase-nudge.md) | todo | |
| M3-4 | [m3-step4-tempo-fix](m3-step4-tempo-fix.md) | todo | |
| M4-1 | [m4-step1-chordpro-grid](m4-step1-chordpro-grid.md) | todo | |
| M4-2 | [m4-step2-musicxml](m4-step2-musicxml.md) | todo | |
| M4-3 | [m4-step3-midi](m4-step3-midi.md) | todo | |
| M4b-1 | [m4b-step1-engine-mode-presets](m4b-step1-engine-mode-presets.md) | todo | |
| M4b-2 | [m4b-step2-bench-modes](m4b-step2-bench-modes.md) | todo | |
| M4b-3 | [m4b-step3-mode-picker-ui](m4b-step3-mode-picker-ui.md) | todo | |
| M5-1 | [m5-step1-engine-fetch](m5-step1-engine-fetch.md) | todo | |
| M5-2 | [m5-step2-engine-record](m5-step2-engine-record.md) | todo | |
| M5-3 | [m5-step3-sources-ui](m5-step3-sources-ui.md) | todo | |
| M5-4 | [m5-step4-sections-fallback](m5-step4-sections-fallback.md) | todo | |
| M6-1 | [m6-step1-engine-setup-script](m6-step1-engine-setup-script.md) | todo | |
| M6-2 | [m6-step2-linux-bundle](m6-step2-linux-bundle.md) | todo | |
| M6-3 | [m6-step3-macos-bundle](m6-step3-macos-bundle.md) | todo | |

Scope of the first unattended run (2026-09-30 night): M0 + M1. M2–M6 are written at a coarser level. Revise them
after M0 (the model choices change M2/M4b/M6), and ask the user before starting them.

## Morning hand-off
When stopping (all tonight's steps done, or everything left is blocked), make sure these are up to date and pushed:
this status table, `BLOCKERS.md`, and `m0-results.md` (from M0-14). CI must be green on the last commit.
