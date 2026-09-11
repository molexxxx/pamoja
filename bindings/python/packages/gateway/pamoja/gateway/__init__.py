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

from pamoja._native import GatewayPacket as Packet
from pamoja._native import GatewayRxpk as Rxpk
from pamoja._native import GatewayStat as Stat
from pamoja._native import GatewayTxpk as Txpk
from pamoja._native import gateway_acknowledgment as acknowledgment
from pamoja._native import gateway_encode as encode
from pamoja._native import gateway_parse as parse

__all__ = [
    "Crc",
    "DEFAULT_PORT",
    "Packet",
    "PacketKind",
    "Rxpk",
    "Stat",
    "TxStatus",
    "Txpk",
    "acknowledgment",
    "encode",
    "parse",
]

#: The port a packet forwarder sends to by convention, which the protocol itself does not fix.
DEFAULT_PORT = 1700


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
