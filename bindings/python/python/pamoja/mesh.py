"""Idiomatic mesh-framing facade.

When the fixed infrastructure is gone or was never there, devices carry each
other's traffic: every node relays what it hears, so a message crosses an area no
single node can reach. This is the packet half of that, addressing and integrity
over radios that give you neither.
"""

from __future__ import annotations

from ._core import MeshFrame, SeenPackets
from ._core import mesh_broadcast_frame as _broadcast_frame
from ._core import mesh_crc16 as _crc16
from ._core import mesh_frame as _frame
from ._core import mesh_limits as _limits
from ._core import mesh_parse_frame as _parse_frame
from ._core import mesh_relayed as _relayed

__all__ = [
    "BROADCAST",
    "DEFAULT_HOP_LIMIT",
    "MAX_FRAME",
    "MAX_PAYLOAD",
    "SEEN_DEFAULT_CAPACITY",
    "MeshFrame",
    "SeenPackets",
    "broadcast",
    "crc16",
    "frame",
    "parse",
    "relayed",
]

_MAX_FRAME, _MAX_PAYLOAD, _BROADCAST, _HOP_LIMIT, _SEEN = _limits()

#: The largest frame, in bytes, including its header and checksum.
MAX_FRAME = _MAX_FRAME
#: The largest payload a single frame can carry, in bytes.
MAX_PAYLOAD = _MAX_PAYLOAD
#: The destination address that means every node.
BROADCAST = _BROADCAST
#: The hop limit a frame starts with unless one is given.
DEFAULT_HOP_LIMIT = _HOP_LIMIT
#: A duplicate-cache size for a caller with no reason to choose one.
SEEN_DEFAULT_CAPACITY = _SEEN


def frame(
    src: int,
    dst: int,
    id: int,
    payload: bytes,
    hop_limit: int | None = None,
) -> MeshFrame:
    """Build a frame addressed to one node.

    :param src: The address of this node.
    :param dst: The address the frame is for, or :data:`BROADCAST`.
    :param id: The sequence number identifying this packet from this source.
    :param payload: The bytes to carry.
    :param hop_limit: How many relays the frame may take, defaulting to
        :data:`DEFAULT_HOP_LIMIT`.
    :returns: The frame, with the bytes to transmit on its ``bytes`` attribute.
    :raises PamojaError: If the payload is larger than :data:`MAX_PAYLOAD`.
    """
    return _frame(src, dst, id, bytes(payload), hop_limit)


def broadcast(
    src: int, id: int, payload: bytes, hop_limit: int | None = None
) -> MeshFrame:
    """Build a frame addressed to every node.

    :param src: The address of this node.
    :param id: The sequence number identifying this packet from this source.
    :param payload: The bytes to carry.
    :param hop_limit: How many relays the frame may take, defaulting to
        :data:`DEFAULT_HOP_LIMIT`.
    :returns: The frame, with the bytes to transmit on its ``bytes`` attribute.
    :raises PamojaError: If the payload is larger than :data:`MAX_PAYLOAD`.
    """
    return _broadcast_frame(src, id, bytes(payload), hop_limit)


def parse(data: bytes) -> MeshFrame:
    """Parse a frame received off a radio.

    :param data: The frame exactly as it arrived.
    :returns: The parsed frame.
    :raises PamojaError: If the frame is truncated, of an unknown version, or
        fails its checksum, which is what a noisy radio produces.
    """
    return _parse_frame(bytes(data))


def relayed(data: bytes) -> MeshFrame | None:
    """Return the same frame with one hop spent, ready to forward.

    :param data: The frame exactly as it arrived.
    :returns: The frame to forward, or ``None`` once its hops have run out,
        which is what stops a flood from circulating forever.
    :raises PamojaError: If the frame cannot be parsed.
    """
    return _relayed(bytes(data))


def crc16(data: bytes) -> int:
    """Compute the CRC-16 a frame carries.

    :param data: The bytes the checksum covers.
    :returns: The checksum.
    """
    return _crc16(bytes(data))
