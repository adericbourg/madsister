"""Local cache of model weights and third-party model repos (never in the repo)."""

import os
import shutil
import tarfile
import urllib.request
from pathlib import Path


def models_dir() -> Path:
    return Path(os.environ.get("MADSISTER_MODELS_DIR", Path.home() / ".cache/madsister/models"))


def ensure_repo(name: str, url: str, sha: str) -> Path:
    """Download GitHub repo `url` at `sha` (its archive: no git needed) into `models_dir()/name` unless it is already there."""
    path = models_dir() / name
    if not path.exists():
        # Extract next to the target and rename, so an interrupted download is never mistaken for a complete one.
        tmp = path.with_name(name + ".partial")
        shutil.rmtree(tmp, ignore_errors=True)
        archive = urllib.request.urlopen(f"{url}/archive/{sha}.tar.gz")
        with archive, tarfile.open(fileobj=archive, mode="r|gz") as tar:
            tar.extractall(tmp, filter="data")
        (top,) = tmp.iterdir()  # GitHub archives hold one `<repo>-<sha>/` directory
        top.rename(path)
        tmp.rmdir()
    return path
