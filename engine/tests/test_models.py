import subprocess

from madsister_engine.models import ensure_repo, models_dir


def _git(cwd, *args):
    return subprocess.run(["git", "-C", str(cwd), *args], check=True, capture_output=True, text=True).stdout.strip()


def test_models_dir_when_env_var_set_uses_it(monkeypatch, tmp_path):
    # Given / When / Then
    monkeypatch.setenv("MADSISTER_MODELS_DIR", str(tmp_path))
    assert models_dir() == tmp_path
    monkeypatch.delenv("MADSISTER_MODELS_DIR")
    assert models_dir().parts[-3:] == (".cache", "madsister", "models")


def test_ensure_repo_clones_at_pinned_sha_once(monkeypatch, tmp_path):
    # Given a local repo with two commits, and an empty models dir
    upstream = tmp_path / "upstream"
    upstream.mkdir()
    _git(upstream, "init", "-q")
    for content in ("v1", "v2"):
        (upstream / "f.txt").write_text(content)
        _git(upstream, "add", ".")
        _git(upstream, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--no-gpg-sign", "-m", content)
    first_sha = _git(upstream, "rev-list", "--max-parents=0", "HEAD")
    monkeypatch.setenv("MADSISTER_MODELS_DIR", str(tmp_path / "models"))

    # When
    path = ensure_repo("demo", str(upstream), first_sha)

    # Then the first commit is checked out under the models dir
    assert path == tmp_path / "models" / "demo"
    assert (path / "f.txt").read_text() == "v1"
    # And a second call reuses the existing clone
    (path / "f.txt").write_text("local")
    assert ensure_repo("demo", str(upstream), first_sha) == path
    assert (path / "f.txt").read_text() == "local"
