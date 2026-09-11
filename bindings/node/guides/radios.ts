// The LoRa radio guide example; see docs/guides/radios.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { LoraRegion, airtimeUs, linkBudget, planFor } from '@pamoja/lora'
import { DutyCycle, sx126x } from '@pamoja/radios'

const hex = (bytes: Uint8Array): string =>
  Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join(' ')

// An SX1262 node sends a ten-byte reading on 868.1 MHz at DR3, SF9 at 125 kHz, through a
// 2.15 dBi whip on half a decibel of pigtail. The plan caps the EIRP there, and the antenna
// and pigtail decide how hard the amplifier may drive under that cap.
const eu868 = planFor(LoraRegion.Eu868)
const frequency = 868_100_000
const link = eu868.linkSettings(3)!
const whip = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
const ceiling = eu868.maxEirpDbm(frequency)
const power = sx126x.txPowerUnderCeiling(sx126x.Amplifier.HighPower, whip, ceiling)
console.log(`power     ${power.settingDbm} dBm under a ${ceiling} dBm EIRP ceiling`)

// The commands in the order section 14.2 of the datasheet gives, each sent in its own SPI
// transaction once BUSY is low. The chip gives up on the frame a second after its airtime.
const airtime = airtimeUs(link, 10)
const events = sx126x.Irq.TxDone | sx126x.Irq.Timeout
const commands: [string, Uint8Array][] = [
  ['standby', sx126x.setStandby()],
  ['packet type', sx126x.setPacketTypeLora()],
  ['frequency', sx126x.setRfFrequency(frequency)],
  ['pa config', sx126x.setPaConfig(power)],
  ['tx params', sx126x.setTxParams(power, 40)],
  ['modulation', sx126x.setLoraModulationParams(link)],
  ['packet', sx126x.setLoraPacketParams(link, 10, false)],
  ['irq', sx126x.setDioIrqParams(events, events)],
  ['tx', sx126x.setTx(airtime + 1_000_000)],
]
for (const [name, bytes] of commands) {
  console.log(`${name.padEnd(12)}${hex(bytes)}`)
}

// Once the frame has left, GetIrqStatus answers with TxDone, and the status byte shows the
// chip back in standby.
const irq = sx126x.irq(Buffer.from([0x00, 0x01]))
const sent = (irq & sx126x.Irq.TxDone) !== 0
const timedOut = (irq & sx126x.Irq.Timeout) !== 0
console.log(`sent      tx done ${sent}, timed out ${timedOut}`)
const status = sx126x.status(0x2c)
console.log(`status    ${status.chipMode}, ${status.commandStatus}`)

// A frame that arrives later comes with the signal levels it was heard at.
const heard = sx126x.packetStatus(Buffer.from([0xdb, 0xf6, 0xe0]))
console.log(`received  RSSI ${heard.rssiDbm} dBm, SNR ${heard.snrDb} dB`)

// The sub-band that holds 868.1 MHz allows 1% of the time, so the frame's airtime buys
// ninety-nine times as long in silence before the next.
const guard = new DutyCycle(eu868.dutyCyclePermille(frequency)!)
const held = guard.transmitted(0, link, 10)
console.log(`airtime   ${held} us, next frame after ${guard.waitUs(0)} us`)
// ANCHOR_END: example

// The bytes each command carries are pinned once, in the crate tests and the generated
// conformance vectors, so a guide asserts behavior instead.
assert.equal(power.settingDbm, 14)
assert.equal(commands.length, 9)
assert.equal(sent, true)
assert.equal(timedOut, false)
assert.equal(held, airtime)
assert.equal(guard.ready(0), false)
assert.equal(guard.ready(held * 100), true)

// ANCHOR: rfm95w
import { sx127x } from '@pamoja/radios'

// An RFM95W wires the SX1276's PA_BOOST amplifier to its antenna. The same whip and the same
// 16 dBm ceiling leave it the same 14 dBm, set through three registers.
const band = planFor(LoraRegion.Eu868)
const channel = 868_100_000
const dr3 = band.linkSettings(3)!
const antenna = linkBudget({ transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 })
const rfm95w = sx127x.txPowerUnderCeiling(sx127x.PaOutput.PaBoost, antenna, band.maxEirpDbm(channel))
const byte = (value: number): string => value.toString(16).padStart(2, '0')
console.log(
  `rfm95w    ${rfm95w.outputDbm} dBm on PA_BOOST: RegPaConfig ${byte(rfm95w.paConfig)}, ` +
    `RegPaDac ${byte(rfm95w.paDac)}, RegOcp ${byte(rfm95w.ocp)}`,
)

// The carrier and the modem go into registers while the chip stands by, and TX mode sends
// the frame the FIFO holds.
const modem = sx127x.modem(dr3, channel)
console.log(`carrier   RegFrf ${sx127x.frequencyWord(channel).toString(16).padStart(6, '0')}`)
console.log(
  `modem     RegModemConfig ${byte(modem.modemConfig1)} ${byte(modem.modemConfig2)} ` +
    `${byte(modem.modemConfig3)}`,
)
console.log(`tx mode   RegOpMode ${byte(sx127x.loraOpMode(sx127x.Mode.Tx))}`)

// A packet that arrives raises RxDone and ValidHeader, and the SNR and RSSI registers give
// its levels on the high frequency port.
const flags = 0x50
const received = (flags & sx127x.Irq.RxDone) !== 0
const corrupt = (flags & sx127x.Irq.PayloadCrcError) !== 0
console.log(`irq       rx done ${received}, crc error ${corrupt}`)
const packet = sx127x.packetStatus(Buffer.from([0xf6, 0x30]), channel)
console.log(
  `received  RSSI ${packet.rssiDbm} dBm, SNR ${packet.snrDb} dB, signal ${packet.signalRssiDbm} dBm`,
)

// An LLCC68 in the RFM95W's place could carry DR3, but not DR2, which is SF10 at 125 kHz.
const fits = (dataRate: number): boolean => sx126x.llcc68Supports(band.linkSettings(dataRate)!)
console.log(`llcc68    DR3 ${fits(3)}, DR2 ${fits(2)}`)
// ANCHOR_END: rfm95w

assert.equal(rfm95w.outputDbm, 14)
assert.equal(rfm95w.paConfig, 0xfc)
assert.equal(modem.modemConfig2, 0x94)
assert.equal(received && !corrupt, true)
assert.equal(fits(3) && !fits(2), true)
