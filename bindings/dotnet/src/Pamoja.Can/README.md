# Pamoja.Can

CAN 2.0 and CAN FD frames with 11- and 29-bit identifiers, J1939 decode and compose, and a node on a bus, simulated or a Linux interface through SocketCAN. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/can.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html)

## Install

```sh
dotnet add package Pamoja.Can
```

```csharp
using Pamoja.Can;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/CanGuide.cs):

```csharp
// The nodes by the address each answers to, and the two parameter groups in play.
const byte Engine = 0;
const byte Gateway = 1;
const uint EngineController1 = 61_444; // carries engine speed
const uint Request = 59_904; // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for it.
const int EngineSpeedAt = 3;
const double RpmPerBit = 0.125;

// J1939 keeps its addressing inside the 29-bit identifier: a priority, the parameter
// group, and the sender's address. A broadcast names no destination.
uint speedId = Can.BroadcastJ1939(J1939Priority.Control, EngineController1, Engine);
J1939Message speed = Can.DecodeJ1939(speedId)!;
Console.WriteLine(
    $"engine speed 0x{speedId:X8}: pgn {speed.Pgn} at priority {speed.Priority}, from node {Engine} to every node");

// A reading starts with every signal marked not available, and the engine writes only
// its speed.
CanFrame Reading(double rpm)
{
    Signals reported = Signals.New();
    reported.SetU16(EngineSpeedAt, (ushort)(rpm / RpmPerBit));
    return Can.Frame(speedId, reported.ToArray(), extended: true);
}

static double RpmOf(CanFrame received) =>
    (Signals.From(received.Data).U16(EngineSpeedAt) ?? 0) * RpmPerBit;

CanFrame first = Reading(1500);
int unreported = first.Data.Count(value => value == Signals.NotAvailable);
Console.WriteLine(Invariant(
    $"payload      {RpmOf(first):F1} rpm in bytes {EngineSpeedAt + 1} and {EngineSpeedAt + 2}, the other {unreported} not available"));

// Four nodes on one bus with nothing plugged in. On a Linux board each is
// CanBus.Open("can0"), and nothing after this statement changes.
using CanBus engine = CanBus.Simulated();
using CanBus gateway = engine.Join();
using CanBus laptop = engine.Join();
using CanBus sensor = engine.Join();

// The gateway keeps engine speed and nothing else; the laptop keeps everything.
gateway.SetFilters(CanFilter.Pgn(EngineController1));

// Two engine readings, and between them the coolant sensor, which speaks plain CAN: its
// level in percent on the 11-bit identifier 0x120.
engine.Send(first);
sensor.Send(Can.Frame(0x120, [87]));
engine.Send(Reading(1512.5));

// Every node hears every frame but its own, and keeps what its filters pass.
while (gateway.Receive(TimeSpan.FromMilliseconds(10)) is { } kept)
{
    byte from = Can.DecodeJ1939(kept.Id, kept.Extended)?.Source ?? 0;
    Console.WriteLine(Invariant($"gateway      {RpmOf(kept):F1} rpm from node {from}"));
}

long onTheBus = engine.Sent + sensor.Sent;
Console.WriteLine($"gateway      kept {gateway.Received} of the {onTheBus} frames on the bus");
var heard = new List<CanFrame>();
while (laptop.Receive(TimeSpan.FromMilliseconds(10)) is { } received)
{
    heard.Add(received);
}

if (heard.Find(received => Can.DecodeJ1939(received.Id, received.Extended) is null) is { } plain)
{
    Console.WriteLine(
        $"laptop       heard {heard.Count}, among them 0x{plain.Id:X3}, an 11-bit identifier and no J1939 message");
}

// A request is addressed rather than broadcast: below the PDU1 limit, eight bits of the
// identifier name the node it is for.
J1939Message request = Can.DecodeJ1939(
    Can.ComposeJ1939((byte)J1939Priority.Normal, Request, Gateway, Engine))!;
Console.WriteLine($"request      pgn {request.Pgn} from node {request.Source} to node {request.Destination}");

// The engine goes quiet. A receive waits for a frame up to its timeout; on a simulated
// bus it returns at once and counts the wait instead of sleeping through it.
ulong before = gateway.WaitedMicros;
CanFrame? quiet = gateway.Receive(TimeSpan.FromMilliseconds(500));
Console.WriteLine(
    $"silent       {(quiet is null ? 0 : 1)} frames in {(gateway.WaitedMicros - before) / 1_000} ms, counted and not slept");

// Above eight bytes CAN FD encodes a length in steps, and a classic frame refuses a
// ninth byte.
CanFrame wide = Can.FdFrame(speedId, new byte[32], extended: true);
Console.WriteLine($"fd           32 bytes travel at data length code {wide.Dlc}");
try
{
    Can.Frame(speedId, new byte[9], extended: true);
}
catch (PamojaException error)
{
    Console.WriteLine($"classic      refused nine bytes: {error.Message}");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-can`](https://crates.io/crates/pamoja-can) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_can/index.html), [docs.rs](https://docs.rs/pamoja-can), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-can) |
| TypeScript | [`@pamoja/can`](https://www.npmjs.com/package/@pamoja/can) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_can.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-can) |
| Python | [`pamoja-can`](https://pypi.org/project/pamoja-can/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/can.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-can) |
| C# | [`Pamoja.Can`](https://www.nuget.org/packages/Pamoja.Can) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-can) |

## Documentation

- [`Pamoja.Can` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Can.html), every type in this namespace.
- [The CAN and J1939 guide](https://pamoja.molex.cloud/docs/guides/can.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
