// The LoRa airtime guide example; see docs/guides/lora.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { LoraRegion, airtimeUs, messagesPerHour, minOffTimeUs, planFor } from '@pamoja/lora'

// EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the setting
// that reaches furthest and holds the channel longest.
const plan = planFor(LoraRegion.Eu868)
const link = plan.linkSettings(0)!
console.log(`${plan.name} DR0 is SF${link.spreadingFactor} at 125 kHz`)

// The time on air for that setting, coding rate 4/5, an eight-symbol preamble, an explicit
// header and CRC on, carrying a ten-byte reading.
const airtime = airtimeUs(link, 10)
console.log(`airtime   ${(airtime / 1e6).toFixed(2)} s for ten bytes`)

// 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every transmission
// buys ninety-nine times its own length in silence.
const channel = 868_100_000
const permille = plan.dutyCyclePermille(channel)!
console.log(`channel   ${permille} per mille duty cycle, ${plan.maxEirpDbm(channel)} dBm`)

const offTime = minOffTimeUs(link, 10, permille)!
console.log(`silence   ${(offTime / 1e6).toFixed(1)} s owed after each reading`)

// The airtime plus that silence is what one reading really costs, which is the budget a
// deployment plans against.
console.log(`budget    ${messagesPerHour(link, 10, permille)} readings an hour`)

// A frequency in no sub-band the plan describes has no duty cycle to budget against. That
// is a limit published elsewhere, not permission to transmit.
const outside = plan.dutyCyclePermille(700_000_000)
console.log(`700 MHz  is outside this plan, so it budgets nothing: ${outside === null}`)
// ANCHOR_END: example

assert.equal(plan.name, 'EU863-870')
assert.equal(link.spreadingFactor, 12)
assert.equal(airtime, 991_232)
assert.equal(permille, 10)
assert.equal(plan.maxEirpDbm(channel), 16)
assert.equal(offTime, airtime * 99)
assert.equal(messagesPerHour(link, 10, permille), 36)
assert.equal(outside, null)

// ANCHOR: range
import {
  GATEWAY_NOISE_FIGURE_DB,
  eirpDbm,
  fccMaxConductedDbm,
  freeSpaceLossDb,
  fresnelRadiusMm,
  linkBudget,
  marginDb,
  maxPathLossDb,
  maxTransmitPowerDbm,
  sensitivityDbm,
} from '@pamoja/lora'

const eu868 = planFor(LoraRegion.Eu868)
const dr0 = eu868.linkSettings(0)!
const frequency = 868_100_000

// A node with a 2.15 dBi whip on half a decibel of pigtail, heard by a gateway with a
// 6 dBi collinear antenna behind 1.5 dB of cable and a 3 dB noise figure.
const whip = { transmitAntennaGainDbi: 2.15, transmitCableLossDb: 0.5 }
const gateway = {
  receiveAntennaGainDbi: 6,
  receiveCableLossDb: 1.5,
  noiseFigureDb: GATEWAY_NOISE_FIGURE_DB,
}

// The plan caps what leaves the antenna, so the antenna and cable decide how hard the
// radio may drive. A radio takes whole decibels, so the setting rounds down.
const ceiling = eu868.maxEirpDbm(frequency)
const most = maxTransmitPowerDbm(linkBudget(whip), ceiling)
const node = linkBudget({ ...whip, ...gateway, transmitPowerDbm: Math.floor(most) })
console.log(`radio     ${most.toFixed(2)} dBm allowed, set to ${node.transmitPowerDbm} dBm`)
console.log(`eirp      ${eirpDbm(node).toFixed(2)} dBm under a ${ceiling} dBm ceiling`)

// The weakest signal the gateway still hears at SF12 and 125 kHz, and so the most path
// loss the link survives.
const sensitivity = sensitivityDbm(node, dr0)
const survives = maxPathLossDb(node, dr0)
console.log(
  `gateway   hears down to ${sensitivity.toFixed(2)} dBm, so ${survives.toFixed(2)} dB of path loss`,
)

// Free space at three distances, and what each path leaves to spare.
const margins = [2_000, 5_000, 15_000].map((distanceM) => {
  const loss = freeSpaceLossDb(distanceM, frequency)
  const margin = marginDb(node, dr0, loss)
  const km = String(distanceM / 1_000).padStart(2)
  console.log(`${km} km     ${loss.toFixed(2)} dB lost, ${margin.toFixed(2)} dB to spare`)
  return margin
})

// Free space assumes nothing is in the way. Terrain inside the first Fresnel zone adds
// diffraction loss, which starts once the clearance falls below 60% of its radius.
const radius = fresnelRadiusMm(2_500, 2_500, frequency)
const clear = Math.floor((radius * 6) / 10)
console.log(
  `fresnel   ${(radius / 1000).toFixed(1)} m at the middle of 5 km, keep ${(clear / 1000).toFixed(1)} m clear`,
)

// In the United States, 47 CFR 15.247 caps conducted power instead, and takes off every
// decibel an antenna has over 6 dBi.
const limit = fccMaxConductedDbm(9, 64)!
console.log(`fcc       a 9 dBi Yagi on 64 hopping channels may carry ${limit.toFixed(2)} dBm`)
// ANCHOR_END: range

assert.equal(most, 14.35)
assert.equal(node.transmitPowerDbm, 14)
assert.equal(eirpDbm(node), 15.65)
assert.equal(sensitivity, -140.03)
assert.equal(survives, 160.18)
assert.deepEqual(margins, [62.94, 54.98, 45.44])
assert.equal(radius, 20_777)
assert.equal(limit, 27)
