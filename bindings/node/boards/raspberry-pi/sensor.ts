// The first program on a Raspberry Pi: a BME280 on the 40-pin header's I2C bus, read through
// the driver pamoja ships, printed every two seconds. Wire the BME280's SDA to GPIO2, its SCL
// to GPIO3, VIN to 3V3, and GND to ground, and turn the I2C interface on. See
// docs/boards/raspberry-pi.md.

// ANCHOR: example
import { setTimeout as sleep } from 'node:timers/promises'
import { I2cBus } from '@pamoja/hal'
import { Bme280, bme280 } from '@pamoja/sensors'

async function main(): Promise<void> {
  // The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
  const bus = I2cBus.open('/dev/i2c-1')

  // The driver runs the datasheet's sequence over that bus: reset, identify, read the
  // calibration, configure, and then a forced measurement per read.
  const sensor = new Bme280(bus, bme280.addressPrimary)
  await sensor.init()

  for (;;) {
    const reading = await sensor.measure()
    console.log(
      `${reading.celsius.toFixed(2)} C, ${reading.hectopascals.toFixed(2)} hPa, ` +
        `${reading.relativeHumidityPercent.toFixed(2)} % humidity`,
    )
    await sleep(2000)
  }
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
// ANCHOR_END: example
