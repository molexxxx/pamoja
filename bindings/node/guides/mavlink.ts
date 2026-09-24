// The MAVLink guide example; see docs/guides/mavlink.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
// ANCHOR_END: example

assert.deepEqual(heard.payload, vehicle.payload)
assert.equal(frames.length, 1)
assert.equal(frames[0]!.messageId, heartbeatShape.id)

// ANCHOR: command
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
// ANCHOR_END: command

assert.equal(arming.confirmation, 1)
assert.equal(sends, 4)

// ANCHOR: signing
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
// ANCHOR_END: signing

assert.ok(signed.signed)
assert.ok(!sent.signed)

// ANCHOR: mission
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
// ANCHOR_END: mission

assert.ok(vehicleSide.complete)
assert.equal(finished, enumValue('MAV_MISSION_ACCEPTED'))
