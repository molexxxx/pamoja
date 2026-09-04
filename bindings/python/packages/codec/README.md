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
from pamoja.codec import Quantizer, from_cbor, pack_samples, to_cbor, unpack_samples

# The same reading in CBOR instead of JSON, half the bytes. 21.5 rides as a
# half-precision float, the shortest form RFC 8949 allows for it, so these are the
# bytes the specification fixes rather than one encoder's dialect.
reading = {"c": 21.5, "ok": True}
cbor = to_cbor(reading)
assert cbor == bytes([0xA2, 0x61, 0x63, 0xF9, 0x4D, 0x60, 0x62, 0x6F, 0x6B, 0xF5])
assert from_cbor(cbor) == reading

# A batch of samples packs to a count, then the difference between each value and the
# one before it, zigzagged and written as a LEB128 varint. The four small steps cost a
# byte each; the jump to 900 zigzags to 1776 and costs the two bytes 0xF0 0x0D.
samples = [10, 11, 13, 12, 900]
packed = pack_samples(samples)
assert packed == bytes([0x05, 0x14, 0x02, 0x04, 0x01, 0xF0, 0x0D])
assert unpack_samples(packed) == samples

# The quantizer packs float readings the same way, rounding at the scale first. Nothing
# in the bytes records the scale, so encode and decode have to agree on it.
quantizer = Quantizer(100)
readings = [20.0, 20.1, 20.2, 20.3]
packed_readings = quantizer.encode(readings)
assert packed_readings == bytes([0x04, 0xA0, 0x1F, 0x14, 0x14, 0x14])
restored = quantizer.decode(packed_readings)
assert all(abs(got - want) <= 0.01 for got, want in zip(restored, readings))
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-codec`](https://crates.io/crates/pamoja-codec) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html), [docs.rs](https://docs.rs/pamoja-codec) |
| TypeScript | [`@pamoja/codec`](https://www.npmjs.com/package/@pamoja/codec) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html) |
| Python | [`pamoja-codec`](https://pypi.org/project/pamoja-codec/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html) |
| C# | [`Pamoja.Codec`](https://www.nuget.org/packages/Pamoja.Codec) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html) |

## Documentation

- [`pamoja.codec` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html), every class and function in this module.
- [The Codecs guide](https://pamoja.molex.cloud/docs/guides/codec.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
