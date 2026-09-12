"""Idiomatic LoRaWAN gateway facade.

A gateway hears packets from every node in range and hands them to a network server, which
hands back the packets to transmit. The protocol between them is a plain exchange of UDP
datagrams, and this speaks it from both sides: build a datagram, :func:`encode` it onto a
socket of your own, and :func:`parse` whatever arrives.

Frequencies are in hertz, payloads are ``bytes`` rather than base64, and a reception time is a
count of microseconds, so nothing has to be formatted by hand.

Example::

    from pamoja.gateway import Packet, PacketKind, Rxpk, encode
    from pamoja.lora import link

    heard = Rxpk(868_100_000, b"hello", link=link(7, 125_000), rssi_dbm=-35.0, snr_db=5.1)
    datagram = encode(Packet(PacketKind.PUSH_DATA, 0x1234, gateway="b827ebfffe010203",
                             packets=[heard]))
"""

from __future__ import annotations

import enum

from pamoja._native import GatewayNetwork as Network
from pamoja._native import GatewayNetworkEvent as NetworkEvent
from pamoja._native import GatewayPacket as Packet
from pamoja._native import GatewayRxpk as Rxpk
from pamoja._native import GatewaySlot as Slot
from pamoja._native import GatewayStat as Stat
from pamoja._native import GatewayTxpk as Txpk
from pamoja._native import GatewayStationBroadcast as StationBroadcast
from pamoja._native import GatewayStationLevels as StationLevels
from pamoja._native import GatewayStationMessage as StationMessage
from pamoja._native import GatewayStationRouter as StationRouter
from pamoja._native import gateway_acknowledgment as acknowledgment
from pamoja._native import gateway_encode as encode
from pamoja._native import gateway_parse as parse
from pamoja._native import station_discovery
from pamoja._native import station_encode
from pamoja._native import station_eui_of
from pamoja._native import station_heard
from pamoja._native import station_id6
from pamoja._native import station_parse
from pamoja._native import station_router_parse

__all__ = [
    "DEFAULT_PORT",
    "DISCOVERY_PATH",
    "STATION_PROTOCOL_VERSION",
    "Crc",
    "Network",
    "NetworkEvent",
    "Packet",
    "PacketKind",
    "Rxpk",
    "Slot",
    "Stat",
    "StationBroadcast",
    "StationKind",
    "StationLevels",
    "StationMessage",
    "StationRouter",
    "TxStatus",
    "Txpk",
    "acknowledgment",
    "encode",
    "parse",
    "station_discovery",
    "station_encode",
    "station_eui_of",
    "station_heard",
    "station_id6",
    "station_parse",
    "station_router_parse",
]

#: The port a packet forwarder sends to by convention, which the protocol itself does not fix.
DEFAULT_PORT = 1700

#: The path a station appends to its configured address to find its network server.
DISCOVERY_PATH = "/router-info"

#: The protocol version a station reports.
STATION_PROTOCOL_VERSION = 2


class PacketKind(str, enum.Enum):
    """Which kind of datagram."""

    #: The gateway forwarding what it heard.
    PUSH_DATA = "PushData"
    #: The server acknowledging a PUSH_DATA.
    PUSH_ACK = "PushAck"
    #: The gateway holding its route open through any address translation in front of it.
    PULL_DATA = "PullData"
    #: The server sending a packet to transmit.
    PULL_RESP = "PullResp"
    #: The server acknowledging a PULL_DATA.
    PULL_ACK = "PullAck"
    #: The gateway reporting what became of a PULL_RESP.
    TX_ACK = "TxAck"


class Crc(str, enum.Enum):
    """What the CRC of a received packet said."""

    #: The CRC checked.
    OK = "Ok"
    #: The CRC failed.
    FAILED = "Failed"
    #: The packet carried no CRC.
    ABSENT = "Absent"


class TxStatus(str, enum.Enum):
    """What became of a downlink the server asked for, as the protocol names it."""

    #: It was scheduled.
    NONE = "NONE"
    #: It arrived too late to schedule.
    TOO_LATE = "TOO_LATE"
    #: Its timestamp is too far ahead.
    TOO_EARLY = "TOO_EARLY"
    #: Another packet was already scheduled then.
    COLLISION_PACKET = "COLLISION_PACKET"
    #: A beacon was already scheduled then.
    COLLISION_BEACON = "COLLISION_BEACON"
    #: The radio chain cannot reach that frequency.
    TX_FREQ = "TX_FREQ"
    #: The gateway cannot transmit at that power.
    TX_POWER = "TX_POWER"
    #: A GPS timestamp was asked for while the GPS is unlocked.
    GPS_UNLOCKED = "GPS_UNLOCKED"


class StationKind(str, enum.Enum):
    """Which kind of message a session carries, as the protocol writes it."""

    #: What the station reports about itself when a session opens.
    VERSION = "version"
    #: How the server tells the station to configure its radios.
    ROUTER_CONFIG = "router_config"
    #: A join request the station heard.
    JOIN_REQUEST = "jreq"
    #: A data frame the station heard.
    UPLINK = "updf"
    #: A frame of a kind this protocol does not describe, carried whole.
    PROPRIETARY = "propdf"
    #: A frame the server asks the station to transmit.
    DOWNLINK = "dnmsg"
    #: Frames the server asks the station to transmit to a group.
    SCHEDULE = "dnsched"
    #: What became of a frame the station was asked to transmit.
    TRANSMITTED = "dntxed"
    #: The clock the two keep between them.
    TIME_SYNC = "timesync"
