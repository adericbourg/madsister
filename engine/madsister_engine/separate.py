"""Harmonic (bass + other) and bass stems via Demucs htdemucs (spec §2.1, §3.3 step 5b). Needs the `separate` dependency group."""

import os
import wave
from pathlib import Path

from madsister_engine.models import models_dir

# Read at import time by huggingface_hub: demucs 4.1 downloads htdemucs from Hugging Face (same cache as all-in-one).
os.environ.setdefault("HF_HUB_CACHE", str(models_dir() / "huggingface"))

import numpy as np  # noqa: E402
from demucs.api import Separator  # noqa: E402
from demucs.pretrained import get_model  # noqa: E402


def prepare() -> None:
    get_model("htdemucs")


def stems(wav_path: str | Path, out_dir: str | Path) -> tuple[Path, Path]:
    """The harmonic stem (the mix without vocals and drums) and the bass stem, as mono 44.1 kHz WAVs `out_dir/harmonic.wav`
    and `out_dir/bass.wav`, from one separation. CPU unless CUDA is available."""
    separator = Separator("htdemucs")
    _, separated = separator.separate_audio_file(Path(wav_path))
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    harmonic, bass = out_dir / "harmonic.wav", out_dir / "bass.wav"
    for dst, mix in ((harmonic, separated["bass"] + separated["other"]), (bass, separated["bass"])):
        with wave.open(str(dst), "wb") as out:
            out.setnchannels(1)
            out.setsampwidth(2)
            out.setframerate(separator.samplerate)  # htdemucs runs at 44.1 kHz
            out.writeframes((np.clip(mix.mean(0).numpy(), -1, 1) * 32767).astype("<i2").tobytes())
    return harmonic, bass
