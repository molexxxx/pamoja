# Serial framing

A UART, an RS232 link, or a USB-serial bridge hands an application a stream of
bytes with no notion of where one message ends and the next begins. Byte stuffing
supplies that boundary: reserve a byte value as the delimiter and encode the
payload so the value can never occur inside it. pamoja implements the two that
field hardware speaks, SLIP and COBS, as pure byte work with no allocation and no
serial port of its own, so the same code runs on a gateway and on a
microcontroller hanging off the bus.

## What the example does

It frames a payload with SLIP and checks it against the escape pairs RFC 1055
fixes, frames a second payload with COBS and checks it against the worked example
in the COBS paper, then feeds a decoder one chunk holding two frames with a
truncated one between them.

It proves:

- A payload carrying the delimiter or the escape byte is stuffed exactly as
  RFC 1055 specifies, so an implementation that is wrong but self-consistent
  still fails.
- COBS encodes `11 22 00 33` as `03 11 22 02 33 00`, the run-length codes the
  specification works through.
- Both framings give the original payload back.
- The streaming decoder splits a chunk into whole frames, drops the truncated one
  rather than the chunk around it, and counts what it dropped.

## Rust

<!-- snippet: examples/tests/guides/serial.rs#example -->
From [`examples/tests/guides/serial.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/serial.rs):

```rust
use pamoja_serial::{cobs, slip};

// SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a payload
// carrying either goes out as the two-byte pair RFC 1055 fixes for it.
let payload = [0x01, 0xC0, 0xDB, 0x02];
let mut frame = [0u8; slip::max_encoded_len(4)];
let n = slip::encode(&payload, &mut frame).expect("room for the frame");
assert_eq!(&frame[..n], &[0x01, 0xDB, 0xDC, 0xDB, 0xDD, 0x02, 0xC0]);
let mut restored = [0u8; 4];
let m = slip::decode(&frame[..n], &mut restored).expect("a well-formed frame");
assert_eq!(&restored[..m], &payload);

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes,
// each run led by its own length. This is the worked example from the COBS paper.
let packet = [0x11, 0x22, 0x00, 0x33];
let mut framed = [0u8; cobs::max_encoded_len(4)];
let n = cobs::encode(&packet, &mut framed).expect("room for the frame");
assert_eq!(&framed[..n], &[0x03, 0x11, 0x22, 0x02, 0x33, 0x00]);

// A serial read returns an arbitrary chunk rather than a packet. This one holds two
// frames with a truncated one between them, and the decoder drops only the bad frame.
let mut decoder: slip::SlipDecoder<16> = slip::SlipDecoder::new();
let mut frames: Vec<Vec<u8>> = Vec::new();
let mut discarded = 0;
for &byte in &[b'o', b'k', 0xC0, 0xDB, 0xC0, b'g', b'o', 0xC0] {
    match decoder.push(byte) {
        Ok(Some(complete)) => frames.push(complete.to_vec()),
        Ok(None) => {}
        Err(_) => discarded += 1,
    }
}
assert_eq!(frames, [b"ok".to_vec(), b"go".to_vec()]);
assert_eq!(discarded, 1);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/serial.ts#example -->
From [`bindings/node/guides/serial.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/serial.ts):

```typescript
import assert from 'node:assert/strict'

import { SlipDecoder, cobs, slip } from '@pamoja/serial'

// SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a payload
// carrying either goes out as the two-byte pair RFC 1055 fixes for it.
const payload = Buffer.from([0x01, 0xc0, 0xdb, 0x02])
const frame = slip.encode(payload)
assert.deepEqual([...frame], [0x01, 0xdb, 0xdc, 0xdb, 0xdd, 0x02, 0xc0])
assert.deepEqual(slip.decode(frame), payload)

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
// run led by its own length. This is the worked example from the COBS paper.
const packet = Buffer.from([0x11, 0x22, 0x00, 0x33])
const framed = cobs.encode(packet)
assert.deepEqual([...framed], [0x03, 0x11, 0x22, 0x02, 0x33, 0x00])
assert.deepEqual(cobs.decode(framed), packet)

// A serial read returns an arbitrary chunk rather than a packet. This one holds two
// frames with a truncated one between them, and the decoder drops only the bad frame.
const decoder = new SlipDecoder()
const frames = decoder.feed(Buffer.from([0x6f, 0x6b, 0xc0, 0xdb, 0xc0, 0x67, 0x6f, 0xc0]))
assert.deepEqual(frames, [Buffer.from('ok'), Buffer.from('go')])
assert.equal(decoder.discarded, 1)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/serial.py#example -->
From [`bindings/python/guides/serial.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/serial.py):

```python
from pamoja.serial import SlipDecoder, cobs, slip

# SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a payload
# carrying either goes out as the two-byte pair RFC 1055 fixes for it.
payload = bytes([0x01, 0xC0, 0xDB, 0x02])
frame = slip.encode(payload)
assert frame == bytes([0x01, 0xDB, 0xDC, 0xDB, 0xDD, 0x02, 0xC0])
assert slip.decode(frame) == payload

# COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
# run led by its own length. This is the worked example from the COBS paper.
packet = bytes([0x11, 0x22, 0x00, 0x33])
framed = cobs.encode(packet)
assert framed == bytes([0x03, 0x11, 0x22, 0x02, 0x33, 0x00])
assert cobs.decode(framed) == packet

# A read from a port returns an arbitrary chunk rather than a packet. This one holds two
# frames with a truncated one between them, and the decoder drops only the bad frame.
decoder = SlipDecoder()
frames = decoder.feed(bytes([0x6F, 0x6B, 0xC0, 0xDB, 0xC0, 0x67, 0x6F, 0xC0]))
assert frames == [b"ok", b"go"]
assert decoder.discarded == 1
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SerialGuide.cs):

```csharp
// SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a
// payload carrying either goes out as the two-byte pair RFC 1055 fixes for it.
byte[] payload = [0x01, 0xC0, 0xDB, 0x02];
byte[] frame = Serial.SlipEncode(payload);
Expect(
    frame.SequenceEqual(new byte[] { 0x01, 0xDB, 0xDC, 0xDB, 0xDD, 0x02, 0xC0 }),
    "the frame is the escaping RFC 1055 fixes");
Expect(Serial.SlipDecode(frame).SequenceEqual(payload), "the payload comes back");

// COBS trades that escaping for one code byte per run of up to 254 non-zero
// bytes, each run led by its own length. This is the COBS paper's worked example.
byte[] packet = [0x11, 0x22, 0x00, 0x33];
byte[] framed = Serial.CobsEncode(packet);
Expect(
    framed.SequenceEqual(new byte[] { 0x03, 0x11, 0x22, 0x02, 0x33, 0x00 }),
    "the frame is the one the COBS paper works through");
Expect(Serial.CobsDecode(framed).SequenceEqual(packet), "the packet comes back");

// A serial read returns an arbitrary chunk rather than a packet. This one holds
// two frames with a truncated one between them, and only the bad frame is dropped.
using SlipDecoder decoder = new();
byte[][] frames = decoder.Feed([0x6F, 0x6B, 0xC0, 0xDB, 0xC0, 0x67, 0x6F, 0xC0]);
Expect(frames.Length == 2, "the frames either side of the bad one survive");
Expect(frames[0].SequenceEqual("ok"u8.ToArray()), "the first frame reassembles");
Expect(frames[1].SequenceEqual("go"u8.ToArray()), "the second frame reassembles");
Expect(decoder.Discarded == 1, "the truncated frame is counted, not raised");
```
<!-- end -->

## Reference

<!-- table: reference serial -->
- Rust: [`pamoja-serial`](https://docs.rs/pamoja-serial) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html))
- TypeScript: [`@pamoja/serial`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html)
- Python: [`pamoja.serial`](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html)
- C#: [`Serial`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.Serial.html), [`SlipDecoder`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.SlipDecoder.html), [`CobsDecoder`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.CobsDecoder.html)
<!-- end -->
