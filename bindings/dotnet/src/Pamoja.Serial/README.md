# Pamoja.Serial

SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/serial.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Serial
```

```csharp
using Pamoja.Serial;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs):

```csharp
// SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a
// payload carrying either goes out as the two-byte pair RFC 1055 fixes for it.
byte[] payload = [0x01, 0xC0, 0xDB, 0x02];
byte[] frame = Serial.SlipEncode(payload);
Expect(
    frame.SequenceEqual(new byte[] { 0x01, 0xDB, 0xDC, 0xDB, 0xDD, 0x02, 0xC0 }),
    "the frame is the escaping RFC 1055 fixes");
Expect(Serial.SlipDecode(frame).SequenceEqual(payload), "the payload comes back");

// COBS trades that escaping for one code byte per run of up to 254 non-zero
// bytes, each run led by its own length. This is the COBS paper's worked example.
byte[] packet = [0x11, 0x22, 0x00, 0x33];
byte[] framed = Serial.CobsEncode(packet);
Expect(
    framed.SequenceEqual(new byte[] { 0x03, 0x11, 0x22, 0x02, 0x33, 0x00 }),
    "the frame is the one the COBS paper works through");
Expect(Serial.CobsDecode(framed).SequenceEqual(packet), "the packet comes back");

// A serial read returns an arbitrary chunk rather than a packet. This one holds
// two frames with a truncated one between them, and only the bad frame is dropped.
using SlipDecoder decoder = new();
byte[][] frames = decoder.Feed([0x6F, 0x6B, 0xC0, 0xDB, 0xC0, 0x67, 0x6F, 0xC0]);
Expect(frames.Length == 2, "the frames either side of the bad one survive");
Expect(frames[0].SequenceEqual("ok"u8.ToArray()), "the first frame reassembles");
Expect(frames[1].SequenceEqual("go"u8.ToArray()), "the second frame reassembles");
Expect(decoder.Discarded == 1, "the truncated frame is counted, not raised");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-serial`](https://crates.io/crates/pamoja-serial) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html), [docs.rs](https://docs.rs/pamoja-serial) |
| TypeScript | [`@pamoja/serial`](https://www.npmjs.com/package/@pamoja/serial) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html) |
| Python | [`pamoja-serial`](https://pypi.org/project/pamoja-serial/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html) |
| C# | [`Pamoja.Serial`](https://www.nuget.org/packages/Pamoja.Serial) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html) |

## Documentation

- [`Pamoja.Serial` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html), every type in this namespace.
- [The Serial framing guide](https://pamoja.molex.cloud/docs/guides/serial.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
