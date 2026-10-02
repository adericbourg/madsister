"""The pure-torch neighborhood attention against outputs recorded from NATTEN 0.17.4 (torch 2.5.1), the version
all-in-one's weights were validated with. Needs torch (the `beats-allinone` group), no weights."""

from pathlib import Path

import numpy as np
import pytest

pytestmark = pytest.mark.slow

GOLDEN = np.load(Path(__file__).parent / "data" / "natten_golden.npz")
CASES = sorted({key.split("/")[0] for key in GOLDEN.files})


@pytest.mark.parametrize("case", CASES)
def test_matches_natten_0_17(case):
    import torch

    from madsister_engine.beats import neighborhood_attention as na

    g = {name: torch.from_numpy(GOLDEN[f"{case}/{name}"]) for name in ("q", "key", "v", "rpb", "scores", "attn", "out", "cfg")}
    k, d = g["cfg"].tolist()
    qkrpb, av = (na.natten1dqkrpb, na.natten1dav) if case.startswith("1d") else (na.natten2dqkrpb, na.natten2dav)
    torch.testing.assert_close(qkrpb(g["q"], g["key"], g["rpb"], k, d), g["scores"])
    torch.testing.assert_close(av(g["attn"], g["v"], k, d), g["out"])


def test_install_as_natten_serves_the_functions_allin1_imports():
    from madsister_engine.beats import neighborhood_attention as na

    na.install_as_natten()
    from natten.functional import natten1dav, natten1dqkrpb, natten2dav, natten2dqkrpb  # noqa: F401
