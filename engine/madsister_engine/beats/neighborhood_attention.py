"""Neighborhood attention with relative position bias, in plain PyTorch.

all-in-one's pretrained weights were trained with the legacy NATTEN ops (`natten{1,2}dqkrpb`, `natten{1,2}dav`), which
NATTEN >= 0.20 removed and which pin torch to 2.5.x. Same signatures and results as NATTEN 0.17, no native build.
Tensors are `[batch, heads, *spatial, dim]`; the bias is `[heads, *(2k-1 per spatial axis)]`.
"""

import sys
import types

import torch


def _neighbors(length: int, kernel_size: int, dilation: int) -> tuple[torch.Tensor, torch.Tensor]:
    """Per position: the `kernel_size` neighbour positions, and their offset from it in the dilated sub-sequence.

    A dilation `d` splits the axis into `d` interleaved sub-sequences, each attended on its own. The window is shifted
    (not padded) at the borders so it always holds `kernel_size` neighbours.
    """
    pos = torch.arange(length)
    group, step = pos % dilation, pos // dilation
    group_len = (length - group + dilation - 1) // dilation
    start = torch.minimum(torch.clamp(step - kernel_size // 2, min=0), group_len - kernel_size)
    taps = torch.arange(kernel_size)
    sub = start[:, None] + taps  # neighbour index inside the sub-sequence
    return group[:, None] + sub * dilation, sub - step[:, None]


def natten1dqkrpb(query, key, rpb, kernel_size, dilation):
    idx, rel = _neighbors(query.shape[2], kernel_size, dilation)
    scores = torch.einsum("bhld,bhlkd->bhlk", query, key[:, :, idx])
    return scores + rpb[:, rel + kernel_size - 1][None]


def natten1dav(attn, value, kernel_size, dilation):
    idx, _ = _neighbors(value.shape[2], kernel_size, dilation)
    return torch.einsum("bhlk,bhlkd->bhld", attn, value[:, :, idx])


def _neighbors_2d(height: int, width: int, kernel_size: int, dilation: int):
    idx_h, rel_h = _neighbors(height, kernel_size, dilation)
    idx_w, rel_w = _neighbors(width, kernel_size, dilation)
    return (idx_h[:, None, :, None], idx_w[None, :, None, :]), (rel_h[:, None, :, None], rel_w[None, :, None, :])


def natten2dqkrpb(query, key, rpb, kernel_size, dilation):
    batch, heads, height, width, _ = query.shape
    idx, (rel_h, rel_w) = _neighbors_2d(height, width, kernel_size, dilation)
    scores = torch.einsum("bhxyd,bhxykzd->bhxykz", query, key[:, :, idx[0], idx[1]])
    bias = rpb[:, rel_h + kernel_size - 1, rel_w + kernel_size - 1]
    return (scores + bias[None]).reshape(batch, heads, height, width, kernel_size * kernel_size)


def natten2dav(attn, value, kernel_size, dilation):
    batch, heads, height, width, _ = attn.shape
    idx, _ = _neighbors_2d(height, width, kernel_size, dilation)
    attn = attn.reshape(batch, heads, height, width, kernel_size, kernel_size)
    return torch.einsum("bhxykz,bhxykzd->bhxyd", attn, value[:, :, idx[0], idx[1]])


def install_as_natten() -> None:
    """Make `from natten.functional import natten1dqkrpb, ...` (what allin1 does) resolve to this module."""
    functional = types.ModuleType("natten.functional")
    for name in ("natten1dqkrpb", "natten1dav", "natten2dqkrpb", "natten2dav"):
        setattr(functional, name, globals()[name])
    package = types.ModuleType("natten")
    package.functional = functional
    sys.modules["natten"] = package
    sys.modules["natten.functional"] = functional
