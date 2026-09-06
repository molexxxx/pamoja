# Pamoja.Routing

Reverse-path routing that learns the cheapest route from overheard traffic. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/routing.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
dotnet add package Pamoja.Routing
```

```csharp
using Pamoja.Routing;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-routing`](https://crates.io/crates/pamoja-routing) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_routing/index.html), [docs.rs](https://docs.rs/pamoja-routing), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-routing) |
| TypeScript | [`@pamoja/routing`](https://www.npmjs.com/package/@pamoja/routing) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_routing.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-routing) |
| Python | [`pamoja-routing`](https://pypi.org/project/pamoja-routing/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/routing.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-routing) |
| C# | [`Pamoja.Routing`](https://www.nuget.org/packages/Pamoja.Routing) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-routing) |

## Documentation

- [`Pamoja.Routing` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html), every type in this namespace.
- [The Routing guide](https://pamoja.molex.cloud/docs/guides/routing.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
