"""all-in-one beat, downbeat and section tracker (spec §2.1). Needs the `beats-allinone` dependency group."""

import os
import tempfile
from pathlib import Path

from madsister_engine.models import models_dir

# Read at import time by huggingface_hub, here and in the Demucs subprocess: all-in-one and htdemucs weights.
os.environ.setdefault("HF_HUB_CACHE", str(models_dir() / "huggingface"))

from madsister_engine.beats import BeatResult  # noqa: E402
from madsister_engine.beats.neighborhood_attention import install_as_natten  # noqa: E402

install_as_natten()  # allin1 imports its attention ops from NATTEN, which is not installed
import allin1  # noqa: E402


def prepare() -> None:
    """Download the weights `analyze()` loads: its default 8-fold ensemble, and htdemucs for its source separation."""
    from allin1.models import load_pretrained_model
    from demucs.pretrained import get_model

    load_pretrained_model("harmonix-all", device="cpu")
    get_model("htdemucs")


def track(wav_path: str | Path, meter: int | None = None) -> BeatResult:
    """all-in-one has no meter input: a forced `meter` is ignored and the detected downbeats are kept."""
    # Demucs stems and spectrograms are per-song byproducts: keep them out of the cwd, allin1 deletes them afterwards.
    with tempfile.TemporaryDirectory() as tmp:
        # No spectrogram pool: macOS spawns its workers, which re-import allin1 without the natten shim and hang.
        result = allin1.analyze(
            Path(wav_path), demix_dir=Path(tmp, "demix"), spec_dir=Path(tmp, "spec"), multiprocess=False
        )
    return BeatResult(
        beats=[float(t) for t in result.beats],
        downbeats=[float(t) for t in result.downbeats],
        segments=[(float(s.start), float(s.end), s.label) for s in result.segments],
    )
