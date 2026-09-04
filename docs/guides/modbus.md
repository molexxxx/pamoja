# Modbus RTU

Modbus RTU is what a long RS485 run usually speaks: a one-byte unit address, a
function code, its data, and a CRC-16/MODBUS, with no framing bytes and no
escaping. pamoja builds those frames and reads them back. It does not own the
serial port, so the same code drives a USB adapter, a hat on a gateway, or a
test that never touches hardware.

## What the example does

It builds a read-holding-registers request for unit `0x11` and checks it against
the exact bytes the specification fixes, then parses the reply that request
draws and reads the three registers out of it. Finally it flips one bit in the
frame and confirms the checksum rejects it.

It proves:

- The request is byte-for-byte the frame in the specification, checksum
  included, so an implementation that is wrong but self-consistent still fails.
- A reply validates its own checksum before any value is read from it.
- The three 16-bit registers decode to `0x022B`, `0x0000`, and `0x0064`.
- A single flipped bit is caught rather than passed on as a plausible reading.

## Rust

<!-- snippet: examples/tests/guides/modbus.rs#example -->
From [`examples/tests/guides/modbus.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/modbus.rs):

```rust
use pamoja_modbus::{Adu, Pdu};

// Ask unit 0x11 for three holding registers starting at 0x006B. The last two bytes are
// the CRC-16/MODBUS, so this is the frame exactly as it goes out on the wire.
let request = Pdu::read_holding_registers(0x006B, 3).to_adu(0x11);
assert_eq!(
    request.as_bytes(),
    &[0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87]
);

// The device answers with three 16-bit registers. A reply carries its own checksum, so
// the receiver validates the frame before reading any value out of it.
let reply = Adu::from_pdu(0x11, &[0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64])
    .expect("a well-formed reply");
let parsed = Adu::parse(reply.as_bytes()).expect("the checksum matches");
let registers: Vec<u16> = parsed
    .response()
    .registers()
    .expect("a register reply")
    .collect();
assert_eq!(registers, [0x022B, 0x0000, 0x0064]);

// One flipped bit anywhere in the frame fails the checksum, which is the whole point of
// carrying one over a long RS485 run.
let mut corrupt = reply.as_bytes().to_vec();
corrupt[2] ^= 0xFF;
assert!(Adu::parse(&corrupt).is_err());
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/modbus.ts#example -->
From [`bindings/node/guides/modbus.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/modbus.ts):

```typescript
import assert from 'node:assert/strict'

import { crc16, parseFrame, readHoldingRegisters } from '@pamoja/modbus'

// Ask unit 0x11 for three holding registers starting at 0x006B. The last two bytes are
// the CRC-16/MODBUS, so this is the frame exactly as it goes out on the wire.
const request = readHoldingRegisters(0x11, 0x006b, 3)
assert.deepEqual([...request], [0x11, 0x03, 0x00, 0x6b, 0x00, 0x03, 0x76, 0x87])

// The device answers with three 16-bit registers. A reply carries its own checksum, so
// the receiver validates the frame before reading any value out of it.
const body = Buffer.from([0x11, 0x03, 0x06, 0x02, 0x2b, 0x00, 0x00, 0x00, 0x64])
const checksum = Buffer.alloc(2)
checksum.writeUInt16LE(crc16(body))
const reply = parseFrame(Buffer.concat([body, checksum]))
assert.equal(reply.address, 0x11)
assert.equal(reply.exception, null)
assert.deepEqual(reply.registers(), [0x022b, 0x0000, 0x0064])

// One flipped bit anywhere in the frame fails the checksum, which is the whole point of
// carrying one over a long RS485 run.
const corrupt = Buffer.concat([body, checksum])
corrupt[2] ^= 0xff
assert.throws(() => parseFrame(corrupt))
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/modbus.py#example -->
From [`bindings/python/guides/modbus.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/modbus.py):

```python
from pamoja.core import PamojaError
from pamoja.modbus import crc16, parse_frame, read_holding_registers

# Ask unit 0x11 for three holding registers starting at 0x006B. The last two bytes are
# the CRC-16/MODBUS, so this is the frame exactly as it goes out on the wire.
request = read_holding_registers(0x11, 0x006B, 3)
assert request == bytes([0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87])

# The device answers with three 16-bit registers. A reply carries its own checksum, so
# the receiver validates the frame before reading any value out of it.
body = bytes([0x11, 0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64])
reply = parse_frame(body + crc16(body).to_bytes(2, "little"))
assert reply.address == 0x11
assert reply.exception is None
assert reply.registers() == [0x022B, 0x0000, 0x0064]

# One flipped bit anywhere in the frame fails the checksum, which is the whole point of
# carrying one over a long RS485 run.
corrupt = bytearray(body + crc16(body).to_bytes(2, "little"))
corrupt[2] ^= 0xFF
try:
    parse_frame(bytes(corrupt))
except PamojaError:
    pass
else:
    raise AssertionError("a frame mangled on the wire should be rejected")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ModbusGuide.cs):

```csharp
// Ask unit 0x11 for three holding registers starting at 0x006B. The last two bytes
// are the CRC-16/MODBUS, so this is the frame exactly as it goes out on the wire.
byte[] request = Modbus.ReadHoldingRegisters(0x11, 0x006B, 3);
Expect(
    request.SequenceEqual(new byte[] { 0x11, 0x03, 0x00, 0x6B, 0x00, 0x03, 0x76, 0x87 }),
    "the request is the frame the specification fixes");

// The device answers with three 16-bit registers. A reply carries its own checksum,
// so the receiver validates the frame before reading any value out of it.
byte[] body = [0x11, 0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64];
ushort checksum = Modbus.Crc16(body);
byte[] wire = [.. body, (byte)(checksum & 0xFF), (byte)(checksum >> 8)];
using ModbusFrame reply = Modbus.ParseFrame(wire);
Expect(reply.Address == 0x11, "the reply comes from the unit that was asked");
Expect(reply.Exception is null, "a served request reports no exception");
Expect(
    reply.Registers().SequenceEqual(new ushort[] { 0x022B, 0x0000, 0x0064 }),
    "the three registers read back");

// One flipped bit anywhere in the frame fails the checksum, which is the whole
// point of carrying one over a long RS485 run.
byte[] corrupt = [.. wire];
corrupt[2] ^= 0xFF;
bool rejected = false;
try
{
    using ModbusFrame _ = Modbus.ParseFrame(corrupt);
}
catch (PamojaException)
{
    rejected = true;
}
Expect(rejected, "a frame mangled on the wire is rejected");
```
<!-- end -->

## Reference

<!-- table: reference modbus -->
- Rust: [`pamoja-modbus`](https://docs.rs/pamoja-modbus) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html))
- TypeScript: [`@pamoja/modbus`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html)
- Python: [`pamoja.modbus`](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html)
- C#: [`Modbus`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.Modbus.html), [`ModbusFrame`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.ModbusFrame.html)
<!-- end -->
