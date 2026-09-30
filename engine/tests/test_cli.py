import json

from madsister_engine import cli


def _lines(capsys):
    return [json.loads(line) for line in capsys.readouterr().out.splitlines()]


def test_main_of_stub_command_emits_error_and_fails(capsys):
    # When
    code = cli.main(["fetch", "https://example.com", "--out-dir", "/tmp"])

    # Then
    assert code != 0
    assert _lines(capsys) == [{"type": "error", "message": "not implemented"}]


def test_main_when_command_raises_emits_error_and_exits_1(capsys, tmp_path):
    # Given a missing audio file
    missing = tmp_path / "missing.mp3"

    # When
    code = cli.main(["transcribe", str(missing), "--out", str(tmp_path / "s.json")])

    # Then
    assert code == 1
    lines = _lines(capsys)
    assert len(lines) == 1
    assert lines[0]["type"] == "error"
    assert "missing.mp3" in lines[0]["message"]
