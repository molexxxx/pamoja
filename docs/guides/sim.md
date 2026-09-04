# Simulators

The point of the simulators is that a node's logic can be tested without the node.
A replay sensor hands back a series that was recorded earlier, so the same input
runs every time. A recording actuator remembers what it was told to do instead of
doing it, so a test can assert on the commands rather than watch a motor. And a
simulated robot integrates the twists it is given into a pose, so guidance and
safety logic can be driven over a route with nothing built.

That makes the control loop itself the thing under test. Feed a captured series in,
drive the loop, and check both what it commanded and where the vehicle ended up.
Nothing here is random unless you ask for it: the replay is exactly the series it
was given, and the robot's pose follows from the kinematics, so an assertion can be
an exact value rather than a tolerance around a run.

There is a noisy sensor too, for when the question is how the loop behaves under
drift and jitter rather than what it does with a known input.

## What the example does

It replays four range readings through a simple drive-or-turn rule, recording each
throttle command and integrating each twist into the rover's pose.

It proves:

- A replay sensor returns exactly the series it was given, in order.
- A recording actuator keeps every command applied to it, so what the loop decided
  is checkable after the fact.
- The robot dead-reckons the pose the kinematics imply: three half-second commands
  at one metre per second travel 1.5 m, and a turn on the spot changes the heading
  without moving the vehicle.

## Rust

<!-- snippet: examples/tests/guides/sim.rs#example -->
From [`examples/tests/guides/sim.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/sim.rs):

```rust
use pamoja_core::{Actuator, Sensor};
use pamoja_kit::Twist;
use pamoja_sim::{RecordingActuator, Replay, SimRobot};

// The clear distance ahead, in metres, taken from an earlier survey run. A replay hands
// it back one reading at a time, so the loop below sees the same input on every run.
let capture = vec![4.0, 3.0, 1.5, 0.5];
let mut ahead = Replay::new(capture.clone());
let mut throttle = RecordingActuator::new();
let log = throttle.log();
let mut rover = SimRobot::new(0.5); // each command advances the rover half a second

let mut seen = Vec::new();
for _ in &capture {
    let reading = ahead.read().await.expect("a reading from the capture");
    seen.push(reading);
    // Drive on while there is room ahead, otherwise stop and turn on the spot.
    let clear = reading > 1.0;
    let speed = if clear { 1.0 } else { 0.0 };
    let turn = if clear { 0.0 } else { 1.0 };
    throttle.apply(speed).await.unwrap();
    rover.apply(Twist::planar(speed, turn)).await.unwrap();
}

assert_eq!(seen, capture);
assert_eq!(log.commands(), vec![1.0, 1.0, 1.0, 0.0]);

// Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on the
// spot at 1 rad/s for half a second, which moves the rover nowhere.
let pose = rover.pose();
assert!((pose.x - 1.5).abs() < 1e-6);
assert!(pose.y.abs() < 1e-6);
assert!((pose.theta - 0.5).abs() < 1e-6);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/sim.ts#example -->
From [`bindings/node/guides/sim.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/sim.ts):

```typescript
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
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/sim.py#example -->
From [`bindings/python/guides/sim.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/sim.py):

```python
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
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs):

```csharp
// The clear distance ahead, in metres, taken from an earlier survey run. A replay
// hands it back one reading at a time, so the loop below sees the same input on
// every run.
float[] capture = [4.0f, 3.0f, 1.5f, 0.5f];
using var ahead = new Replay(capture);
using var throttle = new RecordingActuator();
using var rover = new SimulatedRobot(0.5f); // each command advances half a second

var seen = new List<float>();
foreach (var _ in capture)
{
    var reading = await ahead.ReadAsync();
    seen.Add(reading);

    // Drive on while there is room ahead, otherwise stop and turn on the spot.
    var clear = reading > 1.0f;
    var vx = clear ? 1.0f : 0.0f;
    var omega = clear ? 0.0f : 1.0f;
    await throttle.ApplyAsync(vx);
    await rover.ApplyAsync(new Twist(vx, Omega: omega));
}

Expect(seen.SequenceEqual(capture), "the replay hands back the captured series");
Expect(
    throttle.Commands.SequenceEqual([1.0f, 1.0f, 1.0f, 0.0f]),
    "and the actuator recorded what it was told to do");

// Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on
// the spot at 1 rad/s for half a second, which moves the rover nowhere.
var pose = rover.Pose;
Expect(Math.Abs(pose.X - 1.5f) < 1e-6f, "1.5 m travelled along x");
Expect(Math.Abs(pose.Y) < 1e-6f, "with no sideways drift");
Expect(Math.Abs(pose.Theta - 0.5f) < 1e-6f, "and half a radian turned in place");
```
<!-- end -->

## Reference

<!-- table: reference sim -->
- Rust: [`pamoja-sim`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html)
- TypeScript: [`@pamoja/sim`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html)
- Python: [`pamoja.sim`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html)
- C#: [`Pamoja.Sim`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html)
<!-- end -->
