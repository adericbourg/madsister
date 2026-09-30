import argparse
import sys
import traceback
from pathlib import Path

from madsister_engine import events


def _transcribe(args: argparse.Namespace) -> int:
    if not Path(args.audio).is_file():
        raise FileNotFoundError(f"audio file not found: {args.audio}")
    raise NotImplementedError("transcribe is not implemented yet")


def _not_implemented(args: argparse.Namespace) -> int:
    events.error("not implemented")
    return 2


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="madsister-engine")
    sub = parser.add_subparsers(dest="command", required=True)

    transcribe = sub.add_parser("transcribe", help="audio file -> Song JSON")
    transcribe.add_argument("audio")
    transcribe.add_argument("--out", required=True)
    transcribe.add_argument("--meter", type=int, choices=[3, 4])
    transcribe.add_argument("--no-sections", action="store_true")
    transcribe.set_defaults(func=_transcribe)

    fetch = sub.add_parser("fetch", help="download audio from a URL")
    fetch.add_argument("url")
    fetch.add_argument("--out-dir", required=True)
    fetch.set_defaults(func=_not_implemented)

    record = sub.add_parser("record", help="record from the microphone")
    record.add_argument("--out", required=True)
    record.set_defaults(func=_not_implemented)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        return args.func(args)
    except Exception as e:  # the §3.2 contract: every failure becomes an error line + non-zero exit
        traceback.print_exc(file=sys.stderr)
        events.error(str(e))
        return 1
