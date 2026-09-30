import json

from madsister_engine import events


def test_progress_writes_one_json_line(capsys):
    # When
    events.progress("beats", 40)

    # Then
    out = capsys.readouterr().out
    assert out.count("\n") == 1
    assert json.loads(out) == {"type": "progress", "stage": "beats", "pct": 40}


def test_result_and_error_write_contract_shapes(capsys):
    # When
    events.result("/tmp/song.json")
    events.error("boom")

    # Then
    lines = [json.loads(line) for line in capsys.readouterr().out.splitlines()]
    assert lines == [
        {"type": "result", "path": "/tmp/song.json"},
        {"type": "error", "message": "boom"},
    ]
