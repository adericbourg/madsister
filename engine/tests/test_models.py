import hashlib
import io
import tarfile

import pytest

from madsister_engine.models import ensure_repo, models_dir


def _publish_archive(upstream, sha, files):
    """Write `files` as GitHub serves `<upstream>/archive/<sha>.tar.gz`: one top-level `<repo>-<sha>/` directory."""
    (upstream / "archive").mkdir(parents=True)
    with tarfile.open(upstream / "archive" / f"{sha}.tar.gz", "w:gz") as tar:
        for name, content in files.items():
            info = tarfile.TarInfo(f"repo-{sha}/{name}")
            info.size = len(content)
            tar.addfile(info, io.BytesIO(content))
    return hashlib.sha256((upstream / "archive" / f"{sha}.tar.gz").read_bytes()).hexdigest()


def test_models_dir_when_env_var_set_uses_it(monkeypatch, tmp_path):
    # Given / When / Then
    monkeypatch.setenv("MADSISTER_MODELS_DIR", str(tmp_path))
    assert models_dir() == tmp_path
    monkeypatch.delenv("MADSISTER_MODELS_DIR")
    assert models_dir().parts[-3:] == (".cache", "madsister", "models")


def test_ensure_repo_downloads_pinned_sha_archive_once(monkeypatch, tmp_path):
    # Given the archive of a pinned sha, and an empty models dir
    upstream = tmp_path / "upstream"
    sha256 = _publish_archive(upstream, "abc123", {"f.txt": b"v1", "sub/g.txt": b"g"})
    monkeypatch.setenv("MADSISTER_MODELS_DIR", str(tmp_path / "models"))

    # When
    path = ensure_repo("demo", upstream.as_uri(), "abc123", sha256)

    # Then the archive's content is extracted under the models dir, without its top-level directory
    assert path == tmp_path / "models" / "demo"
    assert (path / "f.txt").read_text() == "v1"
    assert (path / "sub/g.txt").read_text() == "g"
    assert sorted(p.name for p in (tmp_path / "models").iterdir()) == ["demo"]
    # And a second call reuses the extracted repo
    (path / "f.txt").write_text("local")
    assert ensure_repo("demo", upstream.as_uri(), "abc123", sha256) == path
    assert (path / "f.txt").read_text() == "local"


def test_ensure_repo_when_archive_hash_differs_raises_and_extracts_nothing(monkeypatch, tmp_path):
    # Given an archive that isn't the pinned one
    upstream = tmp_path / "upstream"
    _publish_archive(upstream, "abc123", {"f.txt": b"tampered"})
    monkeypatch.setenv("MADSISTER_MODELS_DIR", str(tmp_path / "models"))

    # When / Then
    with pytest.raises(RuntimeError, match="sha256 mismatch"):
        ensure_repo("demo", upstream.as_uri(), "abc123", "0" * 64)
    assert not (tmp_path / "models" / "demo").exists()
