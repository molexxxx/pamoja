# Pamoja.Mavlink

MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mavlink.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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

// The values the MAVLink common dialect gives these fields.
const byte MavTypeGcs = 6;
const byte MavTypeQuadrotor = 2;
const byte MavAutopilotInvalid = 8;
const byte MavAutopilotArdupilotmega = 3;
const byte MavStateActive = 4;
const byte MavStateStandby = 3;
const ushort MavCmdComponentArmDisarm = 400;
const ushort MavCmdNavTakeoff = 22;
const byte MavResultAccepted = 0;

// Every MAVLink node broadcasts a heartbeat to say what it is and that it is
// alive. The fields are set by name rather than by writing the payload out byte
// by byte.
using MavlinkSchema heartbeatShape = MavlinkSchema.ForName("HEARTBEAT");
using MavlinkMessage announce = heartbeatShape.CreateMessage();
announce.Set("type", MavTypeGcs);
announce.Set("autopilot", MavAutopilotInvalid);
announce.Set("system_status", MavStateActive);
announce.Set("mavlink_version", 3);
using MavlinkFrame sent = announce.ToFrame(new MavlinkHeader(Station, 190, 0));
Console.WriteLine($"sent      HEARTBEAT in {sent.Bytes.Length} bytes");

// The vehicle answers with its own heartbeat. This copy arrives after some bytes
// that were already on the wire, and after a copy with one bit flipped in flight.
using MavlinkMessage vehicle = heartbeatShape.CreateMessage();
vehicle.Set("type", MavTypeQuadrotor);
vehicle.Set("autopilot", MavAutopilotArdupilotmega);
vehicle.Set("system_status", MavStateStandby);
vehicle.Set("mavlink_version", 3);
using MavlinkFrame good = vehicle.ToFrame(new MavlinkHeader(Vehicle, Autopilot, 0));
byte[] garbled = [.. good.Bytes];
garbled[^1] ^= 0xFF;
byte[] delivered = [.. "???"u8, .. garbled, .. good.Bytes];

// The parser skips whatever does not start a frame and drops one whose checksum
// fails, so the frame it hands back is the good copy rather than the garbled one.
using MavlinkParser parser = new();
using MavlinkFrame received = parser.Push(delivered)[0];
using MavlinkMessage heard = heartbeatShape.Decode(received.Payload);
Console.WriteLine(
    $"heard     a type-{heard.Get("type")} vehicle in state {heard.Get("system_status")}");

// Arming it is a command, not a message a sender fires and forgets: the vehicle
// has to answer, and the sender keeps asking until it does. The protocol numbers
// each resend, which is how a vehicle tells a retry from a deliberate second one.
using MavlinkCommand arming = new(MavCmdComponentArmDisarm, 3);
using MavlinkSchema commandShape = MavlinkSchema.ForName("COMMAND_LONG");
using MavlinkMessage arm = commandShape.CreateMessage();
arm.Set("param1", 1.0); // 1 arms, 0 disarms
arm.Set("target_system", Vehicle);
arm.Set("target_component", Autopilot);
arm.Set("command", arming.Command);
arm.Set("confirmation", arming.Confirmation);
Console.WriteLine($"sent      arm request, confirmation {arming.Confirmation}");
arm.ToFrame(new MavlinkHeader(Station, 190, 1)).Dispose();

// Nothing comes back in time, so it goes again with the next confirmation number.
byte? resend = arming.OnTimeout();
Console.WriteLine($"silence, resending with confirmation {resend}");

// An acknowledgement names the command it answers, so one for a different command
// is not this exchange finishing.
using MavlinkSchema ackShape = MavlinkSchema.ForName("COMMAND_ACK");
MavlinkAckOutcome? stray = Acknowledge(ackShape, arming, MavCmdNavTakeoff);
Console.WriteLine($"an ack for another command: {stray?.Kind}");

MavlinkAckOutcome? outcome = Acknowledge(ackShape, arming, MavCmdComponentArmDisarm);
Console.WriteLine(
    outcome?.Kind == MavlinkAckKind.Final && outcome?.Value == MavResultAccepted
        ? "armed     the vehicle is ready"
        : $"the vehicle answered {outcome?.Kind} {outcome?.Value}");

static MavlinkAckOutcome? Acknowledge(
    MavlinkSchema shape,
    MavlinkCommand tracked,
    ushort command)
{
    using MavlinkMessage ack = shape.CreateMessage();
    ack.Set("command", command);
    ack.Set("result", 0);
    using MavlinkFrame frame = ack.ToFrame(new MavlinkHeader(1, 1, 0));
    return tracked.OnFrame(frame);
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mavlink`](https://crates.io/crates/pamoja-mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html), [docs.rs](https://docs.rs/pamoja-mavlink) |
| TypeScript | [`@pamoja/mavlink`](https://www.npmjs.com/package/@pamoja/mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html) |
| Python | [`pamoja-mavlink`](https://pypi.org/project/pamoja-mavlink/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html) |
| C# | [`Pamoja.Mavlink`](https://www.nuget.org/packages/Pamoja.Mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html) |

## Documentation

- [`Pamoja.Mavlink` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html), every type in this namespace.
- [The MAVLink guide](https://pamoja.molex.cloud/docs/guides/mavlink.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
