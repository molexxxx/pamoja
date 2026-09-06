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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-modbus`](https://crates.io/crates/pamoja-modbus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_modbus/index.html), [docs.rs](https://docs.rs/pamoja-modbus), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-modbus) |
| TypeScript | [`@pamoja/modbus`](https://www.npmjs.com/package/@pamoja/modbus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-modbus) |
| Python | [`pamoja-modbus`](https://pypi.org/project/pamoja-modbus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/modbus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-modbus) |
| C# | [`Pamoja.Modbus`](https://www.nuget.org/packages/Pamoja.Modbus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Modbus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-modbus) |

## Documentation

- [`@pamoja/modbus` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_modbus.html), every class, function, and type this package exports.
- [The Modbus RTU guide](https://pamoja.molex.cloud/docs/guides/modbus.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
