# pamoja-can

CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/can.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-can`](https://crates.io/crates/pamoja-can) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html), [docs.rs](https://docs.rs/pamoja-can) |
| TypeScript | [`@pamoja/can`](https://www.npmjs.com/package/@pamoja/can) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html) |
| Python | [`pamoja-can`](https://pypi.org/project/pamoja-can/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html) |
| C# | [`Pamoja.Can`](https://www.nuget.org/packages/Pamoja.Can) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html) |

## Documentation

- [`pamoja.can` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html), every class and function in this module.
- [The CAN and J1939 guide](https://pamoja.molex.cloud/docs/guides/can.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
