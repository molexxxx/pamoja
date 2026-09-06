# Modbus RTU

Modbus RTU is what a long RS485 run usually speaks: a one-byte unit address, a
function code, its data, and a CRC-16/MODBUS, with no framing bytes and no
escaping. pamoja builds those frames and reads them back. It does not own the
serial port, so the same code drives a USB adapter, a hat on a gateway, or a
test that never touches hardware.

## What the example does

It polls a power meter: builds the request for three holding registers, reads
the reply back, and turns the registers into a voltage, a current, and a fault
word. Then it corrupts a byte of the frame and confirms the checksum rejects it.
Modbus moves bare 16-bit registers and says nothing about what they hold, so the
unit address, the starting register and the scale on each value all come from
the meter's manual.

On a running gateway the reply arrives over RS485, so there is nothing to type.
The example builds it instead, with the same library that parses it:
`read_holding_registers_reply` returns exactly what a meter reporting those
values would send. It is the answering half of the request builder, which is
what lets a polling loop be written and tested with nothing on the line.
Everything after it is the gateway's own code.

It proves:

- A request for three holding registers is eight bytes on the wire: the unit
  address, the function code, the two 16-bit fields and the checksum.
- A reply validates its own checksum before any value is read out of it.
- In TypeScript, Python and C# the reply reports the unit address it was sent
  to and no exception, so a served request is not read as a refused one.
- The three 16-bit registers come back in the order the meter reported them.
- A corrupted byte is caught rather than passed on as a plausible reading.

## Rust

<!-- snippet: examples/tests/guides/modbus.rs#example -->
From [`examples/tests/guides/modbus.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/modbus.rs):

```rust
use pamoja_modbus::{Adu, Pdu};

// The device this gateway polls: a power meter at unit 17, whose manual says the
// three registers holding voltage, current and a fault word start at address 107.
const METER: u8 = 17;
const FIRST_REGISTER: u16 = 107;

// Ask it for those three registers. The frame is complete, checksum included, exactly
// as it goes out on the wire.
let request = Pdu::read_holding_registers(FIRST_REGISTER, 3).to_adu(METER);
let sent = request.as_bytes().len();
println!("polling unit {METER}, {sent} bytes out");

// A stand-in for the meter. On a running gateway this frame arrives over RS485; here
// the library builds what a meter reporting those three values would send back.
let from_the_meter = Pdu::read_holding_registers_reply(&[2301, 418, 0])
    .expect("three registers fit one reply")
    .to_adu(METER);

// Everything below is the gateway's own code. A reply carries its own checksum, so
// the frame is validated before any value is read out of it.
let reply = Adu::parse(from_the_meter.as_bytes()).expect("the checksum matches");
let registers: Vec<u16> = reply
    .response()
    .registers()
    .expect("a register reply")
    .collect();
let volts = f32::from(registers[0]) / 10.0;
let amps = f32::from(registers[1]) / 100.0;
println!("voltage   {volts:.1} V");
println!("current   {amps:.2} A");
println!("faults    {}", registers[2]);

// One flipped bit anywhere in the frame fails the checksum, which is the whole point
// of carrying one over a long RS485 run.
let mut mangled = from_the_meter.as_bytes().to_vec();
mangled[2] ^= 0xFF;
match Adu::parse(&mangled) {
    Ok(_) => println!("mangled frame accepted, which should never happen"),
    Err(error) => println!("mangled frame rejected: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/modbus.ts#example -->
From [`bindings/node/guides/modbus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/modbus.ts):

```typescript
import { parseFrame, readHoldingRegisters, readHoldingRegistersReply } from '@pamoja/modbus'

// The device this gateway polls: a power meter at unit 17, whose manual says the three
// registers holding voltage, current and a fault word start at address 107.
const METER = 17
const FIRST_REGISTER = 107

// Ask it for those three registers. The frame is complete, checksum included, exactly as
// it goes out on the wire.
const request = readHoldingRegisters(METER, FIRST_REGISTER, 3)
console.log(`polling unit ${METER}, ${request.length} bytes out`)

// A stand-in for the meter. On a running gateway this frame arrives over RS485; here the
// library builds what a meter reporting those three values would send back.
const fromTheMeter = readHoldingRegistersReply(METER, [2301, 418, 0])

// Everything below is the gateway's own code. A reply carries its own checksum, so the
// frame is validated before any value is read out of it.
const reply = parseFrame(fromTheMeter)
const registers = reply.registers()
console.log(`voltage   ${(registers[0] / 10).toFixed(1)} V`)
console.log(`current   ${(registers[1] / 100).toFixed(2)} A`)
console.log(`faults    ${registers[2]}`)

// One flipped bit anywhere in the frame fails the checksum, which is the whole point of
// carrying one over a long RS485 run.
const mangled = Buffer.from(fromTheMeter)
mangled[2] ^= 0xff
try {
  parseFrame(mangled)
  console.log('mangled frame accepted, which should never happen')
} catch (error) {
  console.log(`mangled frame rejected: ${(error as Error).message}`)
}
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/modbus.py#example -->
From [`bindings/python/guides/modbus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/modbus.py):

```python
from pamoja.core import PamojaError
from pamoja.modbus import parse_frame, read_holding_registers, read_holding_registers_reply

# The device this gateway polls: a power meter at unit 17, whose manual says the three
# registers holding voltage, current and a fault word start at address 107.
METER = 17
FIRST_REGISTER = 107

# Ask it for those three registers. The frame is complete, checksum included, exactly as
# it goes out on the wire.
request = read_holding_registers(METER, FIRST_REGISTER, 3)
print(f"polling unit {METER}, {len(request)} bytes out")

# A stand-in for the meter. On a running gateway this frame arrives over RS485; here the
# library builds what a meter reporting those three values would send back.
from_the_meter = read_holding_registers_reply(METER, [2301, 418, 0])

# Everything below is the gateway's own code. A reply carries its own checksum, so the
# frame is validated before any value is read out of it.
reply = parse_frame(from_the_meter)
registers = reply.registers()
print(f"voltage   {registers[0] / 10:.1f} V")
print(f"current   {registers[1] / 100:.2f} A")
print(f"faults    {registers[2]}")

# One flipped bit anywhere in the frame fails the checksum, which is the whole point of
# carrying one over a long RS485 run.
mangled = bytearray(from_the_meter)
mangled[2] ^= 0xFF
try:
    parse_frame(bytes(mangled))
    print("mangled frame accepted, which should never happen")
except PamojaError as error:
    print(f"mangled frame rejected: {error}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs):

```csharp
// The device this gateway polls: a power meter at unit 17, whose manual says the
// three registers holding voltage, current and a fault word start at address 107.
const byte Meter = 17;
const ushort FirstRegister = 107;

// Ask it for those three registers. The frame is complete, checksum included,
// exactly as it goes out on the wire.
byte[] request = Modbus.ReadHoldingRegisters(Meter, FirstRegister, 3);
Console.WriteLine($"polling unit {Meter}, {request.Length} bytes out");

// A stand-in for the meter. On a running gateway this frame arrives over RS485;
// here the library builds what a meter reporting those values would send back.
byte[] fromTheMeter = Modbus.ReadHoldingRegistersReply(Meter, [2301, 418, 0]);

// Everything below is the gateway's own code. A reply carries its own checksum,
// so the frame is validated before any value is read out of it.
ModbusFrame reply = Modbus.ParseFrame(fromTheMeter);
ushort[] registers = reply.Registers();
Console.WriteLine($"voltage   {registers[0] / 10.0:F1} V");
Console.WriteLine($"current   {registers[1] / 100.0:F2} A");
Console.WriteLine($"faults    {registers[2]}");

// One flipped bit anywhere in the frame fails the checksum, which is the whole
// point of carrying one over a long RS485 run.
byte[] mangled = [.. fromTheMeter];
mangled[2] ^= 0xFF;
try
{
    Modbus.ParseFrame(mangled);
    Console.WriteLine("mangled frame accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"mangled frame rejected: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference modbus -->
- Rust: [`pamoja-modbus`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-modbus)
- TypeScript: [`@pamoja/modbus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-modbus)
- Python: [`pamoja.modbus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-modbus)
- C#: [`Pamoja.Modbus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-modbus)
- Hardware: [Modbus RTU over RS-485](https://pamoja.molex.cloud/docs/hardware.html#modbus-rtu)
<!-- end -->
