"""The §3.3 transcription pipeline: audio file -> Song JSON, with progress events."""

import contextlib
import dataclasses
import functools
import hashlib
import importlib
import importlib.util
import os
import statistics
import sys
import tempfile
from pathlib import Path

from madsister_engine import add_heuristic, events
from madsister_engine.beats import BeatResult
from madsister_engine.decode import decode
from madsister_engine.quantize import ChordSegment, build_song
from madsister_engine.song import Audio, Song, write

# option -> (dependency group, module that only that group installs, adapter module). `auto` = first installed, in order:
# madmom + cnnlstm, the fewest edits on the M0 bench (provisional, GuitarSet proxy: doc/roadmap/m0-results.md).
_BEATS = {
    "madmom": ("beats-madmom", "madmom", "madsister_engine.beats.madmom_tracker"),
    "allinone": ("beats-allinone", "allin1", "madsister_engine.beats.allinone_tracker"),
}
_CHORDS = {
    "cnnlstm": ("chords-cnnlstm", "pydub", "madsister_engine.chords.cnnlstm"),
    "btc": ("chords-btc", "mir_eval", "madsister_engine.chords.btc"),
}
_SEPARATE = {"demucs": ("separate", "demucs", "madsister_engine.separate")}
# --meter -> (tracker beats per bar, Song meter unit). 6/8 is tracked at the dotted-quarter pulse (spec §3.2).
METERS = {"3": (3, 4), "4": (4, 4), "6/8": (2, 8)}


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


def setup(has_sections: bool, is_all: bool) -> None:
    """Download what the default models need (+ all-in-one with `has_sections`, + the bench-only ones with `is_all`)
    into `models_dir()`, so transcription then runs offline (D5). Each adapter's `prepare()` is idempotent."""
    items = [("chord model", _CHORDS, "cnnlstm")]  # madmom ships its weights in the package
    if has_sections or is_all:
        items.append(("beat tracker", _BEATS, "allinone"))
    if is_all:
        items += [("chord model", _CHORDS, "btc"), ("source separation", _SEPARATE, "demucs")]
    for done, item in enumerate(items, 1):
        with _stdout_to_stderr():
            _adapter(*item).prepare()
        events.progress("setup", 100 * done // len(items))


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
    meter: str | None,
    has_sections: bool,
    chroma: tuple | None = None,
    add_tau: float | None = None,
) -> Song:
    """Quantization (§3.3 steps 4-6, with 5b when `add_tau` is set): model outputs -> Song. `chroma` = `add_heuristic.chroma`."""
    if len(beat_result.beats) < 2:
        raise ValueError("could not find beats in this audio")
    refine = None
    if add_tau is not None:
        features, times = chroma
        refine = functools.partial(
            add_heuristic.refine_add, chroma=features, chroma_times=times, beats=beat_result.beats, tau=add_tau
        )
    meter_beats, unit = METERS[meter] if meter else (beats_per_bar(beat_result.beats, beat_result.downbeats), 4)
    return build_song(
        title,
        segments,
        beat_result.beats,
        beat_result.downbeats,
        meter_beats,
        beat_result.segments if has_sections else None,
        refine,
        unit,
    )


def transcribe(
    audio: str | Path,
    out: str | Path,
    meter: str | None,
    has_sections: bool,
    beats: str = "auto",
    chords: str = "auto",
    separate: bool = False,
    add_tau: float | None = None,
) -> None:
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
            beat_result = tracker.track(wav, METERS[meter][0] if meter else None)
        events.progress("chords", 70)
        with _stdout_to_stderr():
            segments = recognizer.recognize(harmonic)

        events.progress("quantize", 90)
        chroma = add_heuristic.chroma(harmonic) if add_tau is not None else None
        song = to_song(Path(audio).stem, beat_result, segments, meter, has_sections, chroma, add_tau)

    with open(audio, "rb") as f:
        sha256 = hashlib.file_digest(f, "sha256").hexdigest()
    song = dataclasses.replace(song, audio=Audio(Path(audio).name, sha256))
    events.progress("write", 100)
    write(song, Path(out))
