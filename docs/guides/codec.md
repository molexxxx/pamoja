# Codecs

A payload has to become bytes before it leaves a device, and the shape of those
bytes is what the link charges for. pamoja puts the wire formats behind one
trait: CBOR for constrained devices and metered links, JSON for interop and
debugging, and a raw codec for payloads that are already bytes. Alongside them
sit two packers for a batch of readings, one for integers and one for `f32`,
both of which turn a slow-moving series into about a byte a sample.

## What the example does

It moves one reading from JSON to CBOR and back, then packs five integer samples
and four temperature readings, printing what each form costs on the wire.

Neither the CBOR nor the packed batches are written out by hand; every byte count
comes off the length of what the library returned. The scale of `100` handed to
the quantizer is the caller's choice of precision, and nothing in the packed
bytes records it, so a receiver has to be told the same scale to read the
readings back.

The exact bytes are not on this page. The CBOR encoding is pinned against RFC
8949 in the codec crate's own tests, and the packed batches are pinned in the
conformance vectors every binding checks itself against. A guide that restated
them would be one more copy of the same table.

It proves:

- A reading transcoded to CBOR comes back unchanged, and the CBOR is shorter
  than the JSON it came from.
- Five samples that rise, fall, then jump to 900 pack into fewer bytes than the
  forty the raw values cost, so a negative difference and a large one both stay
  small.
- The packed batch unpacks to the same five numbers, the jump included.
- Quantized readings decode to within `0.01`, the precision a scale of `100`
  sets, and that error is what the packing trades for the bytes.

## Rust

<!-- snippet: examples/tests/guides/codec.rs#example -->
From [`examples/tests/guides/codec.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/codec.rs):

```rust
use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};

// The same reading as JSON and as CBOR. Nothing is lost, and 21.5 rides as a
// half-precision float, the shortest form RFC 8949 allows for it.
let reading = br#"{"c":21.5,"ok":true}"#;
let cbor = json_to_cbor(reading).expect("a valid document");
println!("json      {} bytes", reading.len());
println!("cbor      {} bytes", cbor.len());

// A gateway that speaks JSON gets it back unchanged, so the compact form is a
// transport choice rather than a different data model.
let restored = cbor_to_json(&cbor).expect("a valid document");
println!("back to json, unchanged: {}", restored == reading);

// A batch of readings packs to a count, then the difference between each sample and
// the one before it. Successive readings differ by very little, so the differences
// cost about a byte each where the samples would cost eight.
let samples = [10i64, 11, 13, 12, 900];
let packed = encode_deltas(&samples);
let (count, bytes) = (samples.len(), packed.len());
let unpacked = decode_deltas(&packed).expect("a valid batch");
println!("batch     {count} samples in {bytes} bytes");
println!("unpacked  {unpacked:?}");

// Readings that arrive as floats pack the same way once a scale is chosen. Nothing in
// the bytes records that scale, so the sender and the receiver have to agree on it.
let quantizer = Quantizer::new(100.0);
let celsius = [20.0f32, 20.1, 20.2, 20.3];
let packed_celsius = quantizer.encode(&celsius);
let recovered = quantizer.decode(&packed_celsius).expect("a valid batch");
let (readings, packed_bytes) = (celsius.len(), packed_celsius.len());
println!("degrees   {readings} readings in {packed_bytes} bytes");
println!("recovered {recovered:?}");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/codec.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/codec.py#example -->
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
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs#example -->
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
<!-- end -->

## Reference

<!-- table: reference codec -->
- Rust: [`pamoja-codec`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html)
- TypeScript: [`@pamoja/codec`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html)
- Python: [`pamoja.codec`](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html)
- C#: [`Pamoja.Codec`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html)
<!-- end -->
