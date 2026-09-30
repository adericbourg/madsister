import argparse
import sys
import traceback
from pathlib import Path

from madsister_engine import events, pipeline


def _transcribe(args: argparse.Namespace) -> int:
    if not Path(args.audio).is_file():
        raise FileNotFoundError(f"audio file not found: {args.audio}")
    pipeline.transcribe(
        args.audio, args.out, args.meter, not args.no_sections, args.beats, args.chords, args.separate, args.add_tau
    )
    events.result(str(Path(args.out).resolve()))
    return 0


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
    # Bench-only options (M0-13), hidden until M4b replaces them with --mode.
    transcribe.add_argument("--beats", choices=["auto", "allinone", "madmom"], default="auto", help=argparse.SUPPRESS)
    transcribe.add_argument("--chords", choices=["auto", "btc", "cnnlstm"], default="auto", help=argparse.SUPPRESS)
    transcribe.add_argument("--separate", action="store_true", help=argparse.SUPPRESS)
    transcribe.add_argument("--add-tau", type=float, help=argparse.SUPPRESS)
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
