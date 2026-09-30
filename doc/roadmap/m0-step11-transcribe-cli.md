# M0 · Step 11 — `transcribe` command

## Goal
`madsister-engine transcribe <audio> --out <song.json>` runs the full §3.3 pipeline and follows the §3.2 contract.

## Spec refs
§3.2, §3.3, F-IN-4 (progress stages), F-IN-5 (`--meter`), D7 (single default pipeline before M4b).

## Depends on
M0-2, M0-3, M0-4, at least one of M0-5/M0-6, at least one of M0-7/M0-8. M0-9 and M0-10 are optional.

## Tasks
- `madsister_engine/pipeline.py`: `transcribe(audio, out, meter, has_sections, beats="auto", chords="auto", separate=False, add_tau=None)`.
  Progress stages and rough percentages: `decode` 5, `separate` 20 (if enabled), `beats` 40, `chords` 70, `quantize` 90, `write` 100.
  `auto` = the first installed option in a fixed preference order (all-in-one > madmom; btc > cnnlstm) until M0-14 sets the defaults.
  Import each adapter lazily so a missing optional group gives a clear error ("install group X").
- CLI: public flags from §3.2 (`--meter`, `--no-sections`); **hidden** bench flags (`argparse.SUPPRESS` help)
  `--beats {auto,madmom,allinone}`, `--chords {auto,btc,cnnlstm}`, `--separate`, `--add-tau FLOAT`. `--mode` is added in M4b, not now.
- Temp files in a `tempfile.TemporaryDirectory`. `result` line with the absolute output path.
- `--no-sections` → segments ignored → one "Song" section.

## Tests
- Unit: with adapters monkeypatched to return fixed beats/segments, `transcribe` emits the stages in order, writes a valid Song, and
  exits 0; a failing adapter produces an `error` line and exit 1.
- `@pytest.mark.slow` e2e: synthetic clip → Song with 8 bars ±1, meter 4, first chords C/G/Am/F on most bars.

## Verify
```
cd engine && uv run pytest -m "not slow" && uv run pytest -m slow -k e2e
uv run madsister-engine transcribe <synthetic.wav> --out /tmp/s.madsister.json   # prints progress lines + result
```

## Commit message
`feat(engine): wire the transcribe command end to end`

## Out of scope
`fetch`/`record` (M5), `--mode` (M4b), key estimation.
