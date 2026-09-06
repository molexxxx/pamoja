# pamoja-codec

CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/codec.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-codec
```

```python
from pamoja import codec
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/codec.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/codec.py):

```python
import json

from pamoja.codec import Quantizer, from_cbor, pack_samples, to_cbor, unpack_samples

# The same reading as JSON and as CBOR. Nothing is lost, and 21.5 rides as a
# half-precision float, the shortest form RFC 8949 allows for it.
reading = {"c": 21.5, "ok": True}
as_json = json.dumps(reading, separators=(",", ":")).encode()
cbor = to_cbor(reading)
print(f"json      {len(as_json)} bytes")
print(f"cbor      {len(cbor)} bytes")

# A gateway that speaks JSON gets it back unchanged, so the compact form is a transport
# choice rather than a different data model.
restored = from_cbor(cbor)
print(f"back to json, unchanged: {restored == reading}")

# A batch of readings packs to a count, then the difference between each sample and the
# one before it. Successive readings differ by very little, so the differences cost about
# a byte each where the samples would cost eight.
samples = [10, 11, 13, 12, 900]
packed = pack_samples(samples)
print(f"batch     {len(samples)} samples in {len(packed)} bytes")
print(f"unpacked  {unpack_samples(packed)}")

# Readings that arrive as floats pack the same way once a scale is chosen. Nothing in the
# bytes records that scale, so the sender and the receiver have to agree on it.
quantizer = Quantizer(100)
celsius = [20.0, 20.1, 20.2, 20.3]
packed_celsius = quantizer.encode(celsius)
recovered = quantizer.decode(packed_celsius)
print(f"degrees   {len(celsius)} readings in {len(packed_celsius)} bytes")
print(f"recovered {[round(value, 1) for value in recovered]}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-codec`](https://crates.io/crates/pamoja-codec) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html), [docs.rs](https://docs.rs/pamoja-codec), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-codec) |
| TypeScript | [`@pamoja/codec`](https://www.npmjs.com/package/@pamoja/codec) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-codec) |
| Python | [`pamoja-codec`](https://pypi.org/project/pamoja-codec/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-codec) |
| C# | [`Pamoja.Codec`](https://www.nuget.org/packages/Pamoja.Codec) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-codec) |

## Documentation

- [`pamoja.codec` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html), every class and function in this module.
- [The Codecs guide](https://pamoja.molex.cloud/docs/guides/codec.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
