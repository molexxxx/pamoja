# @pamoja/can

CAN 2.0 and CAN-FD frames with 11- and 29-bit identifiers, plus J1939 decode and compose. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
npm install @pamoja/can
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-can`](https://crates.io/crates/pamoja-can) | [docs.rs](https://docs.rs/pamoja-can), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html) |
| TypeScript | [`@pamoja/can`](https://www.npmjs.com/package/@pamoja/can) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html) |
| Python | [`pamoja-can`](https://pypi.org/project/pamoja-can/) | [`pamoja.can`](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html) |
| C# | [`Pamoja.Can`](https://www.nuget.org/packages/Pamoja.Can) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.Can.html) |

## Documentation

- [The CAN and J1939 guide](https://pamoja.molex.cloud/docs/guides/can.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
