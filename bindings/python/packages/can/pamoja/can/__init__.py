"""Idiomatic CAN facade.

CAN is how the moving parts of a machine talk to each other: motor controllers,
servos, battery management, and the engines and farm equipment that speak J1939 on
top of it. This is the identifier and payload layer; the controller hardware
handles the wire itself.
"""

from __future__ import annotations

from pamoja._native import CanFrame, J1939Message
from pamoja._native import can_dlc_to_len as _dlc_to_len
from pamoja._native import can_fd_frame as _fd_frame
from pamoja._native import can_frame as _frame
from pamoja._native import can_len_to_dlc as _len_to_dlc
from pamoja._native import can_remote_frame as _remote_frame
from pamoja._native import j1939_compose as _j1939_compose
from pamoja._native import j1939_decode as _j1939_decode

__all__ = [
    "CanFrame",
    "J1939Message",
    "compose_j1939",
    "decode_j1939",
    "dlc_to_len",
    "fd_frame",
    "frame",
    "len_to_dlc",
    "remote_frame",
]


def frame(identifier: int, data: bytes, extended: bool = False) -> CanFrame:
    """Build a classic CAN 2.0 frame.

    :param identifier: The arbitration identifier, masked to the width ``extended``
        selects.
    :param data: The payload, at most eight bytes.
    :param extended: Whether the identifier is a 29-bit extended one.
    :returns: The frame.
    :raises PamojaError: If the payload is longer than a classic frame carries.
    """
    return _frame(identifier, extended, bytes(data))


def fd_frame(identifier: int, data: bytes, extended: bool = False) -> CanFrame:
    """Build a CAN-FD frame, which carries up to 64 bytes.

    :param identifier: The arbitration identifier.
    :param data: The payload, at one of the discrete CAN-FD lengths: 0 to 8, then
        12, 16, 20, 24, 32, 48, or 64 bytes.
    :param extended: Whether the identifier is a 29-bit extended one.
    :returns: The frame.
    :raises PamojaError: If the payload length is not one CAN-FD can carry.
    """
    return _fd_frame(identifier, extended, bytes(data))


def remote_frame(identifier: int, length: int, extended: bool = False) -> CanFrame:
    """Build a remote transmission request, which asks another node to send.

    :param identifier: The arbitration identifier.
    :param length: The data length being requested, clamped to eight bytes.
    :param extended: Whether the identifier is a 29-bit extended one.
    :returns: The frame, which carries no payload of its own.
    """
    return _remote_frame(identifier, extended, length)


def len_to_dlc(length: int) -> int:
    """Return the data length code that encodes a payload length.

    :param length: The payload length in bytes.
    :returns: The code, rounding up to the next length CAN-FD can carry.
    """
    return _len_to_dlc(length)


def dlc_to_len(dlc: int) -> int:
    """Return the payload length a data length code encodes.

    :param dlc: The data length code.
    :returns: The length in bytes.
    """
    return _dlc_to_len(dlc)


def decode_j1939(identifier: int, extended: bool = True) -> J1939Message | None:
    """Decode the J1939 fields out of an extended CAN identifier.

    :param identifier: The identifier as it arrived.
    :param extended: Whether it is a 29-bit extended identifier.
    :returns: The decoded message, or ``None`` for a standard identifier, which
        J1939 does not use.
    """
    return _j1939_decode(identifier, extended)


def compose_j1939(priority: int, pgn: int, source: int, destination: int = 0) -> int:
    """Compose the extended CAN identifier a set of J1939 fields describes.

    :param priority: The message priority, 0 (highest) to 7.
    :param pgn: The parameter group number.
    :param source: The address of the sending node.
    :param destination: The destination address, used only for an addressed (PDU1)
        parameter group and ignored for a broadcast (PDU2) one.
    :returns: The 29-bit identifier.
    """
    return _j1939_compose(priority, pgn, source, destination)
