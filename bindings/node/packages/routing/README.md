# @pamoja/routing

Reverse-path routing that learns the cheapest route from overheard traffic. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/routing.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/routing
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/routing.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/routing.ts):

```typescript
import { ForwardAction, Router } from '@pamoja/routing'

// The nodes on this mesh. An address is just a number; naming them is what makes the
// table below read as a map of the site rather than a list of numbers.
const GATEWAY = 1
const PUMP = 9
const TANK = 10
const NORTH_RELAY = 5
const EAST_RELAY = 7
const SOUTH_RELAY = 3
const SILO = 32

// A node learns the way to another from traffic it already hears: a packet from the pump
// that arrived through the north relay proves that relay is a way back, at the cost the
// packet reports.
const router = new Router(GATEWAY, 4)
router.observe(PUMP, NORTH_RELAY, 2)

// The table keeps only the cheapest way it knows to each node, so a cost-1 report through
// the east relay takes over and the later cost-4 report changes nothing.
router.observe(PUMP, EAST_RELAY, 1)
router.observe(PUMP, SOUTH_RELAY, 4)
router.observe(TANK, NORTH_RELAY, 3)

const route = router.route(PUMP)
console.log(`to the pump   via ${route?.nextHop} at cost ${route?.cost}`)
console.log(`routes held   ${router.size}`)

// Every packet gets one of three answers: deliver it here, relay it to the neighbour on
// the way, or flood it because no route is known yet.
for (const [name, address] of [
  ['gateway', GATEWAY],
  ['pump', PUMP],
  ['silo', SILO],
] as const) {
  const decision = router.forward(address)
  if (decision.action === ForwardAction.Deliver) {
    console.log(`for the ${name.padEnd(8)} deliver here`)
  } else if (decision.action === ForwardAction.Relay) {
    console.log(`for the ${name.padEnd(8)} relay via ${decision.nextHop}`)
  } else {
    console.log(`for the ${name.padEnd(8)} flood, no route known`)
  }
}

// Forgetting a node that has gone quiet returns its traffic to flooding, so routing is an
// optimisation over flooding rather than a second thing that can fail.
router.forget(PUMP)
const after = router.forward(PUMP)
console.log(`pump forgotten, so it floods again: ${after.action === ForwardAction.Flood}`)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-routing`](https://crates.io/crates/pamoja-routing) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html), [docs.rs](https://docs.rs/pamoja-routing) |
| TypeScript | [`@pamoja/routing`](https://www.npmjs.com/package/@pamoja/routing) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html) |
| Python | [`pamoja-routing`](https://pypi.org/project/pamoja-routing/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html) |
| C# | [`Pamoja.Routing`](https://www.nuget.org/packages/Pamoja.Routing) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html) |

## Documentation

- [`@pamoja/routing` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html), every class, function, and type this package exports.
- [The Routing guide](https://pamoja.molex.cloud/docs/guides/routing.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
