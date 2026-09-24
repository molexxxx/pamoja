# pamoja-mavlink

MAVLink v1 and v2 framing, signing, named message fields and enum values, and the mission, command, and offboard protocols. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/mavlink.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html)

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
    MavlinkHeader,
    MavlinkMessage,
    MavlinkParser,
    enum_entry,
    enum_names,
    enum_value,
    from_dict,
    schema_for,
)

VEHICLE = 1
AUTOPILOT = 1
STATION = 255
PLANNER = 190


# An enum field travels as a number, and the dialect names each number. Printing the name
# keeps a reader from looking up what 2 or 81 means.
def name(enumeration: str, value: int) -> str:
    return enum_entry(enumeration, value) or str(value)


# Every node broadcasts a heartbeat to say what it is and that it is alive. The frame wraps
# the payload in a header and a checksum seeded with the message's own value.
heartbeat_shape = schema_for("HEARTBEAT")
announce = from_dict(
    heartbeat_shape,
    {
        "type": enum_value("MAV_TYPE_GCS"),
        "autopilot": enum_value("MAV_AUTOPILOT_INVALID"),
        "system_status": enum_value("MAV_STATE_ACTIVE"),
        "mavlink_version": 3,
    },
)
sent = announce.to_frame(MavlinkHeader(STATION, PLANNER, 0))
print(f"sent      {heartbeat_shape.name} in {len(sent.bytes)} bytes")

# The vehicle answers with its own heartbeat, which reaches the station behind some noise and
# a copy with its last byte flipped in flight.
vehicle = from_dict(
    heartbeat_shape,
    {
        "type": enum_value("MAV_TYPE_QUADROTOR"),
        "autopilot": enum_value("MAV_AUTOPILOT_ARDUPILOTMEGA"),
        "base_mode": enum_value("MAV_MODE_FLAG_CUSTOM_MODE_ENABLED")
        | enum_value("MAV_MODE_FLAG_STABILIZE_ENABLED")
        | enum_value("MAV_MODE_FLAG_MANUAL_INPUT_ENABLED"),
        "system_status": enum_value("MAV_STATE_STANDBY"),
        "mavlink_version": 3,
    },
)
good = vehicle.to_frame(MavlinkHeader(VEHICLE, AUTOPILOT, 0))
garbled = bytearray(good.bytes)
garbled[-1] ^= 0xFF
delivered = b"???" + bytes(garbled) + good.bytes

# The parser skips whatever does not start a frame and drops a frame whose checksum fails, so
# only the good copy comes out.
frames = MavlinkParser().push(delivered)
print(
    f"parsed    {len(frames)} frame out of {len(delivered)} bytes,"
    " past the noise and the garbled copy"
)
heard = MavlinkMessage.decode(heartbeat_shape, frames[0].payload)
print(
    f"heard     {name('MAV_TYPE', heard.get_int('type'))} on"
    f" {name('MAV_AUTOPILOT', heard.get_int('autopilot'))},"
    f" in {name('MAV_STATE', heard.get_int('system_status'))}"
)

# The base mode is a bitmask, so it names a set of flags rather than one value.
base_mode = heard.get_int("base_mode")
print(f"flags     {' | '.join(enum_names('MAV_MODE_FLAG', base_mode))}")
if base_mode & enum_value("MAV_MODE_FLAG_SAFETY_ARMED") == 0:
    print("disarmed  MAV_MODE_FLAG_SAFETY_ARMED is not among them")
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-mavlink`](https://crates.io/crates/pamoja-mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html), [docs.rs](https://docs.rs/pamoja-mavlink), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mavlink) |
| TypeScript | [`@pamoja/mavlink`](https://www.npmjs.com/package/@pamoja/mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mavlink) |
| Python | [`pamoja-mavlink`](https://pypi.org/project/pamoja-mavlink/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mavlink) |
| C# | [`Pamoja.Mavlink`](https://www.nuget.org/packages/Pamoja.Mavlink) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mavlink) |

## Documentation

- [`pamoja.mavlink` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html), every class and function in this module.
- [The MAVLink guide](https://pamoja.molex.cloud/docs/guides/mavlink.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
