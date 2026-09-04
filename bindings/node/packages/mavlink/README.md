# @pamoja/mavlink

MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mavlink.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/mavlink
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/mavlink.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mavlink.ts):

```typescript
import assert from 'node:assert/strict'

import { MavlinkParser, MavlinkVersion, crc16, frame, knownCrcExtra } from '@pamoja/mavlink'

// 0x6F91 over "123456789" is the catalogue check value for CRC-16/MCRF4XX, and 50 is the
// CRC_EXTRA the common dialect publishes for HEARTBEAT.
assert.equal(crc16(Buffer.from('123456789')), 0x6f91)
assert.equal(knownCrcExtra(0), 50)

// A HEARTBEAT announcing an onboard controller in an active state. The v2 frame around it
// is the 0xFD marker, the payload length, two flag bytes, the sequence, the sending system
// and component, a 24-bit message id, the payload, then the checksum.
const heartbeat = Buffer.from([0, 0, 0, 0, 18, 0, 0, 4, 3])
const sent = frame({ systemId: 1, componentId: 1, sequence: 7 }, 0, heartbeat)
assert.deepEqual(
  [...sent.bytes],
  [
    0xfd, 0x09, 0x00, 0x00, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x12, 0x00, 0x00, 0x04, 0x03, 0x75, 0x3a,
  ],
)

// A link delivers bytes, not frames. The parser skips whatever does not start one and
// drops a frame whose checksum fails rather than passing it on.
const mangled = Buffer.from(sent.bytes)
mangled[14] ^= 0xff
const parser = new MavlinkParser()
assert.deepEqual(parser.push(Buffer.concat([Buffer.from([0x11, 0x22, 0x33]), mangled])), [])

// The same frame, split across two reads, still arrives whole.
assert.deepEqual(parser.push(sent.bytes.subarray(0, 5)), [])
const found = parser.push(sent.bytes.subarray(5))
assert.equal(found.length, 1)
assert.equal(found[0].version, MavlinkVersion.V2)
assert.equal(found[0].messageId, 0)
assert.deepEqual(found[0].payload, heartbeat)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mavlink`](https://crates.io/crates/pamoja-mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html), [docs.rs](https://docs.rs/pamoja-mavlink) |
| TypeScript | [`@pamoja/mavlink`](https://www.npmjs.com/package/@pamoja/mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html) |
| Python | [`pamoja-mavlink`](https://pypi.org/project/pamoja-mavlink/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html) |
| C# | [`Pamoja.Mavlink`](https://www.nuget.org/packages/Pamoja.Mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html) |

## Documentation

- [`@pamoja/mavlink` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html), every class, function, and type this package exports.
- [The MAVLink guide](https://pamoja.molex.cloud/docs/guides/mavlink.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
