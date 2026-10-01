# madsister engine

Python CLI that turns audio into a chord grid (`*.madsister.json`). Contract: spec §3.2 — JSON Lines on stdout, exit code 0 on success.

```
uv sync --group beats-allinone --group chords-btc   # a beat tracker and a chord model (one command: --group replaces)
uv run madsister-engine transcribe song.mp3 --out song.madsister.json
```
The app (dev build) runs `uv run --project engine --no-sync madsister-engine` (plain `uv run` re-syncs and drops the model
groups), so sync once with `uv sync --group beats-madmom --group chords-cnnlstm` (+ `--group beats-allinone` for sections),
or set `MADSISTER_ENGINE` to another command line.

`setup` downloads the model weights and repos once into `$MADSISTER_MODELS_DIR` (default `~/.cache/madsister/models`);
transcription then runs offline. Idempotent; one progress line per model, then the models dir as `result`:
```
uv run --no-sync madsister-engine setup              # default models (Chord-CNN-LSTM; madmom ships its weights): 30 MB
uv run --no-sync madsister-engine setup --sections   # + all-in-one (sections) and htdemucs: + 98 MB
uv run --no-sync madsister-engine setup --all        # + the bench-only models (BTC, htdemucs for --separate)
```
Without `setup`, each model downloads on first use.

`fetch` downloads a URL's audio with yt-dlp (`--group fetch`; the engine's only network access) and emits the file path:
```
uv sync --group fetch    # add the groups you transcribe with: --group replaces
uv run --no-sync madsister-engine fetch https://… --out-dir ~/Music   # result: <out-dir>/<title>.<ext>, any format decode reads
```
`record` records the default input (`--group record`; Linux needs `libportaudio2`) to a mono 44.1 kHz 16-bit WAV until
Ctrl-C or a `stop` line on stdin (EOF stops too). On macOS the microphone permission is asked for the parent process (the
terminal, or the app).
```
uv run --no-sync madsister-engine record --out take.wav   # progress: {"stage":"record","pct":0,"elapsedSec":n} every second
```
```
uv run pytest -m "not slow"   # fast tests (what CI runs)
uv run pytest -m slow         # needs the model dependency groups and weights
```
The `engine-models` CI workflow runs the slow tests with every model group, on Linux and macOS. Same locally:
```
uv sync --locked --group beats-madmom --group beats-allinone --group chords-btc --group chords-cnnlstm --group separate --group fetch --group record
uv run --no-sync pytest -m slow tests/test_model_imports.py   # seconds: each heavy dep imports and runs one native call
uv run --no-sync pytest -m slow                               # minutes: downloads weights on the first run
```
On Linux, torch and torchaudio come from the PyTorch CPU index (PyPI's are the CUDA builds), and NATTEN is built from
source (needs a C++ compiler).
