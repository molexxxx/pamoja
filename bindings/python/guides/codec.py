"""The codecs guide example; see docs/guides/codec.md."""

# ANCHOR: example
from pamoja.codec import Quantizer, from_cbor, pack_samples, to_cbor, unpack_samples

# The same reading in CBOR instead of JSON, half the bytes. 21.5 rides as a
# half-precision float, the shortest form RFC 8949 allows for it, so these are the
# bytes the specification fixes rather than one encoder's dialect.
reading = {"c": 21.5, "ok": True}
cbor = to_cbor(reading)
assert cbor == bytes([0xA2, 0x61, 0x63, 0xF9, 0x4D, 0x60, 0x62, 0x6F, 0x6B, 0xF5])
assert from_cbor(cbor) == reading

# A batch of samples packs to a count, then the difference between each value and the
# one before it, zigzagged and written as a LEB128 varint. The four small steps cost a
# byte each; the jump to 900 zigzags to 1776 and costs the two bytes 0xF0 0x0D.
samples = [10, 11, 13, 12, 900]
packed = pack_samples(samples)
assert packed == bytes([0x05, 0x14, 0x02, 0x04, 0x01, 0xF0, 0x0D])
assert unpack_samples(packed) == samples

# The quantizer packs float readings the same way, rounding at the scale first. Nothing
# in the bytes records the scale, so encode and decode have to agree on it.
quantizer = Quantizer(100)
readings = [20.0, 20.1, 20.2, 20.3]
packed_readings = quantizer.encode(readings)
assert packed_readings == bytes([0x04, 0xA0, 0x1F, 0x14, 0x14, 0x14])
restored = quantizer.decode(packed_readings)
assert all(abs(got - want) <= 0.01 for got, want in zip(restored, readings))
# ANCHOR_END: example
