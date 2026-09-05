// The serial framing guide example; see docs/guides/serial.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import {
  COBS_DELIMITER_BYTE,
  SLIP_END_BYTE,
  SLIP_ESC_BYTE,
  SlipDecoder,
  cobs,
  slip,
} from '@pamoja/serial'

// A UART carries bytes, not packets, so a framing has to mark where one packet ends.
// SLIP reserves two byte values for that, and the package names both: the end byte closes
// a frame, the escape byte carries a value that would otherwise look like one.
const payload = Buffer.concat([Buffer.from('lvl='), Buffer.from([SLIP_END_BYTE, SLIP_ESC_BYTE])])
const framed = slip.encode(payload)
console.log(`slip      ${payload.length} payload bytes framed as ${framed.length}`)

// Decoding gives the payload back unchanged, reserved bytes and all.
const restored = slip.decode(framed)
console.log(`slip      decoded back to ${restored.length} bytes`)

// COBS trades that escaping for one code byte per run of up to 254 non-zero bytes, each
// run led by its own length, so a frame never grows by more than a byte per 254. Zero is
// the delimiter, and never appears inside a frame.
const packet = Buffer.concat([Buffer.from('lvl='), Buffer.from([COBS_DELIMITER_BYTE]), Buffer.from('7')])
const cobsFramed = cobs.encode(packet)
console.log(`cobs      ${packet.length} payload bytes framed as ${cobsFramed.length}`)

// A read from a port returns whatever arrived, which is rarely one whole frame. This
// chunk holds two good frames with a truncated one between them; the decoder hands over
// the good ones and discards only the bad frame.
const decoder = new SlipDecoder()
const chunk = Buffer.concat([
  Buffer.from('ok'),
  Buffer.from([SLIP_END_BYTE]),
  Buffer.from([SLIP_ESC_BYTE]), // a frame that ends before its escape pair completes
  Buffer.from([SLIP_END_BYTE]),
  Buffer.from('go'),
  Buffer.from([SLIP_END_BYTE]),
])
const frames = decoder.feed(chunk)
for (const frame of frames) {
  console.log(`received  ${frame.toString()}`)
}
console.log(`discarded ${decoder.discarded} frame the stream mangled`)
// ANCHOR_END: example

// The bytes each specification fixes are pinned once, in the crate tests and the
// generated conformance vectors, so a guide asserts behaviour instead.
assert.ok(framed.length > payload.length)
assert.ok(cobsFramed.length > packet.length)
assert.deepEqual([...restored], [...payload])
assert.deepEqual(cobs.decode(cobsFramed), packet)
assert.deepEqual(frames, [Buffer.from('ok'), Buffer.from('go')])
assert.equal(decoder.discarded, 1)
