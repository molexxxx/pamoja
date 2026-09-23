# pamoja-can

CAN 2.0 and CAN FD frames with 11- and 29-bit identifiers, J1939 decode and compose, and a node on a bus, simulated or a Linux interface through SocketCAN. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/can.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html)

## Install

```sh
pip install pamoja-can
```

```python
from pamoja import can
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/can.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/can.py):

```python
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-can`](https://crates.io/crates/pamoja-can) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html), [docs.rs](https://docs.rs/pamoja-can), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-can) |
| TypeScript | [`@pamoja/can`](https://www.npmjs.com/package/@pamoja/can) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-can) |
| Python | [`pamoja-can`](https://pypi.org/project/pamoja-can/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-can) |
| C# | [`Pamoja.Can`](https://www.nuget.org/packages/Pamoja.Can) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-can) |

## Documentation

- [`pamoja.can` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html), every class and function in this module.
- [The CAN and J1939 guide](https://pamoja.molex.cloud/docs/guides/can.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
