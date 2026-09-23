# I2C, SPI, and GPIO

Before a node reaches any network it has to talk to the chips wired to the same
board. Three interfaces cover most of that hardware: a plain pin for the relays,
buttons, and float switches that sit on one line, I2C for the dense breakout
sensors, and SPI for displays, SD cards, and radios. Each carries a small piece
of exact logic, and each is a classic field bug when it is wrong: which level
energizes a relay, which address byte reaches the bus, and which clock mode a
part expects.

pamoja carries that logic and none of the wiring. A relay is a `Switch` and a
float switch is a `Contact`, in all four languages, over whatever line the board
gives you: a scripted line in a test, a `GpioLine` on a Raspberry Pi or any Linux
board, or a microcontroller's own pin. The code between those two lines never
changes.

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

Each program runs with nothing plugged in. On a Raspberry Pi wired as
[On a board](#on-a-board) shows, set `PAMOJA_GPIO_CHIP=/dev/gpiochip0` before the same
command and it runs the real pump as well.

## Rust

`pamoja-gpio` carries the pin model and the two parts, `Switch` and `Contact`, which
take any `embedded-hal` output or input line; `pamoja-hal` carries `PinScript`, the
line a test gives them. `Switch` implements the core `Actuator` and `Contact` the core
`Sensor`, which is why the example drives the pump with `apply` and reads the float with
`read`: a profile or a rule drives the same pump without knowing it is a pin. The direct
calls are `set` and `is_asserted`, used in [On a board](#on-a-board). A line that fails
returns its own error through the `Result`; a scripted line never fails, so the example
`expect`s.

<!-- snippet: examples/guides/gpio.rs#example -->
From [`examples/guides/gpio.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gpio.rs):

```rust
use pamoja_core::{Actuator, Sensor};
use pamoja_gpio::i2c::{Address, Direction, RESERVED_FROM};
use pamoja_gpio::pin::{Edge, Level};
use pamoja_gpio::spi::Mode;
use pamoja_gpio::switch::{Contact, Switch};
use pamoja_hal::digital::PinState;
use pamoja_hal::script::PinScript;

// Most relay boards energize when their input is pulled low, and a float switch wired
// to ground closes the same way. Saying "active low" once, here, is what keeps the
// inversion out of every line below it.
let mut pump = Switch::active_low(PinScript::new([]));
let mut float = Contact::active_low(PinScript::new([PinState::High, PinState::Low]));
let runs_on = pump.polarity().level(true);
println!("a pump on an active-low relay runs when its line is {runs_on:?}");

// The pump runs while the tank fills. The scripted line answers open and then closed,
// so this is the real loop with nothing plugged in; on a board the same two lines take
// a pin from the board's GPIO library instead.
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

## TypeScript

`@pamoja/gpio` has the same three types, `Switch`, `Contact`, and `PinScript`, and the
same pure functions under `pin`, `i2c`, and `spi`. The calls are synchronous. A level is
`PinLevel.High` or `PinLevel.Low`, and anything with a `drive(level)` method goes under a
`Switch` and anything with `read()` under a `Contact`, so a `GpioLine` on Linux, a
`PinScript` in a test, or a two-line adapter over another pin library all fit. A refusal
throws an `Error` whose message names the cause.

<!-- snippet: bindings/node/guides/gpio.ts#example -->
From [`bindings/node/guides/gpio.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gpio.ts):

```typescript
import { Contact, GpioLine, PinEdge, PinLevel, PinScript, Switch, i2c, pin, spi } from '@pamoja/gpio'

// Most relay boards energize when their input is pulled low, and a float switch wired to
// ground closes the same way. Saying "active low" once, here, is what keeps the inversion
// out of every line below it.
const pump = Switch.activeLow(new PinScript())
const float = Contact.activeLow(new PinScript([PinLevel.High, PinLevel.Low]))
const runsOn = pin.levelFor(pump.polarity, true)
console.log(`a pump on an active-low relay runs when its line is ${runsOn}`)

// The pump runs while the tank fills. The scripted line answers open and then closed, so
// this is the real loop with nothing plugged in; on a board the same two lines take a pin
// from the board's GPIO library instead.
pump.set(true)
const whileFilling = float.isAsserted()
const onceFilled = float.isAsserted()
console.log(`the float reads full: ${whileFilling}, then ${onceFilled}`)

// The moment the float closes is that line going low, which is a falling edge. A watch
// armed for the rising one would sleep through the tank filling.
const closing = pin.triggers(PinEdge.Falling, PinLevel.High, PinLevel.Low)
console.log(`the float closing is a falling edge on that line: ${closing}`)

// Full, so the pump stops. Releasing the switch hands the line back, and the levels it was
// driven to are the whole conversation the board saw.
pump.set(false)
const [ran, stopped] = pump.release().driven
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

`pamoja.gpio` has the same types in Python's spelling: `Switch.active_low`,
`is_asserted`, `PinScript`. A level is `Level.HIGH` or `Level.LOW`, and a line is any
object with `drive(level)` or `read()`, which the `OutputLine` and `InputLine` protocols
describe for a type checker. A refusal raises `PamojaError`, except a value that means
nothing at all, such as SPI mode 4, which raises `ValueError`.

<!-- snippet: bindings/python/guides/gpio.py#example -->
From [`bindings/python/guides/gpio.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gpio.py):

```python
from pamoja.gpio import Contact, Edge, GpioLine, Level, PinScript, Switch, i2c, pin, spi

# Most relay boards energize when their input is pulled low, and a float switch wired to
# ground closes the same way. Saying "active low" once, here, is what keeps the inversion
# out of every line below it.
pump = Switch.active_low(PinScript())
float_switch = Contact.active_low(PinScript([Level.HIGH, Level.LOW]))
runs_on = pin.level_for(pump.polarity, True)
print(f"a pump on an active-low relay runs when its line is {runs_on.value}")

# The pump runs while the tank fills. The scripted line answers open and then closed, so
# this is the real loop with nothing plugged in; on a board the same two lines take a pin
# from the board's GPIO library instead.
pump.set(True)
while_filling = float_switch.is_asserted()
once_filled = float_switch.is_asserted()
print(f"the float reads full: {while_filling}, then {once_filled}")

# The moment the float closes is that line going low, which is a falling edge. A watch
# armed for the rising one would sleep through the tank filling.
closing = pin.triggers(Edge.FALLING, Level.HIGH, Level.LOW)
print(f"the float closing is a falling edge on that line: {closing}")

# Full, so the pump stops. Releasing the switch hands the line back, and the levels it was
# driven to are the whole conversation the board saw.
pump.set(False)
ran, stopped = pump.release().driven
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

`Pamoja.Gpio` has the same types. `Switch.ActiveLow(line)` and `Contact.ActiveLow(line)`
infer the line's type, and a line is anything that implements `IOutputLine` or
`IInputLine`: a `GpioLine`, a `PinScript`, or a class of your own over another library. A
refusal throws `PamojaException`; opening a `GpioLine` anywhere but Linux throws
`PlatformNotSupportedException`. A `GpioLine` is `IDisposable`, and disposing it hands the
line back to the kernel.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs):

```csharp
// Most relay boards energize when their input is pulled low, and a float switch
// wired to ground closes the same way. Saying "active low" once, here, is what
// keeps the inversion out of every line below it.
var pump = Switch.ActiveLow(new PinScript());
var floatSwitch = Contact.ActiveLow(new PinScript(PinLevel.High, PinLevel.Low));
PinLevel runsOn = Pin.LevelFor(pump.Polarity, true);
Console.WriteLine($"a pump on an active-low relay runs when its line is {runsOn}");

// The pump runs while the tank fills. The scripted line answers open and then
// closed, so this is the real loop with nothing plugged in; on a board the same two
// lines take a pin from the board's GPIO library instead.
pump.Set(true);
bool whileFilling = floatSwitch.IsAsserted();
bool onceFilled = floatSwitch.IsAsserted();
Console.WriteLine($"the float reads full: {whileFilling}, then {onceFilled}");

// The moment the float closes is that line going low, which is a falling edge. A
// watch armed for the rising one would sleep through the tank filling.
bool closing = Pin.Triggers(PinEdge.Falling, PinLevel.High, PinLevel.Low);
Console.WriteLine($"the float closing is a falling edge on that line: {closing}");

// Full, so the pump stops. Releasing the switch hands the line back, and the levels
// it was driven to are the whole conversation the board saw.
pump.Set(false);
IReadOnlyList<PinLevel> driven = pump.Release().Driven;
(PinLevel ran, PinLevel stopped) = (driven[0], driven[1]);
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

## On a board

On a Raspberry Pi, or any Linux board, a pin is a line on a GPIO chip, the device file
`/dev/gpiochip0` on most boards, and a line's number is the GPIO number a pinout gives,
not the physical pin number. The program is the one above with its two scripted lines
swapped for real ones:

| Part | Line | Header pin | Wiring | Polarity |
| --- | --- | --- | --- | --- |
| Relay board input | GPIO17 | 11 | the board's input to the pin, its ground to a ground pin | active low |
| Float switch | GPIO27 | 13 | between the pin and ground, with a pull-up to 3.3 V | active low |

The relay line is opened high, so the pump stays off from the moment the program takes
the line until it asks for it. The float needs a pull-up because a switch to ground only
ever pulls the line down: a 10 kΩ resistor from the pin to 3.3 V, or `gpio=27=ip,pu` in
`config.txt`, which sets the pull at boot. The pump stops whatever happens, including a
float that never closes, since a pump left running is the fault the program exists to
prevent.

### Rust

<!-- snippet: examples/guides/gpio.rs#board -->
From [`examples/guides/gpio.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/gpio.rs):

```rust
/// The same pump and float on a Raspberry Pi: the relay board's input on GPIO17 and the
/// float switch between GPIO27 and ground. Only the two lines change.
fn on_a_board(chip: &str) -> std::result::Result<(), Box<dyn Error>> {
    use std::time::{Duration, Instant};

    use pamoja_gpio::linux;
    use pamoja_gpio::pin::Level;
    use pamoja_gpio::switch::{Contact, Switch};

    // The relay energizes on a low input, so its line is taken high and the pump stays off
    // until it is asked to run. The float closes to ground against a pull-up.
    let mut pump = Switch::active_low(linux::output(chip, 17, Level::High)?);
    let mut float = Contact::active_low(linux::input(chip, 27)?);

    // Run the pump until the float closes, and stop it whatever happens: a pump left
    // running on a failed float is the fault this whole program exists to prevent.
    pump.set(true)?;
    let deadline = Instant::now() + Duration::from_secs(600);
    let filled = (|| -> std::result::Result<(), Box<dyn Error>> {
        while !float.is_asserted()? {
            if Instant::now() >= deadline {
                return Err(
                    "the tank did not fill in ten minutes; check the float and the supply".into(),
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    })();
    pump.set(false)?;
    filled?;
    println!("the tank is full and the pump is off");
    Ok(())
}
```
<!-- end -->

### TypeScript

<!-- snippet: bindings/node/guides/gpio.ts#board -->
From [`bindings/node/guides/gpio.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gpio.ts):

```typescript
// The same pump and float on a Raspberry Pi: the relay board's input on GPIO17 and the
// float switch between GPIO27 and ground. Only the two lines change.
async function onABoard(chip: string): Promise<void> {
  // The relay energizes on a low input, so its line is taken high and the pump stays off
  // until it is asked to run. The float closes to ground against a pull-up.
  const relay = GpioLine.openOutput(chip, 17, PinLevel.High)
  const floatLine = GpioLine.openInput(chip, 27)
  const pump = Switch.activeLow(relay)
  const float = Contact.activeLow(floatLine)

  // Run the pump until the float closes, and stop it whatever happens: a pump left running
  // on a failed float is the fault this whole program exists to prevent.
  const deadline = Date.now() + 10 * 60_000
  pump.set(true)
  try {
    while (!float.isAsserted()) {
      if (Date.now() >= deadline) {
        throw new Error('the tank did not fill in ten minutes; check the float and the supply')
      }
      await new Promise((resolve) => setTimeout(resolve, 100))
    }
  } finally {
    pump.set(false)
    relay.close()
    floatLine.close()
  }
  console.log('the tank is full and the pump is off')
}
```
<!-- end -->

### Python

<!-- snippet: bindings/python/guides/gpio.py#board -->
From [`bindings/python/guides/gpio.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gpio.py):

```python
def on_a_board(chip: str) -> None:
    """The same pump and float on a Raspberry Pi: the relay board's input on GPIO17 and
    the float switch between GPIO27 and ground. Only the two lines change."""
    # The relay energizes on a low input, so its line is taken high and the pump stays
    # off until it is asked to run. The float closes to ground against a pull-up.
    with (
        GpioLine.open_output(chip, 17, Level.HIGH) as relay,
        GpioLine.open_input(chip, 27) as float_line,
    ):
        pump = Switch.active_low(relay)
        float_switch = Contact.active_low(float_line)

        # Run the pump until the float closes, and stop it whatever happens: a pump left
        # running on a failed float is the fault this whole program exists to prevent.
        deadline = time.monotonic() + 10 * 60
        pump.set(True)
        try:
            while not float_switch.is_asserted():
                if time.monotonic() >= deadline:
                    raise TimeoutError(
                        "the tank did not fill in ten minutes; check the float and the supply"
                    )
                time.sleep(0.1)
        finally:
            pump.set(False)
    print("the tank is full and the pump is off")
```
<!-- end -->

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs#board -->
From [`bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs):

```csharp
/// <summary>
/// The same pump and float on a Raspberry Pi: the relay board's input on GPIO17 and the
/// float switch between GPIO27 and ground. Only the two lines change.
/// </summary>
/// <param name="chip">The GPIO chip's device file.</param>
public static void OnABoard(string chip)
{
    // The relay energizes on a low input, so its line is taken high and the pump stays
    // off until it is asked to run. The float closes to ground against a pull-up.
    using GpioLine relay = GpioLine.OpenOutput(chip, 17, PinLevel.High);
    using GpioLine floatLine = GpioLine.OpenInput(chip, 27);
    var pump = Switch.ActiveLow(relay);
    var floatSwitch = Contact.ActiveLow(floatLine);

    // Run the pump until the float closes, and stop it whatever happens: a pump left
    // running on a failed float is the fault this whole program exists to prevent.
    DateTime deadline = DateTime.UtcNow.AddMinutes(10);
    pump.Set(true);
    try
    {
        while (!floatSwitch.IsAsserted())
        {
            if (DateTime.UtcNow >= deadline)
            {
                throw new TimeoutException(
                    "the tank did not fill in ten minutes; check the float and the supply");
            }

            Thread.Sleep(100);
        }
    }
    finally
    {
        pump.Set(false);
    }

    Console.WriteLine("the tank is full and the pump is off");
}
```
<!-- end -->

<!-- languages end -->

A line is held by one program at a time, and the kernel records which, so `gpioinfo`
shows every line with its holder; one opened here shows `pamoja`. On Raspberry Pi OS a
user in the `gpio` group opens lines without root.

## Values at a glance

**Polarity** is the one fact that keeps an inversion out of every call site. Name it once,
where the part is made:

| Polarity | Asserted by | Turning it on drives | A switch is wired | Typical parts |
| --- | --- | --- | --- | --- |
| Active high | a high level | high | to the supply, with a pull-down | an LED from the pin to ground |
| Active low | a low level | low | to ground, with a pull-up | most relay boards, a button to ground |

**Edges** name which change a watch fires on:

| Edge | Fires on | Fits |
| --- | --- | --- |
| Rising | low to high | a button to the supply being pressed |
| Falling | high to low | a switch to ground closing, such as the float filling |
| Both | either change | a door that reports opening and closing |

**SPI modes** are the clock polarity (CPOL, the level the clock idles at) and the clock
phase (CPHA, which edge data is sampled on), quoted by a datasheet as one number. The LoRa
radios on the [hardware page](../hardware.md) take mode 0.

| Mode | CPOL | CPHA | Clock idles | Data sampled on |
| --- | --- | --- | --- | --- |
| 0 | 0 | 0 | low | the rising edge, the first |
| 1 | 0 | 1 | low | the falling edge, the second |
| 2 | 1 | 0 | high | the falling edge, the first |
| 3 | 1 | 1 | high | the rising edge, the second |

**I2C addresses** are seven bits, and the I2C-bus specification (NXP UM10204, Rev. 7.0,
Table 4) keeps sixteen of them. `is_reserved` answers for the whole of both blocks, and
`is_general_call` for `0x00`:

| 7-bit address | What the specification uses it for |
| --- | --- |
| `0x00` | the general call when writing, the START byte when reading |
| `0x01` | the CBUS address |
| `0x02` | reserved for a different bus format |
| `0x03` | reserved for future purposes |
| `0x04` to `0x07` | the Hs-mode controller code |
| `0x08` to `0x77` | devices |
| `0x78` to `0x7B` | the first byte of a 10-bit address |
| `0x7C` to `0x7F` | the device ID |

On the wire a 7-bit address becomes one byte, the address shifted up one with the read
bit at the bottom. A 10-bit address, `0x000` to `0x3FF`, becomes two: `11110`, the top two
address bits and the read bit, then the low eight bits. The specification allows a
reserved address to be given to a device on a bus that will never use it for its purpose,
which is why the functions report reserved addresses rather than refusing them.

**The same calls in each language:**

| To | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| drive a part on a line | `Switch::active_low(line)`, `set(on)` | `Switch.activeLow(line)`, `set(on)` | `Switch.active_low(line)`, `set(on)` | `Switch.ActiveLow(line)`, `Set(on)` |
| read a part on a line | `Contact::active_low(line)`, `is_asserted()` | `Contact.activeLow(line)`, `isAsserted()` | `Contact.active_low(line)`, `is_asserted()` | `Contact.ActiveLow(line)`, `IsAsserted()` |
| give a test a line | `PinScript::new(reads)` | `new PinScript(reads)` | `PinScript(reads)` | `new PinScript(reads)` |
| open a line on Linux | `linux::output(chip, n, level)`, `linux::input(chip, n)` | `GpioLine.openOutput(chip, n, level)`, `GpioLine.openInput(chip, n)` | `GpioLine.open_output(chip, n, level)`, `GpioLine.open_input(chip, n)` | `GpioLine.OpenOutput(chip, n, level)`, `GpioLine.OpenInput(chip, n)` |
| find the level for a state | `polarity.level(on)` | `pin.levelFor(polarity, on)` | `pin.level_for(polarity, on)` | `Pin.LevelFor(polarity, on)` |
| find the state from a level | `polarity.is_asserted(level)` | `pin.isAsserted(polarity, level)` | `pin.is_asserted(polarity, level)` | `Pin.IsAsserted(polarity, level)` |
| ask whether an edge fires | `edge.triggered_by(from, to)` | `pin.triggers(edge, from, to)` | `pin.triggers(edge, from, to)` | `Pin.Triggers(edge, from, to)` |
| frame an I2C address | `Address::seven_bit(a)?.frame(direction)` | `i2c.addressFrame(a, { read })` | `i2c.address_frame(a, read=read)` | `I2c.AddressFrame(a, read)` |
| check an I2C address | `address.is_reserved()` | `i2c.isReserved(a)` | `i2c.is_reserved(a)` | `I2c.IsReserved(a)` |
| read an SPI mode | `Mode::Mode3.cpol_cpha()` | `spi.clockFor(3)` | `spi.clock_for(3)` | `Spi.ClockFor(3)` |
| name an SPI mode | `Mode::from_cpol_cpha(cpol, cpha)` | `spi.modeFor(cpol, cpha)` | `spi.mode_for(cpol, cpha)` | `Spi.ModeFor(cpol, cpha)` |

## When it goes wrong

What each call refuses, and how each language says so:

| Call | Refused when | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- | --- |
| an I2C address | a 7-bit address above `0x7F`, a 10-bit one above `0x3FF` | `Err(GpioError::AddressOutOfRange)` | throws `Error` | raises `PamojaError` | throws `PamojaException` |
| an SPI mode number | above 3 | `Mode::from_number` returns `None` | throws `Error` | raises `ValueError` | throws `PamojaException` |
| opening a line | the platform is not Linux | `Err(OpenError::Unsupported)` | throws `Error` | raises `PamojaError` | throws `PlatformNotSupportedException` |
| opening a line | the chip or the line cannot be opened | `Err(OpenError::Gpio { .. })` | throws `Error` | raises `PamojaError` | throws `PamojaException` |
| driving or reading a line | the kernel refuses, as it does a drive on an input | `Err(LineError)` | throws `Error` | raises `PamojaError` | throws `PamojaException` |

Every message from opening, driving, or reading a line starts with the chip and the line,
as `/dev/gpiochip0 line 17: `, then says what the kernel reported.

The mistakes that cost an afternoon:

- **The relay clicks on when the program starts.** It is active low and its line started
  low. Open it high, as the examples do, and to hold it off while the board boots, before
  any program runs, add `gpio=17=op,dh` to `config.txt`.
- **A switch reads at random, or never changes.** The line has no pull, so an open switch
  floats. A switch to ground needs a pull-up: a resistor to 3.3 V, or `gpio=27=ip,pu` in
  `config.txt`.
- **Opening a line says permission denied.** The user is not in the `gpio` group. Add it
  with `sudo usermod -aG gpio $USER`, then log out and back in.
- **Opening a line says it is busy.** Another program or a kernel driver holds it;
  `gpioinfo` names the holder.
- **Opening says the chip does not exist, or the line is out of range.** `gpiodetect`
  lists the chips, and `gpioinfo` names each line: the chip whose lines are named `GPIO17`
  and `GPIO27` is the header.
- **An I2C part does not answer at the address its datasheet prints.** Some datasheets
  print the address already shifted, the way it appears on the wire: `0xEC` and `0xED`
  are the part at `0x76`. `i2cdetect -y 1` lists the 7-bit addresses that answer on a
  Raspberry Pi's user bus.
- **An SPI part answers `0x00`, `0xFF`, or noise.** Check the mode against the datasheet's
  CPOL and CPHA, then the chip select.

## Where next

<!-- table: next gpio -->
- [Your own device](device.md): A sensor and an actuator pamoja has never heard of, written against the core traits, run against a rule, and published with nothing plugged in.
- [Sensor drivers](sensors.md): Datasheet-anchored decoders for eleven parts.
- [Actuator drivers](actuators.md): PCA9685 PWM and servo pulses, and stepper coil sequencing.
- Beside it: [Buses and links](../buses.md), [Raspberry Pi](../boards/raspberry-pi.md), [ESP32](../boards/esp32.md), [RP2040](../boards/rp2040.md).
- Also in Field I/O: [Serial framing](serial.md), [Modbus RTU](modbus.md), [CAN and J1939](can.md), [Buses](hal.md).
<!-- end -->

## Reference

<!-- table: reference gpio -->
- Rust: [`pamoja-gpio`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gpio)
- TypeScript: [`@pamoja/gpio`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gpio)
- Python: [`pamoja.gpio`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gpio)
- C#: [`Pamoja.Gpio`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gpio)
<!-- end -->
