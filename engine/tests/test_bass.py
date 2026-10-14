import numpy as np
import pytest

from madsister_engine.bass import refine_bass
from madsister_engine.song import Bar, ChordSlot

BEATS = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5]  # two 4/4 bars at 120 BPM
TIMES = np.arange(0, 4, 0.05)
C, D, E, G = 0, 2, 4, 7


def _chroma(strong: int, root_level: float = 0.5) -> np.ndarray:
    """Low-register chroma: C at `root_level` everywhere, `strong` at 1.0 over the first bar only."""
    chroma = np.full((12, len(TIMES)), 0.05)
    chroma[C] = root_level
    chroma[strong, TIMES < 2.0] = 1.0
    return chroma


def _chords(bars):
    return [[s.chord for s in bar.chords] for bar in bars]


def test_refine_bass_of_slot_with_strong_low_note_sets_or_removes_the_bass():
    # Given C major (no bass) then C over E (a wrong slash), keeping their confidence
    bars = [Bar([ChordSlot("C:maj", 4, 0.9)], start_sec=0.0), Bar([ChordSlot("C:maj/3", 4, 0.8)], start_sec=2.0)]

    # When E is the strongest low note over the first bar (twice the root), then over no bar
    with_e = refine_bass(bars, _chroma(E), TIMES, BEATS, tau=1.5)
    with_c = refine_bass(bars, _chroma(C, root_level=1.0), TIMES, BEATS, tau=1.5)

    # Then the first bar gets /3 and the second, where C is the lowest note, loses its slash; confidences stay
    assert _chords(with_e) == [["C:maj/3"], ["C:maj"]]
    assert [s.confidence for bar in with_e for s in bar.chords] == [0.9, 0.8]
    assert _chords(with_c) == [["C:maj"], ["C:maj"]]


def test_refine_bass_skips_weak_or_non_chord_tone_low_notes_no_chord_and_silence():
    # Given C major, then N
    bars = [Bar([ChordSlot("C:maj", 4)], start_sec=0.0), Bar([ChordSlot("N", 4)], start_sec=2.0)]

    # When G is only 1.25× the root over the first bar (under τ), or D (not a tone of C major) is strong there, or E is
    # strong over N, or the chroma is silent
    weak = refine_bass(bars, _chroma(G, root_level=0.8), TIMES, BEATS, tau=1.5)
    passing = refine_bass(bars, _chroma(D), TIMES, BEATS, tau=1.5)
    over_n = refine_bass(bars, _chroma(E)[:, ::-1], TIMES, BEATS, tau=1.5)
    silent = refine_bass(bars, np.zeros((12, len(TIMES))), TIMES, BEATS, tau=1.5)

    # Then nothing changes
    assert weak == bars
    assert passing == bars
    assert over_n == bars
    assert silent == bars


def test_refine_bass_when_tau_is_none_is_a_no_op():
    # Given C major with a strong E
    bars = [Bar([ChordSlot("C:maj", 4)], start_sec=0.0)]

    # When / Then
    assert refine_bass(bars, _chroma(E), TIMES, BEATS, tau=None) == bars


@pytest.mark.slow
def test_chroma_of_synthetic_clip_returns_twelve_bins_per_hop(make_wav):
    from madsister_engine.bass import chroma

    # When
    features, times = chroma(make_wav())

    # Then one 12-bin frame every 2048 samples
    assert features.shape == (12, len(times))
    assert times[1] - times[0] == pytest.approx(2048 / 44100)
