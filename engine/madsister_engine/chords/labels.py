"""Torch-free chord model helpers: model labels -> Harte, frame predictions -> chord segments."""

import itertools
import statistics

from madsister_engine.quantize import NO_CHORD, ChordSegment

QUALITIES = {"maj", "min", "dim", "aug", "maj6", "min6", "7", "maj7", "min7", "minmaj7", "dim7", "hdim7", "sus2", "sus4"}
QUALITIES |= {"add2", "add4"}  # the §4 extension (GuitarSet `maj(2)` / `maj(11)`)


# Extended qualities some models (and GuitarSet) emit, mapped to the nearest §4.1 quality (the bass is kept).
_DEGRADED = {
    "9": "7", "11": "7", "13": "7", "sus4(b7)": "7", "maj9": "maj7", "min9": "min7", "maj(2)": "add2", "maj(11)": "add4"
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
