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
// The same reading as JSON and as CBOR. Nothing is lost, and 21.5 rides as a
// half-precision float, the shortest form RFC 8949 allows for it.
byte[] asJson = Encoding.UTF8.GetBytes("{\"c\":21.5,\"ok\":true}");
byte[] cbor = Codec.JsonToCbor(asJson);
Console.WriteLine($"json      {asJson.Length} bytes");
Console.WriteLine($"cbor      {cbor.Length} bytes");

// A gateway that speaks JSON gets it back unchanged, so the compact form is a
// transport choice rather than a different data model.
byte[] restored = Codec.CborToJson(cbor);
Console.WriteLine($"back to json, unchanged: {restored.SequenceEqual(asJson)}");

// A batch of readings packs to a count, then the difference between each sample
// and the one before it. Successive readings differ by very little, so the
// differences cost about a byte each where the samples would cost eight.
long[] samples = [10, 11, 13, 12, 900];
byte[] packed = Codec.PackSamples(samples);
Console.WriteLine($"batch     {samples.Length} samples in {packed.Length} bytes");
Console.WriteLine($"unpacked  {string.Join(", ", Codec.UnpackSamples(packed))}");

// Readings that arrive as floats pack the same way once a scale is chosen. Nothing
// in the bytes records that scale, so sender and receiver have to agree on it.
var quantizer = new Quantizer(100.0f);
float[] celsius = [20.0f, 20.1f, 20.2f, 20.3f];
byte[] packedCelsius = quantizer.Encode(celsius);
float[] recovered = quantizer.Decode(packedCelsius);
Console.WriteLine($"degrees   {celsius.Length} readings in {packedCelsius.Length} bytes");
Console.WriteLine($"recovered {string.Join(", ", recovered.Select(v => v.ToString("F1")))}");
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
