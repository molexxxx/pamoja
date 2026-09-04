# Codecs

A payload has to become bytes before it leaves a device, and the shape of those
bytes is what the link charges for. pamoja puts the wire formats behind one
trait: CBOR for constrained devices and metered links, JSON for interop and
debugging, and a raw codec for payloads that are already bytes. Alongside them
sit two packers for a batch of readings, one for integers and one for `f32`,
both of which turn a slow-moving series into about a byte a sample.

## What the example does

It encodes a small reading as CBOR and checks it against the exact bytes RFC
8949 fixes for that document, then reads it back. It packs a batch of integer
samples and a batch of float readings, checking both against the bytes the delta
encoding fixes, and confirms the readings return within the precision their
scale sets.

It proves:

- The CBOR encoding is byte-for-byte what the specification prescribes, down to
  carrying 21.5 as a half-precision float, so an encoder that is wrong but
  self-consistent still fails.
- A document survives the trip to CBOR and back with its content intact.
- Five samples travel as seven bytes rather than forty, the deltas zigzagged and
  written as LEB128 varints.
- Quantized readings decode to within the precision their scale sets, which is
  the loss the packing trades for the bytes.

## Rust

<!-- snippet: examples/tests/guides/codec.rs#example -->
From [`examples/tests/guides/codec.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/codec.rs):

```rust
use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};

// The same reading in CBOR instead of JSON, half the bytes. 21.5 rides as a
// half-precision float, the shortest form RFC 8949 allows for it, so these are the
// bytes the specification fixes rather than one encoder's dialect.
let reading = br#"{"c":21.5,"ok":true}"#;
let cbor = json_to_cbor(reading).expect("a valid document");
assert_eq!(
    cbor,
    [0xA2, 0x61, 0x63, 0xF9, 0x4D, 0x60, 0x62, 0x6F, 0x6B, 0xF5]
);
assert_eq!(cbor_to_json(&cbor).expect("a valid document"), reading);

// A batch packs to a count, then the difference between each sample and the one
// before it, zigzagged and written as a LEB128 varint. The four small steps cost a
// byte each; the jump to 900 zigzags to 1776 and costs the two bytes 0xF0 0x0D.
let samples = [10i64, 11, 13, 12, 900];
let packed = encode_deltas(&samples);
assert_eq!(packed, [0x05, 0x14, 0x02, 0x04, 0x01, 0xF0, 0x0D]);
assert_eq!(decode_deltas(&packed).expect("a valid batch"), samples);

// A quantizer packs f32 readings the same way, rounding at the scale first. Nothing
// in the bytes records the scale, so encode and decode have to agree on it.
let quantizer = Quantizer::new(100.0);
let readings = [20.0f32, 20.1, 20.2, 20.3];
let packed_readings = quantizer.encode(&readings);
assert_eq!(packed_readings, [0x04, 0xA0, 0x1F, 0x14, 0x14, 0x14]);
let restored = quantizer.decode(&packed_readings).expect("a valid batch");
for (got, want) in restored.iter().zip(&readings) {
    assert!((got - want).abs() <= 0.01);
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/codec.ts#example -->
From [`bindings/node/guides/codec.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/codec.ts):

```typescript
import assert from 'node:assert/strict'

import { Quantizer, fromCbor, packSamples, toCbor, unpackSamples } from '@pamoja/codec'

// The same reading in CBOR instead of JSON, half the bytes. 21.5 rides as a half-precision
// float, the shortest form RFC 8949 allows for it, so these are the bytes the specification
// fixes rather than one encoder's dialect.
const reading = { c: 21.5, ok: true }
const cbor = toCbor(reading)
assert.deepEqual([...cbor], [0xa2, 0x61, 0x63, 0xf9, 0x4d, 0x60, 0x62, 0x6f, 0x6b, 0xf5])
assert.deepEqual(fromCbor(cbor), reading)

// A batch packs to a count, then the difference between each sample and the one before it,
// zigzagged and written as a LEB128 varint. The four small steps cost a byte each; the jump
// to 900 zigzags to 1776 and costs the two bytes 0xf0 0x0d.
const samples = [10, 11, 13, 12, 900]
const packed = packSamples(samples)
assert.deepEqual([...packed], [0x05, 0x14, 0x02, 0x04, 0x01, 0xf0, 0x0d])
assert.deepEqual(unpackSamples(packed), samples)

// A quantizer packs float readings the same way, rounding at the scale first. Nothing in
// the bytes records the scale, so encode and decode have to agree on it.
const quantizer = new Quantizer(100)
const readings = [20.0, 20.1, 20.2, 20.3]
const packedReadings = quantizer.encode(readings)
assert.deepEqual([...packedReadings], [0x04, 0xa0, 0x1f, 0x14, 0x14, 0x14])
for (const [index, value] of quantizer.decode(packedReadings).entries()) {
  assert.ok(Math.abs(value - readings[index]) <= 0.01)
}
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/codec.py#example -->
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
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs#example -->
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
<!-- end -->

## Reference

<!-- table: reference codec -->
- Rust: [`pamoja-codec`](https://docs.rs/pamoja-codec) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html))
- TypeScript: [`@pamoja/codec`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html)
- Python: [`pamoja.codec`](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html)
- C#: [`Codec`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.Codec.html), [`Quantizer`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.Quantizer.html)
<!-- end -->
