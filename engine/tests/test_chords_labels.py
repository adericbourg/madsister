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


def test_to_harte_of_cnnlstm_labels_keeps_bass_and_degrades_extended_qualities():
    # Given Chord-CNN-LSTM `submission` dictionary names, When mapping,
    # Then inversions are kept and qualities outside §4.1 fall back to the nearest one, bass included
    assert to_harte("Eb:maj/3") == "Eb:maj/3"
    assert to_harte("A:min/b3") == "A:min/b3"
    assert to_harte("G:maj/b7") == "G:maj/b7"
    assert to_harte("C:9") == "C:7"
    assert to_harte("C:11") == "C:7"
    assert to_harte("C:13") == "C:7"
    assert to_harte("C:maj9") == "C:maj7"
    assert to_harte("D:min9") == "D:min7"
    assert to_harte("F#:sus4(b7)") == "F#:7"
    assert to_harte("C:9/b7") == "C:7/b7"


def test_to_harte_of_guitarset_performed_labels_strips_extensions_and_root_bass():
    # Given GuitarSet performed-chord labels (full Harte: degree lists, `/1` bass, interval sets),
    # When mapping, Then extensions are dropped, `/1` is no bass, and a set without a named quality is `maj`
    assert to_harte("C#:maj/1") == "C#:maj"
    assert to_harte("A:7(*5)/1") == "A:7"
    assert to_harte("F:maj(#11)/b5") == "F:maj/b5"
    assert to_harte("B:hdim7(11,*1)/4") == "B:hdim7/4"
    assert to_harte("E:(1,5)/1") == "E:maj"
    assert to_harte("D:maj(2)/1") == "D:add2"
    assert to_harte("D:maj(11)/4") == "D:add4/4"
    assert to_harte("D:sus4(b7)/1") == "D:7"


def test_to_harte_of_unknown_quality_raises():
    # Given a quality outside the §4.1 set, When mapping, Then it doesn't survive
    with pytest.raises(ValueError):
        to_harte("C:maj13")


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
