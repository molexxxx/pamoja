# @pamoja/sim

Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sim.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/sim
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/sim.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sim.ts):

```typescript
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
