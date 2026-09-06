# pamoja-sim

Noisy and replay sensors, a recording actuator, and a simulated robot that dead-reckons its pose. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/sim.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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

from pamoja.sim import RecordingActuator, Replay, SimulatedRobot


async def main() -> None:
    # The clear distance ahead, in metres, taken from an earlier survey run. A replay hands
    # it back one reading at a time, so the loop below sees the same input on every run: the
    # same rover code, driven by a recording rather than a range finder.
    capture = [4.0, 3.0, 1.5, 0.5]
    ahead = Replay(capture)
    throttle = RecordingActuator()
    rover = SimulatedRobot(0.5)  # each command advances the rover half a second

    seen = []
    for _ in capture:
        reading = await ahead.read()
        seen.append(reading)

        # Drive on while there is room ahead, otherwise stop and turn on the spot.
        clear = reading > 1.0
        speed = 1.0 if clear else 0.0
        turn = 0.0 if clear else 1.0
        await throttle.apply(speed)
        await rover.apply(vx=speed, omega=turn)
        print(f"{reading} m ahead, so drive at {speed} and turn at {turn}")

    # The recording actuator kept every command, which is how a test says what the control
    # loop decided rather than only what it ended up doing.
    commands = await throttle.commands()
    print(f"commands  {commands}")

    # Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on the
    # spot at 1 rad/s for half a second, which moves the rover nowhere.
    pose = await rover.pose()
    print(f"pose      x {pose.x:.1f} m, y {pose.y:.1f} m, heading {pose.theta:.1f} rad")

    return seen, commands, pose


seen, commands, pose = asyncio.run(main())
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
