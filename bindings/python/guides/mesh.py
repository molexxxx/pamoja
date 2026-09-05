"""The mesh framing guide example; see docs/guides/mesh.md."""

# ANCHOR: example
from pamoja.core import PamojaError
from pamoja.mesh import BROADCAST, HEADER_LEN, SeenPackets, broadcast, parse, relayed

# A river gauge floods a level reading to every node in range. The header is fixed and
# big-endian: version, source, destination, sequence id, hop limit, then the payload and a
# checksum over everything but the hop limit.
RIVER_GAUGE = 305419896
reading = broadcast(RIVER_GAUGE, 1, b"level=high")
print(f"sent      {len(reading.bytes)} bytes to every node in range")
print(f"addressed to broadcast: {reading.dst == BROADCAST}")

# A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
# several times over; the source and sequence id decide which copy is the first.
received = parse(reading.bytes)
print(f"payload   {received.payload.decode()}")

seen = SeenPackets(64)
first = seen.record(received.src, received.id)
again = seen.record(received.src, received.id)
print(f"first copy relayed: {first}, second copy relayed: {again}")

# Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
# frame without recomputing it and the check stays end to end.
forwarded = relayed(received.bytes)
print(f"relayed   hop limit {forwarded.hop_limit}")
onward = parse(forwarded.bytes)
print(f"onward    {onward.payload.decode()}")

# A frame that has run out of hops is not relayed again, which is what ends the flood.
spent = relayed(broadcast(RIVER_GAUGE, 1, b"level=high", 0).bytes)
if spent is None:
    print("spent     hop limit reached, the flood stops here")
else:
    print("a spent frame was relayed, which should never happen")

# A payload byte the air mangled fails the checksum rather than reaching the application
# as a plausible reading. The header is a fixed width, so the first byte past it is the
# first byte of the reading itself.
mangled = bytearray(reading.bytes)
mangled[HEADER_LEN] ^= 0xFF
try:
    parse(bytes(mangled))
    print("a mangled frame was accepted, which should never happen")
except PamojaError as error:
    print(f"mangled   rejected: {error}")
# ANCHOR_END: example

# The bytes each specification fixes are pinned once, in the crate tests and the
# generated conformance vectors, so a guide asserts behaviour instead.
assert received.payload == b"level=high"
assert first
assert not again
assert forwarded.hop_limit == received.hop_limit - 1
assert onward.payload == received.payload
assert spent is None
