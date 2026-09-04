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
import assert from 'node:assert/strict'

import { ForwardAction, Router } from '@pamoja/routing'

// A node learns the way to another from traffic it already hears: a packet from 0x09
// that arrived through neighbour 0x05 proves 0x05 is the way back, at the cost the
// packet reports.
const router = new Router(0x01, 4)
assert.equal(router.observe(0x09, 0x05, 2), true)

// The table keeps the cheapest way it knows to each node, so the report of cost 1 takes
// over and the later cost-4 report changes nothing.
assert.equal(router.observe(0x09, 0x07, 1), true)
assert.equal(router.observe(0x09, 0x03, 4), false)
assert.equal(router.observe(0x0a, 0x05, 3), true)
const route = router.route(0x09)
assert.equal(route?.nextHop, 0x07)
assert.equal(route?.cost, 1)
assert.equal(router.size, 2)

// A packet gets one of three answers: deliver it here, relay it to the neighbour on the
// way, or flood it because no route is known yet.
assert.equal(router.forward(0x01).action, ForwardAction.Deliver)
assert.equal(router.forward(0x09).action, ForwardAction.Relay)
assert.equal(router.forward(0x09).nextHop, 0x07)
assert.equal(router.forward(0x20).action, ForwardAction.Flood)

// Forgetting a node that has gone quiet returns its traffic to flooding, so routing is
// an optimisation over flooding rather than a second thing that can fail.
router.forget(0x09)
assert.equal(router.forward(0x09).action, ForwardAction.Flood)
assert.equal(router.size, 1)
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
