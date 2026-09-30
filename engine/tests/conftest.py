import pytest

from fixtures.make_clip import make_clip


@pytest.fixture
def make_wav(tmp_path):
    """Synthetic clip written at the given rate/channel count, in tmp_path."""

    def _make(rate: int = 44100, channels: int = 1):
        return make_clip(tmp_path / f"clip_{rate}_{channels}ch.wav", rate, channels)

    return _make
