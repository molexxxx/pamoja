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

It replays four range readings from an earlier survey through a drive-or-turn
rule, recording every throttle command and integrating every twist into the
rover's pose. Nothing is wired up: the range finder is the capture played back
one reading at a time, the throttle keeps its commands instead of turning a
motor, and the rover advances half a second per command.

The readings and the rule are the only things written out. The pose is not; the
simulated robot integrates each twist with the same exact-arc odometry a real
rover runs, so the 1.5 m and the half radian the run ends on come out of the
kinematics rather than being typed in.

It proves:

- The replay hands back exactly the series it was given: 4 m, 3 m, 1.5 m and
  0.5 m, in that order.
- The recording actuator keeps every command the loop issued: three at one metre
  per second, then a zero once the 0.5 m reading falls under the metre of
  clearance the rule drives on.
- Those three half-second commands dead-reckon to 1.5 m along x and nothing along
  y, so a straight run stays straight.
- The turn on the spot puts the heading at 0.5 rad and leaves x at 1.5 m; an
  integrator that translated on a pure rotation would carry the rover past that
  and still look self-consistent.

## Rust

<!-- snippet: examples/tests/guides/sim.rs#example -->
From [`examples/tests/guides/sim.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/sim.rs):

```rust
use pamoja_core::{Actuator, Sensor};
use pamoja_kit::Twist;
use pamoja_sim::{RecordingActuator, Replay, SimRobot};

// The clear distance ahead, in metres, taken from an earlier survey run. A replay
// hands it back one reading at a time, so the loop below sees the same input on every
// run: the same rover code, driven by a recording rather than a range finder.
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
    throttle.apply(speed).await.expect("the throttle takes it");
    rover
        .apply(Twist::planar(speed, turn))
        .await
        .expect("the rover takes it");
    println!("{reading} m ahead, so drive at {speed} and turn at {turn}");
}

// The recording actuator kept every command, which is how a test says what the control
// loop decided rather than only what it ended up doing.
println!("commands  {:?}", log.commands());

// Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on the
// spot at 1 rad/s for half a second, which moves the rover nowhere.
let pose = rover.pose();
let (x, y, heading) = (pose.x, pose.y, pose.theta);
println!("pose      x {x:.1} m, y {y:.1} m, heading {heading:.1} rad");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/sim.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/sim.py#example -->
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
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs):

```csharp
// The clear distance ahead, in metres, taken from an earlier survey run. A replay
// hands it back one reading at a time, so the loop below sees the same input on
// every run: the same rover code, driven by a recording rather than a range finder.
float[] capture = [4.0f, 3.0f, 1.5f, 0.5f];
using var ahead = new Replay(capture);
using var throttle = new RecordingActuator();
using var rover = new SimulatedRobot(0.5f); // each command advances half a second

List<float> seen = [];
for (int step = 0; step < capture.Length; step++)
{
    float reading = await ahead.ReadAsync();
    seen.Add(reading);

    // Drive on while there is room ahead, otherwise stop and turn on the spot.
    bool clear = reading > 1.0f;
    float vx = clear ? 1.0f : 0.0f;
    float omega = clear ? 0.0f : 1.0f;
    await throttle.ApplyAsync(vx);
    await rover.ApplyAsync(new Twist(vx, Omega: omega));
    Console.WriteLine($"{reading} m ahead, so drive at {vx} and turn at {omega}");
}

// The recording actuator kept every command, which is how a test says what the
// control loop decided rather than only what it ended up doing.
Console.WriteLine($"commands  {string.Join(", ", throttle.Commands)}");

// Three half-second commands at 1 m/s reach 1.5 m along x. The last one turns on
// the spot at 1 rad/s for half a second, which moves the rover nowhere.
Pose pose = rover.Pose;
Console.WriteLine(
    $"pose      x {pose.X:F1} m, y {pose.Y:F1} m, heading {pose.Theta:F1} rad");
```
<!-- end -->

## Reference

<!-- table: reference sim -->
- Rust: [`pamoja-sim`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sim)
- TypeScript: [`@pamoja/sim`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sim)
- Python: [`pamoja.sim`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sim)
- C#: [`Pamoja.Sim`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sim)
<!-- end -->
