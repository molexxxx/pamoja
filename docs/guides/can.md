# CAN and J1939

CAN is the two-wire bus the moving parts of a machine talk over: motor
controllers, servo drives, battery management, and the engines, gensets, and
farm equipment that speak J1939 on top of it. pamoja builds the frames, reads
them back, and decodes the addressing J1939 packs into the identifier. The
controller hardware owns the wire itself: bit timing, arbitration, and the frame
CRC. What is left is the part an application reasons about, and it runs the same
against a USB adapter, a socket on a gateway, or a bus that is not there.

## What the example does

It builds the engine-speed broadcast an engine controller puts on the bus and the
addressed request a gateway sends a gearbox, reads the request back out of the
identifier it packs into, then carries the broadcast in the classic frame that
would go on the wire. A wider frame on that same identifier shows how CAN-FD
encodes a length above eight bytes.

The identifiers are composed from their fields rather than written out as packed
29-bit constants, so a reader sees the priority, the parameter group and the node
addresses that make one up. A broadcast is composed without a destination rather
than with an address that stands for nobody. The payload starts as eight
not-available bytes, the value the standard reserves for a signal a controller is
not reporting, and only the two that carry engine speed are written.

It proves:

- A priority, a parameter group and a source address compose an identifier and
  decode back out of it unchanged.
- The broadcast carries no destination, while a parameter group below the PDU1
  limit is addressed, so those eight bits name a node instead of extending the
  group number.
- A standard 11-bit identifier decodes to nothing, because J1939 does not use
  one.
- Engine speed sits in bytes 4 and 5 of that group at 0.125 rpm per bit, so the
  eight-byte payload reads back as a thousand rpm.
- The CAN-FD length encoding puts 32 bytes at data length code 13, while a
  classic frame still refuses a ninth byte.

## Rust

<!-- snippet: examples/tests/guides/can.rs#example -->
From [`examples/tests/guides/can.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/can.rs):

```rust
use pamoja_can::{priority, CanId, Frame, J1939Id, Signals};

// The nodes on this bus, by the address each answers to, and the two parameter groups
// in play. J1939 publishes both, so naming them is what makes the traffic readable.
const ENGINE: u8 = 0;
const GATEWAY: u8 = 1;
const GEARBOX: u8 = 33;
const ENGINE_CONTROLLER_1: u32 = 61_444; // carries engine speed
const REQUEST: u32 = 59_904; // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for it.
// Naming both is what stops a sender and a receiver disagreeing about either.
const ENGINE_SPEED_AT: usize = 3;
const RPM_PER_BIT: f64 = 0.125;

// J1939 keeps its addressing inside the CAN identifier: a priority, the parameter
// group, and the address of whatever sent it. A broadcast has no destination, so it
// is its own constructor rather than a magic address a caller has to know.
let speed_id = J1939Id::broadcast(priority::CONTROL, ENGINE_CONTROLLER_1, ENGINE);
let (group, sent_at) = (speed_id.pgn(), speed_id.priority());
println!("broadcast pgn {group} at priority {sent_at}");

// A parameter group below the PDU1 limit is addressed rather than broadcast, so those
// eight identifier bits carry a destination instead of extending the group number.
let request_id = J1939Id::from_parts(priority::DEFAULT, REQUEST, GATEWAY, GEARBOX);
let asked_for = request_id.pgn();
println!("request   pgn {asked_for} addressed to node {GEARBOX}");

// Reading one back off the bus is the same thing in reverse, so a receiver never
// unpacks 29 bits by hand.
let heard = J1939Id::from_id(request_id.to_id()).expect("an extended identifier");
let (from, to) = (heard.source(), heard.destination().unwrap());
println!("heard     from node {from} for node {to}");

// The payload. Every signal starts marked not available, and this controller reports
// only engine speed, so that is the only one it writes.
let mut reported = Signals::new();
reported.set_u16(ENGINE_SPEED_AT, (1000.0 / RPM_PER_BIT) as u16);
let frame = Frame::new(speed_id.to_id(), reported.as_bytes()).expect("eight bytes fit");

// The receiving node reads the same offset back, so neither end slices the payload.
let signals = frame.signals().expect("a J1939 frame carries eight bytes");
let rpm = f64::from(signals.u16(ENGINE_SPEED_AT).expect("engine speed")) * RPM_PER_BIT;
println!("engine    {rpm} rpm, carried in {} bytes", frame.dlc());

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
let wide = Frame::fd(speed_id.to_id(), &[0; 32]).expect("a CAN-FD length");
println!("32 bytes carries length code {}", wide.dlc());
match Frame::new(speed_id.to_id(), &[0; 9]) {
    Ok(_) => println!("a classic frame took nine bytes, which should never happen"),
    Err(error) => println!("classic   refused nine bytes: {error}"),
}

// J1939 never rides an 11-bit identifier, so a standard frame is not one of its
// messages however its bits happen to line up.
let short_id = J1939Id::from_id(CanId::standard(291));
println!("an 11-bit identifier is J1939: {}", short_id.is_some());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/can.ts#example -->
From [`bindings/node/guides/can.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/can.ts):

```typescript
import {
  NOT_AVAILABLE,
  broadcastJ1939,
  composeJ1939,
  decodeJ1939,
  fdFrame,
  frame,
  priority,
  signals,
  signalsFrom,
} from '@pamoja/can'

// The nodes on this bus, by the address each answers to, and the two parameter groups
// in play. J1939 publishes both, so naming them is what makes the traffic readable.
const ENGINE = 0
const GATEWAY = 1
const GEARBOX = 33
const ENGINE_CONTROLLER_1 = 61_444 // carries engine speed
const REQUEST = 59_904 // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for it.
// Naming both is what stops a sender and a receiver disagreeing about either.
const ENGINE_SPEED_AT = 3
const RPM_PER_BIT = 0.125

// J1939 keeps its addressing inside the CAN identifier: a priority, the parameter
// group, and the address of whatever sent it. A broadcast has no destination, so it is
// its own constructor rather than a magic address a caller has to know.
const speedId = broadcastJ1939(priority.control, ENGINE_CONTROLLER_1, ENGINE)
const speed = decodeJ1939(speedId)!
console.log(`broadcast pgn ${speed.pgn} at priority ${speed.priority}`)

// A parameter group below the PDU1 limit is addressed rather than broadcast, so those
// eight identifier bits carry a destination instead of extending the group number.
const requestId = composeJ1939(priority.default, REQUEST, GATEWAY, GEARBOX)
console.log(`request   pgn ${decodeJ1939(requestId)!.pgn} addressed to node ${GEARBOX}`)

// Reading one back off the bus is the same thing in reverse, so a receiver never
// unpacks 29 bits by hand.
const heard = decodeJ1939(requestId)!
console.log(`heard     from node ${heard.source} for node ${heard.destination}`)

// The payload. Every signal starts marked not available, and this controller reports
// only engine speed, so that is the only one it writes.
const reported = signals()
reported.setU16(ENGINE_SPEED_AT, 1000 / RPM_PER_BIT)
const eec1 = frame(speedId, reported.bytes, true)

// The receiving node reads the same offset back, so neither end slices the payload.
const rpm = signalsFrom(eec1.data).u16(ENGINE_SPEED_AT)! * RPM_PER_BIT
console.log(`engine    ${rpm} rpm, carried in ${eec1.dlc} bytes`)

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
console.log(`32 bytes carries length code ${fdFrame(speedId, new Uint8Array(32), true).dlc}`)
try {
  frame(speedId, new Uint8Array(9), true)
  console.log('a classic frame took nine bytes, which should never happen')
} catch (error) {
  console.log(`classic   refused nine bytes: ${(error as Error).message}`)
}

// J1939 never rides an 11-bit identifier, so a standard frame is not one of its
// messages however its bits happen to line up.
console.log(`an 11-bit identifier is J1939: ${decodeJ1939(291, false) !== null}`)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/can.py#example -->
From [`bindings/python/guides/can.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/can.py):

```python
from pamoja.can import (
    NOT_AVAILABLE,
    Priority,
    broadcast_j1939,
    compose_j1939,
    decode_j1939,
    fd_frame,
    frame,
    signals,
    signals_from,
)
from pamoja.core import PamojaError

# The nodes on this bus, by the address each answers to, and the two parameter groups
# in play. J1939 publishes both, so naming them is what makes the traffic readable.
ENGINE = 0
GATEWAY = 1
GEARBOX = 33
ENGINE_CONTROLLER_1 = 61_444  # carries engine speed
REQUEST = 59_904  # asks another node for a parameter group

# Where engine speed sits inside that group, and the scale the standard fixes for it.
# Naming both is what stops a sender and a receiver disagreeing about either.
ENGINE_SPEED_AT = 3
RPM_PER_BIT = 0.125

# J1939 keeps its addressing inside the CAN identifier: a priority, the parameter
# group, and the address of whatever sent it. A broadcast has no destination, so it is
# its own constructor rather than a magic address a caller has to know.
speed_id = broadcast_j1939(Priority.CONTROL, ENGINE_CONTROLLER_1, ENGINE)
speed = decode_j1939(speed_id)
print(f"broadcast pgn {speed.pgn} at priority {speed.priority}")

# A parameter group below the PDU1 limit is addressed rather than broadcast, so those
# eight identifier bits carry a destination instead of extending the group number.
request_id = compose_j1939(Priority.DEFAULT, REQUEST, GATEWAY, GEARBOX)
print(f"request   pgn {decode_j1939(request_id).pgn} addressed to node {GEARBOX}")

# Reading one back off the bus is the same thing in reverse, so a receiver never
# unpacks 29 bits by hand.
heard = decode_j1939(request_id)
print(f"heard     from node {heard.source} for node {heard.destination}")

# The payload. Every signal starts marked not available, and this controller reports
# only engine speed, so that is the only one it writes.
reported = signals()
reported.set_u16(ENGINE_SPEED_AT, int(1000 / RPM_PER_BIT))
eec1 = frame(speed_id, reported.bytes, extended=True)

# The receiving node reads the same offset back, so neither end slices the payload.
rpm = signals_from(eec1.data).u16(ENGINE_SPEED_AT) * RPM_PER_BIT
print(f"engine    {rpm} rpm, carried in {eec1.dlc} bytes")

# Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
# classic frame still refuses a ninth byte.
print(f"32 bytes carries length code {fd_frame(speed_id, bytes(32), extended=True).dlc}")
try:
    frame(speed_id, bytes(9), extended=True)
    print("a classic frame took nine bytes, which should never happen")
except PamojaError as error:
    print(f"classic   refused nine bytes: {error}")

# J1939 never rides an 11-bit identifier, so a standard frame is not one of its
# messages however its bits happen to line up.
print(f"an 11-bit identifier is J1939: {decode_j1939(291, extended=False) is not None}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs):

```csharp
// The nodes on this bus, by the address each answers to, and the two parameter
// groups in play. J1939 publishes both, so naming them makes the traffic readable.
const byte Engine = 0;
const byte Gateway = 1;
const byte Gearbox = 33;
const uint EngineController1 = 61_444; // carries engine speed
const uint Request = 59_904; // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for
// it. Naming both is what stops a sender and a receiver disagreeing about either.
const int EngineSpeedAt = 3;
const double RpmPerBit = 0.125;

// J1939 keeps its addressing inside the CAN identifier: a priority, the parameter
// group, and the address of whatever sent it. A broadcast has no destination, so
// it is its own constructor rather than a magic address a caller has to know.
uint speedId = Can.BroadcastJ1939(J1939Priority.Control, EngineController1, Engine);
J1939Message speed = Can.DecodeJ1939(speedId)!;
Console.WriteLine($"broadcast pgn {speed.Pgn} at priority {speed.Priority}");

// A parameter group below the PDU1 limit is addressed rather than broadcast, so
// those eight identifier bits carry a destination instead of extending the group.
uint requestId = Can.ComposeJ1939((byte)J1939Priority.Normal, Request, Gateway, Gearbox);
Console.WriteLine($"request   pgn {Request} addressed to node {Gearbox}");

// Reading one back off the bus is the same thing in reverse, so a receiver never
// unpacks 29 bits by hand.
J1939Message heard = Can.DecodeJ1939(requestId)!;
Console.WriteLine($"heard     from node {heard.Source} for node {heard.Destination}");

// The payload. Every signal starts marked not available, and this controller
// reports only engine speed, so that is the only one it writes.
Signals reported = Signals.New();
reported.SetU16(EngineSpeedAt, (ushort)(1000 / RpmPerBit));
CanFrame eec1 = Can.Frame(speedId, reported.ToArray(), extended: true);

// The receiving node reads the same offset back, so neither end slices the payload.
double rpm = Signals.From(eec1.Data).U16(EngineSpeedAt)!.Value * RpmPerBit;
Console.WriteLine($"engine    {rpm} rpm, carried in {eec1.Dlc} bytes");

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
CanFrame wide = Can.FdFrame(speedId, new byte[32], extended: true);
Console.WriteLine($"32 bytes carries length code {wide.Dlc}");
try
{
    Can.Frame(speedId, new byte[9], extended: true);
    Console.WriteLine("a classic frame took nine bytes, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"classic   refused nine bytes: {error.Message}");
}

// J1939 never rides an 11-bit identifier, so a standard frame is not one of its
// messages however its bits happen to line up.
Console.WriteLine($"an 11-bit identifier is J1939: {Can.DecodeJ1939(291, false) is not null}");
```
<!-- end -->

## Reference

<!-- table: reference can -->
- Rust: [`pamoja-can`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-can)
- TypeScript: [`@pamoja/can`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-can)
- Python: [`pamoja.can`](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-can)
- C#: [`Pamoja.Can`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-can)
- Hardware: [CAN 2.0 and CAN FD](https://pamoja.molex.cloud/docs/hardware.html#can), [SAE J1939](https://pamoja.molex.cloud/docs/hardware.html#j1939)
<!-- end -->
