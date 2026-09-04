# pamoja-modbus

Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-modbus
```

```python
from pamoja import modbus
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/modbus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/modbus.py):

```python
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-modbus`](https://crates.io/crates/pamoja-modbus) | [docs.rs](https://docs.rs/pamoja-modbus), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html) |
| TypeScript | [`@pamoja/modbus`](https://www.npmjs.com/package/@pamoja/modbus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html) |
| Python | [`pamoja-modbus`](https://pypi.org/project/pamoja-modbus/) | [`pamoja.modbus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html) |
| C# | [`Pamoja.Modbus`](https://www.nuget.org/packages/Pamoja.Modbus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.Modbus.html) |

## Documentation

- [The Modbus RTU guide](https://pamoja.molex.cloud/docs/guides/modbus.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
