import io
import json
import signal
import sys
import threading
import types
import wave

import numpy as np
import pytest

from madsister_engine import cli, pipeline


def _lines(capfd):
    return [json.loads(line) for line in capfd.readouterr().out.splitlines()]


@pytest.fixture
def youtube_dl(monkeypatch):
    """A fake `yt_dlp` whose download reports `calls["hooks"]` to the progress hooks, or raises `calls["raise"]`."""
    calls = {"hooks": []}

    class YoutubeDL:
        def __init__(self, params):
            calls["params"] = params

        def __enter__(self):
            return self

        def __exit__(self, *exc):
            return False

        def extract_info(self, url, download):
            calls["url"] = url
            if "raise" in calls:
                raise calls["raise"]
            for status in calls["hooks"]:
                for hook in calls["params"]["progress_hooks"]:
                    hook(status)
            return {"requested_downloads": [{"filepath": str(calls["filepath"])}]}

    monkeypatch.setitem(sys.modules, "yt_dlp", types.SimpleNamespace(YoutubeDL=YoutubeDL))
    monkeypatch.setattr(pipeline, "_installed", lambda module: True)
    return calls


@pytest.fixture
def microphone(monkeypatch):
    """A fake `sounddevice` whose stream feeds `mic["blocks"]` to the callback, then sends SIGINT if `mic["sigint"]`,
    or whose opening raises `mic["raise"]`."""
    mic = {"blocks": [], "sigint": False}

    class InputStream:
        def __init__(self, **params):
            if "raise" in mic:
                raise mic["raise"]
            mic["params"] = params

        def __enter__(self):
            for block in mic["blocks"]:
                mic["params"]["callback"](block, len(block), None, None)
            if mic["sigint"]:
                signal.getsignal(signal.SIGINT)(signal.SIGINT, None)
            return self

        def __exit__(self, *exc):
            return False

    monkeypatch.setitem(sys.modules, "sounddevice", types.SimpleNamespace(InputStream=InputStream))
    monkeypatch.setattr(pipeline, "_installed", lambda module: True)
    return mic


def _half_seconds(count):
    return [np.full((22050, 1), i, dtype=np.int16) for i in range(count)]


def test_main_when_command_raises_emits_error_and_exits_1(capfd, tmp_path):
    # Given a missing audio file
    missing = tmp_path / "missing.mp3"

    # When
    code = cli.main(["transcribe", str(missing), "--out", str(tmp_path / "s.json")])

    # Then
    assert code == 1
    lines = _lines(capfd)
    assert len(lines) == 1
    assert lines[0]["type"] == "error"
    assert "missing.mp3" in lines[0]["message"]


def test_main_of_fetch_emits_download_progress_and_audio_path(youtube_dl, capfd, tmp_path):
    # Given a download reporting bytes, with an unknown then an estimated total
    youtube_dl["hooks"] = [
        {"status": "downloading", "downloaded_bytes": 10, "total_bytes": None},
        {"status": "downloading", "downloaded_bytes": 50, "total_bytes": 200},
        {"status": "downloading", "downloaded_bytes": 51, "total_bytes": 200},
        {"status": "downloading", "downloaded_bytes": 400, "total_bytes_estimate": 400},
        {"status": "finished", "downloaded_bytes": 400, "total_bytes": 400},
    ]
    youtube_dl["filepath"] = tmp_path / "Song.webm"

    # When
    code = cli.main(["fetch", "https://example.com/watch?v=x", "--out-dir", str(tmp_path)])

    # Then the best audio stream is saved in out-dir, with one event per new percentage
    assert code == 0
    assert youtube_dl["url"] == "https://example.com/watch?v=x"
    assert youtube_dl["params"]["format"] == "bestaudio/best"
    assert youtube_dl["params"]["outtmpl"].startswith(str(tmp_path))
    assert _lines(capfd) == [
        {"type": "progress", "stage": "download", "pct": 25},
        {"type": "progress", "stage": "download", "pct": 100},
        {"type": "result", "path": str(tmp_path / "Song.webm")},
    ]


def test_main_of_fetch_when_url_is_not_http_emits_error_without_downloading(youtube_dl, capfd, tmp_path):
    # When
    code = cli.main(["fetch", "file:///etc/passwd", "--out-dir", str(tmp_path)])

    # Then
    assert code == 1
    assert "params" not in youtube_dl
    assert _lines(capfd) == [{"type": "error", "message": "not an http(s) URL: file:///etc/passwd"}]


def test_main_of_fetch_when_download_fails_emits_its_error(youtube_dl, capfd, tmp_path):
    # Given
    youtube_dl["raise"] = RuntimeError("ERROR: Unsupported URL: https://example.com")

    # When
    code = cli.main(["fetch", "https://example.com", "--out-dir", str(tmp_path)])

    # Then
    assert code == 1
    assert _lines(capfd) == [{"type": "error", "message": "ERROR: Unsupported URL: https://example.com"}]


def test_main_of_record_when_stdin_says_stop_writes_the_recorded_wav(microphone, monkeypatch, capfd, tmp_path):
    # Given 2.5 s of input, then a `stop` line on stdin
    microphone["blocks"] = _half_seconds(5)
    monkeypatch.setattr(sys, "stdin", io.StringIO("noise\nstop\n"))
    out = tmp_path / "take.wav"

    # When
    code = cli.main(["record", "--out", str(out)])

    # Then a mono 44.1 kHz 16-bit WAV holds every block, with one progress event per elapsed second
    assert code == 0
    assert microphone["params"]["samplerate"] == 44100
    assert microphone["params"]["channels"] == 1
    assert microphone["params"]["dtype"] == "int16"
    with wave.open(str(out)) as wav:
        assert (wav.getnchannels(), wav.getsampwidth(), wav.getframerate()) == (1, 2, 44100)
        frames = np.frombuffer(wav.readframes(wav.getnframes()), dtype=np.int16)
    assert np.array_equal(frames, np.concatenate(microphone["blocks"]).ravel())
    assert _lines(capfd) == [
        {"type": "progress", "stage": "record", "pct": 0, "elapsedSec": 0},
        {"type": "progress", "stage": "record", "pct": 0, "elapsedSec": 1},
        {"type": "progress", "stage": "record", "pct": 0, "elapsedSec": 2},
        {"type": "result", "path": str(out)},
    ]


def test_main_of_record_when_interrupted_writes_the_wav_and_restores_sigint(microphone, monkeypatch, capfd, tmp_path):
    # Given 1 s of input, then SIGINT, with stdin left open
    microphone["blocks"] = _half_seconds(2)
    microphone["sigint"] = True
    released = threading.Event()
    monkeypatch.setattr(sys, "stdin", iter(released.wait, True))
    out = tmp_path / "take.wav"

    # When
    code = cli.main(["record", "--out", str(out)])
    released.set()

    # Then
    assert code == 0
    with wave.open(str(out)) as wav:
        assert wav.getnframes() == 44100
    assert _lines(capfd)[-1] == {"type": "result", "path": str(out)}
    assert signal.getsignal(signal.SIGINT) is signal.default_int_handler


def test_main_of_record_when_no_input_device_emits_error_without_writing(microphone, capfd, tmp_path):
    # Given
    microphone["raise"] = RuntimeError("Error querying device -1")
    out = tmp_path / "take.wav"

    # When
    code = cli.main(["record", "--out", str(out)])

    # Then
    assert code == 1
    assert not out.exists()
    assert _lines(capfd) == [{"type": "error", "message": "Error querying device -1"}]
    assert signal.getsignal(signal.SIGINT) is signal.default_int_handler


def test_main_of_setup_when_run_twice_downloads_default_models_once(monkeypatch, capfd, tmp_path):
    from madsister_engine.chords import cnnlstm
    from test_models import _publish_archive

    # Given the default chord model's repo archive, and an empty models dir
    upstream = tmp_path / "upstream"
    sha256 = _publish_archive(upstream, "abc123", {"model.sdict": b"weights"})
    monkeypatch.setattr(cnnlstm, "_REPO", ("chord-cnn-lstm", upstream.as_uri(), "abc123", sha256))
    monkeypatch.setattr(pipeline, "_installed", lambda module: True)
    monkeypatch.setenv("MADSISTER_MODELS_DIR", str(tmp_path / "models"))

    # When setup runs, then runs again with the archive gone
    first = cli.main(["setup"])
    (upstream / "archive" / "abc123.tar.gz").unlink()
    second = cli.main(["setup"])

    # Then the repo is downloaded into the models dir, and the second run downloads nothing
    assert first == second == 0
    assert (tmp_path / "models" / "chord-cnn-lstm" / "model.sdict").read_bytes() == b"weights"
    expected = [
        {"type": "progress", "stage": "setup", "pct": 100},
        {"type": "result", "path": str(tmp_path / "models")},
    ]
    assert _lines(capfd) == expected * 2


def test_main_of_setup_with_sections_or_all_prepares_more_models_with_progress_per_model(monkeypatch, capfd, tmp_path):
    # Given every model adapter installed
    prepared = []
    for module in ("chords.cnnlstm", "beats.allinone_tracker", "chords.btc", "separate"):
        fake = types.SimpleNamespace(prepare=lambda module=module: prepared.append(module))
        monkeypatch.setitem(sys.modules, f"madsister_engine.{module}", fake)
    monkeypatch.setattr(pipeline, "_installed", lambda module: True)
    monkeypatch.setenv("MADSISTER_MODELS_DIR", str(tmp_path))

    # When
    sections = cli.main(["setup", "--sections"])
    sections_prepared, prepared[:] = list(prepared), []
    everything = cli.main(["setup", "--all"])

    # Then --sections adds all-in-one to the default models, --all adds the bench-only ones
    assert sections == everything == 0
    assert sections_prepared == ["chords.cnnlstm", "beats.allinone_tracker"]
    assert prepared == ["chords.cnnlstm", "beats.allinone_tracker", "chords.btc", "separate"]
    assert [line.get("pct") for line in _lines(capfd)] == [50, 100, None, 25, 50, 75, 100, None]
