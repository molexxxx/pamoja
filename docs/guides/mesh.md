# Mesh frames

Where the infrastructure is gone or was never there, devices carry each other's
traffic: every node relays what it hears, so a message crosses ground no single
node can reach. The radios that do that cheaply, the connectionless ESP-NOW of
an ESP32 swarm or a pennies-per-node nRF24, hand you a payload and nothing else:
no addressing, no hop count, no integrity. pamoja supplies that layer as pure
logic with no allocation, so the same code runs on a node with a radio and in a
test with none.

## What the example does

It builds the frame a river gauge floods into the mesh, then takes the part of
the node that hears it: parse the frame, drop the second copy, spend a hop, and
stop once the hops run out. Finally it inverts a payload byte and confirms the
checksum rejects the frame.

The header layout is fixed and big-endian, and the byte-for-byte frame is pinned
in the conformance vectors every binding checks itself against, so this page
shows what a node does with a frame rather than restating one. Nothing about the
frame is typed out in hex: the destination is checked against the exported
`BROADCAST` address, the hop limit a new frame starts with comes from the
crate's default, and the size on the air is measured off the frame that was
built.

It proves:

- A broadcast frame parses back out of the bytes that go on the air with its
  payload intact and its destination equal to `BROADCAST`.
- A packet is identified as it floods by its source and sequence id, so the
  second copy to arrive is dropped instead of relayed again.
- Relaying spends exactly one hop, and the forwarded bytes still parse and carry
  the same payload, because the checksum covers every byte except the hop limit.
- A packet whose hops have run out is not relayed, which is what keeps a flood
  finite.
- An inverted payload byte fails the checksum instead of arriving as a plausible
  reading.

## Rust

<!-- snippet: examples/tests/guides/mesh.rs#example -->
From [`examples/tests/guides/mesh.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/mesh.rs):

```rust
use pamoja_mesh::{Frame, SeenCache, BROADCAST};

// A river gauge floods a level reading to every node in range. The header is fixed
// and big-endian: version, source, destination, sequence id, hop limit, then the
// payload and a checksum over everything but the hop limit.
const RIVER_GAUGE: u32 = 305_419_896;
let reading = Frame::broadcast(RIVER_GAUGE, 1, b"level=high").expect("the payload fits");
let on_the_air = reading.as_bytes().len();
let to_everyone = reading.dst() == BROADCAST;
println!("sent      {on_the_air} bytes to every node in range");
println!("addressed to broadcast: {to_everyone}");

// A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
let received = Frame::parse(reading.as_bytes()).expect("the checksum matches");
println!("payload   {}", String::from_utf8_lossy(received.payload()));

let mut seen: SeenCache<64> = SeenCache::new();
let first = seen.record(received.dedup_key());
let again = seen.record(received.dedup_key());
println!("first copy relayed: {first}, second copy relayed: {again}");

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards
// the frame without recomputing it and the check stays end to end.
let forwarded = received.relayed().expect("hops remain");
println!("relayed   hop limit {}", forwarded.hop_limit());
let onward = Frame::parse(forwarded.as_bytes()).expect("the checksum still matches");
println!("onward    {}", String::from_utf8_lossy(onward.payload()));

// A frame that has run out of hops is not relayed again, which is what ends the flood.
match received.with_hop_limit(0).relayed() {
    Some(_) => println!("a spent frame was relayed, which should never happen"),
    None => println!("spent     hop limit reached, the flood stops here"),
}

// A payload byte the air mangled fails the checksum rather than reaching the
// application as a plausible reading. The header is a fixed width, so the first
// byte past it is the first byte of the reading itself.
let mut mangled = reading.as_bytes().to_vec();
mangled[Frame::HEADER_LEN] ^= 0xFF;
match Frame::parse(&mangled) {
    Ok(_) => println!("a mangled frame was accepted, which should never happen"),
    Err(error) => println!("mangled   rejected: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/mesh.ts#example -->
From [`bindings/node/guides/mesh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mesh.ts):

```typescript
import { BROADCAST, HEADER_LEN, SeenPackets, broadcast, parse, relayed } from '@pamoja/mesh'

// A river gauge floods a level reading to every node in range. The header is fixed and
// big-endian: version, source, destination, sequence id, hop limit, then the payload and
// a checksum over everything but the hop limit.
const RIVER_GAUGE = 305419896
const reading = broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'))
console.log(`sent      ${reading.bytes.length} bytes to every node in range`)
console.log(`addressed to broadcast: ${reading.dst === BROADCAST}`)

// A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
const received = parse(reading.bytes)
console.log(`payload   ${received.payload.toString()}`)

const seen = new SeenPackets(64)
const first = seen.record(received.src, received.id)
const again = seen.record(received.src, received.id)
console.log(`first copy relayed: ${first}, second copy relayed: ${again}`)

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
// frame without recomputing it and the check stays end to end.
const forwarded = relayed(received.bytes)!
console.log(`relayed   hop limit ${forwarded.hopLimit}`)
const onward = parse(forwarded.bytes)
console.log(`onward    ${onward.payload.toString()}`)

// A frame that has run out of hops is not relayed again, which is what ends the flood.
const spent = relayed(broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'), 0).bytes)
if (spent === null) {
  console.log('spent     hop limit reached, the flood stops here')
} else {
  console.log('a spent frame was relayed, which should never happen')
}

// A payload byte the air mangled fails the checksum rather than reaching the application
// as a plausible reading. The header is a fixed width, so the first byte past it is the
// first byte of the reading itself.
const mangled = Buffer.from(reading.bytes)
mangled[HEADER_LEN] ^= 0xff
try {
  parse(mangled)
  console.log('a mangled frame was accepted, which should never happen')
} catch (error) {
  console.log(`mangled   rejected: ${(error as Error).message}`)
}
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/mesh.py#example -->
From [`bindings/python/guides/mesh.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mesh.py):

```python
from pamoja.core import PamojaError
from pamoja.mesh import BROADCAST, HEADER_LEN, SeenPackets, broadcast, parse, relayed

# A river gauge floods a level reading to every node in range. The header is fixed and
# big-endian: version, source, destination, sequence id, hop limit, then the payload and a
# checksum over everything but the hop limit.
RIVER_GAUGE = 305419896
reading = broadcast(RIVER_GAUGE, 1, b"level=high")
print(f"sent      {len(reading.bytes)} bytes to every node in range")
print(f"addressed to broadcast: {reading.dst == BROADCAST}")

# A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
# several times over; the source and sequence id decide which copy is the first.
received = parse(reading.bytes)
print(f"payload   {received.payload.decode()}")

seen = SeenPackets(64)
first = seen.record(received.src, received.id)
again = seen.record(received.src, received.id)
print(f"first copy relayed: {first}, second copy relayed: {again}")

# Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
# frame without recomputing it and the check stays end to end.
forwarded = relayed(received.bytes)
print(f"relayed   hop limit {forwarded.hop_limit}")
onward = parse(forwarded.bytes)
print(f"onward    {onward.payload.decode()}")

# A frame that has run out of hops is not relayed again, which is what ends the flood.
spent = relayed(broadcast(RIVER_GAUGE, 1, b"level=high", 0).bytes)
if spent is None:
    print("spent     hop limit reached, the flood stops here")
else:
    print("a spent frame was relayed, which should never happen")

# A payload byte the air mangled fails the checksum rather than reaching the application
# as a plausible reading. The header is a fixed width, so the first byte past it is the
# first byte of the reading itself.
mangled = bytearray(reading.bytes)
mangled[HEADER_LEN] ^= 0xFF
try:
    parse(bytes(mangled))
    print("a mangled frame was accepted, which should never happen")
except PamojaError as error:
    print(f"mangled   rejected: {error}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs):

```csharp
// A river gauge floods a level reading to every node in range. The header is fixed
// and big-endian: version, source, destination, sequence id, hop limit, then the
// payload and a checksum over everything but the hop limit.
const uint RiverGauge = 305_419_896;
MeshFrame reading = Mesh.BroadcastFrame(RiverGauge, 1, "level=high"u8);
Console.WriteLine($"sent      {reading.Bytes.Length} bytes to every node in range");
Console.WriteLine($"addressed to broadcast: {reading.Dst == Mesh.Broadcast}");

// A neighbour hears it. Every node in range rebroadcasts, so the same packet
// arrives several times over; the source and sequence id decide which copy is
// the first.
MeshFrame received = Mesh.Parse(reading.Bytes);
Console.WriteLine($"payload   {System.Text.Encoding.UTF8.GetString(received.Payload)}");

using SeenPackets seen = new(64);
bool first = seen.Record(received.Src, received.Id);
bool again = seen.Record(received.Src, received.Id);
Console.WriteLine($"first copy relayed: {first}, second copy relayed: {again}");

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay
// forwards the frame without recomputing it and the check stays end to end.
MeshFrame forwarded = Mesh.Relayed(received.Bytes)!;
Console.WriteLine($"relayed   hop limit {forwarded.HopLimit}");
MeshFrame onward = Mesh.Parse(forwarded.Bytes);
Console.WriteLine($"onward    {System.Text.Encoding.UTF8.GetString(onward.Payload)}");

// A frame that has run out of hops is not relayed again, which ends the flood.
MeshFrame? spent = Mesh.Relayed(Mesh.BroadcastFrame(RiverGauge, 1, "level=high"u8, 0).Bytes);
Console.WriteLine(spent is null
    ? "spent     hop limit reached, the flood stops here"
    : "a spent frame was relayed, which should never happen");

// A payload byte the air mangled fails the checksum rather than reaching the
// application as a plausible reading. The header is a fixed width, so the first
// byte past it is the first byte of the reading itself.
byte[] mangled = [.. reading.Bytes];
mangled[Mesh.HeaderLen] ^= 0xFF;
try
{
    Mesh.Parse(mangled);
    Console.WriteLine("a mangled frame was accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"mangled   rejected: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference mesh -->
- Rust: [`pamoja-mesh`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mesh)
- TypeScript: [`@pamoja/mesh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mesh)
- Python: [`pamoja.mesh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mesh)
- C#: [`Pamoja.Mesh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mesh)
<!-- end -->
