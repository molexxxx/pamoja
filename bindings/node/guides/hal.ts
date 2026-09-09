// The bus layer: a BME280 read over an I2C bus in the shape of the i2c-bus package a
// Node gateway uses, answered by a script of what the datasheet says the part sends.

import assert from 'node:assert/strict'

// ANCHOR: example
import { setTimeout as sleep } from 'node:timers/promises'
import { bme280 } from '@pamoja/sensors'

const BME280 = bme280.addressPrimary
const hex = (byte: number) => `0x${byte.toString(16).toUpperCase()}`

// A bus with the two calls this conversation needs, in the shape of the i2c-bus
// package: on a gateway `i2c.openSync(1)` gives the real one and nothing below
// changes. This one answers from a script of what a BME280 sends, in the order the
// datasheet lists, and refuses any transfer that is not the next one.
type Step = { register: number; write?: number; reply?: number[] }
class ScriptedBus {
  transfers = 0
  constructor(private readonly script: Step[]) {}

  writeByteSync(address: number, register: number, value: number): void {
    const step = this.next(address, register)
    if (step.write !== value) throw new Error(`unexpected write of ${hex(value)}`)
  }

  readI2cBlockSync(address: number, register: number, length: number, buffer: Buffer): void {
    const step = this.next(address, register)
    if (step.reply?.length !== length) throw new Error(`unexpected read of ${length} bytes`)
    Buffer.from(step.reply).copy(buffer)
  }

  get done(): boolean {
    return this.transfers === this.script.length
  }

  private next(address: number, register: number): Step {
    const step = this.script[this.transfers]
    if (address !== BME280 || step?.register !== register) {
      throw new Error(`unexpected transfer at register ${hex(register)}`)
    }
    this.transfers += 1
    return step
  }
}

const CALIBRATION_A = [
  0x45, 0x6f, 0x6f, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6a, 0xd6, 0xd0, 0x0b, 0x4e, 0x1e, 0x88,
  0xff, 0xf9, 0xff, 0xac, 0x26, 0x0a, 0xd8, 0xbd, 0x10, 0x00, 0x4b,
]
const CALIBRATION_B = [0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1e]
const BURST = [0x65, 0x5a, 0xc0, 0x7e, 0xed, 0x00, 0x75, 0x30]

async function main() {
  const bus = new ScriptedBus([
    { register: 0xe0, write: 0xb6 },
    { register: 0xf3, reply: [0x00] },
    { register: 0xd0, reply: [bme280.chipId] },
    { register: 0x88, reply: CALIBRATION_A },
    { register: 0xe1, reply: CALIBRATION_B },
    { register: 0xf5, write: 0x00 },
    { register: 0xf2, write: 0x01 },
    { register: 0xf4, write: 0x24 },
    { register: 0xf4, write: 0x25 },
    { register: 0xf3, reply: [0x00] },
    { register: 0xf7, reply: BURST },
  ])

  // The datasheet's start-up: the soft reset word, its 2 ms start-up time, and the
  // status register, whose low bit clears once the calibration image has loaded.
  bus.writeByteSync(BME280, 0xe0, 0xb6)
  await sleep(2)
  const status = Buffer.alloc(1)
  bus.readI2cBlockSync(BME280, 0xf3, 1, status)

  // The chip id says it is a BME280, and the two calibration blocks are read once.
  const id = Buffer.alloc(1)
  bus.readI2cBlockSync(BME280, 0xd0, 1, id)
  const tempPress = Buffer.alloc(26)
  const humidity = Buffer.alloc(7)
  bus.readI2cBlockSync(BME280, 0x88, 26, tempPress)
  bus.readI2cBlockSync(BME280, 0xe1, 7, humidity)
  const calibration = bme280.calibration(tempPress, humidity)
  console.log(`calibration  read once: ${id[0] === bme280.chipId}`)

  // config, ctrl_hum, then ctrl_meas, in that order because ctrl_hum only takes effect
  // after the ctrl_meas write: every measurement at oversampling x1, the part asleep.
  bus.writeByteSync(BME280, 0xf5, 0x00)
  bus.writeByteSync(BME280, 0xf2, 0x01)
  bus.writeByteSync(BME280, 0xf4, 0x24)

  // One forced measurement: the mode bits, the datasheet's 9.3 ms maximum for these
  // settings, the status read that confirms the part is idle, and the burst read.
  bus.writeByteSync(BME280, 0xf4, 0x25)
  await sleep(10)
  bus.readI2cBlockSync(BME280, 0xf3, 1, status)
  const burst = Buffer.alloc(8)
  bus.readI2cBlockSync(BME280, 0xf7, 8, burst)
  const measurement = calibration.compensate(burst)
  const celsius = measurement.celsius.toFixed(2)
  const hectopascals = measurement.hectopascals.toFixed(2)
  const humidityPercent = measurement.relativeHumidityPercent.toFixed(2)
  console.log(`measured     ${celsius} C, ${hectopascals} hPa, ${humidityPercent} %`)

  // The script is spent: every transfer the datasheet lists was made, and no other.
  console.log(`bus          ${bus.transfers} transfers, unexpected: ${!bus.done}`)
  return { measurement, transfers: bus.transfers, done: bus.done }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: Awaited<ReturnType<typeof main>>): void {
  assert.equal(seen.measurement.celsius.toFixed(2), '20.44')
  assert.equal(seen.measurement.pascals, 84805)
  assert.equal(seen.measurement.relativeHumidityPercent.toFixed(2), '44.65')
  assert.equal(seen.transfers, 11)
  assert.ok(seen.done)
}
