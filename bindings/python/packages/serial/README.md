# pamoja-serial

SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/serial.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-serial
```

```python
from pamoja import serial
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/serial.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/serial.py):

```python
from pamoja.serial import COBS_DELIMITER, SLIP_END, SLIP_ESC, SlipDecoder, cobs, slip

# A UART carries bytes, not packets, so a framing has to mark where one packet ends. SLIP
# reserves two byte values for that, and the package names both: the end byte closes a
# frame, the escape byte carries a value that would otherwise look like one.
payload = b"lvl=" + bytes([SLIP_END, SLIP_ESC])
framed = slip.encode(payload)
print(f"slip      {len(payload)} payload bytes framed as {len(framed)}")

# Decoding gives the payload back unchanged, reserved bytes and all.
restored = slip.decode(framed)
print(f"slip      decoded back to {len(restored)} bytes")

# COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
# run led by its own length, so a frame never grows by more than a byte per 254. Zero is
# the delimiter, and never appears inside a frame.
packet = b"lvl=" + bytes([COBS_DELIMITER]) + b"7"
cobs_framed = cobs.encode(packet)
print(f"cobs      {len(packet)} payload bytes framed as {len(cobs_framed)}")

# A read from a port returns whatever arrived, which is rarely one whole frame. This chunk
# holds two good frames with a truncated one between them; the decoder hands over the good
# ones and discards only the bad frame.
decoder = SlipDecoder()
chunk = (
    b"ok"
    + bytes([SLIP_END])
    + bytes([SLIP_ESC])  # a frame that ends before its escape pair completes
    + bytes([SLIP_END])
    + b"go"
    + bytes([SLIP_END])
)
frames = decoder.feed(chunk)
for frame in frames:
    print(f"received  {frame.decode()}")
print(f"discarded {decoder.discarded} frame the stream mangled")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-serial`](https://crates.io/crates/pamoja-serial) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html), [docs.rs](https://docs.rs/pamoja-serial), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-serial) |
| TypeScript | [`@pamoja/serial`](https://www.npmjs.com/package/@pamoja/serial) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-serial) |
| Python | [`pamoja-serial`](https://pypi.org/project/pamoja-serial/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-serial) |
| C# | [`Pamoja.Serial`](https://www.nuget.org/packages/Pamoja.Serial) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-serial) |

## Documentation

- [`pamoja.serial` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html), every class and function in this module.
- [The Serial framing guide](https://pamoja.molex.cloud/docs/guides/serial.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
