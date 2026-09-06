# pamoja-routing

Reverse-path routing that learns the cheapest route from overheard traffic. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/routing.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
