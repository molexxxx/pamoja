"""The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md."""

# ANCHOR: parts
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
# ANCHOR_END: parts


# ANCHOR: example
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
# ANCHOR_END: example

assert pin.level_for(RELAY, True) == Level.LOW
assert while_filling is False
assert once_filled is True
assert closing is True
assert pin.triggers(Edge.RISING, Level.HIGH, Level.LOW) is False
assert [ran, stopped] == [Level.LOW, Level.HIGH]
assert (to_write, to_read) == (0xEC, 0xED)
assert i2c.is_reserved(0x76) is False
assert reserved is True
assert (clock.cpol, clock.cpha) == (True, True)
assert spi.mode_for(True, False) == 2
