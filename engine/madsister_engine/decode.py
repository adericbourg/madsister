"""Decode any ffmpeg-readable input to mono 44.1 kHz WAV (spec §3.3 step 1)."""

import shutil
import subprocess
from pathlib import Path


def decode(src: str | Path, dst_dir: str | Path) -> Path:
    if shutil.which("ffmpeg") is None:
        raise RuntimeError("ffmpeg not found on PATH: install it (e.g. `apt install ffmpeg` or `brew install ffmpeg`)")
    dst_dir = Path(dst_dir)
    dst_dir.mkdir(parents=True, exist_ok=True)
    dst = dst_dir / "decoded.wav"
    cmd = ["ffmpeg", "-nostdin", "-y", "-i", str(src), "-ac", "1", "-ar", "44100", str(dst)]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        tail = "\n".join(result.stderr.strip().splitlines()[-5:])
        raise RuntimeError(f"ffmpeg failed to decode {src}:\n{tail}")
    return dst
