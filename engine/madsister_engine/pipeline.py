"""The §3.3 transcription pipeline: audio file -> Song JSON, with progress events."""

import contextlib
import functools
import importlib
import importlib.util
import os
import statistics
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

from madsister_engine import add_heuristic, events
from madsister_engine.beats import BeatResult
from madsister_engine.decode import decode
from madsister_engine.quantize import ChordSegment, build_song
from madsister_engine.song import Song, write

# option -> (dependency group, module that only that group installs, adapter module). `auto` = first installed, in order.
# ponytail: fixed preference order until M0-14 picks the defaults from the bench.
_BEATS = {
    "allinone": ("beats-allinone", "allin1", "madsister_engine.beats.allinone_tracker"),
    "madmom": ("beats-madmom", "madmom", "madsister_engine.beats.madmom_tracker"),
}
_CHORDS = {
    "btc": ("chords-btc", "mir_eval", "madsister_engine.chords.btc"),
    "cnnlstm": ("chords-cnnlstm", "pydub", "madsister_engine.chords.cnnlstm"),
}
_SEPARATE = {"demucs": ("separate", "demucs", "madsister_engine.separate")}


@dataclass(frozen=True)
class Transcription:
    song: Song
    beats: BeatResult  # tracker output, before quantization
    chords: list[ChordSegment]  # recognizer output, before quantization


def _installed(module: str) -> bool:
    return importlib.util.find_spec(module) is not None


def _adapter(kind: str, options: dict, choice: str):
    """Import the adapter lazily, so a missing optional group fails with the group to install."""
    if choice == "auto":
        choice = next((name for name, (_, marker, _) in options.items() if _installed(marker)), None)
        if choice is None:
            groups = " or ".join(f"`uv sync --group {group}`" for group, _, _ in options.values())
            raise RuntimeError(f"no {kind} installed: {groups}")
    group, marker, module = options[choice]
    if not _installed(marker):
        raise RuntimeError(f"{kind} {choice} needs the `{group}` dependency group: `uv sync --group {group}`")
    return importlib.import_module(module)


@contextlib.contextmanager
def _stdout_to_stderr():
    """Models print to stdout, even from subprocesses (allin1 runs Demucs): keep fd 1 for the JSON Lines."""
    sys.stdout.flush()
    saved = os.dup(1)
    os.dup2(2, 1)
    try:
        yield
    finally:
        sys.stdout.flush()
        os.dup2(saved, 1)
        os.close(saved)


def beats_per_bar(beats: list[float], downbeats: list[float]) -> int:
    """Median number of beats between consecutive downbeats; 4 with fewer than two downbeats."""
    starts = sorted({min(range(len(beats)), key=lambda i: abs(beats[i] - d)) for d in downbeats})
    if len(starts) < 2:
        return 4
    return round(statistics.median(b - a for a, b in zip(starts, starts[1:])))


def to_song(
    title: str,
    beat_result: BeatResult,
    segments: list[ChordSegment],
    meter: int | None,
    has_sections: bool,
    chroma: tuple | None = None,
    add_tau: float | None = None,
) -> Song:
    """Quantization (§3.3 steps 4-6, with 5b when `add_tau` is set): model outputs -> Song. `chroma` = `add_heuristic.chroma`."""
    refine = None
    if add_tau is not None:
        features, times = chroma
        refine = functools.partial(
            add_heuristic.refine_add, chroma=features, chroma_times=times, beats=beat_result.beats, tau=add_tau
        )
    return build_song(
        title,
        segments,
        beat_result.beats,
        beat_result.downbeats,
        meter or beats_per_bar(beat_result.beats, beat_result.downbeats),
        beat_result.segments if has_sections else None,
        refine,
    )


def transcribe(
    audio: str | Path,
    out: str | Path,
    meter: int | None,
    has_sections: bool,
    beats: str = "auto",
    chords: str = "auto",
    separate: bool = False,
    add_tau: float | None = None,
) -> Transcription:
    tracker = _adapter("beat tracker", _BEATS, beats)
    recognizer = _adapter("chord model", _CHORDS, chords)
    separator = _adapter("source separation", _SEPARATE, "demucs") if separate else None

    with tempfile.TemporaryDirectory() as tmp:
        events.progress("decode", 5)
        wav = decode(audio, tmp)
        harmonic = wav
        if separator:
            events.progress("separate", 20)
            with _stdout_to_stderr():
                harmonic = separator.harmonic_stem(wav, Path(tmp))
        events.progress("beats", 40)
        with _stdout_to_stderr():
            beat_result = tracker.track(wav, meter)
        events.progress("chords", 70)
        with _stdout_to_stderr():
            segments = recognizer.recognize(harmonic)

        events.progress("quantize", 90)
        chroma = add_heuristic.chroma(harmonic) if add_tau is not None else None
        song = to_song(Path(audio).stem, beat_result, segments, meter, has_sections, chroma, add_tau)

    events.progress("write", 100)
    write(song, Path(out))
    return Transcription(song, beat_result, segments)
