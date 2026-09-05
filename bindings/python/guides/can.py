"""The CAN and J1939 guide example; see docs/guides/can.md."""

# ANCHOR: example
from pamoja.can import (
    NOT_AVAILABLE,
    Priority,
    broadcast_j1939,
    compose_j1939,
    decode_j1939,
    fd_frame,
    frame,
    signals,
    signals_from,
)
from pamoja.core import PamojaError

# The nodes on this bus, by the address each answers to, and the two parameter groups
# in play. J1939 publishes both, so naming them is what makes the traffic readable.
ENGINE = 0
GATEWAY = 1
GEARBOX = 33
ENGINE_CONTROLLER_1 = 61_444  # carries engine speed
REQUEST = 59_904  # asks another node for a parameter group

# Where engine speed sits inside that group, and the scale the standard fixes for it.
# Naming both is what stops a sender and a receiver disagreeing about either.
ENGINE_SPEED_AT = 3
RPM_PER_BIT = 0.125

# J1939 keeps its addressing inside the CAN identifier: a priority, the parameter
# group, and the address of whatever sent it. A broadcast has no destination, so it is
# its own constructor rather than a magic address a caller has to know.
speed_id = broadcast_j1939(Priority.CONTROL, ENGINE_CONTROLLER_1, ENGINE)
speed = decode_j1939(speed_id)
print(f"broadcast pgn {speed.pgn} at priority {speed.priority}")

# A parameter group below the PDU1 limit is addressed rather than broadcast, so those
# eight identifier bits carry a destination instead of extending the group number.
request_id = compose_j1939(Priority.DEFAULT, REQUEST, GATEWAY, GEARBOX)
print(f"request   pgn {decode_j1939(request_id).pgn} addressed to node {GEARBOX}")

# Reading one back off the bus is the same thing in reverse, so a receiver never
# unpacks 29 bits by hand.
heard = decode_j1939(request_id)
print(f"heard     from node {heard.source} for node {heard.destination}")

# The payload. Every signal starts marked not available, and this controller reports
# only engine speed, so that is the only one it writes.
reported = signals()
reported.set_u16(ENGINE_SPEED_AT, int(1000 / RPM_PER_BIT))
eec1 = frame(speed_id, reported.bytes, extended=True)

# The receiving node reads the same offset back, so neither end slices the payload.
rpm = signals_from(eec1.data).u16(ENGINE_SPEED_AT) * RPM_PER_BIT
print(f"engine    {rpm} rpm, carried in {eec1.dlc} bytes")

# Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
# classic frame still refuses a ninth byte.
print(f"32 bytes carries length code {fd_frame(speed_id, bytes(32), extended=True).dlc}")
try:
    frame(speed_id, bytes(9), extended=True)
    print("a classic frame took nine bytes, which should never happen")
except PamojaError as error:
    print(f"classic   refused nine bytes: {error}")

# J1939 never rides an 11-bit identifier, so a standard frame is not one of its
# messages however its bits happen to line up.
print(f"an 11-bit identifier is J1939: {decode_j1939(291, extended=False) is not None}")
# ANCHOR_END: example

assert speed.priority == Priority.CONTROL
assert speed.pgn == ENGINE_CONTROLLER_1
assert speed.broadcast and speed.destination is None
assert heard.pgn == REQUEST
assert heard.destination == GEARBOX
assert heard.source == GATEWAY
assert rpm == 1000.0
assert eec1.dlc == 8
assert fd_frame(speed_id, bytes(32), extended=True).dlc == 13
assert decode_j1939(291, extended=False) is None
assert reported.u8(0) == NOT_AVAILABLE, "every signal this controller does not report"
