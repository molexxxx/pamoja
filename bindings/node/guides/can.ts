// The CAN and J1939 guide example; see docs/guides/can.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { composeJ1939, decodeJ1939, fdFrame, frame } from '@pamoja/can'

// J1939 keeps its addressing inside the CAN identifier: a priority, a parameter group
// that says what the message is, and the address of whatever sent it. Building one from
// those fields is what saves a caller packing 29 bits by hand.
const ENGINE = 0x00
const EEC1 = 61_444 // electronic engine controller 1, which carries engine speed
const broadcast = composeJ1939(3, EEC1, ENGINE)
const engine = decodeJ1939(broadcast)!
console.log(`broadcast priority ${engine.priority} pgn ${engine.pgn}`)
console.log(`addressed to one node: ${!engine.broadcast}`)

// A parameter group below the PDU1 limit is addressed rather than broadcast, so those
// eight identifier bits carry a destination instead of extending the group number.
const REQUEST = 59_904
const GATEWAY = 0x01
const TRANSMISSION = 0x21
const request = decodeJ1939(composeJ1939(6, REQUEST, GATEWAY, TRANSMISSION))!
const hex = (value: number) => `0x${value.toString(16).toUpperCase().padStart(2, '0')}`
console.log(`request   pgn ${request.pgn} to node ${hex(request.destination!)}`)
console.log(`heard     from ${hex(request.source)}`)

// J1939 never rides an 11-bit identifier, so a standard frame is not one.
console.log(`an 11-bit identifier is J1939: ${decodeJ1939(0x123, false) !== null}`)

// The frame that carries the broadcast. Engine speed sits in bytes 4 and 5 of that
// parameter group at 0.125 rpm per bit, and every signal this controller is not
// reporting is filled with the not-available byte the standard reserves.
const payload = Buffer.alloc(8, 0xff)
payload.writeUInt16LE(1000 / 0.125, 3)
const eec1 = frame(broadcast, payload, true)
const speed = eec1.data.readUInt16LE(3) * 0.125
console.log(`engine    ${speed} rpm in ${eec1.dlc} bytes`)

// Above eight bytes CAN-FD encodes the length in steps rather than exactly, and a
// classic frame still refuses a ninth byte.
console.log(`32 bytes carries length code ${fdFrame(broadcast, Buffer.alloc(32), true).dlc}`)
try {
  frame(broadcast, Buffer.alloc(9), true)
  console.log('a classic frame took nine bytes, which should never happen')
} catch (error) {
  console.log(`classic   refused nine bytes: ${(error as Error).message}`)
}
// ANCHOR_END: example

assert.equal(engine.priority, 3)
assert.equal(engine.pgn, EEC1)
assert.ok(engine.broadcast && engine.destination === null)
assert.equal(request.pgn, REQUEST)
assert.equal(request.destination, TRANSMISSION)
assert.equal(request.source, GATEWAY)
assert.equal(decodeJ1939(0x123, false), null)
assert.equal(eec1.dlc, 8)
assert.equal(speed, 1000)
assert.equal(fdFrame(broadcast, Buffer.alloc(32), true).dlc, 13)
