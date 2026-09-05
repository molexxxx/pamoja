// The CAN and J1939 guide example; see docs/guides/can.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import {
  NOT_AVAILABLE,
  broadcastJ1939,
  composeJ1939,
  decodeJ1939,
  fdFrame,
  frame,
  priority,
  signals,
  signalsFrom,
} from '@pamoja/can'

// The nodes on this bus, by the address each answers to, and the two parameter groups
// in play. J1939 publishes both, so naming them is what makes the traffic readable.
const ENGINE = 0
const GATEWAY = 1
const GEARBOX = 33
const ENGINE_CONTROLLER_1 = 61_444 // carries engine speed
const REQUEST = 59_904 // asks another node for a parameter group

// Where engine speed sits inside that group, and the scale the standard fixes for it.
// Naming both is what stops a sender and a receiver disagreeing about either.
const ENGINE_SPEED_AT = 3
const RPM_PER_BIT = 0.125

// J1939 keeps its addressing inside the CAN identifier: a priority, the parameter
// group, and the address of whatever sent it. A broadcast has no destination, so it is
// its own constructor rather than a magic address a caller has to know.
const speedId = broadcastJ1939(priority.control, ENGINE_CONTROLLER_1, ENGINE)
const speed = decodeJ1939(speedId)!
console.log(`broadcast pgn ${speed.pgn} at priority ${speed.priority}`)

// A parameter group below the PDU1 limit is addressed rather than broadcast, so those
// eight identifier bits carry a destination instead of extending the group number.
const requestId = composeJ1939(priority.default, REQUEST, GATEWAY, GEARBOX)
console.log(`request   pgn ${decodeJ1939(requestId)!.pgn} addressed to node ${GEARBOX}`)

// Reading one back off the bus is the same thing in reverse, so a receiver never
// unpacks 29 bits by hand.
const heard = decodeJ1939(requestId)!
console.log(`heard     from node ${heard.source} for node ${heard.destination}`)

// The payload. Every signal starts marked not available, and this controller reports
// only engine speed, so that is the only one it writes.
const reported = signals()
reported.setU16(ENGINE_SPEED_AT, 1000 / RPM_PER_BIT)
const eec1 = frame(speedId, reported.bytes, true)

// The receiving node reads the same offset back, so neither end slices the payload.
const rpm = signalsFrom(eec1.data).u16(ENGINE_SPEED_AT)! * RPM_PER_BIT
console.log(`engine    ${rpm} rpm, carried in ${eec1.dlc} bytes`)

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
console.log(`32 bytes carries length code ${fdFrame(speedId, new Uint8Array(32), true).dlc}`)
try {
  frame(speedId, new Uint8Array(9), true)
  console.log('a classic frame took nine bytes, which should never happen')
} catch (error) {
  console.log(`classic   refused nine bytes: ${(error as Error).message}`)
}

// J1939 never rides an 11-bit identifier, so a standard frame is not one of its
// messages however its bits happen to line up.
console.log(`an 11-bit identifier is J1939: ${decodeJ1939(291, false) !== null}`)
// ANCHOR_END: example

assert.equal(speed.priority, priority.control)
assert.equal(speed.pgn, ENGINE_CONTROLLER_1)
assert.ok(speed.broadcast && speed.destination === null)
assert.equal(heard.pgn, REQUEST)
assert.equal(heard.destination, GEARBOX)
assert.equal(heard.source, GATEWAY)
assert.equal(rpm, 1000)
assert.equal(eec1.dlc, 8)
assert.equal(fdFrame(speedId, new Uint8Array(32), true).dlc, 13)
assert.equal(decodeJ1939(291, false), null)
assert.equal(reported.u8(0), NOT_AVAILABLE)
