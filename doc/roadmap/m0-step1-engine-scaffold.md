# M0 · Step 1 — Engine scaffold

## Goal
A runnable `madsister-engine` CLI skeleton that follows the §3.2 stdout/exit-code contract, plus the repo licensing files.

## Spec refs
§3.1, §3.2, D2, NF-6.

## Depends on
—

## Tasks
- `engine/` uv project: `uv init --package --name madsister-engine`, then pin Python 3.11 (`uv python install 3.11` if missing,
  `.python-version` = `3.11`, `requires-python = ">=3.11,<3.12"`; widen later only if every model allows it).
  Package dir `engine/madsister_engine/`, script entry point `madsister-engine = "madsister_engine.cli:main"`.
- `madsister_engine/events.py`: `progress(stage, pct)`, `result(path)`, `error(message)` → one JSON object per line on stdout, flushed.
- `madsister_engine/cli.py`: argparse with subcommands `transcribe` (positional audio, `--out`, `--meter {3,4}`, `--no-sections`),
  `fetch` and `record` as stubs that emit `error("not implemented")` and exit 2. Wrap `main` so that any exception →
  `error(str(e))` line then exit 1. Nothing but JSONL on stdout (logs go to stderr).
- Dev dependency: pytest. Add `[tool.pytest.ini_options]` with the `slow` marker registered.
- Root files: `LICENSE` (GPLv3 full text, fetched from https://www.gnu.org/licenses/gpl-3.0.txt), `THIRD_PARTY.md` (table header:
  component · purpose · code license · weights license · source URL; filled as deps are added), `.gitignore`
  (`.venv/`, `__pycache__/`, `bench/data/`, `bench/results/*.wav`, `node_modules/`, `dist/`, `app/src-tauri/target/`, `.DS_Store`).
- Short `engine/README.md`: how to run (`uv run madsister-engine …`) and test.
- **CI** `.github/workflows/engine.yml` (GitHub Actions): on push/PR touching `engine/**`, `bench/**` or the workflow itself;
  `ubuntu-latest` (Linux is the v1 target, NF-3); `astral-sh/setup-uv` (check the current major version with Context7),
  `apt-get install ffmpeg`, `uv sync` (core only, no optional model groups), `uv run pytest -m "not slow"`.
  Slow tests (model weights, several GB) never run in CI. Later steps that add non-slow tests are covered automatically;
  a step adding a system dependency needed by non-slow tests updates this workflow in the same commit.

## Tests
- `events`: each function writes exactly one valid JSON line with the §3.2 shape.
- `cli`: a stub command emits an `error` line and exits non-zero; an unexpected exception is converted to `error` + exit 1
  (inject by calling `main` with a missing audio file for `transcribe`, which must fail cleanly until step 11).

## Verify
```
cd engine && uv sync && uv run pytest -m "not slow" && uv run madsister-engine fetch x --out-dir /tmp; echo "exit=$?"
```
Expected: tests pass; the last command prints one `{"type":"error",...}` line and a non-zero exit.
Lint the workflow: `uvx --from actionlint-py actionlint .github/workflows/engine.yml` (no toolchain install needed).
The CI run itself can't be observed from here (`gh` must not be used): list "check CI is green" in the morning hand-off.

## Commit message
`feat(engine): scaffold engine CLI with JSON Lines events and CI`

## Out of scope
Any audio processing.
