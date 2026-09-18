"""Remote multicast setup, TS005-2.0.0, on port :data:`PORT`.

A group of devices is given one address and one key, so a firmware image goes out once rather
than once per device. The group key travels wrapped under a key each device derives from its
own root key and never transmits.

>>> from pamoja.lorawan import multicast
>>> root, group = bytes([0x2B]) * 16, bytes([0x77]) * 16
>>> ke = multicast.ke_key(multicast.root_key(root))
>>> multicast.key(ke, multicast.wrap_key(ke, group)) == group
True
"""

from __future__ import annotations

from pamoja._native import LORAWAN_MULTICAST_PORT
from pamoja._native import lorawan_mc_app_s_key as app_s_key
from pamoja._native import lorawan_mc_ke_key as ke_key
from pamoja._native import lorawan_mc_key as key
from pamoja._native import lorawan_mc_nwk_s_key as nwk_s_key
from pamoja._native import lorawan_mc_root_key as root_key
from pamoja._native import lorawan_wrap_mc_key as wrap_key

__all__ = [
    "PORT",
    "app_s_key",
    "ke_key",
    "key",
    "nwk_s_key",
    "root_key",
    "wrap_key",
]

#: The port this package is spoken on.
PORT = LORAWAN_MULTICAST_PORT
