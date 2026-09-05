# CAN and J1939

CAN is the two-wire bus the moving parts of a machine talk over: motor
controllers, servo drives, battery management, and the engines, gensets, and
farm equipment that speak J1939 on top of it. pamoja builds the frames, reads
them back, and decodes the addressing J1939 packs into the identifier. The
controller hardware owns the wire itself: bit timing, arbitration, and the frame
CRC. What is left is the part an application reasons about, and it runs the same
against a USB adapter, a socket on a gateway, or a bus that is not there.

## What the example does

It builds the engine-speed broadcast a genset controller puts on the bus and the
addressed request a gateway sends, reads both back, then carries the broadcast in
the classic and CAN-FD frames that would go on the wire.

The identifiers are composed from their fields rather than written out as packed
29-bit constants, so a reader sees the priority, the parameter group and the
node addresses that make one up. The payload is filled with the not-available
byte the standard reserves for a signal a controller is not reporting, and only
the two bytes that carry engine speed are written.

It proves:

- A priority, a parameter group and a source address compose an identifier and
  decode back out of it unchanged.
- A parameter group below the PDU1 limit is addressed rather than broadcast, so
  those eight bits carry a destination instead of extending the group number.
- A standard 11-bit identifier decodes to nothing, because J1939 does not use
  one.
- Engine speed sits in bytes 4 and 5 of that group at 0.125 rpm per bit, so the
  payload reads back as a thousand rpm.
- The CAN-FD length encoding puts 32 bytes at data length code 13, while a
  classic frame still refuses a ninth byte.

## Rust

<!-- snippet: examples/tests/guides/can.rs#example -->
From [`examples/tests/guides/can.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/can.rs):

```rust
use pamoja_can::{CanId, Frame, J1939Id};

// J1939 keeps its addressing inside the CAN identifier: a priority, a parameter group
// that says what the message is, and the address of whatever sent it. Building one
// from those fields is what saves a caller packing 29 bits by hand.
const ENGINE: u8 = 0x00;
const EEC1: u32 = 61_444; // electronic engine controller 1, which carries engine speed
let broadcast = J1939Id::from_parts(3, EEC1, ENGINE, 0xFF);
let (priority, group) = (broadcast.priority(), broadcast.pgn());
let to_one_node = !broadcast.is_broadcast();
println!("broadcast priority {priority} pgn {group}");
println!("addressed to one node: {to_one_node}");

// A parameter group below the PDU1 limit is addressed rather than broadcast, so those
// eight identifier bits carry a destination instead of extending the group number.
const REQUEST: u32 = 59_904;
const GATEWAY: u8 = 0x01;
const TRANSMISSION: u8 = 0x21;
let request = J1939Id::from_parts(6, REQUEST, GATEWAY, TRANSMISSION);
let asked_for = request.pgn();
println!("request   pgn {asked_for} to node {TRANSMISSION:#04X}");

// Reading one back off the bus is the same thing in reverse.
let heard = J1939Id::from_id(request.to_id()).expect("an extended identifier");
let from = heard.source();
let for_node = heard.destination().unwrap_or(0xFF);
println!("heard     from {from:#04X} for {for_node:#04X}");

// J1939 never rides an 11-bit identifier, so a standard frame is not one.
let eleven_bit = J1939Id::from_id(CanId::standard(0x123)).is_some();
println!("an 11-bit identifier is J1939: {eleven_bit}");

// The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
// parameter group at 0.125 rpm per bit, and every signal this controller is not
// reporting is filled with the not-available byte the standard reserves.
let mut payload = [0xFF; 8];
payload[3..5].copy_from_slice(&((1000.0 / 0.125) as u16).to_le_bytes());
let eec1 = Frame::new(broadcast.to_id(), &payload).expect("eight bytes fit");
let speed = u16::from_le_bytes([eec1.data()[3], eec1.data()[4]]);
let rpm = f64::from(speed) * 0.125;
let carried = eec1.dlc();
println!("engine    {rpm} rpm in {carried} bytes");

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
let wide = Frame::fd(broadcast.to_id(), &[0; 32]).expect("a CAN-FD length");
println!("32 bytes carries length code {}", wide.dlc());
match Frame::new(broadcast.to_id(), &[0; 9]) {
    Ok(_) => println!("a classic frame took nine bytes, which should never happen"),
    Err(error) => println!("classic   refused nine bytes: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/can.ts#example -->
From [`bindings/node/guides/can.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/can.ts):

```typescript
import { composeJ1939, decodeJ1939, fdFrame, frame } from '@pamoja/can'

// J1939 keeps its addressing inside the CAN identifier: a priority, a parameter group
// that says what the message is, and the address of whatever sent it. Building one from
// those fields is what saves a caller packing 29 bits by hand.
const ENGINE = 0x00
const EEC1 = 61_444 // electronic engine controller 1, which carries engine speed
const broadcast = composeJ1939(3, EEC1, ENGINE)
const engine = decodeJ1939(broadcast)!
console.log(`broadcast priority ${engine.priority} pgn ${engine.pgn}`)
console.log(`addressed to one node: ${!engine.broadcast}`)

// A parameter group below the PDU1 limit is addressed rather than broadcast, so those
// eight identifier bits carry a destination instead of extending the group number.
const REQUEST = 59_904
const GATEWAY = 0x01
const TRANSMISSION = 0x21
const request = decodeJ1939(composeJ1939(6, REQUEST, GATEWAY, TRANSMISSION))!
const hex = (value: number) => `0x${value.toString(16).toUpperCase().padStart(2, '0')}`
console.log(`request   pgn ${request.pgn} to node ${hex(request.destination!)}`)
console.log(`heard     from ${hex(request.source)}`)

// J1939 never rides an 11-bit identifier, so a standard frame is not one.
console.log(`an 11-bit identifier is J1939: ${decodeJ1939(0x123, false) !== null}`)

// The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
// parameter group at 0.125 rpm per bit, and every signal this controller is not
// reporting is filled with the not-available byte the standard reserves.
const payload = Buffer.alloc(8, 0xff)
payload.writeUInt16LE(1000 / 0.125, 3)
const eec1 = frame(broadcast, payload, true)
const speed = eec1.data.readUInt16LE(3) * 0.125
console.log(`engine    ${speed} rpm in ${eec1.dlc} bytes`)

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
console.log(`32 bytes carries length code ${fdFrame(broadcast, Buffer.alloc(32), true).dlc}`)
try {
  frame(broadcast, Buffer.alloc(9), true)
  console.log('a classic frame took nine bytes, which should never happen')
} catch (error) {
  console.log(`classic   refused nine bytes: ${(error as Error).message}`)
}
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/can.py#example -->
From [`bindings/python/guides/can.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/can.py):

```python
from pamoja.can import compose_j1939, decode_j1939, fd_frame, frame
from pamoja.core import PamojaError

# J1939 keeps its addressing inside the CAN identifier: a priority, a parameter group
# that says what the message is, and the address of whatever sent it. Building one from
# those fields is what saves a caller packing 29 bits by hand.
ENGINE = 0x00
EEC1 = 61_444  # electronic engine controller 1, which carries engine speed
broadcast = compose_j1939(3, EEC1, ENGINE)
engine = decode_j1939(broadcast)
print(f"broadcast priority {engine.priority} pgn {engine.pgn}")
print(f"addressed to one node: {not engine.broadcast}")

# A parameter group below the PDU1 limit is addressed rather than broadcast, so those
# eight identifier bits carry a destination instead of extending the group number.
REQUEST = 59_904
GATEWAY = 0x01
TRANSMISSION = 0x21
request = decode_j1939(compose_j1939(6, REQUEST, GATEWAY, TRANSMISSION))
print(f"request   pgn {request.pgn} to node 0x{request.destination:02X}")
print(f"heard     from 0x{request.source:02X}")

# J1939 never rides an 11-bit identifier, so a standard frame is not one.
print(f"an 11-bit identifier is J1939: {decode_j1939(0x123, extended=False) is not None}")

# The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
# parameter group at 0.125 rpm per bit, and every signal this controller is not
# reporting is filled with the not-available byte the standard reserves.
payload = bytearray([0xFF] * 8)
payload[3:5] = int(1000 / 0.125).to_bytes(2, "little")
eec1 = frame(broadcast, bytes(payload), extended=True)
speed = int.from_bytes(eec1.data[3:5], "little") * 0.125
print(f"engine    {speed} rpm in {eec1.dlc} bytes")

# Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
# classic frame still refuses a ninth byte.
print(f"32 bytes carries length code {fd_frame(broadcast, bytes(32), extended=True).dlc}")
try:
    frame(broadcast, bytes(9), extended=True)
    print("a classic frame took nine bytes, which should never happen")
except PamojaError as error:
    print(f"classic   refused nine bytes: {error}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs):

```csharp
// J1939 keeps its addressing inside the CAN identifier: a priority, a parameter
// group that says what the message is, and the address of whatever sent it.
// Building one from those fields saves a caller packing 29 bits by hand.
const byte Engine = 0x00;
const uint Eec1 = 61_444; // electronic engine controller 1, which carries speed
uint broadcast = Can.ComposeJ1939(3, Eec1, Engine);
J1939Message engine = Can.DecodeJ1939(broadcast)!;
Console.WriteLine($"broadcast priority {engine.Priority} pgn {engine.Pgn}");
Console.WriteLine($"addressed to one node: {!engine.Broadcast}");

// A parameter group below the PDU1 limit is addressed rather than broadcast, so
// those eight identifier bits carry a destination instead of extending the group.
const uint Request = 59_904;
const byte Gateway = 0x01;
const byte Transmission = 0x21;
J1939Message request = Can.DecodeJ1939(
    Can.ComposeJ1939(6, Request, Gateway, Transmission))!;
Console.WriteLine($"request   pgn {request.Pgn} to node 0x{request.Destination:X2}");
Console.WriteLine($"heard     from 0x{request.Source:X2}");

// J1939 never rides an 11-bit identifier, so a standard frame is not one.
Console.WriteLine(
    $"an 11-bit identifier is J1939: {Can.DecodeJ1939(0x123, extended: false) is not null}");

// The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of
// that parameter group at 0.125 rpm per bit, and every signal this controller is
// not reporting is filled with the not-available byte the standard reserves.
byte[] payload = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
BitConverter.TryWriteBytes(payload.AsSpan(3), (ushort)(1000 / 0.125));
CanFrame eec1 = Can.Frame(broadcast, payload, extended: true);
double speed = BitConverter.ToUInt16(eec1.Data, 3) * 0.125;
Console.WriteLine($"engine    {speed} rpm in {eec1.Dlc} bytes");

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
Console.WriteLine(
    $"32 bytes carries length code {Can.FdFrame(broadcast, new byte[32], true).Dlc}");
try
{
    Can.Frame(broadcast, new byte[9], extended: true);
    Console.WriteLine("a classic frame took nine bytes, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"classic   refused nine bytes: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference can -->
- Rust: [`pamoja-can`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html)
- TypeScript: [`@pamoja/can`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html)
- Python: [`pamoja.can`](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html)
- C#: [`Pamoja.Can`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html)
<!-- end -->
