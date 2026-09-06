# @pamoja/codec

CBOR, JSON, and raw codecs behind one trait, delta and varint batch packing, and an f32 quantizer for metered links. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/codec.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/codec
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/codec.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/codec.ts):

```typescript
import { Quantizer, fromCbor, packSamples, toCbor, unpackSamples } from '@pamoja/codec'

// The same reading as JSON and as CBOR. Nothing is lost, and 21.5 rides as a
// half-precision float, the shortest form RFC 8949 allows for it.
const reading = { c: 21.5, ok: true }
const asJson = Buffer.from(JSON.stringify(reading))
const cbor = toCbor(reading)
console.log(`json      ${asJson.length} bytes`)
console.log(`cbor      ${cbor.length} bytes`)

// A gateway that speaks JSON gets it back unchanged, so the compact form is a transport
// choice rather than a different data model.
const restored = fromCbor(cbor)
console.log(`back to json, unchanged: ${JSON.stringify(restored) === JSON.stringify(reading)}`)

// A batch of readings packs to a count, then the difference between each sample and the
// one before it. Successive readings differ by very little, so the differences cost about
// a byte each where the samples would cost eight.
const samples = [10, 11, 13, 12, 900]
const packed = packSamples(samples)
console.log(`batch     ${samples.length} samples in ${packed.length} bytes`)
console.log(`unpacked  ${unpackSamples(packed).join(', ')}`)

// Readings that arrive as floats pack the same way once a scale is chosen. Nothing in the
// bytes records that scale, so the sender and the receiver have to agree on it.
const quantizer = new Quantizer(100)
const celsius = [20.0, 20.1, 20.2, 20.3]
const packedCelsius = quantizer.encode(celsius)
const recovered = quantizer.decode(packedCelsius)
console.log(`degrees   ${celsius.length} readings in ${packedCelsius.length} bytes`)
console.log(`recovered ${[...recovered].map((v) => v.toFixed(1)).join(', ')}`)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-codec`](https://crates.io/crates/pamoja-codec) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html), [docs.rs](https://docs.rs/pamoja-codec), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-codec) |
| TypeScript | [`@pamoja/codec`](https://www.npmjs.com/package/@pamoja/codec) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-codec) |
| Python | [`pamoja-codec`](https://pypi.org/project/pamoja-codec/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-codec) |
| C# | [`Pamoja.Codec`](https://www.nuget.org/packages/Pamoja.Codec) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-codec) |

## Documentation

- [`@pamoja/codec` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html), every class, function, and type this package exports.
- [The Codecs guide](https://pamoja.molex.cloud/docs/guides/codec.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
