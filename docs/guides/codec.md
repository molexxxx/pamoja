# Codecs

A payload has to become bytes before it leaves a device, and the shape of those
bytes is what the link charges for. pamoja puts the wire formats behind one
trait: CBOR for constrained devices and metered links, JSON for interop and
debugging, and a raw codec for payloads that are already bytes. Alongside them
sit two packers for a batch of readings, one for whole numbers and one for
`f32` readings, both of which turn a slow-moving series into about a byte a
sample.

The two kinds make different trades. A document keeps its key names, so it
describes itself and any receiver can read it. A batch drops them, so it is a
fraction of the size, and both ends have to agree on what its numbers mean and
at what scale. The smaller the link, the more that agreement is worth.

The packers need no operating system. With default features off, the trait,
the raw codec, and both packers build for a microcontroller, so a node can pack
its own batch before it transmits.

## What the example does

It is a snow gauge on a mountain ridge, reporting over LoRaWAN in the US915
plan at the slowest data rate, the one that reaches farthest. It asks the plan
how much one uplink carries there, then tries each way the codecs offer to fit
its readings in: one reading as a JSON document, the same document as CBOR,
and six hours of readings as two packed batches, the snow depths through a
quantizer and the battery voltages through the integer packer. Each batch goes
up in an uplink of its own, and the LoRaWAN port number tells the server which
is which. Last, the depth sensor hears no echo through falling snow, its driver
reports a depth that is not a number, and the quantizer refuses the batch.

The bytes themselves are not on this page. The CBOR encoding is pinned against
RFC 8949 in the codec crate's own tests, the varint and zigzag layers against
the encodings Protocol Buffers documents, and the packed batches in the
conformance vectors every binding checks itself against.

It proves:

- An uplink at the slowest US915 data rate carries 11 bytes of application
  payload, the figure the channel plan in `pamoja-lora` returns.
- One reading costs 49 bytes as JSON and 36 as CBOR, so neither fits. CBOR
  spends less on each value, and still spends a byte and more on every key
  name.
- The CBOR reads back as the same document, with its keys in sorted order.
- Six depths at a scale of 10 pack into 8 bytes, a count, a first depth, and
  five one-byte steps, and read back to the millimeter.
- Six battery readings pack into 8 bytes as well, falling ones as small as
  rising ones, and read back exactly.
- A batch with a missing depth is refused, and the error names the reading:
  `codec error: reading 1 is NaN, which cannot be quantized`.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example codec" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example codec</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- codec" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- codec</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/codec.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/codec.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- codec" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- codec</code></div>
</div>
<!-- end -->

## Rust

In Rust, the `Codec<T>` trait is the seam every wire format sits behind: `encode(&value)` returns
the bytes and `decode(&bytes)` the value. `CborCodec` and `JsonCodec` implement it for any type
that derives serde's `Serialize` and `Deserialize`, and `BytesCodec` passes a `Vec<u8>` through
unchanged. `json_to_cbor(&json)` and `cbor_to_json(&cbor)` move a whole document between the two
formats without a Rust type for it, which is the call the other languages reach CBOR through.
`encode_deltas(&samples)` packs `i64` samples and `decode_deltas(&bytes)` unpacks them.
`Quantizer::new(scale)` packs `f32` readings with `encode(&readings)` and unpacks them with
`decode(&bytes)`; it takes any scale, and `encode` and `decode` refuse one that is not a
positive, finite number. Every fallible call returns a `Result` whose error is `Error::Codec`,
with a message that says what was wrong. On a microcontroller, depend on `pamoja-codec` with
`default-features = false`: the trait, `BytesCodec`, and the packers stay, and the CBOR and JSON
codecs, which use the standard library, go.

<!-- snippet: examples/guides/codec.rs#example -->
From [`examples/guides/codec.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/codec.rs):

```rust
use pamoja_codec::{cbor_to_json, decode_deltas, encode_deltas, json_to_cbor, Quantizer};
use pamoja_lora::region::Region;

// The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate because
// it reaches farthest. An uplink there carries only a few bytes of payload.
let budget = Region::Us915
    .plan()
    .max_payload(0, false)
    .expect("the slowest data rate carries a payload")
    .application as usize;
let fits = |bytes: usize| {
    if bytes <= budget {
        "fits one uplink"
    } else {
        "too big for one uplink"
    }
};
println!("uplink    carries {budget} bytes at the slowest US915 data rate");

// One reading as the JSON a web service would take. CBOR carries the same document
// in fewer bytes, but every key name still rides along with every reading.
let reading = br#"{"depth_cm":142.5,"air_c":-6.5,"battery_mv":3712}"#;
let cbor = json_to_cbor(reading)?;
println!("json      {} bytes, {}", reading.len(), fits(reading.len()));
println!("cbor      {} bytes, {}", cbor.len(), fits(cbor.len()));
let restored = cbor_to_json(&cbor)?;
println!("cbor      reads back as {}", String::from_utf8(restored)?);

// A batch the gauge and the server agree on needs no key names. Six hourly depths,
// kept to the millimeter, pack to a count, the first depth, and five small steps.
let quantizer = Quantizer::new(10.0);
let depths = [142.5, 143.8, 145.2, 146.0, 145.7, 145.5];
let depth_batch = quantizer.encode(&depths)?;
let (count, size) = (depths.len(), depth_batch.len());
println!("depths    {count} readings in {size} bytes, {}", fits(size));
let depths_back: Vec<String> = quantizer
    .decode(&depth_batch)?
    .iter()
    .map(|depth| format!("{depth:.1}"))
    .collect();
println!("depths    read back as {}", depths_back.join(", "));

// Battery millivolts are whole numbers already, so they pack with no scale, and a
// falling voltage packs as small as a rising one.
let battery = [3712, 3709, 3705, 3702, 3698, 3695];
let battery_batch = encode_deltas(&battery);
let (count, size) = (battery.len(), battery_batch.len());
println!("battery   {count} readings in {size} bytes, {}", fits(size));
let battery_back: Vec<String> = decode_deltas(&battery_batch)?
    .iter()
    .map(i64::to_string)
    .collect();
println!("battery   reads back as {}", battery_back.join(", "));

// Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The
// quantizer refuses the batch rather than send the gap as a depth.
let refused = quantizer
    .encode(&[145.5, f32::NAN])
    .expect_err("a missing depth");
println!("depths    refused a batch with a missing depth: {refused}");
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/codec` takes and returns ordinary values. `toCbor(value)` encodes anything
`JSON.stringify` accepts, or the bytes of a JSON document, and `fromCbor(bytes)` returns the
value. `packSamples(numbers)` and `unpackSamples(bytes)` carry whole numbers, which a JavaScript
number holds exactly up to `Number.MAX_SAFE_INTEGER` either side of zero; a fraction, or a number
past that bound, throws rather than being rounded. `new Quantizer(scale)` throws on a scale that
is not a positive, finite number, and its `encode(readings)` returns a `Buffer` and throws on a
reading it cannot carry. A reading is narrowed to a 32-bit float on its way in, as it is in every
language. Every call is synchronous, and a failure throws an `Error`.

<!-- snippet: bindings/node/guides/codec.ts#example -->
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
<!-- end -->

## Python

In Python, `pamoja.codec` works the same way. `to_cbor(value)` encodes anything `json.dumps`
accepts, or the bytes of a JSON document, and `from_cbor(data)` returns the value.
`pack_samples(ints)` and `unpack_samples(data)` carry `int` samples that fit in 64 bits.
`Quantizer(scale)` raises `ValueError` for a scale that is not a positive, finite number, and
its `encode(readings)` returns `bytes`. A codec failure raises `PamojaError`. A value that never
reaches the codec raises what Python raises for it: `TypeError` for a sample that is not an
`int`, `OverflowError` for one past 64 bits, and `ValueError` for a document holding a NaN.
Every call is synchronous.

<!-- snippet: bindings/python/guides/codec.py#example -->
From [`bindings/python/guides/codec.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/codec.py):

```python
import json
import math

from pamoja.codec import Quantizer, from_cbor, pack_samples, to_cbor, unpack_samples
from pamoja.core import PamojaError
from pamoja.lora import plan_for

# The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate because it
# reaches farthest. An uplink there carries only a few bytes of payload.
budget = plan_for("US915").max_payload(0).application


def fits(size: int) -> str:
    return "fits one uplink" if size <= budget else "too big for one uplink"


print(f"uplink    carries {budget} bytes at the slowest US915 data rate")

# One reading as the JSON a web service would take. CBOR carries the same document in
# fewer bytes, but every key name still rides along with every reading.
reading = {"depth_cm": 142.5, "air_c": -6.5, "battery_mv": 3712}
as_json = json.dumps(reading, separators=(",", ":")).encode()
cbor = to_cbor(reading)
print(f"json      {len(as_json)} bytes, {fits(len(as_json))}")
print(f"cbor      {len(cbor)} bytes, {fits(len(cbor))}")
print(f"cbor      reads back as {json.dumps(from_cbor(cbor), separators=(',', ':'))}")

# A batch the gauge and the server agree on needs no key names. Six hourly depths, kept to
# the millimeter, pack to a count, the first depth, and five small steps.
quantizer = Quantizer(10)
depths = [142.5, 143.8, 145.2, 146.0, 145.7, 145.5]
depth_batch = quantizer.encode(depths)
depth_bytes = len(depth_batch)
print(f"depths    {len(depths)} readings in {depth_bytes} bytes, {fits(depth_bytes)}")
depths_back = [f"{depth:.1f}" for depth in quantizer.decode(depth_batch)]
print(f"depths    read back as {', '.join(depths_back)}")

# Battery millivolts are whole numbers already, so they pack with no scale, and a falling
# voltage packs as small as a rising one.
battery = [3712, 3709, 3705, 3702, 3698, 3695]
battery_batch = pack_samples(battery)
battery_bytes = len(battery_batch)
print(f"battery   {len(battery)} readings in {battery_bytes} bytes, {fits(battery_bytes)}")
print(f"battery   reads back as {', '.join(map(str, unpack_samples(battery_batch)))}")

# Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The quantizer
# refuses the batch rather than send the gap as a depth.
try:
    quantizer.encode([145.5, math.nan])
except PamojaError as error:
    print(f"depths    refused a batch with a missing depth: {error}")
```
<!-- end -->

## C#

In C#, the static `Codec` class in `Pamoja.Codec` works on bytes. `JsonToCbor(json)` and
`CborToJson(cbor)` take and return UTF-8 JSON, `PackSamples(samples)` and `UnpackSamples(bytes)`
carry `long` samples, and `new Quantizer(scale)` packs `float` readings with `Encode(readings)`
and unpacks them with `Decode(bytes)`. Every input is a `ReadOnlySpan`, so an array or a slice of
one works. A bad scale throws `ArgumentOutOfRangeException` from the constructor, and a codec
failure throws `PamojaException`. Every call is synchronous.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CodecGuide.cs):

```csharp
// The gauge reports over LoRaWAN in the US915 plan, at the slowest data rate
// because it reaches farthest. An uplink there carries only a few bytes of payload.
using LoraChannelPlan plan = LoraChannelPlan.ForRegion(LoraRegion.Us915);
int budget = plan.MaxPayload(0)!.Value.Application;
string Fits(int bytes) => bytes <= budget ? "fits one uplink" : "too big for one uplink";
Console.WriteLine($"uplink    carries {budget} bytes at the slowest US915 data rate");

// One reading as the JSON a web service would take. CBOR carries the same
// document in fewer bytes, but every key name still rides along with every
// reading.
byte[] json = Encoding.UTF8.GetBytes("""{"depth_cm":142.5,"air_c":-6.5,"battery_mv":3712}""");
byte[] cbor = Codec.JsonToCbor(json);
Console.WriteLine($"json      {json.Length} bytes, {Fits(json.Length)}");
Console.WriteLine($"cbor      {cbor.Length} bytes, {Fits(cbor.Length)}");
string restored = Encoding.UTF8.GetString(Codec.CborToJson(cbor));
Console.WriteLine($"cbor      reads back as {restored}");

// A batch the gauge and the server agree on needs no key names. Six hourly
// depths, kept to the millimeter, pack to a count, the first depth, and five
// small steps.
var quantizer = new Quantizer(10.0f);
float[] depths = [142.5f, 143.8f, 145.2f, 146.0f, 145.7f, 145.5f];
byte[] depthBatch = quantizer.Encode(depths);
int depthBytes = depthBatch.Length;
Console.WriteLine($"depths    {depths.Length} readings in {depthBytes} bytes, {Fits(depthBytes)}");
IEnumerable<string> depthsBack = quantizer.Decode(depthBatch)
    .Select(depth => depth.ToString("F1", CultureInfo.InvariantCulture));
Console.WriteLine($"depths    read back as {string.Join(", ", depthsBack)}");

// Battery millivolts are whole numbers already, so they pack with no scale, and
// a falling voltage packs as small as a rising one.
long[] battery = [3712, 3709, 3705, 3702, 3698, 3695];
byte[] batteryBatch = Codec.PackSamples(battery);
int batteryBytes = batteryBatch.Length;
Console.WriteLine($"battery   {battery.Length} readings in {batteryBytes} bytes, {Fits(batteryBytes)}");
Console.WriteLine($"battery   reads back as {string.Join(", ", Codec.UnpackSamples(batteryBatch))}");

// Heavy snowfall can swallow the sensor's echo, leaving no depth at all. The
// quantizer refuses the batch rather than send the gap as a depth.
try
{
    quantizer.Encode([145.5f, float.NaN]);
}
catch (PamojaException error)
{
    Console.WriteLine($"depths    refused a batch with a missing depth: {error.Message}");
}
```
<!-- end -->

## Values at a glance

**The calls:**

| What | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| a document to CBOR | `json_to_cbor(&json)` | `toCbor(value)` | `to_cbor(value)` | `Codec.JsonToCbor(json)` |
| CBOR back to a document | `cbor_to_json(&cbor)` | `fromCbor(bytes)` | `from_cbor(data)` | `Codec.CborToJson(cbor)` |
| pack whole numbers | `encode_deltas(&samples)` | `packSamples(samples)` | `pack_samples(samples)` | `Codec.PackSamples(samples)` |
| unpack them | `decode_deltas(&bytes)` | `unpackSamples(bytes)` | `unpack_samples(data)` | `Codec.UnpackSamples(bytes)` |
| a quantizer | `Quantizer::new(scale)` | `new Quantizer(scale)` | `Quantizer(scale)` | `new Quantizer(scale)` |
| pack readings | `encode(&readings)` | `encode(readings)` | `encode(readings)` | `Encode(readings)` |
| unpack them | `decode(&bytes)` | `decode(bytes)` | `decode(data)` | `Decode(bytes)` |

**What each language hands in:**

| Value | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| a sample | `i64` | a whole `number` | an `int` | `long` |
| a reading | `f32` | `number`, narrowed | `float`, narrowed | `float` |
| a scale | `f32` | `number` | `float` | `float` |
| bytes in | `&[u8]` | `Uint8Array` | `bytes` | `ReadOnlySpan<byte>` |
| bytes out | `Vec<u8>` | `Buffer` | `bytes` | `byte[]` |

**The three codecs**, in Rust:

| Codec | Carries | On the wire | Reach for it when |
| --- | --- | --- | --- |
| `CborCodec` | any serde type | CBOR, RFC 8949 | the link is small or metered |
| `JsonCodec` | any serde type | JSON | a person or a web service reads it |
| `BytesCodec` | `Vec<u8>` | the bytes as they are | the payload has a format of its own |

**What a batch is made of.** Both packers write the same layout, and the quantizer first
multiplies each reading by its scale and rounds it to a whole number:

| Part | What it holds | Written as |
| --- | --- | --- |
| count | how many samples follow | an unsigned LEB128 varint |
| first sample | the sample itself, a step from zero | zigzag, then LEB128 |
| each later sample | the step from the one before | zigzag, then LEB128 |

**What a step costs.** Zigzag folds a negative step into a small number, so a fall costs what a
rise does:

| Step, in whole units after scaling | Bytes |
| --- | --- |
| -64 to 63 | 1 |
| -8,192 to 8,191 | 2 |
| -1,048,576 to 1,048,575 | 3 |
| -134,217,728 to 134,217,727 | 4 |
| larger, up to the full 64 bits | 5 to 10 |

The count costs one byte up to 127 samples. The first sample is a step from zero, so it is
usually the costliest: the gauge's first depth, 142.5 at a scale of 10, is 1,425 units and takes
two bytes.

**Choosing a scale:**

| Scale | Keeps | Reads back to within | One byte carries a step of up to |
| --- | --- | --- | --- |
| `1` | whole units | 0.5 | 63 |
| `10` | tenths | 0.05 | 6.3 |
| `100` | hundredths | 0.005 | 0.63 |
| `1000` | thousandths | 0.0005 | 0.063 |

A reading times its scale has to stay under about 9.2e18, the range of the whole number it is
rounded to, and an `f32` keeps about seven significant digits whatever the scale.

**What CBOR spends on a value:**

| Value | Bytes |
| --- | --- |
| a whole number from -24 to 23 | 1 |
| a whole number from -256 to 255 | 2 |
| a whole number from -65,536 to 65,535 | 3 |
| a number a half-precision float holds exactly, such as 21.5 or -6.5 | 3 |
| a number a single-precision float holds exactly, such as 100000.5 | 5 |
| any other number, such as 0.1 | 9 |
| `true`, `false`, or `null` | 1 |
| a string of up to 23 bytes | 1, plus the string |
| an object or an array of up to 23 entries | 1, plus the entries |

An object key is a string, so every key costs its length and a byte, in every reading that
carries it.

## When it goes wrong

What the codecs say:

| What happened | The message | What to check |
| --- | --- | --- |
| a reading that is not a number, or is infinite | `codec error: reading 1 is NaN, which cannot be quantized` | the sensor; leave the reading out, and send where the gap was if it matters |
| a reading too large for its scale | `codec error: reading 0 is 100000000000000000, too large for a scale of 100` | a smaller scale |
| a scale that is zero, negative, or not a number | `a quantizer's scale must be a positive, finite number, not 0` | the scale; Rust reports it from `encode` and `decode`, the other languages from the constructor |
| a batch cut short | `codec error: the batch ends part-way through a value` | that the payload arrived whole |
| bytes after the last sample, such as two batches run together | `codec error: 2 bytes follow the batch's last sample` | one batch to a payload |
| a value that runs past 64 bits, which no packer writes | `codec error: a value in the batch does not fit in 64 bits` | that the payload is a batch at all |
| a JSON document that does not parse | `codec error:` and the parser's reason, such as `expected ident at line 1 column 2` | the document |
| a CBOR document cut short | `codec error: the CBOR ends part-way through a value` | that the payload arrived whole |
| CBOR with a key JSON cannot hold, such as a number | `codec error: the CBOR has no JSON form:` and the reason | read it as CBOR, not as a document |
| TypeScript: a sample that is not a whole number | `sample 1 is 10.5, which is not a whole number` | round it on purpose, or use a quantizer |
| TypeScript: a sample past what a number holds exactly | `sample 0 is 9007199254740992, past the largest whole number a JavaScript number holds exactly` | pack smaller units, or offsets from a start |

The mistakes that cost an afternoon:

- **The server reads the wrong numbers, and nothing says so.** A batch does not record its
  scale. A server that decodes at 100 what the gauge packed at 10 reads every depth as a tenth of
  what it was. Keep the scale in one place both ends share: a schema, a port number, or a topic.
- **A coordinate loses its last digits.** An `f32` keeps about seven significant digits, so a
  longitude such as -122.4194155 becomes -122.4194183, wrong in its sixth decimal place, before
  any scale applies. Send coordinates as whole numbers, degrees times ten million as GPS
  receivers and MAVLink do, and pack them with the integer packer.
- **A round trip does not match byte for byte.** A document comes back from CBOR with its keys in
  sorted order, the same in content and different in bytes. Compare parsed values, not bytes.
- **A NaN goes into a document.** JSON has no NaN. TypeScript's `JSON.stringify` writes `null` in
  its place without a word, and Python raises `ValueError`. Check a reading before it goes into a
  document, as the quantizer does for a batch.
- **Nanosecond timestamps in TypeScript.** A JavaScript number holds whole numbers exactly only
  up to 2^53, and a nanosecond Unix timestamp is past that. `packSamples` refuses one rather than
  round it. Pack milliseconds, or the offsets from a start time.
- **Two batches in one payload.** The decoder refuses bytes after a batch's last sample, so
  batches run together do not decode. Send each batch in its own uplink or on its own port, as
  the gauge does.

## Where next

<!-- table: next codec -->
- [MQTT](mqtt.md): An MQTT client with the topic and wildcard rules, acknowledged delivery, retained messages, a last will, and TLS, as the core transport.
- [Store and forward](sync.md): Offline-first queues in memory or on disk, bounded or not, the on-disk one surviving power loss, and the drain that forwards them in order when a link returns.
- [LoRaWAN](lorawan.md): LoRaWAN 1.0.x MAC framing, AES-CMAC and AES encryption, and both halves of the OTAA join.
<!-- end -->

## Reference

<!-- table: reference codec -->
- Rust: [`pamoja-codec`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_codec/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-codec)
- TypeScript: [`@pamoja/codec`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_codec.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-codec)
- Python: [`pamoja.codec`](https://pamoja.molex.cloud/docs/reference/python/pamoja/codec.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-codec)
- C#: [`Pamoja.Codec`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Codec.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-codec)
<!-- end -->
