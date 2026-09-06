# @pamoja/sensors

Datasheet-anchored decoders for the BME280, DS18B20, INA219, and ADS1115. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sensors.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/sensors
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/sensors.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sensors.ts):

```typescript
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
