# pamoja-codec

CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-codec
```

```python
from pamoja import codec
```

This pulls in `pamoja-native`, the compiled engine, and nothing else. `pip install pamoja` is the whole framework in one package.

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-codec`](https://crates.io/crates/pamoja-codec) | [docs.rs](https://docs.rs/pamoja-codec), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html) |
| TypeScript | [`@pamoja/codec`](https://www.npmjs.com/package/@pamoja/codec) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html) |
| Python | [`pamoja-codec`](https://pypi.org/project/pamoja-codec/) | [`pamoja.codec`](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html) |
| C# | [`Pamoja.Codec`](https://www.nuget.org/packages/Pamoja.Codec) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.Codec.html) |

## Documentation

- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
