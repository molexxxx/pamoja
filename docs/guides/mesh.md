# Mesh frames

Where the infrastructure is gone or was never there, devices carry each other's
traffic: every node relays what it hears, so a message crosses ground no single
node can reach. The radios that do that cheaply, the connectionless ESP-NOW of
an ESP32 swarm or a pennies-per-node nRF24, hand you a payload and nothing else:
no addressing, no hop count, no integrity. pamoja supplies that layer as pure
logic with no allocation, so the same code runs on a node with a radio and in a
test with none.

## What the example does

It builds the frame a river gauge floods into the mesh and checks it against the
bytes that go on the air, then takes the part of the node that hears it: drop
the second copy, spend a hop, and stop once the hops run out. Finally it flips
one payload bit and confirms the checksum rejects the frame.

It proves:

- The header is fixed and big-endian: a version, a source, a destination, a
  sequence number, and a hop limit, with the payload and a checksum after it.
- The checksum is CRC-16/CCITT-FALSE, pinned by the check value published for
  that algorithm rather than by a round trip against itself.
- A packet is identified as it floods by its source and sequence number, so the
  second copy to arrive is dropped instead of relayed again.
- Relaying spends one hop and leaves the frame valid, because the checksum
  covers every byte except the hop limit.
- A packet whose hops have run out is not relayed, which is what keeps a flood
  finite.
- A flipped payload bit fails the checksum instead of arriving as a plausible
  reading.

## Rust

<!-- snippet: examples/tests/guides/mesh.rs#example -->
From [`examples/tests/guides/mesh.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/mesh.rs):

```rust
use pamoja_mesh::{crc16, Frame, SeenCache, BROADCAST};

// A river gauge floods a reading to every node in range. The header is fixed and
// big-endian: version, source, destination, sequence id, hop limit, then the payload
// and a checksum over everything but the hop limit.
let reading = Frame::broadcast(0x1234_5678, 1, b"level=high").expect("the payload fits");
assert_eq!(reading.dst(), BROADCAST);
assert_eq!(
    reading.as_bytes(),
    b"\x01\x12\x34\x56\x78\xFF\xFF\xFF\xFF\x00\x01\x03level=high\x33\x35"
);

// The checksum is CRC-16/CCITT-FALSE, whose published check value fixes the polynomial
// and the starting value.
assert_eq!(crc16(b"123456789"), 0x29B1);

// A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
let received = Frame::parse(reading.as_bytes()).expect("the checksum matches");
assert_eq!(received.payload(), b"level=high");
let mut seen: SeenCache<64> = SeenCache::new();
assert!(seen.record(received.dedup_key()));
assert!(!seen.record(received.dedup_key()));

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards
// the frame without recomputing it and the check stays end to end.
let forwarded = received.relayed().expect("hops remain");
assert_eq!(forwarded.hop_limit(), received.hop_limit() - 1);
let onward = Frame::parse(forwarded.as_bytes()).expect("the checksum still matches");
assert_eq!(onward.payload(), received.payload());
assert_eq!(received.with_hop_limit(0).relayed(), None);

// A payload byte the air mangled fails the checksum rather than reaching the
// application as a plausible reading.
let mut mangled = reading.as_bytes().to_vec();
mangled[12] ^= 0xFF;
assert!(Frame::parse(&mangled).is_err());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/mesh.ts#example -->
From [`bindings/node/guides/mesh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mesh.ts):

```typescript
import assert from 'node:assert/strict'

import { BROADCAST, SeenPackets, broadcast, crc16, parse, relayed } from '@pamoja/mesh'

// A river gauge floods a reading to every node in range. The header is fixed and
// big-endian: version, source, destination, sequence id, hop limit, then the payload
// and a checksum over everything but the hop limit.
const reading = broadcast(0x12345678, 1, Buffer.from('level=high'))
assert.equal(reading.dst, BROADCAST)
assert.equal(
  reading.bytes.toString('hex'),
  '0112345678ffffffff0001036c6576656c3d686967683335'
)

// The checksum is CRC-16/CCITT-FALSE, whose published check value fixes the polynomial
// and the starting value.
assert.equal(crc16(Buffer.from('123456789')), 0x29b1)

// A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
const received = parse(reading.bytes)
assert.equal(received.payload.toString(), 'level=high')
const seen = new SeenPackets(64)
assert.ok(seen.record(received.src, received.id))
assert.ok(!seen.record(received.src, received.id))

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
// frame without recomputing it and the check stays end to end.
const forwarded = relayed(received.bytes)!
assert.equal(forwarded.hopLimit, received.hopLimit - 1)
assert.deepEqual(parse(forwarded.bytes).payload, received.payload)
assert.equal(relayed(broadcast(0x12345678, 1, Buffer.from('level=high'), 0).bytes), null)

// A payload byte the air mangled fails the checksum rather than reaching the application
// as a plausible reading.
const mangled = Buffer.from(reading.bytes)
mangled[12] ^= 0xff
assert.throws(() => parse(mangled))
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/mesh.py#example -->
From [`bindings/python/guides/mesh.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mesh.py):

```python
from pamoja.core import PamojaError
from pamoja.mesh import BROADCAST, SeenPackets, broadcast, crc16, parse, relayed

# A river gauge floods a reading to every node in range. The header is fixed and
# big-endian: version, source, destination, sequence id, hop limit, then the payload
# and a checksum over everything but the hop limit.
reading = broadcast(0x12345678, 1, b"level=high")
assert reading.dst == BROADCAST
assert reading.bytes.hex() == "0112345678ffffffff0001036c6576656c3d686967683335"

# The checksum is CRC-16/CCITT-FALSE, whose published check value fixes the polynomial
# and the starting value.
assert crc16(b"123456789") == 0x29B1

# A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
# several times over; the source and sequence id decide which copy is the first.
received = parse(reading.bytes)
assert received.payload == b"level=high"
seen = SeenPackets(64)
assert seen.record(received.src, received.id)
assert not seen.record(received.src, received.id)

# Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
# frame without recomputing it and the check stays end to end.
forwarded = relayed(received.bytes)
assert forwarded.hop_limit == received.hop_limit - 1
assert parse(forwarded.bytes).payload == received.payload
assert relayed(broadcast(0x12345678, 1, b"level=high", 0).bytes) is None

# A payload byte the air mangled fails the checksum rather than reaching the application
# as a plausible reading.
mangled = bytearray(reading.bytes)
mangled[12] ^= 0xFF
try:
    parse(bytes(mangled))
except PamojaError:
    pass
else:
    raise AssertionError("a frame mangled on the air should be rejected")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs):

```csharp
// A river gauge floods a reading to every node in range. The header is fixed and
// big-endian: version, source, destination, sequence id, hop limit, then the
// payload and a checksum over everything but the hop limit.
MeshFrame reading = Mesh.BroadcastFrame(0x1234_5678, 1, "level=high"u8);
Expect(reading.Dst == Mesh.Broadcast, "a broadcast is addressed to every node");
Expect(
    reading.Bytes.SequenceEqual(
        Convert.FromHexString("0112345678ffffffff0001036c6576656c3d686967683335")),
    "the frame is the bytes that go on the air");

// The checksum is CRC-16/CCITT-FALSE, whose published check value fixes the
// polynomial and the starting value.
Expect(Mesh.Crc16("123456789"u8) == 0x29B1, "the checksum is CRC-16/CCITT-FALSE");

// A neighbour hears it. Every node in range rebroadcasts, so the same packet
// arrives several times over; the source and sequence id decide which copy is
// the first.
MeshFrame received = Mesh.Parse(reading.Bytes);
Expect(received.Payload.SequenceEqual("level=high"u8.ToArray()), "it carries the reading");
using SeenPackets seen = new(64);
Expect(seen.Record(received.Src, received.Id), "the first copy is new");
Expect(!seen.Record(received.Src, received.Id), "a second copy is a duplicate");

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay
// forwards the frame without recomputing it and the check stays end to end.
MeshFrame forwarded = Mesh.Relayed(received.Bytes)!;
Expect(forwarded.HopLimit == received.HopLimit - 1, "relaying spends one hop");
Expect(
    Mesh.Parse(forwarded.Bytes).Payload.SequenceEqual(received.Payload),
    "and leaves the frame valid on the air");
Expect(
    Mesh.Relayed(Mesh.BroadcastFrame(0x1234_5678, 1, "level=high"u8, 0).Bytes) is null,
    "a packet out of hops is not relayed further");

// A payload byte the air mangled fails the checksum rather than reaching the
// application as a plausible reading.
byte[] mangled = [.. reading.Bytes];
mangled[12] ^= 0xFF;
bool rejected = false;
try
{
    Mesh.Parse(mangled);
}
catch (PamojaException)
{
    rejected = true;
}
Expect(rejected, "a frame mangled on the air is rejected");
```
<!-- end -->

## Reference

<!-- table: reference mesh -->
- Rust: [`pamoja-mesh`](https://docs.rs/pamoja-mesh) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html))
- TypeScript: [`@pamoja/mesh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html)
- Python: [`pamoja.mesh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html)
- C#: [`Mesh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.Mesh.html), [`MeshFrame`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.MeshFrame.html), [`SeenPackets`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.SeenPackets.html)
<!-- end -->
