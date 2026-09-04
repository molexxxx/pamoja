"""The MAVLink guide example; see docs/guides/mavlink.md."""

# ANCHOR: example
from pamoja.mavlink import MavlinkHeader, MavlinkParser, crc16, frame, known_crc_extra

# 0x6F91 over "123456789" is the catalogue check value for CRC-16/MCRF4XX, and 50 is the
# CRC_EXTRA the common dialect publishes for HEARTBEAT.
assert crc16(b"123456789") == 0x6F91
assert known_crc_extra(0) == 50

# A HEARTBEAT announcing an onboard controller in an active state. The v2 frame around it
# is the 0xFD marker, the payload length, two flag bytes, the sequence, the sending system
# and component, a 24-bit message id, the payload, then the checksum.
heartbeat = bytes([0, 0, 0, 0, 18, 0, 0, 4, 3])
sent = frame(MavlinkHeader(1, 1, 7), 0, heartbeat)
assert sent.bytes == bytes(
    [0xFD, 0x09, 0x00, 0x00, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
     0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x04, 0x03, 0x75, 0x3A]
)

# A link delivers bytes, not frames. The parser skips whatever does not start one and
# drops a frame whose checksum fails rather than passing it on.
mangled = bytearray(sent.bytes)
mangled[14] ^= 0xFF
parser = MavlinkParser()
assert parser.push(bytes([0x11, 0x22, 0x33]) + bytes(mangled)) == []

# The same frame, split across two reads, still arrives whole.
assert parser.push(sent.bytes[:5]) == []
found = parser.push(sent.bytes[5:])
assert len(found) == 1
assert found[0].version == 2
assert found[0].message_id == 0
assert found[0].payload == heartbeat
# ANCHOR_END: example
