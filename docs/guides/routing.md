# Routing

A mesh gets a packet across by flooding it: every node rebroadcasts, which always
works and spends every node's airtime and battery on every packet. Once a network
settles, most traffic goes to a few known places, and a node that remembers the
way can send to one neighbour instead of shouting at all of them. pamoja keeps
that memory as a table of fixed size, filled from the traffic the node already
hears, and answers one question per packet: deliver, relay, or flood. It owns no
radio, so the same table runs on a microcontroller, on a gateway, or in a test
with nothing on the air.

## What the example does

It builds the table for the node at `0x01`, feeds it four packets of overheard
traffic, and checks which route it settles on and what it decides for a packet
bound for three different places. Then it forgets a node and confirms that node's
traffic falls back to flooding.

It proves:

- A packet from `0x09` that arrived through neighbour `0x05` makes `0x05` the way
  back to `0x09`, learned with no exchange of routing messages.
- A report of cost 1 replaces that route and a later report of cost 4 is refused,
  so the table holds the cheapest way it has heard and says which observations
  changed it.
- Four observations of two nodes leave two routes, not four.
- A packet for `0x01` is delivered, one for `0x09` relays to `0x07`, and one for
  the unknown `0x20` floods.
- Forgetting `0x09` drops that one route and returns its traffic to flooding.

## Rust

<!-- snippet: examples/tests/guides/routing.rs#example -->
From [`examples/tests/guides/routing.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/routing.rs):

```rust
use pamoja_routing::{Forward, Router};

// A node learns the way to another from traffic it already hears: a packet from 0x09
// that arrived through neighbour 0x05 proves 0x05 is the way back, at the cost the
// packet reports.
let mut router: Router<4> = Router::new(0x01);
assert!(router.observe(0x09, 0x05, 2));

// The table keeps the cheapest way it knows to each node, so the report of cost 1
// takes over and the later cost-4 report changes nothing.
assert!(router.observe(0x09, 0x07, 1));
assert!(!router.observe(0x09, 0x03, 4));
assert!(router.observe(0x0A, 0x05, 3));
let route = router.route(0x09).expect("a route to 0x09");
assert_eq!(route.next_hop(), 0x07);
assert_eq!(route.cost(), 1);
assert_eq!(router.len(), 2);

// A packet gets one of three answers: deliver it here, relay it to the neighbour on
// the way, or flood it because no route is known yet.
assert_eq!(router.forward(0x01), Forward::Deliver);
assert_eq!(router.forward(0x09), Forward::Relay(0x07));
assert_eq!(router.forward(0x20), Forward::Flood);

// Forgetting a node that has gone quiet returns its traffic to flooding, so routing
// is an optimisation over flooding rather than a second thing that can fail.
router.forget(0x09);
assert_eq!(router.forward(0x09), Forward::Flood);
assert_eq!(router.len(), 1);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/routing.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/routing.py#example -->
From [`bindings/python/guides/routing.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/routing.py):

```python
from pamoja.routing import ForwardAction, Router

# A node learns the way to another from traffic it already hears: a packet from 0x09
# that arrived through neighbour 0x05 proves 0x05 is the way back, at the cost the
# packet reports.
router = Router(0x01, 4)
assert router.observe(0x09, 0x05, 2)

# The table keeps the cheapest way it knows to each node, so the report of cost 1 takes
# over and the later cost-4 report changes nothing.
assert router.observe(0x09, 0x07, 1)
assert not router.observe(0x09, 0x03, 4)
assert router.observe(0x0A, 0x05, 3)
route = router.route(0x09)
assert route.next_hop == 0x07
assert route.cost == 1
assert len(router) == 2

# A packet gets one of three answers: deliver it here, relay it to the neighbour on the
# way, or flood it because no route is known yet.
assert router.forward(0x01).action == ForwardAction.DELIVER
assert router.forward(0x09).action == ForwardAction.RELAY
assert router.forward(0x09).next_hop == 0x07
assert router.forward(0x20).action == ForwardAction.FLOOD

# Forgetting a node that has gone quiet returns its traffic to flooding, so routing is
# an optimisation over flooding rather than a second thing that can fail.
router.forget(0x09)
assert router.forward(0x09).action == ForwardAction.FLOOD
assert len(router) == 1
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs):

```csharp
// A node learns the way to another from traffic it already hears: a packet from
// 0x09 that arrived through neighbour 0x05 proves 0x05 is the way back, at the
// cost the packet reports.
using Router router = new(0x01, 4);
Expect(router.Observe(0x09, 0x05, 2), "the first packet heard from 0x09 teaches a route");

// The table keeps the cheapest way it knows to each node, so the report of cost 1
// takes over and the later cost-4 report changes nothing.
Expect(router.Observe(0x09, 0x07, 1), "a cheaper neighbour redirects the route");
Expect(!router.Observe(0x09, 0x03, 4), "a costlier way is not worth taking");
Expect(router.Observe(0x0A, 0x05, 3), "a second node is learned");
Route? route = router.RouteTo(0x09);
Expect(route?.NextHop == 0x07, "0x09 is reached through the cheaper neighbour");
Expect(route?.Cost == 1, "at the cost that neighbour reported");
Expect(router.Count == 2, "two nodes are known, not four observations");

// A packet gets one of three answers: deliver it here, relay it to the neighbour
// on the way, or flood it because no route is known yet.
Expect(router.Forward(0x01).Action == ForwardAction.Deliver, "a packet for this node");
ForwardDecision relayed = router.Forward(0x09);
Expect(relayed.Action == ForwardAction.Relay, "a packet for a node we can reach");
Expect(relayed.NextHop == 0x07, "goes to the neighbour on the way");
Expect(router.Forward(0x20).Action == ForwardAction.Flood, "an unknown node still floods");

// Forgetting a node that has gone quiet returns its traffic to flooding, so
// routing is an optimisation over flooding rather than a second thing that can
// fail.
router.Forget(0x09);
Expect(router.Forward(0x09).Action == ForwardAction.Flood, "the way to 0x09 is gone");
Expect(router.Count == 1, "forgetting drops exactly one route");
```
<!-- end -->

## Reference

<!-- table: reference routing -->
- Rust: [`pamoja-routing`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html)
- TypeScript: [`@pamoja/routing`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html)
- Python: [`pamoja.routing`](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html)
- C#: [`Pamoja.Routing`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html)
<!-- end -->
