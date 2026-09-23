// A CAN bus monitor: listens on the Pi's CAN interface, through an MCP2515, and names each
// J1939 message it hears. Load the controller's overlay and bring the interface up at the
// bus's bit rate first. See docs/guides/can.md.

// ANCHOR: example
import { CanBus, decodeJ1939, signalsFrom } from '@pamoja/can'

// The interface the MCP2515 overlay makes.
const INTERFACE = 'can0'

// Engine speed's parameter group, where the speed sits in it, and its scale.
const ENGINE_CONTROLLER_1 = 61_444
const ENGINE_SPEED_AT = 3
const RPM_PER_BIT = 0.125

async function main(): Promise<void> {
  // The monitor only listens and sends nothing, so it is safe on a running machine's bus.
  const bus = CanBus.open(INTERFACE)
  const started = performance.now()
  while (performance.now() - started < 10_000) {
    const frame = await bus.receive(1000)
    if (frame === null) {
      console.log('quiet for a second: check the bit rate, the wiring, and the termination')
      continue
    }
    const at = ((performance.now() - started) / 1000).toFixed(3).padStart(7)
    const id = decodeJ1939(frame.id, frame.extended)
    if (id !== null && id.pgn === ENGINE_CONTROLLER_1) {
      const rpm = (signalsFrom(frame.data).u16(ENGINE_SPEED_AT) ?? 0) * RPM_PER_BIT
      console.log(`${at} s  pgn ${id.pgn} from ${id.source}: ${rpm.toFixed(1)} rpm`)
    } else if (id !== null) {
      console.log(`${at} s  pgn ${id.pgn} from ${id.source}, ${frame.len} bytes`)
    } else {
      const hex = frame.id.toString(16).toUpperCase().padStart(3, '0')
      console.log(`${at} s  0x${hex}, ${frame.len} bytes, an 11-bit identifier`)
    }
  }
  console.log(`${bus.received} frames in ten seconds on ${INTERFACE}`)
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
// ANCHOR_END: example
