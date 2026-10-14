import time
import wave

import numpy as np
import pytest


def _read(path):
    with wave.open(str(path)) as wav:
        samples = np.frombuffer(wav.readframes(wav.getnframes()), dtype="<i2") / 32768
        return wav.getnchannels(), wav.getframerate(), samples


@pytest.mark.slow
def test_stems_of_synthetic_clip_writes_quieter_mono_44k_harmonic_and_bass_wavs_of_same_duration(make_wav, tmp_path):
    from madsister_engine.separate import stems

    # Given the synthetic clip (chords + bass + kicks)
    wav = make_wav()

    # When
    start = time.perf_counter()
    harmonic, bass = stems(wav, tmp_path / "stems")
    print(f"stems wall time on the 16 s clip: {time.perf_counter() - start:.1f} s")

    # Then both stems are mono 44.1 kHz, same duration, with less energy than the mix (kicks removed, chords out of the bass)
    _, _, mix = _read(wav)
    assert (harmonic.name, bass.name) == ("harmonic.wav", "bass.wav")
    for out in (harmonic, bass):
        channels, rate, stem = _read(out)
        assert (channels, rate) == (1, 44100)
        assert len(stem) / rate == pytest.approx(len(mix) / 44100, abs=0.1)
        assert 0 < np.sqrt(np.mean(stem**2)) < np.sqrt(np.mean(mix**2))
