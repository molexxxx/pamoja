# Sensor drivers

A sensor does not report what it measures. It reports register bytes, and the reading
only appears after the conversion its datasheet specifies: Bosch's compensation
polynomials over a per-chip calibration for a BMP280, a two's-complement register worth a
sixteenth of a degree for a DS18B20, a calibration word the INA219 and INA226 need before
they compute current at all, an exponent and a mantissa for the OPT3001's lux, a
CRC-checked word for a Sensirion SHT3x or SCD4x. Getting the bytes takes a conversation in
the datasheet's order and at its pace: reset, identify, configure, trigger, wait out the
conversion, read. pamoja carries both halves for every part it supports, and the drivers
run from all four languages over the one I2C bus the [buses guide](hal.md) introduces.

Eleven parts are covered. Temperature and humidity come from the SHT3x, the HDC1080, and
the TMP117, pressure from the BMP280, carbon dioxide from the SCD40 and SCD41, illuminance
from the OPT3001, current and power from the INA219 and INA226, any analog signal through
the ADS1115, and temperature at the end of a long lead from the 1-Wire DS18B20. The
BME280, the eleventh, has the [buses guide](hal.md) to itself. Every I2C part also has a
simulated twin that answers from its registers the way its datasheet says it does, so a
program with nine sensors on it is written, run, and tested with nothing plugged in.

## What the example does

It builds a greenhouse bench: nine I2C parts on one bus, reporting a warm, humid
afternoon, and a waterproof DS18B20 in a pot of soil.

Nine parts on one bus need nine addresses, and a bench is planned from that. The HDC1080
answers only at `0x40` and the SCD41 only at `0x62`, so the rest move around them with
their address pins: the SHT31 keeps `0x44`, the light sensor moves to `0x47` with its ADDR
pin on SCL, the ADS1115 to `0x4A` with ADDR on SDA, the TMP117 to `0x49` with ADD0 on V+,
and the BMP280 to `0x77` with SDO high. The two current monitors take `0x41` and `0x45`
from their A1 and A0 pins, and the example gets those two from the library's `address`
call rather than reading them off the datasheet's table.

Each simulated part is given the reading it should report, and each driver runs its whole
datasheet sequence against it on the first measurement: the SHT31's soft reset, the
BMP280's chip id and calibration block, the SCD41's stop, serial number and start, the
TMP117's device id, the INA219's configuration and calibration. A simulated bus counts
every wait a driver asks for and sleeps through none of them, so the bench reports 1.48 s
of datasheet waiting, most of it the OPT3001's 800 ms integration and the SCD41's 500 ms
stop, and still finishes at once.

Three scenes follow. The soil probe is read again at the ADS1115's default range of
±2.048 V, where its 2.35 V is past full scale, and the sample says it clipped instead of
passing 2.048 V off as a reading. A TMP117 driver is pointed at the ADS1115's address and
refuses it by its device id before reading any temperature. And the DS18B20 is read the
way a Linux board reads one, through the text file the kernel's `w1_therm` driver serves,
which the example writes itself from a scratchpad the library builds.

It proves:

- Nine drivers share one bus, each holding its own handle, and every reading comes back
  as its part was told: 24.10 C and 62 % in the air, 1003.2 hPa, 1180 ppm, 4200 lux, 2.350 V
  from the soil probe, and 31.25 C and 38 % in the enclosure.
- The INA219's address with A1 on ground and A0 on VS+ is `0x41`, and the INA226's with
  both pins on VS is `0x45`, the rows each datasheet's address table gives.
- Each current monitor is calibrated for its own shunt, 100 milliohms and 3.2 A on the
  panel, 2 milliohms and 20 A on the battery. Current through a shunt is signed by its
  direction, so the battery, discharging into the fans and the pump, reads -0.35 A.
- The waits add up to the datasheets' times, 1.48 s over 66 transfers, and a simulated bus
  spends none of them.
- A conversion at the end of the ADS1115's range is flagged as clipped: the top code of
  the datasheet's Table 7-3.
- A driver aimed at the wrong part refuses it by identity rather than reporting that
  part's registers as a reading.
- The kernel's text for a DS18B20, its checksum verdict and millidegrees included, reads
  back to 19.5 C with alarms at 5 and 30 C.
- Underneath the drivers the arithmetic still meets its published anchors: the INA219
  datasheet's worked example calibrates 1 mA per count across 2 milliohms to `0x5000`, and
  the 1-Wire checksum gives CRC-8/MAXIM-DOW's check value, `0xA1`.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example sensors" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example sensors</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- sensors" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- sensors</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/sensors.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/sensors.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- sensors" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- sensors</code></div>
</div>
<!-- end -->

## Rust

In Rust each part is a module of `pamoja-sensors`: its addresses and register codecs, a
driver, and a `sim` module with the part a test puts on a bus. A driver is generic over the
`embedded-hal` traits and takes a bus and a delay by value, so on a host it takes a clone
of `pamoja_hal::bus::I2cBus` and `bus.delay()`, and on a microcontroller the HAL's own
peripheral and timer. Settings are builder methods such as `with_repeatability`,
`with_gain`, and `with_shunt`. Simulated parts come in three shapes, byte registers for the
BMP280, word registers for the TI parts, and commands for the Sensirion parts, and a `Part`
holds any of them, which is why the example builds its bus with
`I2cBus::simulated::<Part>` and `.into()`. Every call returns a `Result` whose error is a
`DriverError`: `Bus` with the bus's own error, `Sensor` with a `SensorError` when the part
answers with something its datasheet rules out, and `Timeout` when it never finishes. The
DS18B20 behind the kernel is `ds18b20::linux::Thermometer`, from the crate's `linux`
feature.

<!-- snippet: examples/guides/sensors.rs#example -->
From [`examples/guides/sensors.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/sensors.rs):

```rust
use pamoja_hal::bus::I2cBus;
use pamoja_hal::sim::Part;
use pamoja_sensors::ds18b20::{self, linux::Thermometer, Resolution, Scratchpad};
use pamoja_sensors::ina226::AddressPin;
use pamoja_sensors::{ads1115, bmp280, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117};

// The address plan. Every part answers at the address its pins choose, and two parts on
// one address garble each other, so a bench of nine is planned around the two that
// cannot move: the HDC1080 and the SCD41 have one address each.
let air = sht3x::I2C_ADDRESS_A;
let pressure = bmp280::I2C_ADDRESS_SECONDARY;
let light = opt3001::I2C_ADDRESS_SCL;
let soil = ads1115::address::SDA;
let enclosure = tmp117::address::ADD0_VPLUS;
let panel = ina219::address(AddressPin::Ground, AddressPin::Supply);
let battery = ina226::address(AddressPin::Supply, AddressPin::Supply);

// The bench with nothing plugged in: each part answers the way its datasheet says, with
// the reading it is given here, a warm and humid afternoon. A bus holds parts of three
// kinds, and a `Part` is any of them. On a Raspberry Pi the bus is
// `I2cBus::open("/dev/i2c-1")` and nothing after this statement changes.
let bus = I2cBus::simulated::<Part>([
    sht3x::sim::reporting(air, 24.1, 62.0).into(),
    bmp280::sim::reporting(pressure, 24.1, 1003.2).into(),
    scd4x::sim::reporting(1_180, 24.1, 62.0).into(),
    opt3001::sim::reporting(light, 4_200.0).into(),
    ads1115::sim::reporting(soil, ads1115::Pga::Fsr4_096, 2.35).into(),
    tmp117::sim::reporting(enclosure, 31.25).into(),
    hdc1080::sim::reporting(31.25, 38.0).into(),
    ina219::sim::reporting(panel, 100, 3_200_000, 18_400, 1_250_000).into(),
    ina226::sim::reporting(battery, 2, 20_000_000, 12_800_000, -350_000).into(),
]);

// One driver per part. Each holds its own share of the bus and a delay that sleeps only
// when a real part is on the other end, and each runs its datasheet's whole conversation
// on the first measurement: reset, identify, configure, convert, read.
let air_now = sht3x::Sht3x::new(bus.clone(), air, bus.delay()).measure()?;
println!(
    "air          {:.2} C, {:.2} %",
    air_now.temperature_celsius(),
    air_now.relative_humidity()
);

let weather = bmp280::Bmp280::i2c(bus.clone(), pressure, bus.delay()).measure()?;
println!("pressure     {:.1} hPa", weather.hectopascals());

let co2 = scd4x::Scd4x::new(bus.clone(), bus.delay()).measure()?;
println!("co2          {} ppm", co2.co2_ppm);

let sun = opt3001::Opt3001::new(bus.clone(), light, bus.delay()).measure()?;
println!("light        {:.0} lux", sun.lux());

// A capacitive probe's voltage falls as the soil wets and nears 3 V in dry soil, past
// the ADS1115's default range of 2.048 V, so the driver is given the 4.096 V range.
let probe = ads1115::Ads1115::new(bus.clone(), soil, bus.delay())
    .with_input(ads1115::Mux::Ain0Gnd)
    .with_gain(ads1115::Pga::Fsr4_096)
    .sample()?;
println!("soil         {:.3} V", probe.volts());

let case = tmp117::Tmp117::new(bus.clone(), enclosure, bus.delay()).measure()?;
let case_air = hdc1080::Hdc1080::new(bus.clone(), bus.delay()).measure()?;
println!(
    "enclosure    {:.2} C, {:.1} %",
    case.celsius(),
    case_air.relative_humidity()
);

// Two current monitors, each calibrated for its own shunt. The fans and the pump draw
// more than the panel gives, so the battery makes up the rest and its current reads
// negative: current through a shunt is signed by its direction.
let charge = ina219::Ina219::new(bus.clone(), panel, bus.delay())
    .with_shunt(100, 3_200_000)
    .measure()?;
println!(
    "panel        {:.2} V, {:.2} A, {:.2} W",
    f64::from(charge.bus_millivolts()) / 1e3,
    f64::from(charge.current_microamps()) / 1e6,
    f64::from(charge.power_microwatts()) / 1e6
);
let drain = ina226::Ina226::new(bus.clone(), battery, bus.delay())
    .with_shunt(2, 20_000_000)
    .measure()?;
println!(
    "battery      {:.2} V, {:.2} A, {:.2} W",
    drain.bus_volts(),
    drain.current_amps(),
    drain.power_watts()
);

// Every driver waited as its datasheet asks, the OPT3001's 800 ms integration and the
// SCD41's command times most of all. The simulated bus counted the waits and slept none.
println!(
    "waited       {:.2} s across {} transfers",
    bus.waited_micros() as f64 / 1e6,
    bus.transfers()
);

// The same probe read at the default range. Past 2.048 V the converter pins at its top
// code, and the sample says so rather than passing the edge of the range off as a reading.
bus.attach(ads1115::sim::reporting(soil, ads1115::Pga::Fsr2_048, 2.35))?;
let pinned = ads1115::Ads1115::new(bus.clone(), soil, bus.delay())
    .with_input(ads1115::Mux::Ain0Gnd)
    .sample()?;
println!(
    "default gain {:.3} V, clipped: {}",
    pinned.volts(),
    pinned.clipped()
);

// A driver aimed at the wrong address meets whatever answers there. The TMP117 reads its
// device id before anything else, so pointed at the soil probe's converter it refuses
// rather than reporting that part's registers as a temperature.
match tmp117::Tmp117::new(bus.clone(), soil, bus.delay()).init() {
    Ok(()) => println!("wrong part   accepted, which should never happen"),
    Err(error) => println!("wrong part   {error}"),
}

// A DS18B20 on a lead into a pot, read the way a Linux board reads one: the kernel's
// 1-Wire driver serves each probe as a file under /sys/bus/w1/devices. The program writes
// that file itself, with the text the kernel prints, so it runs anywhere.
let devices = std::env::temp_dir().join(format!("pamoja-bench-{}", std::process::id()));
let directory = devices.join(format!("{:02x}-000005e2fdc3", ds18b20::FAMILY_CODE));
std::fs::create_dir_all(&directory)?;
let scratchpad = Scratchpad::new(
    ds18b20::temperature_from_celsius(19.5, Resolution::Bits12),
    Resolution::Bits12,
    30,
    5,
);
std::fs::write(
    directory.join(ds18b20::linux::W1_SLAVE),
    ds18b20::w1_slave_text(&scratchpad),
)?;
let mut found = Vec::new();
for thermometer in Thermometer::discover_in(&devices)? {
    let reading = thermometer.read_scratchpad()?;
    println!(
        "soil probe   {:.4} C, alarms at {} and {} C",
        reading.temperature_celsius(),
        reading.alarm_low(),
        reading.alarm_high()
    );
    found.push(reading);
}
std::fs::remove_dir_all(&devices)?;
```
<!-- end -->

## TypeScript

In TypeScript each driver is a class in `@pamoja/sensors`, and each part's addresses,
setting codes, and simulated part sit on a lowercase object of the same name:
`sht3x.addressA`, `ads1115.pga.fsr4_096`, `ina219.sim.reporting(...)`. Settings go to the
constructor as an options object with the Rust names in camel case, `{ mux, pga }` or
`{ shuntMilliohms, maxMicroamps }`, and anything left out keeps its datasheet default.
`init`, `measure`, and the ADS1115's `sample` run on a worker thread, since each waits out a
conversion, and return promises that reject with the reason. `I2cBus.simulated` from
`@pamoja/hal` takes any mix of `I2cPart`, `WordPart`, and `CommandPart`.

<!-- snippet: bindings/node/guides/sensors.ts#example -->
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
<!-- end -->

## Python

In Python each driver is a class in `pamoja.sensors`. Each part's addresses and simulated
part sit on a module-level object of the same name, `sht3x.ADDRESS_A` or
`ina219.sim.reporting(...)`, and the setting codes are `IntEnum`s named for the part,
`Ads1115Pga.FSR_4_096` or `Tmp117Averaging.X8`. Settings are keyword arguments, `mux=` and
`shunt_milliohms=`. Every call is synchronous and releases the interpreter while the part
answers, so other threads keep running through an 800 ms conversion. A failure raises
`PamojaError` from `pamoja.core`, with the reason as its message.

<!-- snippet: bindings/python/guides/sensors.py#example -->
From [`bindings/python/guides/sensors.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sensors.py):

```python
import os
import shutil
import tempfile
from pathlib import Path

from pamoja.core import PamojaError
from pamoja.hal import I2cBus
from pamoja.sensors import (
    Ads1115,
    Ads1115Mux,
    Ads1115Pga,
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
)

# The address plan. Every part answers at the address its pins choose, and two parts on one
# address garble each other, so a bench of nine is planned around the two that cannot move:
# the HDC1080 and the SCD41 have one address each.
AIR = sht3x.ADDRESS_A
PRESSURE = bmp280.ADDRESS_SECONDARY
LIGHT = opt3001.ADDRESS_SCL
SOIL = ads1115.ADDRESS_SDA
ENCLOSURE = tmp117.ADDRESS_ADD0_VPLUS
PANEL = ina219.address(ina219.PIN_GROUND, ina219.PIN_SUPPLY)
BATTERY = ina226.address(ina226.PIN_SUPPLY, ina226.PIN_SUPPLY)

# The bench with nothing plugged in: each part answers the way its datasheet says, with the
# reading it is given here, a warm and humid afternoon. On a Raspberry Pi the bus is
# I2cBus.open("/dev/i2c-1") and nothing after this statement changes.
bus = I2cBus.simulated([
    sht3x.sim.reporting(AIR, 24.1, 62.0),
    bmp280.sim.reporting(PRESSURE, 24.1, 1003.2),
    scd4x.sim.reporting(1_180, 24.1, 62.0),
    opt3001.sim.reporting(LIGHT, 4_200.0),
    ads1115.sim.reporting(SOIL, Ads1115Pga.FSR_4_096, 2.35),
    tmp117.sim.reporting(ENCLOSURE, 31.25),
    hdc1080.sim.reporting(31.25, 38.0),
    ina219.sim.reporting(PANEL, 100, 3_200_000, 18_400, 1_250_000),
    ina226.sim.reporting(BATTERY, 2, 20_000_000, 12_800_000, -350_000),
])

# One driver per part. Each holds its own share of the bus, runs its datasheet's whole
# conversation on the first measurement, and releases the interpreter while the part answers.
air_now = Sht3x(bus, AIR).measure()
print(f"air          {air_now.celsius:.2f} C, {air_now.relative_humidity:.2f} %")

weather = Bmp280(bus, PRESSURE).measure()
print(f"pressure     {weather.hectopascals:.1f} hPa")

co2 = Scd4x(bus).measure()
print(f"co2          {co2.co2_ppm} ppm")

sun = Opt3001(bus, LIGHT).measure()
print(f"light        {sun.lux:.0f} lux")

# A capacitive probe's voltage falls as the soil wets and nears 3 V in dry soil, past the
# ADS1115's default range of 2.048 V, so the driver is given the 4.096 V range.
probe = Ads1115(bus, SOIL, mux=Ads1115Mux.AIN0_GND, pga=Ads1115Pga.FSR_4_096).sample()
print(f"soil         {probe.volts:.3f} V")

box = Tmp117(bus, ENCLOSURE).measure()
box_air = Hdc1080(bus).measure()
print(f"enclosure    {box.celsius:.2f} C, {box_air.relative_humidity:.1f} %")

# Two current monitors, each calibrated for its own shunt. The fans and the pump draw more than
# the panel gives, so the battery makes up the rest and its current reads negative: current
# through a shunt is signed by its direction.
charge = Ina219(bus, PANEL, shunt_milliohms=100, max_microamps=3_200_000).measure()
print(
    f"panel        {charge.bus_millivolts / 1e3:.2f} V, "
    f"{charge.current_microamps / 1e6:.2f} A, {charge.power_microwatts / 1e6:.2f} W"
)
drain = Ina226(bus, BATTERY, shunt_milliohms=2, max_microamps=20_000_000).measure()
print(
    f"battery      {drain.bus_volts:.2f} V, {drain.current_amps:.2f} A, "
    f"{drain.power_watts:.2f} W"
)

# Every driver waited as its datasheet asks, the OPT3001's 800 ms integration and the SCD41's
# command times most of all. The simulated bus counted the waits and slept none.
print(f"waited       {bus.waited_micros / 1e6:.2f} s across {bus.transfers} transfers")

# The same probe read at the default range. Past 2.048 V the converter pins at its top code,
# and the sample says so rather than passing the edge of the range off as a reading.
bus.attach(ads1115.sim.reporting(SOIL, Ads1115Pga.FSR_2_048, 2.35))
pinned = Ads1115(bus, SOIL, mux=Ads1115Mux.AIN0_GND).sample()
print(f"default gain {pinned.volts:.3f} V, clipped: {str(pinned.clipped).lower()}")

# A driver aimed at the wrong address meets whatever answers there. The TMP117 reads its
# device id before anything else, so pointed at the soil probe's converter it refuses rather
# than reporting that part's registers as a temperature.
try:
    Tmp117(bus, SOIL).init()
    print("wrong part   accepted, which should never happen")
except PamojaError as error:
    print(f"wrong part   {error}")

# A DS18B20 on a lead into a pot, read the way a Linux board reads one: the kernel's 1-Wire
# driver serves each probe as a file under /sys/bus/w1/devices. The program writes that file
# itself, with the text the kernel prints, so it runs anywhere.
devices = Path(tempfile.gettempdir()) / f"pamoja-bench-{os.getpid()}"
directory = devices / f"{ds18b20.FAMILY_CODE:02x}-000005e2fdc3"
directory.mkdir(parents=True, exist_ok=True)
scratchpad = ds18b20.build_scratchpad(19.5, 12, 30, 5)
(directory / "w1_slave").write_text(ds18b20.w1_slave_text(scratchpad), newline="")
found = []
for thermometer in Ds18b20Thermometer.discover(str(devices)):
    reading = thermometer.read()
    print(
        f"soil probe   {reading.celsius:.4f} C, "
        f"alarms at {reading.alarm_low} and {reading.alarm_high} C"
    )
    found.append(reading)
shutil.rmtree(devices)
```
<!-- end -->

## C#

In C# each part is a class in `Pamoja.Sensors` whose static members are its datasheet:
its addresses, its setting enums such as `Ads1115.Pga`, the pin-based `Ina219.Address`, and
a nested `Sim` class with its simulated part. Settings are optional constructor parameters,
as in `new Ads1115(bus, address, Ads1115.Mux.Ain0Gnd, Ads1115.Pga.Fsr4_096)`. Buses, parts,
and drivers hold native handles and are `IDisposable`. `I2cBus.Simulated` takes
`SimulatedPart`s, the base class of `I2cPart`, `WordPart`, and `CommandPart`, and copies
each onto the bus, so the example disposes its own copies straight away. The SHT3x, SCD4x,
and HDC1080 return the flat measurement structs of `Pamoja.Native.Interop`, which the
example takes with `var`. A failure throws `PamojaException` with the reason as its
message.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SensorsGuide.cs):

```csharp
// The address plan. Every part answers at the address its pins choose, and two parts on
// one address garble each other, so a bench of nine is planned around the two that
// cannot move: the HDC1080 and the SCD41 have one address each.
const byte Air = Sht3x.AddressA;
const byte Pressure = Bmp280.AddressSecondary;
const byte Light = Opt3001.AddressScl;
const byte Soil = Ads1115.AddressSda;
const byte Enclosure = Tmp117.AddressAdd0Vplus;
byte panel = Ina219.Address(Ina219.AddressPin.Ground, Ina219.AddressPin.Supply);
byte battery = Ina226.Address(Ina226.AddressPin.Supply, Ina226.AddressPin.Supply);

// The bench with nothing plugged in: each part answers the way its datasheet says, with
// the reading it is given here, a warm and humid afternoon. The bus keeps a copy of each
// part, so the program lets go of its own. On a Raspberry Pi the bus is
// I2cBus.Open("/dev/i2c-1") and nothing after this statement changes.
SimulatedPart[] parts =
[
    Sht3x.Sim.Reporting(Air, 24.1f, 62.0f),
    Bmp280.Sim.Reporting(Pressure, 24.1f, 1003.2f),
    Scd4x.Sim.Reporting(1_180, 24.1f, 62.0f),
    Opt3001.Sim.Reporting(Light, 4_200f),
    Ads1115.Sim.Reporting(Soil, Ads1115.Pga.Fsr4_096, 2.35f),
    Tmp117.Sim.Reporting(Enclosure, 31.25f),
    Hdc1080.Sim.Reporting(31.25f, 38.0f),
    Ina219.Sim.Reporting(panel, 100, 3_200_000, 18_400, 1_250_000),
    Ina226.Sim.Reporting(battery, 2, 20_000_000, 12_800_000, -350_000),
];
using I2cBus bus = I2cBus.Simulated(parts);
foreach (SimulatedPart part in parts)
{
    part.Dispose();
}

// One driver per part. Each holds its own share of the bus, and each runs its
// datasheet's whole conversation on the first measurement: reset, identify, configure,
// convert, read.
using var airSensor = new Sht3x(bus, Air);
var airNow = airSensor.Measure();
Console.WriteLine(Invariant($"air          {airNow.Celsius:F2} C, {airNow.RelativeHumidity:F2} %"));

using var barometer = new Bmp280(bus, Pressure);
Bmp280Reading weather = barometer.Measure();
Console.WriteLine(Invariant($"pressure     {weather.Hectopascals:F1} hPa"));

using var co2Sensor = new Scd4x(bus);
var co2 = co2Sensor.Measure();
Console.WriteLine($"co2          {co2.Co2Ppm} ppm");

using var lightSensor = new Opt3001(bus, Light);
Opt3001Reading sun = lightSensor.Measure();
Console.WriteLine(Invariant($"light        {sun.Lux:F0} lux"));

// A capacitive probe's voltage falls as the soil wets and nears 3 V in dry soil, past the
// ADS1115's default range of 2.048 V, so the driver is given the 4.096 V range.
using var converter = new Ads1115(bus, Soil, Ads1115.Mux.Ain0Gnd, Ads1115.Pga.Fsr4_096);
Ads1115Sample probe = converter.Sample();
Console.WriteLine(Invariant($"soil         {probe.Volts:F3} V"));

using var boxThermometer = new Tmp117(bus, Enclosure);
using var boxHygrometer = new Hdc1080(bus);
Tmp117Reading box = boxThermometer.Measure();
var boxAir = boxHygrometer.Measure();
Console.WriteLine(Invariant($"enclosure    {box.Celsius:F2} C, {boxAir.RelativeHumidity:F1} %"));

// Two current monitors, each calibrated for its own shunt. The fans and the pump draw
// more than the panel gives, so the battery makes up the rest and its current reads
// negative: current through a shunt is signed by its direction.
using var panelMonitor = new Ina219(bus, panel, shuntMilliohms: 100, maxMicroamps: 3_200_000);
Ina219Reading charge = panelMonitor.Measure();
Console.WriteLine(Invariant(
    $"panel        {charge.BusMillivolts / 1e3:F2} V, {charge.CurrentMicroamps / 1e6:F2} A, {charge.PowerMicrowatts / 1e6:F2} W"));
using var batteryMonitor = new Ina226(bus, battery, shuntMilliohms: 2, maxMicroamps: 20_000_000);
Ina226Reading drain = batteryMonitor.Measure();
Console.WriteLine(Invariant(
    $"battery      {drain.BusVolts:F2} V, {drain.CurrentAmps:F2} A, {drain.PowerWatts:F2} W"));

// Every driver waited as its datasheet asks, the OPT3001's 800 ms integration and the
// SCD41's command times most of all. The simulated bus counted the waits and slept none.
Console.WriteLine(Invariant($"waited       {bus.WaitedMicros / 1e6:F2} s across {bus.Transfers} transfers"));

// The same probe read at the default range. Past 2.048 V the converter pins at its top
// code, and the sample says so rather than passing the edge of the range off as a reading.
using (WordPart reprobed = Ads1115.Sim.Reporting(Soil, Ads1115.Pga.Fsr2_048, 2.35f))
{
    bus.Attach(reprobed);
}

using var defaultGain = new Ads1115(bus, Soil, Ads1115.Mux.Ain0Gnd);
Ads1115Sample pinned = defaultGain.Sample();
Console.WriteLine(Invariant(
    $"default gain {pinned.Volts:F3} V, clipped: {pinned.Clipped.ToString().ToLowerInvariant()}"));

// A driver aimed at the wrong address meets whatever answers there. The TMP117 reads its
// device id before anything else, so pointed at the soil probe's converter it refuses
// rather than reporting that part's registers as a temperature.
try
{
    using var misplaced = new Tmp117(bus, Soil);
    misplaced.Init();
    Console.WriteLine("wrong part   accepted, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"wrong part   {error.Message}");
}

// A DS18B20 on a lead into a pot, read the way a Linux board reads one: the kernel's
// 1-Wire driver serves each probe as a file under /sys/bus/w1/devices. The program writes
// that file itself, with the text the kernel prints, so it runs anywhere.
string devices = Path.Combine(Path.GetTempPath(), $"pamoja-bench-{Environment.ProcessId}");
string directory = Path.Combine(devices, $"{Ds18b20.FamilyCode:x2}-000005e2fdc3");
Directory.CreateDirectory(directory);
byte[] scratchpad = Ds18b20.BuildScratchpad(19.5f, 12, 30, 5);
File.WriteAllText(Path.Combine(directory, "w1_slave"), Ds18b20.W1SlaveText(scratchpad));
var found = new List<Ds18b20Reading>();
foreach (Ds18b20Thermometer thermometer in Ds18b20Thermometer.Discover(devices))
{
    using (thermometer)
    {
        Ds18b20Reading reading = thermometer.Read();
        Console.WriteLine(Invariant(
            $"soil probe   {reading.Celsius:F4} C, alarms at {reading.AlarmLow} and {reading.AlarmHigh} C"));
        found.Add(reading);
    }
}

Directory.Delete(devices, recursive: true);
```
<!-- end -->

## On a board

The same drivers on a Raspberry Pi: an [SHT31](../hardware.md#sht3x) breakout on the
header's I2C bus for the air, and [DS18B20](../hardware.md#ds18b20) probes on its 1-Wire bus
for the soil. The program prints the air and then every probe by its serial, every ten
seconds, and carries on through a bad read.

| SHT31 breakout | Raspberry Pi |
| --- | --- |
| VIN | a 3V3 pin |
| GND | a ground pin |
| SDA | GPIO2 |
| SCL | GPIO3 |

| DS18B20 probe | Raspberry Pi |
| --- | --- |
| red lead, VDD | a 3V3 pin |
| black lead, GND | a ground pin |
| yellow or white lead, DQ | GPIO4 |
| a 4.7 kilohm resistor | from GPIO4 to 3V3 |

Every probe shares the one data line and the one resistor, which is the pull-up the
DS18B20 datasheet draws in its circuits; the bus does not work without it.
[Turn the I2C interface and the 1-Wire overlay on](../boards/raspberry-pi.md#turning-the-buses-on)
with `dtparam=i2c_arm=on` and `dtoverlay=w1-gpio` in `config.txt`, reboot, and check both
buses before running anything: `i2cdetect -y 1` should show `44`, and
`ls /sys/bus/w1/devices` should list one `28-` directory per probe. A probe missing from
that list is a wiring problem, and no program will find it.

### Rust

<!-- snippet: examples/boards/raspberry-pi/src/bin/probes.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/probes.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/probes.rs):

```rust
use pamoja_hal::bus::I2cBus;
use pamoja_sensors::ds18b20::linux::Thermometer;
use pamoja_sensors::sht3x::{Sht3x, I2C_ADDRESS_A};

fn main() -> Result<(), Box<dyn Error>> {
    // The air sensor, on the header's I2C bus: /dev/i2c-1 on every model.
    let bus = I2cBus::open("/dev/i2c-1")?;
    let mut air = Sht3x::new(bus.clone(), I2C_ADDRESS_A, bus.delay());

    // The kernel lists each DS18B20 it has found on GPIO4 as a directory named for its
    // serial. The list is taken once, so a probe plugged in later needs a restart.
    let probes = Thermometer::discover()?;
    if probes.is_empty() {
        return Err(
            "no DS18B20 under /sys/bus/w1/devices: check the pull-up and the overlay".into(),
        );
    }

    loop {
        let now = air.measure()?;
        println!(
            "air           {:.2} C, {:.1} %",
            now.temperature_celsius(),
            now.relative_humidity()
        );

        // Reading a probe makes the kernel run a conversion, 750 ms at 12 bits. A reading
        // corrupted on a long lead fails its checksum, and the logger says so and reads the
        // probe again next time rather than stopping.
        for probe in &probes {
            let serial = probe.serial().unwrap_or_default();
            match probe.read_scratchpad() {
                Ok(soil) => println!("{serial}  {:.2} C", soil.temperature_celsius()),
                Err(error) => println!("{serial}  {error}"),
            }
        }
        thread::sleep(Duration::from_secs(10));
    }
}
```
<!-- end -->

```sh
cd examples/boards/raspberry-pi
cargo run --release --bin probes
```

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/probes.ts#example -->
From [`bindings/node/boards/raspberry-pi/probes.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/probes.ts):

```typescript
import { setTimeout as sleep } from 'node:timers/promises'
import { I2cBus } from '@pamoja/hal'
import { Ds18b20Thermometer, Sht3x, sht3x } from '@pamoja/sensors'

async function main(): Promise<void> {
  // The air sensor, on the header's I2C bus: /dev/i2c-1 on every model.
  const bus = I2cBus.open('/dev/i2c-1')
  const air = new Sht3x(bus, sht3x.addressA)

  // The kernel lists each DS18B20 it has found on GPIO4 as a directory named for its
  // serial. The list is taken once, so a probe plugged in later needs a restart.
  const probes = Ds18b20Thermometer.discover()
  if (probes.length === 0) {
    throw new Error('no DS18B20 under /sys/bus/w1/devices: check the pull-up and the overlay')
  }

  for (;;) {
    const now = await air.measure()
    console.log(`air           ${now.celsius.toFixed(2)} C, ${now.relativeHumidity.toFixed(1)} %`)

    // Reading a probe makes the kernel run a conversion, 750 ms at 12 bits. A reading
    // corrupted on a long lead fails its checksum, and the logger says so and reads the probe
    // again next time rather than stopping.
    for (const probe of probes) {
      try {
        const soil = await probe.read()
        console.log(`${probe.serial}  ${soil.celsius.toFixed(2)} C`)
      } catch (error) {
        console.log(`${probe.serial}  ${(error as Error).message}`)
      }
    }
    await sleep(10_000)
  }
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/probes.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/probes.py#example -->
From [`bindings/python/boards/raspberry_pi/probes.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/probes.py):

```python
import time

from pamoja.core import PamojaError
from pamoja.hal import I2cBus
from pamoja.sensors import Ds18b20Thermometer, Sht3x, sht3x


def main() -> None:
    # The air sensor, on the header's I2C bus: /dev/i2c-1 on every model.
    bus = I2cBus.open("/dev/i2c-1")
    air = Sht3x(bus, sht3x.ADDRESS_A)

    # The kernel lists each DS18B20 it has found on GPIO4 as a directory named for its
    # serial. The list is taken once, so a probe plugged in later needs a restart.
    probes = Ds18b20Thermometer.discover()
    if not probes:
        raise SystemExit(
            "no DS18B20 under /sys/bus/w1/devices: check the pull-up and the overlay"
        )

    while True:
        now = air.measure()
        print(f"air           {now.celsius:.2f} C, {now.relative_humidity:.1f} %")

        # Reading a probe makes the kernel run a conversion, 750 ms at 12 bits. A reading
        # corrupted on a long lead fails its checksum, and the logger says so and reads the
        # probe again next time rather than stopping.
        for probe in probes:
            try:
                soil = probe.read()
                print(f"{probe.serial}  {soil.celsius:.2f} C")
            except PamojaError as error:
                print(f"{probe.serial}  {error}")
        time.sleep(10)
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/probes.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Probes.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Probes.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Probes.cs):

```csharp
using System.Globalization;

using Pamoja;
using Pamoja.Hal;
using Pamoja.Sensors;

namespace Boards.RaspberryPi;

/// <summary>
/// A greenhouse logger on a Raspberry Pi: the air from an SHT31 on the header's I2C bus, and
/// every DS18B20 soil probe the kernel's 1-Wire driver has found, printed every ten seconds.
/// Wire the SHT31's SDA to GPIO2, its SCL to GPIO3, VIN to 3V3, and GND to ground, and turn the
/// I2C interface on. Wire each probe's red lead to 3V3, its black lead to ground, and its yellow
/// data lead to GPIO4, with one 4.7 kilohm resistor from GPIO4 to 3V3 for the whole bus; add
/// <c>dtoverlay=w1-gpio</c> to config.txt and reboot.
/// </summary>
public static class Probes
{
    /// <summary>Runs until the process is stopped.</summary>
    public static void Run()
    {
        // The air sensor, on the header's I2C bus: /dev/i2c-1 on every model.
        using I2cBus bus = I2cBus.Open("/dev/i2c-1");
        using var air = new Sht3x(bus, Sht3x.AddressA);

        // The kernel lists each DS18B20 it has found on GPIO4 as a directory named for its
        // serial. The list is taken once, so a probe plugged in later needs a restart.
        IReadOnlyList<Ds18b20Thermometer> probes = Ds18b20Thermometer.Discover();
        if (probes.Count == 0)
        {
            throw new PamojaException("no DS18B20 under /sys/bus/w1/devices: check the pull-up and the overlay");
        }

        while (true)
        {
            var now = air.Measure();
            Console.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"air           {now.Celsius:F2} C, {now.RelativeHumidity:F1} %"));

            // Reading a probe makes the kernel run a conversion, 750 ms at 12 bits. A reading
            // corrupted on a long lead fails its checksum, and the logger says so and reads the
            // probe again next time rather than stopping.
            foreach (Ds18b20Thermometer probe in probes)
            {
                try
                {
                    Ds18b20Reading soil = probe.Read();
                    Console.WriteLine(string.Create(CultureInfo.InvariantCulture, $"{probe.Serial}  {soil.Celsius:F2} C"));
                }
                catch (PamojaException error)
                {
                    Console.WriteLine($"{probe.Serial}  {error.Message}");
                }
            }

            Thread.Sleep(TimeSpan.FromSeconds(10));
        }
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/probes
```

## Values at a glance

**Where each part answers.** Two parts at one address both answer every transfer and
garble each other, so a bench is planned from this table:

| Part | Measures | Addresses | Chosen by | On the bench |
| --- | --- | --- | --- | --- |
| SHT30, SHT31, SHT35 | temperature, humidity | `0x44`, `0x45` | ADDR low or high | `0x44` |
| BMP280 | pressure, temperature | `0x76`, `0x77` | SDO to GND or VDDIO | `0x77` |
| BME280 | temperature, pressure, humidity | `0x76`, `0x77` | SDO to GND or VDDIO | the [buses guide](hal.md) |
| SCD40, SCD41 | carbon dioxide, temperature, humidity | `0x62` | fixed | `0x62` |
| OPT3001 | illuminance | `0x44` to `0x47` | ADDR to GND, VDD, SDA, or SCL | `0x47` |
| ADS1115 | four analog inputs | `0x48` to `0x4B` | ADDR to GND, VDD, SDA, or SCL | `0x4A` |
| TMP117 | temperature | `0x48` to `0x4B` | ADD0 to GND, V+, SDA, or SCL | `0x49` |
| HDC1080 | temperature, humidity | `0x40` | fixed | `0x40` |
| INA219 | shunt current, bus voltage, power | `0x40` to `0x4F` | A1 and A0, below | `0x41` |
| INA226 | shunt current, bus voltage, power | `0x40` to `0x4F` | A1 and A0, below | `0x45` |
| DS18B20 | temperature | a 64-bit id on a 1-Wire bus | the factory | `28-000005e2fdc3` |

**The INA219 and INA226 address pins.** A1 and A0 each go to ground, the supply (VS+ on
the INA219, VS on the INA226), SDA, or SCL, which makes sixteen addresses. `ina219::address`
and `ina226::address` compute them; rows are A1, columns A0:

| A1, A0 | GND | VS | SDA | SCL |
| --- | --- | --- | --- | --- |
| GND | `0x40` | `0x41` | `0x42` | `0x43` |
| VS | `0x44` | `0x45` | `0x46` | `0x47` |
| SDA | `0x48` | `0x49` | `0x4A` | `0x4B` |
| SCL | `0x4C` | `0x4D` | `0x4E` | `0x4F` |

**What a measurement waits.** Each driver waits the time its datasheet gives for its
settings, and the first measurement also runs the start-up sequence. These are the waits
and transfers at the default settings. The bench's 1.48 s is the first column summed over
its nine parts, and its 66 transfers the same:

| Part | At | First measurement | Each one after |
| --- | --- | --- | --- |
| SHT3x | high repeatability | 17.5 ms, 5 transfers | 15 ms, 2 transfers |
| BMP280 | x1 temperature and pressure | 8.425 ms, 9 transfers | 6.425 ms, 3 transfers |
| SCD4x | periodic measurement | 503 ms, 8 transfers | 2 ms, 4 transfers |
| OPT3001 | 800 ms conversions | 800 ms, 6 transfers | 800 ms, 3 transfers |
| ADS1115 | 128 samples a second | 8.595 ms, 5 transfers | 8.595 ms, 3 transfers |
| TMP117 | 8 conversions averaged | 125 ms, 7 transfers | 125 ms, 4 transfers |
| HDC1080 | 14 bits each | 12.85 ms, 5 transfers | 12.85 ms, 2 transfers |
| INA219 | 12-bit shunt and bus | 1.064 ms, 9 transfers | 1.064 ms, 5 transfers |
| INA226 | 1.1 ms each, no averaging | 2.2 ms, 12 transfers | 2.2 ms, 6 transfers |
| DS18B20 | 12 bits, behind the kernel | up to 750 ms, in the kernel | up to 750 ms |

The SCD4x figures are its command times only. A real part has its first result about five
seconds after measurement starts and one every five seconds after that, and `measure`
polls until it arrives; a simulated part always has one waiting. The ADS1115 waits one
period of its data rate plus the ten percent by which the datasheet lets that rate vary.

**Settings each driver takes.** A setting left out keeps the part's power-on default, which
the table after these gives. An INA219's or INA226's configuration carries a mode as well,
but the driver triggers each conversion itself and does not use it. The same settings in
each language:

### Rust

Builder methods on the driver, each returning it, so they chain before the first
measurement:

| Setting | Method | Choices |
| --- | --- | --- |
| SHT3x repeatability | `with_repeatability` | `sht3x::Repeatability` |
| BMP280 oversampling | `with_oversampling(t, p)` | `bmp280::Oversampling` |
| BMP280 IIR filter | `with_filter(code)` | a code, 0 to 4 |
| OPT3001 conversion time | `with_conversion_time` | `opt3001::ConversionTime` |
| OPT3001 range | `with_range(n)` | 0 to 11, or `opt3001::RANGE_AUTOMATIC` |
| HDC1080 resolutions | `with_resolutions(t, h)` | `hdc1080::TemperatureResolution`, `hdc1080::HumidityResolution` |
| TMP117 averaging | `with_averaging` | `tmp117::Averaging` |
| INA219 or INA226 shunt | `with_shunt(milliohms, max_microamps)` | the shunt and the largest current it carries |
| INA219 or INA226 current step | `with_current_lsb(microamps)` | a step in place of the finest |
| INA219 ranges and converters | `with_configuration` | `ina219::Configuration` |
| INA226 averaging and conversion times | `with_configuration` | `ina226::Configuration` |
| ADS1115 input | `with_input` | `ads1115::Mux` |
| ADS1115 range | `with_gain` | `ads1115::Pga` |
| ADS1115 data rate | `with_data_rate` | `ads1115::DataRate` |

### TypeScript

Fields of the options object the constructor takes last:

| Setting | Field | Choices |
| --- | --- | --- |
| SHT3x repeatability | `repeatability` | `Sht3xRepeatability` |
| BMP280 oversampling | `temperature`, `pressure` | `bmp280.oversampling` |
| BMP280 IIR filter | `filter` | a code, 0 to 4 |
| OPT3001 conversion time | `longConversion` | `true` for 800 ms, `false` for 100 ms |
| OPT3001 range | `rangeNumber` | 0 to 11, or `opt3001.rangeAutomatic` |
| HDC1080 resolutions | `temperatureResolutionBits`, `humidityResolutionBits` | 14 or 11, and 14, 11, or 8 |
| TMP117 averaging | `averaging` | `tmp117.averaging` |
| INA219 or INA226 shunt | `shuntMilliohms`, `maxMicroamps` | the shunt and the largest current it carries |
| INA219 or INA226 current step | `currentLsbMicroamps` | a step in place of the finest |
| INA219 ranges and converters | `configuration` | `ina219.busRange`, `ina219.gain`, `ina219.adc`, `ina219.mode` |
| INA226 averaging and conversion times | `configuration` | `ina226.averaging`, `ina226.conversionTime`, `ina226.mode` |
| ADS1115 input | `mux` | `ads1115.mux` |
| ADS1115 range | `pga` | `ads1115.pga` |
| ADS1115 data rate | `dataRate` | `ads1115.dataRate` |

### Python

Keyword arguments to the constructor:

| Setting | Argument | Choices |
| --- | --- | --- |
| SHT3x repeatability | `repeatability` | `Sht3xRepeatability` |
| BMP280 oversampling | `temperature`, `pressure` | `Bmp280Oversampling` |
| BMP280 IIR filter | `filter` | a code, 0 to 4 |
| OPT3001 conversion time | `conversion_time` | `Opt3001ConversionTime` |
| OPT3001 range | `range_number` | 0 to 11, or `opt3001.RANGE_AUTOMATIC` |
| HDC1080 resolutions | `temperature_resolution`, `humidity_resolution` | `Hdc1080TemperatureResolution`, `Hdc1080HumidityResolution` |
| TMP117 averaging | `averaging` | `Tmp117Averaging` |
| INA219 or INA226 shunt | `shunt_milliohms`, `max_microamps` | the shunt and the largest current it carries |
| INA219 or INA226 current step | `current_lsb_microamps` | a step in place of the finest |
| INA219 ranges and converters | `config` | `Ina219Config` of `Ina219BusRange`, `Ina219Gain`, `Ina219Adc`, `Ina219Mode` |
| INA226 averaging and conversion times | `config` | `Ina226Config` of `Ina226Averaging`, `Ina226ConversionTime`, `Ina226Mode` |
| ADS1115 input | `mux` | `Ads1115Mux` |
| ADS1115 range | `pga` | `Ads1115Pga` |
| ADS1115 data rate | `data_rate` | `Ads1115DataRate` |

### C#

Optional parameters of the constructor, best passed by name:

| Setting | Parameter | Choices |
| --- | --- | --- |
| SHT3x repeatability | `repeatability` | `Sht3x.Repeatability` |
| BMP280 oversampling | `temperature`, `pressure` | `Bmp280.Oversampling` |
| BMP280 IIR filter | `filter` | a code, 0 to 4 |
| OPT3001 conversion time | `conversionTime` | `Opt3001.ConversionTime` |
| OPT3001 range | `range` | 0 to 11, or `Opt3001.RangeAutomatic` |
| HDC1080 resolutions | `temperature`, `humidity` | `Hdc1080.TemperatureResolution`, `Hdc1080.HumidityResolution` |
| TMP117 averaging | `averaging` | `Tmp117.Averaging` |
| INA219 or INA226 shunt | `shuntMilliohms`, `maxMicroamps` | the shunt and the largest current it carries |
| INA219 or INA226 current step | `currentLsbMicroamps` | a step in place of the finest |
| INA219 ranges and converters | `config` | `Ina219Config` of `Ina219.BusRange`, `Ina219.Gain`, `Ina219.Adc`, `Ina219.Mode` |
| INA226 averaging and conversion times | `config` | `Ina226Config` of `Ina226.Averaging`, `Ina226.ConversionTime`, `Ina226.Mode` |
| ADS1115 input | `input` | `Ads1115.Mux` |
| ADS1115 range | `gain` | `Ads1115.Pga` |
| ADS1115 data rate | `rate` | `Ads1115.DataRate` |

<!-- languages end -->

What the settings trade, part by part:

| Part | Setting | Choices | Default |
| --- | --- | --- | --- |
| SHT3x | repeatability | low: 0.15 °C and 0.21 %RH noise, 4 ms; medium: 0.08 °C and 0.15 %RH, 6 ms; high: 0.04 °C and 0.08 %RH, 15 ms | high |
| BMP280 | oversampling | skipped, or x1 to x16, each step one more bit of output, 16 to 20 | x1 each |
| BMP280 | IIR filter | off, or coefficients 2, 4, 8, and 16 on pressure and temperature | off |
| OPT3001 | conversion time | 100 ms, losing one to three bits on ranges 0 to 5, or 800 ms | 800 ms |
| OPT3001 | range | automatic, or a fixed range n from 0 to 11: full scale 40.95 × 2ⁿ lux in steps of 0.01 × 2ⁿ lux | automatic |
| HDC1080 | temperature | 14 bits, 6.35 ms, or 11 bits, 3.65 ms | 14 bits |
| HDC1080 | humidity | 14 bits, 6.5 ms, 11 bits, 3.85 ms, or 8 bits, 2.5 ms | 14 bits |
| TMP117 | averaging | none, 15.5 ms; 8, 125 ms; 32, 500 ms; 64, 1 s | 8 |
| INA219 | bus range | 16 V or 32 V of converter scale; the inputs take 26 V at most | 32 V |
| INA219 | gain | ±40, ±80, ±160, or ±320 mV across the shunt | ±320 mV |
| INA219 | converters | 9 to 12 bits, 84 to 532 µs, or 2 to 128 samples averaged, 1.06 to 68.10 ms | 12 bits |
| INA226 | averaging | 1, 4, 16, 64, 128, 256, 512, or 1024 samples | 1 |
| INA226 | conversion time | 140, 204, 332, or 588 µs, or 1.1, 2.116, 4.156, or 8.244 ms | 1.1 ms |
| ADS1115 | input | AIN0 against AIN1, AIN0, AIN1, or AIN2 against AIN3, or any one input against ground | AIN0 against AIN1 |
| ADS1115 | data rate | 8, 16, 32, 64, 128, 250, 475, or 860 samples a second | 128 |
| DS18B20 | resolution | 9 bits, 0.5 °C, 93.75 ms; 10 bits, 0.25 °C, 187.5 ms; 11 bits, 0.125 °C, 375 ms; 12 bits, 0.0625 °C, 750 ms | 12 bits |

**The ADS1115's ranges** set the size of one count, from Table 7-1 of its datasheet. A
range only scales the converter: no input may go more than 0.3 V past the part's supply,
whichever range is set, so a 5 V signal into an ADS1115 on 3.3 V needs a divider even at
±6.144 V.

| Range | One count |
| --- | --- |
| ±6.144 V | 187.5 µV |
| ±4.096 V | 125 µV |
| ±2.048 V, the default | 62.5 µV |
| ±1.024 V | 31.25 µV |
| ±0.512 V | 15.625 µV |
| ±0.256 V | 7.8125 µV |

**Calibrating a current monitor.** An INA219 or INA226 computes nothing until it knows its
shunt. Given the shunt and the largest current it will carry, the driver picks the finest
current step that still reaches that current and writes the calibration word the datasheet
derives from the two:

| | The panel | The battery |
| --- | --- | --- |
| Monitor | INA219 at `0x41` | INA226 at `0x45` |
| Shunt | 100 milliohms | 2 milliohms |
| Largest current | 3.2 A | 20 A |
| Current step the driver picks | 98 µA | 611 µA |
| Calibration word | 4179 | 4189 |
| Its reading on the bench | 1.25 A into the battery | 0.35 A out of it |

The INA219's shunt range limits the current: at its widest, ±320 mV across the 0.1 ohm
shunt most breakouts carry is ±3.2 A.

**Simulated parts.** `sim::part(address)` gives each part a reading of its own, and
`sim::reporting(...)` a chosen one. The kind of part decides what a transfer does:

| Part | Kind | `reporting` takes | `part` reports |
| --- | --- | --- | --- |
| SHT3x | commands | address, °C, %RH | 22.5 °C, 45 % |
| BMP280 | byte registers | address, °C, hPa | 20.44 °C, 848.05 hPa, from a real BME280's pressure and temperature registers, which the BMP280 shares |
| SCD4x | commands | ppm, °C, %RH | 800 ppm, 22.5 °C, 45 % |
| OPT3001 | word registers | address, lux | 380 lux |
| ADS1115 | word registers | address, range, volts | 1.65 V |
| TMP117 | word registers | address, °C | 21.25 °C |
| HDC1080 | word registers | °C, %RH | 22.5 °C, 45 % |
| INA219 | word registers | address, shunt milliohms, largest microamps, bus millivolts, microamps | 12 V and 0.5 A across 100 milliohms |
| INA226 | word registers | address, shunt milliohms, largest microamps, bus microvolts, microamps | 12 V and 0.5 A across 100 milliohms |

**The same calls in each language:**

| To | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| build a driver | `Sht3x::new(bus.clone(), a, bus.delay())` | `new Sht3x(bus, a, settings)` | `Sht3x(bus, a, **settings)` | `new Sht3x(bus, a, ...)` |
| run the start-up sequence | `init()` | `await init()` | `init()` | `Init()` |
| measure | `measure()`, or `sample()` on the ADS1115 | `await measure()`, `await sample()` | `measure()`, `sample()` | `Measure()`, `Sample()` |
| read another ADS1115 input once | `sample_input(mux)` | `await sampleInput(mux)` | `sample_input(mux)` | `SampleInput(mux)` |
| tell whether a conversion clipped | `sample.clipped()` | `sample.clipped` | `sample.clipped` | `sample.Clipped` |
| stand a part up | `sht3x::sim::reporting(a, c, rh)` | `sht3x.sim.reporting(a, c, rh)` | `sht3x.sim.reporting(a, c, rh)` | `Sht3x.Sim.Reporting(a, c, rh)` |
| an INA219 address from its pins | `ina219::address(AddressPin::Ground, AddressPin::Supply)` | `ina219.address(ina219.pin.ground, ina219.pin.supply)` | `ina219.address(ina219.PIN_GROUND, ina219.PIN_SUPPLY)` | `Ina219.Address(Ina219.AddressPin.Ground, Ina219.AddressPin.Supply)` |
| find the kernel's DS18B20s | `Thermometer::discover()` | `Ds18b20Thermometer.discover()` | `Ds18b20Thermometer.discover()` | `Ds18b20Thermometer.Discover()` |
| read one | `read_scratchpad()` | `await read()` | `read()` | `Read()` |
| tell one from another | `serial()` | `serial` | `serial` | `Serial` |
| write the kernel's text for a scratchpad | `ds18b20::w1_slave_text(&s)` | `ds18b20.w1SlaveText(bytes)` | `ds18b20.w1_slave_text(bytes)` | `Ds18b20.W1SlaveText(bytes)` |

**Beyond a measurement**, some parts do more, in every language under the same names in
its own case:

| Part | What | Rust |
| --- | --- | --- |
| SHT3x | the status register, the heater | `read_status()`, `status()`, `heater_on()`, `heater_off()` |
| SCD4x | readiness, stopping and starting, compensation, one-off readings on an SCD41 | `data_ready()`, `stop()`, `start()`, `set_temperature_offset(mc)`, `set_sensor_altitude(m)`, `measure_single_shot()`, `serial()` |
| TMP117 | alert limits and their flags | `set_alert_limits(high, low)`, `alerts()`, `revision()` |
| OPT3001 | the interrupt window | `set_limits(low, high)`, `configuration()` |
| HDC1080 | the heater | `heater(on)`, `configuration()` |
| INA219 | the programmed scale | `current_lsb_microamps()`, `calibration()` |
| INA226 | the alert pin, the programmed scale | `set_alert(mask, limit)`, `current_lsb_microamps()`, `calibration()`, `die_id()` |
| ADS1115 | the configuration it wrote | `config()` |
| BMP280 | the calibration it read | `calibration()` |

The bindings name four of them differently: the SHT3x's `status` is `lastStatus`, the
TMP117's `revision` is `siliconRevision`, the monitors' `calibration` is `calibrationWord`,
and the INA226's `die_id` is `identity`, each cased for its language.

**Underneath every driver** is the part's decode layer, which needs no bus: the BMP280's
compensation, the DS18B20's scratchpad and CRC, the INA219's and INA226's register
arithmetic, the OPT3001's exponent and mantissa, the Sensirion frames and checksums. A
microcontroller that reaches a part some other way, or a gateway that receives the raw
bytes from a node, calls these directly, and every decode has a builder beside it, which is
how the simulated parts are made.

## When it goes wrong

What a driver refuses, and what it says:

| What happened | The message | What to check |
| --- | --- | --- |
| a different part answered at the address | `sensor identification mismatch` | the address plan; the part's id registers, or the ADS1115's configuration read back, did not match |
| a Sensirion word or a DS18B20 scratchpad failed its checksum | `sensor checksum mismatch` | the pull-ups and the wire length; read again, since one corrupted read says nothing about the next |
| a register held a code the datasheet leaves undefined | `sensor register field holds an undefined code` | a part that is not the one the driver expects, or a bus that corrupts bytes |
| the part never reported a finished conversion | `the part did not finish its conversion in time` | the supply, and whether another program drives the same part |
| nothing answered at the address | `nothing answered at 0x62` on a simulated bus, a remote I/O error on a Raspberry Pi | the wiring and the address; [Buses](hal.md#when-it-goes-wrong) lists the bus's own errors |
| the 1-Wire overlay is off | `/sys/bus/w1/devices: No such file or directory (os error 2)` | `dtoverlay=w1-gpio` in `config.txt`, and a reboot |
| a probe went away after discovery | `reading the w1_slave file: No such file or directory (os error 2)` | the lead and its connector; discover again |

How each language hands those over:

| Language | A driver that fails | A kernel thermometer that fails |
| --- | --- | --- |
| Rust | `Err(DriverError::Bus(..))`, `Err(DriverError::Sensor(..))`, `Err(DriverError::Timeout)` | `Err(ThermometerError::Io(..))`, `Err(ThermometerError::Sensor(..))`; `discover` returns the `io::Error` |
| TypeScript | a rejected promise from `init`, `measure`, or `sample` | a thrown `Error` from `discover`, a rejected promise from `read` |
| Python | `PamojaError` | `PamojaError` |
| C# | `PamojaException` | `PamojaException` |

The mistakes that cost an afternoon:

- **Two parts on one address.** The TI parts crowd together: an SHT3x, an OPT3001, and an
  INA219 with A1 on VS+ can all land on `0x44`, and a TMP117, an ADS1115, and an INA219
  with A1 on SDA on `0x48` to `0x4B`. Both parts answer every transfer and garble each
  other, which shows up as checksum or identity errors that come and go. A simulated bus
  keeps only the later of two parts at one address, so the driver meets the wrong part
  every time. Plan the addresses from the table above; `i2cdetect -y 1` should show one
  address per part.
- **The SCD41 takes five seconds.** Periodic measurement produces a result every five
  seconds, so the first `measure` after start-up blocks for about that long, and a loop
  that measures faster waits on the part. Measure every five seconds or more, or ask
  `data_ready` first.
- **The CO2 reads off at altitude, or the temperature reads warm.** The SCD4x corrects its
  CO2 for pressure from the altitude it is given, `set_sensor_altitude`, and subtracts its
  own heating as a temperature offset, 4 °C unless `set_temperature_offset` says
  otherwise. The driver stops measurement to write either, as the datasheet requires, and
  starts it again.
- **The ADS1115 reads its full scale and will not move.** The input is past the range, and
  `clipped` says so. Choose a wider range, but keep every input within 0.3 V of the supply,
  whatever the range.
- **The current reads backwards.** Current is signed by its direction through the shunt,
  so a monitor with its two shunt inputs swapped reports a charge as a discharge. On the
  bench the battery's negative current is real: it is discharging.
- **The INA219's current stops rising.** The shunt voltage saturates at the gain's range,
  so the largest current it can see is that range over the shunt. Tell the driver the real
  shunt and the largest current the load draws. Whatever the 32 V bus range suggests, the
  INA219's inputs take 26 V at most.
- **A DS18B20 reads 85 C.** That is its temperature register's power-on value: the probe
  reset before its conversion finished, usually for want of power. Run it from the 3V3 pin
  on its red lead rather than on parasite power, and check the pull-up.
- **Probes come and go under `/sys/bus/w1/devices`.** The pull-up is missing or too weak
  for the cable. Fit the 4.7 kilohm resistor. The overlay turns the pin's own pull-up on by
  default, but the Raspberry Pi documentation puts that at 50 to 65 kilohms on the older
  models and 33 to 73 on a Pi 4, nowhere near what the DS18B20's circuits use.
- **The light reading lags a change by most of a second.** The OPT3001 integrates for
  800 ms by default. A 100 ms conversion time answers eight times sooner, at up to three
  bits less resolution on the dimmest ranges.

## Where next

<!-- table: next sensors -->
- [Buses](hal.md): The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, one I2C bus and one serial port a program and its drivers share, and delays that sleep or only count.
- [Helpers](kit.md): Plain-language helper math.
- [Device profiles](profile.md): Named, ready-to-run device profiles from plain data or a JSON manifest.
- Beside it: [Hardware](../hardware.md), [Node to dashboard](../boards/walkthrough.md).
- Also in Sensing and actuation: [Actuator drivers](actuators.md), [Your own device](device.md).
<!-- end -->

## Reference

<!-- table: reference sensors -->
- Rust: [`pamoja-sensors`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sensors/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sensors)
- TypeScript: [`@pamoja/sensors`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sensors.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sensors)
- Python: [`pamoja.sensors`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sensors.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sensors)
- C#: [`Pamoja.Sensors`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sensors.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sensors)
- Hardware: [BME280](https://pamoja.molex.cloud/docs/hardware.html#bme280), [DS18B20](https://pamoja.molex.cloud/docs/hardware.html#ds18b20), [INA219](https://pamoja.molex.cloud/docs/hardware.html#ina219), [ADS1115](https://pamoja.molex.cloud/docs/hardware.html#ads1115), [SHT3x-DIS](https://pamoja.molex.cloud/docs/hardware.html#sht3x), [SCD40 and SCD41](https://pamoja.molex.cloud/docs/hardware.html#scd4x), [TMP117](https://pamoja.molex.cloud/docs/hardware.html#tmp117), [HDC1080](https://pamoja.molex.cloud/docs/hardware.html#hdc1080), [OPT3001](https://pamoja.molex.cloud/docs/hardware.html#opt3001), [INA226](https://pamoja.molex.cloud/docs/hardware.html#ina226), [BMP280](https://pamoja.molex.cloud/docs/hardware.html#bmp280)
<!-- end -->
