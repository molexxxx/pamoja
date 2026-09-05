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

It builds the address bytes for a sensor at 7-bit address `0x76`, checks that
address against the ranges the I2C specification reserves, and frames the 10-bit
address UM10204 works through. It then reads back the clock polarity and phase
of SPI mode 3, and switches an active-low relay.

It proves:

- The 7-bit frame is `(address << 1) | r/w`, so a device at `0x76` is addressed
  as `0xEC` to write and `0xED` to read, which is why a datasheet and a bus
  capture rarely print the same number.
- `0x76` is a usable address and `0x78` is not: UM10204 keeps `0x00` to `0x07`
  and `0x78` to `0x7F` for itself.
- The 10-bit address `0x2A5` frames as `0xF4 0xA5`, the worked example in the
  specification, so the prefix and the split of the address bits are pinned
  rather than round-tripped against themselves.
- Mode 3 is CPOL 1 with CPHA 1, and CPOL 1 with CPHA 0 is mode 2, the relation
  `mode = (CPOL << 1) | CPHA` that every datasheet quotes as one number.
- An active-low relay is energised by a low level, and releasing it is a rising
  edge that a falling-edge trigger ignores.

## Rust

<!-- snippet: examples/tests/guides/gpio.rs#example -->
From [`examples/tests/guides/gpio.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/gpio.rs):

```rust
use pamoja_gpio::i2c::{Address, Direction};
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
const TEN_BIT_PREFIX: u8 = 0x78; // the first address the specification keeps back
let sensor_reserved = sensor.is_reserved();
let prefix = Address::seven_bit(TEN_BIT_PREFIX).expect("in range");
println!("{BME280:#04X} reserved: {sensor_reserved}");
println!("{TEN_BIT_PREFIX:#04X} reserved: {}", prefix.is_reserved());

// A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
// driver has to send a different number of bytes depending on the address it holds.
let wide = Address::ten_bit(0x2A5).expect("a 10-bit address");
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
  `${hex(BME280)} reserved: ${i2c.isReserved(BME280)}, 0x78 reserved: ${i2c.isReserved(0x78)}`,
)

// A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
// driver has to send a different number of bytes depending on the address it holds.
console.log(`a 10-bit address takes ${i2c.frameLen(0x2a5, true)} bytes`)

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
print(f"0x{BME280:02X} reserved: {i2c.is_reserved(BME280)}, 0x78 reserved: {i2c.is_reserved(0x78)}")

# A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
# driver has to send a different number of bytes depending on the address it holds.
print(f"a 10-bit address takes {i2c.frame_len(0x2A5, ten_bit=True)} bytes")

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
    + $"0x78 reserved: {I2c.IsReserved(0x78)}");

// A 10-bit address spends a reserved prefix over two bytes rather than one, so a
// bus driver sends a different number of bytes depending on the address it holds.
Console.WriteLine($"a 10-bit address takes {I2c.FrameLen(0x2A5, tenBit: true)} bytes");

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
- Rust: [`pamoja-gpio`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html)
- TypeScript: [`@pamoja/gpio`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html)
- Python: [`pamoja.gpio`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html)
- C#: [`Pamoja.Gpio`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html)
<!-- end -->
