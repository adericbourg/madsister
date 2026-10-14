"""Slash chords from the lowest notes (bass detection, provisional: benchmarked with `bench/run.py --bass-tau`)."""

from dataclasses import replace
from pathlib import Path

import numpy as np

from madsister_engine import add_heuristic
from madsister_engine.add_heuristic import _pitch_class, map_slots
from madsister_engine.quantize import NO_CHORD
from madsister_engine.song import Bar, ChordSlot

DEGREES = ["1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7"]  # Harte bass degrees, by semitones above the root
# ponytail: a fixed low register, E1 (a bass guitar's open E) to ~E3; a calibration knob if the bench asks for one.
LOW_OCTAVES = 2


def _refine(slot: ChordSlot, chroma: np.ndarray, frames: np.ndarray, tau: float) -> ChordSlot:
    if slot.chord in (NO_CHORD, "%") or not frames.any():
        return slot
    mean = chroma[:, frames].mean(axis=1)
    if not mean.any():
        return slot
    root, _, rest = slot.chord.partition(":")
    chord = f"{root}:{rest.partition('/')[0]}"
    lowest, root_pc = int(mean.argmax()), _pitch_class(root)
    if lowest == root_pc:
        return replace(slot, chord=chord)  # the root is in the bass: a predicted slash was wrong
    if mean[lowest] >= tau * mean[root_pc]:
        return replace(slot, chord=f"{chord}/{DEGREES[(lowest - root_pc) % 12]}")
    return slot


def refine_bass(bars: list[Bar], chroma: np.ndarray, chroma_times: np.ndarray, beats: list[float], tau: float | None) -> list[Bar]:
    """Set each chord's bass from the strongest low pitch class over its beats, when it is ≥ tau × the root's.

    `chroma` is the low-register chroma (`chroma()`), (12, T) with C at row 0. The confidence is kept. `tau=None` is a no-op.
    """
    if tau is None:
        return bars
    return map_slots(bars, chroma_times, beats, lambda slot, frames: _refine(slot, chroma, frames, tau))


def chroma(wav_path: str | Path) -> tuple[np.ndarray, np.ndarray]:
    """`add_heuristic.chroma` of the low register only: E1 to ~E3."""
    return add_heuristic.chroma(wav_path, fmin=41.2, n_octaves=LOW_OCTAVES)
