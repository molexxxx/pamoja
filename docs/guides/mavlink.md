# MAVLink

MAVLink is what a ground station and an autopilot say to each other. A frame is a
marker byte, a small header, a payload, and a checksum, and the checksum is
seeded with a per-message constant so a receiver that disagrees about a message's
shape rejects the frame rather than misreading it. pamoja builds and parses v1
and v2 frames and does not own the link, so the same code drives a serial
adapter, a UDP socket, or a test that never leaves the process.

## What the example does

It checks the CRC against the CRC-16/MCRF4XX catalogue check value and the
HEARTBEAT `CRC_EXTRA` the common dialect publishes, then frames a heartbeat and
compares it to the exact bytes that go on the wire. It then feeds a parser the
kind of input a real link delivers: leading noise, a frame with a flipped bit,
and a good frame split across two reads.

It proves:

- The checksum matches the published check value, and the per-message seed is the
  one the dialect publishes, so an implementation that is wrong but
  self-consistent still fails.
- The framed heartbeat is byte-for-byte the v2 frame, marker and checksum
  included.
- A parser skips bytes that do not start a frame, and drops a frame whose
  checksum fails rather than passing it on.
- A frame split across two reads still arrives whole, with its version, message
  id, and payload intact.

## Rust

<!-- snippet: examples/tests/guides/mavlink.rs#example -->
From [`examples/tests/guides/mavlink.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/mavlink.rs):

```rust
use pamoja_mavlink::dialect::{self, Heartbeat, Message};
use pamoja_mavlink::{crc16_mcrf4xx, Frame, Header, Parser, Version};

// 0x6F91 over "123456789" is the catalogue check value for CRC-16/MCRF4XX, and 50 is
// the CRC_EXTRA the common dialect publishes for HEARTBEAT.
assert_eq!(crc16_mcrf4xx(b"123456789"), 0x6F91);
assert_eq!(Heartbeat::CRC_EXTRA, 50);

// A HEARTBEAT announcing an onboard controller in an active state. The v2 frame around
// it is the 0xFD marker, the payload length, two flag bytes, the sequence, the sending
// system and component, a 24-bit message id, the payload, then the checksum.
let heartbeat = [0, 0, 0, 0, 18, 0, 0, 4, 3];
let header = Header::new(1, 1, 7);
let sent = Frame::encode_v2(header, Heartbeat::ID, &heartbeat, Heartbeat::CRC_EXTRA)
    .expect("a payload within the limit");
assert_eq!(
    sent.as_bytes(),
    &[
        0xFD, 0x09, 0x00, 0x00, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x12, 0x00, 0x00, 0x04, 0x03, 0x75, 0x3A
    ]
);

// A link delivers bytes, not frames. The parser skips whatever does not start one and
// drops a frame whose checksum fails rather than passing it on.
let mut mangled = sent.as_bytes().to_vec();
mangled[14] ^= 0xFF;
let mut delivered = vec![0x11, 0x22, 0x33];
delivered.extend_from_slice(&mangled);
delivered.extend_from_slice(sent.as_bytes());

let mut parser = Parser::new();
let received = delivered
    .iter()
    .find_map(|&byte| parser.push_byte(byte, &dialect::crc_extra))
    .expect("the good frame completes");
assert_eq!(received.version(), Version::V2);
assert_eq!(received.message_id(), Heartbeat::ID);
assert_eq!(received.payload(), &heartbeat[..]);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/mavlink.ts#example -->
From [`bindings/node/guides/mavlink.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mavlink.ts):

```typescript
import assert from 'node:assert/strict'

import { MavlinkParser, MavlinkVersion, crc16, frame, knownCrcExtra } from '@pamoja/mavlink'

// 0x6F91 over "123456789" is the catalogue check value for CRC-16/MCRF4XX, and 50 is the
// CRC_EXTRA the common dialect publishes for HEARTBEAT.
assert.equal(crc16(Buffer.from('123456789')), 0x6f91)
assert.equal(knownCrcExtra(0), 50)

// A HEARTBEAT announcing an onboard controller in an active state. The v2 frame around it
// is the 0xFD marker, the payload length, two flag bytes, the sequence, the sending system
// and component, a 24-bit message id, the payload, then the checksum.
const heartbeat = Buffer.from([0, 0, 0, 0, 18, 0, 0, 4, 3])
const sent = frame({ systemId: 1, componentId: 1, sequence: 7 }, 0, heartbeat)
assert.deepEqual(
  [...sent.bytes],
  [
    0xfd, 0x09, 0x00, 0x00, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x12, 0x00, 0x00, 0x04, 0x03, 0x75, 0x3a,
  ],
)

// A link delivers bytes, not frames. The parser skips whatever does not start one and
// drops a frame whose checksum fails rather than passing it on.
const mangled = Buffer.from(sent.bytes)
mangled[14] ^= 0xff
const parser = new MavlinkParser()
assert.deepEqual(parser.push(Buffer.concat([Buffer.from([0x11, 0x22, 0x33]), mangled])), [])

// The same frame, split across two reads, still arrives whole.
assert.deepEqual(parser.push(sent.bytes.subarray(0, 5)), [])
const found = parser.push(sent.bytes.subarray(5))
assert.equal(found.length, 1)
assert.equal(found[0].version, MavlinkVersion.V2)
assert.equal(found[0].messageId, 0)
assert.deepEqual(found[0].payload, heartbeat)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/mavlink.py#example -->
From [`bindings/python/guides/mavlink.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mavlink.py):

```python
from pamoja.mavlink import MavlinkHeader, MavlinkParser, crc16, frame, known_crc_extra

# 0x6F91 over "123456789" is the catalogue check value for CRC-16/MCRF4XX, and 50 is the
# CRC_EXTRA the common dialect publishes for HEARTBEAT.
assert crc16(b"123456789") == 0x6F91
assert known_crc_extra(0) == 50

# A HEARTBEAT announcing an onboard controller in an active state. The v2 frame around it
# is the 0xFD marker, the payload length, two flag bytes, the sequence, the sending system
# and component, a 24-bit message id, the payload, then the checksum.
heartbeat = bytes([0, 0, 0, 0, 18, 0, 0, 4, 3])
sent = frame(MavlinkHeader(1, 1, 7), 0, heartbeat)
assert sent.bytes == bytes(
    [0xFD, 0x09, 0x00, 0x00, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
     0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x04, 0x03, 0x75, 0x3A]
)

# A link delivers bytes, not frames. The parser skips whatever does not start one and
# drops a frame whose checksum fails rather than passing it on.
mangled = bytearray(sent.bytes)
mangled[14] ^= 0xFF
parser = MavlinkParser()
assert parser.push(bytes([0x11, 0x22, 0x33]) + bytes(mangled)) == []

# The same frame, split across two reads, still arrives whole.
assert parser.push(sent.bytes[:5]) == []
found = parser.push(sent.bytes[5:])
assert len(found) == 1
assert found[0].version == 2
assert found[0].message_id == 0
assert found[0].payload == heartbeat
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs):

```csharp
// 0x6F91 over "123456789" is the catalogue check value for CRC-16/MCRF4XX, and 50
// is the CRC_EXTRA the common dialect publishes for HEARTBEAT.
Expect(Mavlink.Crc16("123456789"u8) == 0x6F91, "the checksum is CRC-16/MCRF4XX");
Expect(Mavlink.KnownCrcExtra(0) == 50, "HEARTBEAT's published seed");

// A HEARTBEAT announcing an onboard controller in an active state. The v2 frame
// around it is the 0xFD marker, the payload length, two flag bytes, the sequence,
// the sending system and component, a 24-bit message id, the payload, then the
// checksum.
byte[] heartbeat = [0, 0, 0, 0, 18, 0, 0, 4, 3];
using MavlinkFrame sent = Mavlink.Frame(new MavlinkHeader(1, 1, 7), 0, heartbeat);
byte[] wire =
[
    0xFD, 0x09, 0x00, 0x00, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x04, 0x03, 0x75, 0x3A,
];
Expect(sent.Bytes.SequenceEqual(wire), "the frame is the layout v2 fixes");

// A link delivers bytes, not frames. The parser skips whatever does not start one
// and drops a frame whose checksum fails rather than passing it on.
byte[] mangled = sent.Bytes;
mangled[14] ^= 0xFF;
using MavlinkParser parser = new();
byte[] noisy = [0x11, 0x22, 0x33, .. mangled];
Expect(parser.Push(noisy).Count == 0, "neither noise nor a failed checksum is reported");

// The same frame, split across two reads, still arrives whole.
Expect(parser.Push(sent.Bytes.AsSpan(0, 5)).Count == 0, "half a frame is not a frame");
IReadOnlyList<MavlinkFrame> found = parser.Push(sent.Bytes.AsSpan(5));
Expect(found.Count == 1, "the rest of it completes one");
using MavlinkFrame received = found[0];
Expect(received.Version == MavlinkVersion.V2, "v2 is the current wire format");
Expect(received.MessageId == 0, "and it is the heartbeat that was sent");
Expect(received.Payload.SequenceEqual(heartbeat), "with its payload intact");
```
<!-- end -->

## Reference

<!-- table: reference mavlink -->
- Rust: [`pamoja-mavlink`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html)
- TypeScript: [`@pamoja/mavlink`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html)
- Python: [`pamoja.mavlink`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html)
- C#: [`Pamoja.Mavlink`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html)
<!-- end -->
