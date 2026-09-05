"""The CAN and J1939 guide example; see docs/guides/can.md."""

# ANCHOR: example
from pamoja.can import compose_j1939, decode_j1939, fd_frame, frame
from pamoja.core import PamojaError

# J1939 keeps its addressing inside the CAN identifier: a priority, a parameter group
# that says what the message is, and the address of whatever sent it. Building one from
# those fields is what saves a caller packing 29 bits by hand.
ENGINE = 0x00
EEC1 = 61_444  # electronic engine controller 1, which carries engine speed
broadcast = compose_j1939(3, EEC1, ENGINE)
engine = decode_j1939(broadcast)
print(f"broadcast priority {engine.priority} pgn {engine.pgn}")
print(f"addressed to one node: {not engine.broadcast}")

# A parameter group below the PDU1 limit is addressed rather than broadcast, so those
# eight identifier bits carry a destination instead of extending the group number.
REQUEST = 59_904
GATEWAY = 0x01
TRANSMISSION = 0x21
request = decode_j1939(compose_j1939(6, REQUEST, GATEWAY, TRANSMISSION))
print(f"request   pgn {request.pgn} to node 0x{request.destination:02X}")
print(f"heard     from 0x{request.source:02X}")

# J1939 never rides an 11-bit identifier, so a standard frame is not one.
print(f"an 11-bit identifier is J1939: {decode_j1939(0x123, extended=False) is not None}")

# The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
# parameter group at 0.125 rpm per bit, and every signal this controller is not
# reporting is filled with the not-available byte the standard reserves.
payload = bytearray([0xFF] * 8)
payload[3:5] = int(1000 / 0.125).to_bytes(2, "little")
eec1 = frame(broadcast, bytes(payload), extended=True)
speed = int.from_bytes(eec1.data[3:5], "little") * 0.125
print(f"engine    {speed} rpm in {eec1.dlc} bytes")

# Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
# classic frame still refuses a ninth byte.
print(f"32 bytes carries length code {fd_frame(broadcast, bytes(32), extended=True).dlc}")
try:
    frame(broadcast, bytes(9), extended=True)
    print("a classic frame took nine bytes, which should never happen")
except PamojaError as error:
    print(f"classic   refused nine bytes: {error}")
# ANCHOR_END: example

assert engine.priority == 3
assert engine.pgn == EEC1
assert engine.broadcast and engine.destination is None
assert request.pgn == REQUEST
assert request.destination == TRANSMISSION
assert request.source == GATEWAY
assert decode_j1939(0x123, extended=False) is None
assert eec1.dlc == 8
assert speed == 1000.0
assert fd_frame(broadcast, bytes(32), extended=True).dlc == 13
