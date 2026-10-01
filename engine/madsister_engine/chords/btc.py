"""BTC large-vocabulary chord recognizer (spec §2.1). Needs the `chords-btc` dependency group.

Code and weights come from a pinned archive of the upstream repo under `models_dir()`; the inference path mirrors its `test.py`.
"""

import sys
from pathlib import Path

from madsister_engine.chords.labels import merge_frames, to_harte
from madsister_engine.models import ensure_repo
from madsister_engine.quantize import ChordSegment

_REPO = ("btc-ismir19", "https://github.com/jayg996/BTC-ISMIR19", "2682317be668032e6e4b269ded36adaa2ad57df0")
_NUM_CHORDS = 170  # large vocabulary: 12 roots x 14 qualities + X + N


def recognize(wav_path: str | Path) -> list[ChordSegment]:
    import numpy as np
    import torch
    import yaml

    repo = ensure_repo(*_REPO)
    # ponytail: upstream's top-level `utils` package goes on sys.path; breaks if another `utils` module is already imported.
    if str(repo) not in sys.path:
        sys.path.insert(0, str(repo))
    np.float = float  # removed in NumPy 1.24, still used by upstream's transformer_modules; process-wide alias
    from btc_model import BTC_model
    from utils.hparams import HParams
    from utils.mir_eval_modules import audio_file_to_features, idx2voca_chord

    config = HParams(**yaml.safe_load((repo / "run_config.yaml").read_text()))  # HParams.load predates PyYAML 6
    config.feature["large_voca"] = True
    config.model["num_chords"] = _NUM_CHORDS
    config.model["probs_out"] = True  # the output layer then returns logits instead of argmax indices
    checkpoint = torch.load(repo / "test/btc_model_large_voca.pt", map_location="cpu", weights_only=False)
    model = BTC_model(config=config.model)
    model.load_state_dict(checkpoint["model"])
    model.eval()

    feature, frame_duration, _ = audio_file_to_features(str(wav_path), config)
    feature = (feature.T - checkpoint["mean"]) / checkpoint["std"]
    n_frames, timestep = len(feature), config.model["timestep"]
    feature = np.pad(feature, ((0, -n_frames % timestep), (0, 0)))
    with torch.no_grad():
        windows = torch.tensor(feature, dtype=torch.float32).view(-1, timestep, feature.shape[1])
        hidden, _ = model.self_attn_layers(windows)
        probs = torch.softmax(model.output_layer(hidden), -1).reshape(-1, _NUM_CHORDS)[:n_frames]
    confidences, indices = probs.max(-1)

    names = idx2voca_chord()
    return merge_frames([to_harte(names[i]) for i in indices.tolist()], confidences.tolist(), frame_duration)
