# MAVLink

MAVLink is what a ground station and an autopilot say to each other. A frame is a
marker byte, a small header, a payload, and a checksum, and the checksum is
seeded with a per-message constant so a receiver that disagrees about a message's
shape rejects the frame rather than misreading it. pamoja builds and parses v1
and v2 frames and does not own the link, so the same code drives a serial
adapter, a UDP socket, or a test that never leaves the process.

## What the example does

It runs one side of a ground station: announce itself with a heartbeat, read
the vehicle's heartbeat back off a link that delivers noise and a garbled copy
first, then arm the vehicle and follow that command through to its answer.

The garbled copy is the vehicle's own encoded frame with its last byte flipped,
so the payload is intact and only the checksum disagrees. Fields are set by
name rather than written into a payload buffer, and the message hands the frame
builder its own id and checksum seed, so there is no pair of constants to keep
in step. The confirmation number on the arm request comes from the exchange
rather than a counter the caller keeps. Rust names the common dialect values;
the other three bindings list the handful they use at the top of the file. The
exact v1 and v2 frame bytes are pinned in the conformance vectors every binding
checks itself against, so this page shows the exchange instead.

It proves:
- Fed noise and a copy whose checksum fails, the parser still recovers the
  frame behind them, and it decodes back to the vehicle's heartbeat: Rust
  compares every field, the other three the type it reports.
- The recovered frame carries the message id the dialect gives `HEARTBEAT`, so
  the header agrees with the payload it wraps.
- The first arm request goes out with confirmation `0`, and a timeout hands back
  `1` for the resend, so the vehicle can tell a retry from a second, deliberate
  command.
- An acknowledgement for `NAV_TAKEOFF` comes back unrelated, so another
  command's answer leaves this exchange still waiting.
- The acknowledgement naming `COMPONENT_ARM_DISARM` ends the exchange and hands
  back the result the vehicle sent.

## Rust

<!-- snippet: examples/tests/guides/mavlink.rs#example -->
From [`examples/tests/guides/mavlink.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/mavlink.rs):

```rust
use pamoja_mavlink::dialect::{self, mav_autopilot, mav_cmd, mav_result, mav_state, mav_type};
use pamoja_mavlink::dialect::{CommandAck, CommandLong, Heartbeat, Message};
use pamoja_mavlink::protocol::{AckOutcome, CommandProtocol};
use pamoja_mavlink::{Frame, Header, Parser};

const VEHICLE: u8 = 1;
const AUTOPILOT: u8 = 1;
const STATION: u8 = 255;

// Every MAVLink node broadcasts a heartbeat to say what it is and that it is alive.
// The fields have names and the common values have names, so nothing here is a number
// a reader has to look up.
let announce = Heartbeat {
    type_: mav_type::GCS,
    autopilot: mav_autopilot::INVALID,
    system_status: mav_state::ACTIVE,
    mavlink_version: 3,
    ..Default::default()
};

// Framing adds the start marker, the lengths and flags, the sending system and
// component, the message id, and a checksum seeded with this message's own value. The
// message supplies its own id and seed, so there is nothing to keep in step by hand.
let sent = Frame::encode_message(Header::new(STATION, 190, 0), &announce).expect("it fits");
let on_the_wire = sent.as_bytes().len();
println!("sent      {} in {on_the_wire} bytes", Heartbeat::NAME);

// The vehicle answers with its own heartbeat. This copy arrives after some bytes that
// were already on the wire, and after a copy with one bit flipped in flight.
let vehicle = Heartbeat {
    type_: mav_type::QUADROTOR,
    autopilot: mav_autopilot::ARDUPILOTMEGA,
    system_status: mav_state::STANDBY,
    mavlink_version: 3,
    ..Default::default()
};
let good =
    Frame::encode_message(Header::new(VEHICLE, AUTOPILOT, 0), &vehicle).expect("it fits");
let mut garbled = good.as_bytes().to_vec();
*garbled.last_mut().expect("a frame byte") ^= 0xFF;
let delivered = [b"???".as_slice(), &garbled, good.as_bytes()].concat();

// The parser skips whatever does not start a frame and drops one whose checksum fails,
// so the frame it hands back is the good copy rather than the garbled one.
let mut parser = Parser::new();
let received = delivered
    .iter()
    .find_map(|&byte| parser.push_byte(byte, &dialect::crc_extra))
    .expect("the good frame completes");
let heard: Heartbeat = received.decode_message().expect("a heartbeat payload");
let (kind, state) = (heard.type_, heard.system_status);
println!("heard     a type-{kind} vehicle in state {state}");

// Arming it is a command, not a message a sender fires and forgets: the vehicle has to
// answer, and the sender keeps asking until it does. The protocol numbers each resend,
// which is how a vehicle tells a retry from a second, deliberate command.
let mut arming = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 3);
let arm = CommandLong {
    param1: 1.0, // 1 arms, 0 disarms
    target_system: VEHICLE,
    target_component: AUTOPILOT,
    command: arming.command(),
    confirmation: arming.confirmation(),
    ..Default::default()
};
Frame::encode_message(Header::new(STATION, 190, 1), &arm).expect("a command fits");
println!("sent      arm request, confirmation {}", arm.confirmation);

// Nothing comes back in time, so it goes again with the next confirmation number.
match arming.on_timeout() {
    Some(confirmation) => println!("silence, resending with confirmation {confirmation}"),
    None => println!("out of retries, the vehicle is unreachable"),
}

// An acknowledgement names the command it answers, so one for a different command is
// not this exchange finishing.
let someone_elses = CommandAck {
    command: mav_cmd::NAV_TAKEOFF,
    result: mav_result::ACCEPTED,
    ..Default::default()
};
let stray = arming.on_ack(&someone_elses);
println!("an ack for another command: {stray:?}");

let accepted = CommandAck {
    command: mav_cmd::COMPONENT_ARM_DISARM,
    result: mav_result::ACCEPTED,
    ..Default::default()
};
match arming.on_ack(&accepted) {
    AckOutcome::Final(mav_result::ACCEPTED) => println!("armed     the vehicle is ready"),
    AckOutcome::Final(result) => println!("refused   the vehicle answered {result}"),
    AckOutcome::InProgress(percent) => println!("arming    {percent}% done"),
    AckOutcome::Unrelated => println!("that acknowledgement was for something else"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/mavlink.ts#example -->
From [`bindings/node/guides/mavlink.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mavlink.ts):

```typescript
import {
  CommandProtocol,
  type MavlinkFrame,
  MavlinkMessage,
  MavlinkParser,
  fromObject,
  message,
  schemaFor,
} from '@pamoja/mavlink'

const VEHICLE = 1
const AUTOPILOT = 1
const STATION = 255

// The values the MAVLink common dialect gives these fields.
const MAV_TYPE_GCS = 6
const MAV_TYPE_QUADROTOR = 2
const MAV_AUTOPILOT_INVALID = 8
const MAV_AUTOPILOT_ARDUPILOTMEGA = 3
const MAV_STATE_ACTIVE = 4
const MAV_STATE_STANDBY = 3
const MAV_CMD_COMPONENT_ARM_DISARM = 400
const MAV_CMD_NAV_TAKEOFF = 22
const MAV_RESULT_ACCEPTED = 0

// Every MAVLink node broadcasts a heartbeat to say what it is and that it is alive. The
// fields are set by name rather than by writing the payload out byte by byte.
const announce = message('HEARTBEAT')
announce.set('type', MAV_TYPE_GCS)
announce.set('autopilot', MAV_AUTOPILOT_INVALID)
announce.set('system_status', MAV_STATE_ACTIVE)
announce.set('mavlink_version', 3)
const sent = announce.toFrame({ systemId: STATION, componentId: 190, sequence: 0 })
console.log(`sent      HEARTBEAT in ${sent.bytes.length} bytes`)

// The vehicle answers with its own heartbeat. This copy arrives after some bytes that were
// already on the wire, and after a copy with one bit flipped in flight.
const heartbeatShape = schemaFor('HEARTBEAT')
const vehicle = fromObject(heartbeatShape, {
  type: MAV_TYPE_QUADROTOR,
  autopilot: MAV_AUTOPILOT_ARDUPILOTMEGA,
  system_status: MAV_STATE_STANDBY,
  mavlink_version: 3,
})
const good = vehicle.toFrame({ systemId: VEHICLE, componentId: AUTOPILOT, sequence: 0 })
const garbled = Buffer.from(good.bytes)
garbled[garbled.length - 1] ^= 0xff
const delivered = Buffer.concat([Buffer.from('???'), garbled, good.bytes])

// The parser skips whatever does not start a frame and drops one whose checksum fails, so
// the frame it hands back is the good copy rather than the garbled one.
const parser = new MavlinkParser()
const received = parser.push(delivered)[0]!
const heard = MavlinkMessage.decode(heartbeatShape, received.payload)
console.log(`heard     a type-${heard.get('type')} vehicle in state ${heard.get('system_status')}`)

// Arming it is a command, not a message a sender fires and forgets: the vehicle has to
// answer, and the sender keeps asking until it does. The protocol numbers each resend,
// which is how a vehicle tells a retry from a second, deliberate command.
const arming = new CommandProtocol(MAV_CMD_COMPONENT_ARM_DISARM, 3)
const commandShape = schemaFor('COMMAND_LONG')
const arm = fromObject(commandShape, {
  param1: 1, // 1 arms, 0 disarms
  target_system: VEHICLE,
  target_component: AUTOPILOT,
  command: arming.command,
  confirmation: arming.confirmation,
})
arm.toFrame({ systemId: STATION, componentId: 190, sequence: 1 })
console.log(`sent      arm request, confirmation ${arming.confirmation}`)

// Nothing comes back in time, so it goes again with the next confirmation number.
const resend = arming.onTimeout()
console.log(`silence, resending with confirmation ${resend}`)

// An acknowledgement names the command it answers, so one for a different command is not
// this exchange finishing.
const ackShape = schemaFor('COMMAND_ACK')
const acknowledgement = (command: number): MavlinkFrame =>
  fromObject(ackShape, { command, result: MAV_RESULT_ACCEPTED }).toFrame({
    systemId: VEHICLE,
    componentId: AUTOPILOT,
    sequence: 0,
  })

const stray = arming.onFrame(acknowledgement(MAV_CMD_NAV_TAKEOFF))
console.log(`an ack for another command: ${stray?.kind}`)

const outcome = arming.onFrame(acknowledgement(MAV_CMD_COMPONENT_ARM_DISARM))
if (outcome?.kind === 'final' && outcome.value === MAV_RESULT_ACCEPTED) {
  console.log('armed     the vehicle is ready')
} else {
  console.log(`the vehicle answered ${outcome?.kind} ${outcome?.value}`)
}
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/mavlink.py#example -->
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
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs#example -->
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
<!-- end -->

## Reference

<!-- table: reference mavlink -->
- Rust: [`pamoja-mavlink`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mavlink)
- TypeScript: [`@pamoja/mavlink`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mavlink)
- Python: [`pamoja.mavlink`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mavlink)
- C#: [`Pamoja.Mavlink`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mavlink)
- Hardware: [ArduPilot](https://pamoja.molex.cloud/docs/hardware.html#ardupilot), [PX4 Autopilot](https://pamoja.molex.cloud/docs/hardware.html#px4), [Pixhawk standard](https://pamoja.molex.cloud/docs/hardware.html#pixhawk)
<!-- end -->
