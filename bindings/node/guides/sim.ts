// The simulators guide example; see docs/guides/sim.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { RecordingActuator, Replay, SimulatedRobot } from '@pamoja/sim'

async function main() {
  // The clear distance ahead, in metres, taken from an earlier survey run. A replay hands
  // it back one reading at a time, so the loop below sees the same input on every run.
  const capture = [4.0, 3.0, 1.5, 0.5]
  const ahead = new Replay(capture)
  const throttle = new RecordingActuator()
  const rover = new SimulatedRobot(0.5) // each command advances the rover half a second

  const seen: number[] = []
  for (let step = 0; step < capture.length; step += 1) {
    const reading = await ahead.read()
    seen.push(reading)
    // Drive on while there is room ahead, otherwise stop and turn on the spot.
    const clear = reading > 1.0
    const vx = clear ? 1.0 : 0.0
    const omega = clear ? 0.0 : 1.0
    await throttle.apply(vx)
    await rover.apply({ vx, vy: 0, omega })
  }

  assert.deepEqual(seen, capture)
  assert.deepEqual(await throttle.commands(), [1.0, 1.0, 1.0, 0.0])

  // Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on the
  // spot at 1 rad/s for half a second, which moves the rover nowhere.
  const pose = await rover.pose()
  assert.ok(Math.abs(pose.x - 1.5) < 1e-6)
  assert.ok(Math.abs(pose.y) < 1e-6)
  assert.ok(Math.abs(pose.theta - 0.5) < 1e-6)
}

main()
// ANCHOR_END: example
