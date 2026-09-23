"""The serial framing guide example: a weather mast whose node sends COBS frames up a UART to a
gateway; see docs/guides/serial.md."""

# ANCHOR: example
from pamoja.hal import SerialPort, SerialSettings
from pamoja.serial import COBS_DELIMITER, CobsDecoder, cobs, slip


def reading(sequence: int, text: str) -> bytes:
    """A reading: a two-byte sequence number, most significant byte first, then its text."""
    return sequence.to_bytes(2, "big") + text.encode()


# The line: 115200 baud, eight data bits, no parity, one stop bit. Ten bits a character.
settings = SerialSettings(115_200)
print(
    f"line         {settings}, {settings.bits_per_character} bits a character, "
    f"{settings.character_nanos / 1_000:.2f} us each"
)

# The two ends of the cable with nothing plugged in. On a Raspberry Pi the gateway's end is
# SerialPort.open("/dev/serial0", settings) and nothing after this statement changes.
gateway, node = SerialPort.pair(settings)

# A UART carries bytes, and nothing marks where a message ends, so the node frames each reading
# with COBS: zero becomes the one byte that ends a frame and never appears inside one, which
# matters here, since the sequence number is full of zeros.
texts = ["wind=12.4", "wind=13.1", "wind=11.8"]
sent = 0
for sequence, text in enumerate(texts, start=1):
    frame = cobs.encode(reading(sequence, text))
    node.write(frame)
    sent += len(frame)
print(
    f"node         {len(texts)} readings of {len(reading(1, texts[0]))} bytes, "
    f"framed as {sent} bytes"
)

# A read returns whatever has arrived, which is rarely one frame: here it is all three. The
# decoder splits the stream back into payloads at each delimiter.
arrived = gateway.read(256, timeout=0.1)
print(f"gateway      {len(arrived)} bytes in one read")
decoder = CobsDecoder()
payloads = decoder.feed(arrived)
for payload in payloads:
    print(f"reading {int.from_bytes(payload[:2], 'big')}    {payload[2:].decode()}")

# What one frame costs on the wire at this speed, start and stop bits included.
frame_length = sent // len(texts)
print(
    f"on the wire  {settings.transfer_micros(frame_length) / 1_000:.2f} ms "
    f"for a {frame_length}-byte frame at {settings}"
)

# The node restarts partway through a frame. As it comes back up it sends a lone delimiter,
# which closes off the half frame, so the gateway drops it rather than gluing it to the next
# one, and then it sends the reading again.
again = cobs.encode(reading(4, "wind=12.9"))
node.write(again[: len(again) // 2])
node.write(bytes([COBS_DELIMITER]))
node.write(again)
dropped_before = decoder.discarded
resent = decoder.feed(gateway.read(256, timeout=0.1))
dropped = decoder.discarded - dropped_before
print(f"restart      {dropped} frame cut short and dropped, then {resent[0][2:].decode()}")

# SLIP, the older framing, ends a frame with one reserved byte and escapes that byte and its
# own escape byte inside one. With no reserved bytes in a reading it costs a byte less than
# COBS; a payload full of them costs up to twice its length under SLIP, and never more than
# one byte in 254 over under COBS.
first = reading(1, texts[0])
slip_length = len(slip.encode(first))
cobs_length = len(cobs.encode(first))
print(
    f"framing      {len(first)} payload bytes: {slip_length} under SLIP, "
    f"{cobs_length} under COBS"
)

# The node goes quiet. A read waits for the first byte up to its timeout; on a port with
# nothing plugged in it returns at once and counts the wait instead of sleeping through it, so
# a test of a silent node takes no time.
waited_before = gateway.waited_micros
quiet = gateway.read(256, timeout=0.5)
waited = (gateway.waited_micros - waited_before) // 1_000
print(f"silence      {len(quiet)} bytes in {waited} ms, counted and not slept")
# ANCHOR_END: example

assert settings.character_nanos == 86_806, "10 bits at 115200"
assert payloads == [reading(1, "wind=12.4"), reading(2, "wind=13.1"), reading(3, "wind=11.8")]
assert sent == 39, "each 11-byte payload gains one code byte and a delimiter"
assert settings.transfer_micros(13) == 1_129
assert dropped == 1
assert resent == [reading(4, "wind=12.9")]
assert (slip_length, cobs_length) == (12, 13)
assert gateway.waited_micros == 500_000
assert node.written == 39 + len(again) // 2 + 1 + len(again)
