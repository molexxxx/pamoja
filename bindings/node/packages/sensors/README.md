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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sensors`](https://crates.io/crates/pamoja-sensors) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html), [docs.rs](https://docs.rs/pamoja-sensors) |
| TypeScript | [`@pamoja/sensors`](https://www.npmjs.com/package/@pamoja/sensors) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html) |
| Python | [`pamoja-sensors`](https://pypi.org/project/pamoja-sensors/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html) |
| C# | [`Pamoja.Sensors`](https://www.nuget.org/packages/Pamoja.Sensors) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html) |

## Documentation

- [`@pamoja/sensors` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html), every class, function, and type this package exports.
- [The Sensor drivers guide](https://pamoja.molex.cloud/docs/guides/sensors.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
