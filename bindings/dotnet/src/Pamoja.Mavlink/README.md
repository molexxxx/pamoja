# Pamoja.Mavlink

MAVLink v1 and v2 framing, signing, named message fields and enum values, and the mission, command, and offboard protocols. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mavlink.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html)

## Install

```sh
dotnet add package Pamoja.Mavlink
```

```csharp
using Pamoja.Mavlink;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs):

```csharp
const byte Vehicle = 1;
const byte Autopilot = 1;
const byte Station = 255;
const byte Planner = 190;

// An enum field travels as a number, and the dialect names each number. Printing
// the name keeps a reader from looking up what 2 or 81 means.
static string Name(string enumeration, double value) =>
    MavlinkEnum.Entry(enumeration, (ulong)value) ?? Invariant($"{value}");

// Every node broadcasts a heartbeat to say what it is and that it is alive. The
// frame wraps the payload in a header and a checksum seeded with the message's own
// value.
using MavlinkSchema heartbeatShape = MavlinkSchema.ForName("HEARTBEAT");
using MavlinkMessage announce = heartbeatShape.CreateMessage();
announce.Set("type", MavlinkEnum.Value("MAV_TYPE_GCS"));
announce.Set("autopilot", MavlinkEnum.Value("MAV_AUTOPILOT_INVALID"));
announce.Set("system_status", MavlinkEnum.Value("MAV_STATE_ACTIVE"));
announce.Set("mavlink_version", 3);
using MavlinkFrame sent = announce.ToFrame(new MavlinkHeader(Station, Planner, 0));
Console.WriteLine($"sent      {heartbeatShape.Name} in {sent.Bytes.Length} bytes");

// The vehicle answers with its own heartbeat, which reaches the station behind some
// noise and a copy with its last byte flipped in flight.
using MavlinkMessage vehicle = heartbeatShape.CreateMessage();
vehicle.Set("type", MavlinkEnum.Value("MAV_TYPE_QUADROTOR"));
vehicle.Set("autopilot", MavlinkEnum.Value("MAV_AUTOPILOT_ARDUPILOTMEGA"));
vehicle.Set(
    "base_mode",
    MavlinkEnum.Value("MAV_MODE_FLAG_CUSTOM_MODE_ENABLED")
        | MavlinkEnum.Value("MAV_MODE_FLAG_STABILIZE_ENABLED")
        | MavlinkEnum.Value("MAV_MODE_FLAG_MANUAL_INPUT_ENABLED"));
vehicle.Set("system_status", MavlinkEnum.Value("MAV_STATE_STANDBY"));
vehicle.Set("mavlink_version", 3);
using MavlinkFrame good = vehicle.ToFrame(new MavlinkHeader(Vehicle, Autopilot, 0));
byte[] garbled = [.. good.Bytes];
garbled[^1] ^= 0xFF;
byte[] delivered = [.. "???"u8, .. garbled, .. good.Bytes];

// The parser skips whatever does not start a frame and drops a frame whose checksum
// fails, so only the good copy comes out.
using MavlinkParser parser = new();
IReadOnlyList<MavlinkFrame> frames = parser.Push(delivered);
Console.WriteLine(
    $"parsed    {frames.Count} frame out of {delivered.Length} bytes,"
    + " past the noise and the garbled copy");
using MavlinkMessage heard = heartbeatShape.Decode(frames[0].Payload);
Console.WriteLine(
    $"heard     {Name("MAV_TYPE", heard.Get("type"))} on"
    + $" {Name("MAV_AUTOPILOT", heard.Get("autopilot"))},"
    + $" in {Name("MAV_STATE", heard.Get("system_status"))}");

// The base mode is a bitmask, so it names a set of flags rather than one value.
ulong baseMode = (ulong)heard.Get("base_mode");
Console.WriteLine(
    $"flags     {string.Join(" | ", MavlinkEnum.Names("MAV_MODE_FLAG", baseMode))}");
if ((baseMode & MavlinkEnum.Value("MAV_MODE_FLAG_SAFETY_ARMED")) == 0)
{
    Console.WriteLine("disarmed  MAV_MODE_FLAG_SAFETY_ARMED is not among them");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mavlink`](https://crates.io/crates/pamoja-mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html), [docs.rs](https://docs.rs/pamoja-mavlink), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mavlink) |
| TypeScript | [`@pamoja/mavlink`](https://www.npmjs.com/package/@pamoja/mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mavlink) |
| Python | [`pamoja-mavlink`](https://pypi.org/project/pamoja-mavlink/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mavlink) |
| C# | [`Pamoja.Mavlink`](https://www.nuget.org/packages/Pamoja.Mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mavlink) |

## Documentation

- [`Pamoja.Mavlink` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html), every type in this namespace.
- [The MAVLink guide](https://pamoja.molex.cloud/docs/guides/mavlink.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
