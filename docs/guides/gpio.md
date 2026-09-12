# I2C, SPI, and GPIO

Before a node reaches any network it has to talk to the chips wired to the same
board. Three interfaces cover most of that hardware: a plain pin for the relays,
buttons, and float switches that sit on one line, I2C for the dense breakout
sensors, and SPI for displays, SD cards, and radios. Each carries a small piece
of exact logic, and each is a classic field bug when it is wrong: which level
energizes a relay, which address byte reaches the bus, and which clock mode a
part expects.

pamoja carries that logic and none of the wiring. A relay is a `Switch` and a
float switch is a `Contact`, both over whatever output and input line the board
gives you, so the same code runs on a Raspberry Pi through the kernel's GPIO
character device, on a microcontroller through its own pins, and in a test
against a scripted line with nothing plugged in.

## What the example does

It runs a pump while a tank fills. The pump sits on a relay board that energizes
when its input is pulled low, and the float switch closes to ground when the tank
is full, so both are active low and the example says so once instead of inverting
a level by hand at every call site. The float reads open, then closed; the pump
stops. Then it looks up the two facts a datasheet asks for before a part on a
shared bus will answer at all: the byte the address becomes on the wire, and the
clock mode.

It proves:

- Naming a relay active low is enough: running the pump drives its line low and
  stopping it drives the line high, with no inversion written at the call site.
- The same polarity read the other way turns a level into an answer, so an open
  float reads as not full and a closed one reads as full.
- A tank reaching full is a falling edge on that line, which a watch armed for
  the rising edge would sleep through.
- A part at `0x76` is written to as `0xEC` and read from as `0xED`, which is why
  a datasheet and a bus capture rarely print the same number.
- `0x78` is not a device address at all, because it opens a block the I2C
  specification keeps for itself.
- SPI mode 3 is clock polarity 1 with phase 1, the pair a datasheet quotes as one
  number.

## Run it

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example gpio" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example gpio</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- gpio" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- gpio</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/gpio.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/gpio.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- gpio" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- gpio</code></div>
</div>
<!-- end -->

## Rust

<!-- snippet: examples/guides/gpio.rs#example -->
From [`examples/guides/gpio.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gpio.rs):

```rust
use pamoja_core::{Actuator, Sensor};
use pamoja_gpio::i2c::{Address, Direction, RESERVED_FROM};
use pamoja_gpio::pin::{Edge, Level, Polarity};
use pamoja_gpio::spi::Mode;
use pamoja_gpio::switch::{Contact, Switch};
use pamoja_hal::digital::PinState;
use pamoja_hal::script::PinScript;

// Most relay boards energize when their input is pulled low, and a float switch wired
// to ground closes the same way. Saying "active low" once, here, is what keeps the
// inversion out of every line below it.
let mut pump = Switch::new(PinScript::new([]), Polarity::ActiveLow);
let mut float = Contact::new(
    PinScript::new([PinState::High, PinState::Low]),
    Polarity::ActiveLow,
);
println!(
    "a pump on an active-low relay runs when its line is {:?}",
    Polarity::ActiveLow.level(true)
);

// The pump runs while the tank fills. The scripted line answers open and then closed,
// so this is the real loop with nothing plugged in; on a board the same two lines take
// a pin from the host's GPIO library instead.
pump.apply(true).await.expect("the relay takes it");
let while_filling = float.read().await.expect("the line reads");
let once_filled = float.read().await.expect("the line reads");
println!("the float reads full: {while_filling}, then {once_filled}");

// The moment the float closes is that line going low, which is a falling edge. A watch
// armed for the rising one would sleep through the tank filling.
let closing = Edge::Falling.triggered_by(Level::High, Level::Low);
println!("the float closing is a falling edge on that line: {closing}");

// Full, so the pump stops. Releasing the switch hands the line back, and the levels it
// was driven to are the whole conversation the board saw.
pump.apply(false).await.expect("the relay takes it");
let line = pump.release();
let (ran, stopped) = (line.driven()[0], line.driven()[1]);
println!("running drove the line {ran:?} and stopping drove it {stopped:?}");

// A part on a shared bus answers to an address, and the byte on the wire is not the
// address the datasheet prints: it shifts up one and the low bit says read or write.
let sensor = Address::seven_bit(0x76).expect("a 7-bit address");
let to_write = sensor.frame(Direction::Write).as_bytes()[0];
let to_read = sensor.frame(Direction::Read).as_bytes()[0];
println!("a part at 0x76 is written to as {to_write:#04X} and read from as {to_read:#04X}");

// Two ranges belong to the specification itself, so a part answering in either is a
// wiring mistake rather than a device.
let reserved = Address::seven_bit(RESERVED_FROM)
    .expect("in range")
    .is_reserved();
println!("{RESERVED_FROM:#04X} is reserved by the specification: {reserved}");

// And a datasheet quotes SPI's clock polarity and phase as one mode number.
let (idles_high, trailing_edge) = Mode::Mode3.cpol_cpha();
println!("SPI mode 3 idles high: {idles_high}, samples on the trailing edge: {trailing_edge}");
```
<!-- end -->

`Switch` and `Contact` take any `embedded-hal` output and input line, so the two
lines that build them are the only ones that change between a board and a test.
`PinScript` is the test line: it answers the reads it was given and records every
level it was driven to, which is why this runs in CI. On a Raspberry Pi those two
arguments come from `pamoja-hal`'s Linux backend instead, and on a
microcontroller from the chip's own HAL. `Switch` implements `Actuator` and
`Contact` implements `Sensor`, so a profile or a rule drives the pump without
knowing it is a pin.

## TypeScript

There is no trait to implement here. The board's own GPIO library drives the
line, and pamoja carries the polarity, the edge, and the addressing. The stand-in
below is what a real node replaces with `onoff`, `rpio`, or a vendor SDK:

<!-- snippet: bindings/node/guides/gpio.ts#parts -->
From [`bindings/node/guides/gpio.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gpio.ts):

```typescript
import { PinEdge, PinLevel, PinPolarity, i2c, pin, spi } from '@pamoja/gpio'

// The board's own library drives the line: `onoff` or `rpio` on a Raspberry Pi, a vendor
// SDK on a microcontroller. This stands in for one so the example runs with nothing
// plugged in, and it is the only part a real node replaces.
class Line {
  driven: PinLevel[] = []

  constructor(private readings: PinLevel[] = []) {}

  drive(level: PinLevel): void {
    this.driven.push(level)
  }

  read(): PinLevel {
    return this.readings.shift()!
  }
}
```
<!-- end -->

<!-- snippet: bindings/node/guides/gpio.ts#example -->
From [`bindings/node/guides/gpio.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gpio.ts):

```typescript
// Most relay boards energize when their input is pulled low, and a float switch wired to
// ground closes the same way. Saying "active low" once, here, is what keeps the inversion
// out of every line below it.
const RELAY = PinPolarity.ActiveLow
const FLOAT = PinPolarity.ActiveLow
const pump = new Line()
const float = new Line([PinLevel.High, PinLevel.Low])
console.log(`a pump on an active-low relay runs when its line is ${pin.levelFor(RELAY, true)}`)

// The pump runs while the tank fills. The stand-in line answers open and then closed, so
// this is the real loop with nothing plugged in.
pump.drive(pin.levelFor(RELAY, true))
const whileFilling = pin.isAsserted(FLOAT, float.read())
const onceFilled = pin.isAsserted(FLOAT, float.read())
console.log(`the float reads full: ${whileFilling}, then ${onceFilled}`)

// The moment the float closes is that line going low, which is a falling edge. A watch
// armed for the rising one would sleep through the tank filling.
const closing = pin.triggers(PinEdge.Falling, PinLevel.High, PinLevel.Low)
console.log(`the float closing is a falling edge on that line: ${closing}`)

// Full, so the pump stops, and the levels the line was driven to are the whole
// conversation the board saw.
pump.drive(pin.levelFor(RELAY, false))
const [ran, stopped] = pump.driven
console.log(`running drove the line ${ran} and stopping drove it ${stopped}`)

// A part on a shared bus answers to an address, and the byte on the wire is not the
// address the datasheet prints: it shifts up one and the low bit says read or write.
const hex = (byte: number) => `0x${byte.toString(16).toUpperCase().padStart(2, '0')}`
const toWrite = i2c.addressFrame(0x76)[0]!
const toRead = i2c.addressFrame(0x76, { read: true })[0]!
console.log(`a part at 0x76 is written to as ${hex(toWrite)} and read from as ${hex(toRead)}`)

// Two ranges belong to the specification itself, so a part answering in either is a wiring
// mistake rather than a device.
const reserved = i2c.isReserved(i2c.RESERVED_FROM)
console.log(`${hex(i2c.RESERVED_FROM)} is reserved by the specification: ${reserved}`)

// And a datasheet quotes SPI's clock polarity and phase as one mode number.
const clock = spi.clockFor(3)
console.log(`SPI mode 3 idles high: ${clock.cpol}, samples on the trailing edge: ${clock.cpha}`)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/gpio.py#parts -->
From [`bindings/python/guides/gpio.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gpio.py):

```python
from pamoja.gpio import Edge, Level, Polarity, i2c, pin, spi


class Line:
    """The board's own library drives the line: `gpiozero` or `lgpio` on a Raspberry
    Pi, a vendor SDK on a microcontroller. This stands in for one so the example runs
    with nothing plugged in, and it is the only part a real node replaces."""

    def __init__(self, readings: list[Level] | None = None) -> None:
        self.driven: list[Level] = []
        self._readings = list(readings or [])

    def drive(self, level: Level) -> None:
        self.driven.append(level)

    def read(self) -> Level:
        return self._readings.pop(0)
```
<!-- end -->

<!-- snippet: bindings/python/guides/gpio.py#example -->
From [`bindings/python/guides/gpio.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gpio.py):

```python
# Most relay boards energize when their input is pulled low, and a float switch wired to
# ground closes the same way. Saying "active low" once, here, is what keeps the inversion
# out of every line below it.
RELAY = Polarity.ACTIVE_LOW
FLOAT = Polarity.ACTIVE_LOW
pump = Line()
float_switch = Line([Level.HIGH, Level.LOW])
print(f"a pump on an active-low relay runs when its line is {pin.level_for(RELAY, True).value}")

# The pump runs while the tank fills. The stand-in line answers open and then closed, so
# this is the real loop with nothing plugged in.
pump.drive(pin.level_for(RELAY, True))
while_filling = pin.is_asserted(FLOAT, float_switch.read())
once_filled = pin.is_asserted(FLOAT, float_switch.read())
print(f"the float reads full: {while_filling}, then {once_filled}")

# The moment the float closes is that line going low, which is a falling edge. A watch
# armed for the rising one would sleep through the tank filling.
closing = pin.triggers(Edge.FALLING, Level.HIGH, Level.LOW)
print(f"the float closing is a falling edge on that line: {closing}")

# Full, so the pump stops, and the levels the line was driven to are the whole
# conversation the board saw.
pump.drive(pin.level_for(RELAY, False))
ran, stopped = pump.driven
print(f"running drove the line {ran.value} and stopping drove it {stopped.value}")

# A part on a shared bus answers to an address, and the byte on the wire is not the
# address the datasheet prints: it shifts up one and the low bit says read or write.
to_write = i2c.address_frame(0x76)[0]
to_read = i2c.address_frame(0x76, read=True)[0]
print(f"a part at 0x76 is written to as 0x{to_write:02X} and read from as 0x{to_read:02X}")

# Two ranges belong to the specification itself, so a part answering in either is a wiring
# mistake rather than a device.
reserved = i2c.is_reserved(i2c.RESERVED_FROM)
print(f"0x{i2c.RESERVED_FROM:02X} is reserved by the specification: {reserved}")

# And a datasheet quotes SPI's clock polarity and phase as one mode number.
clock = spi.clock_for(3)
print(f"SPI mode 3 idles high: {clock.cpol}, samples on the trailing edge: {clock.cpha}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs#parts -->
From [`bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs):

```csharp
/// <summary>
/// The board's own library drives the line: <c>System.Device.Gpio</c> on a Raspberry
/// Pi, a vendor SDK on a microcontroller. This stands in for one so the example runs
/// with nothing plugged in, and it is the only part a real node replaces.
/// </summary>
private sealed class Line
{
    private readonly Queue<PinLevel> _readings;

    public Line(params PinLevel[] readings) => _readings = new Queue<PinLevel>(readings);

    public List<PinLevel> Driven { get; } = new();

    public void Drive(PinLevel level) => Driven.Add(level);

    public PinLevel Read() => _readings.Dequeue();
}
```
<!-- end -->

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs):

```csharp
// Most relay boards energize when their input is pulled low, and a float switch
// wired to ground closes the same way. Saying "active low" once, here, is what
// keeps the inversion out of every line below it.
const PinPolarity Relay = PinPolarity.ActiveLow;
const PinPolarity Float = PinPolarity.ActiveLow;
var pump = new Line();
var floatSwitch = new Line(PinLevel.High, PinLevel.Low);
Console.WriteLine(
    $"a pump on an active-low relay runs when its line is {Pin.LevelFor(Relay, true)}");

// The pump runs while the tank fills. The stand-in line answers open and then
// closed, so this is the real loop with nothing plugged in.
pump.Drive(Pin.LevelFor(Relay, true));
bool whileFilling = Pin.IsAsserted(Float, floatSwitch.Read());
bool onceFilled = Pin.IsAsserted(Float, floatSwitch.Read());
Console.WriteLine($"the float reads full: {whileFilling}, then {onceFilled}");

// The moment the float closes is that line going low, which is a falling edge. A
// watch armed for the rising one would sleep through the tank filling.
bool closing = Pin.Triggers(PinEdge.Falling, PinLevel.High, PinLevel.Low);
Console.WriteLine($"the float closing is a falling edge on that line: {closing}");

// Full, so the pump stops, and the levels the line was driven to are the whole
// conversation the board saw.
pump.Drive(Pin.LevelFor(Relay, false));
(PinLevel ran, PinLevel stopped) = (pump.Driven[0], pump.Driven[1]);
Console.WriteLine($"running drove the line {ran} and stopping drove it {stopped}");

// A part on a shared bus answers to an address, and the byte on the wire is not
// the address the datasheet prints: it shifts up one and the low bit says read or
// write.
byte toWrite = I2c.AddressFrame(0x76)[0];
byte toRead = I2c.AddressFrame(0x76, read: true)[0];
Console.WriteLine(
    $"a part at 0x76 is written to as 0x{toWrite:X2} and read from as 0x{toRead:X2}");

// Two ranges belong to the specification itself, so a part answering in either is
// a wiring mistake rather than a device.
bool reserved = I2c.IsReserved(I2c.ReservedFrom);
Console.WriteLine(
    $"0x{I2c.ReservedFrom:X2} is reserved by the specification: {reserved}");

// And a datasheet quotes SPI's clock polarity and phase as one mode number.
SpiClock clock = Spi.ClockFor(3);
Console.WriteLine(
    $"SPI mode 3 idles high: {clock.Cpol}, samples on the trailing edge: {clock.Cpha}");
```
<!-- end -->

## Where next

- [Buses and links](../buses.md), for what each bus is for and which crate
  carries its logic, and [Buses](hal.md) for the bus traits, the scripted buses,
  and the Linux backend.
- The board pages wire a part to a [Raspberry Pi](../boards/raspberry-pi.md), an
  [ESP32](../boards/esp32.md), or an [RP2040](../boards/rp2040.md) and run a
  first program on it.
- [Your own device](device.md), for a part pamoja has never heard of, and
  [Sensor drivers](sensors.md) for the ones it has.

## Reference

<!-- table: reference gpio -->
- Rust: [`pamoja-gpio`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gpio)
- TypeScript: [`@pamoja/gpio`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gpio)
- Python: [`pamoja.gpio`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gpio)
- C#: [`Pamoja.Gpio`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gpio)
<!-- end -->
