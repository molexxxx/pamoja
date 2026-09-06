# I2C, SPI, and GPIO

Before a node reaches any network it has to talk to the chips wired to the same
board. Three interfaces cover most of that hardware: I2C for the dense breakout
sensors, SPI for displays, SD cards, and radios, and a plain pin for the relays,
buttons, and switches that sit on one line. Each carries a small piece of exact
logic, and each is a classic field bug when it is wrong: the address byte, the
clock mode, and whether a relay switches on high or low. pamoja carries that
logic and none of the wiring, so the same code runs on a gateway, on a
microcontroller, or in a test with nothing plugged in.

## What the example does

It builds the byte a sensor at 7-bit address `0x76` is written to and the byte
it is read from, checks that address against the ranges the I2C specification
keeps back, then asks how many bytes a 10-bit address puts on the wire. It
reads the clock polarity and phase back out of SPI mode 3, and works out which
level energises an active-low relay and which edge releasing it raises.

The numbers typed in are the ones a datasheet or the specification prints: the

address `0x76`, the 10-bit address `0x2A5` that UM10204 works through, and the

mode number 3. The reserved address the check runs against, the bytes that

reach the bus, the polarity and phase pair, and the level that drives the

relay all come back from the library, so no call site shifts an address or

reads a mode table itself.

It proves:

- A device at `0x76` is written to as `0xEC` and read from as `0xED`, one byte
  either way, which is why a datasheet and a bus capture rarely print the same
  number.
- `0x76` is a device address and `0x78` is not, because `0x78` opens the block
  the specification keeps back for itself.
- A 10-bit address takes two bytes on the wire where a 7-bit one takes a single
  byte, so a bus driver sends a different number of bytes depending on the
  address it holds.
- Mode 3 is CPOL 1 with CPHA 1, and the pair maps back the other way: CPOL 1
  with CPHA 0 is mode 2, not mode 3 again.
- An active-low relay is energised by a low level, which that polarity reads
  back as asserted, and releasing it is a rising edge that a falling-edge
  trigger ignores.

## Rust

<!-- snippet: examples/tests/guides/gpio.rs#example -->
From [`examples/tests/guides/gpio.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/gpio.rs):

```rust
use pamoja_gpio::i2c::{Address, Direction, RESERVED_FROM};
use pamoja_gpio::pin::{Edge, Level, Polarity};
use pamoja_gpio::spi::Mode;

// A BME280 answers at the 7-bit address its datasheet gives. That is not the byte
// that goes on the wire: the address shifts up one and the low bit says whether this
// transaction reads or writes, which is the step easiest to get wrong by hand.
const BME280: u8 = 0x76;
let sensor = Address::seven_bit(BME280).expect("a 7-bit address");
let to_write = sensor.frame(Direction::Write);
let to_read = sensor.frame(Direction::Read);
println!("write to  {:#04X}", to_write.as_bytes()[0]);
println!("read from {:#04X}", to_read.as_bytes()[0]);

// The I2C specification keeps two ranges of addresses for itself, so a part answering
// in either is a wiring mistake rather than a device.
let sensor_reserved = sensor.is_reserved();
let prefix = Address::seven_bit(RESERVED_FROM).expect("in range");
let prefix_reserved = prefix.is_reserved();
println!("{BME280:#04X} reserved: {sensor_reserved}, {RESERVED_FROM:#04X} reserved: {prefix_reserved}");

// A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
// driver has to send a different number of bytes depending on the address it holds.
// This is the worked example UM10204 itself prints.
const TEN_BIT_DEVICE: u16 = 0x2A5;
let wide = Address::ten_bit(TEN_BIT_DEVICE).expect("a 10-bit address");
let wide_frame = wide.frame(Direction::Write);
println!("a 10-bit address takes {} bytes", wide_frame.len());

// Datasheets quote clock polarity and phase as one mode number. Mode 3 idles the
// clock high and samples on the trailing edge.
let (idles_high, trailing_edge) = Mode::Mode3.cpol_cpha();
println!("spi mode 3: idles high {idles_high}, samples on the trailing edge {trailing_edge}");

// A relay board sold as active low energises when its pin is driven low. The polarity
// carries that inversion, so no call site has to remember which way round it is.
let energise = Polarity::ActiveLow.level(true);
println!("to energise an active-low relay, drive the pin {energise:?}");

// Releasing it drives the line back high, an edge a falling trigger ignores.
let rising = Edge::Rising.triggered_by(Level::Low, Level::High);
let falling = Edge::Falling.triggered_by(Level::Low, Level::High);
println!("release seen by a rising trigger: {rising}, by a falling trigger: {falling}");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/gpio.ts#example -->
From [`bindings/node/guides/gpio.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gpio.ts):

```typescript
import { PinEdge, PinLevel, PinPolarity, i2c, pin, spi } from '@pamoja/gpio'

// A BME280 answers at the 7-bit address its datasheet gives. That is not the byte that
// goes on the wire: the address shifts up one and the low bit says whether this
// transaction reads or writes, which is the step easiest to get wrong by hand.
const BME280 = 0x76
const hex = (byte: number) => `0x${byte.toString(16).toUpperCase()}`
console.log(`write to  ${hex(i2c.addressFrame(BME280)[0]!)}`)
console.log(`read from ${hex(i2c.addressFrame(BME280, { read: true })[0]!)}`)

// The I2C specification keeps two ranges of addresses for itself, so a part answering in
// either is a wiring mistake rather than a device.
console.log(
  `${hex(BME280)} reserved: ${i2c.isReserved(BME280)}, ` +
    `${hex(i2c.RESERVED_FROM)} reserved: ${i2c.isReserved(i2c.RESERVED_FROM)}`,
)

// A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
// driver has to send a different number of bytes depending on the address it holds.
// This is the worked example UM10204 itself prints.
const TEN_BIT_DEVICE = 0x2a5
console.log(`a 10-bit address takes ${i2c.frameLen(TEN_BIT_DEVICE, true)} bytes`)

// Datasheets quote clock polarity and phase as one mode number. Mode 3 idles the clock
// high and samples on the trailing edge.
const clock = spi.clockFor(3)
console.log(`spi mode 3: idles high ${clock.cpol}, samples on the trailing edge ${clock.cpha}`)

// A relay board sold as active low energises when its pin is driven low. The polarity
// carries that inversion, so no call site has to remember which way round it is.
const energise = pin.levelFor(PinPolarity.ActiveLow, true)
console.log(`to energise an active-low relay, drive the pin ${energise}`)

// Releasing it drives the line back high, an edge a falling trigger ignores.
const rising = pin.triggers(PinEdge.Rising, PinLevel.Low, PinLevel.High)
const falling = pin.triggers(PinEdge.Falling, PinLevel.Low, PinLevel.High)
console.log(`release seen by a rising trigger: ${rising}, by a falling trigger: ${falling}`)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/gpio.py#example -->
From [`bindings/python/guides/gpio.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gpio.py):

```python
from pamoja.gpio import Edge, Level, Polarity, i2c, pin, spi

# A BME280 answers at the 7-bit address its datasheet gives. That is not the byte that
# goes on the wire: the address shifts up one and the low bit says whether this
# transaction reads or writes, which is the step easiest to get wrong by hand.
BME280 = 0x76
print(f"write to  0x{i2c.address_frame(BME280)[0]:02X}")
print(f"read from 0x{i2c.address_frame(BME280, read=True)[0]:02X}")

# The I2C specification keeps two ranges of addresses for itself, so a part answering in
# either is a wiring mistake rather than a device.
reserved = i2c.RESERVED_FROM
print(f"0x{BME280:02X} reserved: {i2c.is_reserved(BME280)}, "
      f"0x{reserved:02X} reserved: {i2c.is_reserved(reserved)}")

# A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
# driver has to send a different number of bytes depending on the address it holds.
# This is the worked example UM10204 itself prints.
TEN_BIT_DEVICE = 0x2A5
print(f"a 10-bit address takes {i2c.frame_len(TEN_BIT_DEVICE, ten_bit=True)} bytes")

# Datasheets quote clock polarity and phase as one mode number. Mode 3 idles the clock
# high and samples on the trailing edge.
clock = spi.clock_for(3)
print(f"spi mode 3: idles high {clock.cpol}, samples on the trailing edge {clock.cpha}")

# A relay board sold as active low energises when its pin is driven low. The polarity
# carries that inversion, so no call site has to remember which way round it is.
energise = pin.level_for(Polarity.ACTIVE_LOW, True)
print(f"to energise an active-low relay, drive the pin {energise.name}")

# Releasing it drives the line back high, an edge a falling trigger ignores.
rising = pin.triggers(Edge.RISING, Level.LOW, Level.HIGH)
falling = pin.triggers(Edge.FALLING, Level.LOW, Level.HIGH)
print(f"release seen by a rising trigger: {rising}, by a falling trigger: {falling}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs):

```csharp
// A BME280 answers at the 7-bit address its datasheet gives. That is not the byte
// that goes on the wire: the address shifts up one and the low bit says whether
// this transaction reads or writes, which is easiest to get wrong by hand.
const byte Bme280 = 0x76;
Console.WriteLine($"write to  0x{I2c.AddressFrame(Bme280)[0]:X2}");
Console.WriteLine($"read from 0x{I2c.AddressFrame(Bme280, read: true)[0]:X2}");

// The I2C specification keeps two ranges of addresses for itself, so a part
// answering in either is a wiring mistake rather than a device.
Console.WriteLine(
    $"0x{Bme280:X2} reserved: {I2c.IsReserved(Bme280)}, "
    + $"0x{I2c.ReservedFrom:X2} reserved: {I2c.IsReserved(I2c.ReservedFrom)}");

// A 10-bit address spends a reserved prefix over two bytes rather than one, so a
// bus driver sends a different number of bytes depending on the address it holds.
// This is the worked example UM10204 itself prints.
const ushort TenBitDevice = 0x2A5;
Console.WriteLine($"a 10-bit address takes {I2c.FrameLen(TenBitDevice, tenBit: true)} bytes");

// Datasheets quote clock polarity and phase as one mode number. Mode 3 idles the
// clock high and samples on the trailing edge.
SpiClock clock = Spi.ClockFor(3);
Console.WriteLine(
    $"spi mode 3: idles high {clock.Cpol}, samples on the trailing edge {clock.Cpha}");

// A relay board sold as active low energises when its pin is driven low. The
// polarity carries that inversion, so no call site has to remember it.
PinLevel energise = Pin.LevelFor(PinPolarity.ActiveLow, true);
Console.WriteLine($"to energise an active-low relay, drive the pin {energise}");

// Releasing it drives the line back high, an edge a falling trigger ignores.
bool rising = Pin.Triggers(PinEdge.Rising, PinLevel.Low, PinLevel.High);
bool falling = Pin.Triggers(PinEdge.Falling, PinLevel.Low, PinLevel.High);
Console.WriteLine(
    $"release seen by a rising trigger: {rising}, by a falling trigger: {falling}");
```
<!-- end -->

## Reference

<!-- table: reference gpio -->
- Rust: [`pamoja-gpio`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-gpio)
- TypeScript: [`@pamoja/gpio`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-gpio)
- Python: [`pamoja.gpio`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-gpio)
- C#: [`Pamoja.Gpio`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-gpio)
<!-- end -->
