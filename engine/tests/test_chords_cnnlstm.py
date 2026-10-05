import pytest

from fixtures.make_clip import PROGRESSION
from madsister_engine.models import ensure_repo
from madsister_engine.quantize import beat_labels


@pytest.mark.slow
def test_recognize_cnnlstm_of_synthetic_clip_finds_progression_on_most_bars(make_wav):
    from madsister_engine.chords.cnnlstm import recognize

    # Given the synthetic clip: one chord per 2.0 s bar
    wav = make_wav()

    # When
    segments = recognize(wav)

    # Then the majority label matches on >= 6 of 8 bars
    bar_labels = [label for label, _ in beat_labels(segments, [2.0 * i for i in range(8)])]
    assert sum(a == b for a, b in zip(bar_labels, PROGRESSION)) >= 6, bar_labels
    assert all(0.0 < s.confidence <= 1.0 for s in segments)


@pytest.mark.slow
def test_chord_list_templates_all_decode_to_distinct_known_chords():
    import sys

    from madsister_engine.chords.cnnlstm import _CHORD_LIST, _REPO, prepare
    from madsister_engine.chords.labels import to_harte

    # Given the shipped chord list and upstream's `Chord` parser
    prepare()
    repo = ensure_repo(*_REPO)
    sys.path.insert(0, str(repo))
    import numpy as np

    np.int = int
    from complex_chord import Chord

    names = [line.strip() for line in _CHORD_LIST.read_text().splitlines() if ":" in line]

    # When each name is turned into the component array the decoder keys its states on
    arrays = [tuple(Chord(name).to_numpy()) for name in names]

    # Then none is dropped by the decoder (-2), none duplicates another, and each maps to an app quality
    assert all(-2 not in array for array in arrays)
    assert len(set(arrays)) == len(names)
    assert [to_harte(name) for name in names]
