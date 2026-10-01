"""F-IN-3: record the default input to a WAV until SIGINT or a `stop` line on stdin (D3: in the engine, not the webview)."""

import queue
import signal
import sys
import threading
import wave
from pathlib import Path

from madsister_engine import events, pipeline

RATE = 44100


def record(out: str | Path) -> Path:
    sd = pipeline._adapter("recorder", {"sounddevice": ("record", "sounddevice", "sounddevice")}, "sounddevice")
    stop = threading.Event()
    blocks = queue.Queue()

    def watch_stdin() -> None:
        for line in sys.stdin:
            if line.strip() == "stop":
                break
        stop.set()  # also on EOF: the parent is gone

    previous = signal.signal(signal.SIGINT, lambda *_: stop.set())
    try:
        # Opens the device, so a missing input or a denied permission fails before any file is written.
        stream = sd.InputStream(
            samplerate=RATE, channels=1, dtype="int16", callback=lambda indata, *_: blocks.put(indata.tobytes())
        )
        threading.Thread(target=watch_stdin, daemon=True).start()
        with wave.open(str(out), "wb") as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(RATE)
            seconds = 0

            def drain() -> None:
                nonlocal seconds
                while not blocks.empty():
                    wav.writeframes(blocks.get())
                    while wav.tell() // RATE > seconds:
                        seconds += 1
                        events.progress("record", 0, elapsedSec=seconds)

            with stream:
                events.progress("record", 0, elapsedSec=0)
                while not stop.wait(0.2):
                    drain()
            drain()
    finally:
        signal.signal(signal.SIGINT, previous)
    return Path(out)
