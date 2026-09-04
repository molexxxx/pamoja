// The LoRa airtime guide example; see docs/guides/lora.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { LoraRegion, airtimeUs, messagesPerHour, minOffTimeUs, planFor } from '@pamoja/lora'

// EU863-870 numbers its data rates from the slowest. DR0 is SF12 at 125 kHz, the setting
// that reaches furthest and holds the channel longest.
const plan = planFor(LoraRegion.Eu868)
assert.equal(plan.name, 'EU863-870')
const link = plan.linkSettings(0)!
assert.equal(link.spreadingFactor, 12)

// The published time on air for SF12 at 125 kHz, coding rate 4/5, an eight-symbol
// preamble, an explicit header and CRC on, carrying ten bytes.
const airtime = airtimeUs(link, 10)
assert.equal(airtime, 991_232)

// 868.1 MHz falls in a sub-band capped at 1% of the time and 16 dBm, so every
// transmission buys ninety-nine times its own length in silence.
const permille = plan.dutyCyclePermille(868_100_000)!
assert.equal(permille, 10)
assert.equal(plan.maxEirpDbm(868_100_000), 16)
assert.equal(minOffTimeUs(link, 10, permille), airtime * 99)

// The airtime plus that silence is what one reading really costs, which is the budget a
// deployment plans against: at SF12, thirty-six readings an hour.
assert.equal(messagesPerHour(link, 10, permille), 36)

// A frequency in no sub-band the plan describes has no duty cycle to budget against. That
// is a limit published elsewhere, not permission to transmit.
assert.equal(plan.dutyCyclePermille(700_000_000), null)
// ANCHOR_END: example
