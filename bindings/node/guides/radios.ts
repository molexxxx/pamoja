// The LoRa radio guide example; see docs/guides/radios.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { LoraRegion, linkBudget, planFor } from '@pamoja/lora'
import { DutyCycle, SimulatedLoraChip, sx126x } from '@pamoja/radios'

async function onTheBench() {
  // An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
  // 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the
  // antenna and pigtail decide how hard the amplifier may drive under that cap.
  const eu868 = planFor(LoraRegion.Eu868)
  const frequency = 868_100_000
  const link = eu868.linkSettings(3)!
  const whip = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
  const ceiling = eu868.maxEirpDbm(frequency)
  const power = sx126x.txPowerUnderCeiling(sx126x.Amplifier.HighPower, whip, ceiling)
  console.log(`power     ${power.settingDbm} dBm under a ${ceiling} dBm EIRP ceiling`)

  // A simulated SX1262 stands in for the chip on the node's board, driven by the same code
  // that drives a real one, and it reports what that code told it.
  const chip = SimulatedLoraChip.sx126x({ amplifier: sx126x.Amplifier.HighPower })
  const radio = chip.radio()
  await radio.configure({ frequencyHz: frequency, link, outputDbm: power.settingDbm })
  const tuned = chip.tuning()
  console.log(
    `tuned     ${(tuned.frequencyHz / 1e6).toFixed(1)} MHz, SF${tuned.link.spreadingFactor} ` +
      `at ${tuned.link.bandwidthHz / 1000} kHz, ${tuned.outputDbm} dBm`,
  )

  // The reading goes out, and the airtime comes back for the duty-cycle guard. The sub-band
  // that holds 868.1 MHz allows 1% of the time, so the frame buys ninety-nine times as long
  // in silence before the next.
  const reading = Buffer.from('level=0.42')
  const airtime = await radio.transmit(reading)
  console.log(`sent      ${chip.sent()[0].payload.length} bytes, ${airtime} us on air`)
  const guard = new DutyCycle(eu868.dutyCyclePermille(frequency)!)
  guard.transmitted(0, link, reading.length)
  console.log(`silence   the next frame starts ${guard.waitUs(0)} us after this one did`)

  // A gateway's answer arrives from the edge of range, 2.5 dB under the noise.
  chip.hear(Buffer.from('ack'), -109, -2.5)
  const heard = await radio.receive(1_000_000)
  if (heard.outcome === 'Frame') {
    console.log(
      `received  ${heard.payload!.toString()} at ${heard.rssiDbm!.toFixed(2)} dBm, ` +
        `SNR ${heard.snrDb!.toFixed(2)} dB`,
    )
  }

  // With nothing on the air the reception times out, and a frame whose CRC fails is dropped
  // rather than handed over.
  const quiet = (await radio.receive(1_000_000)).outcome
  chip.hearCorrupt(-121, -12)
  const broken = (await radio.receive(1_000_000)).outcome
  console.log(`then      ${quiet}, then ${broken}`)
  radio.close()
  return { power, tuned, airtime, link, reading, guard, chip, quiet, broken }
}
// ANCHOR_END: example

function checkTheBench(bench: Awaited<ReturnType<typeof onTheBench>>): void {
  assert.equal(bench.power.settingDbm, 14)
  assert.equal(bench.tuned.frequencyHz, 868_100_000)
  assert.equal(bench.chip.sent()[0].payload.toString(), bench.reading.toString())
  assert.equal(bench.guard.waitUs(0), bench.airtime * 100)
  assert.equal(bench.quiet, 'Timeout')
  assert.equal(bench.broken, 'Corrupt')
}

// ANCHOR: rfm95w
import { sx127x } from '@pamoja/radios'

async function anRfm95w() {
  // An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the
  // same 16 dBm ceiling leave it the same 14 dBm.
  const band = planFor(LoraRegion.Eu868)
  const channel = 868_100_000
  const dr3 = band.linkSettings(3)!
  const antenna = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
  const rfm95w = sx127x.txPowerUnderCeiling(
    sx127x.PaOutput.PaBoost,
    antenna,
    band.maxEirpDbm(channel),
  )

  // The same driver calls tune it, through registers this time. Its synthesizer steps in
  // 61 Hz, so the carrier lands on the step nearest the one asked for.
  const chip = SimulatedLoraChip.sx127x({ output: sx127x.PaOutput.PaBoost })
  const radio = chip.radio()
  await radio.configure({ frequencyHz: channel, link: dr3, outputDbm: rfm95w.outputDbm })
  const tuned = chip.tuning()
  const off = Math.abs(channel - tuned.frequencyHz)
  console.log(
    `rfm95w    ${tuned.outputDbm} dBm on PA_BOOST, carrier ${tuned.frequencyHz} Hz, ` +
      `${off} Hz from ${channel}`,
  )

  // The SX1276 gives a packet's strength in whole decibels, and works out the strength of
  // the signal itself from the SNR when it arrived under the noise.
  chip.hear(Buffer.from('ack'), -109, -2.5)
  const heard = await radio.receive(1_000_000)
  if (heard.outcome === 'Frame') {
    console.log(
      `received  RSSI ${heard.rssiDbm!.toFixed(2)} dBm, SNR ${heard.snrDb!.toFixed(2)} dB, ` +
        `signal ${heard.signalRssiDbm!.toFixed(2)} dBm`,
    )
  }
  radio.close()

  // An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
  const carries = (dataRate: number): string =>
    sx126x.llcc68Supports(band.linkSettings(dataRate)!) ? 'carries' : 'cannot carry'
  console.log(`llcc68    ${carries(3)} DR3 and ${carries(2)} DR2`)
  return { rfm95w, tuned, channel, carries }
}
// ANCHOR_END: rfm95w

function checkTheRfm95w(rfm95w: Awaited<ReturnType<typeof anRfm95w>>): void {
  assert.equal(rfm95w.rfm95w.outputDbm, 14)
  assert.equal(rfm95w.tuned.outputDbm, 14)
  assert.ok(Math.abs(rfm95w.channel - rfm95w.tuned.frequencyHz) <= 61)
  assert.equal(rfm95w.carries(3), 'carries')
}

// ANCHOR: hardware
import { LoraRadio } from '@pamoja/radios'

async function onALinuxBoard(): Promise<void> {
  // An RFM95W on a Raspberry Pi: the header's first chip select, with the module's reset pin
  // on GPIO25. The SX1276 family has no BUSY line, so the wiring names none.
  const wiring = { spi: '/dev/spidev0.0', gpioChip: '/dev/gpiochip0', resetLine: 25 }
  console.log(`radio     an RFM95W on ${wiring.spi}, reset on GPIO${wiring.resetLine}`)

  // The channel and the power the same whip leaves under the same ceiling, now as the number
  // the radio is set to rather than the registers it goes into.
  const band = planFor(LoraRegion.Eu868)
  const channel = 868_100_000
  const dr3 = band.linkSettings(3)!
  const antenna = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
  const ceiling = band.maxEirpDbm(channel)
  const rfm95w = sx127x.txPowerUnderCeiling(sx127x.PaOutput.PaBoost, antenna, ceiling)
  console.log(`plan      ${channel} Hz at DR3, ${rfm95w.outputDbm} dBm on PA_BOOST`)

  // Opening resets the chip and reads its version back, so a wiring mistake is caught here
  // rather than on the first frame. With no radio wired, this is the line that prints.
  let radio
  try {
    radio = LoraRadio.openSx127x(wiring, { output: sx127x.PaOutput.PaBoost })
  } catch {
    console.log('absent    no radio answered, so nothing went out')
    return
  }
  try {
    await radio.configure({ frequencyHz: channel, link: dr3, outputDbm: rfm95w.outputDbm })
    const airtimeUs = await radio.transmit(Buffer.from('21.5'))
    console.log(`sent      a reading in ${airtimeUs} us on air`)
  } finally {
    radio.close()
  }
}
// ANCHOR_END: hardware

async function run(): Promise<void> {
  checkTheBench(await onTheBench())
  checkTheRfm95w(await anRfm95w())
  await onALinuxBoard()
}

run().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
