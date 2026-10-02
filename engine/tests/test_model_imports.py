"""Smoke test of the model dependency groups: import each heavy dep and make one native call.

Catches ABI mismatches between pinned wheels (e.g. torchaudio built for another torch) that our own code would only hit
deep inside demucs/allin1. Needs every model group installed, no weights.
"""

import importlib

import numpy as np
import pytest

pytestmark = pytest.mark.slow


def test_torchaudio_resample_of_tiny_tensor_halves_its_length():
    import torch
    import torchaudio

    assert torchaudio.functional.resample(torch.zeros(1, 160), 16000, 8000).shape == (1, 80)


def test_madmom_dbn_processor_builds_its_cython_hmm():
    from madmom.features.downbeats import DBNDownBeatTrackingProcessor

    assert DBNDownBeatTrackingProcessor(beats_per_bar=[4], fps=100).hmms


def test_librosa_chroma_cqt_of_short_sine_returns_twelve_bins():
    import librosa

    y = np.sin(2 * np.pi * 440 * np.arange(2 * 22050) / 22050).astype(np.float32)
    assert librosa.feature.chroma_cqt(y=y, sr=22050).shape[0] == 12


@pytest.mark.parametrize(
    "module",
    [
        "madsister_engine.add_heuristic",
        "madsister_engine.beats.allinone_tracker",
        "madsister_engine.beats.madmom_tracker",
        "madsister_engine.chords.btc",
        "madsister_engine.chords.cnnlstm",
        "madsister_engine.separate",
    ],
)
def test_import_of_model_module_succeeds(module):
    importlib.import_module(module)
