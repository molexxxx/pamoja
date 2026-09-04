# Pamoja.Codec

CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/codec.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Codec
```

```csharp
using Pamoja.Codec;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs):

```csharp
// The same reading in CBOR instead of JSON, half the bytes. 21.5 rides as a
// half-precision float, the shortest form RFC 8949 allows for it, so these are
// the bytes the specification fixes rather than one encoder's dialect.
byte[] reading = Encoding.UTF8.GetBytes("{\"c\":21.5,\"ok\":true}");
byte[] cbor = Codec.JsonToCbor(reading);
Expect(
    cbor.SequenceEqual(
        new byte[] { 0xA2, 0x61, 0x63, 0xF9, 0x4D, 0x60, 0x62, 0x6F, 0x6B, 0xF5 }),
    "the document encodes to the bytes the specification fixes");
Expect(Codec.CborToJson(cbor).SequenceEqual(reading), "and reads back unchanged");

// A batch packs to a count, then the difference between each sample and the one
// before it, zigzagged and written as a LEB128 varint. The four small steps cost
// a byte each; the jump to 900 zigzags to 1776 and costs the bytes 0xF0 0x0D.
long[] samples = [10, 11, 13, 12, 900];
byte[] packed = Codec.PackSamples(samples);
Expect(
    packed.SequenceEqual(new byte[] { 0x05, 0x14, 0x02, 0x04, 0x01, 0xF0, 0x0D }),
    "five samples travel as seven bytes rather than forty");
Expect(Codec.UnpackSamples(packed).SequenceEqual(samples), "and decode exactly");

// A quantizer packs float readings the same way, rounding at the scale first.
// Nothing in the bytes records the scale, so encode and decode have to agree.
var quantizer = new Quantizer(100.0f);
float[] readings = [20.0f, 20.1f, 20.2f, 20.3f];
byte[] packedReadings = quantizer.Encode(readings);
Expect(
    packedReadings.SequenceEqual(new byte[] { 0x04, 0xA0, 0x1F, 0x14, 0x14, 0x14 }),
    "four readings travel as six bytes");
float[] restored = quantizer.Decode(packedReadings);
for (int i = 0; i < readings.Length; i++)
{
    Expect(Math.Abs(restored[i] - readings[i]) <= 0.01f, "to within the precision");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-codec`](https://crates.io/crates/pamoja-codec) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html), [docs.rs](https://docs.rs/pamoja-codec) |
| TypeScript | [`@pamoja/codec`](https://www.npmjs.com/package/@pamoja/codec) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html) |
| Python | [`pamoja-codec`](https://pypi.org/project/pamoja-codec/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html) |
| C# | [`Pamoja.Codec`](https://www.nuget.org/packages/Pamoja.Codec) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html) |

## Documentation

- [`Pamoja.Codec` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html), every type in this namespace.
- [The Codecs guide](https://pamoja.molex.cloud/docs/guides/codec.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
