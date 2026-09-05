"""The codecs guide example; see docs/guides/codec.md."""

# ANCHOR: example
import json

from pamoja.codec import Quantizer, from_cbor, pack_samples, to_cbor, unpack_samples

# The same reading as JSON and as CBOR. Nothing is lost, and 21.5 rides as a
# half-precision float, the shortest form RFC 8949 allows for it.
reading = {"c": 21.5, "ok": True}
as_json = json.dumps(reading, separators=(",", ":")).encode()
cbor = to_cbor(reading)
print(f"json      {len(as_json)} bytes")
print(f"cbor      {len(cbor)} bytes")

# A gateway that speaks JSON gets it back unchanged, so the compact form is a transport
# choice rather than a different data model.
restored = from_cbor(cbor)
print(f"back to json, unchanged: {restored == reading}")

# A batch of readings packs to a count, then the difference between each sample and the
# one before it. Successive readings differ by very little, so the differences cost about
# a byte each where the samples would cost eight.
samples = [10, 11, 13, 12, 900]
packed = pack_samples(samples)
print(f"batch     {len(samples)} samples in {len(packed)} bytes")
print(f"unpacked  {unpack_samples(packed)}")

# Readings that arrive as floats pack the same way once a scale is chosen. Nothing in the
# bytes records that scale, so the sender and the receiver have to agree on it.
quantizer = Quantizer(100)
celsius = [20.0, 20.1, 20.2, 20.3]
packed_celsius = quantizer.encode(celsius)
recovered = quantizer.decode(packed_celsius)
print(f"degrees   {len(celsius)} readings in {len(packed_celsius)} bytes")
print(f"recovered {[round(value, 1) for value in recovered]}")
# ANCHOR_END: example

# The bytes each specification fixes are pinned once, in the crate tests and the
# generated conformance vectors, so a guide asserts behaviour instead.
assert len(cbor) < len(as_json)
assert restored == reading
assert unpack_samples(packed) == samples
assert len(packed) < len(samples) * 8
assert all(abs(got - want) <= 0.01 for got, want in zip(recovered, celsius))
