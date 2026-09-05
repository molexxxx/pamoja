"""The serial framing guide example; see docs/guides/serial.md."""

# ANCHOR: example
from pamoja.serial import COBS_DELIMITER, SLIP_END, SLIP_ESC, SlipDecoder, cobs, slip

# A UART carries bytes, not packets, so a framing has to mark where one packet ends. SLIP
# reserves two byte values for that, and the package names both: the end byte closes a
# frame, the escape byte carries a value that would otherwise look like one.
payload = b"lvl=" + bytes([SLIP_END, SLIP_ESC])
framed = slip.encode(payload)
print(f"slip      {len(payload)} payload bytes framed as {len(framed)}")

# Decoding gives the payload back unchanged, reserved bytes and all.
restored = slip.decode(framed)
print(f"slip      decoded back to {len(restored)} bytes")

# COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
# run led by its own length, so a frame never grows by more than a byte per 254. Zero is
# the delimiter, and never appears inside a frame.
packet = b"lvl=" + bytes([COBS_DELIMITER]) + b"7"
cobs_framed = cobs.encode(packet)
print(f"cobs      {len(packet)} payload bytes framed as {len(cobs_framed)}")

# A read from a port returns whatever arrived, which is rarely one whole frame. This chunk
# holds two good frames with a truncated one between them; the decoder hands over the good
# ones and discards only the bad frame.
decoder = SlipDecoder()
chunk = (
    b"ok"
    + bytes([SLIP_END])
    + bytes([SLIP_ESC])  # a frame that ends before its escape pair completes
    + bytes([SLIP_END])
    + b"go"
    + bytes([SLIP_END])
)
frames = decoder.feed(chunk)
for frame in frames:
    print(f"received  {frame.decode()}")
print(f"discarded {decoder.discarded} frame the stream mangled")
# ANCHOR_END: example

# The bytes each specification fixes are pinned once, in the crate tests and the
# generated conformance vectors, so a guide asserts behaviour instead.
assert len(framed) > len(payload)
assert len(cobs_framed) > len(packet)
assert restored == payload
assert cobs.decode(cobs_framed) == packet
assert frames == [b"ok", b"go"]
assert decoder.discarded == 1
