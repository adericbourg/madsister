import wave

import pytest

from madsister_engine.decode import decode


def test_decode_of_stereo_22k_wav_yields_mono_44k_wav(make_wav, tmp_path):
    # Given the synthetic clip at 22.05 kHz stereo
    src = make_wav(rate=22050, channels=2)

    # When
    out = decode(src, tmp_path / "out")

    # Then
    with wave.open(str(out)) as wav:
        assert (wav.getnchannels(), wav.getframerate()) == (1, 44100)
        assert wav.getnframes() / 44100 == pytest.approx(16, abs=0.1)


def test_decode_of_non_audio_file_raises_with_ffmpeg_message(tmp_path):
    # Given a text file
    src = tmp_path / "notes.txt"
    src.write_text("not audio")

    # When / Then
    with pytest.raises(RuntimeError, match="Invalid data"):
        decode(src, tmp_path / "out")
