// The sensor-driver guide example; see docs/guides/sensors.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { ds18b20, ina219 } from '@pamoja/sensors'

// Every Maxim 1-Wire part checks itself with CRC-8/MAXIM-DOW, whose published check value
// over the ASCII digits 1 to 9 is 0xA1.
assert.equal(ds18b20.crc8(Buffer.from('123456789')), 0xa1)

// A DS18B20 answers a read with nine scratchpad bytes, the ninth that CRC over the other
// eight, so a reading is verified before it is believed.
const scratchpad = Buffer.from([0x91, 0x01, 0x4b, 0xf6, 0x7f, 0xff, 0x0c, 0x10, 0x00])
scratchpad[8] = ds18b20.crc8(scratchpad.subarray(0, 8))
const reading = ds18b20.parseScratchpad(scratchpad)

// Register 0x0191 is the +25.0625 degree row of the datasheet's temperature table, each
// count a sixteenth of a degree, so micro-degrees stay exact in integer arithmetic.
assert.equal(reading.rawTemperature, 0x0191)
assert.equal(reading.microCelsius, 25_062_500)
assert.equal(reading.resolutionBits, 12)
assert.equal(reading.alarmHigh, 75)

// A bit flipped on a long 1-Wire run fails the CRC instead of arriving as a plausible
// temperature a few degrees off.
const corrupt = Buffer.from(scratchpad)
corrupt[0] ^= 0x01
assert.throws(() => ds18b20.parseScratchpad(corrupt))

// The INA219 datasheet's worked design example: 1 mA per count across a 2 milliohm shunt
// calibrates to 0x5000, and its registers then read 11.98 V, 10 A, and 119.8 W.
const currentLsb = 1_000
assert.equal(ina219.calibration(currentLsb, 2), 0x5000)
assert.equal(ina219.busMillivolts(0x5d98), 11_980)
assert.equal(ina219.currentMicroamps(0x2710, currentLsb), 10_000_000)
assert.equal(ina219.powerMicrowatts(0x1766, currentLsb), 119_800_000)
// ANCHOR_END: example
