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
