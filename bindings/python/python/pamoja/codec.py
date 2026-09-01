"""Idiomatic codec facade.

Python already has :mod:`json`, so this facade takes and returns ordinary values
and does the encoding itself, leaving callers to think in documents rather than
buffers. The conversion and the packing happen in the Rust core.
"""

from __future__ import annotations

import json
from typing import Any, Sequence

from ._core import Quantizer as _NativeQuantizer
from ._core import cbor_to_json_bytes as _cbor_to_json_bytes
from ._core import decode_delta_samples as _decode_delta_samples
from ._core import encode_delta_samples as _encode_delta_samples
from ._core import json_to_cbor_bytes as _json_to_cbor_bytes

__all__ = ["Quantizer", "from_cbor", "pack_samples", "to_cbor", "unpack_samples"]


def to_cbor(value: Any) -> bytes:
    """Encode a value as CBOR, typically much smaller than its JSON form.

    :param value: Any JSON-serializable value, or the raw bytes of a JSON
        document.
    :returns: The CBOR encoding.
    :raises PamojaError: If the value cannot be encoded.
    """
    if isinstance(value, (bytes, bytearray, memoryview)):
        document = bytes(value)
    else:
        document = json.dumps(value).encode("utf-8")
    return _json_to_cbor_bytes(document)


def from_cbor(cbor: bytes) -> Any:
    """Decode a CBOR document back into an ordinary Python value.

    :param cbor: The CBOR document to decode.
    :returns: The decoded value.
    :raises PamojaError: If the document is malformed, or holds a construct with
        no JSON equivalent such as a non-string map key.
    """
    return json.loads(_cbor_to_json_bytes(bytes(cbor)).decode("utf-8"))


def pack_samples(samples: Sequence[int]) -> bytes:
    """Delta-encode a series of integer samples into a compact buffer.

    :param samples: The samples, in order.
    :returns: The packed encoding, far smaller than the samples for a
        slow-moving series.
    """
    return _encode_delta_samples(list(samples))


def unpack_samples(data: bytes) -> list[int]:
    """Unpack a buffer produced by :func:`pack_samples`.

    :param data: The packed encoding.
    :returns: The samples, in order.
    :raises PamojaError: If the buffer is malformed.
    """
    return _decode_delta_samples(bytes(data))


class Quantizer:
    """Packs float readings to a fixed precision, for a link charging per byte.

    Example::

        quantizer = Quantizer(100)  # keep two decimal places
        packed = quantizer.encode([20.0, 20.1, 20.2])
        quantizer.decode(packed)  # [20.0, 20.1, 20.2], to within 0.01
    """

    __slots__ = ("_native",)

    def __init__(self, scale: float) -> None:
        """Create a quantizer at the given precision.

        :param scale: The multiplier applied before rounding; ``100`` keeps two
            decimal places. Must be positive and finite.
        :raises ValueError: If the scale is not positive and finite.
        """
        self._native = _NativeQuantizer(scale)

    def encode(self, readings: Sequence[float]) -> bytes:
        """Quantize and pack a batch of readings.

        :param readings: The readings, in order.
        :returns: The packed encoding.
        """
        return self._native.encode(list(readings))

    def decode(self, data: bytes) -> list[float]:
        """Unpack a batch, to within this quantizer's precision.

        :param data: The encoding produced by :meth:`encode` at the same scale.
        :returns: The readings, in order.
        :raises PamojaError: If the buffer is malformed.
        """
        return self._native.decode(bytes(data))
