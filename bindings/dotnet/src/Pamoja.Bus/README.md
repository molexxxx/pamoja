# Pamoja.Bus

An in-memory typed publish and subscribe event bus, with publishers that never wait and subscribers that count what they miss. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/bus.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html)

## Install

```sh
dotnet add package Pamoja.Bus
```

```csharp
using Pamoja.Bus;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Codec`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/BusGuide.cs):

```csharp
// The station's wiring makes one bus and hands each part what it needs: a
// publisher to announce, an endpoint to listen. No part holds a reference to
// another, so any of them can be replaced without touching the rest.
using var bus = new EventPublisher(2);
using EventPublisher power = bus.Publisher();
using EventPublisher sampler = bus.Publisher();
using EventBus heater = bus.Subscribe();
using EventBus logger = bus.Subscribe();

// One announcement reaches every part that listens, and each reads its own copy.
int reached = power.Publish("battery.low");
Console.WriteLine($"power     handed battery.low to {reached} parts");
string heaterTook = await heater.NextTextAsync();
Console.WriteLine($"heater    took {heaterTook}");
string loggerTook = await logger.NextTextAsync();
Console.WriteLine($"logger    took {loggerTook}");

// Publishing never waits, even while the part's own wait is open, and a part
// hears what it publishes.
Task<string> waiting = heater.NextTextAsync();
heater.Publish("heater.off");
string heard = await waiting;
Console.WriteLine($"heater    heard its own {heard}, sent while it waited");

// A part that joins late sees only what is published after it subscribes.
// There is no history to replay.
using EventBus radio = bus.Subscribe();
power.Publish("battery.ok");
string first = await radio.NextTextAsync();
Console.WriteLine($"radio     joined late, so the first event it sees is {first}");

// Each endpoint buffers two events. The logger, busy writing to flash, falls
// behind while the sampler publishes five readings: it loses the oldest
// events, resumes with the newest, and counts what it lost.
for (int reading = 0; reading < 5; reading++)
{
    sampler.Publish($"wind {reading}");
}

string resumed = await logger.NextTextAsync();
long missed = logger.Missed;
Console.WriteLine($"logger    missed {missed} and resumes at {resumed}");
string newest = await logger.NextTextAsync();
Console.WriteLine($"logger    then took {newest}");

// A wait with a limit gives up without taking anything, so a part can do
// other work between events and lose nothing by it.
TimeSpan quiet = TimeSpan.FromMilliseconds(50);
try
{
    await logger.NextTextAsync(quiet);
    Console.WriteLine("logger    took an event no one published, which should never happen");
}
catch (TimeoutException)
{
    Console.WriteLine($"logger    heard nothing more within {quiet.TotalMilliseconds} ms");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-bus`](https://crates.io/crates/pamoja-bus) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_bus/index.html), [docs.rs](https://docs.rs/pamoja-bus), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-bus) |
| TypeScript | [`@pamoja/bus`](https://www.npmjs.com/package/@pamoja/bus) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_bus.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-bus) |
| Python | [`pamoja-bus`](https://pypi.org/project/pamoja-bus/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/bus.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-bus) |
| C# | [`Pamoja.Bus`](https://www.nuget.org/packages/Pamoja.Bus) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-bus) |

## Documentation

- [`Pamoja.Bus` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Bus.html), every type in this namespace.
- [The Event bus guide](https://pamoja.molex.cloud/docs/guides/bus.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
