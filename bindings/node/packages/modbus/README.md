# @pamoja/modbus

Modbus RTU requests and replies with CRC-16/MODBUS for RS485 field devices. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/modbus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/modbus
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-modbus`](https://crates.io/crates/pamoja-modbus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [docs.rs](https://docs.rs/pamoja-modbus) |
| TypeScript | [`@pamoja/modbus`](https://www.npmjs.com/package/@pamoja/modbus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html) |
| Python | [`pamoja-modbus`](https://pypi.org/project/pamoja-modbus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html) |
| C# | [`Pamoja.Modbus`](https://www.nuget.org/packages/Pamoja.Modbus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html) |

## Documentation

- [`@pamoja/modbus` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html), every class, function, and type this package exports.
- [The Modbus RTU guide](https://pamoja.molex.cloud/docs/guides/modbus.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
