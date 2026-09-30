import pytest

from madsister_engine.chords.labels import merge_frames, to_harte
from madsister_engine.quantize import ChordSegment


def test_to_harte_maps_btc_labels_to_canonical_harte():
    # Given BTC large-vocabulary names, When mapping, Then only X and bare roots change
    assert to_harte("X") == "N"
    assert to_harte("N") == "N"
    assert to_harte("C") == "C:maj"
    assert to_harte("C#:maj6") == "C#:maj6"
    assert to_harte("A#:hdim7") == "A#:hdim7"


def test_to_harte_of_unknown_quality_raises():
    # Given a quality outside the §4.1 set, When mapping, Then it doesn't survive
    with pytest.raises(ValueError):
        to_harte("C:maj9")


def test_merge_frames_merges_equal_consecutive_labels_with_mean_confidence():
    # Given 5 frames of 0.1 s: C C G G C
    labels = ["C:maj", "C:maj", "G:maj", "G:maj", "C:maj"]
    confidences = [0.9, 0.7, 0.4, 0.6, 1.0]

    # When
    segments = merge_frames(labels, confidences, frame_duration=0.1)

    # Then
    assert segments == [
        ChordSegment(0.0, pytest.approx(0.2), "C:maj", pytest.approx(0.8)),
        ChordSegment(pytest.approx(0.2), pytest.approx(0.4), "G:maj", pytest.approx(0.5)),
        ChordSegment(pytest.approx(0.4), pytest.approx(0.5), "C:maj", pytest.approx(1.0)),
    ]
    assert merge_frames([], [], 0.1) == []
