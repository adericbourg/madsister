# M5 · Step 2 — Engine `record` (microphone)

## Goal
`madsister-engine record --out <file.wav>` records from the default input until SIGINT or `stop` on stdin (F-IN-3, D3).

## Spec refs
F-IN-3, D3, §3.2.

## Depends on
M0-1.

## Contract (what M5-3 relies on)
- Args: `record --out <file.wav>`. Output: mono 44.1 kHz 16-bit PCM WAV, the default input device.
- Events: `progress` with `stage: "record"`, `pct: 0` (no known end; the Rust bridge requires `pct`) and `elapsedSec`
  (whole seconds written): `0` once the device is open (recording has started), then one line per second. Then `result`
  with the absolute path. §3.2 documents the extension.
- Stop: a `stop` line on stdin (other lines ignored), SIGINT, or stdin EOF (the parent is gone). Everything captured
  up to the stop is written. The app must spawn with a piped stdin (the bridge uses `Stdio::null()` today: EOF would stop
  at once) and write `stop\n`; killing the process loses the WAV header.
- Errors: no input device / permission denied → PortAudio's message as the `error` line, exit 1, no file written.
  Without the group: `recorder sounddevice needs the \`record\` dependency group: \`uv sync --group record\``.
- macOS: the microphone permission applies to the parent process (Terminal, or the app: M6 packaging adds
  `NSMicrophoneUsageDescription`).

## Tasks
- Dependency group `record = ["sounddevice"]`, imported lazily (`pipeline._adapter`); PortAudio is bundled in the macOS
  wheels, `libportaudio2` on Linux (not needed in CI: the fast tests fake the module). Add the group to the
  `engine-models` workflow sync. `THIRD_PARTY.md`: sounddevice (MIT), PortAudio (MIT).
- `madsister_engine/record.py`: `InputStream(samplerate=44100, channels=1, dtype="int16", callback)` → queue → main
  loop writes with stdlib `wave` every 0.2 s; SIGINT handler sets a stop event (restored afterwards); a daemon thread
  reads stdin.
- `events.progress` takes extra fields (`elapsedSec`).

## Tests
- `tests/test_cli.py`, `sounddevice` faked via `sys.modules` (a stream feeding N blocks to the callback): stop via stdin →
  WAV with the exact samples and format, progress per second; SIGINT via the installed handler → WAV, handler restored;
  device error → `error` line, exit 1, no file.

## Verify
- `cd engine && uv run --no-sync pytest -m "not slow"`, `uv lock --check`,
  `uvx --from actionlint-py actionlint ../.github/workflows/engine-models.yml`.
- Manual: `uv sync --group record`, record a few seconds, `stop` on stdin. Not done unattended: devices are listed, but
  opening the input stream blocks (pending macOS microphone permission for the agent's process); to check by hand.

## Commit message
`feat(engine): record from the microphone`
