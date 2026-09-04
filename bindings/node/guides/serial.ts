// The serial framing guide example; see docs/guides/serial.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { SlipDecoder, cobs, slip } from '@pamoja/serial'

// SLIP reserves two byte values, 0xC0 to end a frame and 0xDB to escape, so a payload
// carrying either goes out as the two-byte pair RFC 1055 fixes for it.
const payload = Buffer.from([0x01, 0xc0, 0xdb, 0x02])
const frame = slip.encode(payload)
assert.deepEqual([...frame], [0x01, 0xdb, 0xdc, 0xdb, 0xdd, 0x02, 0xc0])
assert.deepEqual(slip.decode(frame), payload)

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
// run led by its own length. This is the worked example from the COBS paper.
const packet = Buffer.from([0x11, 0x22, 0x00, 0x33])
const framed = cobs.encode(packet)
assert.deepEqual([...framed], [0x03, 0x11, 0x22, 0x02, 0x33, 0x00])
assert.deepEqual(cobs.decode(framed), packet)

// A serial read returns an arbitrary chunk rather than a packet. This one holds two
// frames with a truncated one between them, and the decoder drops only the bad frame.
const decoder = new SlipDecoder()
const frames = decoder.feed(Buffer.from([0x6f, 0x6b, 0xc0, 0xdb, 0xc0, 0x67, 0x6f, 0xc0]))
assert.deepEqual(frames, [Buffer.from('ok'), Buffer.from('go')])
assert.equal(decoder.discarded, 1)
// ANCHOR_END: example
