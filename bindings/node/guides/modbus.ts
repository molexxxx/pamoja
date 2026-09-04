// The Modbus RTU guide example; see docs/guides/modbus.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { crc16, parseFrame, readHoldingRegisters } from '@pamoja/modbus'

// Ask unit 0x11 for three holding registers starting at 0x006B. The last two bytes are
// the CRC-16/MODBUS, so this is the frame exactly as it goes out on the wire.
const request = readHoldingRegisters(0x11, 0x006b, 3)
assert.deepEqual([...request], [0x11, 0x03, 0x00, 0x6b, 0x00, 0x03, 0x76, 0x87])

// The device answers with three 16-bit registers. A reply carries its own checksum, so
// the receiver validates the frame before reading any value out of it.
const body = Buffer.from([0x11, 0x03, 0x06, 0x02, 0x2b, 0x00, 0x00, 0x00, 0x64])
const checksum = Buffer.alloc(2)
checksum.writeUInt16LE(crc16(body))
const reply = parseFrame(Buffer.concat([body, checksum]))
assert.equal(reply.address, 0x11)
assert.equal(reply.exception, null)
assert.deepEqual(reply.registers(), [0x022b, 0x0000, 0x0064])

// One flipped bit anywhere in the frame fails the checksum, which is the whole point of
// carrying one over a long RS485 run.
const corrupt = Buffer.concat([body, checksum])
corrupt[2] ^= 0xff
assert.throws(() => parseFrame(corrupt))
// ANCHOR_END: example
