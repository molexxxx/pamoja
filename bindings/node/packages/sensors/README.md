# @pamoja/sensors

Datasheet-anchored drivers for eleven parts, from every language: the BME280, BMP280, DS18B20, HDC1080, INA219, INA226, ADS1115, OPT3001, SCD4x, SHT3x, and TMP117. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sensors.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html)

## Install

```sh
npm install @pamoja/sensors
```

This pulls in `@pamoja/native`, the compiled engine, and `@pamoja/hal`. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/sensors.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sensors.ts):

```typescript
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { I2cBus } from '@pamoja/hal'
import {
  Ads1115,
  Bmp280,
  Ds18b20Thermometer,
  Hdc1080,
  Ina219,
  Ina226,
  Opt3001,
  Scd4x,
  Sht3x,
  Tmp117,
  ads1115,
  bmp280,
  ds18b20,
  hdc1080,
  ina219,
  ina226,
  opt3001,
  scd4x,
  sht3x,
  tmp117,
} from '@pamoja/sensors'

// The address plan. Every part answers at the address its pins choose, and two parts on one
// address garble each other, so a bench of nine is planned around the two that cannot move:
// the HDC1080 and the SCD41 have one address each.
const air = sht3x.addressA
const pressure = bmp280.addressSecondary
const light = opt3001.addressScl
const soil = ads1115.addressSda
const enclosure = tmp117.address.add0Vplus
const panel = ina219.address(ina219.pin.ground, ina219.pin.supply)
const battery = ina226.address(ina226.pin.supply, ina226.pin.supply)

async function main() {
  // The bench with nothing plugged in: each part answers the way its datasheet says, with the
  // reading it is given here, a warm and humid afternoon. On a Raspberry Pi the bus is
  // I2cBus.open('/dev/i2c-1') and nothing after this statement changes.
  const bus = I2cBus.simulated([
    sht3x.sim.reporting(air, 24.1, 62.0),
    bmp280.sim.reporting(pressure, 24.1, 1003.2),
    scd4x.sim.reporting(1_180, 24.1, 62.0),
    opt3001.sim.reporting(light, 4_200),
    ads1115.sim.reporting(soil, ads1115.pga.fsr4_096, 2.35),
    tmp117.sim.reporting(enclosure, 31.25),
    hdc1080.sim.reporting(31.25, 38.0),
    ina219.sim.reporting(panel, 100, 3_200_000, 18_400, 1_250_000),
    ina226.sim.reporting(battery, 2, 20_000_000, 12_800_000, -350_000),
  ])

  // One driver per part. Each holds its own share of the bus, runs its datasheet's whole
  // conversation on the first measurement, and resolves once the part has answered.
  const airNow = await new Sht3x(bus, air).measure()
  console.log(`air          ${airNow.celsius.toFixed(2)} C, ${airNow.relativeHumidity.toFixed(2)} %`)

  const weather = await new Bmp280(bus, pressure).measure()
  console.log(`pressure     ${weather.hectopascals.toFixed(1)} hPa`)

  const co2 = await new Scd4x(bus).measure()
  console.log(`co2          ${co2.co2Ppm} ppm`)

  const sun = await new Opt3001(bus, light).measure()
  console.log(`light        ${sun.lux.toFixed(0)} lux`)

  // A capacitive probe's voltage falls as the soil wets and nears 3 V in dry soil, past the
  // ADS1115's default range of 2.048 V, so the driver is given the 4.096 V range.
  const probe = await new Ads1115(bus, soil, {
    mux: ads1115.mux.ain0Gnd,
    pga: ads1115.pga.fsr4_096,
  }).sample()
  console.log(`soil         ${probe.volts.toFixed(3)} V`)

  const box = await new Tmp117(bus, enclosure).measure()
  const boxAir = await new Hdc1080(bus).measure()
  console.log(`enclosure    ${box.celsius.toFixed(2)} C, ${boxAir.relativeHumidity.toFixed(1)} %`)

  // Two current monitors, each calibrated for its own shunt. The fans and the pump draw more
  // than the panel gives, so the battery makes up the rest and its current reads negative:
  // current through a shunt is signed by its direction.
  const charge = await new Ina219(bus, panel, {
    shuntMilliohms: 100,
    maxMicroamps: 3_200_000,
  }).measure()
  console.log(
    `panel        ${(charge.busMillivolts / 1e3).toFixed(2)} V, ` +
      `${(charge.currentMicroamps / 1e6).toFixed(2)} A, ` +
      `${(charge.powerMicrowatts / 1e6).toFixed(2)} W`,
  )
  const drain = await new Ina226(bus, battery, {
    shuntMilliohms: 2,
    maxMicroamps: 20_000_000,
  }).measure()
  console.log(
    `battery      ${drain.busVolts.toFixed(2)} V, ${drain.currentAmps.toFixed(2)} A, ` +
      `${drain.powerWatts.toFixed(2)} W`,
  )

  // Every driver waited as its datasheet asks, the OPT3001's 800 ms integration and the
  // SCD41's command times most of all. The simulated bus counted the waits and slept none.
  console.log(`waited       ${(bus.waitedMicros / 1e6).toFixed(2)} s across ${bus.transfers} transfers`)

  // The same probe read at the default range. Past 2.048 V the converter pins at its top code,
  // and the sample says so rather than passing the edge of the range off as a reading.
  bus.attach(ads1115.sim.reporting(soil, ads1115.pga.fsr2_048, 2.35))
  const pinned = await new Ads1115(bus, soil, { mux: ads1115.mux.ain0Gnd }).sample()
  console.log(`default gain ${pinned.volts.toFixed(3)} V, clipped: ${pinned.clipped}`)

  // A driver aimed at the wrong address meets whatever answers there. The TMP117 reads its
  // device id before anything else, so pointed at the soil probe's converter it refuses
  // rather than reporting that part's registers as a temperature.
  try {
    await new Tmp117(bus, soil).init()
    console.log('wrong part   accepted, which should never happen')
  } catch (error) {
    console.log(`wrong part   ${(error as Error).message}`)
  }

  // A DS18B20 on a lead into a pot, read the way a Linux board reads one: the kernel's 1-Wire
  // driver serves each probe as a file under /sys/bus/w1/devices. The program writes that
  // file itself, with the text the kernel prints, so it runs anywhere.
  const devices = join(tmpdir(), `pamoja-bench-${process.pid}`)
  const directory = join(devices, `${ds18b20.familyCode.toString(16)}-000005e2fdc3`)
  mkdirSync(directory, { recursive: true })
  const scratchpad = ds18b20.buildScratchpad(19.5, 12, 30, 5)
  writeFileSync(join(directory, 'w1_slave'), ds18b20.w1SlaveText(scratchpad))
  const found = []
  for (const thermometer of Ds18b20Thermometer.discover(devices)) {
    const reading = await thermometer.read()
    console.log(
      `soil probe   ${reading.celsius.toFixed(4)} C, ` +
        `alarms at ${reading.alarmLow} and ${reading.alarmHigh} C`,
    )
    found.push(reading)
  }
  rmSync(devices, { recursive: true })

  return { airNow, weather, co2, sun, probe, box, boxAir, charge, drain, pinned, found }
}

main()
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sensors`](https://crates.io/crates/pamoja-sensors) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html), [docs.rs](https://docs.rs/pamoja-sensors), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sensors) |
| TypeScript | [`@pamoja/sensors`](https://www.npmjs.com/package/@pamoja/sensors) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sensors) |
| Python | [`pamoja-sensors`](https://pypi.org/project/pamoja-sensors/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sensors) |
| C# | [`Pamoja.Sensors`](https://www.nuget.org/packages/Pamoja.Sensors) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sensors) |

## Documentation

- [`@pamoja/sensors` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html), every class, function, and type this package exports.
- [The Sensor drivers guide](https://pamoja.molex.cloud/docs/guides/sensors.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
