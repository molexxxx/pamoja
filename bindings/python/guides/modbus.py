"""The Modbus RTU guide example; see docs/guides/modbus.md."""

# ANCHOR: example
from pamoja.core import PamojaError
from pamoja.modbus import crc16, parse_frame, read_holding_registers

# Ask unit 0x11 for three holding registers starting at 0x006B. The last two bytes are
# the CRC-16/MODBUS, so this is the frame exactly as it goes out on the wire.
request = read_holding_registers(0x11, 0x006B, 3)
assert request == bytes([0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87])

# The device answers with three 16-bit registers. A reply carries its own checksum, so
# the receiver validates the frame before reading any value out of it.
body = bytes([0x11, 0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64])
reply = parse_frame(body + crc16(body).to_bytes(2, "little"))
assert reply.address == 0x11
assert reply.exception is None
assert reply.registers() == [0x022B, 0x0000, 0x0064]

# One flipped bit anywhere in the frame fails the checksum, which is the whole point of
# carrying one over a long RS485 run.
corrupt = bytearray(body + crc16(body).to_bytes(2, "little"))
corrupt[2] ^= 0xFF
try:
    parse_frame(bytes(corrupt))
except PamojaError:
    pass
else:
    raise AssertionError("a frame mangled on the wire should be rejected")
# ANCHOR_END: example
