// The simulators guide example; see docs/guides/sim.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { RecordingActuator, Replay, SimulatedRobot } from '@pamoja/sim'

async function main() {
  // The clear distance ahead, in metres, taken from an earlier survey run. A replay hands
  // it back one reading at a time, so the loop below sees the same input on every run: the
  // same rover code, driven by a recording rather than a range finder.
  const capture = [4, 3, 1.5, 0.5]
  const ahead = new Replay(capture)
  const throttle = new RecordingActuator()
  const rover = new SimulatedRobot(0.5) // each command advances the rover half a second

  const seen: number[] = []
  for (let step = 0; step < capture.length; step += 1) {
    const reading = (await ahead.read())!
    seen.push(reading)

    // Drive on while there is room ahead, otherwise stop and turn on the spot.
    const clear = reading > 1
    const speed = clear ? 1 : 0
    const turn = clear ? 0 : 1
    await throttle.apply(speed)
    await rover.apply({ vx: speed, vy: 0, omega: turn })
    console.log(`${reading} m ahead, so drive at ${speed} and turn at ${turn}`)
  }

  // The recording actuator kept every command, which is how a test says what the control
  // loop decided rather than only what it ended up doing.
  const commands = await throttle.commands()
  console.log(`commands  ${commands.join(', ')}`)

  // Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on the spot
  // at 1 rad/s for half a second, which moves the rover nowhere.
  const pose = await rover.pose()
  console.log(
    `pose      x ${pose.x.toFixed(1)} m, y ${pose.y.toFixed(1)} m,` +
      ` heading ${pose.theta.toFixed(1)} rad`,
  )

  return { seen, commands, pose }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  seen: number[]
  commands: number[]
  pose: { x: number; y: number; theta: number }
}): void {
  assert.deepEqual(seen.seen, [4, 3, 1.5, 0.5])
  assert.deepEqual(seen.commands, [1, 1, 1, 0])
  assert.ok(Math.abs(seen.pose.x - 1.5) < 1e-6)
  assert.ok(Math.abs(seen.pose.y) < 1e-6)
  assert.ok(Math.abs(seen.pose.theta - 0.5) < 1e-6)
}
