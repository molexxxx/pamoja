# @pamoja/codec

CBOR, JSON, and raw codecs behind one trait, and batch packing for metered links: delta and varint for integers, and a quantizer for f32 readings. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/codec.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html)

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
import { LoraRegion, planFor } from '@pamoja/lora'

// The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate because it
// reaches farthest. An uplink there carries only a few bytes of payload.
const budget = planFor(LoraRegion.Us915).maxPayload(0)!.application
const fits = (bytes: number) => (bytes <= budget ? 'fits one uplink' : 'too big for one uplink')
console.log(`uplink    carries ${budget} bytes at the slowest US915 data rate`)

// One reading as the JSON a web service would take. CBOR carries the same document in
// fewer bytes, but every key name still rides along with every reading.
const reading = { depth_cm: 142.5, air_c: -6.5, battery_mv: 3712 }
const json = Buffer.from(JSON.stringify(reading))
const cbor = toCbor(reading)
console.log(`json      ${json.length} bytes, ${fits(json.length)}`)
console.log(`cbor      ${cbor.length} bytes, ${fits(cbor.length)}`)
console.log(`cbor      reads back as ${JSON.stringify(fromCbor(cbor))}`)

// A batch the gauge and the server agree on needs no key names. Six hourly depths, kept to
// the millimeter, pack to a count, the first depth, and five small steps.
const quantizer = new Quantizer(10)
const depths = [142.5, 143.8, 145.2, 146.0, 145.7, 145.5]
const depthBatch = quantizer.encode(depths)
const depthBytes = depthBatch.length
console.log(`depths    ${depths.length} readings in ${depthBytes} bytes, ${fits(depthBytes)}`)
const depthsBack = quantizer.decode(depthBatch).map((depth) => depth.toFixed(1))
console.log(`depths    read back as ${depthsBack.join(', ')}`)

// Battery millivolts are whole numbers already, so they pack with no scale, and a falling
// voltage packs as small as a rising one.
const battery = [3712, 3709, 3705, 3702, 3698, 3695]
const batteryBatch = packSamples(battery)
const batteryBytes = batteryBatch.length
console.log(`battery   ${battery.length} readings in ${batteryBytes} bytes, ${fits(batteryBytes)}`)
console.log(`battery   reads back as ${unpackSamples(batteryBatch).join(', ')}`)

// Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The quantizer
// refuses the batch rather than send the gap as a depth.
try {
  quantizer.encode([145.5, NaN])
} catch (error) {
  console.log(`depths    refused a batch with a missing depth: ${(error as Error).message}`)
}
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
