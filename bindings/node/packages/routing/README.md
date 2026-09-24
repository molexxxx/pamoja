# @pamoja/routing

Reverse-path routing that learns the cheapest route from overheard traffic. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/routing.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html)

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
// that arrived through a relay proves that relay is a way back, at the cost the packet
// reports. The table keeps the cheapest way it has heard, and a tie keeps the way in use so
// two equal paths do not flap. Word from the relay already in use is taken even when it is
// worse, which is how a failing link lets a detour win.
const router = new Router(GATEWAY, 4)
for (const [via, cost] of [
  [NORTH_RELAY, 2],
  [EAST_RELAY, 1],
  [SOUTH_RELAY, 4],
  [NORTH_RELAY, 1],
  [EAST_RELAY, 3],
  [NORTH_RELAY, 2],
]) {
  const changed = router.observe(PUMP, via, cost)
  const route = router.route(PUMP)
  const outcome = changed ? 'so the route is' : 'and the route stays'
  console.log(
    `heard     the pump via ${via} at cost ${cost}, ${outcome} ${route?.nextHop} at cost ${route?.cost}`,
  )
}

// The table lists what it holds, one route for each node it has heard from.
router.observe(TANK, NORTH_RELAY, 3)
const held = router
  .routes()
  .map((route) => `to ${route.dst} via ${route.nextHop} at cost ${route.cost}`)
console.log(`table     ${router.size} routes of ${router.capacity}: ${held.join(', ')}`)

// Every packet gets one of three answers: deliver it here, relay it to the neighbor on the
// way, or flood it because no route is known yet.
for (const [name, address] of [
  ['gateway', GATEWAY],
  ['pump', PUMP],
  ['silo', SILO],
] as const) {
  const decision = router.forward(address)
  if (decision.action === ForwardAction.Deliver) {
    console.log(`${name.padEnd(10)}deliver here`)
  } else if (decision.action === ForwardAction.Relay) {
    console.log(`${name.padEnd(10)}relay via ${decision.nextHop}`)
  } else {
    console.log(`${name.padEnd(10)}flood, no route known`)
  }
}

// The table keeps no clock, so a route through a relay that has gone quiet stays until the
// caller forgets it, typically when a relayed packet goes unanswered. Forgetting returns the
// node's traffic to flooding, the answer that always works.
router.forget(PUMP)
if (router.forward(PUMP).action === ForwardAction.Flood) {
  console.log(`forgot    the pump, so it floods again, and ${router.size} route is left`)
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-routing`](https://crates.io/crates/pamoja-routing) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html), [docs.rs](https://docs.rs/pamoja-routing), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-routing) |
| TypeScript | [`@pamoja/routing`](https://www.npmjs.com/package/@pamoja/routing) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-routing) |
| Python | [`pamoja-routing`](https://pypi.org/project/pamoja-routing/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-routing) |
| C# | [`Pamoja.Routing`](https://www.nuget.org/packages/Pamoja.Routing) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-routing) |

## Documentation

- [`@pamoja/routing` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html), every class, function, and type this package exports.
- [The Routing guide](https://pamoja.molex.cloud/docs/guides/routing.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
