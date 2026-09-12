# Buses

Every part a node talks to sits on a bus: I2C for the dense breakout sensors, SPI
for displays and radios, a GPIO line for a relay or a button, 1-Wire for a
waterproof thermometer on a long cable. A driver is a conversation over one of
them, in the order and at the pace the part's datasheet lays down: reset, identify,
read the calibration, configure, trigger, wait, read. pamoja's drivers are written
against the `embedded-hal` traits rather than against a board, so the same driver
runs over a microcontroller's peripheral, over the kernel's `/dev/i2c-1` on a
Raspberry Pi, and over a script of what the part would have sent, with nothing
plugged in. [Buses and links](../buses.md) explains each bus in its own terms,
and the board pages wire a part to a [Raspberry Pi](../boards/raspberry-pi.md),
an [ESP32](../boards/esp32.md), or an [RP2040](../boards/rp2040.md) and run a
driver there.

In Rust the bus layer is `pamoja-hal`: the traits every driver takes, a bit-banged
1-Wire bus over any pin, the Linux backends, and the scripted buses that play a
part's side of a conversation so a driver is tested against the datasheet's own
transfer sequence. Each shipped part has a driver on top of its decoder, and a
part pamoja has never heard of takes the same traits, as the [own device
guide](device.md) shows.

In TypeScript, Python, and C# the bus is the host's own library: `i2c-bus` on
Node, `smbus2` on Python, `System.Device.I2c` on .NET. pamoja carries the
datasheet's decoding, the calibration and the compensation, and the program
carries the conversation, which is the same sequence the Rust driver runs. The
examples here stand a scripted bus in for those libraries, in their shape, so the
program reads the way it does on a gateway and still runs in a test.

## What the example does

It reads a BME280 once. The bus is a script of what the part answers, in the
order the datasheet lists: the soft reset, the status register once the
calibration image has loaded, the chip id, the two calibration blocks, the three
configuration writes in the order the part requires, then one forced measurement
with the datasheet's maximum measurement time before the status read and the
burst read. In Rust the `Bme280` driver runs that sequence; in the other three
languages the program does, through a bus shaped like the host library, with
pamoja compensating the burst against the calibration.

The calibration bytes and the burst are what a particular part holds, and they
decode to 20.44 C, 848.05 hPa, and 44.65 %. A transfer the script does not expect
is refused, so a program that skipped a step or wrote the wrong register would
fail here rather than on a bench.

It proves:

- The chip id, the calibration, and the configuration are read and written
  exactly as the datasheet orders them, and once: `ctrl_hum` before `ctrl_meas`,
  the part left asleep until the forced measurement.
- A forced measurement waits the datasheet's maximum for the oversampling in use,
  then confirms the part is idle before the burst read.
- The same calibration and burst compensate to the same temperature, pressure,
  and humidity in every language, to the hundredth.
- Eleven transfers were made and no other, because the scripted bus refuses
  anything the datasheet did not list.

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

<!-- snippet: examples/guides/hal.rs#example -->
From [`examples/guides/hal.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/hal.rs):

```rust
use pamoja_core::Sensor;
use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

// On a Raspberry Pi the bus is `pamoja_hal::linux::i2c("/dev/i2c-1")` and nothing
// below changes. Here a script plays the part's side: what a BME280 answers to the
// reset, the status and chip id reads, the two calibration reads, the three
// configuration writes, and one forced measurement, in the order the datasheet
// lists them. A transfer the script does not expect is refused.
const BME280: u8 = I2C_ADDRESS_PRIMARY;
let calibration_a = [
    0x45, 0x6F, 0x6F, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6A, 0xD6, 0xD0, 0x0B, 0x4E, 0x1E, 0x88,
    0xFF, 0xF9, 0xFF, 0xAC, 0x26, 0x0A, 0xD8, 0xBD, 0x10, 0x00, 0x4B,
];
let calibration_b = [0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1E];
let burst = [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x75, 0x30];
let bus = I2cScript::new([
    I2cStep::write(BME280, [0xE0, 0xB6]),
    I2cStep::write_read(BME280, [0xF3], [0x00]),
    I2cStep::write_read(BME280, [0xD0], [0x60]),
    I2cStep::write_read(BME280, [0x88], calibration_a),
    I2cStep::write_read(BME280, [0xE1], calibration_b),
    I2cStep::write(BME280, [0xF5, 0x00]),
    I2cStep::write(BME280, [0xF2, 0x01]),
    I2cStep::write(BME280, [0xF4, 0x24]),
    I2cStep::write(BME280, [0xF4, 0x25]),
    I2cStep::write_read(BME280, [0xF3], [0x00]),
    I2cStep::write_read(BME280, [0xF7], burst),
]);

// The driver runs that sequence, checks the chip id, and keeps the calibration.
let mut sensor = Bme280::i2c(bus, BME280, DelayLog::new());
sensor.init().expect("the part answers");
let calibrated = sensor.calibration().is_some();
println!("calibration  read once: {calibrated}");

// A reading is the whole measurement, compensated with that calibration.
let measurement = block_on(sensor.read()).expect("a measurement");
let celsius = measurement.celsius();
let hectopascals = measurement.hectopascals();
let humidity = measurement.relative_humidity_percent();
println!("measured     {celsius:.2} C, {hectopascals:.2} hPa, {humidity:.2} %");

// The script is spent: every transfer the datasheet lists was made, and no other.
let (registers, delay) = sensor.release();
let bus = registers.release();
let transfers = bus.consumed();
let unexpected = !bus.done();
println!("bus          {transfers} transfers, unexpected: {unexpected}");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/hal.ts#example -->
From [`bindings/node/guides/hal.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/hal.ts):

```typescript
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
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/hal.py#example -->
From [`bindings/python/guides/hal.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/hal.py):

```python
import time

from pamoja.sensors import bme280

BME280 = bme280.ADDRESS_PRIMARY


class ScriptedBus:
    """A bus with the two calls this conversation needs, in the shape of smbus2.

    On a gateway ``SMBus(1)`` gives the real one and nothing below changes. This one
    answers from a script of what a BME280 sends, in the order the datasheet lists,
    and refuses any transfer that is not the next one.
    """

    def __init__(self, script):
        self.script = script
        self.transfers = 0

    def write_byte_data(self, address, register, value):
        step = self._next(address, register)
        if step.get("write") != value:
            raise ValueError(f"unexpected write of {value:#04x}")

    def read_i2c_block_data(self, address, register, length):
        step = self._next(address, register)
        reply = step.get("reply")
        if reply is None or len(reply) != length:
            raise ValueError(f"unexpected read of {length} bytes")
        return bytes(reply)

    @property
    def done(self):
        return self.transfers == len(self.script)

    def _next(self, address, register):
        step = self.script[self.transfers] if self.transfers < len(self.script) else None
        if address != BME280 or step is None or step["register"] != register:
            raise ValueError(f"unexpected transfer at register {register:#04x}")
        self.transfers += 1
        return step


CALIBRATION_A = bytes([
    0x45, 0x6F, 0x6F, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6A, 0xD6, 0xD0, 0x0B, 0x4E, 0x1E,
    0x88, 0xFF, 0xF9, 0xFF, 0xAC, 0x26, 0x0A, 0xD8, 0xBD, 0x10, 0x00, 0x4B,
])
CALIBRATION_B = bytes([0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1E])
BURST = bytes([0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x75, 0x30])


def main():
    bus = ScriptedBus([
        {"register": 0xE0, "write": 0xB6},
        {"register": 0xF3, "reply": [0x00]},
        {"register": 0xD0, "reply": [bme280.CHIP_ID]},
        {"register": 0x88, "reply": CALIBRATION_A},
        {"register": 0xE1, "reply": CALIBRATION_B},
        {"register": 0xF5, "write": 0x00},
        {"register": 0xF2, "write": 0x01},
        {"register": 0xF4, "write": 0x24},
        {"register": 0xF4, "write": 0x25},
        {"register": 0xF3, "reply": [0x00]},
        {"register": 0xF7, "reply": BURST},
    ])

    # The datasheet's start-up: the soft reset word, its 2 ms start-up time, and the
    # status register, whose low bit clears once the calibration image has loaded.
    bus.write_byte_data(BME280, 0xE0, 0xB6)
    time.sleep(0.002)
    bus.read_i2c_block_data(BME280, 0xF3, 1)

    # The chip id says it is a BME280, and the two calibration blocks are read once.
    chip_id = bus.read_i2c_block_data(BME280, 0xD0, 1)[0]
    temp_press = bus.read_i2c_block_data(BME280, 0x88, 26)
    humidity = bus.read_i2c_block_data(BME280, 0xE1, 7)
    calibration = bme280.calibration(temp_press, humidity)
    print(f"calibration  read once: {str(chip_id == bme280.CHIP_ID).lower()}")

    # config, ctrl_hum, then ctrl_meas, in that order because ctrl_hum only takes
    # effect after the ctrl_meas write: every measurement at oversampling x1, asleep.
    bus.write_byte_data(BME280, 0xF5, 0x00)
    bus.write_byte_data(BME280, 0xF2, 0x01)
    bus.write_byte_data(BME280, 0xF4, 0x24)

    # One forced measurement: the mode bits, the datasheet's 9.3 ms maximum for these
    # settings, the status read that confirms the part is idle, and the burst read.
    bus.write_byte_data(BME280, 0xF4, 0x25)
    time.sleep(0.010)
    bus.read_i2c_block_data(BME280, 0xF3, 1)
    burst = bus.read_i2c_block_data(BME280, 0xF7, 8)
    measurement = calibration.compensate(burst)
    celsius = measurement.celsius
    hectopascals = measurement.hectopascals
    humidity_percent = measurement.relative_humidity_percent
    print(f"measured     {celsius:.2f} C, {hectopascals:.2f} hPa, {humidity_percent:.2f} %")

    # The script is spent: every transfer the datasheet lists was made, and no other.
    print(f"bus          {bus.transfers} transfers, unexpected: {str(not bus.done).lower()}")
    return measurement, bus.transfers, bus.done


measurement, transfers, done = main()
```
<!-- end -->

## C#

The scripted device:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs#parts -->
From [`bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs):

```csharp
/// <summary>One transfer the script expects: a register written or read.</summary>
private sealed record Step(byte Register, byte? Write = null, byte[]? Reply = null);

/// <summary>
/// A device with the two calls this conversation needs, in the shape of
/// System.Device.I2c's I2cDevice: on a gateway <c>I2cDevice.Create</c> gives the
/// real one and nothing below changes. This one answers from a script of what a
/// BME280 sends, in the order the datasheet lists, and refuses any other transfer.
/// </summary>
private sealed class ScriptedDevice(Step[] script)
{
    public int Transfers { get; private set; }

    public bool Done => Transfers == script.Length;

    public void Write(ReadOnlySpan<byte> bytes)
    {
        Step step = Next(bytes[0]);
        if (bytes.Length != 2 || step.Write != bytes[1])
        {
            throw new InvalidOperationException($"unexpected write of 0x{bytes[1]:X2}");
        }
    }

    public void WriteRead(ReadOnlySpan<byte> write, Span<byte> read)
    {
        Step step = Next(write[0]);
        if (step.Reply is null || step.Reply.Length != read.Length)
        {
            throw new InvalidOperationException($"unexpected read of {read.Length} bytes");
        }

        step.Reply.CopyTo(read);
    }

    private Step Next(byte register)
    {
        if (Transfers >= script.Length || script[Transfers].Register != register)
        {
            throw new InvalidOperationException($"unexpected transfer at register 0x{register:X2}");
        }

        return script[Transfers++];
    }
}
```
<!-- end -->

The conversation:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/HalGuide.cs):

```csharp
var bus = new ScriptedDevice(
[
    new Step(0xE0, Write: 0xB6),
    new Step(0xF3, Reply: [0x00]),
    new Step(0xD0, Reply: [Bme280.ChipId]),
    new Step(0x88, Reply: CalibrationA),
    new Step(0xE1, Reply: CalibrationB),
    new Step(0xF5, Write: 0x00),
    new Step(0xF2, Write: 0x01),
    new Step(0xF4, Write: 0x24),
    new Step(0xF4, Write: 0x25),
    new Step(0xF3, Reply: [0x00]),
    new Step(0xF7, Reply: Burst),
]);

// The datasheet's start-up: the soft reset word, its 2 ms start-up time, and
// the status register, whose low bit clears once the calibration image loaded.
bus.Write([0xE0, 0xB6]);
await Task.Delay(2);
byte[] status = new byte[1];
bus.WriteRead([0xF3], status);

// The chip id says it is a BME280, and the two calibration blocks are read once.
byte[] id = new byte[1];
bus.WriteRead([0xD0], id);
byte[] tempPress = new byte[26];
byte[] humidity = new byte[7];
bus.WriteRead([0x88], tempPress);
bus.WriteRead([0xE1], humidity);
using var calibration = new Bme280Calibration(tempPress, humidity);
Console.WriteLine($"calibration  read once: {(id[0] == Bme280.ChipId).ToString().ToLowerInvariant()}");

// config, ctrl_hum, then ctrl_meas, in that order because ctrl_hum only takes
// effect after the ctrl_meas write: every measurement at oversampling x1, asleep.
bus.Write([0xF5, 0x00]);
bus.Write([0xF2, 0x01]);
bus.Write([0xF4, 0x24]);

// One forced measurement: the mode bits, the datasheet's 9.3 ms maximum for
// these settings, the status read that confirms the part is idle, the burst.
bus.Write([0xF4, 0x25]);
await Task.Delay(10);
bus.WriteRead([0xF3], status);
byte[] burst = new byte[8];
bus.WriteRead([0xF7], burst);
Bme280Measurement measurement = calibration.Compensate(burst);
string celsius = measurement.Celsius.ToString("F2", CultureInfo.InvariantCulture);
string hectopascals = measurement.Hectopascals.ToString("F2", CultureInfo.InvariantCulture);
string humidityPercent = measurement.RelativeHumidityPercent.ToString("F2", CultureInfo.InvariantCulture);
Console.WriteLine($"measured     {celsius} C, {hectopascals} hPa, {humidityPercent} %");

// The script is spent: every transfer the datasheet lists was made, and no other.
Console.WriteLine($"bus          {bus.Transfers} transfers, unexpected: {(!bus.Done).ToString().ToLowerInvariant()}");
```
<!-- end -->

## Reference

<!-- table: reference hal -->
- Rust: [`pamoja-hal`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_hal/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-hal)
- TypeScript: [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-hal)
- Python: [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-hal)
- C#: [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-hal)
<!-- end -->
