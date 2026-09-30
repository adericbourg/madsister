# M0 · Step 12 — Proxy bench data: GuitarSet

## Goal
A reproducible, provisional reference set, until the user annotates their own `bench/` songs (§7).

## Spec refs
§7, M0 in §8. User decision: GuitarSet proxy, Rock + Singer-Songwriter "comp" takes, results marked provisional.

## Depends on
M0-2 (Song model, to express references as Song JSON).

## Tasks
- `bench/fetch_guitarset.py` (run with `uv run --project engine`; `jams` goes in a `bench` dependency group of the engine project):
  download `annotation.zip` and `audio_mono-mic.zip` from the Zenodo GuitarSet record (look up the current record / file URLs;
  record 3371780 at the time of writing) into `bench/data/guitarset/`. Skip files already present.
- Select `*_Rock*_comp*` and `*_SS*_comp*` takes (filename convention `<player>_<Style><n>-<bpm>-<key>_comp`). Document the count.
- `bench/references.py`: JAMS → reference: `chord` annotation (use the **performed** chords if both instructed and performed exist;
  check the namespace/annotation metadata), `beat_position` annotation → beats + downbeats (position == 1), meter from the annotation.
  Save as `bench/data/guitarset/refs/<id>.lab` (mir_eval chord format) + `<id>.beats.json`. Also build a reference `Song` per
  take by running the M0-3 quantization on the reference chords and beats (used for "edits needed").
- GuitarSet chord labels are Harte-like (`C:maj7`, `G:7`, `A:min/5`…). Normalize anything outside §4.1 with the same degradation
  table as M0-8 (reuse it, don't duplicate).
- `bench/README.md`: what the proxy bench is, its license (CC BY 4.0, attribution), its limits (solo acoustic guitar, jazzy
  voicings, no add chords, no sections, short excerpts), and how the user's own songs will plug in later
  (`bench/data/user/<id>.<ext>` + `<id>.madsister.json` reference — same runner).
- `THIRD_PARTY.md`: GuitarSet (bench only, not shipped).
- CI: the JAMS unit test needs the `bench` group → `uv sync --group bench` in `engine.yml` (same commit).

## Tests
- Unit test on a tiny hand-written JAMS fixture (a few lines of JSON in `engine/tests/fixtures/`): reference extraction gives the
  expected `.lab` lines and downbeats.

## Verify
`uv run --project engine python bench/fetch_guitarset.py && ls bench/data/guitarset/refs | wc -l` (> 0), plus the unit test.

## Commit message
`feat(bench): add GuitarSet proxy reference set`

## Out of scope
The user's own songs (they provide and annotate them).
