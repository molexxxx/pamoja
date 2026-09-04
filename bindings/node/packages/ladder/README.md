# @pamoja/ladder

Cheapest reachable link first, buffering to a store when every link is down. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/ladder.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/ladder
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/ladder.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ladder.ts):

```typescript
import assert from 'node:assert/strict'

import { Transport } from '@pamoja/core'
import { Delivery, Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Store } from '@pamoja/sync'

async function main() {
  // Two links off the same node: a near mesh hop and a metered backhaul. Each is a
  // separate broker, so which one carried a reading is visible from its subscriber.
  const mesh = new LoopbackBroker()
  const backhaul = new LoopbackBroker()
  const gateway = backhaul.link()
  await gateway.connect()
  await gateway.subscribe('sensors/1/temperature')

  // Rungs are tried in the order they are added, cheapest first. The mesh hop loses every
  // packet here; the backhaul carries one send, then drops the next two.
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.degraded(mesh.rung(), 1, 0, 0))
  await ladder.rung(Transport.degraded(backhaul.rung(), 0, 1, 2))
  await ladder.connect()

  // The mesh hop refuses, so the reading goes out over the backhaul and arrives on the
  // broker only that rung publishes to.
  const topic = 'sensors/1/temperature'
  assert.equal(await ladder.send(topic, Buffer.from('21.5')), Delivery.Sent)
  assert.equal((await gateway.recv())?.payload.toString(), '21.5')

  // Now nothing will take a send, so the next reading is buffered rather than lost.
  assert.equal(await ladder.send(topic, Buffer.from('21.6')), Delivery.Buffered)
  assert.equal(await ladder.buffered(), 1)

  // A flush while the links are still down forwards nothing and leaves the backlog
  // intact, because a record is removed only once a rung has accepted it.
  assert.equal(await ladder.flush(), 0)
  assert.equal(await ladder.buffered(), 1)

  // The backhaul is reachable again, so the buffered reading goes out exactly once.
  assert.equal(await ladder.flush(), 1)
  assert.equal(await ladder.buffered(), 0)
  assert.equal((await gateway.recv())?.payload.toString(), '21.6')
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-ladder`](https://crates.io/crates/pamoja-ladder) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ladder/index.html), [docs.rs](https://docs.rs/pamoja-ladder) |
| TypeScript | [`@pamoja/ladder`](https://www.npmjs.com/package/@pamoja/ladder) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html) |
| Python | [`pamoja-ladder`](https://pypi.org/project/pamoja-ladder/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/ladder.html) |
| C# | [`Pamoja.Ladder`](https://www.nuget.org/packages/Pamoja.Ladder) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ladder.html) |

## Documentation

- [`@pamoja/ladder` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ladder.html), every class, function, and type this package exports.
- [The Transport ladder guide](https://pamoja.molex.cloud/docs/guides/ladder.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
