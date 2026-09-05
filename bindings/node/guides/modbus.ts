// The Modbus RTU guide example; see docs/guides/modbus.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { parseFrame, readHoldingRegisters, readHoldingRegistersReply } from '@pamoja/modbus'

// The device this gateway polls: a power meter at unit 17, whose manual says the three
// registers holding voltage, current and a fault word start at address 107.
const METER = 17
const FIRST_REGISTER = 107

// Ask it for those three registers. The frame is complete, checksum included, exactly as
// it goes out on the wire.
const request = readHoldingRegisters(METER, FIRST_REGISTER, 3)
console.log(`polling unit ${METER}, ${request.length} bytes out`)

// A stand-in for the meter. On a running gateway this frame arrives over RS485; here the
// library builds what a meter reporting those three values would send back.
const fromTheMeter = readHoldingRegistersReply(METER, [2301, 418, 0])

// Everything below is the gateway's own code. A reply carries its own checksum, so the
// frame is validated before any value is read out of it.
const reply = parseFrame(fromTheMeter)
const registers = reply.registers()
console.log(`voltage   ${(registers[0] / 10).toFixed(1)} V`)
console.log(`current   ${(registers[1] / 100).toFixed(2)} A`)
console.log(`faults    ${registers[2]}`)

// One flipped bit anywhere in the frame fails the checksum, which is the whole point of
// carrying one over a long RS485 run.
const mangled = Buffer.from(fromTheMeter)
mangled[2] ^= 0xff
try {
  parseFrame(mangled)
  console.log('mangled frame accepted, which should never happen')
} catch (error) {
  console.log(`mangled frame rejected: ${(error as Error).message}`)
}
// ANCHOR_END: example

// The bytes each specification fixes are pinned once, in the crate tests and the
// generated conformance vectors, so a guide asserts behaviour instead.
assert.equal(request.length, 8)
assert.equal(reply.address, METER)
assert.equal(reply.exception, null)
assert.deepEqual(registers, [2301, 418, 0])
assert.throws(() => parseFrame(mangled))
