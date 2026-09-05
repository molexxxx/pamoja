"""The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md."""

# ANCHOR: example
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
# ANCHOR_END: example

assert i2c.address_frame(BME280, read=True) == bytes([0xED])
assert not i2c.is_reserved(BME280)
assert i2c.is_reserved(0x78)
assert i2c.frame_len(0x2A5, ten_bit=True) == 2
assert clock.cpol and clock.cpha
assert spi.mode_for(True, False) == 2
assert energise is Level.LOW
assert pin.is_asserted(Polarity.ACTIVE_LOW, Level.LOW)
assert rising
assert not falling
