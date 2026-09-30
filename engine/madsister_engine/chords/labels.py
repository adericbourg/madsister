"""Torch-free chord model helpers: model labels -> Harte, frame predictions -> chord segments."""

import itertools
import statistics

from madsister_engine.quantize import NO_CHORD, ChordSegment

QUALITIES = {"maj", "min", "dim", "aug", "maj6", "min6", "7", "maj7", "min7", "minmaj7", "dim7", "hdim7", "sus2", "sus4"}


def to_harte(btc_label: str) -> str:
    """BTC writes major as a bare root and has an `X` (unknown chord) class; canonical Harte spells `maj` and has no `X`."""
    if btc_label in ("N", "X"):
        return NO_CHORD
    root, _, quality = btc_label.partition(":")
    quality = quality or "maj"
    if quality not in QUALITIES:
        raise ValueError(f"unsupported chord quality: {btc_label}")
    return f"{root}:{quality}"


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
