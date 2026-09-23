# Buses

Every part a node talks to sits on a bus: I2C for the dense breakout sensors, SPI
for displays and radios, a GPIO line for a relay or a button, 1-Wire for a
waterproof thermometer on a long cable. A driver is a conversation over one of
them, in the order and at the pace the part's datasheet lays down: reset, identify,
read the calibration, configure, trigger, wait, read. pamoja's drivers are written
against the `embedded-hal` traits rather than against a board, so the same driver
runs over a microcontroller's peripheral, over the kernel's `/dev/i2c-1` on a
Raspberry Pi, and over a part that is not there at all. [Buses and links](../buses.md)
explains each bus in its own terms, and the board pages wire a part to a
[Raspberry Pi](../boards/raspberry-pi.md), an [ESP32](../boards/esp32.md), or an
[RP2040](../boards/rp2040.md) and run a driver there.

This guide is the I2C bus as a program holds it: one bus that the program and every
driver on it share, whatever is on the other end. That is the kernel's adapter on a
Linux board, simulated parts that answer from their registers, or a script of the
transfers a driver is expected to make. A driver runs the same way over all three, so a
program is written and tested with nothing plugged in and then pointed at `/dev/i2c-1`,
in any of the four languages. The part throughout is a BME280, a temperature, pressure,
and humidity sensor on an inexpensive breakout, whose driver pamoja ships.

## What the example does

It reads a BME280 with nothing plugged in, first through a simulated part and then
through a script of the datasheet's own sequence, and prints what each step saw.

The simulated part holds a real part's calibration and one measurement that part took.
The driver resets it, identifies it, reads the calibration, configures it and measures,
and because the part keeps what was written to it, the configuration reads back off the
bus afterward: humidity oversampling written before the measurement register, and the
part left asleep until a measurement is forced. A simulated bus counts every wait a
driver asks for without sleeping through it, so the program reports the datasheet's
start-up and measurement times and still finishes at once. Then a second part takes the
first one's place, asked to report four degrees at 1013.25 hPa, and the same driver
reads it, which is how a program meets a reading it would otherwise wait on the weather
to produce. A driver pointed at an address no part holds fails with the bus's own reason.

The script is the other half. It lists the transfers the datasheet prescribes, in its
order: the soft reset, the status register once the calibration image has loaded, the
chip id, the two calibration blocks, the three configuration writes in the order the
part requires, then one forced measurement, a status read, and the burst. A transfer the
script does not expect is refused, so a driver that skipped a step or wrote the wrong
register fails here rather than on a bench.

The calibration and the burst are what one real part held, and they compensate to
20.44 C, 848.05 hPa, and 44.65 %.

It proves:

- A driver runs against a part that answers from its registers, with nothing plugged in
  and no transfer sequence written down first.
- The configuration goes out in the datasheet's order, `ctrl_hum` before `ctrl_meas`,
  and leaves the part asleep until a measurement is forced.
- The driver waits the datasheet's 2 ms start-up and 9.3 ms measurement time, which the
  bus counts rather than sleeps through.
- A part reports what it is asked to, and a driver carries on across a part swapped in
  underneath it.
- An address nothing answers at is an error that names the address, never a reading.
- The datasheet sequence is eleven transfers and no other, and it compensates to the same
  reading, to the hundredth, in every language.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example hal" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example hal</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- hal" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- hal</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/hal.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/hal.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- hal" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- hal</code></div>
</div>
<!-- end -->

## Rust

In Rust the bus layer is `pamoja-hal`, and a driver is generic over its `embedded-hal`
traits, taking the bus by value. On a microcontroller that is the HAL's own I2C
peripheral, and in a test a simulated part or a script on its own. The example uses
`pamoja_hal::bus::I2cBus`, the host-side handle a gateway wants: it clones cheaply into
every driver on one bus while the program keeps its own, and `bus.delay()` hands each
driver a delay that sleeps only when real parts are on the other end. `I2cBus::open`
needs the `linux` feature; a single driver that owns the adapter outright can take
`pamoja_hal::linux::i2c` instead. Every call returns a `Result`, and a driver's error is a
`DriverError` whose `Bus` variant carries the bus's own `BusError`.

<!-- snippet: examples/guides/hal.rs#example -->
From [`examples/guides/hal.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/hal.rs):

```rust
use pamoja_hal::bus::I2cBus;
use pamoja_hal::script::{I2cScript, I2cStep};
use pamoja_hal::sim::I2cPart;
use pamoja_sensors::bme280::{
    register, sim, Bme280, Config, CtrlHum, CtrlMeas, Mode, Oversampling, CHIP_ID,
    I2C_ADDRESS_PRIMARY, I2C_ADDRESS_SECONDARY, RESET_WORD,
};
use pamoja_sensors::DriverError;

const BME280: u8 = I2C_ADDRESS_PRIMARY;

// A bus with one part on it: a BME280 that is not there. It holds a real part's
// calibration and one measurement that part took, and it answers from its registers, so
// the driver runs its whole datasheet sequence against it. On a Raspberry Pi the bus is
// `I2cBus::open("/dev/i2c-1")` and nothing after this line changes.
let bus = I2cBus::simulated([sim::part(BME280)]);
let mut sensor = Bme280::i2c(bus.clone(), BME280, bus.delay());

// Reset, identify, calibrate, configure. The datasheet wants ctrl_hum written before
// ctrl_meas, and the part left asleep until a measurement is forced. The part keeps what
// the driver wrote, so the configuration reads back off the bus.
sensor.init()?;
let part: I2cPart = bus.part(BME280).ok_or("no part at the address")?;
let humidity = CtrlHum::from_bits(part.register(register::CTRL_HUM)).humidity;
let ctrl = CtrlMeas::from_bits(part.register(register::CTRL_MEAS));
println!(
    "configured   humidity x{}, temperature x{}, pressure x{}, asleep: {}",
    humidity.factor(),
    ctrl.temperature.factor(),
    ctrl.pressure.factor(),
    ctrl.mode == Mode::Sleep
);

// One forced measurement. The driver waits the datasheet's longest measurement time for
// these settings before it reads, and a simulated bus counts that wait rather than
// sleeping through it.
let reading = sensor.measure()?;
println!(
    "measured     {:.2} C, {:.2} hPa, {:.2} %",
    reading.celsius(),
    reading.hectopascals(),
    reading.relative_humidity_percent()
);
println!(
    "waited       {:.2} ms across {} transfers",
    bus.waited_micros() as f64 / 1000.0,
    bus.transfers()
);

// A part reports whatever it is asked to. Putting one in the first one's place is how a
// program meets a reading it would otherwise wait on the weather for, here a cold store
// at four degrees, and the driver carries on without noticing.
bus.attach(sim::reporting(BME280, 4.0, 1013.25, 80.0))?;
let cold = sensor.measure()?;
println!(
    "cold store   {:.2} C, {:.2} hPa, {:.2} %",
    cold.celsius(),
    cold.hectopascals(),
    cold.relative_humidity_percent()
);

// Nothing answers at the part's other address, and the driver says so rather than
// returning a reading.
match Bme280::i2c(bus.clone(), I2C_ADDRESS_SECONDARY, bus.delay()).init() {
    Ok(()) => println!("absent       a part answered"),
    Err(DriverError::Bus(error)) => println!("absent       {error}"),
    Err(error) => println!("absent       {error}"),
}

// The other half of the bus layer. A script plays one conversation and refuses anything
// else, which proves a driver follows the datasheet rather than merely working: the
// reset, the status once the calibration has loaded, the chip id, the two calibration
// blocks, the three configuration writes in the order the part requires, then one forced
// measurement.
let settings = CtrlMeas {
    temperature: Oversampling::X1,
    pressure: Oversampling::X1,
    mode: Mode::Sleep,
};
let forced = CtrlMeas {
    mode: Mode::Forced,
    ..settings
};
let script = I2cBus::scripted(I2cScript::new([
    I2cStep::write(BME280, [register::RESET, RESET_WORD]),
    I2cStep::write_read(BME280, [register::STATUS], [sim::STATUS_IDLE]),
    I2cStep::write_read(BME280, [register::CHIP_ID], [CHIP_ID]),
    I2cStep::write_read(BME280, [register::CALIB_TEMP_PRESS], sim::CALIBRATION),
    I2cStep::write_read(
        BME280,
        [register::CALIB_HUMIDITY],
        sim::CALIBRATION_HUMIDITY,
    ),
    I2cStep::write(BME280, [register::CONFIG, Config::default().bits()]),
    I2cStep::write(
        BME280,
        [
            register::CTRL_HUM,
            CtrlHum {
                humidity: Oversampling::X1,
            }
            .bits(),
        ],
    ),
    I2cStep::write(BME280, [register::CTRL_MEAS, settings.bits()]),
    I2cStep::write(BME280, [register::CTRL_MEAS, forced.bits()]),
    I2cStep::write_read(BME280, [register::STATUS], [sim::STATUS_IDLE]),
    I2cStep::write_read(BME280, [register::DATA], sim::BURST),
]));
let checked = Bme280::i2c(script.clone(), BME280, script.delay()).measure()?;
println!(
    "datasheet    {:.2} C after {} transfers, {} steps left",
    checked.celsius(),
    script.transfers(),
    script.remaining().unwrap_or_default()
);
```
<!-- end -->

## TypeScript

In TypeScript the bus is `I2cBus` from `@pamoja/hal`, and the driver is `Bme280` from
`@pamoja/sensors`. Transfers on the bus are synchronous and throw on failure. The
driver's `init` and `measure` run on a worker thread, because a measurement waits the
datasheet's time before it reads, and return promises that reject with the reason. Bytes
go in and come out as `Buffer`s, and the setting codes are named on `bme280`, as
`bme280.oversampling.x1` and `bme280.mode.sleep`.

<!-- snippet: bindings/node/guides/hal.ts#example -->
From [`bindings/node/guides/hal.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/hal.ts):

```typescript
import { I2cBus, I2cPart, I2cStep } from '@pamoja/hal'
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
  const part = bus.part(BME280)
  if (!(part instanceof I2cPart)) throw new Error('no part with byte-wide registers at the address')
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
<!-- end -->

## Python

In Python the bus is `I2cBus` from `pamoja.hal`, the driver is `Bme280` from
`pamoja.sensors`, and every call is synchronous. The driver releases the interpreter while
the part answers, so other threads keep running through a measurement. The setting codes
are `IntEnum`s on `bme280`, as `bme280.Oversampling.X1`, and a failure raises
`PamojaError`, which `pamoja.core` exports, with the reason as its message.

<!-- snippet: bindings/python/guides/hal.py#example -->
From [`bindings/python/guides/hal.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/hal.py):

```python
from pamoja.core import PamojaError
from pamoja.hal import I2cBus, I2cStep
from pamoja.sensors import Bme280, Bme280Config, Bme280CtrlMeas, Bme280Measurement, bme280

BME280 = bme280.ADDRESS_PRIMARY


def shown(reading: Bme280Measurement) -> str:
    """A reading the way every line below prints it."""
    return (
        f"{reading.celsius:.2f} C, {reading.hectopascals:.2f} hPa, "
        f"{reading.relative_humidity_percent:.2f} %"
    )


def x(code: int) -> str:
    """An oversampling setting the way a datasheet writes it."""
    return f"x{bme280.oversampling_factor(code)}"


# A bus with one part on it: a BME280 that is not there. It holds a real part's calibration
# and one measurement that part took, and it answers from its registers, so the driver runs
# its whole datasheet sequence against it. On a Raspberry Pi the bus is
# I2cBus.open("/dev/i2c-1") and nothing after this line changes.
bus = I2cBus.simulated([bme280.sim.part(BME280)])
sensor = Bme280(bus, BME280)

# Reset, identify, calibrate, configure. The datasheet wants ctrl_hum written before
# ctrl_meas, and the part left asleep until a measurement is forced. The part keeps what the
# driver wrote, so the configuration reads back off the bus.
sensor.init()
part = bus.part(BME280)
humidity = bme280.ctrl_hum_from_bits(part.register(bme280.REGISTER_CTRL_HUM))
ctrl = bme280.ctrl_meas_from_bits(part.register(bme280.REGISTER_CTRL_MEAS))
asleep = ctrl.mode == bme280.Mode.SLEEP
print(
    f"configured   humidity {x(humidity)}, temperature {x(ctrl.temperature)}, "
    f"pressure {x(ctrl.pressure)}, asleep: {str(asleep).lower()}"
)

# One forced measurement. The driver waits the datasheet's longest measurement time for
# these settings before it reads, and a simulated bus counts that wait rather than sleeping
# through it.
reading = sensor.measure()
print(f"measured     {shown(reading)}")
print(f"waited       {bus.waited_micros / 1000:.2f} ms across {bus.transfers} transfers")
waited = bus.waited_micros

# A part reports whatever it is asked to. Putting one in the first one's place is how a
# program meets a reading it would otherwise wait on the weather for, here a cold store at
# four degrees, and the driver carries on without noticing.
bus.attach(bme280.sim.reporting(BME280, 4.0, 1013.25, 80.0))
cold = sensor.measure()
print(f"cold store   {shown(cold)}")

# Nothing answers at the part's other address, and the driver says so rather than
# returning a reading.
try:
    Bme280(bus, bme280.ADDRESS_SECONDARY).init()
    print("absent       a part answered")
except PamojaError as error:
    print(f"absent       {error}")

# The other half of the bus layer. A script plays one conversation and refuses anything
# else, which proves a driver follows the datasheet rather than merely working: the reset,
# the status once the calibration has loaded, the chip id, the two calibration blocks, the
# three configuration writes in the order the part requires, then one forced measurement.
x1 = bme280.Oversampling.X1
settings = Bme280CtrlMeas(temperature=x1, pressure=x1, mode=bme280.Mode.SLEEP)
forced = Bme280CtrlMeas(temperature=x1, pressure=x1, mode=bme280.Mode.FORCED)
idle = bytes([bme280.sim.STATUS_IDLE])
script = I2cBus.scripted([
    I2cStep.write(BME280, bytes([bme280.REGISTER_RESET, bme280.RESET_WORD])),
    I2cStep.write_read(BME280, bytes([bme280.REGISTER_STATUS]), idle),
    I2cStep.write_read(BME280, bytes([bme280.REGISTER_CHIP_ID]), bytes([bme280.CHIP_ID])),
    I2cStep.write_read(
        BME280, bytes([bme280.REGISTER_CALIB_TEMP_PRESS]), bme280.sim.calibration()
    ),
    I2cStep.write_read(
        BME280, bytes([bme280.REGISTER_CALIB_HUMIDITY]), bme280.sim.calibration_humidity()
    ),
    I2cStep.write(BME280, bytes([bme280.REGISTER_CONFIG, bme280.config_bits(Bme280Config())])),
    I2cStep.write(BME280, bytes([bme280.REGISTER_CTRL_HUM, bme280.ctrl_hum_bits(x1)])),
    I2cStep.write(BME280, bytes([bme280.REGISTER_CTRL_MEAS, bme280.ctrl_meas_bits(settings)])),
    I2cStep.write(BME280, bytes([bme280.REGISTER_CTRL_MEAS, bme280.ctrl_meas_bits(forced)])),
    I2cStep.write_read(BME280, bytes([bme280.REGISTER_STATUS]), idle),
    I2cStep.write_read(BME280, bytes([bme280.REGISTER_DATA]), bme280.sim.burst()),
])
checked = Bme280(script, BME280).measure()
print(
    f"datasheet    {checked.celsius:.2f} C after {script.transfers} transfers, "
    f"{script.remaining} steps left"
)
```
<!-- end -->

## C#

In C# the bus is `I2cBus` in `Pamoja.Hal`, and the driver is `Bme280` in
`Pamoja.Sensors`, whose static members are the part's datasheet: its addresses,
`Bme280.Register`, the setting enums, and `Bme280.Sim`. Buses, parts, and drivers hold
native handles, so they are `IDisposable` and the example takes them with `using`; a
driver keeps its own share of the bus, so the bus may be disposed first. A failure throws
`PamojaException` with the reason as its message, and opening an adapter anywhere but
Linux throws `PlatformNotSupportedException`.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs):

```csharp
const byte Part = Bme280.AddressPrimary;
static string Fixed(float value) => value.ToString("F2", CultureInfo.InvariantCulture);
static string Shown(Bme280Measurement reading) =>
    $"{Fixed(reading.Celsius)} C, {Fixed(reading.Hectopascals)} hPa, {Fixed(reading.RelativeHumidityPercent)} %";
static string X(Bme280.Oversampling oversampling) => $"x{Bme280.OversamplingFactor(oversampling)}";

// A bus with one part on it: a BME280 that is not there. It holds a real part's
// calibration and one measurement that part took, and it answers from its registers,
// so the driver runs its whole datasheet sequence against it. On a Raspberry Pi the
// bus is I2cBus.Open("/dev/i2c-1") and nothing after this line changes.
using I2cPart simulated = Bme280.Sim.Part(Part);
using I2cBus bus = I2cBus.Simulated(simulated);
using var sensor = new Bme280(bus, Part);

// Reset, identify, calibrate, configure. The datasheet wants ctrl_hum written before
// ctrl_meas, and the part left asleep until a measurement is forced. The part keeps
// what the driver wrote, so the configuration reads back off the bus.
sensor.Init();
using I2cPart part = bus.Part<I2cPart>(Part)!;
Bme280.Oversampling humidity = Bme280.CtrlHumFromBits(part.Register(Bme280.Register.CtrlHum));
Bme280CtrlMeas ctrl = Bme280.CtrlMeasFromBits(part.Register(Bme280.Register.CtrlMeas));
bool asleep = ctrl.Mode == Bme280.Mode.Sleep;
Console.WriteLine(
    $"configured   humidity {X(humidity)}, temperature {X(ctrl.Temperature)}, " +
    $"pressure {X(ctrl.Pressure)}, asleep: {asleep.ToString().ToLowerInvariant()}");

// One forced measurement. The driver waits the datasheet's longest measurement time
// for these settings before it reads, and a simulated bus counts that wait rather
// than sleeping through it.
Bme280Measurement reading = sensor.Measure();
Console.WriteLine($"measured     {Shown(reading)}");
ulong waited = bus.WaitedMicros;
string waitedMs = (waited / 1000.0).ToString("F2", CultureInfo.InvariantCulture);
Console.WriteLine($"waited       {waitedMs} ms across {bus.Transfers} transfers");

// A part reports whatever it is asked to. Putting one in the first one's place is how
// a program meets a reading it would otherwise wait on the weather for, here a cold
// store at four degrees, and the driver carries on without noticing.
using (I2cPart chilled = Bme280.Sim.Reporting(Part, 4.0f, 1013.25f, 80.0f))
{
    bus.Attach(chilled);
}

Bme280Measurement cold = sensor.Measure();
Console.WriteLine($"cold store   {Shown(cold)}");

// Nothing answers at the part's other address, and the driver says so rather than
// returning a reading.
try
{
    using var absent = new Bme280(bus, Bme280.AddressSecondary);
    absent.Init();
    Console.WriteLine("absent       a part answered");
}
catch (PamojaException error)
{
    Console.WriteLine($"absent       {error.Message}");
}

// The other half of the bus layer. A script plays one conversation and refuses
// anything else, which proves a driver follows the datasheet rather than merely
// working: the reset, the status once the calibration has loaded, the chip id, the
// two calibration blocks, the three configuration writes in the order the part
// requires, then one forced measurement.
const Bme280.Oversampling X1 = Bme280.Oversampling.X1;
var settings = new Bme280CtrlMeas(X1, X1, Bme280.Mode.Sleep);
var forced = settings with { Mode = Bme280.Mode.Forced };
var resetState = new Bme280Config(Bme280.Standby.Ms0_5, Bme280.Filter.Off, Spi3Wire: false);
byte[] idle = [Bme280.Sim.StatusIdle];
using I2cBus script = I2cBus.Scripted(
    I2cStep.Write(Part, [Bme280.Register.Reset, Bme280.ResetWord]),
    I2cStep.WriteRead(Part, [Bme280.Register.Status], idle),
    I2cStep.WriteRead(Part, [Bme280.Register.ChipId], [Bme280.ChipId]),
    I2cStep.WriteRead(Part, [Bme280.Register.CalibTempPress], Bme280.Sim.Calibration()),
    I2cStep.WriteRead(Part, [Bme280.Register.CalibHumidity], Bme280.Sim.CalibrationHumidity()),
    I2cStep.Write(Part, [Bme280.Register.Config, Bme280.ConfigBits(resetState)]),
    I2cStep.Write(Part, [Bme280.Register.CtrlHum, Bme280.CtrlHumBits(X1)]),
    I2cStep.Write(Part, [Bme280.Register.CtrlMeas, Bme280.CtrlMeasBits(settings)]),
    I2cStep.Write(Part, [Bme280.Register.CtrlMeas, Bme280.CtrlMeasBits(forced)]),
    I2cStep.WriteRead(Part, [Bme280.Register.Status], idle),
    I2cStep.WriteRead(Part, [Bme280.Register.Data], Bme280.Sim.Burst()));
using var datasheet = new Bme280(script, Part);
Bme280Measurement checkedReading = datasheet.Measure();
Console.WriteLine(
    $"datasheet    {Fixed(checkedReading.Celsius)} C after {script.Transfers} transfers, " +
    $"{script.Remaining ?? 0} steps left");
```
<!-- end -->

## On a board

The same driver on a Raspberry Pi, reading a real BME280 every two seconds. Only the
line that opens the bus changes. Wire the breakout to the header's I2C pins:

| BME280 breakout | Raspberry Pi |
| --- | --- |
| VIN | a 3V3 pin |
| GND | a ground pin |
| SDA | GPIO2 |
| SCL | GPIO3 |

GPIO2 and GPIO3 carry the board's own pull-up resistors, so the breakout needs none.
[Turn the I2C interface on](../boards/raspberry-pi.md#turning-the-buses-on), make sure the
account is [in the `i2c` group](../boards/raspberry-pi.md#permissions), and run
`i2cdetect -y 1` before anything else: it prints every address that answers, and a part
missing from that grid is a wiring problem, not a software one. The
[Raspberry Pi page](../boards/raspberry-pi.md#reading-the-sensor) runs the same program as
the first of four.

### Rust

<!-- snippet: examples/boards/raspberry-pi/src/main.rs#example -->
From [`examples/boards/raspberry-pi/src/main.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/main.rs):

```rust
use pamoja_hal::bus::I2cBus;
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

fn main() -> Result<(), Box<dyn Error>> {
    // The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
    let bus = I2cBus::open("/dev/i2c-1")?;

    // The driver runs the datasheet's sequence over that bus: reset, identify, read the
    // calibration, configure, and then a forced measurement per read.
    let mut sensor = Bme280::i2c(bus.clone(), I2C_ADDRESS_PRIMARY, bus.delay());
    sensor.init()?;

    loop {
        let measurement = sensor.measure()?;
        println!(
            "{:.2} C, {:.2} hPa, {:.2} % humidity",
            measurement.celsius(),
            measurement.hectopascals(),
            measurement.relative_humidity_percent()
        );
        thread::sleep(Duration::from_secs(2));
    }
}
```
<!-- end -->

```sh
cd examples/boards/raspberry-pi
cargo run --release
```

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/sensor.ts#example -->
From [`bindings/node/boards/raspberry-pi/sensor.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/sensor.ts):

```typescript
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
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/sensor.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/sensor.py#example -->
From [`bindings/python/boards/raspberry_pi/sensor.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/sensor.py):

```python
import time

from pamoja.hal import I2cBus
from pamoja.sensors import Bme280, bme280


def main() -> None:
    # The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
    bus = I2cBus.open("/dev/i2c-1")

    # The driver runs the datasheet's sequence over that bus: reset, identify, read the
    # calibration, configure, and then a forced measurement per read.
    sensor = Bme280(bus, bme280.ADDRESS_PRIMARY)
    sensor.init()

    while True:
        reading = sensor.measure()
        print(
            f"{reading.celsius:.2f} C, {reading.hectopascals:.2f} hPa, "
            f"{reading.relative_humidity_percent:.2f} % humidity"
        )
        time.sleep(2)
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/sensor.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Sensor.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Sensor.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Sensor.cs):

```csharp
using System.Globalization;

using Pamoja.Hal;
using Pamoja.Sensors;

namespace Boards.RaspberryPi;

/// <summary>
/// The first program on a Raspberry Pi: a BME280 on the 40-pin header's I2C bus, read through
/// the driver pamoja ships, printed every two seconds. Wire the BME280's SDA to GPIO2, its SCL
/// to GPIO3, VIN to 3V3, and GND to ground, and turn the I2C interface on.
/// </summary>
public static class Sensor
{
    /// <summary>Runs until the process is stopped.</summary>
    public static void Run()
    {
        // The header's I2C bus is a file once the interface is on: /dev/i2c-1 on every model.
        using I2cBus bus = I2cBus.Open("/dev/i2c-1");

        // The driver runs the datasheet's sequence over that bus: reset, identify, read the
        // calibration, configure, and then a forced measurement per read.
        using var sensor = new Bme280(bus, Bme280.AddressPrimary);
        sensor.Init();

        while (true)
        {
            Bme280Measurement reading = sensor.Measure();
            Console.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"{reading.Celsius:F2} C, {reading.Hectopascals:F2} hPa, {reading.RelativeHumidityPercent:F2} % humidity"));
            Thread.Sleep(TimeSpan.FromSeconds(2));
        }
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/sensor
```

## Values at a glance

**What answers on a bus** decides what a transfer does and whether a driver's wait is
spent:

| Kind | Made with | What answers | A driver's wait |
| --- | --- | --- | --- |
| Adapter | `open(path)`, such as `/dev/i2c-1` | the parts wired to the bus | sleeps the process |
| Simulated | `simulated(parts)` | each part at its own address, in the shape of its datasheet; nothing anywhere else | counted, not slept |
| Scripted | `scripted(steps)` | the next step, when the transfer matches it; a refusal otherwise | counted, not slept |

**A simulated part comes in three kinds**, because parts talk in three ways. Every driver's
simulated part is one of them, and a bus holds any mix:

| Kind | How it talks | Parts built on it |
| --- | --- | --- |
| `I2cPart` | a register address, then bytes from 256 byte-wide registers | BME280, BMP280 |
| `WordPart` | a pointer byte, then 16-bit registers sent most significant byte first, with bits the part keeps for itself marked read-only | TMP117, OPT3001, HDC1080, INA219, INA226, ADS1115 |
| `CommandPart` | a command and its arguments, then a read that takes the reply that command left, once | SHT3x, SCD4x |

**The BME280** answers at `0x76` with its SDO pin low and `0x77` with it high. The
registers a driver touches, in the order it first reaches them:

| Register | Address | What it holds |
| --- | --- | --- |
| `reset` | `0xE0` | writing `0xB6` restarts the part |
| `status` | `0xF3` | whether a measurement is running, and whether the calibration image is still loading |
| `chip_id` | `0xD0` | `0x60` on a BME280, `0x58` on a BMP280 |
| calibration | `0x88` and `0xE1` | 26 bytes and 7 bytes, read once at start-up |
| `config` | `0xF5` | the normal-mode standby period, the IIR filter, and 3-wire SPI |
| `ctrl_hum` | `0xF2` | humidity oversampling, which takes effect at the next `ctrl_meas` write |
| `ctrl_meas` | `0xF4` | temperature and pressure oversampling, and the power mode |
| data | `0xF7` | 8 bytes: pressure, temperature, and humidity |

**Oversampling** averages samples for each measurement: code 0 skips it, and codes 1 to 5
average 1, 2, 4, 8, and 16. More samples mean less noise and a longer wait, which the
driver takes from the datasheet's longest measurement time for the settings in use:

| Settings | Temperature | Pressure | Humidity | Longest measurement | Typical |
| --- | --- | --- | --- | --- | --- |
| each measured once | x1 | x1 | x1 | 9.3 ms | 8 ms |
| humidity left out | x1 | x1 | skipped | 6.425 ms | 5.5 ms |
| pressure averaged over 16 | x2 | x16 | x1 | 46.1 ms | 40 ms |
| each averaged over 16 | x16 | x16 | x16 | 112.8 ms | 98 ms |

**The IIR filter** smooths pressure and temperature across measurements, never humidity.
Codes 0 to 4 give off, 2, 4, 8, and 16. The **standby period** only matters in normal
mode, which the driver does not use: it measures on demand in forced mode, and the part
sleeps in between.

**The same calls in each language:**

| To | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| open the kernel's adapter | `I2cBus::open(path)` | `I2cBus.open(path)` | `I2cBus.open(path)` | `I2cBus.Open(path)` |
| put parts on a bus | `I2cBus::simulated(parts)` | `I2cBus.simulated(parts)` | `I2cBus.simulated(parts)` | `I2cBus.Simulated(parts)` |
| swap a part in | `bus.attach(part)` | `bus.attach(part)` | `bus.attach(part)` | `bus.Attach(part)` |
| play a script | `I2cBus::scripted(I2cScript::new(steps))` | `I2cBus.scripted(steps)` | `I2cBus.scripted(steps)` | `I2cBus.Scripted(steps)` |
| read a register | `bus.write_read(a, bytes, &mut out)` | `bus.writeRead(a, bytes, n)` | `bus.write_read(a, data, n)` | `bus.WriteRead(a, bytes, n)` |
| see what a part holds | `bus.part::<I2cPart>(a)` | `bus.part(a)` | `bus.part(a)` | `bus.Part<I2cPart>(a)` |
| count transfers and waits | `transfers()`, `waited_micros()` | `transfers`, `waitedMicros` | `transfers`, `waited_micros` | `Transfers`, `WaitedMicros` |
| see what a script has left | `remaining()` | `remaining` | `remaining` | `Remaining` |
| build a BME280 driver | `Bme280::i2c(bus.clone(), a, bus.delay())` | `new Bme280(bus, a)` | `Bme280(bus, a)` | `new Bme280(bus, a)` |
| stand a BME280 up | `sim::part(a)`, `sim::reporting(a, c, hpa, rh)` | `bme280.sim.part(a)`, `bme280.sim.reporting(...)` | `bme280.sim.part(a)`, `bme280.sim.reporting(...)` | `Bme280.Sim.Part(a)`, `Bme280.Sim.Reporting(...)` |

A step can also fail on purpose, the way a missing or busy part does. The faults are the
ones `embedded-hal` names: no acknowledge of the address, of a data byte, or of either;
a bus error; lost arbitration; an overrun; and anything else.

## When it goes wrong

What a bus or a driver refuses, and what it says:

| What happened | The message | What to check |
| --- | --- | --- |
| an adapter opened anywhere but Linux | `an I2C adapter is opened through the kernel's i2c-dev interface, which only Linux has` | run it on the board, or run it on a simulated bus |
| the I2C interface is off | `/dev/i2c-1: No such file or directory (os error 2)` | [turn it on](../boards/raspberry-pi.md#turning-the-buses-on) and reboot |
| the account may not open the bus | `/dev/i2c-1: Permission denied (os error 13)` | [add the account to the `i2c` group](../boards/raspberry-pi.md#permissions) |
| nothing wired answered on a Raspberry Pi | `/dev/i2c-1: EREMOTEIO: Remote I/O error` | the wiring, the power, and the address; `i2cdetect -y 1` shows what answers |
| nothing on a simulated bus holds the address | `nothing answered at 0x77` | the part's address, or attach a part there |
| a part answered that is not a BME280 | `sensor identification mismatch` | the chip id; `0x58` is a BMP280, which has no humidity |
| a kernel driver already holds the part | `/dev/i2c-1: EBUSY: Device or resource busy` | a `dtoverlay=i2c-sensor` line in `config.txt`, which gives the part to the kernel's own driver; remove it and reboot |
| the part stayed busy past the datasheet's time | `the part did not finish its conversion in time` | the supply, and whether another program is driving the same part |
| a script met a transfer it did not expect | `i2c script step 3: expected ..., the driver issued ...` | the step the message names, against the datasheet |

How each language hands those over:

| Language | A transfer or a driver call that fails | Opening an adapter off Linux |
| --- | --- | --- |
| Rust | `Err(DriverError::Bus(BusError))`, `Err(DriverError::Sensor(..))`, `Err(DriverError::Timeout)` | `Err(OpenError::Unsupported)` |
| TypeScript | a thrown `Error`, or a rejected promise from `init` and `measure` | a thrown `Error` |
| Python | `PamojaError` | `PamojaError` |
| C# | `PamojaException` | `PlatformNotSupportedException` |

A Raspberry Pi's I2C controllers report a part that did not acknowledge as `EREMOTEIO`,
which the bus treats as a missing acknowledge, the same as a simulated bus with nothing at
the address.

The mistakes that cost an afternoon:

- **Nothing answers at all.** SDA and SCL are swapped, or the breakout has no power.
  `i2cdetect -y 1` prints an empty grid until they are right.
- **The part answers at the other address.** Breakouts differ in which way they tie SDO,
  so one board answers at `0x76` and another at `0x77`. `i2cdetect -y 1` shows which.
- **The chip id says `0x58`.** The part is a BMP280, which comes on breakouts that look
  the same: it measures pressure and temperature and has no humidity at all. Its driver
  is in the [sensor drivers guide](sensors.md).
- **Two parts share an address.** Both answer every transfer and garble each other's
  replies. Move one with its address jumper.
- **The breakout is powered from 5 V.** The header's pins are 3.3 V logic, and a
  breakout whose pull-ups go to its own supply puts that supply on SDA and SCL. Power it
  from a 3V3 pin, as the [Raspberry Pi page](../boards/raspberry-pi.md#the-header)
  explains.

## Where next

<!-- table: next hal -->
- [Sensor drivers](sensors.md): Datasheet-anchored drivers for eleven parts, from every language.
- [Actuator drivers](actuators.md): A PCA9685 driver for servos, LEDs, and valves in every language, and stepper coil sequencing.
- [Your own device](device.md): A sensor and an actuator pamoja has never heard of, written against the core traits, run against a rule, and published with nothing plugged in.
- Beside it: [Buses and links](../buses.md), [Raspberry Pi](../boards/raspberry-pi.md), [RP2040](../boards/rp2040.md).
- Also in Field I/O: [Serial framing](serial.md), [Modbus RTU](modbus.md), [CAN and J1939](can.md), [I2C, SPI, and GPIO](gpio.md).
<!-- end -->

## Reference

<!-- table: reference hal -->
- Rust: [`pamoja-hal`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_hal/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-hal)
- TypeScript: [`@pamoja/hal`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_hal.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-hal)
- Python: [`pamoja.hal`](https://pamoja.molex.cloud/docs/reference/python/pamoja/hal.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-hal)
- C#: [`Pamoja.Hal`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Hal.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-hal)
<!-- end -->
