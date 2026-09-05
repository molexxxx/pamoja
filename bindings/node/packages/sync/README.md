# @pamoja/sync

Offline-first queues: in memory, and a crash-safe on-disk queue that survives power loss. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sync.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/sync
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/sync.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sync.ts):

```typescript
import { Store } from '@pamoja/sync'

async function main() {
  // A node with nowhere to send buffers its readings. This queue is held in memory, so it
  // lasts as long as the process; Store.file(dir) is the same queue on disk, which is what
  // a node uses to survive a reboot with its backlog intact.
  const outbox = Store.memory()
  for (const reading of ['20.1', '20.4', '20.2']) {
    await outbox.append(Buffer.from(reading))
  }
  console.log(`queued    ${await outbox.len()} readings with no link`)

  // Peek reads the oldest record without taking it, so a send that fails part-way leaves
  // the queue exactly as it was.
  const oldest = (await outbox.peek())!
  console.log(`oldest    ${oldest.toString()} and still ${await outbox.len()} held`)

  // The link returns and the queue drains oldest first, in the order the readings were
  // taken rather than the order they happen to come back off a buffer.
  const drained: string[] = []
  for (let record = await outbox.pop(); record !== null; record = await outbox.pop()) {
    drained.push(record.toString())
  }
  console.log(`drained   ${drained.join(', ')}`)

  // A bounded queue refuses the append that would overflow it. A full store is
  // backpressure the caller is told about, not a reading dropped behind its back.
  const bounded = Store.memory(2)
  await bounded.append(Buffer.from('20.1'))
  await bounded.append(Buffer.from('20.4'))
  try {
    await bounded.append(Buffer.from('20.2'))
    console.log('a full queue took a third reading, which should never happen')
  } catch (error) {
    console.log(`full      refused the third reading: ${(error as Error).message}`)
  }

  return { oldest, drained, left: await outbox.len(), held: await bounded.len() }
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sync`](https://crates.io/crates/pamoja-sync) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sync/index.html), [docs.rs](https://docs.rs/pamoja-sync) |
| TypeScript | [`@pamoja/sync`](https://www.npmjs.com/package/@pamoja/sync) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html) |
| Python | [`pamoja-sync`](https://pypi.org/project/pamoja-sync/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sync.html) |
| C# | [`Pamoja.Sync`](https://www.nuget.org/packages/Pamoja.Sync) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sync.html) |

## Documentation

- [`@pamoja/sync` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sync.html), every class, function, and type this package exports.
- [The Store and forward guide](https://pamoja.molex.cloud/docs/guides/sync.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
