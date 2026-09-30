# M0 · Step 4 — Audio decode + synthetic test clip

## Goal
Decode any supported input to mono 44.1 kHz WAV, and have a license-free audio fixture for the end-to-end tests.

## Spec refs
§3.3 step 1, F-IN-1 formats, §9 ("one end-to-end test on a short clip").

## Depends on
M0-1.

## Tasks
- `madsister_engine/decode.py`: `decode(src, dst_dir) -> Path` via `subprocess.run(["ffmpeg", "-nostdin", "-y", "-i", src,
  "-ac", "1", "-ar", "44100", dst])`. On a non-zero exit, raise with the last lines of ffmpeg's stderr. Check `shutil.which("ffmpeg")`
  first with a clear message.
- `engine/tests/fixtures/make_clip.py` (plain function, used by a pytest fixture, output in `tmp_path`, never committed):
  numpy-synthesized 120 BPM 4/4 clip, ~16 s: progression `C G A:min F` one bar each, repeated ×2. Each chord = triad of sine
  partials (plus a quieter octave-down bass) and a click/kick on each beat, accented on the downbeat, so beat trackers have
  something to lock onto. Write it with the stdlib `wave` module (no soundfile dependency).
- Add numpy as a dependency.

## Tests
- `decode` of the synthetic WAV written at 22.05 kHz stereo yields mono 44.1 kHz (read the header with `wave`).
- `decode` of a non-audio file raises with an ffmpeg message.

## Verify
`cd engine && uv run pytest -m "not slow"`

## Commit message
`feat(engine): decode audio to mono 44.1 kHz WAV`

## Out of scope
Resampling in Python; loudness normalization.
