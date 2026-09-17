"""Idiomatic LoRaWAN facade.

A long-range public-band link is wide open, so LoRaWAN wraps every frame in two
guarantees: a message integrity code keyed to the network proves the frame is
authentic and intact, and the payload is encrypted to the application so only its
owner can read it. This builds and verifies exactly that.
"""

from __future__ import annotations

import enum

from pamoja._native import (
    LORAWAN_ADR_ACK_DELAY,
    LORAWAN_ADR_ACK_LIMIT,
    LORAWAN_JOIN_ACCEPT_DELAY1_US,
    LORAWAN_JOIN_ACCEPT_DELAY2_US,
    LORAWAN_MAX_FCNT_GAP,
    LORAWAN_RECEIVE_DELAY1_US,
    LORAWAN_RECEIVE_DELAY2_US,
    LORAWAN_RECEIVE_WINDOW_TOLERANCE_US,
    LORAWAN_RETRANSMIT_TIMEOUT_MAX_US,
    LORAWAN_RETRANSMIT_TIMEOUT_MIN_US,
    LORAWAN_SAVED_LEN,
    LorawanBackoff,
    LorawanBackoffStep,
    LorawanCfList,
    LorawanChannel,
    LorawanDelivery,
    LorawanDevice,
    LorawanDeviceError,
    LorawanDeviceSettings,
    LorawanEndDevice,
    LorawanHeard,
    LorawanNext,
    LorawanTransmission,
    LorawanWindow,
    LorawanGrant,
    LorawanHeader,
    LorawanJoinAccept,
    LorawanJoinRequest,
    LorawanMacCommand,
    LorawanRxData,
    LorawanSession,
)
from pamoja._native import lorawan_mac_parse as _mac_parse
from pamoja._native import lorawan_parse_header as _parse_header
from pamoja._native import lorawan_parse_join_request as _parse_join_request

__all__ = [
    "ADR_ACK_DELAY",
    "ADR_ACK_LIMIT",
    "Backoff",
    "BackoffStep",
    "CfList",
    "Channel",
    "Delivery",
    "Device",
    "DeviceError",
    "DeviceSettings",
    "Direction",
    "EndDevice",
    "Grant",
    "Header",
    "Heard",
    "JOIN_ACCEPT_DELAY1_US",
    "JOIN_ACCEPT_DELAY2_US",
    "JoinAccept",
    "JoinRequest",
    "MAX_FCNT_GAP",
    "MacCommand",
    "MessageType",
    "Next",
    "RECEIVE_DELAY1_US",
    "RECEIVE_DELAY2_US",
    "RECEIVE_WINDOW_TOLERANCE_US",
    "RETRANSMIT_TIMEOUT_MAX_US",
    "RETRANSMIT_TIMEOUT_MIN_US",
    "RxData",
    "SAVED_LEN",
    "Session",
    "Transmission",
    "Version",
    "Window",
    "device",
    "end_device",
    "grant",
    "mac_parse",
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
#: The optional channel list at the end of a join accept.
CfList = LorawanCfList
#: A device's count of how long the network has been silent.
Backoff = LorawanBackoff
#: What a back-off says to do with one uplink.
BackoffStep = LorawanBackoffStep
#: A LoRaWAN Class A end device, without a radio.
EndDevice = LorawanEndDevice
#: What a device's radio can do, and how it takes part.
DeviceSettings = LorawanDeviceSettings
#: Raised when an end device cannot do what it was asked; ``kind`` names why.
DeviceError = LorawanDeviceError
#: A frame to put on the air, and where to listen afterward.
Transmission = LorawanTransmission
#: When and where to listen for a downlink.
Window = LorawanWindow
#: What a frame heard in a receive window turned out to be.
Heard = LorawanHeard
#: A downlink, read and acted on.
Delivery = LorawanDelivery
#: What to do once both receive windows closed with nothing for the device.
Next = LorawanNext
#: A channel a device may send on.
Channel = LorawanChannel
#: How many bytes a saved device state takes.
SAVED_LEN = LORAWAN_SAVED_LEN

#: How long after an uplink the first receive window opens, RP002-1.0.5 section 3.3.
RECEIVE_DELAY1_US = LORAWAN_RECEIVE_DELAY1_US
#: How long after an uplink the second receive window opens.
RECEIVE_DELAY2_US = LORAWAN_RECEIVE_DELAY2_US
#: How long after a join request the first join accept window opens.
JOIN_ACCEPT_DELAY1_US = LORAWAN_JOIN_ACCEPT_DELAY1_US
#: How long after a join request the second join accept window opens.
JOIN_ACCEPT_DELAY2_US = LORAWAN_JOIN_ACCEPT_DELAY2_US
#: How far a receive window may open either side of its time, LoRaWAN 1.0.3 section 3.3.1.
RECEIVE_WINDOW_TOLERANCE_US = LORAWAN_RECEIVE_WINDOW_TOLERANCE_US
#: The largest gap a frame counter may jump across and still be accepted.
MAX_FCNT_GAP = LORAWAN_MAX_FCNT_GAP
#: How many unanswered uplinks before a device asks the network to answer.
ADR_ACK_LIMIT = LORAWAN_ADR_ACK_LIMIT
#: How many more before a device starts giving back what adaptive data rate took.
ADR_ACK_DELAY = LORAWAN_ADR_ACK_DELAY
#: The shortest wait before a confirmed uplink is sent again.
RETRANSMIT_TIMEOUT_MIN_US = LORAWAN_RETRANSMIT_TIMEOUT_MIN_US
#: The longest wait before a confirmed uplink is sent again.
RETRANSMIT_TIMEOUT_MAX_US = LORAWAN_RETRANSMIT_TIMEOUT_MAX_US


class Version(str, enum.Enum):
    """A revision of the LoRaWAN link layer, as :class:`Backoff` takes it.

    >>> backoff = Backoff(Version.V1_0_3)
    >>> backoff.version
    '1.0.3'
    """

    #: LoRaWAN 1.0.3.
    V1_0_3 = "1.0.3"
    #: TS001-1.0.4, the LoRaWAN 1.0.4 link layer.
    V1_0_4 = "1.0.4"


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
    """The direction a frame traveled, which its MIC and encryption fold in."""

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


#: One of the commands a network and a device configure each other with.
#:
#: kind names the command and cid is the identifier it travels under. Only the
#: fields that command carries are set; the rest are None. Writing one out is
#: command.encode().
MacCommand = LorawanMacCommand


def mac_parse(direction: Direction | str, data: bytes) -> list[MacCommand]:
    """Read the commands packed into a frame options field, or a port 0 payload.

    The same identifier means a different command in each direction, so the direction
    decides what is read and there is no default: 0x03 going down is a request to
    change data rate, and the same byte coming up answers one.

    A command does not carry its own length, so one this build does not know cannot be
    stepped over. Reading stops there and returns what came before it.

    Args:
        direction: Which way the frame carrying them travels.
        data: The options field, or the payload.

    Returns:
        The commands that were readable, in order.
    """
    name = direction.value if isinstance(direction, enum.Enum) else str(direction)
    return _mac_parse(name, data)


def end_device(
    plan,
    dev_eui: bytes,
    join_eui: bytes,
    app_key: bytes,
    settings: LorawanDeviceSettings,
    fcnt_up: int = 0,
    fcnt_down: int | None = None,
) -> LorawanEndDevice:
    """Make an end device that joins over the air.

    It owns no radio and no clock: every call takes the time in microseconds and hands
    back what to put on the air, so the same device runs over any radio, or in a test
    with none. :meth:`EndDevice.join` or :meth:`EndDevice.send` returns a transmission
    and two receive windows; a frame heard in either goes to :meth:`EndDevice.heard`,
    and :meth:`EndDevice.nothing_heard` says what to do when neither held one.

    :param plan: A published channel plan, from ``pamoja.lora.plan_for`` or
        ``pamoja.lora.cn470_plan``.
    :param dev_eui: The 8-byte device EUI.
    :param join_eui: The 8-byte join EUI.
    :param app_key: The 16-byte root key.
    :param settings: What the radio can do.
    :param fcnt_up: The next uplink frame counter carried over a restart.
    :param fcnt_down: The last downlink frame counter accepted, if any was.
    :returns: The device, not yet joined.
    :raises PamojaError: If the plan was built rather than published, or a credential is
        the wrong length.

    >>> from pamoja import lora
    >>> node = end_device(lora.plan_for("EU868"), bytes(8), bytes(8), bytes(16), DeviceSettings(2, 14))
    >>> request = node.join(1, 0)
    >>> request.rx1.delay_us
    5000000
    """
    return LorawanEndDevice.over_the_air(
        plan,
        LorawanDevice(bytes(dev_eui), bytes(join_eui), bytes(app_key)),
        settings,
        fcnt_up,
        fcnt_down,
    )
