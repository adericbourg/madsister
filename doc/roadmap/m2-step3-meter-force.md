# M2 · Step 3 — Force the meter before transcription

## Goal
F-IN-5: the user may force 3/4, 4/4 or 6/8 before transcribing. Default: auto.

## Spec refs
F-IN-5, §3.2 (`--meter`), §4 (`meta.meter` 6/8).

## Depends on
M2-2.

## Decisions
- CLI: `--meter 3|4|6/8` (3 and 4 unchanged, quarter-note beats). The Rust `transcribe` command and `ui/engine.ts` pass the
  same string through (`"3" | "4" | "6/8" | null`); argparse rejects anything else.
- 6/8: the trackers see 2 dotted-quarter beats per bar (madmom DBN `beats_per_bar=[2]`). Quantization runs per tracker beat
  (so the add2/add4 step still counts tracker beats), then every slot is written in eighths (×3): `meta.meter = {6, 8}`,
  short-bar overrides `{3n, 8}`. `tempoBpm` stays the tracker pulse (dotted-quarter BPM). Documented in spec §3.2.
- all-in-one ignores the meter (unchanged): with 6/8 its downbeats are still used, its 3/4-beat bars become overrides.

## Tasks
- Engine: `--meter` choices, `pipeline` maps the choice to (tracker beats per bar, unit), `build_song(..., unit=8)` converts.
- App: labelled select (Auto, 4/4, 3/4, 6/8) in the importer, value passed to `transcribe`; Rust `meter: Option<String>`.
- The grid already sizes slots by `beats` (`flexGrow`), so 6/8 bars render as 6 eighths without change.

## Tests
- Engine: `build_song` with `unit=8` (slots ×3, override in eighths, meta 6/8); CLI `--meter 6/8` reaches the tracker as 2.
- Rust: `transcribe_args` passes `--meter 6/8`. UI: the chosen meter reaches `transcribe`.

## Verify
`cd engine && uv run --no-sync pytest -m "not slow"`; `cd app && pnpm test && pnpm build && (cd src-tauri && cargo test && cargo clippy -- -D warnings)`.
One real run with `--meter 3` and `--meter 6/8` on a GuitarSet take.

## Commit message
`feat: let the user force the meter before transcription`
