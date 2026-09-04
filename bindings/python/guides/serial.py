"""The serial framing guide example; see docs/guides/serial.md."""

# ANCHOR: example
from pamoja.serial import SlipDecoder, cobs, slip

# SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a payload
# carrying either goes out as the two-byte pair RFC 1055 fixes for it.
payload = bytes([0x01, 0xC0, 0xDB, 0x02])
frame = slip.encode(payload)
assert frame == bytes([0x01, 0xDB, 0xDC, 0xDB, 0xDD, 0x02, 0xC0])
assert slip.decode(frame) == payload

# COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
# run led by its own length. This is the worked example from the COBS paper.
packet = bytes([0x11, 0x22, 0x00, 0x33])
framed = cobs.encode(packet)
assert framed == bytes([0x03, 0x11, 0x22, 0x02, 0x33, 0x00])
assert cobs.decode(framed) == packet

# A read from a port returns an arbitrary chunk rather than a packet. This one holds two
# frames with a truncated one between them, and the decoder drops only the bad frame.
decoder = SlipDecoder()
frames = decoder.feed(bytes([0x6F, 0x6B, 0xC0, 0xDB, 0xC0, 0x67, 0x6F, 0xC0]))
assert frames == [b"ok", b"go"]
assert decoder.discarded == 1
# ANCHOR_END: example
