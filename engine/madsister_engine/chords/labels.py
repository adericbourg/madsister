"""Torch-free chord model helpers: model labels -> Harte, frame predictions -> chord segments."""

import itertools
import statistics

from madsister_engine.quantize import NO_CHORD, ChordSegment

QUALITIES = {"maj", "min", "dim", "aug", "maj6", "min6", "7", "maj7", "min7", "minmaj7", "dim7", "hdim7", "sus2", "sus4"}


# Extended qualities some models emit, mapped to the nearest §4.1 quality (the bass is kept).
_DEGRADED = {"9": "7", "11": "7", "13": "7", "sus4(b7)": "7", "maj9": "maj7", "min9": "min7"}


def to_harte(label: str) -> str:
    """Model label -> canonical Harte: `X` -> `N`, bare root -> `maj`, extended qualities degraded, inversion kept."""
    if label in ("N", "X"):
        return NO_CHORD
    root, _, quality = label.partition(":")
    quality, slash, bass = (quality or "maj").partition("/")
    quality = _DEGRADED.get(quality, quality)
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
