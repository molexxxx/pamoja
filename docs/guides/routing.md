# Routing

A mesh gets a packet across by flooding it: every node rebroadcasts, which always
works and spends every node's airtime and battery on every packet. Once a network
settles, most traffic goes to a few known places, and a node that remembers the
way can send to one neighbor instead of shouting at all of them. pamoja keeps
that memory as a table of fixed size, filled from the traffic the node already
hears, and answers one question per packet: deliver, relay, or flood. It owns no
radio, so the same table runs on a microcontroller, on a gateway, or in a test
with nothing on the air.

## What the example does

It builds the table for a gateway and feeds it six reports about the pump, each
arriving through one of three relays, and prints after each one whether the
table changed its route. Then it lists the table, asks what to do with a packet
bound for three places, and forgets the pump.

The second part fills a table of two routes and offers it a cheaper route and a
costlier one, has the gateway hear its own packet come back, and makes a table
with no room at all.

The third part puts a table on every node of a site of seven: a gateway, three
relays around it, and a tank, a pump, and a silo, each beyond one relay. The pump
floods a reading in [mesh frames](mesh.md), every node that hears it learns the
way back, and the gateway's answer then crosses the site by route. The same
answer on a site that has learned nothing floods, and the example counts the
radio sends both ways.

An address on a mesh is just a number. The example names the ones it uses, so
the table reads as a map of the site. No route is written down anywhere: every
hop and cost printed is the table's own pick from the reports it heard.

It proves:

- One packet heard from the pump through a relay teaches the way back to it,
  with no routing messages exchanged.
- A cheaper report takes the route over. A costlier one, or an equal one through
  another relay, changes nothing, and every observation says whether it changed
  the table.
- A report from the relay in use is taken even when it is worse, so a detour
  that is now cheaper can win.
- A packet for the gateway is delivered, one for the pump relays, and one for the
  silo floods. Forgetting the pump leaves the tank's route, and packets for the
  pump flood again.
- A full table gives the slot of its costliest route to a cheaper one and refuses
  a costlier one, whose traffic floods.
- A route to the node itself is never learned, and a table with no room learns
  nothing and still delivers.
- One flood teaches every node on the site the way back to its source, and the
  answer then takes 2 sends where a flood of it takes 6.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example routing" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example routing</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- routing" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- routing</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/routing.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/routing.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- routing" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- routing</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-routing` is `no_std` and never allocates. `Router<N>` is a table
of `N` routes sized at compile time; `DynamicRouter`, behind the `alloc`
feature, takes its size at run time, and the two decide identically.
`observe(origin, via, cost)` returns whether the table changed, and
`forward(dst)` returns `Forward::Deliver`, `Forward::Relay(next_hop)`, or
`Forward::Flood`. `route(dst)` returns a `Route` with `dst()`, `next_hop()`, and
`cost()`, and `routes()` lists every one held. No call can fail; each one
answers.

<!-- snippet: examples/guides/routing.rs#example -->
From [`examples/guides/routing.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/routing.rs):

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
// pump that arrived through a relay proves that relay is a way back, at the cost the
// packet reports. The table keeps the cheapest way it has heard, and a tie keeps the
// way in use so two equal paths do not flap. Word from the relay already in use is
// taken even when it is worse, which is how a failing link lets a detour win.
let mut router: Router<4> = Router::new(GATEWAY);
for (via, cost) in [
    (NORTH_RELAY, 2),
    (EAST_RELAY, 1),
    (SOUTH_RELAY, 4),
    (NORTH_RELAY, 1),
    (EAST_RELAY, 3),
    (NORTH_RELAY, 2),
] {
    let changed = router.observe(PUMP, via, cost);
    let route = router.route(PUMP).expect("a route to the pump");
    let outcome = if changed {
        "so the route is"
    } else {
        "and the route stays"
    };
    println!(
        "heard     the pump via {via} at cost {cost}, {outcome} {} at cost {}",
        route.next_hop(),
        route.cost()
    );
}

// The table lists what it holds, one route for each node it has heard from.
router.observe(TANK, NORTH_RELAY, 3);
let held: Vec<String> = router
    .routes()
    .map(|route| {
        let (dst, hop, cost) = (route.dst(), route.next_hop(), route.cost());
        format!("to {dst} via {hop} at cost {cost}")
    })
    .collect();
println!(
    "table     {} routes of {}: {}",
    router.len(),
    router.capacity(),
    held.join(", ")
);

// Every packet gets one of three answers: deliver it here, relay it to the neighbor
// on the way, or flood it because no route is known yet.
for (name, address) in [("gateway", GATEWAY), ("pump", PUMP), ("silo", SILO)] {
    match router.forward(address) {
        Forward::Deliver => println!("{name:<10}deliver here"),
        Forward::Relay(next) => println!("{name:<10}relay via {next}"),
        Forward::Flood => println!("{name:<10}flood, no route known"),
    }
}

// The table keeps no clock, so a route through a relay that has gone quiet stays
// until the caller forgets it, typically when a relayed packet goes unanswered.
// Forgetting returns the node's traffic to flooding, the answer that always works.
router.forget(PUMP);
if router.forward(PUMP) == Forward::Flood {
    println!(
        "forgot    the pump, so it floods again, and {} route is left",
        router.len()
    );
}
```
<!-- end -->

What a full table keeps, continuing from above:

<!-- snippet: examples/guides/routing.rs#limits -->
From [`examples/guides/routing.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/routing.rs):

```rust
// A table has a fixed number of slots. Once they are full, a cheaper route takes the
// slot of the costliest one held and a costlier route is refused, so a small table
// keeps the nodes nearest to it and floods to the rest.
const WELL: u32 = 11;
const GATE: u32 = 12;
let mut small: Router<2> = Router::new(GATEWAY);
small.observe(TANK, NORTH_RELAY, 3);
small.observe(SILO, SOUTH_RELAY, 5);
let held: Vec<String> = small
    .routes()
    .map(|route| {
        let (dst, hop, cost) = (route.dst(), route.next_hop(), route.cost());
        format!("to {dst} via {hop} at cost {cost}")
    })
    .collect();
println!(
    "full      {} routes of {}: {}",
    small.len(),
    small.capacity(),
    held.join(", ")
);
if small.observe(WELL, EAST_RELAY, 2) && small.route(SILO).is_none() {
    println!("evicted   the well at cost 2 took the slot of the silo, the costliest held");
}
if !small.observe(GATE, EAST_RELAY, 6) && small.forward(GATE) == Forward::Flood {
    println!("refused   the gate at cost 6 costs more than every route held, so it floods");
}

// A flood echoes, and the gateway hears its own packets come back through the
// relays. A route to the node itself is never learned, whatever it costs.
if !router.observe(GATEWAY, EAST_RELAY, 2) {
    println!("echo      the gateway's own packet coming back teaches it nothing");
}

// A table with no slots is flooding with nothing remembered, which a node with no
// memory to spare can still do. A packet for the node itself is still delivered.
let mut none: Router<0> = Router::new(GATEWAY);
let learned = none.observe(PUMP, EAST_RELAY, 1);
if !learned && none.forward(PUMP) == Forward::Flood && none.forward(GATEWAY) == Forward::Deliver
{
    println!(
        "no room   a table of 0 learns nothing: the pump floods, and the gateway still delivers"
    );
}
```
<!-- end -->

The site, continuing from above:

<!-- snippet: examples/guides/routing.rs#site -->
From [`examples/guides/routing.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/routing.rs):

```rust
use std::collections::{BTreeMap, VecDeque};

use pamoja_mesh::{Frame, MeshError, SeenCache};

// Who hears whom on the site. The gateway hears the three relays, and each relay
// hears the one node beyond it; the pump is out of the gateway's range.
let site: BTreeMap<u32, Vec<u32>> = BTreeMap::from([
    (GATEWAY, vec![NORTH_RELAY, EAST_RELAY, SOUTH_RELAY]),
    (NORTH_RELAY, vec![GATEWAY, TANK]),
    (EAST_RELAY, vec![GATEWAY, PUMP]),
    (SOUTH_RELAY, vec![GATEWAY, SILO]),
    (TANK, vec![NORTH_RELAY]),
    (PUMP, vec![EAST_RELAY]),
    (SILO, vec![SOUTH_RELAY]),
]);

// Sends a frame from one node and plays out what the site does with it. A node that
// hears a frame for the first time learns the way back to its source, then asks its
// own table what to do: deliver it, relay it to the one neighbor on the way, or flood
// it to every neighbor in range, spending a hop each time it goes on. Every frame here
// starts at the default hop limit, and one heard straight from its source still has
// all of it, so the hops a frame has come are what it has spent, plus one. Returns
// how many times a radio sent, and the nodes that took the frame.
fn send(
    site: &BTreeMap<u32, Vec<u32>>,
    tables: &mut BTreeMap<u32, Router<8>>,
    from: u32,
    frame: Frame,
) -> Result<(usize, Vec<u32>), MeshError> {
    let mut seen: BTreeMap<u32, SeenCache<64>> =
        site.keys().map(|&node| (node, SeenCache::new())).collect();
    seen.entry(from).or_default().record(frame.dedup_key());
    let (mut sends, mut reached) = (0, Vec::new());
    let mut on_the_air = VecDeque::from([(from, frame)]);
    while let Some((sender, sent)) = on_the_air.pop_front() {
        let to = match tables[&sender].forward(sent.dst()) {
            Forward::Deliver => continue,
            Forward::Relay(next) => vec![next],
            Forward::Flood => site[&sender].clone(),
        };
        sends += 1;
        for node in to {
            let heard = Frame::parse(sent.as_bytes())?;
            if !seen.entry(node).or_default().record(heard.dedup_key()) {
                continue;
            }
            reached.push(node);
            let hops = Frame::DEFAULT_HOP_LIMIT - heard.hop_limit() + 1;
            if let Some(table) = tables.get_mut(&node) {
                table.observe(heard.src(), sender, hops.into());
            }
            if let Some(onward) = heard.relayed() {
                on_the_air.push_back((node, onward));
            }
        }
    }
    Ok((sends, reached))
}

// The pump floods a reading, and every node that takes it learns the way back.
let mut tables: BTreeMap<u32, Router<8>> =
    site.keys().map(|&node| (node, Router::new(node))).collect();
let reading = Frame::broadcast(PUMP, 1, b"flow=12")?;
let (sends, reached) = send(&site, &mut tables, PUMP, reading)?;
println!(
    "flood     the pump's reading took {sends} sends to reach {} nodes, and each learned the way back",
    reached.len()
);

// The gateway answers the pump. Each node on the way relays to the one neighbor its
// table names, so the answer reaches only the nodes on the path.
let answer = Frame::new(GATEWAY, PUMP, 1, b"run=10min")?;
let (routed, path) = send(&site, &mut tables, GATEWAY, answer)?;
let path: Vec<String> = path.iter().map(u32::to_string).collect();
println!(
    "routed    the gateway's answer reached only {}, in {routed} sends",
    path.join(" and ")
);

// The same answer on a site that has learned nothing floods, and every node relays it.
let mut blank: BTreeMap<u32, Router<8>> =
    site.keys().map(|&node| (node, Router::new(node))).collect();
let (flooded, everyone) = send(&site, &mut blank, GATEWAY, answer)?;
println!(
    "flooded   with nothing learned, the same answer reached all {} other nodes in {flooded} sends",
    everyone.len()
);
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/routing` has a `Router` class sized when it is made, 64
routes unless told otherwise, and `size` counts the routes it holds. `observe`
returns whether the table changed. `forward` returns a decision whose `action` is
`ForwardAction.Deliver`, `Relay`, or `Flood`, with a `nextHop` only when it
relays. `route` and `routes` return plain objects. A capacity that is negative or
not a whole number throws an `Error`.

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
<!-- end -->

What a full table keeps, continuing from above:

<!-- snippet: bindings/node/guides/routing.ts#limits -->
From [`bindings/node/guides/routing.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/routing.ts):

```typescript
// A table has a fixed number of slots. Once they are full, a cheaper route takes the slot of
// the costliest one held and a costlier route is refused, so a small table keeps the nodes
// nearest to it and floods to the rest.
const WELL = 11
const GATE = 12
const small = new Router(GATEWAY, 2)
small.observe(TANK, NORTH_RELAY, 3)
small.observe(SILO, SOUTH_RELAY, 5)
const kept = small
  .routes()
  .map((route) => `to ${route.dst} via ${route.nextHop} at cost ${route.cost}`)
console.log(`full      ${small.size} routes of ${small.capacity}: ${kept.join(', ')}`)
if (small.observe(WELL, EAST_RELAY, 2) && small.route(SILO) === null) {
  console.log('evicted   the well at cost 2 took the slot of the silo, the costliest held')
}
if (!small.observe(GATE, EAST_RELAY, 6) && small.forward(GATE).action === ForwardAction.Flood) {
  console.log('refused   the gate at cost 6 costs more than every route held, so it floods')
}

// A flood echoes, and the gateway hears its own packets come back through the relays. A
// route to the node itself is never learned, whatever it costs.
if (!router.observe(GATEWAY, EAST_RELAY, 2)) {
  console.log("echo      the gateway's own packet coming back teaches it nothing")
}

// A table with no slots is flooding with nothing remembered, which a node with no memory to
// spare can still do. A packet for the node itself is still delivered.
const none = new Router(GATEWAY, 0)
const learned = none.observe(PUMP, EAST_RELAY, 1)
if (
  !learned &&
  none.forward(PUMP).action === ForwardAction.Flood &&
  none.forward(GATEWAY).action === ForwardAction.Deliver
) {
  console.log(
    'no room   a table of 0 learns nothing: the pump floods, and the gateway still delivers',
  )
}
```
<!-- end -->

The site, continuing from above:

<!-- snippet: bindings/node/guides/routing.ts#site -->
From [`bindings/node/guides/routing.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/routing.ts):

```typescript
import {
  DEFAULT_HOP_LIMIT,
  type MeshFrame,
  SeenPackets,
  broadcast,
  frame,
  parse,
  relayed,
} from '@pamoja/mesh'

// Who hears whom on the site. The gateway hears the three relays, and each relay hears the
// one node beyond it; the pump is out of the gateway's range.
const site: Record<number, number[]> = {
  [GATEWAY]: [NORTH_RELAY, EAST_RELAY, SOUTH_RELAY],
  [NORTH_RELAY]: [GATEWAY, TANK],
  [EAST_RELAY]: [GATEWAY, PUMP],
  [SOUTH_RELAY]: [GATEWAY, SILO],
  [TANK]: [NORTH_RELAY],
  [PUMP]: [EAST_RELAY],
  [SILO]: [SOUTH_RELAY],
}
const nodes = Object.keys(site).map(Number)

// Sends a frame from one node and plays out what the site does with it. A node that hears a
// frame for the first time learns the way back to its source, then asks its own table what
// to do: deliver it, relay it to the one neighbor on the way, or flood it to every neighbor
// in range, spending a hop each time it goes on. Every frame here starts at the default hop
// limit, and one heard straight from its source still has all of it, so the hops a frame has
// come are what it has spent, plus one. Returns how many times a radio sent, and the nodes
// that took the frame.
function send(
  tables: Record<number, Router>,
  from: number,
  first: MeshFrame,
): [number, number[]] {
  const seen = Object.fromEntries(nodes.map((node) => [node, new SeenPackets(64)]))
  seen[from].record(first.src, first.id)
  let sends = 0
  const reached: number[] = []
  const onTheAir = [{ sender: from, sent: first }]
  for (let next = onTheAir.shift(); next; next = onTheAir.shift()) {
    const { sender, sent } = next
    const decision = tables[sender].forward(sent.dst)
    if (decision.action === ForwardAction.Deliver) {
      continue
    }
    const to = decision.nextHop === null ? site[sender] : [decision.nextHop]
    sends += 1
    for (const node of to) {
      const heard = parse(sent.bytes)
      if (!seen[node].record(heard.src, heard.id)) {
        continue
      }
      reached.push(node)
      tables[node].observe(heard.src, sender, DEFAULT_HOP_LIMIT - heard.hopLimit + 1)
      const onward = relayed(heard.bytes)
      if (onward) {
        onTheAir.push({ sender: node, sent: onward })
      }
    }
  }
  return [sends, reached]
}

// The pump floods a reading, and every node that takes it learns the way back.
const tables = Object.fromEntries(nodes.map((node) => [node, new Router(node, 8)]))
const reading = broadcast(PUMP, 1, Buffer.from('flow=12'))
const [sends, reached] = send(tables, PUMP, reading)
console.log(
  `flood     the pump's reading took ${sends} sends to reach ${reached.length} nodes, and each learned the way back`,
)

// The gateway answers the pump. Each node on the way relays to the one neighbor its table
// names, so the answer reaches only the nodes on the path.
const answer = frame(GATEWAY, PUMP, 1, Buffer.from('run=10min'))
const [routed, path] = send(tables, GATEWAY, answer)
console.log(`routed    the gateway's answer reached only ${path.join(' and ')}, in ${routed} sends`)

// The same answer on a site that has learned nothing floods, and every node relays it.
const blank = Object.fromEntries(nodes.map((node) => [node, new Router(node, 8)]))
const [flooded, everyone] = send(blank, GATEWAY, answer)
console.log(
  `flooded   with nothing learned, the same answer reached all ${everyone.length} other nodes in ${flooded} sends`,
)
```
<!-- end -->

## Python

In Python, `pamoja.routing` has a `Router` sized when it is made, 64 routes unless
told otherwise, and `len(router)` counts the routes it holds. `forward` returns a
decision whose `action` compares equal to a `ForwardAction` member and whose
`next_hop` is `None` unless it relays. `route` returns a `Route` or `None`, and
`routes` a list of them. A negative capacity raises `OverflowError`.

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
# that arrived through a relay proves that relay is a way back, at the cost the packet
# reports. The table keeps the cheapest way it has heard, and a tie keeps the way in use so
# two equal paths do not flap. Word from the relay already in use is taken even when it is
# worse, which is how a failing link lets a detour win.
router = Router(GATEWAY, 4)
for via, cost in [
    (NORTH_RELAY, 2),
    (EAST_RELAY, 1),
    (SOUTH_RELAY, 4),
    (NORTH_RELAY, 1),
    (EAST_RELAY, 3),
    (NORTH_RELAY, 2),
]:
    changed = router.observe(PUMP, via, cost)
    route = router.route(PUMP)
    outcome = "so the route is" if changed else "and the route stays"
    print(
        f"heard     the pump via {via} at cost {cost}, {outcome} {route.next_hop} "
        f"at cost {route.cost}"
    )

# The table lists what it holds, one route for each node it has heard from.
router.observe(TANK, NORTH_RELAY, 3)
held = [f"to {route.dst} via {route.next_hop} at cost {route.cost}" for route in router.routes()]
print(f"table     {len(router)} routes of {router.capacity}: {', '.join(held)}")

# Every packet gets one of three answers: deliver it here, relay it to the neighbor on the
# way, or flood it because no route is known yet.
for name, address in [("gateway", GATEWAY), ("pump", PUMP), ("silo", SILO)]:
    decision = router.forward(address)
    if decision.action == ForwardAction.DELIVER:
        print(f"{name:<10}deliver here")
    elif decision.action == ForwardAction.RELAY:
        print(f"{name:<10}relay via {decision.next_hop}")
    else:
        print(f"{name:<10}flood, no route known")

# The table keeps no clock, so a route through a relay that has gone quiet stays until the
# caller forgets it, typically when a relayed packet goes unanswered. Forgetting returns the
# node's traffic to flooding, the answer that always works.
router.forget(PUMP)
if router.forward(PUMP).action == ForwardAction.FLOOD:
    print(f"forgot    the pump, so it floods again, and {len(router)} route is left")
```
<!-- end -->

What a full table keeps, continuing from above:

<!-- snippet: bindings/python/guides/routing.py#limits -->
From [`bindings/python/guides/routing.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/routing.py):

```python
# A table has a fixed number of slots. Once they are full, a cheaper route takes the slot of
# the costliest one held and a costlier route is refused, so a small table keeps the nodes
# nearest to it and floods to the rest.
WELL = 11
GATE = 12
small = Router(GATEWAY, 2)
small.observe(TANK, NORTH_RELAY, 3)
small.observe(SILO, SOUTH_RELAY, 5)
kept = [f"to {route.dst} via {route.next_hop} at cost {route.cost}" for route in small.routes()]
print(f"full      {len(small)} routes of {small.capacity}: {', '.join(kept)}")
if small.observe(WELL, EAST_RELAY, 2) and small.route(SILO) is None:
    print("evicted   the well at cost 2 took the slot of the silo, the costliest held")
if not small.observe(GATE, EAST_RELAY, 6) and small.forward(GATE).action == ForwardAction.FLOOD:
    print("refused   the gate at cost 6 costs more than every route held, so it floods")

# A flood echoes, and the gateway hears its own packets come back through the relays. A
# route to the node itself is never learned, whatever it costs.
if not router.observe(GATEWAY, EAST_RELAY, 2):
    print("echo      the gateway's own packet coming back teaches it nothing")

# A table with no slots is flooding with nothing remembered, which a node with no memory to
# spare can still do. A packet for the node itself is still delivered.
none = Router(GATEWAY, 0)
learned = none.observe(PUMP, EAST_RELAY, 1)
if (
    not learned
    and none.forward(PUMP).action == ForwardAction.FLOOD
    and none.forward(GATEWAY).action == ForwardAction.DELIVER
):
    print("no room   a table of 0 learns nothing: the pump floods, and the gateway still delivers")
```
<!-- end -->

The site, continuing from above:

<!-- snippet: bindings/python/guides/routing.py#site -->
From [`bindings/python/guides/routing.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/routing.py):

```python
from collections import deque

from pamoja.mesh import DEFAULT_HOP_LIMIT, SeenPackets, broadcast, frame, parse, relayed

# Who hears whom on the site. The gateway hears the three relays, and each relay hears the
# one node beyond it; the pump is out of the gateway's range.
site = {
    GATEWAY: [NORTH_RELAY, EAST_RELAY, SOUTH_RELAY],
    NORTH_RELAY: [GATEWAY, TANK],
    EAST_RELAY: [GATEWAY, PUMP],
    SOUTH_RELAY: [GATEWAY, SILO],
    TANK: [NORTH_RELAY],
    PUMP: [EAST_RELAY],
    SILO: [SOUTH_RELAY],
}


def send(tables, source, first):
    """Send a frame from one node and play out what the site does with it.

    A node that hears a frame for the first time learns the way back to its source, then
    asks its own table what to do: deliver it, relay it to the one neighbor on the way, or
    flood it to every neighbor in range, spending a hop each time it goes on. Every frame
    here starts at the default hop limit, and one heard straight from its source still has
    all of it, so the hops a frame has come are what it has spent, plus one.

    :returns: How many times a radio sent, and the nodes that took the frame.
    """
    seen = {node: SeenPackets(64) for node in site}
    seen[source].record(first.src, first.id)
    sends, reached = 0, []
    on_the_air = deque([(source, first)])
    while on_the_air:
        sender, sent = on_the_air.popleft()
        decision = tables[sender].forward(sent.dst)
        if decision.action == ForwardAction.DELIVER:
            continue
        to = site[sender] if decision.next_hop is None else [decision.next_hop]
        sends += 1
        for node in to:
            heard = parse(sent.bytes)
            if not seen[node].record(heard.src, heard.id):
                continue
            reached.append(node)
            tables[node].observe(heard.src, sender, DEFAULT_HOP_LIMIT - heard.hop_limit + 1)
            onward = relayed(heard.bytes)
            if onward is not None:
                on_the_air.append((node, onward))
    return sends, reached


# The pump floods a reading, and every node that takes it learns the way back.
tables = {node: Router(node, 8) for node in site}
reading = broadcast(PUMP, 1, b"flow=12")
sends, reached = send(tables, PUMP, reading)
print(
    f"flood     the pump's reading took {sends} sends to reach {len(reached)} nodes, "
    "and each learned the way back"
)

# The gateway answers the pump. Each node on the way relays to the one neighbor its table
# names, so the answer reaches only the nodes on the path.
answer = frame(GATEWAY, PUMP, 1, b"run=10min")
routed, path = send(tables, GATEWAY, answer)
print(
    f"routed    the gateway's answer reached only {' and '.join(map(str, path))}, "
    f"in {routed} sends"
)

# The same answer on a site that has learned nothing floods, and every node relays it.
blank = {node: Router(node, 8) for node in site}
flooded, everyone = send(blank, GATEWAY, answer)
print(
    f"flooded   with nothing learned, the same answer reached all {len(everyone)} other nodes "
    f"in {flooded} sends"
)
```
<!-- end -->

## C#

In C#, a `Router` holds native state and is disposed with `using`. It is sized
when it is made, 64 routes unless told otherwise, and `Count` counts the routes it
holds. `Forward` returns a `ForwardDecision` whose `NextHop` is `null` unless the
`Action` is `ForwardAction.Relay`. `RouteTo` returns a `Route?`, and `Routes` a
read-only list of them. A negative capacity throws
`ArgumentOutOfRangeException`.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs):

```csharp
// The nodes on this mesh. An address is just a number; naming them is what makes
// the table below read as a map of the site rather than a list of numbers.
const uint Gateway = 1;
const uint Pump = 9;
const uint Tank = 10;
const uint NorthRelay = 5;
const uint EastRelay = 7;
const uint SouthRelay = 3;
const uint Silo = 32;

// A node learns the way to another from traffic it already hears: a packet from
// the pump that arrived through a relay proves that relay is a way back, at the
// cost the packet reports. The table keeps the cheapest way it has heard, and a tie
// keeps the way in use so two equal paths do not flap. Word from the relay already
// in use is taken even when it is worse, which is how a failing link lets a detour
// win.
using Router router = new(Gateway, 4);
(uint Via, ushort Cost)[] heardFromThePump =
[
    (NorthRelay, 2),
    (EastRelay, 1),
    (SouthRelay, 4),
    (NorthRelay, 1),
    (EastRelay, 3),
    (NorthRelay, 2),
];
foreach ((uint via, ushort cost) in heardFromThePump)
{
    bool changed = router.Observe(Pump, via, cost);
    Route? route = router.RouteTo(Pump);
    string outcome = changed ? "so the route is" : "and the route stays";
    Console.WriteLine(
        $"heard     the pump via {via} at cost {cost}, {outcome} {route?.NextHop} at cost {route?.Cost}");
}

// The table lists what it holds, one route for each node it has heard from.
router.Observe(Tank, NorthRelay, 3);
IEnumerable<string> held = router.Routes()
    .Select(route => $"to {route.Dst} via {route.NextHop} at cost {route.Cost}");
Console.WriteLine($"table     {router.Count} routes of {router.Capacity}: {string.Join(", ", held)}");

// Every packet gets one of three answers: deliver it here, relay it to the
// neighbor on the way, or flood it because no route is known yet.
foreach ((string name, uint address) in
    new[] { ("gateway", Gateway), ("pump", Pump), ("silo", Silo) })
{
    ForwardDecision decision = router.Forward(address);
    Console.WriteLine(decision.Action switch
    {
        ForwardAction.Deliver => $"{name,-10}deliver here",
        ForwardAction.Relay => $"{name,-10}relay via {decision.NextHop}",
        _ => $"{name,-10}flood, no route known",
    });
}

// The table keeps no clock, so a route through a relay that has gone quiet stays
// until the caller forgets it, typically when a relayed packet goes unanswered.
// Forgetting returns the node's traffic to flooding, the answer that always works.
router.Forget(Pump);
if (router.Forward(Pump).Action == ForwardAction.Flood)
{
    Console.WriteLine($"forgot    the pump, so it floods again, and {router.Count} route is left");
}
```
<!-- end -->

What a full table keeps, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs#limits -->
From [`bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs):

```csharp
// A table has a fixed number of slots. Once they are full, a cheaper route takes
// the slot of the costliest one held and a costlier route is refused, so a small
// table keeps the nodes nearest to it and floods to the rest.
const uint Well = 11;
const uint Gate = 12;
using Router small = new(Gateway, 2);
small.Observe(Tank, NorthRelay, 3);
small.Observe(Silo, SouthRelay, 5);
IEnumerable<string> kept = small.Routes()
    .Select(route => $"to {route.Dst} via {route.NextHop} at cost {route.Cost}");
Console.WriteLine($"full      {small.Count} routes of {small.Capacity}: {string.Join(", ", kept)}");
if (small.Observe(Well, EastRelay, 2) && small.RouteTo(Silo) is null)
{
    Console.WriteLine("evicted   the well at cost 2 took the slot of the silo, the costliest held");
}

if (!small.Observe(Gate, EastRelay, 6) && small.Forward(Gate).Action == ForwardAction.Flood)
{
    Console.WriteLine("refused   the gate at cost 6 costs more than every route held, so it floods");
}

// A flood echoes, and the gateway hears its own packets come back through the
// relays. A route to the node itself is never learned, whatever it costs.
if (!router.Observe(Gateway, EastRelay, 2))
{
    Console.WriteLine("echo      the gateway's own packet coming back teaches it nothing");
}

// A table with no slots is flooding with nothing remembered, which a node with no
// memory to spare can still do. A packet for the node itself is still delivered.
using Router none = new(Gateway, 0);
bool learned = none.Observe(Pump, EastRelay, 1);
if (!learned
    && none.Forward(Pump).Action == ForwardAction.Flood
    && none.Forward(Gateway).Action == ForwardAction.Deliver)
{
    Console.WriteLine(
        "no room   a table of 0 learns nothing: the pump floods, and the gateway still delivers");
}
```
<!-- end -->

The site, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs#site -->
From [`bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RoutingGuide.cs):

```csharp
// Who hears whom on the site. The gateway hears the three relays, and each relay
// hears the one node beyond it; the pump is out of the gateway's range.
var site = new Dictionary<uint, uint[]>
{
    [Gateway] = [NorthRelay, EastRelay, SouthRelay],
    [NorthRelay] = [Gateway, Tank],
    [EastRelay] = [Gateway, Pump],
    [SouthRelay] = [Gateway, Silo],
    [Tank] = [NorthRelay],
    [Pump] = [EastRelay],
    [Silo] = [SouthRelay],
};

// Sends a frame from one node and plays out what the site does with it. A node that
// hears a frame for the first time learns the way back to its source, then asks its
// own table what to do: deliver it, relay it to the one neighbor on the way, or
// flood it to every neighbor in range, spending a hop each time it goes on. Every
// frame here starts at the default hop limit, and one heard straight from its
// source still has all of it, so the hops a frame has come are what it has spent,
// plus one. Returns how many times a radio sent, and the nodes that took the frame.
(int Sends, List<uint> Reached) Send(
    Dictionary<uint, Router> tables,
    uint source,
    MeshFrame first)
{
    Dictionary<uint, SeenPackets> seen = site.Keys.ToDictionary(node => node, _ => new SeenPackets(64));
    seen[source].Record(first.Src, first.Id);
    int sends = 0;
    var reached = new List<uint>();
    var onTheAir = new Queue<(uint Sender, MeshFrame Sent)>();
    onTheAir.Enqueue((source, first));
    while (onTheAir.TryDequeue(out var next))
    {
        ForwardDecision decision = tables[next.Sender].Forward(next.Sent.Dst);
        if (decision.Action == ForwardAction.Deliver)
        {
            continue;
        }

        uint[] to = decision.NextHop is uint hop ? [hop] : site[next.Sender];
        sends++;
        foreach (uint node in to)
        {
            MeshFrame heard = Mesh.Parse(next.Sent.Bytes);
            if (!seen[node].Record(heard.Src, heard.Id))
            {
                continue;
            }

            reached.Add(node);
            ushort hops = (ushort)(Mesh.DefaultHopLimit - heard.HopLimit + 1);
            tables[node].Observe(heard.Src, next.Sender, hops);
            if (Mesh.Relayed(heard.Bytes) is { } onward)
            {
                onTheAir.Enqueue((node, onward));
            }
        }
    }

    foreach (SeenPackets cache in seen.Values)
    {
        cache.Dispose();
    }

    return (sends, reached);
}

// The pump floods a reading, and every node that takes it learns the way back.
Dictionary<uint, Router> tables = site.Keys.ToDictionary(node => node, node => new Router(node, 8));
MeshFrame reading = Mesh.BroadcastFrame(Pump, 1, "flow=12"u8);
(int sends, List<uint> reached) = Send(tables, Pump, reading);
Console.WriteLine(
    $"flood     the pump's reading took {sends} sends to reach {reached.Count} nodes, and each learned the way back");

// The gateway answers the pump. Each node on the way relays to the one neighbor its
// table names, so the answer reaches only the nodes on the path.
MeshFrame answer = Mesh.Frame(Gateway, Pump, 1, "run=10min"u8);
(int routed, List<uint> path) = Send(tables, Gateway, answer);
Console.WriteLine(
    $"routed    the gateway's answer reached only {string.Join(" and ", path)}, in {routed} sends");

// The same answer on a site that has learned nothing floods, and every node relays
// it.
Dictionary<uint, Router> blank = site.Keys.ToDictionary(node => node, node => new Router(node, 8));
(int flooded, List<uint> everyone) = Send(blank, Gateway, answer);
Console.WriteLine(
    $"flooded   with nothing learned, the same answer reached all {everyone.Count} other nodes in {flooded} sends");
```
<!-- end -->

## Values at a glance

**What a report does to the table,** where a report is a packet from a node,
the neighbor it came through, and its cost:

| The report | The table | `observe` returns |
| --- | --- | --- |
| a node with no route held, and a slot free | adds the route | true |
| a node with no route held, the table full, and the report cheaper than the costliest route held | gives that route's slot to the new one | true |
| a node with no route held, the table full, and the report as costly as every route held or more | refuses it | false |
| cheaper than the route held | takes it, through the new neighbor | true |
| through the neighbor the route already uses | takes its cost, better or worse | true, unless the cost is the same |
| as costly or costlier, through another neighbor | keeps the route held, so equal paths do not flap | false |
| about this node itself | ignores it | false |

**What a packet gets:**

| The packet is for | `forward` answers |
| --- | --- |
| this node | deliver |
| a node the table holds a route to | relay, to that route's next hop |
| any other node, a broadcast included | flood |

**The table:**

| | Value |
| --- | --- |
| Routes a binding's table holds when nothing says otherwise | 64 |
| Routes a Rust `Router<N>` holds | `N`, fixed in the type |
| Memory of a `Router<N>` on a 32-bit or 64-bit target | 16 bytes a route and 4 for the node's address: 1,028 bytes for 64 routes |
| Work per call | a scan of every slot, so it grows with the capacity |
| A cost | 0 to 65,535, lower is better, in whatever unit the caller reports |
| An address | 32 bits, the same as a mesh frame's source and destination |
| Order `routes` lists them in | the table's own: a new route takes the first free slot |
| When a route expires | never on its own: it stays until it is forgotten or displaced |

**What the pump's flood taught the site.** Each node took its cost from the hops
the frame had come, the default hop limit of 3 less what was left of it, plus one:

| Node | Next hop to the pump | Cost |
| --- | --- | --- |
| east relay, 7 | the pump, 9 | 1 |
| gateway, 1 | east relay, 7 | 2 |
| north relay, 5 | gateway, 1 | 3 |
| south relay, 3 | gateway, 1 | 3 |
| tank, 10 | north relay, 5 | 4 |
| silo, 32 | south relay, 3 | 4 |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| make a table | `Router::<N>::new(address)`, `DynamicRouter::new(address, capacity)` |
| learn a route | `observe(origin, via, cost)`, `true` if the table changed |
| decide for a packet | `forward(dst)`: `Forward::Deliver`, `Forward::Relay(next_hop)`, or `Forward::Flood` |
| read a route | `route(dst)`, `next_hop(dst)`, `cost(dst)`, each `None` without one; `routes()` for every one |
| drop a route | `forget(dst)` |
| size it up | `len()`, `is_empty()`, `capacity()`, `address()` |

### TypeScript

| To | Call |
| --- | --- |
| make a table | `new Router(address, capacity?)`, `router(address, capacity?)`, `DEFAULT_CAPACITY` |
| learn a route | `observe(origin, via, cost)`, `true` if the table changed |
| decide for a packet | `forward(dst)`, with an `action` of `ForwardAction.Deliver`, `Relay`, or `Flood`, and a `nextHop` |
| read a route | `route(dst)`, `nextHop(dst)`, `cost(dst)`, each `null` without one; `routes()` for every one |
| drop a route | `forget(dst)` |
| size it up | `size`, `isEmpty`, `capacity`, `address` |

### Python

| To | Call |
| --- | --- |
| make a table | `Router(address, capacity=64)`, `router(address, capacity=DEFAULT_CAPACITY)` |
| learn a route | `observe(origin, via, cost)`, `True` if the table changed |
| decide for a packet | `forward(dst)`, with an `action` equal to `ForwardAction.DELIVER`, `RELAY`, or `FLOOD`, and a `next_hop` |
| read a route | `route(dst)`, `next_hop(dst)`, `cost(dst)`, each `None` without one; `routes()` for every one |
| drop a route | `forget(dst)` |
| size it up | `len(router)`, `capacity`, `address` |

### C#

| To | Call |
| --- | --- |
| make a table | `new Router(address, capacity = 64)` |
| learn a route | `Observe(origin, via, cost)`, `true` if the table changed |
| decide for a packet | `Forward(dst)`, with an `Action` of `ForwardAction.Deliver`, `Relay`, or `Flood`, and a `NextHop` |
| read a route | `RouteTo(dst)`, `NextHop(dst)`, `Cost(dst)`, each `null` without one; `Routes()` for every one |
| drop a route | `Forget(dst)` |
| size it up | `Count`, `Capacity`, `Address` |

<!-- languages end -->

## When it goes wrong

The table never fails a call, so its mistakes show up as packets that go the
wrong way or nowhere. The ones that cost an afternoon:

- **Packets keep going to a relay that has died.** The table keeps no clock, so a
  route through a failed relay stays, and every packet for that node goes to it.
  Forget the route when a relayed packet goes unanswered; the next packet floods
  and teaches the way that still works.
- **Answers vanish over a link that only works one way.** Hearing a neighbor
  proves it reaches this node, not that this node reaches it back. A gateway with
  a stronger radio than its sensors is the usual case. Keep transmit power alike
  across the mesh, and forget a route whose relayed packets go unanswered.
- **The router has no neighbor to learn from.** A route needs the neighbor a
  packet came through, which the link has to say. ESP-NOW reports the sending
  radio's MAC with every packet. A LoRa radio reports nothing about the sender,
  and a mesh frame names its source, not its last relay, so on LoRa the relay's
  address has to travel in the payload.
- **The route flaps with every packet.** Any strictly cheaper report takes the
  route, so a cost built from signal strength that wobbles by one switches relays
  back and forth. Count hops, or round the measure into coarse steps.
- **A better path is ignored, or a worse one wins.** Costs are only ever compared
  with each other, so every node has to report the same kind. A relay that counts
  hops and another that reports signal strength are compared as if they meant the
  same thing.
- **Costs come out wrong after one node changes its hop limit.** The site takes a
  cost from how much of the hop limit a frame has spent, which holds only while
  every node starts its frames at the same limit. A node that starts higher looks
  closer than it is. Keep one hop limit across the mesh, or carry the cost in the
  payload.
- **A packet bounces between two nodes.** Two tables that each route through the
  other, left behind by a failure, pass a packet back and forth; the table has no
  loop check. Carry routed traffic in mesh frames and relay each with `relayed`,
  so the hop limit ends a loop the way it ends a flood.
- **Far nodes always flood.** A full table refuses a route costlier than every one
  it holds, so on a crowded mesh the far nodes are the ones left out. Size the
  table for the nodes this one sends to: the whole fleet on a gateway, a handful on
  a sensor that only talks to the gateway.

## Where next

<!-- table: next routing -->
- [Mesh frames](mesh.md): Addressed, hop-limited, CRC-checked frames and duplicate suppression that floods a packet exactly once.
- [Transport ladder](ladder.md): Cheapest reachable link first, buffering to a store when every link is down.
- [Store and forward](sync.md): Offline-first queues in memory or on disk, bounded or not, the on-disk one surviving power loss, and the drain that forwards them in order when a link returns.
- Also in Radio and reach: [LoRa airtime and range](lora.md), [LoRaWAN](lorawan.md), [LoRa radios](radios.md), [LoRaWAN gateways](gateway.md).
<!-- end -->

## Reference

<!-- table: reference routing -->
- Rust: [`pamoja-routing`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-routing)
- TypeScript: [`@pamoja/routing`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-routing)
- Python: [`pamoja.routing`](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-routing)
- C#: [`Pamoja.Routing`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-routing)
<!-- end -->
