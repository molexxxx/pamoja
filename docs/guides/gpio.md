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

// A BME280 answers at 7-bit address 0x76, which is not the byte that goes on the wire:
// the address shifts up one and the read/write bit fills the low bit.
let mut frame = [0u8; 2];
let sensor = Address::seven_bit(0x76).expect("a 7-bit address");
assert_eq!(sensor.write_frame(Direction::Write, &mut frame), Ok(1));
assert_eq!(frame[0], 0xEC);
assert_eq!(sensor.write_frame(Direction::Read, &mut frame), Ok(1));
assert_eq!(frame[0], 0xED);

// UM10204 keeps 0x00..=0x07 and 0x78..=0x7F for itself, so an address in either range
// is a wiring mistake rather than a device.
assert!(!sensor.is_reserved());
assert!(Address::seven_bit(0x78).expect("in range").is_reserved());

// A 10-bit address spends the reserved 11110 prefix over two bytes: the prefix, the top
// two address bits, and the read/write bit, then the low eight bits.
let wide = Address::ten_bit(0x2A5).expect("a 10-bit address");
assert_eq!(wide.frame_len(), 2);
assert_eq!(wide.write_frame(Direction::Write, &mut frame), Ok(2));
assert_eq!(frame, [0xF4, 0xA5]);
assert_eq!(wide.write_frame(Direction::Read, &mut frame), Ok(2));
assert_eq!(frame, [0xF5, 0xA5]);

// Datasheets quote clock polarity and phase as one mode number, (CPOL << 1) | CPHA, so
// mode 3 idles the clock high and samples on the trailing edge.
assert_eq!(Mode::Mode3.cpol_cpha(), (true, true));
assert_eq!(Mode::from_cpol_cpha(true, false).number(), 2);

// A relay board sold as active low energises when its pin is driven low. The polarity
// carries that inversion so no call site has to remember it.
assert_eq!(Polarity::ActiveLow.level(true), Level::Low);
assert!(Polarity::ActiveLow.is_asserted(Level::Low));

// Releasing the relay drives the line back high, an edge a falling trigger ignores.
assert!(Edge::Rising.triggered_by(Level::Low, Level::High));
assert!(!Edge::Falling.triggered_by(Level::Low, Level::High));
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/gpio.ts#example -->
From [`bindings/node/guides/gpio.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/gpio.ts):

```typescript
import assert from 'node:assert/strict'

import { PinEdge, PinLevel, PinPolarity, i2c, pin, spi } from '@pamoja/gpio'

// A BME280 answers at 7-bit address 0x76, which is not the byte that goes on the wire:
// the address shifts up one and the read/write bit fills the low bit.
assert.deepEqual([...i2c.addressFrame(0x76)], [0xec])
assert.deepEqual([...i2c.addressFrame(0x76, { read: true })], [0xed])

// UM10204 keeps 0x00..0x07 and 0x78..0x7F for itself, so an address in either range is
// a wiring mistake rather than a device.
assert.equal(i2c.isReserved(0x76), false)
assert.equal(i2c.isReserved(0x78), true)

// A 10-bit address spends the reserved 11110 prefix over two bytes: the prefix, the top
// two address bits, and the read/write bit, then the low eight bits.
assert.equal(i2c.frameLen(0x2a5, true), 2)
assert.deepEqual([...i2c.addressFrame(0x2a5, { tenBit: true })], [0xf4, 0xa5])
assert.deepEqual([...i2c.addressFrame(0x2a5, { read: true, tenBit: true })], [0xf5, 0xa5])

// Datasheets quote clock polarity and phase as one mode number, (CPOL << 1) | CPHA, so
// mode 3 idles the clock high and samples on the trailing edge.
assert.deepEqual(spi.clockFor(3), { cpol: true, cpha: true })
assert.equal(spi.modeFor(true, false), 2)

// A relay board sold as active low energises when its pin is driven low. The polarity
// carries that inversion so no call site has to remember it.
const relay = PinPolarity.ActiveLow
const energised = pin.levelFor(relay, true)
assert.equal(energised, PinLevel.Low)
assert.equal(pin.isAsserted(relay, energised), true)

// Releasing the relay drives the line back high, an edge a falling trigger ignores.
const released = pin.invert(energised)
assert.equal(pin.triggers(PinEdge.Rising, energised, released), true)
assert.equal(pin.triggers(PinEdge.Falling, energised, released), false)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/gpio.py#example -->
From [`bindings/python/guides/gpio.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/gpio.py):

```python
from pamoja.gpio import Edge, Level, Polarity, i2c, pin, spi

# A BME280 answers at 7-bit address 0x76, which is not the byte that goes on the wire:
# the address shifts up one and the read/write bit fills the low bit.
assert i2c.address_frame(0x76) == bytes([0xEC])
assert i2c.address_frame(0x76, read=True) == bytes([0xED])

# UM10204 keeps 0x00..0x07 and 0x78..0x7F for itself, so an address in either range is
# a wiring mistake rather than a device.
assert not i2c.is_reserved(0x76)
assert i2c.is_reserved(0x78)

# A 10-bit address spends the reserved 11110 prefix over two bytes: the prefix, the top
# two address bits, and the read/write bit, then the low eight bits.
assert i2c.frame_len(0x2A5, ten_bit=True) == 2
assert i2c.address_frame(0x2A5, ten_bit=True) == bytes([0xF4, 0xA5])
assert i2c.address_frame(0x2A5, read=True, ten_bit=True) == bytes([0xF5, 0xA5])

# Datasheets quote clock polarity and phase as one mode number, (CPOL << 1) | CPHA, so
# mode 3 idles the clock high and samples on the trailing edge.
clock = spi.clock_for(3)
assert clock.cpol and clock.cpha
assert spi.mode_for(True, False) == 2

# A relay board sold as active low energises when its pin is driven low. The polarity
# carries that inversion so no call site has to remember it.
assert pin.level_for(Polarity.ACTIVE_LOW, True) is Level.LOW
assert pin.is_asserted(Polarity.ACTIVE_LOW, Level.LOW)

# Releasing the relay drives the line back high, an edge a falling trigger ignores.
assert pin.triggers(Edge.RISING, Level.LOW, Level.HIGH)
assert not pin.triggers(Edge.FALLING, Level.LOW, Level.HIGH)
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/GpioGuide.cs):

```csharp
// A BME280 answers at 7-bit address 0x76, which is not the byte that goes on the
// wire: the address shifts up one and the read/write bit fills the low bit.
Expect(
    I2c.AddressFrame(0x76).SequenceEqual(new byte[] { 0xEC }),
    "the write frame carries the address shifted up");
Expect(
    I2c.AddressFrame(0x76, read: true).SequenceEqual(new byte[] { 0xED }),
    "and the read frame sets the low bit");

// UM10204 keeps 0x00..0x07 and 0x78..0x7F for itself, so an address in either
// range is a wiring mistake rather than a device.
Expect(!I2c.IsReserved(0x76), "the sensor sits in the usable range");
Expect(I2c.IsReserved(0x78), "0x78 is the 10-bit prefix, not a device");

// A 10-bit address spends the reserved 11110 prefix over two bytes: the prefix,
// the top two address bits, and the read/write bit, then the low eight bits.
Expect(I2c.FrameLen(0x2A5, tenBit: true) == 2, "a 10-bit address takes two bytes");
Expect(
    I2c.AddressFrame(0x2A5, tenBit: true).SequenceEqual(new byte[] { 0xF4, 0xA5 }),
    "the two bytes the specification works through");
Expect(
    I2c.AddressFrame(0x2A5, read: true, tenBit: true)
        .SequenceEqual(new byte[] { 0xF5, 0xA5 }),
    "which differ only in the read/write bit");

// Datasheets quote clock polarity and phase as one mode number, (CPOL << 1) |
// CPHA, so mode 3 idles the clock high and samples on the trailing edge.
SpiClock clock = Spi.ClockFor(3);
Expect(clock.Cpol && clock.Cpha, "mode 3 idles high and samples late");
Expect(Spi.ModeFor(cpol: true, cpha: false) == 2, "that pair is mode 2");

// A relay board sold as active low energises when its pin is driven low. The
// polarity carries that inversion so no call site has to remember it.
PinLevel energised = Pin.LevelFor(PinPolarity.ActiveLow, asserted: true);
Expect(energised == PinLevel.Low, "an active-low relay switches on at a low level");
Expect(
    Pin.IsAsserted(PinPolarity.ActiveLow, energised),
    "and that level reads back as asserted");

// Releasing the relay drives the line back high, an edge a falling trigger ignores.
PinLevel released = Pin.Invert(energised);
Expect(
    Pin.Triggers(PinEdge.Rising, energised, released),
    "releasing it is a low-to-high transition");
Expect(
    !Pin.Triggers(PinEdge.Falling, energised, released),
    "which a falling trigger ignores");
```
<!-- end -->

## Reference

<!-- table: reference gpio -->
- Rust: [`pamoja-gpio`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_gpio/index.html)
- TypeScript: [`@pamoja/gpio`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_gpio.html)
- Python: [`pamoja.gpio`](https://pamoja.molex.cloud/docs/reference/python/pamoja/gpio.html)
- C#: [`Pamoja.Gpio`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Gpio.html)
<!-- end -->
