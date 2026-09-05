"""The Modbus RTU guide example; see docs/guides/modbus.md."""

# ANCHOR: example
from pamoja.core import PamojaError
from pamoja.modbus import parse_frame, read_holding_registers, read_holding_registers_reply

# The device this gateway polls: a power meter at unit 17, whose manual says the three
# registers holding voltage, current and a fault word start at address 107.
METER = 17
FIRST_REGISTER = 107

# Ask it for those three registers. The frame is complete, checksum included, exactly as
# it goes out on the wire.
request = read_holding_registers(METER, FIRST_REGISTER, 3)
print(f"polling unit {METER}, {len(request)} bytes out")

# A stand-in for the meter. On a running gateway this frame arrives over RS485; here the
# library builds what a meter reporting those three values would send back.
from_the_meter = read_holding_registers_reply(METER, [2301, 418, 0])

# Everything below is the gateway's own code. A reply carries its own checksum, so the
# frame is validated before any value is read out of it.
reply = parse_frame(from_the_meter)
registers = reply.registers()
print(f"voltage   {registers[0] / 10:.1f} V")
print(f"current   {registers[1] / 100:.2f} A")
print(f"faults    {registers[2]}")

# One flipped bit anywhere in the frame fails the checksum, which is the whole point of
# carrying one over a long RS485 run.
mangled = bytearray(from_the_meter)
mangled[2] ^= 0xFF
try:
    parse_frame(bytes(mangled))
    print("mangled frame accepted, which should never happen")
except PamojaError as error:
    print(f"mangled frame rejected: {error}")
# ANCHOR_END: example

# The bytes each specification fixes are pinned once, in the crate tests and the
# generated conformance vectors, so a guide asserts behaviour instead.
assert len(request) == 8
assert reply.address == METER
assert reply.exception is None
assert registers == [2301, 418, 0]
