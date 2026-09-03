"""Idiomatic MAVLink wire-protocol facade.

MAVLink is the language drones speak: PX4 and ArduPilot autopilots and MAVSDK
ground stations all exchange MAVLink frames, so talking to a vehicle means
putting exactly the right bytes on the wire and trusting the bytes that come
back. This is that byte layer: v1 and v2 frames, the CRC-16/MCRF4XX checksum
every frame carries, the per-message ``CRC_EXTRA`` seed that catches a frame
whose shape does not match, and MAVLink 2 signing.

Nothing here is limited to the messages this build happens to know. The common
dialect's seeds are built in, and :class:`Dialect` carries any others, derived
from a message definition the way the specification does.
"""

from __future__ import annotations

import time

from ._core import (
    Dialect,
    MavlinkFrame,
    MavlinkHeader,
    MavlinkParser,
    MavlinkSigner,
    MavlinkVerifier,
    mavlink_crc16_mcrf4xx,
    mavlink_known_crc_extra,
    mavlink_message_crc_extra,
    mavlink_timestamp_from_unix_micros,
)

__all__ = [
    "DEFAULT_TIMESTAMP_WINDOW",
    "Dialect",
    "KEY_LEN",
    "MAX_FRAME",
    "MAX_PAYLOAD",
    "MavlinkFrame",
    "MavlinkHeader",
    "MavlinkParser",
    "MavlinkSigner",
    "MavlinkVerifier",
    "SIGNATURE_LEN",
    "crc16",
    "frame",
    "known_crc_extra",
    "message_crc_extra",
    "timestamp_from_unix_micros",
    "timestamp_now",
]

#: The largest payload a frame can carry, in bytes.
MAX_PAYLOAD = 255

#: The largest frame, in bytes, header, checksum and signature included.
MAX_FRAME = 280

#: The length of a v2 signature block, in bytes.
SIGNATURE_LEN = 13

#: The length of a signing key, in bytes.
KEY_LEN = 32

#: The default window a verifier accepts a timestamp within.
DEFAULT_TIMESTAMP_WINDOW = 6_000_000


def crc16(data: bytes) -> int:
    """Return the CRC-16/MCRF4XX checksum of a byte string.

    This is the checksum every MAVLink frame carries, exposed because a host
    that implements part of the protocol itself needs the same arithmetic.

    :param data: The data to checksum.
    :returns: The checksum.
    """
    return mavlink_crc16_mcrf4xx(data)


def message_crc_extra(name: str, fields: list[tuple[str, str, int]]) -> int:
    """Derive the ``CRC_EXTRA`` seed of a message from its definition.

    This is what makes a dialect this build has never seen usable: given a
    message's name and its base fields in wire order, the seed comes out the
    same as the one the dialect publishes, and a frame carrying that message
    then checks like any other.

    Extension fields are excluded from the seed and must not be listed, which is
    what lets a peer that predates them still check the frame.

    :param name: The message name, such as ``HEARTBEAT``.
    :param fields: The base fields in wire order, as ``(type, name, array_len)``
        triples; ``array_len`` is ``0`` for a scalar.
    :returns: The seed.

    >>> message_crc_extra("PRIVATE_STATUS", [("uint32_t", "uptime", 0)]) >= 0
    True
    """
    return mavlink_message_crc_extra(name, fields)


def known_crc_extra(msgid: int) -> int | None:
    """Return the ``CRC_EXTRA`` the common dialect publishes for a message id.

    :param msgid: The message id to look up.
    :returns: The seed, or ``None`` for an id outside the common dialect, which
        is what a :class:`Dialect` is for.

    >>> known_crc_extra(0)
    50
    >>> known_crc_extra(9999) is None
    True
    """
    return mavlink_known_crc_extra(msgid)


def timestamp_from_unix_micros(unix_micros: int) -> int:
    """Convert Unix time into the timestamp MAVLink signing counts in.

    :param unix_micros: The time in microseconds since the Unix epoch.
    :returns: The signing timestamp, in units of ten microseconds since 2015.
    """
    return mavlink_timestamp_from_unix_micros(unix_micros)


def timestamp_now() -> int:
    """Return a signing timestamp for now.

    :returns: The signing timestamp matching the current clock.
    """
    return mavlink_timestamp_from_unix_micros(int(time.time() * 1_000_000))


def frame(header: MavlinkHeader, msgid: int, payload: bytes) -> MavlinkFrame:
    """Build a v2 frame carrying a message the common dialect defines.

    The seed is looked up rather than passed, which is the usual case: a sender
    emitting a standard message should not have to know its checksum constant.

    :param header: The addressing fields to stamp on the frame.
    :param msgid: The message id.
    :param payload: The message payload.
    :returns: The frame ready to send.
    :raises ValueError: If the id is outside the common dialect, in which case
        build the frame with :meth:`MavlinkFrame.raw` and a seed of your own.

    >>> heartbeat = bytes([0, 0, 0, 0, 18, 0, 0, 4, 3])
    >>> sent = frame(MavlinkHeader(1, 1), 0, heartbeat)
    >>> sent.message_id
    0
    >>> MavlinkFrame.parse_known(sent.bytes).payload == heartbeat
    True
    """
    crc_extra = mavlink_known_crc_extra(msgid)
    if crc_extra is None:
        raise ValueError(
            f"message {msgid} is not in the common dialect; "
            "supply its CRC_EXTRA with MavlinkFrame.raw"
        )
    return MavlinkFrame.encode_v2(header, msgid, payload, crc_extra)
