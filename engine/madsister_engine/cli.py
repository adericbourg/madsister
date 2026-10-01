import argparse
import sys
import traceback
from pathlib import Path

from madsister_engine import events, fetch, pipeline, record
from madsister_engine.models import models_dir


def _transcribe(args: argparse.Namespace) -> int:
    if not Path(args.audio).is_file():
        raise FileNotFoundError(f"audio file not found: {args.audio}")
    pipeline.transcribe(
        args.audio, args.out, args.meter, not args.no_sections, args.beats, args.chords, args.separate, args.add_tau
    )
    events.result(str(Path(args.out).resolve()))
    return 0


def _fetch(args: argparse.Namespace) -> int:
    events.result(str(fetch.fetch(args.url, args.out_dir).resolve()))
    return 0


def _setup(args: argparse.Namespace) -> int:
    pipeline.setup(args.sections, args.all)
    events.result(str(models_dir()))
    return 0


def _record(args: argparse.Namespace) -> int:
    events.result(str(record.record(args.out).resolve()))
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="madsister-engine")
    sub = parser.add_subparsers(dest="command", required=True)

    transcribe = sub.add_parser("transcribe", help="audio file -> Song JSON")
    transcribe.add_argument("audio")
    transcribe.add_argument("--out", required=True)
    transcribe.add_argument("--meter", choices=pipeline.METERS)
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
    fetch.set_defaults(func=_fetch)

    setup = sub.add_parser("setup", help="download the model weights once, for offline transcription")
    setup.add_argument("--sections", action="store_true", help="also all-in-one (section detection)")
    setup.add_argument("--all", action="store_true", help="every model, bench-only ones included")
    setup.set_defaults(func=_setup)

    record = sub.add_parser("record", help="record from the microphone")
    record.add_argument("--out", required=True)
    record.set_defaults(func=_record)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        return args.func(args)
    except Exception as e:  # the §3.2 contract: every failure becomes an error line + non-zero exit
        traceback.print_exc(file=sys.stderr)
        events.error(str(e))
        return 1
