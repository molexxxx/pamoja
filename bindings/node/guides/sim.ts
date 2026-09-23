// The simulators guide example; see docs/guides/sim.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { Transport } from '@pamoja/core'
import { LoopbackBroker } from '@pamoja/loopback'
import { RecordingActuator, Replay, SimulatedRobot, SimulatedSensor } from '@pamoja/sim'

async function main() {
  // The clear distance ahead, in meters, replayed from an earlier survey of the row, so
  // every run sees the same row.
  const ahead = new Replay([4, 3, 1.5, 0.5])
  // The drive keeps the commands it is given instead of turning a motor.
  const drive = new RecordingActuator()
  // The rover's pose comes from integrating each command over half a second.
  const dt = 0.5
  const rover = new SimulatedRobot(dt)
  // The soil probe reads around 31 percent, drying half a point a reading, with a wobble
  // drawn from a seed, so the same seed gives the same readings every run.
  const soil = new SimulatedSensor(31, -0.5, 0.3, 7)
  // The radio loses every third report on its way to the base.
  const radio = Transport.degraded(new LoopbackBroker().rung(), { dropEvery: 3 })
  await radio.connect()

  const moisture: number[] = []
  let delivered = 0
  for (;;) {
    // A replay that has handed back every reading reports that it is closed.
    let clear: number
    try {
      clear = await ahead.read()
    } catch (error) {
      console.log(`ahead     ran out after ${moisture.length} readings: ${(error as Error).message}`)
      break
    }
    const [speed, turn] = clear > 1 ? [1, 0] : [0, 1]
    await drive.apply(speed)
    await rover.apply({ vx: speed, vy: 0, omega: turn })
    const wet = await soil.read()
    const elapsed = moisture.length * dt
    moisture.push(wet)
    const report = wet.toFixed(1)
    const sent = await radio.send('vineyard/row-4/soil', report).then(
      () => true,
      () => false,
    )
    if (sent) delivered += 1
    console.log(
      `${elapsed.toFixed(1)} s     ${clear.toFixed(1)} m clear:` +
        ` drive ${speed.toFixed(1)}, turn ${turn.toFixed(1)}, soil ${report}`,
    )
  }

  // The drive kept every command, which is how a test says what the loop decided rather
  // than only what it ended up doing.
  const commands = await drive.commands()
  console.log(`drive     recorded ${commands.map((command) => command.toFixed(1)).join(', ')}`)

  // Three half-second commands at 1 m/s reach 1.5 m along x. The last turns on the spot
  // at 1 rad/s for half a second, which moves the rover nowhere.
  const pose = await rover.pose()
  console.log(
    `rover     ended at x ${pose.x.toFixed(1)} m, y ${pose.y.toFixed(1)} m,` +
      ` heading ${pose.theta.toFixed(1)} rad`,
  )

  // A second probe with the same seed reads exactly the same values.
  const twin = new SimulatedSensor(31, -0.5, 0.3, 7)
  const again: number[] = []
  for (let read = 0; read < moisture.length; read += 1) {
    again.push(await twin.read())
  }
  const same = again.every((value, index) => value === moisture[index])
  console.log(`soil      a probe with the same seed read ${same ? 'the same' : 'different'} ${again.length} values`)
  console.log(`radio     delivered ${delivered} of ${moisture.length} soil reports and lost the third`)

  return { commands, pose, same, delivered }
}

main()
// ANCHOR_END: example
  .then(check)

function check(seen: {
  commands: number[]
  pose: { x: number; y: number; theta: number }
  same: boolean
  delivered: number
}): void {
  assert.deepEqual(seen.commands, [1, 1, 1, 0])
  assert.ok(Math.abs(seen.pose.x - 1.5) < 1e-6 && Math.abs(seen.pose.y) < 1e-6)
  assert.ok(Math.abs(seen.pose.theta - 0.5) < 1e-6)
  assert.ok(seen.same)
  assert.equal(seen.delivered, 3)
}
