import time
import wave

import numpy as np
import pytest


def _read(path):
    with wave.open(str(path)) as wav:
        samples = np.frombuffer(wav.readframes(wav.getnframes()), dtype="<i2") / 32768
        return wav.getnchannels(), wav.getframerate(), samples


@pytest.mark.slow
def test_harmonic_stem_of_synthetic_clip_writes_quieter_mono_44k_wav_of_same_duration(make_wav, tmp_path):
    from madsister_engine.separate import harmonic_stem

    # Given the synthetic clip (chords + bass + kicks)
    wav = make_wav()

    # When
    start = time.perf_counter()
    out = harmonic_stem(wav, tmp_path / "stems")
    print(f"harmonic_stem wall time on the 16 s clip: {time.perf_counter() - start:.1f} s")

    # Then the stem is mono 44.1 kHz, same duration, with less energy than the mix (kicks removed)
    _, _, mix = _read(wav)
    channels, rate, stem = _read(out)
    assert (channels, rate) == (1, 44100)
    assert len(stem) / rate == pytest.approx(len(mix) / 44100, abs=0.1)
    assert 0 < np.sqrt(np.mean(stem**2)) < np.sqrt(np.mean(mix**2))
