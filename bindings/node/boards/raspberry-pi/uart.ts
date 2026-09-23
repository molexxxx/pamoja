// A UART self-test: the Pi's own serial port with TX jumpered to RX, sending COBS frames and
// reading each one straight back. Turn the serial port hardware on and the serial console off
// (raspi-config, Interface Options, Serial Port), reboot, and put one jumper between GPIO14 and
// GPIO15. See docs/guides/serial.md.

// ANCHOR: example
import { setTimeout as sleep } from 'node:timers/promises'
import { SerialPort } from '@pamoja/hal'
import { CobsDecoder, cobs } from '@pamoja/serial'

// The primary UART, on GPIO14 and GPIO15 on every model but the Raspberry Pi 5.
const PORT = '/dev/serial0'

async function main(): Promise<void> {
  const port = SerialPort.open(PORT, { baud: 115200 })
  const decoder = new CobsDecoder()

  for (let sequence = 1; sequence <= 5; sequence += 1) {
    const number = Buffer.alloc(2)
    number.writeUInt16BE(sequence)
    const payload = Buffer.concat([number, Buffer.from('ping')])
    const frame = cobs.encode(payload)
    const started = performance.now()
    await port.write(frame)

    // The frame comes back on RX as it goes out on TX. Read until the decoder has it, or
    // until the line has been quiet for 100 ms.
    let echoed: Buffer | undefined
    while (echoed === undefined) {
      const arrived = await port.read(64, 100)
      if (arrived.length === 0) {
        break
      }
      echoed = decoder.feed(arrived)[0]
    }
    if (echoed === undefined) {
      console.log(`frame ${sequence}  nothing came back: check the jumper, and that the console is off`)
    } else if (echoed.equals(payload)) {
      const millis = performance.now() - started
      console.log(`frame ${sequence}  ${frame.length} bytes back in ${millis.toFixed(2)} ms`)
    } else {
      console.log(`frame ${sequence}  came back changed: check the speed and the wiring`)
    }
    await sleep(500)
  }
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
// ANCHOR_END: example
