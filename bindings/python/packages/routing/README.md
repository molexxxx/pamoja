# pamoja-routing

Reverse-path routing that learns the cheapest route from overheard traffic. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/routing.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html)

## Install

```sh
pip install pamoja-routing
```

```python
from pamoja import routing
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-routing`](https://crates.io/crates/pamoja-routing) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html), [docs.rs](https://docs.rs/pamoja-routing), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-routing) |
| TypeScript | [`@pamoja/routing`](https://www.npmjs.com/package/@pamoja/routing) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-routing) |
| Python | [`pamoja-routing`](https://pypi.org/project/pamoja-routing/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-routing) |
| C# | [`Pamoja.Routing`](https://www.nuget.org/packages/Pamoja.Routing) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-routing) |

## Documentation

- [`pamoja.routing` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html), every class and function in this module.
- [The Routing guide](https://pamoja.molex.cloud/docs/guides/routing.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
