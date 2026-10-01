import json
import sys
import types

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


def test_main_of_stub_command_emits_error_and_fails(capfd):
    # When
    code = cli.main(["record", "--out", "/tmp/r.wav"])

    # Then
    assert code != 0
    assert _lines(capfd) == [{"type": "error", "message": "not implemented"}]


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
