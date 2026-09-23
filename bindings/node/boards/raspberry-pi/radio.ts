// A LoRa radio on the header: an RFM95W breakout on SPI0, beaconing a reading and printing
// every frame it hears in between. Wire the breakout's VIN to a 3V3 pin, GND to ground, SCK
// to GPIO11, MISO to GPIO9, MOSI to GPIO10, CS to GPIO8 (CE0), and RST to GPIO25, and screw
// on an antenna for the band before powering it. Two boards running it hear each other. See
// docs/boards/raspberry-pi.md.

// ANCHOR: example
import { LoraRegion, linkBudget, maxTransmitPowerDbm, planFor } from '@pamoja/lora'
import { DutyCycle, LoraRadio, ReceptionOutcome, sx127x } from '@pamoja/radios'

// The header's first SPI chip select, the GPIO chip its lines are on, and the line the
// breakout's reset pin is wired to.
const SPI = '/dev/spidev0.0'
const CHIP = '/dev/gpiochip0'
const RESET_LINE = 25

// The channel this node uses, the data rate it sends at, and how long it listens between
// beacons.
const FREQUENCY_HZ = 868_100_000
const DATA_RATE = 3
const LISTEN_US = 10_000_000

async function main(): Promise<void> {
  // The regional plan decides the channel's power ceiling and its duty cycle, so no limit
  // below is a number anyone has to remember.
  const plan = planFor(LoraRegion.Eu868)
  const link = plan.linkSettings(DATA_RATE)!
  const ceilingDbm = plan.maxEirpDbm(FREQUENCY_HZ)
  const permille = plan.dutyCyclePermille(FREQUENCY_HZ)!

  // A 2.15 dBi whip on half a decibel of pigtail. The antenna's gain counts against the
  // ceiling and the pigtail's loss counts for it, so the amplifier takes what is left.
  const whip = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
  const outputDbm = Math.floor(maxTransmitPowerDbm(whip, ceilingDbm))

  // Opening resets the chip and reads its version back, so a wiring mistake is caught here
  // rather than on the first frame.
  const radio = LoraRadio.openSx127x(
    { spi: SPI, gpioChip: CHIP, resetLine: RESET_LINE },
    { output: sx127x.PaOutput.PaBoost },
  )
  await radio.configure({ frequencyHz: FREQUENCY_HZ, link, outputDbm })
  console.log(
    `beacon on ${FREQUENCY_HZ} Hz at DR${DATA_RATE}, ${outputDbm} dBm under a ${ceilingDbm} dBm ceiling`,
  )

  // The duty cycle is the radio's other budget: each frame buys silence in proportion to its
  // airtime, and the guard says when the next one may go out.
  const duty = new DutyCycle(permille)
  const started = process.hrtime.bigint()
  const nowUs = (): number => Number((process.hrtime.bigint() - started) / 1000n)
  let reading = 0

  for (;;) {
    // Listening returns as soon as a frame arrives, and a frame comes with the levels it was
    // heard at: how strong it was, and how far above the noise.
    const heard = await radio.receive(LISTEN_US)
    if (heard.outcome === ReceptionOutcome.Frame) {
      console.log(
        `heard  ${heard.payload?.toString()} at ${heard.rssiDbm?.toFixed(0)} dBm, SNR ${heard.snrDb?.toFixed(1)} dB`,
      )
    } else if (heard.outcome === ReceptionOutcome.Corrupt) {
      console.log('heard  a frame whose CRC failed')
    }

    const now = nowUs()
    if (duty.ready(now)) {
      const frame = `pi reading ${reading}`
      const airtimeUs = await radio.transmit(Buffer.from(frame))
      duty.transmitted(now, link, frame.length)
      console.log(`sent   ${frame} in ${airtimeUs} us on air`)
      reading += 1
    }
  }
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
// ANCHOR_END: example
