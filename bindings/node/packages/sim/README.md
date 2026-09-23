# @pamoja/sim

Noisy and replay sensors, a recording actuator, a simulated robot that dead-reckons its pose, and a link that loses sends on a pattern. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sim.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html)

## Install

```sh
npm install @pamoja/sim
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/sim.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sim.ts):

```typescript
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sim`](https://crates.io/crates/pamoja-sim) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html), [docs.rs](https://docs.rs/pamoja-sim), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sim) |
| TypeScript | [`@pamoja/sim`](https://www.npmjs.com/package/@pamoja/sim) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sim) |
| Python | [`pamoja-sim`](https://pypi.org/project/pamoja-sim/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sim) |
| C# | [`Pamoja.Sim`](https://www.nuget.org/packages/Pamoja.Sim) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sim) |

## Documentation

- [`@pamoja/sim` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html), every class, function, and type this package exports.
- [The Simulators guide](https://pamoja.molex.cloud/docs/guides/sim.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
