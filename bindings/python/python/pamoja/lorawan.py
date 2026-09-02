"""Idiomatic LoRaWAN facade.

A long-range public-band link is wide open, so LoRaWAN wraps every frame in two
guarantees: a message integrity code keyed to the network proves the frame is
authentic and intact, and the payload is encrypted to the application so only its
owner can read it. This builds and verifies exactly that.
"""

from __future__ import annotations

import enum

from ._core import (
    LorawanDevice,
    LorawanJoinAccept,
    LorawanRxData,
    LorawanSession,
)

__all__ = [
    "Device",
    "Direction",
    "JoinAccept",
    "RxData",
    "Session",
    "device",
    "session",
]

#: An activated session: a device address and its two session keys.
Session = LorawanSession
#: The root credentials over-the-air activation is built on.
Device = LorawanDevice
#: An accepted join: the network settings, and the session it grants.
JoinAccept = LorawanJoinAccept
#: A decoded data frame, with its payload decrypted.
RxData = LorawanRxData


class Direction(str, enum.Enum):
    """The direction a frame travelled, which its MIC and encryption fold in."""

    #: From an end device up to the network.
    UPLINK = "Uplink"
    #: From the network down to an end device.
    DOWNLINK = "Downlink"


def session(dev_addr: int, nwk_skey: bytes, app_skey: bytes) -> LorawanSession:
    """Create a session for a device already activated by personalization.

    :param dev_addr: The device address the network assigned.
    :param nwk_skey: The 16-byte network session key, which authenticates frames.
    :param app_skey: The 16-byte application session key, which encrypts payloads.
    :returns: The session, ready to encode and decode data frames.
    :raises PamojaError: If either key is not 16 bytes.
    """
    return LorawanSession(dev_addr, bytes(nwk_skey), bytes(app_skey))


def device(dev_eui: bytes, app_eui: bytes, app_key: bytes) -> LorawanDevice:
    """Create a device holding the root credentials for over-the-air activation.

    :param dev_eui: The 8-byte device EUI.
    :param app_eui: The 8-byte application (join) EUI.
    :param app_key: The 16-byte application key the join exchange is secured with.
    :returns: The device, ready to build a join request.
    :raises PamojaError: If any credential is the wrong length.
    """
    return LorawanDevice(bytes(dev_eui), bytes(app_eui), bytes(app_key))
