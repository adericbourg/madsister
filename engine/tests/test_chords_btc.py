import pytest

from fixtures.make_clip import PROGRESSION
from madsister_engine.quantize import beat_labels


@pytest.mark.slow
def test_recognize_btc_of_synthetic_clip_finds_progression_on_most_bars(make_wav):
    from madsister_engine.chords.btc import recognize

    # Given the synthetic clip: one chord per 2.0 s bar
    wav = make_wav()

    # When
    segments = recognize(wav)

    # Then the majority label matches on >= 6 of 8 bars
    bar_labels = [label for label, _ in beat_labels(segments, [2.0 * i for i in range(8)])]
    assert sum(a == b for a, b in zip(bar_labels, PROGRESSION)) >= 6, bar_labels
    assert all(0.0 < s.confidence <= 1.0 for s in segments)
