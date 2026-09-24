"""The mesh framing guide example; see docs/guides/mesh.md."""

# ANCHOR: example
from pamoja.core import PamojaError
from pamoja.mesh import BROADCAST, HEADER_LEN, SeenPackets, broadcast, parse, relayed

# A river gauge floods a level reading to every node in range. The header is fixed and
# big-endian: version, source, destination, sequence id, hop limit, then the payload and a
# checksum over everything but the hop limit.
RIVER_GAUGE = 305419896
reading = broadcast(RIVER_GAUGE, 1, b"level=high")
to = "every node in range" if reading.dst == BROADCAST else "one node"
print(f"sent      {len(reading.bytes)} bytes to {to}, hop limit {reading.hop_limit}")

# A neighbor hears it. Every node in range rebroadcasts, so the same packet arrives several
# times over; the source and sequence id decide which copy is the first.
received = parse(reading.bytes)
print(f"payload   {received.payload.decode()}")
seen = SeenPackets(64)
first = seen.record(received.src, received.id)
again = seen.record(received.src, received.id)
if first and not again:
    print("dedup     the first copy is relayed, and the second is dropped")

# Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
# frame without recomputing it and the check stays end to end.
forwarded = relayed(received.bytes)
onward = parse(forwarded.bytes)
print(
    f"relayed   hop limit {forwarded.hop_limit}, and the checksum still holds: "
    f"{onward.payload.decode()}"
)

# A frame that has run out of hops is not relayed again, which is what ends the flood.
if relayed(broadcast(RIVER_GAUGE, 1, b"level=high", hop_limit=0).bytes) is None:
    print("spent     at hop limit 0 the frame goes no further")

# A payload byte the air mangled fails the checksum rather than reaching the application as a
# plausible reading. The header is a fixed width, so the first byte past it is the first byte
# of the reading itself.
mangled = bytearray(reading.bytes)
mangled[HEADER_LEN] ^= 0xFF
try:
    parse(bytes(mangled))
    print("a mangled frame was accepted, which should never happen")
except PamojaError as error:
    print(f"mangled   rejected: {error}")
# ANCHOR_END: example

assert received.payload == b"level=high"
assert forwarded.hop_limit == received.hop_limit - 1
assert onward.payload == received.payload

# ANCHOR: flood
from collections import deque

from pamoja.mesh import DEFAULT_HOP_LIMIT, MAX_PAYLOAD

# Six nodes down a river valley, each in range of its neighbors only. The gauge at the top,
# node 0, floods a reading, and every node that hears a packet for the first time takes it
# and relays it while hops remain. Copies that come back up the valley are echoes, which the
# memory of each node drops.
nodes = 6
memory = [SeenPackets(64) for _ in range(nodes)]
flooded = broadcast(0, 1, b"level=high")
memory[0].record(flooded.src, flooded.id)
on_the_air = deque([(0, flooded.bytes)])
delivered = relays = echoes = farthest = 0
while on_the_air:
    sender, sent = on_the_air.popleft()
    for node in (sender - 1, sender + 1):
        if not 0 <= node < nodes:
            continue
        heard = parse(sent)
        if not memory[node].record(heard.src, heard.id):
            echoes += 1
            continue
        delivered += 1
        farthest = max(farthest, node)
        onward_frame = relayed(heard.bytes)
        if onward_frame is not None:
            relays += 1
            on_the_air.append((node, onward_frame.bytes))
print(f"flood     {delivered} nodes took the reading, {relays} relayed it, {echoes} echoes were dropped")
print(
    f"reach     node {farthest} was the farthest, {farthest} hops out, "
    f"and node {nodes - 1} never heard it"
)

# A node whose memory holds two packets hears the reading, then two packets from other nodes,
# then a late copy of the reading by a longer path. It has forgotten the reading by then and
# relays it again; on a busy mesh that repeats without end.
small = SeenPackets(2)
rain = broadcast(7, 1, b"rain=4mm")
wind = broadcast(8, 1, b"wind=12")
small.record(flooded.src, flooded.id)
small.record(rain.src, rain.id)
small.record(wind.src, wind.id)
if small.record(flooded.src, flooded.id):
    print("forgot    a memory of two packets relays the late copy again")

# A payload one byte past what a frame carries is refused before it is built. A frame is
# sized to the 250 bytes ESP-NOW carries, less the header and the checksum.
try:
    broadcast(0, 2, bytes(MAX_PAYLOAD + 1))
    print("an oversized payload was framed, which should never happen")
except PamojaError as error:
    print(f"too long  {MAX_PAYLOAD + 1} bytes refused: {error}")
# ANCHOR_END: flood

assert farthest == DEFAULT_HOP_LIMIT + 1
