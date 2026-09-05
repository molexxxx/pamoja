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
// A UART carries bytes, not packets, so a framing has to mark where one packet
// ends. SLIP reserves two byte values for that, and the package names both: the
// end byte closes a frame, the escape byte carries a value that would otherwise
// look like one.
byte[] payload = [.. "lvl="u8, Serial.SlipEnd, Serial.SlipEsc];
byte[] framed = Serial.SlipEncode(payload);
Console.WriteLine($"slip      {payload.Length} payload bytes framed as {framed.Length}");

// Decoding gives the payload back unchanged, reserved bytes and all.
byte[] restored = Serial.SlipDecode(framed);
Console.WriteLine($"slip      decoded back to {restored.Length} bytes");

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes,
// each run led by its own length, so a frame never grows by more than a byte per
// 254. Zero is the delimiter, and never appears inside a frame.
byte[] packet = [.. "lvl="u8, Serial.CobsDelimiter, .. "7"u8];
byte[] cobsFramed = Serial.CobsEncode(packet);
Console.WriteLine($"cobs      {packet.Length} payload bytes framed as {cobsFramed.Length}");

// A read from a port returns whatever arrived, which is rarely one whole frame.
// This chunk holds two good frames with a truncated one between them; the decoder
// hands over the good ones and discards only the bad frame.
using SlipDecoder decoder = new();
byte[] chunk =
[
    .. "ok"u8,
    Serial.SlipEnd,
    Serial.SlipEsc, // a frame that ends before its escape pair completes
    Serial.SlipEnd,
    .. "go"u8,
    Serial.SlipEnd,
];
IReadOnlyList<byte[]> frames = decoder.Feed(chunk);
foreach (byte[] frame in frames)
{
    Console.WriteLine($"received  {System.Text.Encoding.UTF8.GetString(frame)}");
}

Console.WriteLine($"discarded {decoder.Discarded} frame the stream mangled");
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
