"""Harmonic stem (bass + other) via Demucs htdemucs (spec §2.1, §3.3 step 5b). Needs the `separate` dependency group."""

import os
import wave
from pathlib import Path

from madsister_engine.models import models_dir

# Read at import time by huggingface_hub: demucs 4.1 downloads htdemucs from Hugging Face (same cache as all-in-one).
os.environ.setdefault("HF_HUB_CACHE", str(models_dir() / "huggingface"))

import numpy as np  # noqa: E402
from demucs.api import Separator  # noqa: E402


def harmonic_stem(wav_path: str | Path, out_dir: str | Path) -> Path:
    """The mix without vocals and drums, as mono 44.1 kHz WAV `out_dir/harmonic.wav`. CPU unless CUDA is available."""
    separator = Separator("htdemucs")
    _, stems = separator.separate_audio_file(Path(wav_path))
    mono = (stems["bass"] + stems["other"]).mean(0).numpy()

    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    dst = out_dir / "harmonic.wav"
    with wave.open(str(dst), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(separator.samplerate)  # htdemucs runs at 44.1 kHz
        out.writeframes((np.clip(mono, -1, 1) * 32767).astype("<i2").tobytes())
    return dst
