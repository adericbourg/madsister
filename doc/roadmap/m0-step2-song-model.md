# M0 · Step 2 — Song model (engine side)

## Goal
The engine can build and write a `Song` JSON exactly as specified in §4.

## Spec refs
§4 (types + rules), NF-7.

## Depends on
M0-1.

## Tasks
- `madsister_engine/song.py`: frozen dataclasses `Song`, `Meta`, `Meter`, `Section`, `Bar`, `ChordSlot`, and `to_json(song) -> dict`
  that emits camelCase keys (`tempoBpm`, `startSec`), omits `None` fields, `version: 1`.
- `validate(song)`: each bar's `sum(slot.beats) == (bar.meter or song.meta.meter).beats`, beats positive ints, ≥1 slot,
  confidence in [0,1]. Raises `ValueError` with the section/bar index.
- `write(song, path)`: validate, then write UTF-8 JSON (indent 2).
- Section ids: `uuid4().hex[:8]` is enough.

## Tests
- A two-section song round-trips to the expected dict (golden dict in the test, including omitted optional fields).
- `validate` rejects a bar whose beats don't sum to the meter, and accepts a bar with a `meter` override (e.g. 2/4 in a 4/4 song).

## Verify
`cd engine && uv run pytest -m "not slow"`

## Commit message
`feat(engine): add Song model and JSON writer`

## Out of scope
Reading Song files (the engine only writes them; reading is for the bench, see M0-12).
