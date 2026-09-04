# pamoja-routing

Reverse-path routing that learns the cheapest route from overheard traffic. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-routing`](https://crates.io/crates/pamoja-routing) | [docs.rs](https://docs.rs/pamoja-routing), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html) |
| TypeScript | [`@pamoja/routing`](https://www.npmjs.com/package/@pamoja/routing) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html) |
| Python | [`pamoja-routing`](https://pypi.org/project/pamoja-routing/) | [`pamoja.routing`](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html) |
| C# | [`Pamoja.Routing`](https://www.nuget.org/packages/Pamoja.Routing) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.Router.html) |

## Documentation

- [The Routing guide](https://pamoja.molex.cloud/docs/guides/routing.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
