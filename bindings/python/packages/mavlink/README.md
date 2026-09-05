# pamoja-mavlink

MAVLink v1 and v2 framing, signing, named message fields, and the mission, command, and offboard protocols. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mavlink.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-mavlink
```

```python
from pamoja import mavlink
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/mavlink.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mavlink.py):

```python
from pamoja.mavlink import (
    CommandProtocol,
    MavlinkHeader,
    MavlinkMessage,
    MavlinkParser,
    from_dict,
    message,
    schema_for,
)

VEHICLE = 1
AUTOPILOT = 1
STATION = 255

# The values the MAVLink common dialect gives these fields.
MAV_TYPE_GCS = 6
MAV_TYPE_QUADROTOR = 2
MAV_AUTOPILOT_INVALID = 8
MAV_AUTOPILOT_ARDUPILOTMEGA = 3
MAV_STATE_ACTIVE = 4
MAV_STATE_STANDBY = 3
MAV_CMD_COMPONENT_ARM_DISARM = 400
MAV_CMD_NAV_TAKEOFF = 22
MAV_RESULT_ACCEPTED = 0

# Every MAVLink node broadcasts a heartbeat to say what it is and that it is alive. The
# fields are set by name rather than by writing the payload out byte by byte.
announce = message("HEARTBEAT")
announce.set("type", MAV_TYPE_GCS)
announce.set("autopilot", MAV_AUTOPILOT_INVALID)
announce.set("system_status", MAV_STATE_ACTIVE)
announce.set("mavlink_version", 3)
sent = announce.to_frame(MavlinkHeader(STATION, 190, 0))
print(f"sent      HEARTBEAT in {len(sent.bytes)} bytes")

# The vehicle answers with its own heartbeat. This copy arrives after some bytes that were
# already on the wire, and after a copy with one bit flipped in flight.
vehicle_shape = schema_for("HEARTBEAT")
vehicle = from_dict(
    vehicle_shape,
    {
        "type": MAV_TYPE_QUADROTOR,
        "autopilot": MAV_AUTOPILOT_ARDUPILOTMEGA,
        "system_status": MAV_STATE_STANDBY,
        "mavlink_version": 3,
    },
)
good = vehicle.to_frame(MavlinkHeader(VEHICLE, AUTOPILOT, 0))
garbled = bytearray(good.bytes)
garbled[-1] ^= 0xFF
delivered = b"???" + bytes(garbled) + good.bytes

# The parser skips whatever does not start a frame and drops one whose checksum fails, so
# the frame it hands back is the good copy rather than the garbled one.
parser = MavlinkParser()
received = parser.push(delivered)[0]
heard = MavlinkMessage.decode(vehicle_shape, received.payload)
print(f"heard     a type-{heard.get_int('type')} vehicle in state {heard.get_int('system_status')}")

# Arming it is a command, not a message a sender fires and forgets: the vehicle has to
# answer, and the sender keeps asking until it does. The protocol numbers each resend,
# which is how a vehicle tells a retry from a second, deliberate command.
arming = CommandProtocol(MAV_CMD_COMPONENT_ARM_DISARM, 3)
command_shape = schema_for("COMMAND_LONG")
arm = from_dict(
    command_shape,
    {
        "param1": 1.0,  # 1 arms, 0 disarms
        "target_system": VEHICLE,
        "target_component": AUTOPILOT,
        "command": arming.command,
        "confirmation": arming.confirmation,
    },
)
arm.to_frame(MavlinkHeader(STATION, 190, 1))
print(f"sent      arm request, confirmation {arming.confirmation}")

# Nothing comes back in time, so it goes again with the next confirmation number.
resend = arming.on_timeout()
print(f"silence, resending with confirmation {resend}")

# An acknowledgement names the command it answers, so one for a different command is not
# this exchange finishing.
ack_shape = schema_for("COMMAND_ACK")


def acknowledgement(command: int) -> object:
    built = from_dict(ack_shape, {"command": command, "result": MAV_RESULT_ACCEPTED})
    return built.to_frame(MavlinkHeader(VEHICLE, AUTOPILOT, 0))


stray = arming.on_frame(acknowledgement(MAV_CMD_NAV_TAKEOFF))
print(f"an ack for another command: {stray.kind}")

outcome = arming.on_frame(acknowledgement(MAV_CMD_COMPONENT_ARM_DISARM))
if outcome.kind == "final" and outcome.value == MAV_RESULT_ACCEPTED:
    print("armed     the vehicle is ready")
else:
    print(f"the vehicle answered {outcome.kind} {outcome.value}")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mavlink`](https://crates.io/crates/pamoja-mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html), [docs.rs](https://docs.rs/pamoja-mavlink) |
| TypeScript | [`@pamoja/mavlink`](https://www.npmjs.com/package/@pamoja/mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html) |
| Python | [`pamoja-mavlink`](https://pypi.org/project/pamoja-mavlink/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html) |
| C# | [`Pamoja.Mavlink`](https://www.nuget.org/packages/Pamoja.Mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html) |

## Documentation

- [`pamoja.mavlink` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html), every class and function in this module.
- [The MAVLink guide](https://pamoja.molex.cloud/docs/guides/mavlink.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
