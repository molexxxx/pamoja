# pamoja-serial

SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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
from pamoja.serial import SlipDecoder, cobs, slip

# SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a payload
# carrying either goes out as the two-byte pair RFC 1055 fixes for it.
payload = bytes([0x01, 0xC0, 0xDB, 0x02])
frame = slip.encode(payload)
assert frame == bytes([0x01, 0xDB, 0xDC, 0xDB, 0xDD, 0x02, 0xC0])
assert slip.decode(frame) == payload

# COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
# run led by its own length. This is the worked example from the COBS paper.
packet = bytes([0x11, 0x22, 0x00, 0x33])
framed = cobs.encode(packet)
assert framed == bytes([0x03, 0x11, 0x22, 0x02, 0x33, 0x00])
assert cobs.decode(framed) == packet

# A read from a port returns an arbitrary chunk rather than a packet. This one holds two
# frames with a truncated one between them, and the decoder drops only the bad frame.
decoder = SlipDecoder()
frames = decoder.feed(bytes([0x6F, 0x6B, 0xC0, 0xDB, 0xC0, 0x67, 0x6F, 0xC0]))
assert frames == [b"ok", b"go"]
assert decoder.discarded == 1
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-serial`](https://crates.io/crates/pamoja-serial) | [docs.rs](https://docs.rs/pamoja-serial), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html) |
| TypeScript | [`@pamoja/serial`](https://www.npmjs.com/package/@pamoja/serial) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html) |
| Python | [`pamoja-serial`](https://pypi.org/project/pamoja-serial/) | [`pamoja.serial`](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html) |
| C# | [`Pamoja.Serial`](https://www.nuget.org/packages/Pamoja.Serial) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.Serial.html) |

## Documentation

- [The Serial framing guide](https://pamoja.molex.cloud/docs/guides/serial.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
