# @pamoja/serial

SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/serial.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/serial
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/serial.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/serial.ts):

```typescript
import assert from 'node:assert/strict'

import { SlipDecoder, cobs, slip } from '@pamoja/serial'

// SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a payload
// carrying either goes out as the two-byte pair RFC 1055 fixes for it.
const payload = Buffer.from([0x01, 0xc0, 0xdb, 0x02])
const frame = slip.encode(payload)
assert.deepEqual([...frame], [0x01, 0xdb, 0xdc, 0xdb, 0xdd, 0x02, 0xc0])
assert.deepEqual(slip.decode(frame), payload)

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
// run led by its own length. This is the worked example from the COBS paper.
const packet = Buffer.from([0x11, 0x22, 0x00, 0x33])
const framed = cobs.encode(packet)
assert.deepEqual([...framed], [0x03, 0x11, 0x22, 0x02, 0x33, 0x00])
assert.deepEqual(cobs.decode(framed), packet)

// A serial read returns an arbitrary chunk rather than a packet. This one holds two
// frames with a truncated one between them, and the decoder drops only the bad frame.
const decoder = new SlipDecoder()
const frames = decoder.feed(Buffer.from([0x6f, 0x6b, 0xc0, 0xdb, 0xc0, 0x67, 0x6f, 0xc0]))
assert.deepEqual(frames, [Buffer.from('ok'), Buffer.from('go')])
assert.equal(decoder.discarded, 1)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-serial`](https://crates.io/crates/pamoja-serial) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html), [docs.rs](https://docs.rs/pamoja-serial) |
| TypeScript | [`@pamoja/serial`](https://www.npmjs.com/package/@pamoja/serial) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html) |
| Python | [`pamoja-serial`](https://pypi.org/project/pamoja-serial/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html) |
| C# | [`Pamoja.Serial`](https://www.nuget.org/packages/Pamoja.Serial) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html) |

## Documentation

- [`@pamoja/serial` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html), every class, function, and type this package exports.
- [The Serial framing guide](https://pamoja.molex.cloud/docs/guides/serial.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
