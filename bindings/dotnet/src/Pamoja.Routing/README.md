# Pamoja.Routing

Reverse-path routing that learns the cheapest route from overheard traffic. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/routing.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Routing.html)

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
