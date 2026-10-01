# M2 · Step 7 — Developer documentation

## Goal
A developer (or Claude in a fresh session) can build, run, test and package the app locally from the root `README.md` alone.

## Spec refs
§3.1 (layout), §9 (testing). User request 2026-10-01: "how to build, test and package the app locally", at the end of the README.

## Depends on
M2-1 (engine resolution in dev). Independent of the other M2 steps.

## Tasks
- Root `README.md`, section "Development" at the end (the top of the README is for users, see M6-4; leave a one-line
  placeholder there if nothing exists yet):
  - prerequisites with the versions actually used (node, pnpm, Rust, uv, Python 3.11 via uv, ffmpeg), Linux system packages for
    Tauri (link the Tauri prerequisites page; same list as `.github/workflows/app.yml`);
  - one-time engine setup: `cd engine && uv sync --group beats-madmom --group chords-cnnlstm` (+ `--group beats-allinone` for section
    detection); warn that plain `uv sync`/`uv run` removes the model groups; where weights are cached (`MADSISTER_MODELS_DIR`);
  - run: `cd app && pnpm install && pnpm tauri dev`; how the app finds the engine (`MADSISTER_ENGINE` override);
  - test: engine fast/slow tests, app (`pnpm test`), Rust (`cargo test`, `cargo clippy -- -D warnings`), the benchmark (`bench/README.md`);
    what each CI workflow covers (engine, engine-models if present, app);
  - package locally: `pnpm tauri build` (what it produces today, and that the engine isn't bundled before M6 — M6 updates this part).
- Every command in the section must be run once to check it's correct (except long downloads already done).
- Keep it short: commands + one line of context each. Link `engine/README.md` and `bench/README.md` instead of duplicating them.

## Verify
Run each documented command (or its fast equivalent) and paste nothing into the doc that you didn't run.

## Commit message
`docs: add developer guide to the README`
