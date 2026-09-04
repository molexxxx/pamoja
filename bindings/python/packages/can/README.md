# pamoja-can

CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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

# The engine-speed broadcast a J1939 engine or genset puts on the bus. J1939 keeps its
# addressing in the identifier: a priority, a parameter group, a source address.
engine = decode_j1939(0x0CF00400)
assert engine.priority == 3
assert engine.pgn == 61444
assert engine.broadcast and engine.destination is None

# A PDU format below 0xF0 is addressed rather than broadcast, so those eight bits hold a
# destination instead of extending the parameter group. 59904 is the request group.
request = decode_j1939(0x18EA2101)
assert request.pgn == 59904
assert request.destination == 0x21 and not request.broadcast
assert compose_j1939(6, 59904, 0x01, 0x21) == 0x18EA2101

# J1939 never rides an 11-bit identifier.
assert decode_j1939(0x123, extended=False) is None

# The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
# parameter group, little-endian at 0.125 rpm per bit, so 0x1F40 reads as 1000 rpm.
payload = bytes([0xF0, 0x7D, 0x7D, 0x40, 0x1F, 0x00, 0xF0, 0xFF])
eec1 = frame(0x0CF00400, payload, extended=True)
assert eec1.dlc == 8
assert int.from_bytes(eec1.data[3:5], "little") * 0.125 == 1000.0

# Above eight bytes CAN-FD encodes the length in steps, so 32 bytes is code 13, while a
# classic frame still refuses a ninth byte.
assert fd_frame(0x0CF00400, bytes(32), extended=True).dlc == 13
try:
    frame(0x0CF00400, bytes(9), extended=True)
except PamojaError:
    pass
else:
    raise AssertionError("classic CAN carries at most eight bytes")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-can`](https://crates.io/crates/pamoja-can) | [docs.rs](https://docs.rs/pamoja-can), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html) |
| TypeScript | [`@pamoja/can`](https://www.npmjs.com/package/@pamoja/can) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html) |
| Python | [`pamoja-can`](https://pypi.org/project/pamoja-can/) | [`pamoja.can`](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html) |
| C# | [`Pamoja.Can`](https://www.nuget.org/packages/Pamoja.Can) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.Can.html) |

## Documentation

- [The CAN and J1939 guide](https://pamoja.molex.cloud/docs/guides/can.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
