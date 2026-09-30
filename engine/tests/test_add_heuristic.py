import numpy as np
import pytest

from madsister_engine.add_heuristic import refine_add
from madsister_engine.song import Bar, ChordSlot

BEATS = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5]  # two 4/4 bars at 120 BPM
TIMES = np.arange(0, 4, 0.05)
C, D, E, F, G, A = 0, 2, 4, 5, 7, 9


def _chroma(*extra: int, first_bar_only: bool = False) -> np.ndarray:
    """C-major triad everywhere, plus strong `extra` pitch classes (over the first bar only, if asked)."""
    chroma = np.full((12, len(TIMES)), 0.05)
    chroma[[C, E, G, A]] = 1.0  # A: the A:sus4 / A:min slots also get their tones
    for pc in extra:
        chroma[pc, TIMES < 2.0 if first_bar_only else slice(None)] = 1.0
    return chroma


def _labels(bars):
    return [[(s.chord, s.confidence) for s in bar.chords] for bar in bars]


def test_refine_add_of_major_slot_relabels_by_strong_second_or_fourth_and_caps_confidence():
    # Given C major over a whole bar, then over another whole bar
    bars = [Bar([ChordSlot("C:maj", 4, 0.9)], start_sec=0.0), Bar([ChordSlot("C:maj", 4, 0.3)], start_sec=2.0)]

    # When D (resp. F, both, neither) is strong over the first bar only
    with_d = refine_add(bars, _chroma(D, first_bar_only=True), TIMES, BEATS, tau=0.6)
    with_f = refine_add(bars, _chroma(F, first_bar_only=True), TIMES, BEATS, tau=0.6)
    with_both = refine_add(bars, _chroma(D, F, first_bar_only=True), TIMES, BEATS, tau=0.6)
    with_neither = refine_add(bars, _chroma(), TIMES, BEATS, tau=0.6)

    # Then only the first bar is relabelled, its confidence capped below the 0.5 flag threshold; the 2nd wins over the 4th
    assert _labels(with_d) == [[("C:add2", 0.49)], [("C:maj", 0.3)]]
    assert _labels(with_f) == [[("C:add4", 0.49)], [("C:maj", 0.3)]]
    assert _labels(with_both)[0] == [("C:add2", 0.49)]
    assert with_neither == bars


def test_refine_add_skips_sus_minor_inversions_and_one_beat_slots():
    # Given strong D and F everywhere (sus4 of A has D; C:maj/3 and a 1-beat C would otherwise qualify)
    bars = [
        Bar([ChordSlot("A:sus4", 2), ChordSlot("A:min", 2)], start_sec=0.0),
        Bar([ChordSlot("C:maj/3", 3), ChordSlot("C:maj", 1)], start_sec=2.0),
    ]

    # When
    result = refine_add(bars, _chroma(D, F), TIMES, BEATS, tau=0.1)

    # Then nothing changes
    assert result == bars


def test_refine_add_when_tau_is_none_is_a_no_op():
    # Given a C major slot with a strong D
    bars = [Bar([ChordSlot("C:maj", 4, 0.9)], start_sec=0.0)]

    # When
    result = refine_add(bars, _chroma(D), TIMES, BEATS, tau=None)

    # Then
    assert result == bars


@pytest.mark.slow
def test_chroma_of_synthetic_clip_returns_twelve_bins_per_hop(make_wav):
    from madsister_engine.add_heuristic import chroma

    # When
    features, times = chroma(make_wav())

    # Then one 12-bin frame every 2048 samples
    assert features.shape == (12, len(times))
    assert times[1] - times[0] == pytest.approx(2048 / 44100)
