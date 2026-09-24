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
stop once the hops run out. Then it inverts a payload byte and confirms the
checksum rejects the frame.

The second part floods the same reading down a valley of six nodes, each in range
of its neighbors only, and counts what happens: who took the reading, who relayed
it, how many copies came back up the valley as echoes, and how far it got before
the hops ran out. It ends with two mistakes: a node whose memory of packets is too
small, and a payload too large for a frame.

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
  finite, and a flood that starts with a hop limit of 3 reaches four radio hops
  from its source.
- Each node takes the reading once and relays it once. The copies its neighbor
  sends back are echoes, and the node's memory drops them.
- A memory that holds too few packets forgets one while copies of it are still in
  the air, and relays a late copy again.
- An inverted payload byte fails the checksum instead of arriving as a plausible
  reading, and a payload past what a frame carries is refused before it is built.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example mesh" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example mesh</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- mesh" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- mesh</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/mesh.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/mesh.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- mesh" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- mesh</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-mesh` is `no_std` and never allocates. `Frame` builds a frame
with `new` or `broadcast`, reads one with `parse`, and gives a relay its next copy
with `relayed`, which is `None` once the hops run out. `SeenCache<N>` is a node's
memory of the last `N` packets, sized at compile time; `DynamicSeenCache` takes its
size at run time. `crc16` and `Crc16` are the checksum on their own. A frame that
cannot be built or read is an `Err` holding a `MeshError`.

<!-- snippet: examples/guides/mesh.rs#example -->
From [`examples/guides/mesh.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/mesh.rs):

```rust
use pamoja_mesh::{Frame, SeenCache, BROADCAST};

// A river gauge floods a level reading to every node in range. The header is fixed
// and big-endian: version, source, destination, sequence id, hop limit, then the
// payload and a checksum over everything but the hop limit.
const RIVER_GAUGE: u32 = 305_419_896;
let reading = Frame::broadcast(RIVER_GAUGE, 1, b"level=high")?;
let to = if reading.dst() == BROADCAST {
    "every node in range"
} else {
    "one node"
};
println!(
    "sent      {} bytes to {to}, hop limit {}",
    reading.as_bytes().len(),
    reading.hop_limit()
);

// A neighbor hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
let received = Frame::parse(reading.as_bytes())?;
println!("payload   {}", String::from_utf8_lossy(received.payload()));
let mut seen: SeenCache<64> = SeenCache::new();
let first = seen.record(received.dedup_key());
let again = seen.record(received.dedup_key());
if first && !again {
    println!("dedup     the first copy is relayed, and the second is dropped");
}

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards
// the frame without recomputing it and the check stays end to end.
let forwarded = received.relayed().expect("hops remain");
let onward = Frame::parse(forwarded.as_bytes())?;
println!(
    "relayed   hop limit {}, and the checksum still holds: {}",
    forwarded.hop_limit(),
    String::from_utf8_lossy(onward.payload())
);

// A frame that has run out of hops is not relayed again, which is what ends the flood.
if received.with_hop_limit(0).relayed().is_none() {
    println!("spent     at hop limit 0 the frame goes no further");
}

// A payload byte the air mangled fails the checksum rather than reaching the
// application as a plausible reading. The header is a fixed width, so the first byte
// past it is the first byte of the reading itself.
let mut mangled = reading.as_bytes().to_vec();
mangled[Frame::HEADER_LEN] ^= 0xFF;
match Frame::parse(&mangled) {
    Ok(_) => println!("a mangled frame was accepted, which should never happen"),
    Err(error) => println!("mangled   rejected: {error}"),
}
```
<!-- end -->

The flood down the valley, continuing from above:

<!-- snippet: examples/guides/mesh.rs#flood -->
From [`examples/guides/mesh.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/mesh.rs):

```rust
// Six nodes down a river valley, each in range of its neighbors only. The gauge at the
// top, node 0, floods a reading, and every node that hears a packet for the first time
// takes it and relays it while hops remain. Copies that come back up the valley are
// echoes, which the memory of each node drops.
let nodes = 6;
let mut memory = vec![SeenCache::<64>::new(); nodes];
let flooded = Frame::broadcast(0, 1, b"level=high")?;
memory[0].record(flooded.dedup_key());
let mut on_the_air = std::collections::VecDeque::from([(0usize, flooded)]);
let (mut delivered, mut relays, mut echoes, mut farthest) = (0, 0, 0, 0);
while let Some((from, frame)) = on_the_air.pop_front() {
    for node in [from.checked_sub(1), Some(from + 1)].into_iter().flatten() {
        if node >= nodes {
            continue;
        }
        let heard = Frame::parse(frame.as_bytes())?;
        if !memory[node].record(heard.dedup_key()) {
            echoes += 1;
            continue;
        }
        delivered += 1;
        farthest = farthest.max(node);
        if let Some(onward) = heard.relayed() {
            relays += 1;
            on_the_air.push_back((node, onward));
        }
    }
}
println!(
    "flood     {delivered} nodes took the reading, {relays} relayed it, {echoes} echoes were dropped"
);
println!(
    "reach     node {farthest} was the farthest, {farthest} hops out, and node {} never heard it",
    nodes - 1
);

// A node whose memory holds two packets hears the reading, then two packets from
// other nodes, then a late copy of the reading by a longer path. It has forgotten the
// reading by then and relays it again; on a busy mesh that repeats without end.
let mut small: SeenCache<2> = SeenCache::new();
let rain = Frame::broadcast(7, 1, b"rain=4mm")?;
let wind = Frame::broadcast(8, 1, b"wind=12")?;
small.record(flooded.dedup_key());
small.record(rain.dedup_key());
small.record(wind.dedup_key());
if small.record(flooded.dedup_key()) {
    println!("forgot    a memory of two packets relays the late copy again");
}

// A payload one byte past what a frame carries is refused before it is built. A frame
// is sized to the 250 bytes ESP-NOW carries, less the header and the checksum.
match Frame::broadcast(0, 2, &[0u8; Frame::MAX_PAYLOAD + 1]) {
    Ok(_) => println!("an oversized payload was framed, which should never happen"),
    Err(error) => println!(
        "too long  {} bytes refused: {error}",
        Frame::MAX_PAYLOAD + 1
    ),
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/mesh` builds a frame with `frame` or `broadcast`, reads one
with `parse`, and relays it with `relayed`, which returns `null` once the hops run
out. A frame is a plain object with its `bytes` and each field. `SeenPackets` is a
node's memory, sized when it is made, and `record` answers whether a packet is new.
A frame that cannot be built or read throws an `Error` saying why.

<!-- snippet: bindings/node/guides/mesh.ts#example -->
From [`bindings/node/guides/mesh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mesh.ts):

```typescript
import { BROADCAST, HEADER_LEN, SeenPackets, broadcast, parse, relayed } from '@pamoja/mesh'

// A river gauge floods a level reading to every node in range. The header is fixed and
// big-endian: version, source, destination, sequence id, hop limit, then the payload and
// a checksum over everything but the hop limit.
const RIVER_GAUGE = 305419896
const reading = broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'))
const to = reading.dst === BROADCAST ? 'every node in range' : 'one node'
console.log(`sent      ${reading.bytes.length} bytes to ${to}, hop limit ${reading.hopLimit}`)

// A neighbor hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
const received = parse(reading.bytes)
console.log(`payload   ${received.payload.toString()}`)
const seen = new SeenPackets(64)
const first = seen.record(received.src, received.id)
const again = seen.record(received.src, received.id)
if (first && !again) {
  console.log('dedup     the first copy is relayed, and the second is dropped')
}

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
// frame without recomputing it and the check stays end to end.
const forwarded = relayed(received.bytes)!
const onward = parse(forwarded.bytes)
console.log(
  `relayed   hop limit ${forwarded.hopLimit}, and the checksum still holds: ${onward.payload.toString()}`,
)

// A frame that has run out of hops is not relayed again, which is what ends the flood.
if (relayed(broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'), 0).bytes) === null) {
  console.log('spent     at hop limit 0 the frame goes no further')
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

The flood down the valley, continuing from above:

<!-- snippet: bindings/node/guides/mesh.ts#flood -->
From [`bindings/node/guides/mesh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mesh.ts):

```typescript
import { DEFAULT_HOP_LIMIT, MAX_PAYLOAD } from '@pamoja/mesh'

// Six nodes down a river valley, each in range of its neighbors only. The gauge at the top,
// node 0, floods a reading, and every node that hears a packet for the first time takes it
// and relays it while hops remain. Copies that come back up the valley are echoes, which the
// memory of each node drops.
const nodes = 6
const memory = Array.from({ length: nodes }, () => new SeenPackets(64))
const flooded = broadcast(0, 1, Buffer.from('level=high'))
memory[0].record(flooded.src, flooded.id)
const onTheAir = [{ from: 0, bytes: flooded.bytes }]
let delivered = 0
let relays = 0
let echoes = 0
let farthest = 0
for (let sent = onTheAir.shift(); sent; sent = onTheAir.shift()) {
  for (const node of [sent.from - 1, sent.from + 1]) {
    if (node < 0 || node >= nodes) {
      continue
    }
    const heard = parse(sent.bytes)
    if (!memory[node].record(heard.src, heard.id)) {
      echoes += 1
      continue
    }
    delivered += 1
    farthest = Math.max(farthest, node)
    const onwardFrame = relayed(heard.bytes)
    if (onwardFrame) {
      relays += 1
      onTheAir.push({ from: node, bytes: onwardFrame.bytes })
    }
  }
}
console.log(`flood     ${delivered} nodes took the reading, ${relays} relayed it, ${echoes} echoes were dropped`)
console.log(
  `reach     node ${farthest} was the farthest, ${farthest} hops out, and node ${nodes - 1} never heard it`,
)

// A node whose memory holds two packets hears the reading, then two packets from other
// nodes, then a late copy of the reading by a longer path. It has forgotten the reading by
// then and relays it again; on a busy mesh that repeats without end.
const small = new SeenPackets(2)
const rain = broadcast(7, 1, Buffer.from('rain=4mm'))
const wind = broadcast(8, 1, Buffer.from('wind=12'))
small.record(flooded.src, flooded.id)
small.record(rain.src, rain.id)
small.record(wind.src, wind.id)
if (small.record(flooded.src, flooded.id)) {
  console.log('forgot    a memory of two packets relays the late copy again')
}

// A payload one byte past what a frame carries is refused before it is built. A frame is
// sized to the 250 bytes ESP-NOW carries, less the header and the checksum.
try {
  broadcast(0, 2, Buffer.alloc(MAX_PAYLOAD + 1))
  console.log('an oversized payload was framed, which should never happen')
} catch (error) {
  console.log(`too long  ${MAX_PAYLOAD + 1} bytes refused: ${(error as Error).message}`)
}
```
<!-- end -->

## Python

In Python, `pamoja.mesh` has `frame`, `broadcast`, `parse`, and `relayed`, which
returns `None` once the hops run out, and `SeenPackets` for a node's memory. A
frame's `bytes` and payload are `bytes`, and a hop limit can be set when a frame is
built. A frame that cannot be built or read raises `PamojaError`.

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
to = "every node in range" if reading.dst == BROADCAST else "one node"
print(f"sent      {len(reading.bytes)} bytes to {to}, hop limit {reading.hop_limit}")

# A neighbor hears it. Every node in range rebroadcasts, so the same packet arrives several
# times over; the source and sequence id decide which copy is the first.
received = parse(reading.bytes)
print(f"payload   {received.payload.decode()}")
seen = SeenPackets(64)
first = seen.record(received.src, received.id)
again = seen.record(received.src, received.id)
if first and not again:
    print("dedup     the first copy is relayed, and the second is dropped")

# Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
# frame without recomputing it and the check stays end to end.
forwarded = relayed(received.bytes)
onward = parse(forwarded.bytes)
print(
    f"relayed   hop limit {forwarded.hop_limit}, and the checksum still holds: "
    f"{onward.payload.decode()}"
)

# A frame that has run out of hops is not relayed again, which is what ends the flood.
if relayed(broadcast(RIVER_GAUGE, 1, b"level=high", hop_limit=0).bytes) is None:
    print("spent     at hop limit 0 the frame goes no further")

# A payload byte the air mangled fails the checksum rather than reaching the application as a
# plausible reading. The header is a fixed width, so the first byte past it is the first byte
# of the reading itself.
mangled = bytearray(reading.bytes)
mangled[HEADER_LEN] ^= 0xFF
try:
    parse(bytes(mangled))
    print("a mangled frame was accepted, which should never happen")
except PamojaError as error:
    print(f"mangled   rejected: {error}")
```
<!-- end -->

The flood down the valley, continuing from above:

<!-- snippet: bindings/python/guides/mesh.py#flood -->
From [`bindings/python/guides/mesh.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mesh.py):

```python
from collections import deque

from pamoja.mesh import DEFAULT_HOP_LIMIT, MAX_PAYLOAD

# Six nodes down a river valley, each in range of its neighbors only. The gauge at the top,
# node 0, floods a reading, and every node that hears a packet for the first time takes it
# and relays it while hops remain. Copies that come back up the valley are echoes, which the
# memory of each node drops.
nodes = 6
memory = [SeenPackets(64) for _ in range(nodes)]
flooded = broadcast(0, 1, b"level=high")
memory[0].record(flooded.src, flooded.id)
on_the_air = deque([(0, flooded.bytes)])
delivered = relays = echoes = farthest = 0
while on_the_air:
    sender, sent = on_the_air.popleft()
    for node in (sender - 1, sender + 1):
        if not 0 <= node < nodes:
            continue
        heard = parse(sent)
        if not memory[node].record(heard.src, heard.id):
            echoes += 1
            continue
        delivered += 1
        farthest = max(farthest, node)
        onward_frame = relayed(heard.bytes)
        if onward_frame is not None:
            relays += 1
            on_the_air.append((node, onward_frame.bytes))
print(f"flood     {delivered} nodes took the reading, {relays} relayed it, {echoes} echoes were dropped")
print(
    f"reach     node {farthest} was the farthest, {farthest} hops out, "
    f"and node {nodes - 1} never heard it"
)

# A node whose memory holds two packets hears the reading, then two packets from other nodes,
# then a late copy of the reading by a longer path. It has forgotten the reading by then and
# relays it again; on a busy mesh that repeats without end.
small = SeenPackets(2)
rain = broadcast(7, 1, b"rain=4mm")
wind = broadcast(8, 1, b"wind=12")
small.record(flooded.src, flooded.id)
small.record(rain.src, rain.id)
small.record(wind.src, wind.id)
if small.record(flooded.src, flooded.id):
    print("forgot    a memory of two packets relays the late copy again")

# A payload one byte past what a frame carries is refused before it is built. A frame is
# sized to the 250 bytes ESP-NOW carries, less the header and the checksum.
try:
    broadcast(0, 2, bytes(MAX_PAYLOAD + 1))
    print("an oversized payload was framed, which should never happen")
except PamojaError as error:
    print(f"too long  {MAX_PAYLOAD + 1} bytes refused: {error}")
```
<!-- end -->

## C#

In C#, the static `Mesh` class builds a frame with `Frame` or `BroadcastFrame`,
reads one with `Parse`, and relays it with `Relayed`, which returns `null` once the
hops run out; the limits are constants on it. A `MeshFrame` is a record of the
bytes and each field. `SeenPackets` is a node's memory, which holds native state
and is disposed with `using`. A frame that cannot be built or read throws
`PamojaException`.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs):

```csharp
// A river gauge floods a level reading to every node in range. The header is fixed
// and big-endian: version, source, destination, sequence id, hop limit, then the
// payload and a checksum over everything but the hop limit.
const uint RiverGauge = 305_419_896;
MeshFrame reading = Mesh.BroadcastFrame(RiverGauge, 1, "level=high"u8);
string to = reading.Dst == Mesh.Broadcast ? "every node in range" : "one node";
Console.WriteLine($"sent      {reading.Bytes.Length} bytes to {to}, hop limit {reading.HopLimit}");

// A neighbor hears it. Every node in range rebroadcasts, so the same packet
// arrives several times over; the source and sequence id decide which copy is
// the first.
MeshFrame received = Mesh.Parse(reading.Bytes);
Console.WriteLine($"payload   {Encoding.UTF8.GetString(received.Payload)}");
using SeenPackets seen = new(64);
bool first = seen.Record(received.Src, received.Id);
bool again = seen.Record(received.Src, received.Id);
if (first && !again)
{
    Console.WriteLine("dedup     the first copy is relayed, and the second is dropped");
}

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay
// forwards the frame without recomputing it and the check stays end to end.
MeshFrame forwarded = Mesh.Relayed(received.Bytes)!;
MeshFrame onward = Mesh.Parse(forwarded.Bytes);
Console.WriteLine(
    $"relayed   hop limit {forwarded.HopLimit}, and the checksum still holds: " +
    Encoding.UTF8.GetString(onward.Payload));

// A frame that has run out of hops is not relayed again, which is what ends the
// flood.
if (Mesh.Relayed(Mesh.BroadcastFrame(RiverGauge, 1, "level=high"u8, hopLimit: 0).Bytes) is null)
{
    Console.WriteLine("spent     at hop limit 0 the frame goes no further");
}

// A payload byte the air mangled fails the checksum rather than reaching the
// application as a plausible reading. The header is a fixed width, so the first
// byte past it is the first byte of the reading itself.
byte[] mangled = reading.Bytes.ToArray();
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

The flood down the valley, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs#flood -->
From [`bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MeshGuide.cs):

```csharp
// Six nodes down a river valley, each in range of its neighbors only. The gauge at
// the top, node 0, floods a reading, and every node that hears a packet for the
// first time takes it and relays it while hops remain. Copies that come back up the
// valley are echoes, which the memory of each node drops.
const int Nodes = 6;
SeenPackets[] memory = Enumerable.Range(0, Nodes).Select(_ => new SeenPackets(64)).ToArray();
MeshFrame flooded = Mesh.BroadcastFrame(0, 1, "level=high"u8);
memory[0].Record(flooded.Src, flooded.Id);
var onTheAir = new Queue<(int From, byte[] Bytes)>();
onTheAir.Enqueue((0, flooded.Bytes));
int delivered = 0;
int relays = 0;
int echoes = 0;
int farthest = 0;
while (onTheAir.TryDequeue(out var sent))
{
    foreach (int node in new[] { sent.From - 1, sent.From + 1 })
    {
        if (node < 0 || node >= Nodes)
        {
            continue;
        }

        MeshFrame heard = Mesh.Parse(sent.Bytes);
        if (!memory[node].Record(heard.Src, heard.Id))
        {
            echoes++;
            continue;
        }

        delivered++;
        farthest = Math.Max(farthest, node);
        if (Mesh.Relayed(heard.Bytes) is { } onwardFrame)
        {
            relays++;
            onTheAir.Enqueue((node, onwardFrame.Bytes));
        }
    }
}

foreach (SeenPackets cache in memory)
{
    cache.Dispose();
}

Console.WriteLine($"flood     {delivered} nodes took the reading, {relays} relayed it, {echoes} echoes were dropped");
Console.WriteLine(
    $"reach     node {farthest} was the farthest, {farthest} hops out, and node {Nodes - 1} never heard it");

// A node whose memory holds two packets hears the reading, then two packets from
// other nodes, then a late copy of the reading by a longer path. It has forgotten
// the reading by then and relays it again; on a busy mesh that repeats without end.
using SeenPackets small = new(2);
MeshFrame rain = Mesh.BroadcastFrame(7, 1, "rain=4mm"u8);
MeshFrame wind = Mesh.BroadcastFrame(8, 1, "wind=12"u8);
small.Record(flooded.Src, flooded.Id);
small.Record(rain.Src, rain.Id);
small.Record(wind.Src, wind.Id);
if (small.Record(flooded.Src, flooded.Id))
{
    Console.WriteLine("forgot    a memory of two packets relays the late copy again");
}

// A payload one byte past what a frame carries is refused before it is built. A
// frame is sized to the 250 bytes ESP-NOW carries, less the header and the checksum.
try
{
    Mesh.BroadcastFrame(0, 2, new byte[Mesh.MaxPayload + 1]);
    Console.WriteLine("an oversized payload was framed, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"too long  {Mesh.MaxPayload + 1} bytes refused: {error.Message}");
}
```
<!-- end -->

## Values at a glance

**A frame on the air,** big-endian:

| Bytes | Field | |
| --- | --- | --- |
| 0 | version | 1 |
| 1 to 4 | source node | the address of the node the packet started at |
| 5 to 8 | destination node | one node's address, or `BROADCAST`, 0xFFFFFFFF, for every node |
| 9 to 10 | sequence id | the source's number for the packet, rising with each one |
| 11 | hop limit | how many more relays the packet may take |
| 12 onward | payload | up to 236 bytes |
| last 2 | checksum | CRC-16 over every byte except the hop limit |

**The limits:**

| Limit | Value |
| --- | --- |
| Largest frame | 250 bytes, what one ESP-NOW frame carries |
| Header and checksum | 14 bytes |
| Largest payload | 236 bytes |
| Hop limit a new frame starts with | 3 |
| Radio hops a flood reaches | one more than the hop limit it starts with |
| Packets a node remembers, when nothing says otherwise | 64 |

**The checksum** is CRC-16/CCITT-FALSE: polynomial 0x1021, starting from 0xFFFF,
with no reflection and no final inversion. Over the nine characters `123456789` it
comes to 0x29B1, the check value every implementation of it prints. It skips the hop
limit, so a relay changes that byte without recomputing the checksum, and the check
holds from the source to every node the flood reaches.

**What a node does with a frame it hears:**

| The frame | The node |
| --- | --- |
| fails its checksum, or names another version | drops it |
| is a packet it has seen before | drops it as a duplicate |
| is new, with hops left | takes it, and relays a copy with one hop fewer |
| is new, with no hops left | takes it, and relays nothing |

**What a frame can be refused for:**

| Refusal | Means |
| --- | --- |
| mesh payload is larger than a single frame can carry | more than 236 bytes to send |
| mesh frame is shorter than its header and checksum | fewer than 14 bytes arrived |
| mesh frame is larger than the maximum frame size | more than 250 bytes arrived |
| unsupported mesh protocol version | a frame from a newer build |
| mesh CRC mismatch | the frame changed on the way |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| build a frame | `Frame::new(src, dst, id, payload)`, `Frame::broadcast(src, id, payload)`, `with_hop_limit(hops)` |
| read one | `Frame::parse(bytes)`, then `src()`, `dst()`, `id()`, `hop_limit()`, `payload()`, `is_broadcast()` |
| relay it | `frame.relayed()`, `None` once the hops run out |
| remember packets | `SeenCache::<64>::new()`, `DynamicSeenCache::new(capacity)`, `record(frame.dedup_key())`, `contains(key)` |
| check bytes | `crc16(data)`, `Crc16::new()`, `update(part)`, `finish()` |

### TypeScript

| To | Call |
| --- | --- |
| build a frame | `frame(src, dst, id, payload, hopLimit?)`, `broadcast(src, id, payload, hopLimit?)` |
| read one | `parse(bytes)`, then `src`, `dst`, `id`, `hopLimit`, `payload` |
| relay it | `relayed(bytes)`, `null` once the hops run out |
| remember packets | `new SeenPackets(capacity)`, `record(src, id)` |
| check bytes | `crc16(data)` |

### Python

| To | Call |
| --- | --- |
| build a frame | `frame(src, dst, id, payload, hop_limit=None)`, `broadcast(src, id, payload, hop_limit=None)` |
| read one | `parse(data)`, then `src`, `dst`, `id`, `hop_limit`, `payload` |
| relay it | `relayed(data)`, `None` once the hops run out |
| remember packets | `SeenPackets(capacity)`, `record(src, id)` |
| check bytes | `crc16(data)` |

### C#

| To | Call |
| --- | --- |
| build a frame | `Mesh.Frame(src, dst, id, payload, hopLimit)`, `Mesh.BroadcastFrame(src, id, payload, hopLimit)` |
| read one | `Mesh.Parse(bytes)`, then `Src`, `Dst`, `Id`, `HopLimit`, `Payload` |
| relay it | `Mesh.Relayed(bytes)`, `null` once the hops run out |
| remember packets | `new SeenPackets(capacity)`, `Record(src, id)` |
| check bytes | `Mesh.Crc16(data)` |

<!-- languages end -->

## When it goes wrong

A frame that cannot be read is refused with the reason, and nothing else in a mesh
reports an error: its mistakes show up as traffic. The ones that cost an afternoon:

- **The mesh drowns in its own traffic.** A node that relays without recording what
  it has seen, or with a memory of none, relays every copy, and each relay makes
  more. Record every packet a node hears before it relays anything.
- **A packet comes round again and again.** A node's memory is too small for the
  traffic, so it forgets a packet while copies of it are still in the air. Size it
  for every packet that can be in flight at once across the mesh, not for the node's
  own.
- **Nodes at the edge never hear.** A flood reaches one radio hop more than the hop
  limit it starts with. Raise the limit for a longer chain of nodes, rather than
  relaying without one.
- **The air is always busy.** Every node relays every new packet once, so a dense
  mesh spends its airtime on relays. Lower the hop limit where nodes are close
  together, and keep the payload short.
- **Every packet from one node is dropped as a duplicate.** A packet is known by its
  source and sequence id, so a node that restarts and counts from 1 again sends ids
  its neighbors still remember. Keep the sequence counting across restarts, or give
  the node a new address.
- **A payload is refused, or never arrives.** A frame holds 236 bytes of payload,
  sized to ESP-NOW. An nRF24 carries 32 bytes a packet, so on that radio a payload
  must stay at 18 bytes.
- **A forged reading passes the checksum.** A CRC catches what the air does, not what
  a person does: anyone can recompute it. Carry readings that matter in a
  [secured session](session.md) and use the mesh to move its frames.

## Where next

<!-- table: next mesh -->
- [Routing](routing.md): Reverse-path routing that learns the cheapest route from overheard traffic.
- [Store and forward](sync.md): Offline-first queues in memory or on disk, bounded or not, the on-disk one surviving power loss, and the drain that forwards them in order when a link returns.
- [LoRa radios](radios.md): The Semtech SX126x and SX127x LoRa radios and the SX1302 and SX1303 gateway concentrators.
- Also in Radio and reach: [LoRa airtime and range](lora.md), [LoRaWAN](lorawan.md), [LoRaWAN gateways](gateway.md).
<!-- end -->

## Reference

<!-- table: reference mesh -->
- Rust: [`pamoja-mesh`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mesh)
- TypeScript: [`@pamoja/mesh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mesh)
- Python: [`pamoja.mesh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mesh)
- C#: [`Pamoja.Mesh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mesh)
- Hardware: [ESP-NOW](https://pamoja.molex.cloud/docs/hardware.html#esp-now)
<!-- end -->
