// The serial framing guide example: a weather mast whose node sends COBS frames up a UART to a
// gateway; see docs/guides/serial.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { SerialPort } from '@pamoja/hal'
import { COBS_DELIMITER_BYTE, CobsDecoder, cobs, slip } from '@pamoja/serial'

// A reading is a two-byte sequence number, most significant byte first, then its text.
function reading(sequence: number, text: string): Buffer {
  const number = Buffer.alloc(2)
  number.writeUInt16BE(sequence)
  return Buffer.concat([number, Buffer.from(text)])
}

async function main() {
  // The line: 115200 baud, eight data bits, no parity, one stop bit. Ten bits a character.
  const settings = { baud: 115200 }
  const line = SerialPort.describe(settings)
  const characterMicros = SerialPort.characterNanos(settings) / 1000
  console.log(
    `line         ${line}, ${SerialPort.bitsPerCharacter(settings)} bits a character, ` +
      `${characterMicros.toFixed(2)} us each`,
  )

  // The two ends of the cable with nothing plugged in. On a Raspberry Pi the gateway's end is
  // SerialPort.open('/dev/serial0', settings) and nothing after this statement changes.
  const [gateway, node] = SerialPort.pair(settings)

  // A UART carries bytes, and nothing marks where a message ends, so the node frames each
  // reading with COBS: zero becomes the one byte that ends a frame and never appears inside
  // one, which matters here, since the sequence number is full of zeros.
  const texts = ['wind=12.4', 'wind=13.1', 'wind=11.8']
  let sent = 0
  for (const [index, text] of texts.entries()) {
    const frame = cobs.encode(reading(index + 1, text))
    await node.write(frame)
    sent += frame.length
  }
  console.log(
    `node         ${texts.length} readings of ${reading(1, texts[0]).length} bytes, framed as ${sent} bytes`,
  )

  // A read returns whatever has arrived, which is rarely one frame: here it is all three. The
  // decoder splits the stream back into payloads at each delimiter.
  const arrived = await gateway.read(256, 100)
  console.log(`gateway      ${arrived.length} bytes in one read`)
  const decoder = new CobsDecoder()
  const payloads = decoder.feed(arrived)
  for (const payload of payloads) {
    console.log(`reading ${payload.readUInt16BE(0)}    ${payload.subarray(2).toString()}`)
  }

  // What one frame costs on the wire at this speed, start and stop bits included.
  const frameLength = sent / texts.length
  const frameMillis = SerialPort.transferMicros(settings, frameLength) / 1000
  console.log(`on the wire  ${frameMillis.toFixed(2)} ms for a ${frameLength}-byte frame at ${line}`)

  // The node restarts partway through a frame. As it comes back up it sends a lone delimiter,
  // which closes off the half frame, so the gateway drops it rather than gluing it to the next
  // one, and then it sends the reading again.
  const again = cobs.encode(reading(4, 'wind=12.9'))
  await node.write(again.subarray(0, Math.floor(again.length / 2)))
  await node.write(Buffer.from([COBS_DELIMITER_BYTE]))
  await node.write(again)
  const droppedBefore = decoder.discarded
  const resent = decoder.feed(await gateway.read(256, 100))
  const dropped = decoder.discarded - droppedBefore
  console.log(
    `restart      ${dropped} frame cut short and dropped, then ${resent[0].subarray(2).toString()}`,
  )

  // SLIP, the older framing, ends a frame with one reserved byte and escapes that byte and its
  // own escape byte inside one. With no reserved bytes in a reading it costs a byte less than
  // COBS; a payload full of them costs up to twice its length under SLIP, and never more than
  // one byte in 254 over under COBS.
  const first = reading(1, texts[0])
  const slipLength = slip.encode(first).length
  const cobsLength = cobs.encode(first).length
  console.log(
    `framing      ${first.length} payload bytes: ${slipLength} under SLIP, ${cobsLength} under COBS`,
  )

  // The node goes quiet. A read waits for the first byte up to its timeout; on a port with
  // nothing plugged in it resolves at once and counts the wait instead of sleeping through it,
  // so a test of a silent node takes no time.
  const waitedBefore = gateway.waitedMicros
  const quiet = await gateway.read(256, 500)
  const waited = (gateway.waitedMicros - waitedBefore) / 1000
  console.log(`silence      ${quiet.length} bytes in ${waited} ms, counted and not slept`)

  return { settings, payloads, sent, dropped, resent, slipLength, cobsLength, gateway, node, again }
}

main()
  // ANCHOR_END: example
  .then(check)

function check(seen: Awaited<ReturnType<typeof main>>): void {
  assert.equal(SerialPort.characterNanos(seen.settings), 86806, '10 bits at 115200')
  assert.deepEqual(seen.payloads, [
    reading(1, 'wind=12.4'),
    reading(2, 'wind=13.1'),
    reading(3, 'wind=11.8'),
  ])
  assert.equal(seen.sent, 39, 'each 11-byte payload gains one code byte and a delimiter')
  assert.equal(SerialPort.transferMicros(seen.settings, 13), 1129)
  assert.equal(seen.dropped, 1)
  assert.deepEqual(seen.resent, [reading(4, 'wind=12.9')])
  assert.deepEqual([seen.slipLength, seen.cobsLength], [12, 13])
  assert.equal(seen.gateway.waitedMicros, 500000)
  assert.equal(seen.node.written, 39 + Math.floor(seen.again.length / 2) + 1 + seen.again.length)
}
