"""Idiomatic serial-framing facade.

A serial line is a stream of bytes with no packet boundaries, so something has to
mark where one message ends and the next begins. SLIP and COBS are the two ways to
do that, and each is offered both as a one-shot call over a complete frame and as
a streaming decoder for the arbitrary chunks a port delivers.

The streaming decoders are what a real read loop uses. A corrupt frame does not
raise, because the frames around it are still good; it is dropped and counted on
:attr:`SlipDecoder.discarded`.
"""

from __future__ import annotations

from typing import Protocol

from pamoja._native import CobsDecoder as _NativeCobsDecoder
from pamoja._native import SlipDecoder as _NativeSlipDecoder
from pamoja._native import cobs_decode as _cobs_decode
from pamoja._native import cobs_encode as _cobs_encode
from pamoja._native import cobs_max_encoded_len as _cobs_max_encoded_len
from pamoja._native import slip_decode as _slip_decode
from pamoja._native import slip_encode as _slip_encode
from pamoja._native import slip_max_encoded_len as _slip_max_encoded_len

__all__ = ["CobsDecoder", "Framing", "SlipDecoder", "cobs", "slip"]


class Framing(Protocol):
    """One of the two byte-stuffing framings this module offers."""

    def encode(self, payload: bytes) -> bytes:
        """Frame a payload for the wire.

        :param payload: The bytes to send.
        :returns: The frame, delimiter included.
        """

    def decode(self, frame: bytes) -> bytes:
        """Read the payload back out of a complete frame.

        :param frame: The frame as it arrived.
        :returns: The payload.
        :raises PamojaError: If the frame is corrupt.
        """

    def max_encoded_len(self, payload_len: int) -> int:
        """Return the largest frame a payload of this length can produce.

        :param payload_len: The payload length in bytes.
        :returns: The worst-case frame length.
        """


class _Slip:
    """SLIP (RFC 1055): an ``END`` byte ends a packet, and an escape pair carries it."""

    __slots__ = ()

    def encode(self, payload: bytes) -> bytes:
        """Frame a payload as a SLIP packet."""
        return _slip_encode(bytes(payload))

    def decode(self, frame: bytes) -> bytes:
        """Read the payload back out of a SLIP frame."""
        return _slip_decode(bytes(frame))

    def max_encoded_len(self, payload_len: int) -> int:
        """Return the worst-case SLIP frame length for a payload."""
        return _slip_max_encoded_len(payload_len)


class _Cobs:
    """COBS: removes the zero byte so one zero delimits packets unambiguously."""

    __slots__ = ()

    def encode(self, payload: bytes) -> bytes:
        """Frame a payload as a COBS packet."""
        return _cobs_encode(bytes(payload))

    def decode(self, frame: bytes) -> bytes:
        """Read the payload back out of a COBS frame."""
        return _cobs_decode(bytes(frame))

    def max_encoded_len(self, payload_len: int) -> int:
        """Return the worst-case COBS frame length for a payload."""
        return _cobs_max_encoded_len(payload_len)


#: SLIP framing, the simplest there is.
slip: Framing = _Slip()

#: COBS framing, for links where the overhead has to stay small and predictable.
cobs: Framing = _Cobs()


class SlipDecoder:
    """Reassembles whole SLIP frames from the chunks a serial port delivers.

    Example::

        decoder = SlipDecoder()
        while True:
            for frame in decoder.feed(port.read(256)):
                handle(frame)
    """

    __slots__ = ("_native",)

    def __init__(self) -> None:
        """Create an empty decoder, ready for the first chunk."""
        self._native = _NativeSlipDecoder()

    def feed(self, chunk: bytes) -> list[bytes]:
        """Feed a chunk of the stream.

        :param chunk: The bytes just read from the port.
        :returns: Every frame this chunk completed, in order, which is often none.
        """
        return self._native.feed(bytes(chunk))

    @property
    def discarded(self) -> int:
        """How many corrupt frames this decoder has discarded."""
        return self._native.discarded

    def reset(self) -> None:
        """Discard any partly assembled frame."""
        self._native.reset()


class CobsDecoder:
    """Reassembles whole COBS frames from the chunks a serial port delivers.

    The counterpart to :class:`SlipDecoder`, for links where the framing overhead
    has to stay small and predictable.
    """

    __slots__ = ("_native",)

    def __init__(self) -> None:
        """Create an empty decoder, ready for the first chunk."""
        self._native = _NativeCobsDecoder()

    def feed(self, chunk: bytes) -> list[bytes]:
        """Feed a chunk of the stream.

        :param chunk: The bytes just read from the port.
        :returns: Every frame this chunk completed, in order.
        """
        return self._native.feed(bytes(chunk))

    @property
    def discarded(self) -> int:
        """How many corrupt frames this decoder has discarded."""
        return self._native.discarded

    def reset(self) -> None:
        """Discard any partly assembled frame."""
        self._native.reset()
