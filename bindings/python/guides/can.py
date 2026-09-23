"""The CAN and J1939 guide example: a standby generator's bus, with the engine controller
broadcasting its speed, a monitoring gateway keeping only that, a service laptop hearing
everything, and a coolant sensor speaking plain CAN; see docs/guides/can.md."""

# ANCHOR: example
from pamoja.can import (
    NOT_AVAILABLE,
    CanBus,
    CanFilter,
    CanFrame,
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

# The nodes by the address each answers to, and the two parameter groups in play.
ENGINE = 0
GATEWAY = 1
ENGINE_CONTROLLER_1 = 61_444  # carries engine speed
REQUEST = 59_904  # asks another node for a parameter group

# Where engine speed sits inside that group, and the scale the standard fixes for it.
ENGINE_SPEED_AT = 3
RPM_PER_BIT = 0.125


def reading(speed_id: int, rpm: float) -> CanFrame:
    """A reading: every signal marked not available but the engine's speed."""
    reported = signals()
    reported.set_u16(ENGINE_SPEED_AT, int(rpm / RPM_PER_BIT))
    return frame(speed_id, reported.bytes, extended=True)


def rpm_of(received: CanFrame) -> float:
    """The engine speed a reading carries."""
    return (signals_from(received.data).u16(ENGINE_SPEED_AT) or 0) * RPM_PER_BIT


# J1939 keeps its addressing inside the 29-bit identifier: a priority, the parameter group, and
# the sender's address. A broadcast names no destination.
speed_id = broadcast_j1939(Priority.CONTROL, ENGINE_CONTROLLER_1, ENGINE)
speed = decode_j1939(speed_id)
print(
    f"engine speed 0x{speed_id:08X}: pgn {speed.pgn} at priority {speed.priority}, "
    f"from node {ENGINE} to every node"
)

first = reading(speed_id, 1500)
unreported = sum(1 for byte in first.data if byte == NOT_AVAILABLE)
print(
    f"payload      {rpm_of(first):.1f} rpm in bytes {ENGINE_SPEED_AT + 1} and "
    f"{ENGINE_SPEED_AT + 2}, the other {unreported} not available"
)

# Four nodes on one bus with nothing plugged in. On a Linux board each is
# CanBus.open("can0"), and nothing after this statement changes.
engine = CanBus.simulated()
gateway = engine.join()
laptop = engine.join()
sensor = engine.join()

# The gateway keeps engine speed and nothing else; the laptop keeps everything.
gateway.set_filters([CanFilter.pgn(ENGINE_CONTROLLER_1)])

# Two engine readings, and between them the coolant sensor, which speaks plain CAN: its level
# in percent on the 11-bit identifier 0x120.
engine.send(first)
sensor.send(frame(0x120, bytes([87])))
engine.send(reading(speed_id, 1512.5))

# Every node hears every frame but its own, and keeps what its filters pass.
while (kept := gateway.receive(timeout=0.01)) is not None:
    source = decode_j1939(kept.id, kept.extended).source
    print(f"gateway      {rpm_of(kept):.1f} rpm from node {source}")
on_the_bus = engine.sent + sensor.sent
print(f"gateway      kept {gateway.received} of the {on_the_bus} frames on the bus")
heard = []
while (received := laptop.receive(timeout=0.01)) is not None:
    heard.append(received)
plain = next((f for f in heard if decode_j1939(f.id, f.extended) is None), None)
if plain is not None:
    print(
        f"laptop       heard {len(heard)}, among them 0x{plain.id:03X}, "
        "an 11-bit identifier and no J1939 message"
    )

# A request is addressed rather than broadcast: below the PDU1 limit, eight bits of the
# identifier name the node it is for.
request = decode_j1939(compose_j1939(Priority.DEFAULT, REQUEST, GATEWAY, ENGINE))
print(f"request      pgn {request.pgn} from node {request.source} to node {request.destination}")

# The engine goes quiet. A receive waits for a frame up to its timeout; on a simulated bus it
# returns at once and counts the wait instead of sleeping through it.
before = gateway.waited_micros
quiet = gateway.receive(timeout=0.5)
waited = (gateway.waited_micros - before) // 1_000
print(f"silent       {0 if quiet is None else 1} frames in {waited} ms, counted and not slept")

# Above eight bytes CAN FD encodes a length in steps, and a classic frame refuses a ninth byte.
wide = fd_frame(speed_id, bytes(32), extended=True)
print(f"fd           32 bytes travel at data length code {wide.dlc}")
try:
    frame(speed_id, bytes(9), extended=True)
except PamojaError as error:
    print(f"classic      refused nine bytes: {error}")
# ANCHOR_END: example

assert speed_id == 0x0CF00400
assert unreported == 6
assert gateway.received == 2
assert len(heard) == 3
assert request.destination == ENGINE
assert quiet is None
assert wide.dlc == 13
