"""Chord-CNN-LSTM large-vocabulary chord recognizer (spec §2.1). Needs the `chords-cnnlstm` dependency group.

Code and weights come from a pinned archive of the upstream repo under `models_dir()`; the inference path mirrors its
`chord_recognition.py` (5-model ensemble + HMM decoding over the `submission` chord dictionary, which has inversions).
"""

import contextlib
import sys
from pathlib import Path

from madsister_engine.chords.labels import merge_frames, to_harte
from madsister_engine.models import ensure_repo
from madsister_engine.quantize import ChordSegment

_REPO = (
    "chord-cnn-lstm",
    "https://github.com/music-x-lab/ISMIR2019-Large-Vocabulary-Chord-Recognition",
    "481f4ce703f8822b99f4037e9104ba1760e21ea3",
    "d82917aac315be3b3b23d54b0a27a196d009b6c9d7a65f3d50c5939834c2c09e",
)
_MODELS = [f"joint_chord_net_ismir_naive_v1.0_reweight(0.0,10.0)_s{i}.best" for i in range(5)]


def prepare() -> None:
    ensure_repo(*_REPO)


def recognize(wav_path: str | Path) -> list[ChordSegment]:
    import numpy as np

    repo = ensure_repo(*_REPO)
    wav_path = Path(wav_path).resolve()
    # ponytail: upstream's top-level `mir`, `extractors`, `settings`… go on sys.path (no clash with BTC's modules);
    # breaks if another package with one of those names is imported first.
    if str(repo) not in sys.path:
        sys.path.insert(0, str(repo))
    np.int = int  # removed in NumPy 1.24, still used by upstream's HMM decoder; process-wide alias
    # Upstream resolves `cache_data/` (weights) and `data/` (chord lists) against the working directory.
    with contextlib.chdir(repo):
        from chordnet_ismir_naive import ChordNet
        from extractors.cqt import CQTV2
        from extractors.xhmm_ismir import XHMMDecoder
        from mir import DataEntry, io
        from mir.nn.train import NetworkInterface
        from settings import DEFAULT_HOP_LENGTH, DEFAULT_SR

        entry = DataEntry()
        entry.prop.set("sr", DEFAULT_SR)
        entry.prop.set("hop_length", DEFAULT_HOP_LENGTH)
        entry.append_file(str(wav_path), io.MusicIO, "music")
        entry.append_extractor(CQTV2, "cqt")
        ensemble = [NetworkInterface(ChordNet(None), name).inference(entry.cqt) for name in _MODELS]
        probs = [np.mean(heads, axis=0) for heads in zip(*ensemble)]
        hmm = XHMMDecoder(template_file="data/submission_chord_list.txt")

    names, logprob = hmm.get_chord_tag_obs(probs)
    labels = hmm.decode(probs, np.ones(len(logprob), dtype=np.int8))  # no beat constraint, as upstream by default
    # Confidence = the decoded chord's share of the frame's probability mass over the dictionary.
    posterior = np.exp(logprob - logprob.max(axis=1, keepdims=True))
    posterior /= posterior.sum(axis=1, keepdims=True)
    confidences = posterior[range(len(labels)), [names.index(label) for label in labels]].tolist()
    return merge_frames([to_harte(label) for label in labels], confidences, DEFAULT_HOP_LENGTH / DEFAULT_SR)
