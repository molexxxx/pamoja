# @pamoja/profile

Named, ready-to-run device profiles from plain data or a JSON manifest. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/profile.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html)

## Install

```sh
npm install @pamoja/profile
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import { readFileSync } from 'node:fs'
import { LoopbackBroker } from '@pamoja/loopback'
import { Node, Profile } from '@pamoja/profile'

// A profile is a file. This one ships in the catalog under profiles/: it holds a brooder at
// 32 C by switching a heat lamp, says what it reads, and says how a dashboard draws it.
const text = readFileSync('profiles/brooder-heater.json', 'utf8')
const profile = Profile.fromJson(text)

async function example(): Promise<boolean> {
  const reads = profile.reads!
  console.log(`profile   ${profile.name} reads ${reads.quantity} in ${reads.unit} and reports on ${profile.topic}`)

  // A node is the profile and the parts that make it run: a sensor, an output, and a link.
  // A morning of readings stands in for the probe, and a dashboard listens on the same
  // broker.
  const broker = new LoopbackBroker()
  const link = broker.link()
  const dashboard = broker.link()
  await link.connect()
  await dashboard.connect()
  await dashboard.subscribe(profile.topic)
  const morning = [27.5, 31.8, 32.6, 32.1, 31.4]
  let lamp = false
  const node = new Node({
    profile,
    read: () => morning.shift()!,
    drive: (on) => {
      lamp = on
    },
    link,
  })

  // Each tick reads, decides, switches the lamp, and publishes the reading. The lamp comes on
  // at 31.5 C or below and goes off at 32.5 C or above, and in between it stays as it was; a
  // reading more than 4 C from 32 raises an alert as well.
  let was = false
  for (let at = 0; at < 5; at += 1) {
    const { reading, reaction } = await node.tick()
    const on = reaction.actuator === true
    const change = on ? (was ? 'lamp stays on' : 'lamp on') : was ? 'lamp off' : 'lamp stays off'
    const alert = reaction.alert ? `, alert ${reaction.alert.kind}` : ''
    console.log(`${`${reading} C`.padEnd(10)}${change}${alert}`)
    was = on
  }

  // The dashboard heard every reading the node published.
  const heard: number[] = []
  for (let at = 0; at < 5; at += 1) {
    heard.push((await dashboard.recv())!.number!)
  }
  console.log(`heard     ${heard.join(', ')} on ${profile.topic}`)

  // Between ticks the node waits as long as its battery allows: often on a healthy charge,
  // sparingly on a low one. run() does this until stopped, waiting each interval.
  for (const charge of [0.8, 0.3, 0.1]) {
    const { mode, waitMs } = node.schedule(charge)
    console.log(`battery   at ${(charge * 100).toFixed(0)}% it runs ${mode} and waits ${waitMs / 1000} s`)
  }

  // The same file says how a dashboard draws the node.
  const element = profile.presentation!.elements[0]
  const [low, high] = element.band!
  console.log(`draws     ${element.key} in ${element.unit} on a ${element.viz}, safe from ${low} to ${high}`)
  return lamp
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-profile`](https://crates.io/crates/pamoja-profile) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [docs.rs](https://docs.rs/pamoja-profile), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-profile) |
| TypeScript | [`@pamoja/profile`](https://www.npmjs.com/package/@pamoja/profile) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-profile) |
| Python | [`pamoja-profile`](https://pypi.org/project/pamoja-profile/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-profile) |
| C# | [`Pamoja.Profile`](https://www.nuget.org/packages/Pamoja.Profile) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-profile) |

## Documentation

- [`@pamoja/profile` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), every class, function, and type this package exports.
- [The Device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
