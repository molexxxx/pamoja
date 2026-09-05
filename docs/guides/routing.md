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

It builds the table for a gateway, feeds it four packets of overheard traffic,
and checks which route it settles on and what it decides for a packet bound for
three different places. Then it forgets a node and confirms that node's traffic
falls back to flooding.

An address on a mesh is just a number. The example names the ones it uses, so
the table reads as a map of the site: a gateway, a pump and a tank, three relays
between them, and a silo nothing has been heard from yet.

It proves:

- A packet from the pump that arrived through the north relay makes that relay
  the way back to the pump, learned with no exchange of routing messages.
- A report of cost 1 replaces that route and a later report of cost 4 is refused,
  so the table holds the cheapest way it has heard and says which observations
  changed it.
- Four observations of two nodes leave two routes, not four.
- A packet for the gateway is delivered, one for the pump relays to the east
  relay, and one for the silo floods.
- Forgetting the pump drops that one route and returns its traffic to flooding.

## Rust

<!-- snippet: examples/tests/guides/routing.rs#example -->
From [`examples/tests/guides/routing.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/routing.rs):

```rust
use pamoja_routing::{Forward, Router};

// The nodes on this mesh. An address is just a number; naming them is what makes the
// table below read as a map of the site rather than a list of numbers.
const GATEWAY: u32 = 1;
const PUMP: u32 = 9;
const TANK: u32 = 10;
const NORTH_RELAY: u32 = 5;
const EAST_RELAY: u32 = 7;
const SOUTH_RELAY: u32 = 3;
const SILO: u32 = 32;

// A node learns the way to another from traffic it already hears: a packet from the
// pump that arrived through the north relay proves that relay is a way back, at the
// cost the packet reports.
let mut router: Router<4> = Router::new(GATEWAY);
router.observe(PUMP, NORTH_RELAY, 2);

// The table keeps only the cheapest way it knows to each node, so a cost-1 report
// through the east relay takes over and the later cost-4 report changes nothing.
router.observe(PUMP, EAST_RELAY, 1);
router.observe(PUMP, SOUTH_RELAY, 4);
router.observe(TANK, NORTH_RELAY, 3);

let route = router.route(PUMP).expect("a route to the pump");
let (hop, cost) = (route.next_hop(), route.cost());
println!("to the pump   via {hop} at cost {cost}");
println!("routes held   {}", router.len());

// Every packet gets one of three answers: deliver it here, relay it to the neighbour
// on the way, or flood it because no route is known yet.
for (name, address) in [("gateway", GATEWAY), ("pump", PUMP), ("silo", SILO)] {
    match router.forward(address) {
        Forward::Deliver => println!("for the {name:<8} deliver here"),
        Forward::Relay(next) => println!("for the {name:<8} relay via {next}"),
        Forward::Flood => println!("for the {name:<8} flood, no route known"),
    }
}

// Forgetting a node that has gone quiet returns its traffic to flooding, so routing
// is an optimisation over flooding rather than a second thing that can fail.
router.forget(PUMP);
let after = router.forward(PUMP);
let floods_again = after == Forward::Flood;
println!("pump forgotten, so it floods again: {floods_again}");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/routing.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/routing.py#example -->
From [`bindings/python/guides/routing.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/routing.py):

```python
from pamoja.routing import ForwardAction, Router

# The nodes on this mesh. An address is just a number; naming them is what makes the
# table below read as a map of the site rather than a list of numbers.
GATEWAY = 1
PUMP = 9
TANK = 10
NORTH_RELAY = 5
EAST_RELAY = 7
SOUTH_RELAY = 3
SILO = 32

# A node learns the way to another from traffic it already hears: a packet from the pump
# that arrived through the north relay proves that relay is a way back, at the cost the
# packet reports.
router = Router(GATEWAY, 4)
router.observe(PUMP, NORTH_RELAY, 2)

# The table keeps only the cheapest way it knows to each node, so a cost-1 report through
# the east relay takes over and the later cost-4 report changes nothing.
router.observe(PUMP, EAST_RELAY, 1)
router.observe(PUMP, SOUTH_RELAY, 4)
router.observe(TANK, NORTH_RELAY, 3)

route = router.route(PUMP)
print(f"to the pump   via {route.next_hop} at cost {route.cost}")
print(f"routes held   {len(router)}")

# Every packet gets one of three answers: deliver it here, relay it to the neighbour on
# the way, or flood it because no route is known yet.
for name, address in [("gateway", GATEWAY), ("pump", PUMP), ("silo", SILO)]:
    decision = router.forward(address)
    if decision.action == ForwardAction.DELIVER:
        print(f"for the {name:<8} deliver here")
    elif decision.action == ForwardAction.RELAY:
        print(f"for the {name:<8} relay via {decision.next_hop}")
    else:
        print(f"for the {name:<8} flood, no route known")

# Forgetting a node that has gone quiet returns its traffic to flooding, so routing is an
# optimisation over flooding rather than a second thing that can fail.
router.forget(PUMP)
after = router.forward(PUMP)
print(f"pump forgotten, so it floods again: {after.action == ForwardAction.FLOOD}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs):

```csharp
// The nodes on this mesh. An address is just a number; naming them is what makes
// the table below read as a map of the site rather than a list of numbers.
const byte Gateway = 1;
const byte Pump = 9;
const byte Tank = 10;
const byte NorthRelay = 5;
const byte EastRelay = 7;
const byte SouthRelay = 3;
const byte Silo = 32;

// A node learns the way to another from traffic it already hears: a packet from
// the pump that arrived through the north relay proves that relay is a way back,
// at the cost the packet reports.
using Router router = new(Gateway, 4);
router.Observe(Pump, NorthRelay, 2);

// The table keeps only the cheapest way it knows to each node, so a cost-1 report
// through the east relay takes over and the later cost-4 report changes nothing.
router.Observe(Pump, EastRelay, 1);
router.Observe(Pump, SouthRelay, 4);
router.Observe(Tank, NorthRelay, 3);

Route? route = router.RouteTo(Pump);
Console.WriteLine($"to the pump   via {route?.NextHop} at cost {route?.Cost}");
Console.WriteLine($"routes held   {router.Count}");

// Every packet gets one of three answers: deliver it here, relay it to the
// neighbour on the way, or flood it because no route is known yet.
foreach ((string name, byte address) in
    new[] { ("gateway", Gateway), ("pump", Pump), ("silo", Silo) })
{
    ForwardDecision decision = router.Forward(address);
    Console.WriteLine(decision.Action switch
    {
        ForwardAction.Deliver => $"for the {name,-8} deliver here",
        ForwardAction.Relay => $"for the {name,-8} relay via {decision.NextHop}",
        _ => $"for the {name,-8} flood, no route known",
    });
}

// Forgetting a node that has gone quiet returns its traffic to flooding, so
// routing is an optimisation over flooding rather than a second thing that can
// fail.
router.Forget(Pump);
ForwardDecision after = router.Forward(Pump);
Console.WriteLine(
    $"pump forgotten, so it floods again: {after.Action == ForwardAction.Flood}");
```
<!-- end -->

## Reference

<!-- table: reference routing -->
- Rust: [`pamoja-routing`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html)
- TypeScript: [`@pamoja/routing`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html)
- Python: [`pamoja.routing`](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html)
- C#: [`Pamoja.Routing`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html)
<!-- end -->
