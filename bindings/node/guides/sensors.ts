// The sensor-driver guide example; see docs/guides/sensors.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { ds18b20, ina219 } from '@pamoja/sensors'

// Stand-ins for the two parts. On a running node the thermometer's nine bytes come off
// the 1-Wire bus and the monitor's registers off I2C; here the library builds what each
// would send, so the program runs with nothing plugged in.
const thermometer = ds18b20.buildScratchpad(25.0625, 12, 75, -10)

// The monitor is set up for 1 mA per count across a 2 milliohm shunt, the load its
// datasheet's worked design example describes: 11.98 V, 10 A, and 119.8 W.
const currentLsb = 1_000
const bus = ina219.busRegister(11_980)
const current = ina219.currentRegister(10_000_000, currentLsb)
const power = ina219.powerRegister(119_800_000, currentLsb)

// Everything below is the node's own code. The thermometer checksums every read, so a
// reading is verified before it is believed.
const reading = ds18b20.parseScratchpad(thermometer)
console.log(`temperature  ${reading.celsius.toFixed(4)} C`)
console.log(`resolution   ${reading.resolutionBits} bits`)
console.log(`alarms       ${reading.alarmHigh} / ${reading.alarmLow} C`)

// The monitor computes nothing until it has been told what shunt it is across.
console.log(`calibration  0x${ina219.calibration(currentLsb, 2).toString(16).toUpperCase()}`)
console.log(`bus          ${ina219.busMillivolts(bus)} mV`)
console.log(`current      ${ina219.currentMicroamps(current, currentLsb) / 1_000} mA`)
console.log(`power        ${ina219.powerMicrowatts(power, currentLsb) / 1_000} mW`)

// A bit flipped on a long 1-Wire run fails the checksum, so the node repeats the read
// instead of logging a temperature a couple of degrees off.
const corrupted = Buffer.from(thermometer)
corrupted[0] ^= 1
try {
  ds18b20.parseScratchpad(corrupted)
  console.log('corrupt read accepted, which should never happen')
} catch (error) {
  console.log(`corrupt read rejected: ${(error as Error).message}`)
}
// ANCHOR_END: example

assert.equal(reading.rawTemperature, 0x0191)
assert.equal(reading.microCelsius, 25_062_500)
assert.equal(reading.resolutionBits, 12)
assert.equal(reading.alarmHigh, 75)
assert.equal(reading.alarmLow, -10)

// The datasheet's own figures for that design: calibration 0x5000, and registers that
// read back 11.98 V, 10 A, and 119.8 W.
assert.equal(ina219.calibration(currentLsb, 2), 0x5000)
assert.equal(ina219.busMillivolts(bus), 11_980)
assert.equal(ina219.currentMicroamps(current, currentLsb), 10_000_000)
assert.equal(ina219.powerMicrowatts(power, currentLsb), 119_800_000)

// The published check value for CRC-8/MAXIM-DOW, the checksum every 1-Wire part appends
// to what it sends.
assert.equal(ds18b20.crc8(Buffer.from('123456789')), 0xa1)
