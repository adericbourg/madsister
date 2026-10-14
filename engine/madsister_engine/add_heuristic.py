"""add2/add4 (and minor add2/add4) refinement from chroma (spec §3.3 step 5b, D6). Disabled (tau=None) until the bench validates it."""

from collections.abc import Callable
from dataclasses import replace
from pathlib import Path

import numpy as np

from madsister_engine.song import Bar, ChordSlot

_PITCH_CLASSES = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}


def _pitch_class(root: str) -> int:
    return (_PITCH_CLASSES[root[0]] + root.count("#") - root.count("b")) % 12


def _refine(slot: ChordSlot, chroma: np.ndarray, frames: np.ndarray, tau: float, flag_threshold: float) -> ChordSlot:
    root, _, quality = slot.chord.partition(":")
    # ponytail: inversions (`C:maj/3`) are skipped because `add2/3` isn't worth the ambiguity, and a minor 9th voicing
    # (no b7 check) reads as `minadd2`; extend if the bench shows real cases.
    third = {"maj": 4, "min": 3}.get(quality)
    if third is None or slot.beats < 2 or not frames.any():
        return slot
    mean = chroma[:, frames].mean(axis=1)
    pc = _pitch_class(root)
    triad = mean[[pc, (pc + third) % 12, (pc + 7) % 12]].mean()
    for degree, label in ((2, "add2"), (5, "add4")):  # the 2nd wins when both pass
        if mean[(pc + degree) % 12] >= tau * triad:
            confidence = min(c for c in (slot.confidence, flag_threshold - 0.01) if c is not None)
            prefix = "min" if quality == "min" else ""
            return replace(slot, chord=f"{root}:{prefix}{label}", confidence=confidence)
    return slot


def refine_add(
    bars: list[Bar],
    chroma: np.ndarray,
    chroma_times: np.ndarray,
    beats: list[float],
    tau: float | None,
    flag_threshold: float = 0.5,
) -> list[Bar]:
    """Relabel `maj` (`min`) slots of ≥ 2 beats as `add2`/`add4` (`minadd2`/`minadd4`) when that degree's mean chroma
    ≥ tau × the triad tones' mean.

    `chroma` is (12, T) with C at row 0, `chroma_times` the T frame times. Relabelled slots get a confidence below
    `flag_threshold` so the user reviews them. `tau=None` returns `bars` unchanged.
    """
    if tau is None:
        return bars
    return map_slots(bars, chroma_times, beats, lambda slot, frames: _refine(slot, chroma, frames, tau, flag_threshold))


def map_slots(
    bars: list[Bar], chroma_times: np.ndarray, beats: list[float], fn: Callable[[ChordSlot, np.ndarray], ChordSlot]
) -> list[Bar]:
    """Rewrite each slot with `fn(slot, frames)`, `frames` being the mask of the chroma frames during the slot's beats."""
    result = []
    for bar in bars:
        i = min(range(len(beats)), key=lambda k: abs(beats[k] - bar.start_sec))
        slots = []
        for slot in bar.chords:
            start = beats[i]
            end = beats[i + slot.beats] if i + slot.beats < len(beats) else np.inf
            slots.append(fn(slot, (chroma_times >= start) & (chroma_times < end)))
            i += slot.beats
        result.append(replace(bar, chords=slots))
    return result


def chroma(wav_path: str | Path, **register) -> tuple[np.ndarray, np.ndarray]:
    """CQT chroma (12, T) of the harmonic stem, hop 2048, with the frame times. Needs librosa (chords-* groups).

    `register`: `chroma_cqt`'s `fmin` and `n_octaves`, to keep a range only (`bass.chroma`)."""
    import librosa

    y, sr = librosa.load(str(wav_path), sr=None, mono=True)
    hop = 2048
    features = librosa.feature.chroma_cqt(y=y, sr=sr, hop_length=hop, **register)
    return features, librosa.frames_to_time(np.arange(features.shape[1]), sr=sr, hop_length=hop)
