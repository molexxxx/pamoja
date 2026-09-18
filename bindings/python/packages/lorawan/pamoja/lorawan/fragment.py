"""Fragmented data block transport, TS004-2.0.0, on port :data:`PORT`.

A block too large for one frame goes across in pieces, followed by coded fragments that let a
device solve for the ones it missed rather than asking for them again. A device sets aside its
working memory once, sized by the losses it is willing to solve for rather than by the block.

>>> from pamoja.lorawan import fragment
>>> block = bytes(range(64))
>>> session = fragment.session(len(block), 16)
>>> receiver = fragment.Defragmenter(session.nb_frag, 16, 2)
>>> n = 0
>>> while not receiver.done:
...     n += 1
...     _ = receiver.fragment(n, fragment.fragment(block, 16, n))
>>> receiver.block[: len(block)] == block
True
"""

from __future__ import annotations

from pamoja._native import (
    LORAWAN_FRAGMENT_PORT,
    LORAWAN_MAX_FRAGMENTS,
    LorawanBlockMic,
    LorawanDefragmenter,
    LorawanFragSession,
)
from pamoja._native import lorawan_data_block_int_key as data_block_int_key
from pamoja._native import lorawan_frag_fragment as fragment
from pamoja._native import lorawan_frag_parity_line as parity_line
from pamoja._native import lorawan_frag_prbs23 as prbs23
from pamoja._native import lorawan_frag_session as session

__all__ = [
    "MAX_FRAGMENTS",
    "PORT",
    "BlockMic",
    "Defragmenter",
    "Session",
    "data_block_int_key",
    "fragment",
    "parity_line",
    "prbs23",
    "session",
]

#: The port this package is spoken on.
PORT = LORAWAN_FRAGMENT_PORT

#: The most fragments one session carries.
MAX_FRAGMENTS = LORAWAN_MAX_FRAGMENTS

#: How a block is cut into fragments.
Session = LorawanFragSession

#: A session being put back together from whatever arrives.
Defragmenter = LorawanDefragmenter

#: The code taken over a block as it arrives.
BlockMic = LorawanBlockMic
