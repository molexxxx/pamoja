# Sensor drivers

A sensor does not report what it measures. It reports register bytes, and the
reading only appears after the conversion its datasheet specifies: Bosch's
compensation polynomials over a per-chip calibration for a BME280, a
two's-complement register worth a sixteenth of a degree for a DS18B20, a
calibration value the INA219 needs before it computes current at all, a
full-scale range for the ADS1115. pamoja carries that per-part arithmetic and
none of the wiring. Driving the bus stays the caller's job, so the same decode
runs on a microcontroller, on a gateway, and in a test with nothing plugged in.

## What the example does

It reads a DS18B20 thermometer and an INA219 power monitor, the two parts a
battery node usually has on it. The thermometer's scratchpad is checked against
the CRC the part appends before any temperature is taken from it, and the
monitor's registers are the ones its datasheet's worked design example fixes.

It proves:

- The 1-Wire checksum is CRC-8/MAXIM-DOW, which over the ASCII digits 1 to 9
  produces the published check value `0xA1`.
- Register `0x0191` is +25.0625 degrees Celsius, the row the DS18B20
  temperature table publishes, decoded exactly in integer arithmetic rather
  than through a float.
- The same scratchpad reports the resolution its configuration byte selects and
  the alarm thresholds written into it.
- One flipped bit fails the CRC, so a read corrupted on a long 1-Wire run is
  repeated instead of logged as a temperature a couple of degrees off.
- The INA219 lands on the datasheet's own numbers: calibration `0x5000` for 1 mA
  per count across a 2 milliohm shunt, and 11.98 V, 10 A, and 119.8 W out of the
  registers that design produces.

## Rust

<!-- snippet: examples/tests/guides/sensors.rs#example -->
From [`examples/tests/guides/sensors.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/sensors.rs):

```rust
use pamoja_sensors::ds18b20::{self, Scratchpad};
use pamoja_sensors::ina219;

// Every Maxim 1-Wire part checks itself with CRC-8/MAXIM-DOW, whose published check
// value over the ASCII digits 1 to 9 is 0xA1.
assert_eq!(ds18b20::crc8(b"123456789"), 0xA1);

// A DS18B20 answers a read with nine scratchpad bytes, the ninth that CRC over the
// other eight, so a reading is verified before it is believed.
let mut scratchpad = [0x91, 0x01, 0x4B, 0xF6, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
scratchpad[8] = ds18b20::crc8(&scratchpad[..8]);
let reading = Scratchpad::parse(&scratchpad).expect("the CRC matches");

// Register 0x0191 is the +25.0625 degree row of the datasheet's temperature table, each
// count a sixteenth of a degree, so micro-degrees stay exact in integer arithmetic.
assert_eq!(reading.raw_temperature(), 0x0191);
assert_eq!(reading.temperature_micro_celsius(), 25_062_500);
assert_eq!(reading.resolution().bits(), 12);
assert_eq!(reading.alarm_high(), 75);

// A bit flipped on a long 1-Wire run fails the CRC instead of arriving as a plausible
// temperature a few degrees off.
let mut corrupt = scratchpad;
corrupt[0] ^= 0x01;
assert!(Scratchpad::parse(&corrupt).is_err());

// The INA219 datasheet's worked design example: 1 mA per count across a 2 milliohm
// shunt calibrates to 0x5000, and its registers then read 11.98 V, 10 A, and 119.8 W.
const CURRENT_LSB: u32 = 1_000;
assert_eq!(ina219::calibration(CURRENT_LSB, 2), 0x5000);
assert_eq!(ina219::bus_millivolts(0x5D98), 11_980);
assert_eq!(ina219::current_microamps(0x2710, CURRENT_LSB), 10_000_000);
assert_eq!(ina219::power_microwatts(0x1766, CURRENT_LSB), 119_800_000);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/sensors.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/sensors.py#example -->
From [`bindings/python/guides/sensors.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sensors.py):

```python
from pamoja.core import PamojaError
from pamoja.sensors import ds18b20, ina219

# Every Maxim 1-Wire part checks itself with CRC-8/MAXIM-DOW, whose published check
# value over the ASCII digits 1 to 9 is 0xA1.
assert ds18b20.crc8(b"123456789") == 0xA1

# A DS18B20 answers a read with nine scratchpad bytes, the ninth that CRC over the
# other eight, so a reading is verified before it is believed.
scratchpad = bytearray([0x91, 0x01, 0x4B, 0xF6, 0x7F, 0xFF, 0x0C, 0x10, 0x00])
scratchpad[8] = ds18b20.crc8(bytes(scratchpad[:8]))
reading = ds18b20.parse_scratchpad(bytes(scratchpad))

# Register 0x0191 is the +25.0625 degree row of the datasheet's temperature table, each
# count a sixteenth of a degree, so micro-degrees stay exact in integer arithmetic.
assert reading.raw_temperature == 0x0191
assert reading.micro_celsius == 25_062_500
assert reading.resolution_bits == 12
assert reading.alarm_high == 75

# A bit flipped on a long 1-Wire run fails the CRC instead of arriving as a plausible
# temperature a few degrees off.
corrupt = bytearray(scratchpad)
corrupt[0] ^= 0x01
try:
    ds18b20.parse_scratchpad(bytes(corrupt))
except PamojaError:
    pass
else:
    raise AssertionError("a scratchpad corrupted on the bus should be rejected")

# The INA219 datasheet's worked design example: 1 mA per count across a 2 milliohm
# shunt calibrates to 0x5000, and its registers then read 11.98 V, 10 A, and 119.8 W.
current_lsb = 1_000
assert ina219.calibration(current_lsb, 2) == 0x5000
assert ina219.bus_millivolts(0x5D98) == 11_980
assert ina219.current_microamps(0x2710, current_lsb) == 10_000_000
assert ina219.power_microwatts(0x1766, current_lsb) == 119_800_000
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs):

```csharp
// Every Maxim 1-Wire part checks itself with CRC-8/MAXIM-DOW, whose published
// check value over the ASCII digits 1 to 9 is 0xA1.
Expect(Ds18b20.Crc8("123456789"u8) == 0xA1, "the published CRC check value");

// A DS18B20 answers a read with nine scratchpad bytes, the ninth that CRC over
// the other eight, so a reading is verified before it is believed.
byte[] scratchpad = [0x91, 0x01, 0x4B, 0xF6, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
scratchpad[8] = Ds18b20.Crc8(scratchpad.AsSpan(0, 8));
Ds18b20Reading reading = Ds18b20.ParseScratchpad(scratchpad);

// Register 0x0191 is the +25.0625 degree row of the datasheet's temperature
// table, each count a sixteenth of a degree, so micro-degrees stay exact.
Expect(reading.RawTemperature == 0x0191, "the temperature register reads back");
Expect(reading.MicroCelsius == 25_062_500, "the datasheet's temperature row");
Expect(reading.ResolutionBits == 12, "the configuration byte selects 12 bits");
Expect(reading.AlarmHigh == 75, "and the scratchpad carries its alarm threshold");

// A bit flipped on a long 1-Wire run fails the CRC instead of arriving as a
// plausible temperature a few degrees off.
byte[] corrupt = [.. scratchpad];
corrupt[0] ^= 0x01;
bool rejected = false;
try
{
    Ds18b20.ParseScratchpad(corrupt);
}
catch (PamojaException)
{
    rejected = true;
}
Expect(rejected, "a scratchpad corrupted on the bus is rejected");

// The INA219 datasheet's worked design example: 1 mA per count across a 2
// milliohm shunt calibrates to 0x5000, and its registers then read 11.98 V,
// 10 A, and 119.8 W.
const uint currentLsb = 1_000;
Expect(Ina219.Calibration(currentLsb, 2) == 0x5000, "the calibration register");
Expect(Ina219.BusMillivolts(0x5D98) == 11_980, "the bus sits at 11.98 V");
Expect(Ina219.CurrentMicroamps(0x2710, currentLsb) == 10_000_000, "10 A in the shunt");
Expect(Ina219.PowerMicrowatts(0x1766, currentLsb) == 119_800_000, "drawing 119.8 W");
```
<!-- end -->

## Reference

<!-- table: reference sensors -->
- Rust: [`pamoja-sensors`](https://docs.rs/pamoja-sensors) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html))
- TypeScript: [`@pamoja/sensors`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html)
- Python: [`pamoja.sensors`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html)
- C#: [`Bme280`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.Bme280.html), [`Bme280Calibration`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.Bme280Calibration.html), [`Bme280Measurement`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.Bme280Measurement.html), [`Ds18b20`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.Ds18b20.html), [`Ds18b20Reading`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.Ds18b20Reading.html), [`Ina219`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.Ina219.html), [`Ads1115`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.Ads1115.html), [`Ads1115Config`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.Ads1115Config.html)
<!-- end -->
