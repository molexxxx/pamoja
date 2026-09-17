"""A LoRaWAN relay, TS011-1.0.1: what an end device and a relay say to each other.

A relay sleeps, waking every scan period to look for radio activity. An end device out of a
gateway's reach first sends a wake-on-radio (WOR) frame whose preamble spans that sleep and
which says where its LoRaWAN uplink follows. The relay may acknowledge with its own timing, so
the next preamble can be short, then forwards the uplink to the network on port
:data:`LA_FPORT_RELAY` over its own session.

>>> from pamoja.lorawan import session
>>> from pamoja.lorawan import relay
>>> device = session(0x26011BDA, bytes([0x2B]) * 16, bytes([0x99]) * 16)
>>> keys = device.wor_keys()
>>> wor, uplink = relay.Carrier(865_100_000, 3), relay.Carrier(868_100_000, 5)
>>> frame = relay.wor_uplink(keys, device.dev_addr, 1, uplink, wor)
>>> relay.open_wor(frame, keys, 1, wor) == uplink
True
"""

from __future__ import annotations

import enum

from pamoja._native import (
    LORAWAN_FORWARD_OVERHEAD,
    LORAWAN_LA_FPORT_RELAY,
    LORAWAN_MIN_WOR_PREAMBLE_SYMBOLS,
    LORAWAN_RELAY_FWD_DELAY_US,
    LORAWAN_RXR_DELAY_US,
    LORAWAN_TRUSTED_ED_NUMBER,
    LORAWAN_WOR_ACK_DELAY_US,
    LORAWAN_WOR_ATTEMPTS_WO_ACK,
    LORAWAN_WOR_DATA_DELAY_US,
    LoraRelayChannel,
    LorawanCarrier,
    LorawanForwardedUplink,
    LorawanStateSync,
    LorawanSynchronization,
    LorawanUplinkMetadata,
    LorawanWor,
    LorawanWorKeys,
    LorawanWorSlot,
    lorawan_relay_forward_encode,
    lorawan_relay_forward_parse,
    lorawan_relay_next_wor,
    lorawan_relay_root_wor_s_key,
    lorawan_relay_second_channel,
    lorawan_relay_synchronization,
    lorawan_relay_t_offset_ms,
    lorawan_relay_unsynchronized_preamble,
    lorawan_relay_wor_ack,
    lorawan_relay_wor_ack_open,
    lorawan_relay_wor_join_request,
    lorawan_relay_wor_keys,
    lorawan_relay_wor_open,
    lorawan_relay_wor_parse,
    lorawan_relay_wor_uplink,
)

__all__ = [
    "FORWARD_OVERHEAD",
    "LA_FPORT_RELAY",
    "MIN_WOR_PREAMBLE_SYMBOLS",
    "RELAY_FWD_DELAY_US",
    "RXR_DELAY_US",
    "TRUSTED_ED_NUMBER",
    "WOR_ACK_DELAY_US",
    "WOR_ATTEMPTS_WO_ACK",
    "WOR_DATA_DELAY_US",
    "CadPeriodicity",
    "CadToRx",
    "Carrier",
    "ForwardedUplink",
    "RelayForward",
    "StateSync",
    "Synchronization",
    "UplinkMetadata",
    "Wor",
    "WorChannel",
    "WorKeys",
    "WorKind",
    "WorSlot",
    "XtalAccuracy",
    "encode_forward",
    "next_wor",
    "open_wor",
    "open_wor_ack",
    "parse_forward",
    "parse_wor",
    "root_wor_s_key",
    "second_channel",
    "synchronization",
    "t_offset_ms",
    "unsynchronized_preamble",
    "wor_ack",
    "wor_join_request",
    "wor_keys",
    "wor_uplink",
]

#: The port every message between a relay and its network uses, TS011-1.0.1 section 9.
LA_FPORT_RELAY = LORAWAN_LA_FPORT_RELAY
#: How many end devices a relay verifies wake-on-radio frames for.
TRUSTED_ED_NUMBER = LORAWAN_TRUSTED_ED_NUMBER
#: How many WOR frames go without an acknowledgment before the uplink goes anyway.
WOR_ATTEMPTS_WO_ACK = LORAWAN_WOR_ATTEMPTS_WO_ACK
#: The gap between a WOR frame, or its acknowledgment, and the LoRaWAN frame after it.
WOR_DATA_DELAY_US = LORAWAN_WOR_DATA_DELAY_US
#: The gap between a WOR frame and its acknowledgment.
WOR_ACK_DELAY_US = LORAWAN_WOR_ACK_DELAY_US
#: The gap between a relay hearing an uplink and forwarding it.
RELAY_FWD_DELAY_US = LORAWAN_RELAY_FWD_DELAY_US
#: How long after an uplink an end device's RXR window opens at the latest.
RXR_DELAY_US = LORAWAN_RXR_DELAY_US
#: The bytes a forwarded uplink adds in front of the end device's frame.
FORWARD_OVERHEAD = LORAWAN_FORWARD_OVERHEAD
#: The shortest WOR preamble, in symbols.
MIN_WOR_PREAMBLE_SYMBOLS = LORAWAN_MIN_WOR_PREAMBLE_SYMBOLS

#: Where a frame goes and how fast.
Carrier = LorawanCarrier
#: The integrity and encryption keys of one end device's WOR frames.
WorKeys = LorawanWorKeys
#: A WOR frame, as a relay reads it.
Wor = LorawanWor
#: What a relay tells an end device about itself in a WOR ACK.
StateSync = LorawanStateSync
#: What a relay heard of an uplink it forwards.
UplinkMetadata = LorawanUplinkMetadata
#: An end device's uplink as a relay forwards it on port 226.
ForwardedUplink = LorawanForwardedUplink
#: What an end device knows of a relay's scans once a WOR ACK has arrived.
Synchronization = LorawanSynchronization
#: When a synchronized end device's next WOR frame goes out.
WorSlot = LorawanWorSlot


class WorKind(str, enum.Enum):
    """Which WOR frame a relay heard."""

    #: Ahead of a join request, which nothing protects.
    JOIN_REQUEST = "join_request"
    #: Ahead of a Class A uplink, sealed with the device's WOR keys.
    UPLINK = "uplink"


class CadPeriodicity(str, enum.Enum):
    """How often a relay scans a channel for a WOR preamble, TS011-1.0.1 table 18."""

    #: Once a second, the default.
    MS1000 = "ms1000"
    #: Every 500 milliseconds.
    MS500 = "ms500"
    #: Every 250 milliseconds.
    MS250 = "ms250"
    #: Every 100 milliseconds.
    MS100 = "ms100"
    #: Every 50 milliseconds.
    MS50 = "ms50"
    #: Every 20 milliseconds.
    MS20 = "ms20"


class CadToRx(str, enum.Enum):
    """How many symbols a relay takes from detecting activity to receiving, table 15."""

    #: Two symbols.
    SYMBOLS2 = "symbols2"
    #: Four symbols.
    SYMBOLS4 = "symbols4"
    #: Six symbols.
    SYMBOLS6 = "symbols6"
    #: Eight symbols, which a device assumes before it has heard from a relay.
    SYMBOLS8 = "symbols8"


class XtalAccuracy(str, enum.Enum):
    """How accurate a relay's crystal is, table 17."""

    #: Better than 10 parts per million.
    PPM10 = "ppm10"
    #: Better than 20.
    PPM20 = "ppm20"
    #: Better than 30.
    PPM30 = "ppm30"
    #: Better than 40, which a device assumes before it has heard from a relay.
    PPM40 = "ppm40"


class RelayForward(str, enum.Enum):
    """Whether a relay will forward the uplink after a WOR frame, table 16."""

    #: It has room to.
    AVAILABLE = "available"
    #: A forwarding limit is reached; try again in 30 minutes.
    RETRY_IN_30_MINUTES = "retry_in_30_minutes"
    #: A forwarding limit is reached; try again in 60 minutes.
    RETRY_IN_60_MINUTES = "retry_in_60_minutes"
    #: Forwarding is off.
    DISABLED = "disabled"


class WorChannel(str, enum.Enum):
    """Which of a relay's channels a WOR frame arrived on, table 28."""

    #: The default channel.
    DEFAULT = "default"
    #: The second channel a network configured.
    SECOND = "second"


def _name(value: str | enum.Enum) -> str:
    return value.value if isinstance(value, enum.Enum) else str(value)


def root_wor_s_key(network_key: bytes) -> bytes:
    """Derive an end device's root relay session key from its network key, section 4.4.

    :param network_key: The 16-byte network session key.
    :returns: The 16-byte root key.
    :raises PamojaError: If the key is not 16 bytes.
    """
    return lorawan_relay_root_wor_s_key(bytes(network_key))


def wor_keys(root_key: bytes, dev_addr: int) -> LorawanWorKeys:
    """Derive an end device's WOR keys from its root relay session key, section 4.5.

    :param root_key: The 16-byte root key.
    :param dev_addr: The device's address.
    :returns: The integrity and encryption keys.
    :raises PamojaError: If the key is not 16 bytes.
    """
    return lorawan_relay_wor_keys(bytes(root_key), dev_addr)


def wor_join_request(uplink: LorawanCarrier) -> bytes:
    """Build the WOR frame ahead of a join request, section 5.3.1.

    :param uplink: Where and how fast the join request follows.
    :returns: The five-byte frame.
    :raises PamojaError: For a data rate past 15 or a frequency the field cannot carry.
    """
    return lorawan_relay_wor_join_request(uplink)


def wor_uplink(
    keys: LorawanWorKeys, dev_addr: int, wfcnt: int, uplink: LorawanCarrier, wor: LorawanCarrier
) -> bytes:
    """Build the WOR frame ahead of a Class A uplink, section 5.3.2.

    :param keys: The device's WOR keys.
    :param dev_addr: Its address.
    :param wfcnt: The WOR frame counter, raised for every WOR frame.
    :param uplink: Where and how fast the uplink follows.
    :param wor: The carrier this WOR frame goes out on.
    :returns: The fifteen-byte frame.
    :raises PamojaError: For a carrier the fields cannot carry.
    """
    return lorawan_relay_wor_uplink(keys, dev_addr, wfcnt, uplink, wor)


def parse_wor(frame: bytes) -> LorawanWor:
    """Read a WOR frame, leaving an uplink's carrier sealed.

    :param frame: The bytes a relay received.
    :returns: The frame's kind, and a join request's carrier or an uplink's address and
        counter.
    :raises PamojaError: For a reserved or proprietary type, or a length that is not the
        type's.
    """
    return lorawan_relay_wor_parse(bytes(frame))


def open_wor(frame: bytes, keys: LorawanWorKeys, wfcnt: int, wor: LorawanCarrier) -> LorawanCarrier:
    """Check a WOR frame ahead of a Class A uplink and read where the uplink follows.

    :param frame: The frame a relay received.
    :param keys: The keys of the device it names.
    :param wfcnt: The full 32-bit counter the relay takes it to carry.
    :param wor: The carrier it arrived on.
    :returns: Where and how fast the uplink follows.
    :raises PamojaError: When the frame is not an uplink WOR, or its integrity code does not
        verify.
    """
    return lorawan_relay_wor_open(bytes(frame), keys, wfcnt, wor)


def wor_ack(
    keys: LorawanWorKeys,
    dev_addr: int,
    wfcnt: int,
    ack: LorawanCarrier,
    uplink: LorawanCarrier,
    state: LorawanStateSync,
) -> bytes:
    """Build a relay's WOR ACK, section 6.2.

    :param keys: The end device's WOR keys.
    :param dev_addr: Its address.
    :param wfcnt: The counter of the acknowledged WOR frame.
    :param ack: The carrier the acknowledgment goes out on.
    :param uplink: The carrier the WOR frame named for the uplink.
    :param state: What the relay tells the device.
    :returns: The seven-byte acknowledgment.
    :raises PamojaError: For a state or carrier the fields cannot carry.
    """
    return lorawan_relay_wor_ack(keys, dev_addr, wfcnt, ack, uplink, state)


def open_wor_ack(
    frame: bytes,
    keys: LorawanWorKeys,
    dev_addr: int,
    wfcnt: int,
    ack: LorawanCarrier,
    uplink: LorawanCarrier,
) -> LorawanStateSync:
    """Check and read a WOR ACK, section 6.2.

    :param frame: The acknowledgment an end device received.
    :param keys: The device's WOR keys.
    :param dev_addr: Its address.
    :param wfcnt: The counter of the WOR frame it sent.
    :param ack: The carrier the acknowledgment arrived on.
    :param uplink: The carrier the WOR frame named.
    :returns: What the relay said about itself.
    :raises PamojaError: When the integrity code does not verify, or the periodicity is
        reserved.
    """
    return lorawan_relay_wor_ack_open(bytes(frame), keys, dev_addr, wfcnt, ack, uplink)


def state_sync(
    cad_to_rx: str | CadToRx,
    forward: str | RelayForward,
    relay_data_rate: int,
    xtal_accuracy: str | XtalAccuracy,
    cad_periodicity: str | CadPeriodicity,
    t_offset_ms: int,
) -> LorawanStateSync:
    """Describe what a relay tells an end device, taking the enums or their names.

    :param cad_to_rx: How long the relay takes to start receiving.
    :param forward: Whether it forwards.
    :param relay_data_rate: The data rate it forwards at.
    :param xtal_accuracy: How accurate its crystal is.
    :param cad_periodicity: How often it scans.
    :param t_offset_ms: Milliseconds from the scan to the end of the WOR preamble.
    :returns: The state.
    :raises PamojaError: For a name that is not one of the enum's.
    """
    return LorawanStateSync(
        _name(cad_to_rx),
        _name(forward),
        relay_data_rate,
        _name(xtal_accuracy),
        _name(cad_periodicity),
        t_offset_ms,
    )


def encode_forward(
    metadata: LorawanUplinkMetadata, frequency_hz: int, phy_payload: bytes
) -> bytes:
    """Write an uplink a relay forwards on port 226, section 9.1.

    A strength or ratio past what its field carries goes out as the closest value.

    :param metadata: What the relay heard of it.
    :param frequency_hz: The frequency it arrived on.
    :param phy_payload: The end device's frame.
    :returns: The relay uplink's payload.
    :raises PamojaError: For a data rate or frequency the fields cannot carry.
    """
    return lorawan_relay_forward_encode(metadata, frequency_hz, bytes(phy_payload))


def parse_forward(payload: bytes) -> LorawanForwardedUplink:
    """Read an uplink a relay forwarded, section 9.1.

    :param payload: The relay uplink's payload on port 226.
    :returns: The metadata, the frequency, and the end device's frame.
    :raises PamojaError: For a payload too short or a reserved WOR channel.
    """
    return lorawan_relay_forward_parse(bytes(payload))


def unsynchronized_preamble(
    cad_periodicity: str | CadPeriodicity, symbol_us: int, cad_to_rx: str | CadToRx
) -> int:
    """The WOR preamble of an end device that does not know when the relay scans, section 5.2.

    :param cad_periodicity: How often the relay scans.
    :param symbol_us: The symbol time of the WOR frame's data rate, in microseconds.
    :param cad_to_rx: The relay's time to start receiving.
    :returns: The preamble length in symbols.

    >>> unsynchronized_preamble(CadPeriodicity.MS500, 8_192, CadToRx.SYMBOLS4)
    72
    """
    return lorawan_relay_unsynchronized_preamble(_name(cad_periodicity), symbol_us, _name(cad_to_rx))


def t_offset_ms(scan_start_us: int, wor_end_us: int, wor_airtime_us: int, symbol_us: int) -> int | None:
    """The offset a relay reports in a WOR ACK, appendix 1.

    :param scan_start_us: When the scan that detected the frame started.
    :param wor_end_us: When the frame finished arriving.
    :param wor_airtime_us: Its time on air.
    :param symbol_us: The symbol time of its data rate.
    :returns: The offset in milliseconds, or ``None`` when it is negative or past eleven bits.
    """
    return lorawan_relay_t_offset_ms(scan_start_us, wor_end_us, wor_airtime_us, symbol_us)


def synchronization(
    wor_start_us: int, preamble_symbols: int, symbol_us: int, state: LorawanStateSync
) -> LorawanSynchronization:
    """Work out when a relay scanned from the WOR ACK that answered a frame, appendix 1.

    :param wor_start_us: When the acknowledged WOR frame started going out.
    :param preamble_symbols: Its preamble length.
    :param symbol_us: Its symbol time.
    :param state: What the acknowledgment said.
    :returns: The synchronization.
    """
    return lorawan_relay_synchronization(wor_start_us, preamble_symbols, symbol_us, state)


def next_wor(
    sync: LorawanSynchronization,
    now_us: int,
    device_xtal_ppm: int,
    symbol_us: int,
    other_channel: bool = False,
) -> LorawanWorSlot | None:
    """Pick the relay scan a synchronized end device aims its next WOR frame at, appendix 1.

    :param sync: What the device knows of the relay.
    :param now_us: The time.
    :param device_xtal_ppm: The device's crystal accuracy.
    :param symbol_us: The symbol time of the WOR frame's data rate.
    :param other_channel: Whether the frame goes out on the relay's other channel.
    :returns: When to send and how long a preamble, or ``None`` once the drift exceeds a
        period.
    """
    return lorawan_relay_next_wor(sync, now_us, device_xtal_ppm, symbol_us, other_channel)


def second_channel(
    second_channel_index: int, data_rate: int, ack_offset: int, frequency_hz: int
) -> LoraRelayChannel | None:
    """Read the second channel a relay or end device configuration describes.

    :param second_channel_index: The coded index, 1 for a second channel.
    :param data_rate: Its data rate.
    :param ack_offset: The coded acknowledgment offset, table 35.
    :param frequency_hz: Its frequency.
    :returns: The channel with its acknowledgment frequency, or ``None`` when none is named.
    """
    return lorawan_relay_second_channel(second_channel_index, data_rate, ack_offset, frequency_hz)
