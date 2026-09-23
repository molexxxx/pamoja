"""A relay board on GPIO17 that follows a limit switch on GPIO27, debounced.

Wire the relay board's IN to GPIO17 and its VCC and GND to the header's 5V and ground,
and the switch between GPIO27 and ground with ``gpio=27=ip,pu`` in config.txt. See
docs/boards/raspberry-pi.md.
"""

# ANCHOR: example
import time

from pamoja.gpio import Contact, GpioLine, Level, Switch
from pamoja.kit import Debounce

# The GPIO chip the header's lines live on. Every current model exposes them here, and the
# line numbers below are the BCM numbers the documentation and the kernel both use.
CHIP = "/dev/gpiochip0"
RELAY_LINE = 17
SWITCH_LINE = 27


def main() -> None:
    # Taking the line as an output also says what to drive the moment it is taken. Until
    # then every GPIO is an input, so a relay board sees whatever its own pull gives it;
    # driving the resting level immediately is what keeps a vent from opening at boot.
    # Most relay boards energize on a low input, which is what `active_low` says once so
    # that nothing below this line has to think about the inversion again.
    relay = Switch.active_low(GpioLine.open_output(CHIP, RELAY_LINE, Level.HIGH))

    # The switch is wired to pull the line down when it closes, so it is active low too.
    limit = Contact.active_low(GpioLine.open_input(CHIP, SWITCH_LINE))

    # A mechanical contact bounces for a few milliseconds as it closes. Sampling every
    # 20 ms and requiring three agreeing samples means the state has to hold for 60 ms
    # before it counts, which is longer than the bounce and shorter than a person.
    settled = Debounce(3, False)
    was_closed = False

    print(f"watching GPIO{SWITCH_LINE}, driving GPIO{RELAY_LINE}; Ctrl-C to stop")
    while True:
        closed = settled.update(limit.is_asserted())
        if closed != was_closed:
            print(f"the limit switch {'closed' if closed else 'opened'}")
            # The relay follows the switch. A real vent would run its motor until the
            # limit closes and then stop; this is the same two calls either way.
            relay.set(not closed)
            was_closed = closed
        time.sleep(0.02)
# ANCHOR_END: example


if __name__ == "__main__":
    main()
