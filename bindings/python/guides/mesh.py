"""The mesh framing guide example; see docs/guides/mesh.md."""

# ANCHOR: example
from pamoja.core import PamojaError
from pamoja.mesh import BROADCAST, SeenPackets, broadcast, crc16, parse, relayed

# A river gauge floods a reading to every node in range. The header is fixed and
# big-endian: version, source, destination, sequence id, hop limit, then the payload
# and a checksum over everything but the hop limit.
reading = broadcast(0x12345678, 1, b"level=high")
assert reading.dst == BROADCAST
assert reading.bytes.hex() == "0112345678ffffffff0001036c6576656c3d686967683335"

# The checksum is CRC-16/CCITT-FALSE, whose published check value fixes the polynomial
# and the starting value.
assert crc16(b"123456789") == 0x29B1

# A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
# several times over; the source and sequence id decide which copy is the first.
received = parse(reading.bytes)
assert received.payload == b"level=high"
seen = SeenPackets(64)
assert seen.record(received.src, received.id)
assert not seen.record(received.src, received.id)

# Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
# frame without recomputing it and the check stays end to end.
forwarded = relayed(received.bytes)
assert forwarded.hop_limit == received.hop_limit - 1
assert parse(forwarded.bytes).payload == received.payload
assert relayed(broadcast(0x12345678, 1, b"level=high", 0).bytes) is None

# A payload byte the air mangled fails the checksum rather than reaching the application
# as a plausible reading.
mangled = bytearray(reading.bytes)
mangled[12] ^= 0xFF
try:
    parse(bytes(mangled))
except PamojaError:
    pass
else:
    raise AssertionError("a frame mangled on the air should be rejected")
# ANCHOR_END: example
