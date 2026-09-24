# MAVLink

MAVLink is what a ground station and an autopilot say to each other. A frame is a
marker byte, a small header, a payload, and a checksum, and the checksum is
seeded with a per-message constant, so a receiver that disagrees about a
message's shape rejects the frame rather than misreading it. pamoja builds,
parses, and signs v1 and v2 frames, names every value the dialect names, and
runs the command and mission exchanges. It does not own the link, so the same
code drives a serial adapter, a UDP socket, or a test that never leaves the
process.

## What the example does

It runs one side of a ground station talking to a vehicle, in four parts.

The first announces the station with a heartbeat and reads the vehicle's
heartbeat back off a link that delivers noise and a garbled copy first. Every
value is set and read by the name the dialect gives it: the vehicle is
`MAV_TYPE_QUADROTOR`, not 2, and its base mode is a set of named flags, not 81.

The second sees commands through. The station arms the vehicle, hears nothing,
and resends; an answer for another command leaves the arm waiting until its
own answer comes. Then a long command reports progress, a mode change is
refused, and a command nobody answers runs out of retries.

The third signs a frame with a key both ends share, and has the vehicle accept
it, then refuse it played back, refuse a frame signed with another key, and
refuse a frame with no signature at all. The fourth uploads a three-item plan
the way the mission protocol runs it, with the vehicle asking for each item in
turn, then asks for an item past the end of the plan.

It proves:

- The parser recovers the one good frame out of 45 bytes, past the noise and a
  copy whose checksum fails, and it decodes back to the vehicle's heartbeat.
- A value reads back as the dialect's name for it, and a bitmask as the names of
  the flags it holds, with `MAV_MODE_FLAG_SAFETY_ARMED` not among them.
- The first arm request goes out with confirmation `0` and the resend with `1`,
  an answer for `MAV_CMD_NAV_TAKEOFF` leaves the arm waiting, and its own
  answer ends it.
- `MAV_RESULT_IN_PROGRESS` carries a percentage and is not the end, a refusal
  names its result, and a command sent four times without an answer stops.
- A signed heartbeat is 13 bytes longer, the verifier accepts it once, and it
  refuses the replay, the forgery, and the unsigned frame, each with its reason.
- The vehicle asks for items 0, 1, and 2 in order and answers
  `MAV_MISSION_ACCEPTED`, and a request for item 7 is answered
  `MAV_MISSION_INVALID_SEQUENCE` rather than with an item.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example mavlink" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example mavlink</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- mavlink" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- mavlink</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/mavlink.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/mavlink.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- mavlink" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- mavlink</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-mavlink` is `no_std` and allocation-free at its core. A message
is a typed struct, such as `Heartbeat`, `CommandLong`, or `MissionItemInt`, with
its `ID`, `NAME`, and `CRC_EXTRA`, and every field the dialect defines for it,
MAVLink 2 extensions included; `Frame::encode_message` frames one and
`decode_message` reads one back. Each enumeration is a module of constants, such
as `mav_type::QUADROTOR`, and `dialect::ENUMS` holds the same values under the
dialect's names, looked up with `enum_named` and `entry_value`. The protocol
machines, `CommandProtocol`, `MissionSender`, and `MissionReceiver`, take a frame
and hand back the frame to send, and keep no timers: the caller waits and calls
`on_timeout`. The default `std` feature adds serial, UDP, and TCP links and a
vehicle model that runs over them.

<!-- snippet: examples/guides/mavlink.rs#example -->
From [`examples/guides/mavlink.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/mavlink.rs):

```rust
use pamoja_mavlink::dialect::{self, enum_named, mav_autopilot, mav_mode_flag, mav_state};
use pamoja_mavlink::dialect::{mav_type, Heartbeat, Message};
use pamoja_mavlink::{Frame, Header, Parser};

const VEHICLE: u8 = 1;
const AUTOPILOT: u8 = 1;
const STATION: u8 = 255;
const PLANNER: u8 = 190;

// An enum field travels as a number, and the dialect names each number. Printing the
// name keeps a reader from looking up what 2 or 81 means.
let name = |enumeration: &str, value: u64| {
    enum_named(enumeration)
        .and_then(|described| described.entry(value))
        .map_or_else(|| value.to_string(), str::to_owned)
};

// Every node broadcasts a heartbeat to say what it is and that it is alive. The frame
// wraps the payload in a header and a checksum seeded with the message's own value.
let announce = Heartbeat {
    type_: mav_type::GCS,
    autopilot: mav_autopilot::INVALID,
    system_status: mav_state::ACTIVE,
    mavlink_version: 3,
    ..Default::default()
};
let sent = Frame::encode_message(Header::new(STATION, PLANNER, 0), &announce)?;
println!(
    "sent      {} in {} bytes",
    Heartbeat::NAME,
    sent.as_bytes().len()
);

// The vehicle answers with its own heartbeat, which reaches the station behind some
// noise and a copy with its last byte flipped in flight.
let vehicle = Heartbeat {
    type_: mav_type::QUADROTOR,
    autopilot: mav_autopilot::ARDUPILOTMEGA,
    base_mode: mav_mode_flag::CUSTOM_MODE_ENABLED
        | mav_mode_flag::STABILIZE_ENABLED
        | mav_mode_flag::MANUAL_INPUT_ENABLED,
    system_status: mav_state::STANDBY,
    mavlink_version: 3,
    ..Default::default()
};
let good = Frame::encode_message(Header::new(VEHICLE, AUTOPILOT, 0), &vehicle)?;
let mut garbled = good.as_bytes().to_vec();
*garbled.last_mut().expect("a frame byte") ^= 0xFF;
let delivered = [b"???".as_slice(), &garbled, good.as_bytes()].concat();

// The parser skips whatever does not start a frame and drops a frame whose checksum
// fails, so only the good copy comes out.
let mut parser = Parser::new();
let frames: Vec<Frame> = delivered
    .iter()
    .filter_map(|&byte| parser.push_byte(byte, &dialect::crc_extra))
    .collect();
println!(
    "parsed    {} frame out of {} bytes, past the noise and the garbled copy",
    frames.len(),
    delivered.len()
);
let heard: Heartbeat = frames[0].decode_message()?;
println!(
    "heard     {} on {}, in {}",
    name("MAV_TYPE", heard.type_.into()),
    name("MAV_AUTOPILOT", heard.autopilot.into()),
    name("MAV_STATE", heard.system_status.into())
);

// The base mode is a bitmask, so it names a set of flags rather than one value.
let modes = enum_named("MAV_MODE_FLAG").expect("a dialect enumeration");
let flags: Vec<&str> = modes.names(heard.base_mode.into()).collect();
println!("flags     {}", flags.join(" | "));
if heard.base_mode & mav_mode_flag::SAFETY_ARMED == 0 {
    println!("disarmed  MAV_MODE_FLAG_SAFETY_ARMED is not among them");
}
```
<!-- end -->

Seeing commands through, continuing from above:

<!-- snippet: examples/guides/mavlink.rs#command -->
From [`examples/guides/mavlink.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/mavlink.rs):

```rust
use pamoja_mavlink::dialect::{mav_cmd, mav_result, CommandAck, CommandLong};
use pamoja_mavlink::protocol::{AckOutcome, CommandProtocol};

// The vehicle's answers, each naming the command it answers.
let answer = |command: u16, result: u8, progress: u8| {
    let ack = CommandAck {
        command,
        result,
        progress,
        ..Default::default()
    };
    Frame::encode_message(Header::new(VEHICLE, AUTOPILOT, 0), &ack)
};

// A command is not fired and forgotten: the vehicle has to answer, and the sender asks
// again until it does. Each resend carries the next confirmation number, which is how
// the vehicle tells a retry from a second, deliberate command.
let mut arming = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 3);
let arm = CommandLong {
    param1: 1.0,
    target_system: VEHICLE,
    target_component: AUTOPILOT,
    command: arming.command(),
    confirmation: arming.confirmation(),
    ..Default::default()
};
Frame::encode_message(Header::new(STATION, PLANNER, 1), &arm)?;
println!(
    "sent      {}, confirmation {}",
    name("MAV_CMD", arm.command.into()),
    arm.confirmation
);
if let Some(confirmation) = arming.on_timeout() {
    println!("silence   resent with confirmation {confirmation}");
}

// An answer names the command it answers, so one for another command leaves this one
// waiting.
for (command, result) in [
    (mav_cmd::NAV_TAKEOFF, mav_result::ACCEPTED),
    (mav_cmd::COMPONENT_ARM_DISARM, mav_result::ACCEPTED),
] {
    match arming.on_frame(&answer(command, result, 0)?)? {
        Some(AckOutcome::Unrelated) => println!(
            "stray     an answer for {} leaves it waiting",
            name("MAV_CMD", command.into())
        ),
        Some(AckOutcome::Final(result)) => {
            println!("armed     {}", name("MAV_RESULT", result.into()))
        }
        _ => {}
    }
}

// A long command reports progress before its final answer, and a refused one says why.
let calibrating = CommandProtocol::new(mav_cmd::PREFLIGHT_CALIBRATION, 3);
if let Some(AckOutcome::InProgress(percent)) =
    calibrating.on_frame(&answer(calibrating.command(), mav_result::IN_PROGRESS, 40)?)?
{
    println!(
        "progress  {} is {percent}% done",
        name("MAV_CMD", calibrating.command().into())
    );
}
let changing = CommandProtocol::new(mav_cmd::DO_SET_MODE, 3);
if let Some(AckOutcome::Final(result)) =
    changing.on_frame(&answer(changing.command(), mav_result::DENIED, 0)?)?
{
    println!(
        "refused   {} answered {}",
        name("MAV_CMD", changing.command().into()),
        name("MAV_RESULT", result.into())
    );
}

// A command nobody answers runs out of retries, and the caller stops asking.
let mut returning = CommandProtocol::new(mav_cmd::NAV_RETURN_TO_LAUNCH, 3);
let mut sends = 1;
while returning.on_timeout().is_some() {
    sends += 1;
}
println!(
    "gave up   {} went unanswered {sends} times",
    name("MAV_CMD", returning.command().into())
);
```
<!-- end -->

Signing, continuing from above:

<!-- snippet: examples/guides/mavlink.rs#signing -->
From [`examples/guides/mavlink.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/mavlink.rs):

```rust
use pamoja_mavlink::signing::{timestamp_from_unix_micros, KEY_LEN};
use pamoja_mavlink::{Signer, Verifier};

// Both ends share a secret key; replace these filler bytes with your own. The signer
// stamps each frame with its link id and a timestamp that only moves forward.
let key = [7u8; KEY_LEN];
let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as u64;
let mut signer = Signer::new(key, 1, timestamp_from_unix_micros(now));
let signed = signer.sign(
    Header::new(STATION, PLANNER, 2),
    Heartbeat::ID,
    sent.payload(),
    Heartbeat::CRC_EXTRA,
)?;
let signature = signed.signature().expect("a signed frame").len();
println!(
    "signed    {} bytes: the {} of the frame, then a {signature}-byte signature",
    signed.as_bytes().len(),
    sent.as_bytes().len()
);

// The vehicle checks each frame against the same key, and remembers the newest
// timestamp from each sender, so a recording played back later is refused.
let mut verifier = Verifier::new(key);
if verifier.verify(&signed).is_ok() {
    println!("accepted  the same key, and a timestamp it has not seen");
}
if let Err(refusal) = verifier.verify(&signed) {
    println!("replayed  the same frame again is refused: {refusal}");
}
let mut stranger = Signer::new([9u8; KEY_LEN], 1, timestamp_from_unix_micros(now));
let forged = stranger.sign(
    Header::new(STATION, PLANNER, 3),
    Heartbeat::ID,
    sent.payload(),
    Heartbeat::CRC_EXTRA,
)?;
if let Err(refusal) = verifier.verify(&forged) {
    println!("forged    another key's frame is refused: {refusal}");
}
if let Err(refusal) = verifier.verify(&sent) {
    println!("unsigned  a frame with no signature is refused: {refusal}");
}
```
<!-- end -->

Moving a plan, continuing from above:

<!-- snippet: examples/guides/mavlink.rs#mission -->
From [`examples/guides/mavlink.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/mavlink.rs):

```rust
use pamoja_mavlink::dialect::{
    mav_frame, mav_mission_type, MissionAck, MissionItemInt, MissionRequestInt,
};
use pamoja_mavlink::protocol::{MissionReceiver, MissionSender, ReceiverAction, SenderStep};

// A plan: take off to 20 m, fly to a point 50 m up, and return to launch. Positions
// travel as degrees times ten million.
let (latitude, longitude) = (-33.856_78_f64, 151.215_3_f64);
let item = |command: u16, x: i32, y: i32, z: f32| MissionItemInt {
    command,
    frame: mav_frame::GLOBAL_RELATIVE_ALT_INT,
    x,
    y,
    z,
    autocontinue: 1,
    ..Default::default()
};
let plan = [
    item(mav_cmd::NAV_TAKEOFF, 0, 0, 20.0),
    item(
        mav_cmd::NAV_WAYPOINT,
        (latitude * 1e7).round() as i32,
        (longitude * 1e7).round() as i32,
        50.0,
    ),
    item(mav_cmd::NAV_RETURN_TO_LAUNCH, 0, 0, 0.0),
];

// The station offers the plan, and the vehicle drives the transfer: it asks for each
// item in turn and acknowledges the last one.
let station = Header::new(STATION, PLANNER, 0);
let aboard = Header::new(VEHICLE, AUTOPILOT, 0);
let upload = MissionSender::new(&plan, VEHICLE, AUTOPILOT, mav_mission_type::MISSION);
let mut vehicle_side = MissionReceiver::new(STATION, PLANNER, mav_mission_type::MISSION);
let mut to_vehicle = upload.count_frame(station)?;
println!("count     the station offers {} items", upload.len());
let finished = loop {
    let step = vehicle_side
        .on_frame(&to_vehicle, aboard)?
        .expect("a mission frame");
    if let Some(arrived) = step.accepted {
        println!(
            "arrived   item {}, {}",
            arrived.seq,
            name("MAV_CMD", arrived.command.into())
        );
    }
    if let ReceiverAction::Request(_) = step.action {
        println!(
            "request   the vehicle asks for item {}",
            vehicle_side.expected()
        );
    }
    match upload.on_frame(&step.reply, station)? {
        Some(SenderStep::Reply(next)) => to_vehicle = next,
        Some(SenderStep::Finished(result)) => break result,
        None => unreachable!("the vehicle only sends mission frames"),
    }
};
println!(
    "done      the vehicle answered {}",
    name("MAV_MISSION_RESULT", finished.into())
);

// A request past the end of the plan is answered with a refusal, not with an item.
let past = MissionRequestInt {
    seq: 7,
    target_system: STATION,
    target_component: PLANNER,
    mission_type: mav_mission_type::MISSION,
};
if let Some(SenderStep::Reply(reply)) =
    upload.on_frame(&Frame::encode_message(aboard, &past)?, station)?
{
    let refused: MissionAck = reply.decode_message()?;
    println!(
        "refused   a request for item {} is answered {}",
        past.seq,
        name("MAV_MISSION_RESULT", refused.type_.into())
    );
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/mavlink` works by name: `schemaFor('HEARTBEAT')` is a
message's shape, `fromObject` and `message` build one, and
`MavlinkMessage.decode` reads one. `enumValue`, `enumEntry`, and `enumNames`
carry the dialect's names for values. The machines are Rust's, and an outcome's
`kind` is a string: `'unrelated'`, `'inProgress'`, or `'final'`. A refused frame
or signature throws an `Error` whose message is the reason.

<!-- snippet: bindings/node/guides/mavlink.ts#example -->
From [`bindings/node/guides/mavlink.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mavlink.ts):

```typescript
import {
  type MavlinkFrame,
  type MavlinkHeader,
  MavlinkMessage,
  MavlinkParser,
  enumEntry,
  enumNames,
  enumValue,
  fromObject,
  message,
  schemaFor,
} from '@pamoja/mavlink'

const VEHICLE = 1
const AUTOPILOT = 1
const STATION = 255
const PLANNER = 190

// An enum field travels as a number, and the dialect names each number. Printing the name
// keeps a reader from looking up what 2 or 81 means.
const name = (enumeration: string, value: number): string =>
  enumEntry(enumeration, value) ?? String(value)
const header = (systemId: number, componentId: number, sequence: number): MavlinkHeader => ({
  systemId,
  componentId,
  sequence,
})

// Every node broadcasts a heartbeat to say what it is and that it is alive. The frame wraps
// the payload in a header and a checksum seeded with the message's own value.
const heartbeatShape = schemaFor('HEARTBEAT')
const announce = fromObject(heartbeatShape, {
  type: enumValue('MAV_TYPE_GCS'),
  autopilot: enumValue('MAV_AUTOPILOT_INVALID'),
  system_status: enumValue('MAV_STATE_ACTIVE'),
  mavlink_version: 3,
})
const sent = announce.toFrame(header(STATION, PLANNER, 0))
console.log(`sent      ${heartbeatShape.name} in ${sent.bytes.length} bytes`)

// The vehicle answers with its own heartbeat, which reaches the station behind some noise
// and a copy with its last byte flipped in flight.
const vehicle = fromObject(heartbeatShape, {
  type: enumValue('MAV_TYPE_QUADROTOR'),
  autopilot: enumValue('MAV_AUTOPILOT_ARDUPILOTMEGA'),
  base_mode:
    enumValue('MAV_MODE_FLAG_CUSTOM_MODE_ENABLED') |
    enumValue('MAV_MODE_FLAG_STABILIZE_ENABLED') |
    enumValue('MAV_MODE_FLAG_MANUAL_INPUT_ENABLED'),
  system_status: enumValue('MAV_STATE_STANDBY'),
  mavlink_version: 3,
})
const good = vehicle.toFrame(header(VEHICLE, AUTOPILOT, 0))
const garbled = Buffer.from(good.bytes)
garbled[garbled.length - 1] ^= 0xff
const delivered = Buffer.concat([Buffer.from('???'), garbled, good.bytes])

// The parser skips whatever does not start a frame and drops a frame whose checksum fails,
// so only the good copy comes out.
const frames = new MavlinkParser().push(delivered)
console.log(
  `parsed    ${frames.length} frame out of ${delivered.length} bytes,` +
    ' past the noise and the garbled copy',
)
const heard = MavlinkMessage.decode(heartbeatShape, frames[0]!.payload)
console.log(
  `heard     ${name('MAV_TYPE', heard.get('type'))} on` +
    ` ${name('MAV_AUTOPILOT', heard.get('autopilot'))},` +
    ` in ${name('MAV_STATE', heard.get('system_status'))}`,
)

// The base mode is a bitmask, so it names a set of flags rather than one value.
const baseMode = heard.get('base_mode')
console.log(`flags     ${enumNames('MAV_MODE_FLAG', baseMode).join(' | ')}`)
if ((baseMode & enumValue('MAV_MODE_FLAG_SAFETY_ARMED')) === 0) {
  console.log('disarmed  MAV_MODE_FLAG_SAFETY_ARMED is not among them')
}
```
<!-- end -->

Seeing commands through, continuing from above:

<!-- snippet: bindings/node/guides/mavlink.ts#command -->
From [`bindings/node/guides/mavlink.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mavlink.ts):

```typescript
import { CommandProtocol } from '@pamoja/mavlink'

// The vehicle's answers, each naming the command it answers.
const ackShape = schemaFor('COMMAND_ACK')
const answer = (command: number, result: number, progress: number): MavlinkFrame =>
  fromObject(ackShape, { command, result, progress }).toFrame(header(VEHICLE, AUTOPILOT, 0))

// A command is not fired and forgotten: the vehicle has to answer, and the sender asks again
// until it does. Each resend carries the next confirmation number, which is how the vehicle
// tells a retry from a second, deliberate command.
const arming = new CommandProtocol(enumValue('MAV_CMD_COMPONENT_ARM_DISARM'), 3)
const arm = fromObject(schemaFor('COMMAND_LONG'), {
  param1: 1,
  target_system: VEHICLE,
  target_component: AUTOPILOT,
  command: arming.command,
  confirmation: arming.confirmation,
})
arm.toFrame(header(STATION, PLANNER, 1))
console.log(
  `sent      ${name('MAV_CMD', arm.get('command'))}, confirmation ${arm.get('confirmation')}`,
)
const resend = arming.onTimeout()
if (resend !== null) {
  console.log(`silence   resent with confirmation ${resend}`)
}

// An answer names the command it answers, so one for another command leaves this one
// waiting.
for (const [command, result] of [
  [enumValue('MAV_CMD_NAV_TAKEOFF'), enumValue('MAV_RESULT_ACCEPTED')],
  [enumValue('MAV_CMD_COMPONENT_ARM_DISARM'), enumValue('MAV_RESULT_ACCEPTED')],
] as const) {
  const outcome = arming.onFrame(answer(command, result, 0))
  if (outcome?.kind === 'unrelated') {
    console.log(`stray     an answer for ${name('MAV_CMD', command)} leaves it waiting`)
  } else if (outcome?.kind === 'final') {
    console.log(`armed     ${name('MAV_RESULT', outcome.value!)}`)
  }
}

// A long command reports progress before its final answer, and a refused one says why.
const calibrating = new CommandProtocol(enumValue('MAV_CMD_PREFLIGHT_CALIBRATION'), 3)
const progress = calibrating.onFrame(
  answer(calibrating.command, enumValue('MAV_RESULT_IN_PROGRESS'), 40),
)
if (progress?.kind === 'inProgress') {
  console.log(`progress  ${name('MAV_CMD', calibrating.command)} is ${progress.value}% done`)
}
const changing = new CommandProtocol(enumValue('MAV_CMD_DO_SET_MODE'), 3)
const refusal = changing.onFrame(answer(changing.command, enumValue('MAV_RESULT_DENIED'), 0))
if (refusal?.kind === 'final') {
  console.log(
    `refused   ${name('MAV_CMD', changing.command)} answered ${name('MAV_RESULT', refusal.value!)}`,
  )
}

// A command nobody answers runs out of retries, and the caller stops asking.
const returning = new CommandProtocol(enumValue('MAV_CMD_NAV_RETURN_TO_LAUNCH'), 3)
let sends = 1
while (returning.onTimeout() !== null) {
  sends += 1
}
console.log(`gave up   ${name('MAV_CMD', returning.command)} went unanswered ${sends} times`)
```
<!-- end -->

Signing, continuing from above:

<!-- snippet: bindings/node/guides/mavlink.ts#signing -->
From [`bindings/node/guides/mavlink.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mavlink.ts):

```typescript
import { KEY_LEN, MavlinkSigner, MavlinkVerifier, timestampNow } from '@pamoja/mavlink'

// Both ends share a secret key; replace these filler bytes with your own. The signer stamps
// each frame with its link id and a timestamp that only moves forward.
const key = Buffer.alloc(KEY_LEN, 7)
const signer = new MavlinkSigner(key, 1, timestampNow())
const signed = signer.sign(
  header(STATION, PLANNER, 2),
  heartbeatShape.id,
  sent.payload,
  heartbeatShape.crcExtra,
)
console.log(
  `signed    ${signed.bytes.length} bytes: the ${sent.bytes.length} of the frame,` +
    ` then a ${signed.signature!.length}-byte signature`,
)

// The vehicle checks each frame against the same key, and remembers the newest timestamp
// from each sender, so a recording played back later is refused.
const refusedWith = (check: () => void): string | null => {
  try {
    check()
    return null
  } catch (error) {
    return (error as Error).message
  }
}
const verifier = new MavlinkVerifier(key)
if (refusedWith(() => verifier.verify(signed)) === null) {
  console.log('accepted  the same key, and a timestamp it has not seen')
}
const replayed = refusedWith(() => verifier.verify(signed))
if (replayed !== null) {
  console.log(`replayed  the same frame again is refused: ${replayed}`)
}
const stranger = new MavlinkSigner(Buffer.alloc(KEY_LEN, 9), 1, timestampNow())
const forged = stranger.sign(
  header(STATION, PLANNER, 3),
  heartbeatShape.id,
  sent.payload,
  heartbeatShape.crcExtra,
)
const forgery = refusedWith(() => verifier.verify(forged))
if (forgery !== null) {
  console.log(`forged    another key's frame is refused: ${forgery}`)
}
const unsigned = refusedWith(() => verifier.verify(sent))
if (unsigned !== null) {
  console.log(`unsigned  a frame with no signature is refused: ${unsigned}`)
}
```
<!-- end -->

Moving a plan, continuing from above:

<!-- snippet: bindings/node/guides/mavlink.ts#mission -->
From [`bindings/node/guides/mavlink.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/mavlink.ts):

```typescript
import { MissionReceiver, MissionSender } from '@pamoja/mavlink'

// A plan: take off to 20 m, fly to a point 50 m up, and return to launch. Positions travel
// as degrees times ten million.
const [latitude, longitude] = [-33.85678, 151.2153]
const itemShape = schemaFor('MISSION_ITEM_INT')
const item = (command: string, x: number, y: number, z: number): Buffer =>
  fromObject(itemShape, {
    command: enumValue(command),
    frame: enumValue('MAV_FRAME_GLOBAL_RELATIVE_ALT_INT'),
    x,
    y,
    z,
    autocontinue: 1,
  }).payload

// The station offers the plan, and the vehicle drives the transfer: it asks for each item in
// turn and acknowledges the last one.
const station = header(STATION, PLANNER, 0)
const aboard = header(VEHICLE, AUTOPILOT, 0)
const missionType = enumValue('MAV_MISSION_TYPE_MISSION')
const upload = new MissionSender(VEHICLE, AUTOPILOT, missionType)
upload.addItem(item('MAV_CMD_NAV_TAKEOFF', 0, 0, 20))
upload.addItem(
  item(
    'MAV_CMD_NAV_WAYPOINT',
    Math.round(latitude * 1e7),
    Math.round(longitude * 1e7),
    50,
  ),
)
upload.addItem(item('MAV_CMD_NAV_RETURN_TO_LAUNCH', 0, 0, 0))
const vehicleSide = new MissionReceiver(STATION, PLANNER, missionType)
let toVehicle = upload.count(station)
console.log(`count     the station offers ${upload.length} items`)
let finished: number | null = null
while (finished === null) {
  const step = vehicleSide.onFrame(toVehicle, aboard)!
  if (step.accepted !== null) {
    console.log(
      `arrived   item ${step.accepted.get('seq')}, ${name('MAV_CMD', step.accepted.get('command'))}`,
    )
  }
  if (step.kind === 'request') {
    console.log(`request   the vehicle asks for item ${vehicleSide.expected}`)
  }
  const next = upload.onFrame(step.reply, station)!
  if (next.kind === 'finished') {
    finished = next.result
  } else {
    toVehicle = next.reply!
  }
}
console.log(`done      the vehicle answered ${name('MAV_MISSION_RESULT', finished)}`)

// A request past the end of the plan is answered with a refusal, not with an item.
const past = message('MISSION_REQUEST_INT')
past.set('seq', 7)
past.set('target_system', STATION)
past.set('target_component', PLANNER)
past.set('mission_type', missionType)
const reply = upload.onFrame(past.toFrame(aboard), station)
if (reply?.kind === 'reply') {
  const refused = MavlinkMessage.decode(schemaFor('MISSION_ACK'), reply.reply!.payload)
  console.log(
    `refused   a request for item ${past.get('seq')} is answered` +
      ` ${name('MAV_MISSION_RESULT', refused.get('type'))}`,
  )
}
```
<!-- end -->

## Python

In Python, `pamoja.mavlink` has the same calls in snake case, with `get_int` to
read an integer field. An outcome's `kind` is `'unrelated'`, `'in_progress'`, or
`'final'`. A refused frame or signature raises `PamojaError`, and a name the
dialect does not know raises `ValueError`.

<!-- snippet: bindings/python/guides/mavlink.py#example -->
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
<!-- end -->

Seeing commands through, continuing from above:

<!-- snippet: bindings/python/guides/mavlink.py#command -->
From [`bindings/python/guides/mavlink.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mavlink.py):

```python
from pamoja.mavlink import CommandProtocol

# The vehicle's answers, each naming the command it answers.
ack_shape = schema_for("COMMAND_ACK")


def answer(command: int, result: int, progress: int):
    fields = {"command": command, "result": result, "progress": progress}
    return from_dict(ack_shape, fields).to_frame(MavlinkHeader(VEHICLE, AUTOPILOT, 0))


# A command is not fired and forgotten: the vehicle has to answer, and the sender asks again
# until it does. Each resend carries the next confirmation number, which is how the vehicle
# tells a retry from a second, deliberate command.
arming = CommandProtocol(enum_value("MAV_CMD_COMPONENT_ARM_DISARM"), 3)
arm = from_dict(
    schema_for("COMMAND_LONG"),
    {
        "param1": 1.0,
        "target_system": VEHICLE,
        "target_component": AUTOPILOT,
        "command": arming.command,
        "confirmation": arming.confirmation,
    },
)
arm.to_frame(MavlinkHeader(STATION, PLANNER, 1))
print(
    f"sent      {name('MAV_CMD', arm.get_int('command'))},"
    f" confirmation {arm.get_int('confirmation')}"
)
resend = arming.on_timeout()
if resend is not None:
    print(f"silence   resent with confirmation {resend}")

# An answer names the command it answers, so one for another command leaves this one
# waiting.
for command, result in (
    (enum_value("MAV_CMD_NAV_TAKEOFF"), enum_value("MAV_RESULT_ACCEPTED")),
    (enum_value("MAV_CMD_COMPONENT_ARM_DISARM"), enum_value("MAV_RESULT_ACCEPTED")),
):
    outcome = arming.on_frame(answer(command, result, 0))
    if outcome is not None and outcome.kind == "unrelated":
        print(f"stray     an answer for {name('MAV_CMD', command)} leaves it waiting")
    elif outcome is not None and outcome.kind == "final":
        print(f"armed     {name('MAV_RESULT', outcome.value)}")

# A long command reports progress before its final answer, and a refused one says why.
calibrating = CommandProtocol(enum_value("MAV_CMD_PREFLIGHT_CALIBRATION"), 3)
progress = calibrating.on_frame(
    answer(calibrating.command, enum_value("MAV_RESULT_IN_PROGRESS"), 40)
)
if progress is not None and progress.kind == "in_progress":
    print(f"progress  {name('MAV_CMD', calibrating.command)} is {progress.value}% done")
changing = CommandProtocol(enum_value("MAV_CMD_DO_SET_MODE"), 3)
refusal = changing.on_frame(answer(changing.command, enum_value("MAV_RESULT_DENIED"), 0))
if refusal is not None and refusal.kind == "final":
    print(
        f"refused   {name('MAV_CMD', changing.command)}"
        f" answered {name('MAV_RESULT', refusal.value)}"
    )

# A command nobody answers runs out of retries, and the caller stops asking.
returning = CommandProtocol(enum_value("MAV_CMD_NAV_RETURN_TO_LAUNCH"), 3)
sends = 1
while returning.on_timeout() is not None:
    sends += 1
print(f"gave up   {name('MAV_CMD', returning.command)} went unanswered {sends} times")
```
<!-- end -->

Signing, continuing from above:

<!-- snippet: bindings/python/guides/mavlink.py#signing -->
From [`bindings/python/guides/mavlink.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mavlink.py):

```python
from pamoja.core import PamojaError
from pamoja.mavlink import KEY_LEN, MavlinkSigner, MavlinkVerifier, timestamp_now

# Both ends share a secret key; replace these filler bytes with your own. The signer stamps
# each frame with its link id and a timestamp that only moves forward.
key = bytes([7]) * KEY_LEN
signer = MavlinkSigner(key, 1, timestamp_now())
signed = signer.sign(
    MavlinkHeader(STATION, PLANNER, 2),
    heartbeat_shape.id,
    sent.payload,
    heartbeat_shape.crc_extra,
)
print(
    f"signed    {len(signed.bytes)} bytes: the {len(sent.bytes)} of the frame,"
    f" then a {len(signed.signature)}-byte signature"
)


# The vehicle checks each frame against the same key, and remembers the newest timestamp from
# each sender, so a recording played back later is refused.
def refused_with(check) -> str | None:
    try:
        check()
    except PamojaError as refusal:
        return str(refusal)
    return None


verifier = MavlinkVerifier(key)
if refused_with(lambda: verifier.verify(signed)) is None:
    print("accepted  the same key, and a timestamp it has not seen")
replayed = refused_with(lambda: verifier.verify(signed))
if replayed is not None:
    print(f"replayed  the same frame again is refused: {replayed}")
stranger = MavlinkSigner(bytes([9]) * KEY_LEN, 1, timestamp_now())
forged = stranger.sign(
    MavlinkHeader(STATION, PLANNER, 3),
    heartbeat_shape.id,
    sent.payload,
    heartbeat_shape.crc_extra,
)
forgery = refused_with(lambda: verifier.verify(forged))
if forgery is not None:
    print(f"forged    another key's frame is refused: {forgery}")
unsigned = refused_with(lambda: verifier.verify(sent))
if unsigned is not None:
    print(f"unsigned  a frame with no signature is refused: {unsigned}")
```
<!-- end -->

Moving a plan, continuing from above:

<!-- snippet: bindings/python/guides/mavlink.py#mission -->
From [`bindings/python/guides/mavlink.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/mavlink.py):

```python
from pamoja.mavlink import MissionReceiver, MissionSender, message

# A plan: take off to 20 m, fly to a point 50 m up, and return to launch. Positions travel as
# degrees times ten million.
latitude, longitude = -33.85678, 151.2153
item_shape = schema_for("MISSION_ITEM_INT")


def item(command: str, x: int, y: int, z: float) -> bytes:
    fields = {
        "command": enum_value(command),
        "frame": enum_value("MAV_FRAME_GLOBAL_RELATIVE_ALT_INT"),
        "x": x,
        "y": y,
        "z": z,
        "autocontinue": 1,
    }
    return from_dict(item_shape, fields).payload


# The station offers the plan, and the vehicle drives the transfer: it asks for each item in
# turn and acknowledges the last one.
station = MavlinkHeader(STATION, PLANNER, 0)
aboard = MavlinkHeader(VEHICLE, AUTOPILOT, 0)
mission_type = enum_value("MAV_MISSION_TYPE_MISSION")
upload = MissionSender(VEHICLE, AUTOPILOT, mission_type)
upload.add_item(item("MAV_CMD_NAV_TAKEOFF", 0, 0, 20.0))
upload.add_item(
    item("MAV_CMD_NAV_WAYPOINT", round(latitude * 1e7), round(longitude * 1e7), 50.0)
)
upload.add_item(item("MAV_CMD_NAV_RETURN_TO_LAUNCH", 0, 0, 0.0))
vehicle_side = MissionReceiver(STATION, PLANNER, mission_type)
to_vehicle = upload.count(station)
print(f"count     the station offers {len(upload)} items")
finished = None
while finished is None:
    step = vehicle_side.on_frame(to_vehicle, aboard)
    if step.accepted is not None:
        arrived = step.accepted
        print(
            f"arrived   item {arrived.get_int('seq')},"
            f" {name('MAV_CMD', arrived.get_int('command'))}"
        )
    if step.kind == "request":
        print(f"request   the vehicle asks for item {vehicle_side.expected}")
    following = upload.on_frame(step.reply, station)
    if following.kind == "finished":
        finished = following.result
    else:
        to_vehicle = following.reply
print(f"done      the vehicle answered {name('MAV_MISSION_RESULT', finished)}")

# A request past the end of the plan is answered with a refusal, not with an item.
past = message("MISSION_REQUEST_INT")
past.set_int("seq", 7)
past.set_int("target_system", STATION)
past.set_int("target_component", PLANNER)
past.set_int("mission_type", mission_type)
reply = upload.on_frame(past.to_frame(aboard), station)
if reply is not None and reply.kind == "reply":
    refused = MavlinkMessage.decode(schema_for("MISSION_ACK"), reply.reply.payload)
    print(
        f"refused   a request for item {past.get_int('seq')} is answered"
        f" {name('MAV_MISSION_RESULT', refused.get_int('type'))}"
    )
```
<!-- end -->

## C#

In C#, `Pamoja.Mavlink` holds `MavlinkSchema`, `MavlinkMessage`, `MavlinkFrame`,
and the machines, `MavlinkCommand`, `MavlinkMissionSender`, and
`MavlinkMissionReceiver`. Each holds a native handle and is disposable, so it is
declared with `using`. `MavlinkEnum.Value`, `Entry`, and `Names` carry the
dialect's names. `Get` returns a `double`, and a `MavlinkCommand` takes its
command as a `ushort`. A refusal throws `PamojaException`.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs#example -->
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
<!-- end -->

Seeing commands through, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs#command -->
From [`bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs):

```csharp
// The vehicle's answers, each naming the command it answers.
using MavlinkSchema ackShape = MavlinkSchema.ForName("COMMAND_ACK");
MavlinkFrame Answer(ulong command, ulong result, byte progress)
{
    using MavlinkMessage ack = ackShape.CreateMessage();
    ack.Set("command", command);
    ack.Set("result", result);
    ack.Set("progress", progress);
    return ack.ToFrame(new MavlinkHeader(Vehicle, Autopilot, 0));
}

MavlinkAckOutcome? Hear(MavlinkCommand tracked, ulong command, ulong result, byte progress)
{
    using MavlinkFrame frame = Answer(command, result, progress);
    return tracked.OnFrame(frame);
}

// A command is not fired and forgotten: the vehicle has to answer, and the sender
// asks again until it does. Each resend carries the next confirmation number, which
// is how the vehicle tells a retry from a second, deliberate command.
using MavlinkCommand arming =
    new((ushort)MavlinkEnum.Value("MAV_CMD_COMPONENT_ARM_DISARM"), 3);
using MavlinkSchema commandShape = MavlinkSchema.ForName("COMMAND_LONG");
using MavlinkMessage arm = commandShape.CreateMessage();
arm.Set("param1", 1.0);
arm.Set("target_system", Vehicle);
arm.Set("target_component", Autopilot);
arm.Set("command", arming.Command);
arm.Set("confirmation", arming.Confirmation);
arm.ToFrame(new MavlinkHeader(Station, Planner, 1)).Dispose();
Console.WriteLine(
    $"sent      {Name("MAV_CMD", arm.Get("command"))},"
    + $" confirmation {arm.Get("confirmation")}");
byte? resend = arming.OnTimeout();
if (resend is not null)
{
    Console.WriteLine($"silence   resent with confirmation {resend}");
}

// An answer names the command it answers, so one for another command leaves this
// one waiting.
foreach ((ulong command, ulong result) in new[]
{
    (MavlinkEnum.Value("MAV_CMD_NAV_TAKEOFF"), MavlinkEnum.Value("MAV_RESULT_ACCEPTED")),
    (MavlinkEnum.Value("MAV_CMD_COMPONENT_ARM_DISARM"), MavlinkEnum.Value("MAV_RESULT_ACCEPTED")),
})
{
    MavlinkAckOutcome? outcome = Hear(arming, command, result, 0);
    if (outcome?.Kind == MavlinkAckKind.Unrelated)
    {
        Console.WriteLine(
            $"stray     an answer for {Name("MAV_CMD", command)} leaves it waiting");
    }
    else if (outcome?.Kind == MavlinkAckKind.Final)
    {
        Console.WriteLine($"armed     {Name("MAV_RESULT", outcome.Value.Value!.Value)}");
    }
}

// A long command reports progress before its final answer, and a refused one says
// why.
using MavlinkCommand calibrating =
    new((ushort)MavlinkEnum.Value("MAV_CMD_PREFLIGHT_CALIBRATION"), 3);
MavlinkAckOutcome? progress =
    Hear(calibrating, calibrating.Command, MavlinkEnum.Value("MAV_RESULT_IN_PROGRESS"), 40);
if (progress?.Kind == MavlinkAckKind.InProgress)
{
    Console.WriteLine(
        $"progress  {Name("MAV_CMD", calibrating.Command)} is {progress.Value.Value}% done");
}
using MavlinkCommand changing = new((ushort)MavlinkEnum.Value("MAV_CMD_DO_SET_MODE"), 3);
MavlinkAckOutcome? refusal =
    Hear(changing, changing.Command, MavlinkEnum.Value("MAV_RESULT_DENIED"), 0);
if (refusal?.Kind == MavlinkAckKind.Final)
{
    Console.WriteLine(
        $"refused   {Name("MAV_CMD", changing.Command)}"
        + $" answered {Name("MAV_RESULT", refusal.Value.Value!.Value)}");
}

// A command nobody answers runs out of retries, and the caller stops asking.
using MavlinkCommand returning =
    new((ushort)MavlinkEnum.Value("MAV_CMD_NAV_RETURN_TO_LAUNCH"), 3);
int sends = 1;
while (returning.OnTimeout() is not null)
{
    sends += 1;
}
Console.WriteLine(
    $"gave up   {Name("MAV_CMD", returning.Command)} went unanswered {sends} times");
```
<!-- end -->

Signing, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs#signing -->
From [`bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs):

```csharp
// Both ends share a secret key; replace these filler bytes with your own. The signer
// stamps each frame with its link id and a timestamp that only moves forward.
byte[] key = Enumerable.Repeat((byte)7, Mavlink.KeyLength).ToArray();
using MavlinkSigner signer = new(key, 1, Mavlink.TimestampNow());
using MavlinkFrame signed = signer.Sign(
    new MavlinkHeader(Station, Planner, 2),
    heartbeatShape.MessageId,
    sent.Payload,
    heartbeatShape.CrcExtra);
Console.WriteLine(
    $"signed    {signed.Bytes.Length} bytes: the {sent.Bytes.Length} of the frame,"
    + $" then a {signed.Signature!.Length}-byte signature");

// The vehicle checks each frame against the same key, and remembers the newest
// timestamp from each sender, so a recording played back later is refused.
static string? RefusedWith(Action check)
{
    try
    {
        check();
        return null;
    }
    catch (PamojaException refusal)
    {
        return refusal.Message;
    }
}

using MavlinkVerifier verifier = new(key);
if (RefusedWith(() => verifier.Verify(signed)) is null)
{
    Console.WriteLine("accepted  the same key, and a timestamp it has not seen");
}
string? replayed = RefusedWith(() => verifier.Verify(signed));
if (replayed is not null)
{
    Console.WriteLine($"replayed  the same frame again is refused: {replayed}");
}
byte[] otherKey = Enumerable.Repeat((byte)9, Mavlink.KeyLength).ToArray();
using MavlinkSigner stranger = new(otherKey, 1, Mavlink.TimestampNow());
using MavlinkFrame forged = stranger.Sign(
    new MavlinkHeader(Station, Planner, 3),
    heartbeatShape.MessageId,
    sent.Payload,
    heartbeatShape.CrcExtra);
string? forgery = RefusedWith(() => verifier.Verify(forged));
if (forgery is not null)
{
    Console.WriteLine($"forged    another key's frame is refused: {forgery}");
}
string? unsigned = RefusedWith(() => verifier.Verify(sent));
if (unsigned is not null)
{
    Console.WriteLine($"unsigned  a frame with no signature is refused: {unsigned}");
}
```
<!-- end -->

Moving a plan, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs#mission -->
From [`bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/MavlinkGuide.cs):

```csharp
// A plan: take off to 20 m, fly to a point 50 m up, and return to launch. Positions
// travel as degrees times ten million.
(double latitude, double longitude) = (-33.85678, 151.2153);
using MavlinkSchema itemShape = MavlinkSchema.ForName("MISSION_ITEM_INT");
MavlinkMessage Item(string command, double x, double y, double z)
{
    MavlinkMessage built = itemShape.CreateMessage();
    built.Set("command", MavlinkEnum.Value(command));
    built.Set("frame", MavlinkEnum.Value("MAV_FRAME_GLOBAL_RELATIVE_ALT_INT"));
    built.Set("x", x);
    built.Set("y", y);
    built.Set("z", z);
    built.Set("autocontinue", 1);
    return built;
}

// The station offers the plan, and the vehicle drives the transfer: it asks for each
// item in turn and acknowledges the last one.
MavlinkHeader station = new(Station, Planner, 0);
MavlinkHeader aboard = new(Vehicle, Autopilot, 0);
byte missionType = (byte)MavlinkEnum.Value("MAV_MISSION_TYPE_MISSION");
using MavlinkMissionSender upload = new(Vehicle, Autopilot, missionType);
using (MavlinkMessage takeoff = Item("MAV_CMD_NAV_TAKEOFF", 0, 0, 20))
using (MavlinkMessage waypoint = Item(
    "MAV_CMD_NAV_WAYPOINT",
    Math.Round(latitude * 1e7),
    Math.Round(longitude * 1e7),
    50))
using (MavlinkMessage home = Item("MAV_CMD_NAV_RETURN_TO_LAUNCH", 0, 0, 0))
{
    upload.AddItem(takeoff);
    upload.AddItem(waypoint);
    upload.AddItem(home);
}
using MavlinkMissionReceiver vehicleSide = new(Station, Planner, missionType);
MavlinkFrame toVehicle = upload.CountFrame(station);
Console.WriteLine($"count     the station offers {upload.Count} items");
byte? finished = null;
while (finished is null)
{
    MavlinkReceiverStep step = vehicleSide.OnFrame(toVehicle, aboard)!.Value;
    toVehicle.Dispose();
    if (step.Accepted is MavlinkMessage arrived)
    {
        Console.WriteLine(
            $"arrived   item {arrived.Get("seq")}, {Name("MAV_CMD", arrived.Get("command"))}");
        arrived.Dispose();
    }
    if (step.Kind == MavlinkReceiverKind.Request)
    {
        Console.WriteLine($"request   the vehicle asks for item {vehicleSide.Expected}");
    }
    MavlinkSenderStep following = upload.OnFrame(step.Reply, station)!.Value;
    step.Reply.Dispose();
    if (following.Kind == MavlinkSenderKind.Finished)
    {
        finished = following.Result;
    }
    else
    {
        toVehicle = following.Reply!;
    }
}
Console.WriteLine($"done      the vehicle answered {Name("MAV_MISSION_RESULT", finished.Value)}");

// A request past the end of the plan is answered with a refusal, not with an item.
using MavlinkSchema requestShape = MavlinkSchema.ForName("MISSION_REQUEST_INT");
using MavlinkMessage past = requestShape.CreateMessage();
past.Set("seq", 7);
past.Set("target_system", Station);
past.Set("target_component", Planner);
past.Set("mission_type", missionType);
using MavlinkFrame asked = past.ToFrame(aboard);
MavlinkSenderStep? reply = upload.OnFrame(asked, station);
if (reply?.Kind == MavlinkSenderKind.Reply)
{
    using MavlinkFrame answered = reply.Value.Reply!;
    using MavlinkSchema ackShapeForPlans = MavlinkSchema.ForName("MISSION_ACK");
    using MavlinkMessage refused = ackShapeForPlans.Decode(answered.Payload);
    Console.WriteLine(
        $"refused   a request for item {past.Get("seq")} is answered"
        + $" {Name("MAV_MISSION_RESULT", refused.Get("type"))}");
}
```
<!-- end -->

## Values at a glance

**A frame on the wire,** from the MAVLink serialization guide:

| | MAVLink 1 | MAVLink 2 |
| --- | --- | --- |
| start marker | `0xFE` | `0xFD` |
| header | 6 bytes: marker, length, sequence, system, component, message id | 10 bytes: marker, length, two flag bytes, sequence, system, component, and a 3-byte message id |
| message ids | 0 to 255 | 0 to 16,777,215 |
| payload | up to 255 bytes, every base field | up to 255 bytes, with trailing zero bytes cut off |
| extension fields | never sent | sent after the base fields |
| checksum | 2 bytes, CRC-16/MCRF4XX | the same |
| signature | none | 13 more bytes when signed |

The checksum covers everything after the start marker, then the message's
`CRC_EXTRA`, a seed derived from the message's name and fields. `HEARTBEAT`'s is
50. A receiver with a different idea of the message computes a different seed
and drops the frame. A MAVLink 2 receiver must also drop a frame with an
incompatibility flag it does not understand, and pamoja's parser does; signing is
the one flag it knows.

**The heartbeat.** MAVLink leaves its rate to the link. On radio telemetry a
node typically sends one a second and treats a peer as gone after four or five
missed. A component that is not a flight controller, such as a ground station,
sets `autopilot` to `MAV_AUTOPILOT_INVALID`, which is how an autopilot is told
apart: its `autopilot` is anything else.

**The named values.** Every enumeration a field of a typed message uses is here
in full, as the MAVLink common dialect defines it:

| Enumeration | Values | Kind | In |
| --- | --- | --- | --- |
| `MAV_AUTOPILOT` | 22 | value | `HEARTBEAT.autopilot` |
| `MAV_TYPE` | 50 | value | `HEARTBEAT.type` |
| `MAV_MODE_FLAG` | 8 | bitmask | `HEARTBEAT.base_mode`, `SET_MODE.base_mode` |
| `MAV_STATE` | 9 | value | `HEARTBEAT.system_status` |
| `MAV_PROTOCOL_CAPABILITY` | 21 | bitmask | `AUTOPILOT_VERSION.capabilities` |
| `MAV_SYS_STATUS_SENSOR` | 32 | bitmask | the three `SYS_STATUS.onboard_control_sensors_*` fields |
| `MAV_SYS_STATUS_SENSOR_EXTENDED` | 8 | bitmask | their three `*_extended` fields |
| `MAV_FRAME` | 22 | value | `MISSION_ITEM_INT.frame`, `COMMAND_INT.frame`, and the two position targets |
| `MAV_CMD` | 170 | value | `MISSION_ITEM_INT.command`, `COMMAND_INT.command`, `COMMAND_LONG.command`, `COMMAND_ACK.command` |
| `MAV_PARAM_TYPE` | 10 | value | `PARAM_VALUE.param_type`, `PARAM_SET.param_type` |
| `MAV_RESULT` | 11 | value | `COMMAND_ACK.result` |
| `MAV_MISSION_RESULT` | 16 | value | `MISSION_ACK.type` |
| `MAV_SEVERITY` | 8 | value | `STATUSTEXT.severity` |
| `MAV_MISSION_TYPE` | 4 | value | the `mission_type` of seven mission messages |
| `MAV_BATTERY_TYPE` | 5 | value | `BATTERY_STATUS.type` |
| `MAV_BATTERY_FUNCTION` | 5 | value | `BATTERY_STATUS.battery_function` |
| `MAV_BATTERY_CHARGE_STATE` | 8 | value | `BATTERY_STATUS.charge_state` |
| `MAV_BATTERY_MODE` | 3 | value | `BATTERY_STATUS.mode` |
| `MAV_BATTERY_FAULT` | 9 | bitmask | `BATTERY_STATUS.fault_bitmask` |
| `MAV_VTOL_STATE` | 5 | value | `EXTENDED_SYS_STATE.vtol_state` |
| `MAV_LANDED_STATE` | 5 | value | `EXTENDED_SYS_STATE.landed_state` |
| `GPS_FIX_TYPE` | 9 | value | `GPS_RAW_INT.fix_type` |
| `POSITION_TARGET_TYPEMASK` | 12 | bitmask | the `type_mask` of both position targets |
| `MISSION_STATE` | 6 | value | `MISSION_CURRENT.mission_state` |

A name is looked up whole, as the dialect writes it: `MAV_STATE_STANDBY`, not
`STANDBY`. The dialect's names are not all regular, so `MAV_RESULT_CANCELLED`
keeps its double l and `MAV_MISSION_RESULT`'s entries start `MAV_MISSION_`. A
value no entry names, from a newer dialect or a vendor's, looks up as nothing.

**A command's answer,** as `MAV_RESULT` defines it:

| Result | Means | The sender |
| --- | --- | --- |
| `MAV_RESULT_ACCEPTED` | it ran | is done |
| `MAV_RESULT_TEMPORARILY_REJECTED` | it cannot run yet, and waiting may fix that | may ask again later |
| `MAV_RESULT_DENIED` | it is supported, but a parameter is not | changes the parameter |
| `MAV_RESULT_UNSUPPORTED` | the vehicle does not know it | stops asking |
| `MAV_RESULT_FAILED` | it ran and failed | fixes the cause before asking again |
| `MAV_RESULT_IN_PROGRESS` | it is running, and more answers follow with a percentage | keeps waiting |
| `MAV_RESULT_CANCELLED` | a `COMMAND_CANCEL` stopped it | is done |
| `MAV_RESULT_COMMAND_LONG_ONLY` | it is only accepted as a `COMMAND_LONG` | sends it as one |
| `MAV_RESULT_COMMAND_INT_ONLY` | it is only accepted as a `COMMAND_INT` | sends it as one |
| `MAV_RESULT_COMMAND_UNSUPPORTED_MAV_FRAME` | the coordinate frame it gave is not supported | gives another frame |
| `MAV_RESULT_NOT_IN_CONTROL` | the sender is not in control of the vehicle | gains control first |

A resend carries the next `confirmation` number, starting from `0`, and an
answer names the command it answers.

**A signature,** from the MAVLink signing guide:

| Part | Size | Holds |
| --- | --- | --- |
| link id | 1 byte | which of the sender's links the frame left on |
| timestamp | 6 bytes | ten-microsecond ticks since 1 January 2015, rising with every frame on a link |
| signature | 6 bytes | the first 48 bits of a SHA-256 over the key, the frame, the link id, and the timestamp |
| key | 32 bytes, never sent | the secret both ends share |

The verifier refuses a signed frame whose signature does not match its key, whose
timestamp is not newer than the last one from the same system, component, and
link, or which starts a new stream more than a minute, 6,000,000 ticks, behind
the newest timestamp it has accepted.

**Moving a plan,** as the mission protocol runs an upload:

| Turn | From | Message |
| --- | --- | --- |
| 1 | station | `MISSION_COUNT`, the number of items |
| 2 | vehicle | `MISSION_REQUEST_INT` for item 0 |
| 3 | station | `MISSION_ITEM_INT` 0 |
| and so on | | a request and an item for each one |
| last | vehicle | `MISSION_ACK`, with a `MAV_MISSION_RESULT` |

A download runs the other way, opened by the station's `MISSION_REQUEST_LIST`.
Each side times out a message that needs an answer and sends it again: after
1500 ms by default and 250 ms for plan items, up to 5 times, as the protocol
recommends. The machines keep no clock, so the caller does the waiting.
`MAV_MISSION_ACCEPTED` ends a transfer that worked; `MAV_MISSION_INVALID_SEQUENCE`,
`MAV_MISSION_NO_SPACE`, and `MAV_MISSION_UNSUPPORTED_FRAME` are the refusals a
plan most often meets.

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| build and read a message | `Heartbeat { .. }`, `Frame::encode_message(header, &message)`, `frame.decode_message()`, `Parser::new()`, `push_byte(byte, &dialect::crc_extra)` |
| name a value | `mav_type::QUADROTOR`, `dialect::entry_value(entry)`, `dialect::enum_named(enumeration)`, `entry(value)`, `names(value)` |
| see a command through | `CommandProtocol::new(command, retries)`, `confirmation()`, `on_frame(&frame)`, `on_timeout()` |
| sign and check | `Signer::new(key, link_id, timestamp)`, `sign(header, id, payload, crc_extra)`, `Verifier::new(key)`, `verify(&frame)`, `signing::timestamp_from_unix_micros(micros)` |
| move a plan | `MissionSender::new(&items, system, component, mission_type)`, `count_frame(header)`, `on_frame(&frame, header)`, `MissionReceiver::new(system, component, mission_type)`, `on_frame(&frame, header)`, `expected()` |

### TypeScript

| To | Call |
| --- | --- |
| build and read a message | `schemaFor(name)`, `fromObject(schema, fields)`, `message(name)`, `toFrame(header)`, `MavlinkMessage.decode(schema, payload)`, `new MavlinkParser().push(bytes)` |
| name a value | `enumValue(entry)`, `enumEntry(enumeration, value)`, `enumNames(enumeration, value)`, `enumEntries(enumeration)`, `knownEnums()` |
| see a command through | `new CommandProtocol(command, retries)`, `confirmation`, `onFrame(frame)`, `onTimeout()` |
| sign and check | `new MavlinkSigner(key, linkId, timestampNow())`, `sign(header, id, payload, crcExtra)`, `new MavlinkVerifier(key)`, `verify(frame)` |
| move a plan | `new MissionSender(system, component, missionType)`, `addItem(payload)`, `count(header)`, `onFrame(frame, header)`, `new MissionReceiver(system, component, missionType)`, `expected` |

### Python

| To | Call |
| --- | --- |
| build and read a message | `schema_for(name)`, `from_dict(schema, fields)`, `message(name)`, `to_frame(header)`, `MavlinkMessage.decode(schema, payload)`, `get_int(field)`, `MavlinkParser().push(data)` |
| name a value | `enum_value(entry)`, `enum_entry(enumeration, value)`, `enum_names(enumeration, value)`, `enum_entries(enumeration)`, `known_enums()` |
| see a command through | `CommandProtocol(command, retries)`, `confirmation`, `on_frame(frame)`, `on_timeout()` |
| sign and check | `MavlinkSigner(key, link_id, timestamp_now())`, `sign(header, id, payload, crc_extra)`, `MavlinkVerifier(key)`, `verify(frame)` |
| move a plan | `MissionSender(system, component, mission_type)`, `add_item(payload)`, `count(header)`, `on_frame(frame, header)`, `MissionReceiver(system, component, mission_type)`, `expected` |

### C#

| To | Call |
| --- | --- |
| build and read a message | `MavlinkSchema.ForName(name)`, `CreateMessage()`, `Set(field, value)`, `ToFrame(header)`, `Decode(payload)`, `new MavlinkParser().Push(bytes)` |
| name a value | `MavlinkEnum.Value(entry)`, `MavlinkEnum.Entry(enumeration, value)`, `MavlinkEnum.Names(enumeration, value)`, `MavlinkEnum.Entries(enumeration)`, `MavlinkEnum.Known()` |
| see a command through | `new MavlinkCommand(command, retries)`, `Confirmation`, `OnFrame(frame)`, `OnTimeout()` |
| sign and check | `new MavlinkSigner(key, linkId, Mavlink.TimestampNow())`, `Sign(header, id, payload, crcExtra)`, `new MavlinkVerifier(key)`, `Verify(frame)` |
| move a plan | `new MavlinkMissionSender(system, component, missionType)`, `AddItem(message)`, `CountFrame(header)`, `OnFrame(frame, header)`, `new MavlinkMissionReceiver(system, component, missionType)`, `Expected` |

<!-- languages end -->

## When it goes wrong

A frame the rules refuse is dropped by the parser, or refused with its reason by
the verifier. What gets past them shows up as a vehicle that never answers, or
answers something else. The ones that cost an afternoon:

- **Frames of one message never arrive.** A parser that does not know a
  message's `CRC_EXTRA` cannot check its frame, so it drops it rather than trust
  it. A message from ArduPilot's dialect, PX4's, or a vendor's needs its seed in
  a `Dialect`, which `MessageSchemaBuilder` derives from the definition.
- **Every signed frame is refused after a restart.** A signer that starts again
  from a fixed timestamp sends timestamps the vehicle has already passed. Seed it
  from the clock with `timestamp_now`, or carry the last timestamp across the
  restart.
- **A command runs twice.** A resend that keeps confirmation `0` looks like a new
  command, so a vehicle that did hear the first may act on both.
  `CommandProtocol` numbers each resend.
- **The wrong answer ends the wait.** A sender that takes the next
  `COMMAND_ACK` as its answer can read another command's result. An answer names
  its command, and `CommandProtocol` matches on it.
- **An upload stalls halfway.** The vehicle drives the transfer, so a lost
  request or item leaves both sides waiting. The machines keep no timers: time
  out, send the last frame again, and give up after five tries.
- **A field reads zero from an older autopilot.** A peer that predates a MAVLink
  2 extension field does not send it, and it decodes as zero. The dialect gives
  such fields a zero that means not provided, as in
  `BATTERY_STATUS.time_remaining`, so read a zero there as unknown rather than as
  a measurement.
- **A value prints as a number.** `enumEntry` looks up nothing for a value no
  entry names, from a newer dialect or a vendor's. Print the number then, rather
  than a name that is not there.

## Where next

<!-- table: next mavlink -->
- [ROS 2 rules](ros2.md): ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed.
- [Serial framing](serial.md): SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets.
- [Telemetry](telemetry.md): Observability that ships only what is worth the bytes as link cost rises, while counting everything.
<!-- end -->

## Reference

<!-- table: reference mavlink -->
- Rust: [`pamoja-mavlink`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_mavlink/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-mavlink)
- TypeScript: [`@pamoja/mavlink`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-mavlink)
- Python: [`pamoja.mavlink`](https://pamoja.molex.cloud/docs/reference/python/pamoja/mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-mavlink)
- C#: [`Pamoja.Mavlink`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Mavlink.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-mavlink)
- Hardware: [ArduPilot](https://pamoja.molex.cloud/docs/hardware.html#ardupilot), [PX4 Autopilot](https://pamoja.molex.cloud/docs/hardware.html#px4), [Pixhawk standard](https://pamoja.molex.cloud/docs/hardware.html#pixhawk)
<!-- end -->
