import subprocess
import wave

import pytest

from madsister_engine.decode import decode, tags


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


@pytest.mark.parametrize("ext", ["mp3", "ogg", "m4a"])
def test_tags_of_tagged_file_reads_title_and_artist_unescaped(make_wav, tmp_path, ext):
    # Given the clip encoded with tags that need escaping (Ogg keeps them on the stream, not globally)
    src = tmp_path / f"tagged.{ext}"
    cmd = ["ffmpeg", "-v", "error", "-i", str(make_wav()), "-metadata", "title=A=B; c\\d\\", "-metadata", "artist=Ünï"]
    subprocess.run([*cmd, str(src)], check=True)

    # When
    found = tags(src)

    # Then
    assert (found["title"], found["artist"]) == ("A=B; c\\d\\", "Ünï")


def test_tags_of_untagged_or_non_audio_file_has_no_title(make_wav, tmp_path):
    # Given an untagged wav and a text file
    notes = tmp_path / "notes.txt"
    notes.write_text("not audio")

    # When / Then
    assert "title" not in tags(make_wav())
    assert tags(notes) == {}
