# Pamoja.Profile

Named, ready-to-run device profiles from plain data or a JSON manifest. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/profile.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html)

## Install

```sh
dotnet add package Pamoja.Profile
```

```csharp
using Pamoja.Profile;
```

This pulls in `Pamoja.Native`, the compiled engine, and `Pamoja.Core`, `Pamoja.Kit` and `Pamoja.Power`. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs):

```csharp
// A profile is a file. This one ships in the catalog under profiles/: it holds a
// brooder at 32 C by switching a heat lamp, says what it reads, and says how a
// dashboard draws it.
string text = File.ReadAllText("profiles/brooder-heater.json");
using var profile = Profile.FromJson(text);
Reads reads = profile.Reads!.Value;
Console.WriteLine($"profile   {profile.Name} reads {reads.Quantity} in {reads.Unit} and reports on {profile.Topic}");

// A node is the profile and the parts that make it run: a sensor, an output, and a
// link. A morning of readings stands in for the probe, and a dashboard listens on the
// same broker.
using var broker = new LoopbackBroker();
using var link = broker.Link();
using var dashboard = broker.Link();
await link.ConnectAsync();
await dashboard.ConnectAsync();
await dashboard.SubscribeAsync(profile.Topic);
var morning = new Queue<float>([27.5f, 31.8f, 32.6f, 32.1f, 31.4f]);
bool lamp = false;
using var node = new Node(
    profile,
    () => ValueTask.FromResult(morning.Dequeue()),
    link,
    drive: on =>
    {
        lamp = on;
        return ValueTask.CompletedTask;
    });

// Each tick reads, decides, switches the lamp, and publishes the reading. The lamp
// comes on at 31.5 C or below and goes off at 32.5 C or above, and in between it stays
// as it was; a reading more than 4 C from 32 raises an alert as well.
bool was = false;
for (int at = 0; at < 5; at++)
{
    Tick tick = await node.TickAsync();
    bool on = tick.Reaction.Actuator == true;
    string change = on ? (was ? "lamp stays on" : "lamp on") : was ? "lamp off" : "lamp stays off";
    string alert = tick.Reaction.Alert is { } raised ? $", alert {raised.Kind}" : string.Empty;
    Console.WriteLine(Invariant($"{Invariant($"{tick.Reading} C"),-10}{change}{alert}"));
    was = on;
}

// The dashboard heard every reading the node published.
var heard = new List<string>();
for (int at = 0; at < 5; at++)
{
    TransportMessage? message = await dashboard.ReceiveAsync(TimeSpan.FromSeconds(5));
    heard.Add(Invariant($"{message!.Number}"));
}

Console.WriteLine($"heard     {string.Join(", ", heard)} on {profile.Topic}");

// Between ticks the node waits as long as its battery allows: often on a healthy
// charge, sparingly on a low one. RunAsync does this until cancelled, waiting each
// interval.
foreach (float charge in new[] { 0.8f, 0.3f, 0.1f })
{
    (var mode, TimeSpan wait) = node.Schedule(charge);
    Console.WriteLine(Invariant($"battery   at {charge * 100:F0}% it runs {mode} and waits {wait.TotalSeconds} s"));
}

// The same file says how a dashboard draws the node.
ElementSpec element = profile.Presentation!.Elements[0];
string graphic = element.Viz.ToString().ToLowerInvariant();
Console.WriteLine(Invariant(
    $"draws     {element.Key} in {element.Unit} on a {graphic}, safe from {element.Band![0]} to {element.Band[1]}"));
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-profile`](https://crates.io/crates/pamoja-profile) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [docs.rs](https://docs.rs/pamoja-profile), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-profile) |
| TypeScript | [`@pamoja/profile`](https://www.npmjs.com/package/@pamoja/profile) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-profile) |
| Python | [`pamoja-profile`](https://pypi.org/project/pamoja-profile/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-profile) |
| C# | [`Pamoja.Profile`](https://www.nuget.org/packages/Pamoja.Profile) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-profile) |

## Documentation

- [`Pamoja.Profile` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), every type in this namespace.
- [The Device profiles guide](https://pamoja.molex.cloud/docs/guides/profile.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
