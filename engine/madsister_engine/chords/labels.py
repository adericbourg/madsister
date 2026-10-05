"""Torch-free chord model helpers: model labels -> Harte, frame predictions -> chord segments."""

import itertools
import statistics

from madsister_engine.quantize import NO_CHORD, ChordSegment

QUALITIES = {"maj", "min", "dim", "aug", "maj6", "min6", "7", "maj7", "min7", "minmaj7", "dim7", "hdim7", "sus2", "sus4"}
QUALITIES |= {"9", "maj9", "min9", "11", "13", "min11", "maj13"}
QUALITIES |= {"add2", "add4", "minadd2", "minadd4", "7sus4", "9sus4", "69", "min69"}  # the §4 extensions


# Extended qualities some models (and GuitarSet) emit, mapped to the app's quality (the bass is kept). Other
# `(degree, ...)` lists are dropped by `to_harte`; a quality listed here wins over that.
_DEGRADED = {
    "min13": "min11",
    "sus4(b7)": "7sus4", "sus4(b7,9)": "9sus4",
    "maj(2)": "add2", "maj(9)": "add2", "maj(4)": "add4", "maj(11)": "add4",
    "min(2)": "minadd2", "min(9)": "minadd2", "min(4)": "minadd4", "min(11)": "minadd4",
    "maj6(9)": "69", "min6(9)": "min69",
}


def to_harte(label: str) -> str:
    """Model label -> canonical Harte: `X` -> `N`, bare root -> `maj`, extended qualities degraded, inversion kept.

    Other `(degree, ...)` lists are dropped and a `/1` bass (the root) removed.
    """
    if label in ("N", "X"):
        return NO_CHORD
    root, _, quality = label.partition(":")
    quality, slash, bass = (quality or "maj").partition("/")
    if bass == "1":
        slash = bass = ""
    # ponytail: an interval set with no named quality (`(1,5)` power chord, `(1)` single note) becomes `maj`,
    # even the rare `(1,b5,b7)`; derive the quality from the degrees if the bench shows it matters.
    quality = _DEGRADED.get(quality, quality.partition("(")[0] or "maj")
    if quality not in QUALITIES:
        raise ValueError(f"unsupported chord quality: {label}")
    return f"{root}:{quality}{slash}{bass}"


def merge_frames(labels: list[str], confidences: list[float], frame_duration: float) -> list[ChordSegment]:
    """Merge consecutive equal frame labels; a segment's confidence is the mean of its frames' confidences."""
    segments = []
    start = 0
    for label, group in itertools.groupby(zip(labels, confidences), key=lambda item: item[0]):
        frames = [c for _, c in group]
        end = start + len(frames)
        segments.append(ChordSegment(start * frame_duration, end * frame_duration, label, statistics.fmean(frames)))
        start = end
    return segments
