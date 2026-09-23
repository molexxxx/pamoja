"""Idiomatic CAN facade.

CAN is how the moving parts of a machine talk to each other: motor controllers,
servos, battery management, and the engines and farm equipment that speak J1939 on
top of it. :class:`CanBus` is one node on a bus, simulated or a kernel interface
through SocketCAN, and the rest is the identifier and payload layer; the controller
hardware handles the wire itself.
"""

from __future__ import annotations

import enum
from enum import IntEnum
from typing import Iterable

from pamoja._native import CanBus as _NativeBus
from pamoja._native import CanFilter, CanFrame, J1939Message, Signals
from pamoja._native import can_dlc_to_len as _dlc_to_len
from pamoja._native import can_fd_frame as _fd_frame
from pamoja._native import can_frame as _frame
from pamoja._native import can_len_to_dlc as _len_to_dlc
from pamoja._native import can_remote_frame as _remote_frame
from pamoja._native import j1939_compose as _j1939_compose
from pamoja._native import j1939_broadcast as _j1939_broadcast
from pamoja._native import j1939_decode as _j1939_decode
from pamoja._native import j1939_limits as _limits

__all__ = [
    "BROADCAST_ADDRESS",
    "NOT_AVAILABLE",
    "CanBus",
    "CanBusKind",
    "CanFilter",
    "CanFrame",
    "J1939Message",
    "Priority",
    "Signals",
    "broadcast_j1939",
    "compose_j1939",
    "decode_j1939",
    "dlc_to_len",
    "fd_frame",
    "frame",
    "len_to_dlc",
    "remote_frame",
    "signals",
    "signals_from",
]

_NOT_AVAILABLE, _BROADCAST_ADDRESS, _CONTROL, _DEFAULT, _LOWEST = _limits()

#: The byte a J1939 sender writes for a signal it is not reporting.
NOT_AVAILABLE = _NOT_AVAILABLE

#: The destination address every node on the bus reads.
BROADCAST_ADDRESS = _BROADCAST_ADDRESS


class Priority(IntEnum):
    """The priorities J1939 publishes, so a caller does not write the number out."""

    CONTROL = _CONTROL
    """Ahead of ordinary traffic, for a message that controls something."""

    DEFAULT = _DEFAULT
    """What ordinary traffic uses."""

    LOWEST = _LOWEST
    """Yields to everything else on the bus."""


def signals() -> Signals:
    """Build a J1939 payload with every signal marked not available.

    :returns: Eight bytes a controller writes only its own signals into.
    """
    return Signals()


def signals_from(data: bytes) -> Signals:
    """Read the eight data bytes of a frame that arrived off the bus.

    :param data: The frame's payload.
    :returns: The payload, ready for its signals to be read out.
    :raises PamojaError: The payload is not exactly eight bytes.
    """
    return Signals.from_bytes(data)


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


def broadcast_j1939(priority: int, pgn: int, source: int) -> int:
    """Compose the identifier of a J1939 broadcast, which every node reads.

    Most parameter groups are broadcast, so this is the common case; it saves a
    caller knowing that a broadcast is addressed to ``0xFF``.

    :param priority: The message priority, 0 (highest) to 7.
    :param pgn: The parameter group number.
    :param source: The address of the sending node.
    :returns: The 29-bit identifier.
    """
    return _j1939_broadcast(priority, pgn, source)


class CanBusKind(str, enum.Enum):
    """What a node's bus is."""

    #: A kernel CAN interface reached through SocketCAN.
    DEVICE = "Device"
    #: A bus inside the program.
    SIMULATED = "Simulated"


class CanBus:
    """One node's place on a CAN bus.

    :meth:`open` opens a kernel interface such as ``can0`` through SocketCAN on a Linux board,
    once it is up with ``ip link set can0 up type can bitrate 250000``. :meth:`simulated` makes
    a bus inside the program, and :meth:`join` puts another node on the same bus. A node hears
    every frame the others send and none of its own, and keeps only the frames its filters
    pass. A send and a receive release the interpreter while the bus is busy; a receive on a
    simulated bus with nothing waiting returns ``None`` at once and counts its timeout in
    :attr:`waited_micros`.

    >>> engine = CanBus.simulated()
    >>> gateway = engine.join()
    >>> speed = (500).to_bytes(2, "big")
    >>> engine.send(frame(0x20A, speed))
    >>> bytes(gateway.receive(timeout=0.01).data) == speed
    True
    """

    __slots__ = ("_native",)

    def __init__(self, native: _NativeBus) -> None:
        """Wrap a native node; use :meth:`open` or :meth:`simulated` instead."""
        self._native = native

    @classmethod
    def open(cls, interface: str) -> CanBus:
        """Open a kernel CAN interface through SocketCAN, as one node on its bus.

        :param interface: ``can0`` for the first controller, ``vcan0`` for a virtual one.
        :returns: The node.
        :raises PamojaError: Anywhere but Linux, or when the interface does not exist.
        """
        return cls(_NativeBus.open(interface))

    @classmethod
    def simulated(cls) -> CanBus:
        """A new bus inside the program, with this node the first on it.

        :returns: The node.
        """
        return cls(_NativeBus.simulated())

    def join(self) -> CanBus:
        """Put another node on the same bus.

        :returns: The new node.
        """
        return CanBus(self._native.join())

    @property
    def kind(self) -> CanBusKind:
        """What the bus is."""
        return CanBusKind(self._native.kind)

    @property
    def interface(self) -> str | None:
        """The kernel interface the node is on, or ``None`` on a simulated bus."""
        return self._native.interface

    def send(self, frame: CanFrame) -> None:
        """Send a frame to every other node on the bus.

        :param frame: The frame.
        :raises PamojaError: When the kernel refuses the frame.
        """
        self._native.send(frame)

    def receive(self, timeout: float) -> CanFrame | None:
        """Take the next frame the node keeps, waiting up to ``timeout`` for one.

        :param timeout: How long to wait, in seconds.
        :returns: The frame, or ``None`` when the timeout passed with nothing.
        :raises PamojaError: When the interface fails.
        """
        if not timeout >= 0:
            raise ValueError("a time must be zero or more seconds")
        return self._native.receive(round(timeout * 1_000_000))

    def set_filters(self, filters: Iterable[CanFilter]) -> None:
        """Keep only the frames that pass at least one of the filters, from now on.

        An empty list keeps nothing; :meth:`clear_filters` keeps everything again.

        :param filters: ``CanFilter.pgn(61444)``, ``CanFilter.exact(0x20A)``, and the like.
        """
        self._native.set_filters(list(filters))

    def clear_filters(self) -> None:
        """Keep every frame again, as a node does when it joins."""
        self._native.clear_filters()

    @property
    def sent(self) -> int:
        """How many frames the node has sent."""
        return self._native.sent

    @property
    def received(self) -> int:
        """How many frames the node has received."""
        return self._native.received

    @property
    def waited_micros(self) -> int:
        """How long receives have waited without a frame, whether or not the process slept."""
        return self._native.waited_micros