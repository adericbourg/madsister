"""Decode any ffmpeg-readable input to mono 44.1 kHz WAV (spec §3.3 step 1)."""

import re
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


def tags(src: str | Path) -> dict[str, str]:
    """The file's metadata tags (ID3, Vorbis comments, MP4 atoms…), lower-cased keys, first one wins; {} if unreadable.
    Read with ffmpeg, not ffprobe: the Windows installer bundles ffmpeg only."""
    # The audio stream's tags too, merged into the global ones: Ogg keeps its Vorbis comments on the stream.
    cmd = ["ffmpeg", "-nostdin", "-v", "error", "-i", str(src), "-map_metadata", "0:s:a:0", "-map_metadata", "0"]
    cmd += ["-f", "ffmetadata", "-"]
    result = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if result.returncode != 0:
        return {}
    found: dict[str, str] = {}
    # A backslash escapes the next character, newline included: read escapes whole, or `\\` would escape the newline.
    for line in re.findall(r"(?:\\.|[^\\\n])*\n", result.stdout, flags=re.S):
        entry = re.fullmatch(r"((?:\\.|[^\\=])*)=(.*)\n", line, flags=re.S)
        if entry and not line.startswith((";", "#", "[")):
            key, value = (re.sub(r"\\(.)", r"\1", s, flags=re.S) for s in entry.groups())
            found.setdefault(key.lower(), value)
    return found
