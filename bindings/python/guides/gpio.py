"""The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md."""

import os
import time

# ANCHOR: example
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
# ANCHOR_END: example

assert runs_on == Level.LOW
assert while_filling is False
assert once_filled is True
assert closing is True
assert pin.triggers(Edge.RISING, Level.HIGH, Level.LOW) is False
assert [ran, stopped] == [Level.LOW, Level.HIGH]
assert pump.is_asserted is False
assert (to_write, to_read) == (0xEC, 0xED)
assert i2c.is_reserved(0x76) is False
assert reserved is True
assert (clock.cpol, clock.cpha) == (True, True)
assert spi.mode_for(True, False) == 2


# ANCHOR: board
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
# ANCHOR_END: board


if chip := os.environ.get("PAMOJA_GPIO_CHIP"):
    on_a_board(chip)
