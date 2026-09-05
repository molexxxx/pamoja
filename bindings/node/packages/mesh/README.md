# @pamoja/mesh

Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mesh.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/mesh
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/mesh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mesh.ts):

```typescript
import { BROADCAST, HEADER_LEN, SeenPackets, broadcast, parse, relayed } from '@pamoja/mesh'

// A river gauge floods a level reading to every node in range. The header is fixed and
// big-endian: version, source, destination, sequence id, hop limit, then the payload and
// a checksum over everything but the hop limit.
const RIVER_GAUGE = 305419896
const reading = broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'))
console.log(`sent      ${reading.bytes.length} bytes to every node in range`)
console.log(`addressed to broadcast: ${reading.dst === BROADCAST}`)

// A neighbour hears it. Every node in range rebroadcasts, so the same packet arrives
// several times over; the source and sequence id decide which copy is the first.
const received = parse(reading.bytes)
console.log(`payload   ${received.payload.toString()}`)

const seen = new SeenPackets(64)
const first = seen.record(received.src, received.id)
const again = seen.record(received.src, received.id)
console.log(`first copy relayed: ${first}, second copy relayed: ${again}`)

// Relaying spends one hop. The checksum skips the hop-limit byte, so a relay forwards the
// frame without recomputing it and the check stays end to end.
const forwarded = relayed(received.bytes)!
console.log(`relayed   hop limit ${forwarded.hopLimit}`)
const onward = parse(forwarded.bytes)
console.log(`onward    ${onward.payload.toString()}`)

// A frame that has run out of hops is not relayed again, which is what ends the flood.
const spent = relayed(broadcast(RIVER_GAUGE, 1, Buffer.from('level=high'), 0).bytes)
if (spent === null) {
  console.log('spent     hop limit reached, the flood stops here')
} else {
  console.log('a spent frame was relayed, which should never happen')
}

// A payload byte the air mangled fails the checksum rather than reaching the application
// as a plausible reading. The header is a fixed width, so the first byte past it is the
// first byte of the reading itself.
const mangled = Buffer.from(reading.bytes)
mangled[HEADER_LEN] ^= 0xff
try {
  parse(mangled)
  console.log('a mangled frame was accepted, which should never happen')
} catch (error) {
  console.log(`mangled   rejected: ${(error as Error).message}`)
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mesh`](https://crates.io/crates/pamoja-mesh) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mesh/index.html), [docs.rs](https://docs.rs/pamoja-mesh) |
| TypeScript | [`@pamoja/mesh`](https://www.npmjs.com/package/@pamoja/mesh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html) |
| Python | [`pamoja-mesh`](https://pypi.org/project/pamoja-mesh/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mesh.html) |
| C# | [`Pamoja.Mesh`](https://www.nuget.org/packages/Pamoja.Mesh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mesh.html) |

## Documentation

- [`@pamoja/mesh` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mesh.html), every class, function, and type this package exports.
- [The Mesh frames guide](https://pamoja.molex.cloud/docs/guides/mesh.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
