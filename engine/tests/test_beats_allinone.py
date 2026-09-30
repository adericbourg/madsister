import numpy as np
import pytest


@pytest.mark.slow
def test_track_of_synthetic_clip_finds_120_bpm_downbeats_every_2s_and_segments(make_wav):
    from madsister_engine.beats.allinone_tracker import track

    # Given the synthetic 120 BPM 4/4 clip (downbeats every 2.0 s)
    wav = make_wav()

    # When
    result = track(wav, meter=None)

    # Then the beat period is within 5% of 0.5 s
    assert np.median(np.diff(result.beats)) == pytest.approx(0.5, rel=0.05)
    # And >= 75% of the downbeats are within 70 ms of a true downbeat
    downbeats = np.array(result.downbeats)
    errors = np.abs(downbeats - 2.0 * np.round(downbeats / 2.0))
    assert len(downbeats) > 0 and np.mean(errors <= 0.07) >= 0.75
    # And segments are (start, end, label) tuples
    assert result.segments
    assert all(isinstance(s, float) and isinstance(e, float) and isinstance(label, str) for s, e, label in result.segments)
