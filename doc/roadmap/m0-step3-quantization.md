# M0 · Step 3 — Beat-synchronous quantization

## Goal
Pure, model-independent functions that turn frame-level chord labels + beats/downbeats (+ segments) into sections/bars/slots.
This is the heart of the engine and the main unit-test target of §9.

## Spec refs
§3.3 steps 4, 5, 6; §4; §9.

## Depends on
M0-2.

## Tasks
`madsister_engine/quantize.py`, all pure (inputs are plain lists / numpy arrays, outputs are `song.py` objects):
- Input types: `ChordSegment(start, end, label, confidence)` (models give segments or frames; convert frames to segments in the
  adapters); `beats: list[float]`; `downbeats: list[float]` (subset of the beat times); `segments: list[(start, end, label)] | None`.
- `beat_labels(chords, beats)`: for each beat interval [b_i, b_{i+1}), majority label by overlapped duration; confidence =
  duration-weighted mean of the winning label's segment confidences. The last beat uses the median beat period as its duration.
- `bars_from_beats(beat_labels, beats, downbeats, meter_beats)`: split the beats into bars at downbeats; group consecutive equal
  labels into `ChordSlot(beats=n)`; `startSec` = downbeat time. Beats before the first downbeat form a pickup bar only if
  there are ≥ 2 of them, otherwise drop them (`ponytail:` comment). A bar whose beat count ≠ meter keeps its actual count through a
  `Bar.meter` override (this is expected at song edges and from tracker errors).
- `simplify(bars)`: absorb `N` slots into the previous slot of the same bar (or the next one if first); a bar that is all `N` stays `N`.
  (Slots < 1 beat can't exist after beat quantization; keep the rule implicit.)
- `sections_from_segments(bars, segments)`: snap each segment start to the nearest bar start; label mapping
  `intro→Intro, verse→Verse, chorus→Chorus, bridge→Bridge, inst→Instrumental, solo→Solo, break→Break, outro→Outro`,
  otherwise capitalize; drop zero-bar sections; `segments is None` → one section "Song".
- `build_song(...)`: composition of the above → `Song` (title = audio file stem, `tempoBpm` = 60 / median beat period, rounded).

## Tests (synthetic, no audio)
- Chord change on beat 3 of a 4/4 bar → slots `[2, 2]`.
- A short spurious label covering < half a beat doesn't create a slot (majority wins).
- `N` in the middle of a bar is absorbed; an all-`N` bar stays one `N` slot of 4 beats.
- Pickup of 1 beat is dropped; pickup of 2 beats becomes a bar with a 2-beat meter override.
- Segment boundaries 0.3 s after a downbeat snap to that bar; no segments → single "Song" section.
- 3/4 input produces 3-beat bars.

## Verify
`cd engine && uv run pytest -m "not slow"`

## Commit message
`feat(engine): quantize chord segments into bars and sections`

## Out of scope
The add2/add4 relabelling (M0-10), key estimation (§3.3 step 7, skipped in M0).
