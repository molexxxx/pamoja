// The MAVLink guide example; see docs/guides/mavlink.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
// ANCHOR_END: example

assert.equal(heard.get('type'), MAV_TYPE_QUADROTOR)
assert.equal(received.messageId, 0)
assert.equal(resend, 1)
assert.equal(stray?.kind, 'unrelated')
assert.equal(outcome?.kind, 'final')
assert.equal(outcome?.value, MAV_RESULT_ACCEPTED)
