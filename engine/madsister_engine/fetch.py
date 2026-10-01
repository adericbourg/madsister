"""F-IN-2: download a URL's audio with yt-dlp, the engine's only network access (D5, NF-1)."""

from pathlib import Path
from urllib.parse import urlparse

from madsister_engine import events, pipeline


def fetch(url: str, out_dir: str | Path) -> Path:
    if urlparse(url).scheme not in ("http", "https"):
        raise ValueError(f"not an http(s) URL: {url}")
    yt_dlp = pipeline._adapter("downloader", {"yt-dlp": ("fetch", "yt_dlp", "yt_dlp")}, "yt-dlp")
    last_pct = None

    def on_progress(status: dict) -> None:
        nonlocal last_pct
        total = status.get("total_bytes") or status.get("total_bytes_estimate")
        if not total:
            return
        pct = min(100, int(100 * status["downloaded_bytes"] / total))
        if pct != last_pct:
            last_pct = pct
            events.progress("download", pct)

    params = {
        "format": "bestaudio/best",  # no post-processing: decode reads any container
        "outtmpl": str(Path(out_dir) / "%(title)s.%(ext)s"),  # the file stem becomes the Song title
        "noplaylist": True,
        "progress_hooks": [on_progress],
        "noprogress": True,
        "quiet": True,  # keeps fd 1 for the JSON Lines (warnings and errors go to stderr)
    }
    with yt_dlp.YoutubeDL(params) as ydl:
        info = ydl.extract_info(url, download=True)
    return Path(info["requested_downloads"][0]["filepath"])
