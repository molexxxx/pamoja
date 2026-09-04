# CAN and J1939

CAN is the two-wire bus the moving parts of a machine talk over: motor
controllers, servo drives, battery management, and the engines, gensets, and
farm equipment that speak J1939 on top of it. pamoja builds the frames, reads
them back, and decodes the addressing J1939 packs into the identifier. The
controller hardware owns the wire itself: bit timing, arbitration, and the frame
CRC. What is left is the part an application reasons about, and it runs the same
against a USB adapter, a socket on a gateway, or a bus that is not there.

## What the example does

It decodes the two identifiers the standard fixes for an engine-speed broadcast
and for an addressed request, composes the second one back out of its fields,
then builds the classic and CAN-FD frames that carry that traffic.

It proves:

- The 29 bits of `0x0CF00400` split into priority 3, parameter group 61444, and
  a broadcast with no destination.
- A PDU format below `0xF0` is addressed, so `0x18EA2101` names destination
  `0x21` instead of extending the parameter group, and those same fields compose
  that identifier back.
- A standard 11-bit identifier decodes to nothing, because J1939 does not use
  one.
- Engine speed sits in bytes 4 and 5 of that parameter group at 0.125 rpm per
  bit, so the payload reads as 1000 rpm.
- The CAN-FD length encoding puts 32 bytes at data length code 13, while a
  classic frame still refuses a ninth byte.

## Rust

<!-- snippet: examples/tests/guides/can.rs#example -->
From [`examples/tests/guides/can.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/can.rs):

```rust
use pamoja_can::{CanId, Frame, J1939Id};

// The engine-speed broadcast a J1939 engine or genset puts on the bus. J1939 keeps its
// addressing in the identifier: a priority, a parameter group, a source address.
let engine = J1939Id::from_id(CanId::extended(0x0CF0_0400)).expect("an extended identifier");
assert_eq!(engine.priority(), 3);
assert_eq!(engine.pgn(), 61_444);
assert!(engine.is_broadcast() && engine.destination().is_none());

// A PDU format below 0xF0 is addressed rather than broadcast, so those eight bits hold a
// destination instead of extending the parameter group. 59904 is the request group.
let request = J1939Id::from_id(CanId::extended(0x18EA_2101)).expect("an extended identifier");
assert_eq!(request.pgn(), 59_904);
assert_eq!(request.destination(), Some(0x21));
let composed = J1939Id::from_parts(6, 59_904, 0x01, 0x21);
assert_eq!(composed.to_id().raw(), 0x18EA_2101);

// J1939 never rides an 11-bit identifier.
assert_eq!(J1939Id::from_id(CanId::standard(0x123)), None);

// The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
// parameter group, little-endian at 0.125 rpm per bit, so 0x1F40 reads as 1000 rpm.
let payload = [0xF0, 0x7D, 0x7D, 0x40, 0x1F, 0x00, 0xF0, 0xFF];
let eec1 = Frame::new(CanId::extended(0x0CF0_0400), &payload).expect("eight bytes fit");
assert_eq!(eec1.dlc(), 8);
let speed = u16::from_le_bytes([eec1.data()[3], eec1.data()[4]]);
assert_eq!(f64::from(speed) * 0.125, 1000.0);

// Above eight bytes CAN-FD encodes the length in steps, so 32 bytes is code 13, while a
// classic frame still refuses a ninth byte.
let wide = Frame::fd(CanId::extended(0x0CF0_0400), &[0; 32]).expect("a CAN-FD length");
assert_eq!(wide.dlc(), 13);
assert!(Frame::new(CanId::extended(0x0CF0_0400), &[0; 9]).is_err());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/can.ts#example -->
From [`bindings/node/guides/can.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/can.ts):

```typescript
import assert from 'node:assert/strict'

import { composeJ1939, decodeJ1939, fdFrame, frame } from '@pamoja/can'

// The engine-speed broadcast a J1939 engine or genset puts on the bus. J1939 keeps its
// addressing in the identifier: a priority, a parameter group, a source address.
const engine = decodeJ1939(0x0cf00400)!
assert.equal(engine.priority, 3)
assert.equal(engine.pgn, 61444)
assert.ok(engine.broadcast && engine.destination === null)

// A PDU format below 0xF0 is addressed rather than broadcast, so those eight bits hold a
// destination instead of extending the parameter group. 59904 is the request group.
const request = decodeJ1939(0x18ea2101)!
assert.equal(request.pgn, 59904)
assert.ok(request.destination === 0x21 && !request.broadcast)
assert.equal(composeJ1939(6, 59904, 0x01, 0x21), 0x18ea2101)

// J1939 never rides an 11-bit identifier.
assert.equal(decodeJ1939(0x123, false), null)

// The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
// parameter group, little-endian at 0.125 rpm per bit, so 0x1F40 reads as 1000 rpm.
const payload = Buffer.from([0xf0, 0x7d, 0x7d, 0x40, 0x1f, 0x00, 0xf0, 0xff])
const eec1 = frame(0x0cf00400, payload, true)
assert.equal(eec1.dlc, 8)
assert.equal(eec1.data.readUInt16LE(3) * 0.125, 1000)

// Above eight bytes CAN-FD encodes the length in steps, so 32 bytes is code 13, while a
// classic frame still refuses a ninth byte.
assert.equal(fdFrame(0x0cf00400, Buffer.alloc(32), true).dlc, 13)
assert.throws(() => frame(0x0cf00400, Buffer.alloc(9), true))
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/can.py#example -->
From [`bindings/python/guides/can.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/can.py):

```python
from pamoja.can import compose_j1939, decode_j1939, fd_frame, frame
from pamoja.core import PamojaError

# The engine-speed broadcast a J1939 engine or genset puts on the bus. J1939 keeps its
# addressing in the identifier: a priority, a parameter group, a source address.
engine = decode_j1939(0x0CF00400)
assert engine.priority == 3
assert engine.pgn == 61444
assert engine.broadcast and engine.destination is None

# A PDU format below 0xF0 is addressed rather than broadcast, so those eight bits hold a
# destination instead of extending the parameter group. 59904 is the request group.
request = decode_j1939(0x18EA2101)
assert request.pgn == 59904
assert request.destination == 0x21 and not request.broadcast
assert compose_j1939(6, 59904, 0x01, 0x21) == 0x18EA2101

# J1939 never rides an 11-bit identifier.
assert decode_j1939(0x123, extended=False) is None

# The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
# parameter group, little-endian at 0.125 rpm per bit, so 0x1F40 reads as 1000 rpm.
payload = bytes([0xF0, 0x7D, 0x7D, 0x40, 0x1F, 0x00, 0xF0, 0xFF])
eec1 = frame(0x0CF00400, payload, extended=True)
assert eec1.dlc == 8
assert int.from_bytes(eec1.data[3:5], "little") * 0.125 == 1000.0

# Above eight bytes CAN-FD encodes the length in steps, so 32 bytes is code 13, while a
# classic frame still refuses a ninth byte.
assert fd_frame(0x0CF00400, bytes(32), extended=True).dlc == 13
try:
    frame(0x0CF00400, bytes(9), extended=True)
except PamojaError:
    pass
else:
    raise AssertionError("classic CAN carries at most eight bytes")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs):

```csharp
// The engine-speed broadcast a J1939 engine or genset puts on the bus. J1939 keeps
// its addressing in the identifier: a priority, a parameter group, a source address.
J1939Message engine = Can.DecodeJ1939(0x0CF00400)!;
Expect(engine.Priority == 3, "the broadcast carries priority 3");
Expect(engine.Pgn == 61444, "engine speed is parameter group 61444");
Expect(engine.Broadcast && engine.Destination is null, "a broadcast has no destination");

// A PDU format below 0xF0 is addressed rather than broadcast, so those eight bits
// hold a destination instead of extending the parameter group. 59904 is the
// request group.
J1939Message request = Can.DecodeJ1939(0x18EA2101)!;
Expect(request.Pgn == 59904, "the request group decodes");
Expect(request.Destination == 0x21 && !request.Broadcast, "addressed to node 0x21");
Expect(Can.ComposeJ1939(6, 59904, 0x01, 0x21) == 0x18EA2101, "the fields compose back");

// J1939 never rides an 11-bit identifier.
Expect(Can.DecodeJ1939(0x123, extended: false) is null, "J1939 needs 29 bits");

// The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
// parameter group, little-endian at 0.125 rpm per bit, so 0x1F40 reads as 1000 rpm.
byte[] payload = [0xF0, 0x7D, 0x7D, 0x40, 0x1F, 0x00, 0xF0, 0xFF];
CanFrame eec1 = Can.Frame(0x0CF00400, payload, extended: true);
Expect(eec1.Dlc == 8, "eight bytes is data length code 8");
double rpm = BinaryPrimitives.ReadUInt16LittleEndian(eec1.Data.AsSpan(3, 2)) * 0.125;
Expect(rpm == 1000.0, "the payload reads as 1000 rpm");

// Above eight bytes CAN-FD encodes the length in steps, so 32 bytes is code 13,
// while a classic frame still refuses a ninth byte.
CanFrame wide = Can.FdFrame(0x0CF00400, new byte[32], extended: true);
Expect(wide.Dlc == 13, "32 bytes is data length code 13");
bool rejected = false;
try
{
    Can.Frame(0x0CF00400, new byte[9], extended: true);
}
catch (PamojaException)
{
    rejected = true;
}
Expect(rejected, "classic CAN carries at most eight bytes");
```
<!-- end -->

## Reference

<!-- table: reference can -->
- Rust: [`pamoja-can`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html)
- TypeScript: [`@pamoja/can`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html)
- Python: [`pamoja.can`](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html)
- C#: [`Pamoja.Can`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html)
<!-- end -->
