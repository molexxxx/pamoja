# pamoja-modbus

Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/modbus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-modbus`](https://crates.io/crates/pamoja-modbus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [docs.rs](https://docs.rs/pamoja-modbus) |
| TypeScript | [`@pamoja/modbus`](https://www.npmjs.com/package/@pamoja/modbus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html) |
| Python | [`pamoja-modbus`](https://pypi.org/project/pamoja-modbus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html) |
| C# | [`Pamoja.Modbus`](https://www.nuget.org/packages/Pamoja.Modbus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html) |

## Documentation

- [`pamoja.modbus` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html), every class and function in this module.
- [The Modbus RTU guide](https://pamoja.molex.cloud/docs/guides/modbus.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
