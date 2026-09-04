# Pamoja.Codec

CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
dotnet add package Pamoja.Codec
```

```csharp
using Pamoja.Codec;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core`, and nothing else. `dotnet add package Pamoja` is the whole framework in one package.

## Documentation

- [The reference for `Pamoja.Codec`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html), generated from its source.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
