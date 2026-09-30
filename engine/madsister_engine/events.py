"""JSON Lines events on stdout: the engine's only output channel (spec §3.2)."""

import json
import sys


def _emit(event: dict) -> None:
    sys.stdout.write(json.dumps(event) + "\n")
    sys.stdout.flush()


def progress(stage: str, pct: int) -> None:
    _emit({"type": "progress", "stage": stage, "pct": pct})


def result(path: str) -> None:
    _emit({"type": "result", "path": path})


def error(message: str) -> None:
    _emit({"type": "error", "message": message})
