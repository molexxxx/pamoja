# @pamoja/serial

SLIP and COBS byte stuffing with streaming decoders, so a UART byte stream carries discrete packets. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/serial.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html)

## Install

```sh
npm install @pamoja/serial
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/serial.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/serial.ts):

```typescript
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-serial`](https://crates.io/crates/pamoja-serial) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_serial/index.html), [docs.rs](https://docs.rs/pamoja-serial), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-serial) |
| TypeScript | [`@pamoja/serial`](https://www.npmjs.com/package/@pamoja/serial) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-serial) |
| Python | [`pamoja-serial`](https://pypi.org/project/pamoja-serial/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/serial.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-serial) |
| C# | [`Pamoja.Serial`](https://www.nuget.org/packages/Pamoja.Serial) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Serial.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-serial) |

## Documentation

- [`@pamoja/serial` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_serial.html), every class, function, and type this package exports.
- [The Serial framing guide](https://pamoja.molex.cloud/docs/guides/serial.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
