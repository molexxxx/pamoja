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
    LorawanGrant,
    LorawanHeader,
    LorawanJoinAccept,
    LorawanJoinRequest,
    LorawanRxData,
    LorawanSession,
)
from ._core import lorawan_parse_header as _parse_header
from ._core import lorawan_parse_join_request as _parse_join_request

__all__ = [
    "Device",
    "Direction",
    "Grant",
    "Header",
    "JoinAccept",
    "JoinRequest",
    "MessageType",
    "RxData",
    "Session",
    "device",
    "grant",
    "parse_header",
    "parse_join_request",
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
#: What a frame says about itself before any key is involved.
Header = LorawanHeader
#: A join-request a device broadcast, with its integrity already verified.
JoinRequest = LorawanJoinRequest
#: What a network grants a device that joined.
Grant = LorawanGrant


class MessageType(str, enum.Enum):
    """What kind of message a frame is, read from its header."""

    #: A device asking to join a network.
    JOIN_REQUEST = "JoinRequest"
    #: A network admitting a device.
    JOIN_ACCEPT = "JoinAccept"
    #: Data from a device that does not need acknowledging.
    UNCONFIRMED_UP = "UnconfirmedUp"
    #: Data from a device that asks to be acknowledged.
    CONFIRMED_UP = "ConfirmedUp"
    #: Data to a device that does not need acknowledging.
    UNCONFIRMED_DOWN = "UnconfirmedDown"
    #: Data to a device that asks to be acknowledged.
    CONFIRMED_DOWN = "ConfirmedDown"


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


def parse_header(data: bytes) -> Header:
    """Read a frame far enough to route it, without any key.

    A receiver holding many sessions uses this to find which one a frame belongs
    to: the device address travels in the clear, so it can be read before the
    session that would verify the frame is even known.

    Nothing this reports is authenticated. Treat it as a routing hint until
    :meth:`Session.decode` has verified the frame.

    :param data: The raw frame as it came off the radio.
    :returns: What the header says the frame is.
    :raises PamojaError: If the frame is truncated or carries a message type this
        build does not read.
    """
    return _parse_header(bytes(data))


def parse_join_request(data: bytes, app_key: bytes) -> JoinRequest:
    """Verify a join-request and read the identifiers out of it.

    This is the network side of activation: it proves the request came from a
    holder of the application key before reporting who sent it.

    :param data: The raw join-request as it came off the radio.
    :param app_key: The 16-byte application root key the device shares.
    :returns: The verified request.
    :raises PamojaError: If the MIC does not verify or the frame is not a
        join-request.
    """
    return _parse_join_request(bytes(data), bytes(app_key))


def grant(
    app_nonce: int,
    net_id: int,
    dev_addr: int,
    dl_settings: int = 0,
    rx_delay: int = 0,
    cflist: bytes | None = None,
) -> Grant:
    """Describe what this network grants a device that joined.

    :param app_nonce: A nonce this network must not reuse for the device, since
        the session keys are derived from it; low 24 bits only.
    :param net_id: The network identifier; low 24 bits only.
    :param dev_addr: The address to assign the device.
    :param dl_settings: The downlink settings byte.
    :param rx_delay: The delay before the first receive window, in seconds.
    :param cflist: The optional 16-byte channel list.
    :returns: The grant, which signs its own join-accept and derives the session.
    :raises PamojaError: If the channel list is not 16 bytes.
    """
    return Grant(
        app_nonce,
        net_id,
        dev_addr,
        dl_settings,
        rx_delay,
        None if cflist is None else bytes(cflist),
    )
