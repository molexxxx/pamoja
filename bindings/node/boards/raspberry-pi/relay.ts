// A relay board on GPIO17 that follows a limit switch on GPIO27, debounced. Wire the relay
// board's IN to GPIO17 and its VCC and GND to the header's 5V and ground, and the switch
// between GPIO27 and ground with `gpio=27=ip,pu` in config.txt. See
// docs/boards/raspberry-pi.md.

// ANCHOR: example
import { Contact, GpioLine, PinLevel, Switch } from '@pamoja/gpio'
import { Debounce } from '@pamoja/kit'

// The GPIO chip the header's lines live on. Every current model exposes them here, and the
// line numbers below are the BCM numbers the documentation and the kernel both use.
const CHIP = '/dev/gpiochip0'
const RELAY_LINE = 17
const SWITCH_LINE = 27

// Taking the line as an output also says what to drive the moment it is taken. Until then
// every GPIO is an input, so a relay board sees whatever its own pull gives it; driving the
// resting level immediately is what keeps a vent from opening at boot. Most relay boards
// energize on a low input, which is what `activeLow` says once so that nothing below this
// line has to think about the inversion again.
const relay = Switch.activeLow(GpioLine.openOutput(CHIP, RELAY_LINE, PinLevel.High))

// The switch is wired to pull the line down when it closes, so it is active low too.
const limit = Contact.activeLow(GpioLine.openInput(CHIP, SWITCH_LINE))

// A mechanical contact bounces for a few milliseconds as it closes. Sampling every 20 ms and
// requiring three agreeing samples means the state has to hold for 60 ms before it counts,
// which is longer than the bounce and shorter than a person.
const settled = new Debounce(3, false)
let wasClosed = false

console.log(`watching GPIO${SWITCH_LINE}, driving GPIO${RELAY_LINE}; Ctrl-C to stop`)
setInterval(() => {
  const closed = settled.update(limit.isAsserted())
  if (closed !== wasClosed) {
    console.log(`the limit switch ${closed ? 'closed' : 'opened'}`)
    // The relay follows the switch. A real vent would run its motor until the limit closes
    // and then stop; this is the same two calls either way.
    relay.set(!closed)
    wasClosed = closed
  }
}, 20)
// ANCHOR_END: example
