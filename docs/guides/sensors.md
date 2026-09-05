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
monitor is calibrated for the shunt it sits across, since it computes nothing
until it has been. The shunt, the current resolution and the load are the ones
its datasheet's worked design example describes.

On a running node those bytes arrive from the bus, so there is nothing to type.
The example builds them instead, with the same library that decodes them:
`Scratchpad::new(..).to_bytes()` returns exactly what a thermometer at that
temperature sends, and `bus_register`, `current_register`, and `power_register`
return what a monitor across that load reports. Every builder is the inverse of
the decode beside it, which is what lets a node be written and tested with
nothing wired up. Everything after those first few lines is the node's own code
and does not care where the bytes came from.

It proves:

- 25.0625 degrees Celsius at 12-bit resolution builds register `0x0191`, the
  row the DS18B20 temperature table publishes, and that register decodes back
  to the same temperature, exact in integer micro-degrees.
- The same nine bytes report the resolution the configuration byte selects and
  both alarm thresholds, 75 and -10 degrees, written into them.
- One flipped bit fails the CRC, so a read corrupted on a long 1-Wire run is
  repeated instead of logged as a temperature a couple of degrees off.
- The 1-Wire checksum is CRC-8/MAXIM-DOW, which over the ASCII digits 1 to 9
  produces the published check value `0xA1`.
- 1 mA per count across a 2 milliohm shunt calibrates to `0x5000`, the number
  the INA219 datasheet's design example works out, and the registers a monitor
  across that load reports decode back to 11.98 V, 10 A, and 119.8 W.

## Rust

<!-- snippet: examples/tests/guides/sensors.rs#example -->
From [`examples/tests/guides/sensors.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/sensors.rs):

```rust
use pamoja_sensors::ds18b20::{self, Resolution, Scratchpad};
use pamoja_sensors::ina219;

// Stand-ins for the two parts. On a running node the thermometer's nine bytes come
// off the 1-Wire bus and the monitor's registers off I2C; here the library builds
// what each would send, so the program runs with nothing plugged in.
let thermometer = Scratchpad::new(
    ds18b20::temperature_from_celsius(25.0625, Resolution::Bits12),
    Resolution::Bits12,
    75,
    -10,
)
.to_bytes();

// The monitor is set up for 1 mA per count across a 2 milliohm shunt, the load its
// datasheet's worked design example describes: 11.98 V, 10 A, and 119.8 W.
const CURRENT_LSB: u32 = 1_000;
let bus = ina219::bus_register(11_980);
let current = ina219::current_register(10_000_000, CURRENT_LSB);
let power = ina219::power_register(119_800_000, CURRENT_LSB);

// Everything below is the node's own code. The thermometer checksums every read, so
// a reading is verified before it is believed.
let reading = Scratchpad::parse(&thermometer).expect("the checksum matches");
let celsius = reading.temperature_celsius();
let bits = reading.resolution().bits();
let (high, low) = (reading.alarm_high(), reading.alarm_low());
println!("temperature  {celsius:.4} C");
println!("resolution   {bits} bits");
println!("alarms       {high} / {low} C");

// The monitor computes nothing until it has been told what shunt it is across.
let calibration = ina219::calibration(CURRENT_LSB, 2);
let millivolts = ina219::bus_millivolts(bus);
let milliamps = ina219::current_microamps(current, CURRENT_LSB) / 1_000;
let milliwatts = ina219::power_microwatts(power, CURRENT_LSB) / 1_000;
println!("calibration  {calibration:#06X}");
println!("bus          {millivolts} mV");
println!("current      {milliamps} mA");
println!("power        {milliwatts} mW");

// A bit flipped on a long 1-Wire run fails the checksum, so the node repeats the read
// instead of logging a temperature a couple of degrees off.
let mut corrupted = thermometer;
corrupted[0] ^= 1;
match Scratchpad::parse(&corrupted) {
    Ok(_) => println!("corrupt read accepted, which should never happen"),
    Err(error) => println!("corrupt read rejected: {error}"),
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/sensors.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/sensors.py#example -->
From [`bindings/python/guides/sensors.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sensors.py):

```python
from pamoja.core import PamojaError
from pamoja.sensors import ds18b20, ina219

# Stand-ins for the two parts. On a running node the thermometer's nine bytes come off
# the 1-Wire bus and the monitor's registers off I2C; here the library builds what each
# would send, so the program runs with nothing plugged in.
thermometer = ds18b20.build_scratchpad(25.0625, 12, 75, -10)

# The monitor is set up for 1 mA per count across a 2 milliohm shunt, the load its
# datasheet's worked design example describes: 11.98 V, 10 A, and 119.8 W.
CURRENT_LSB = 1_000
bus = ina219.bus_register(11_980)
current = ina219.current_register(10_000_000, CURRENT_LSB)
power = ina219.power_register(119_800_000, CURRENT_LSB)

# Everything below is the node's own code. The thermometer checksums every read, so a
# reading is verified before it is believed.
reading = ds18b20.parse_scratchpad(thermometer)
print(f"temperature  {reading.celsius:.4f} C")
print(f"resolution   {reading.resolution_bits} bits")
print(f"alarms       {reading.alarm_high} / {reading.alarm_low} C")

# The monitor computes nothing until it has been told what shunt it is across.
print(f"calibration  0x{ina219.calibration(CURRENT_LSB, 2):04X}")
print(f"bus          {ina219.bus_millivolts(bus)} mV")
print(f"current      {ina219.current_microamps(current, CURRENT_LSB) // 1_000} mA")
print(f"power        {ina219.power_microwatts(power, CURRENT_LSB) // 1_000} mW")

# A bit flipped on a long 1-Wire run fails the checksum, so the node repeats the read
# instead of logging a temperature a couple of degrees off.
corrupted = bytearray(thermometer)
corrupted[0] ^= 1
try:
    ds18b20.parse_scratchpad(bytes(corrupted))
    print("corrupt read accepted, which should never happen")
except PamojaError as error:
    print(f"corrupt read rejected: {error}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs):

```csharp
// Stand-ins for the two parts. On a running node the thermometer's nine bytes
// come off the 1-Wire bus and the monitor's registers off I2C; here the library
// builds what each would send, so the program runs with nothing plugged in.
byte[] thermometer = Ds18b20.BuildScratchpad(25.0625f, 12, 75, -10);

// The monitor is set up for 1 mA per count across a 2 milliohm shunt, the load
// its datasheet's worked design example describes: 11.98 V, 10 A, and 119.8 W.
const uint CurrentLsb = 1_000;
ushort bus = Ina219.BusRegister(11_980);
short current = Ina219.CurrentRegister(10_000_000, CurrentLsb);
ushort power = Ina219.PowerRegister(119_800_000, CurrentLsb);

// Everything below is the node's own code. The thermometer checksums every read,
// so a reading is verified before it is believed.
Ds18b20Reading reading = Ds18b20.ParseScratchpad(thermometer);
Console.WriteLine($"temperature  {reading.Celsius:F4} C");
Console.WriteLine($"resolution   {reading.ResolutionBits} bits");
Console.WriteLine($"alarms       {reading.AlarmHigh} / {reading.AlarmLow} C");

// The monitor computes nothing until it has been told what shunt it is across.
Console.WriteLine($"calibration  0x{Ina219.Calibration(CurrentLsb, 2):X4}");
Console.WriteLine($"bus          {Ina219.BusMillivolts(bus)} mV");
Console.WriteLine($"current      {Ina219.CurrentMicroamps(current, CurrentLsb) / 1_000} mA");
Console.WriteLine($"power        {Ina219.PowerMicrowatts(power, CurrentLsb) / 1_000} mW");

// A bit flipped on a long 1-Wire run fails the checksum, so the node repeats the
// read instead of logging a temperature a couple of degrees off.
byte[] corrupted = [.. thermometer];
corrupted[0] ^= 1;
try
{
    Ds18b20.ParseScratchpad(corrupted);
    Console.WriteLine("corrupt read accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"corrupt read rejected: {error.Message}");
}
```
<!-- end -->

## Reference

<!-- table: reference sensors -->
- Rust: [`pamoja-sensors`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html)
- TypeScript: [`@pamoja/sensors`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html)
- Python: [`pamoja.sensors`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html)
- C#: [`Pamoja.Sensors`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html)
<!-- end -->
