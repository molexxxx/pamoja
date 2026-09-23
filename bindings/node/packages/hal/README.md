# @pamoja/hal

The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, and one I2C bus a program and its drivers share. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/hal.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_hal.html)

## Install

```sh
npm install @pamoja/hal
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/hal.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/hal.ts):

```typescript
import { I2cBus, I2cStep } from '@pamoja/hal'
import { Bme280, type Bme280Measurement, bme280 } from '@pamoja/sensors'

const BME280 = bme280.addressPrimary
const x = (code: number): string => `x${bme280.oversamplingFactor(code)}`
const shown = (reading: Bme280Measurement): string =>
  `${reading.celsius.toFixed(2)} C, ${reading.hectopascals.toFixed(2)} hPa, ` +
  `${reading.relativeHumidityPercent.toFixed(2)} %`

async function main() {
  // A bus with one part on it: a BME280 that is not there. It holds a real part's
  // calibration and one measurement that part took, and it answers from its registers, so
  // the driver runs its whole datasheet sequence against it. On a Raspberry Pi the bus is
  // I2cBus.open('/dev/i2c-1') and nothing after this line changes.
  const bus = I2cBus.simulated([bme280.sim.part(BME280)])
  const sensor = new Bme280(bus, BME280)

  // Reset, identify, calibrate, configure. The datasheet wants ctrl_hum written before
  // ctrl_meas, and the part left asleep until a measurement is forced. The part keeps what
  // the driver wrote, so the configuration reads back off the bus.
  await sensor.init()
  const part = bus.part(BME280)!
  const humidity = bme280.ctrlHumFromBits(part.register(bme280.register.ctrlHum))
  const ctrl = bme280.ctrlMeasFromBits(part.register(bme280.register.ctrlMeas))
  const asleep = ctrl.mode === bme280.mode.sleep
  console.log(
    `configured   humidity ${x(humidity)}, temperature ${x(ctrl.temperature)}, ` +
      `pressure ${x(ctrl.pressure)}, asleep: ${asleep}`,
  )

  // One forced measurement. The driver waits the datasheet's longest measurement time for
  // these settings before it reads, and a simulated bus counts that wait rather than
  // sleeping through it.
  const reading = await sensor.measure()
  console.log(`measured     ${shown(reading)}`)
  const waitedMs = (bus.waitedMicros / 1000).toFixed(2)
  console.log(`waited       ${waitedMs} ms across ${bus.transfers} transfers`)

  // A part reports whatever it is asked to. Putting one in the first one's place is how a
  // program meets a reading it would otherwise wait on the weather for, here a cold store
  // at four degrees, and the driver carries on without noticing.
  bus.attach(bme280.sim.reporting(BME280, 4.0, 1013.25, 80.0))
  const cold = await sensor.measure()
  console.log(`cold store   ${shown(cold)}`)

  // Nothing answers at the part's other address, and the driver says so rather than
  // returning a reading.
  try {
    await new Bme280(bus, bme280.addressSecondary).init()
    console.log('absent       a part answered')
  } catch (error) {
    console.log(`absent       ${(error as Error).message}`)
  }

  // The other half of the bus layer. A script plays one conversation and refuses anything
  // else, which proves a driver follows the datasheet rather than merely working: the
  // reset, the status once the calibration has loaded, the chip id, the two calibration
  // blocks, the three configuration writes in the order the part requires, then one forced
  // measurement.
  const { register, oversampling, mode, sim } = bme280
  const settings = { temperature: oversampling.x1, pressure: oversampling.x1, mode: mode.sleep }
  const forced = { ...settings, mode: mode.forced }
  const resetState = { standby: bme280.standby.ms0_5, filter: bme280.filter.off, spi3Wire: false }
  const script = I2cBus.scripted([
    I2cStep.write(BME280, Buffer.from([register.reset, bme280.resetWord])),
    I2cStep.writeRead(BME280, Buffer.from([register.status]), Buffer.from([sim.statusIdle])),
    I2cStep.writeRead(BME280, Buffer.from([register.chipId]), Buffer.from([bme280.chipId])),
    I2cStep.writeRead(BME280, Buffer.from([register.calibTempPress]), sim.calibration()),
    I2cStep.writeRead(BME280, Buffer.from([register.calibHumidity]), sim.calibrationHumidity()),
    I2cStep.write(BME280, Buffer.from([register.config, bme280.configBits(resetState)])),
    I2cStep.write(BME280, Buffer.from([register.ctrlHum, bme280.ctrlHumBits(oversampling.x1)])),
    I2cStep.write(BME280, Buffer.from([register.ctrlMeas, bme280.ctrlMeasBits(settings)])),
    I2cStep.write(BME280, Buffer.from([register.ctrlMeas, bme280.ctrlMeasBits(forced)])),
    I2cStep.writeRead(BME280, Buffer.from([register.status]), Buffer.from([sim.statusIdle])),
    I2cStep.writeRead(BME280, Buffer.from([register.data]), sim.burst()),
  ])
  const checked = await new Bme280(script, BME280).measure()
  const left = script.remaining ?? 0
  console.log(
    `datasheet    ${checked.celsius.toFixed(2)} C after ${script.transfers} transfers, ` +
      `${left} steps left`,
  )

  return { humidity, asleep, reading, cold, waited: bus.waitedMicros, checked, script }
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-hal`](https://crates.io/crates/pamoja-hal) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_hal/index.html), [docs.rs](https://docs.rs/pamoja-hal), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-hal) |
| TypeScript | [`@pamoja/hal`](https://www.npmjs.com/package/@pamoja/hal) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_hal.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-hal) |
| Python | [`pamoja-hal`](https://pypi.org/project/pamoja-hal/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/hal.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-hal) |
| C# | [`Pamoja.Hal`](https://www.nuget.org/packages/Pamoja.Hal) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Hal.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-hal) |

## Documentation

- [`@pamoja/hal` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_hal.html), every class, function, and type this package exports.
- [The Buses guide](https://pamoja.molex.cloud/docs/guides/hal.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
