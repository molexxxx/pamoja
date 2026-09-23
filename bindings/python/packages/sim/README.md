# pamoja-sim

Noisy and replay sensors, a recording actuator, a simulated robot that dead-reckons its pose, and a link that loses sends on a pattern. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sim.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html)

## Install

```sh
pip install pamoja-sim
```

```python
from pamoja import sim
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/sim.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sim.py):

```python
import asyncio

from pamoja.core import PamojaError, Transport
from pamoja.loopback import LoopbackBroker
from pamoja.sim import RecordingActuator, Replay, SimulatedRobot, SimulatedSensor


async def main() -> None:
    # The clear distance ahead, in meters, replayed from an earlier survey of the row,
    # so every run sees the same row.
    ahead = Replay([4.0, 3.0, 1.5, 0.5])
    # The drive keeps the commands it is given instead of turning a motor.
    drive = RecordingActuator()
    # The rover's pose comes from integrating each command over half a second.
    dt = 0.5
    rover = SimulatedRobot(dt)
    # The soil probe reads around 31 percent, drying half a point a reading, with a
    # wobble drawn from a seed, so the same seed gives the same readings every run.
    soil = SimulatedSensor(31.0, drift_per_read=-0.5, noise=0.3, seed=7)
    # The radio loses every third report on its way to the base.
    radio = Transport.degraded(LoopbackBroker().rung(), drop_every=3)
    await radio.connect()

    moisture = []
    delivered = 0
    while True:
        # A replay that has handed back every reading reports that it is closed.
        try:
            clear = await ahead.read()
        except PamojaError as error:
            print(f"ahead     ran out after {len(moisture)} readings: {error}")
            break
        speed, turn = (1.0, 0.0) if clear > 1.0 else (0.0, 1.0)
        await drive.apply(speed)
        await rover.apply(vx=speed, omega=turn)
        wet = await soil.read()
        elapsed = len(moisture) * dt
        moisture.append(wet)
        report = f"{wet:.1f}"
        try:
            await radio.send("vineyard/row-4/soil", report)
            delivered += 1
        except PamojaError:
            pass
        print(
            f"{elapsed:.1f} s     {clear:.1f} m clear:"
            f" drive {speed:.1f}, turn {turn:.1f}, soil {report}"
        )

    # The drive kept every command, which is how a test says what the loop decided
    # rather than only what it ended up doing.
    commands = await drive.commands()
    print(f"drive     recorded {', '.join(f'{command:.1f}' for command in commands)}")

    # Three half-second commands at 1 m/s reach 1.5 m along x. The last turns on the
    # spot at 1 rad/s for half a second, which moves the rover nowhere.
    pose = await rover.pose()
    print(f"rover     ended at x {pose.x:.1f} m, y {pose.y:.1f} m, heading {pose.theta:.1f} rad")

    # A second probe with the same seed reads exactly the same values.
    twin = SimulatedSensor(31.0, drift_per_read=-0.5, noise=0.3, seed=7)
    again = [await twin.read() for _ in moisture]
    verdict = "the same" if again == moisture else "different"
    print(f"soil      a probe with the same seed read {verdict} {len(again)} values")
    print(f"radio     delivered {delivered} of {len(moisture)} soil reports and lost the third")

    return commands, pose, again == moisture, delivered


commands, pose, same, delivered = asyncio.run(main())
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-sim`](https://crates.io/crates/pamoja-sim) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html), [docs.rs](https://docs.rs/pamoja-sim), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sim) |
| TypeScript | [`@pamoja/sim`](https://www.npmjs.com/package/@pamoja/sim) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sim) |
| Python | [`pamoja-sim`](https://pypi.org/project/pamoja-sim/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sim) |
| C# | [`Pamoja.Sim`](https://www.nuget.org/packages/Pamoja.Sim) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sim) |

## Documentation

- [`pamoja.sim` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html), every class and function in this module.
- [The Simulators guide](https://pamoja.molex.cloud/docs/guides/sim.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
