"""The simulators guide example; see docs/guides/sim.md."""

# ANCHOR: example
import asyncio

from pamoja.sim import RecordingActuator, Replay, SimulatedRobot


async def main() -> None:
    # The clear distance ahead, in metres, taken from an earlier survey run. A replay
    # hands it back one reading at a time, so the loop below sees the same input on
    # every run.
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

    assert seen == capture
    assert await throttle.commands() == [1.0, 1.0, 1.0, 0.0]

    # Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on the
    # spot at 1 rad/s for half a second, which moves the rover nowhere.
    pose = await rover.pose()
    assert abs(pose.x - 1.5) < 1e-6
    assert abs(pose.y) < 1e-6
    assert abs(pose.theta - 0.5) < 1e-6


asyncio.run(main())
# ANCHOR_END: example
