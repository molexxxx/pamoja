# Serial framing

A UART, an RS232 link, or a USB-serial bridge hands an application a stream of
bytes with no notion of where one message ends and the next begins. Byte stuffing
supplies that boundary: reserve a byte value as the delimiter and encode the
payload so the value can never occur inside it. pamoja implements the two that
field hardware speaks, SLIP and COBS, as pure byte work with no allocation and no
serial port of its own, so the same code runs on a gateway and on a
microcontroller hanging off the bus.

## What the example does

It frames a payload with SLIP, frames a second payload with COBS, and feeds a
decoder one chunk holding two frames with a truncated one between them.

The reserved bytes are named rather than typed: the package exports the SLIP end
and escape bytes and the COBS delimiter, so a payload built to contain them says
so. The frames are checked against the ones RFC 1055 and the COBS paper fix.

It proves:

- A payload carrying the delimiter or the escape byte is stuffed rather than
  mistaken for a frame boundary. The exact stuffing RFC 1055 fixes, and the COBS
  paper's run-length codes, are pinned in the serial crate's own tests.
- Both framings cost bytes, and both give the original payload back.
- The streaming decoder splits a chunk into whole frames, drops the truncated one
  rather than the chunk around it, and counts what it dropped.

## Rust

<!-- snippet: examples/tests/guides/serial.rs#example -->
From [`examples/tests/guides/serial.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/serial.rs):

```rust
use pamoja_serial::{cobs, slip};

// A UART carries bytes, not packets, so a framing has to mark where one packet ends.
// SLIP reserves two byte values for that, and the crate names both: END closes a
// frame, ESC carries a byte that would otherwise look like one. The hard case is a
// payload that already contains them, so this one does.
let mut payload = b"lvl=".to_vec();
payload.push(slip::END);
payload.push(slip::ESC);
let mut framed = [0u8; slip::max_encoded_len(8)];
let n = slip::encode(&payload, &mut framed).expect("room for the frame");
println!("slip      {} payload bytes framed as {n}", payload.len());

// Decoding gives the payload back unchanged, reserved bytes and all.
let mut restored = [0u8; 8];
let m = slip::decode(&framed[..n], &mut restored).expect("a well-formed frame");
println!("slip      decoded back to {m} bytes");

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes,
// each run led by its own length, so a frame never grows by more than a byte per 254.
// Zero is the delimiter, and COBS is what takes it out of the data.
let mut packet = b"lvl=".to_vec();
packet.push(cobs::DELIMITER);
packet.extend_from_slice(b"7");
let mut cobs_framed = [0u8; cobs::max_encoded_len(8)];
let framed_len = cobs::encode(&packet, &mut cobs_framed).expect("room for the frame");
let packet_len = packet.len();
println!("cobs      {packet_len} payload bytes framed as {framed_len}");

// A serial read returns whatever arrived, which is rarely one whole frame. This chunk
// holds two good frames with a truncated one between them; the decoder hands over the
// good ones and discards only the bad frame.
let mut chunk = Vec::new();
chunk.extend_from_slice(b"ok");
chunk.push(slip::END);
chunk.push(slip::ESC); // a frame that ends before its escape pair completes
chunk.push(slip::END);
chunk.extend_from_slice(b"go");
chunk.push(slip::END);

let mut decoder: slip::SlipDecoder<16> = slip::SlipDecoder::new();
let mut frames: Vec<Vec<u8>> = Vec::new();
let mut discarded = 0;
for &byte in &chunk {
    match decoder.push(byte) {
        Ok(Some(complete)) => frames.push(complete.to_vec()),
        Ok(None) => {}
        Err(_) => discarded += 1,
    }
}
for frame in &frames {
    println!("received  {}", String::from_utf8_lossy(frame));
}
println!("discarded {discarded} frame the stream mangled");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/serial.ts#example -->
From [`bindings/node/guides/serial.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/serial.ts):

```typescript
import {
  COBS_DELIMITER_BYTE,
  SLIP_END_BYTE,
  SLIP_ESC_BYTE,
  SlipDecoder,
  cobs,
  slip,
} from '@pamoja/serial'

// A UART carries bytes, not packets, so a framing has to mark where one packet ends.
// SLIP reserves two byte values for that, and the package names both: the end byte closes
// a frame, the escape byte carries a value that would otherwise look like one.
const payload = Buffer.concat([Buffer.from('lvl='), Buffer.from([SLIP_END_BYTE, SLIP_ESC_BYTE])])
const framed = slip.encode(payload)
console.log(`slip      ${payload.length} payload bytes framed as ${framed.length}`)

// Decoding gives the payload back unchanged, reserved bytes and all.
const restored = slip.decode(framed)
console.log(`slip      decoded back to ${restored.length} bytes`)

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
// run led by its own length, so a frame never grows by more than a byte per 254. Zero is
// the delimiter, and never appears inside a frame.
const packet = Buffer.concat([Buffer.from('lvl='), Buffer.from([COBS_DELIMITER_BYTE]), Buffer.from('7')])
const cobsFramed = cobs.encode(packet)
console.log(`cobs      ${packet.length} payload bytes framed as ${cobsFramed.length}`)

// A read from a port returns whatever arrived, which is rarely one whole frame. This
// chunk holds two good frames with a truncated one between them; the decoder hands over
// the good ones and discards only the bad frame.
const decoder = new SlipDecoder()
const chunk = Buffer.concat([
  Buffer.from('ok'),
  Buffer.from([SLIP_END_BYTE]),
  Buffer.from([SLIP_ESC_BYTE]), // a frame that ends before its escape pair completes
  Buffer.from([SLIP_END_BYTE]),
  Buffer.from('go'),
  Buffer.from([SLIP_END_BYTE]),
])
const frames = decoder.feed(chunk)
for (const frame of frames) {
  console.log(`received  ${frame.toString()}`)
}
console.log(`discarded ${decoder.discarded} frame the stream mangled`)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/serial.py#example -->
From [`bindings/python/guides/serial.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/serial.py):

```python
from pamoja.serial import COBS_DELIMITER, SLIP_END, SLIP_ESC, SlipDecoder, cobs, slip

# A UART carries bytes, not packets, so a framing has to mark where one packet ends. SLIP
# reserves two byte values for that, and the package names both: the end byte closes a
# frame, the escape byte carries a value that would otherwise look like one.
payload = b"lvl=" + bytes([SLIP_END, SLIP_ESC])
framed = slip.encode(payload)
print(f"slip      {len(payload)} payload bytes framed as {len(framed)}")

# Decoding gives the payload back unchanged, reserved bytes and all.
restored = slip.decode(framed)
print(f"slip      decoded back to {len(restored)} bytes")

# COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
# run led by its own length, so a frame never grows by more than a byte per 254. Zero is
# the delimiter, and never appears inside a frame.
packet = b"lvl=" + bytes([COBS_DELIMITER]) + b"7"
cobs_framed = cobs.encode(packet)
print(f"cobs      {len(packet)} payload bytes framed as {len(cobs_framed)}")

# A read from a port returns whatever arrived, which is rarely one whole frame. This chunk
# holds two good frames with a truncated one between them; the decoder hands over the good
# ones and discards only the bad frame.
decoder = SlipDecoder()
chunk = (
    b"ok"
    + bytes([SLIP_END])
    + bytes([SLIP_ESC])  # a frame that ends before its escape pair completes
    + bytes([SLIP_END])
    + b"go"
    + bytes([SLIP_END])
)
frames = decoder.feed(chunk)
for frame in frames:
    print(f"received  {frame.decode()}")
print(f"discarded {decoder.discarded} frame the stream mangled")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs):

```csharp
// A UART carries bytes, not packets, so a framing has to mark where one packet
// ends. SLIP reserves two byte values for that, and the package names both: the
// end byte closes a frame, the escape byte carries a value that would otherwise
// look like one.
byte[] payload = [.. "lvl="u8, Serial.SlipEnd, Serial.SlipEsc];
byte[] framed = Serial.SlipEncode(payload);
Console.WriteLine($"slip      {payload.Length} payload bytes framed as {framed.Length}");

// Decoding gives the payload back unchanged, reserved bytes and all.
byte[] restored = Serial.SlipDecode(framed);
Console.WriteLine($"slip      decoded back to {restored.Length} bytes");

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes,
// each run led by its own length, so a frame never grows by more than a byte per
// 254. Zero is the delimiter, and never appears inside a frame.
byte[] packet = [.. "lvl="u8, Serial.CobsDelimiter, .. "7"u8];
byte[] cobsFramed = Serial.CobsEncode(packet);
Console.WriteLine($"cobs      {packet.Length} payload bytes framed as {cobsFramed.Length}");

// A read from a port returns whatever arrived, which is rarely one whole frame.
// This chunk holds two good frames with a truncated one between them; the decoder
// hands over the good ones and discards only the bad frame.
using SlipDecoder decoder = new();
byte[] chunk =
[
    .. "ok"u8,
    Serial.SlipEnd,
    Serial.SlipEsc, // a frame that ends before its escape pair completes
    Serial.SlipEnd,
    .. "go"u8,
    Serial.SlipEnd,
];
IReadOnlyList<byte[]> frames = decoder.Feed(chunk);
foreach (byte[] frame in frames)
{
    Console.WriteLine($"received  {System.Text.Encoding.UTF8.GetString(frame)}");
}

Console.WriteLine($"discarded {decoder.Discarded} frame the stream mangled");
```
<!-- end -->

## Reference

<!-- table: reference serial -->
- Rust: [`pamoja-serial`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html)
- TypeScript: [`@pamoja/serial`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html)
- Python: [`pamoja.serial`](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html)
- C#: [`Pamoja.Serial`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html)
<!-- end -->
