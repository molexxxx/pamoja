# Simulators

The point of the simulators is that a node's logic can be tested without the node.
A replay sensor hands back a series that was recorded earlier, so the same input
runs every time. A recording actuator remembers what it was told to do instead of
doing it, so a test can assert on the commands rather than watch a motor. A
simulated robot integrates the twists it is given into a pose, so guidance and
safety logic can be driven over a route with nothing built. A degraded link loses
sends on a pattern, so a node's handling of a bad radio can be tested at a desk.

That makes the control loop itself the thing under test. Feed a captured series in,
drive the loop, and check what it commanded, where the vehicle ended up, and what
reached the other end. Nothing here is random unless you ask for it, and even then
it is not: the noisy sensor draws its wobble from a seed, so an assertion can be an
exact value rather than a tolerance around a run.

Every simulator implements the same trait as the thing it stands in for, a sensor, an
actuator, or a transport, so the code under test does not change when the real part
arrives.

## What the example does

It drives a rover down a vineyard row with nothing built. The clear distance ahead is
a replay of an earlier survey of the row, the drive records the commands it is given,
the rover's pose comes from integrating each command over half a second, a soil probe
reads a seeded, drifting value, and the rover's radio loses every third report on its
way to the base. The loop drives while there is more than a meter clear and turns on
the spot when there is not.

It proves:

- The replay hands back exactly the series it was given, 4, 3, 1.5, and 0.5 meters,
  and then reports that it is closed, which is what ends the loop.
- The drive kept every command the loop issued: three at one meter per second, then a
  zero once the 0.5 m reading falls under the meter the rule drives on.
- Those three half-second commands dead-reckon to 1.5 m along x and nothing along y,
  and the turn on the spot leaves x where it was and the heading at 0.5 rad. An
  integrator that moved the rover on a pure rotation would carry it past 1.5 m and
  still look self-consistent.
- A second probe built with the same seed reads the same 4 values, bit for bit.
- The radio delivered 3 of the 4 reports and refused the third, as a link set to lose
  every third send does.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example sim" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example sim</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- sim" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- sim</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/sim.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/sim.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- sim" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- sim</code></div>
</div>
<!-- end -->

## Rust

In Rust, the simulators are in `pamoja-sim`. `SimSensor::new(baseline)` reads a steady value, and
`with_drift(per_read)`, `with_noise(amplitude)`, and `with_seed(seed)` make it drift, wobble, and
repeat; it implements `Sensor`, and it is `Copy`, so one configured probe can seed several.
`Replay::new(readings)` hands back a series once and then reports `Error::Closed`, and
`Replay::repeating(readings)` loops it. `RecordingActuator::new()` implements `Actuator`, and
`log()` gives a handle whose `commands()` and `len()` read what it was told, from anywhere.
`SimRobot::new(dt)` and `SimRobot::starting_at(pose, dt)` integrate each `Twist` over `dt`
seconds, and `pose()` reads where the robot is. `DegradedLink::new(link)` wraps any transport,
`drop_every(n)` refuses every nth send, and `intermittent(up, down)` carries `up` sends and then
refuses `down`, round and round; connecting, subscribing, and receiving pass straight through.

<!-- snippet: examples/guides/sim.rs#example -->
From [`examples/guides/sim.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/sim.rs):

```rust
use pamoja_core::{Actuator, Sensor, Transport};
use pamoja_kit::Twist;
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_sim::{DegradedLink, RecordingActuator, Replay, SimRobot, SimSensor};

// The clear distance ahead, in meters, replayed from an earlier survey of the row,
// so every run sees the same row.
let mut ahead = Replay::new(vec![4.0, 3.0, 1.5, 0.5]);
// The drive keeps the commands it is given instead of turning a motor.
let mut drive = RecordingActuator::new();
let told = drive.log();
// The rover's pose comes from integrating each command over half a second.
let dt = 0.5;
let mut rover = SimRobot::new(dt);
// The soil probe reads around 31 percent, drying half a point a reading, with a
// wobble drawn from a seed, so the same seed gives the same readings every run.
let mut soil = SimSensor::new(31.0)
    .with_drift(-0.5)
    .with_noise(0.3)
    .with_seed(7);
// The radio loses every third report on its way to the base.
let mut radio = DegradedLink::new(LoopbackTransport::new(LoopbackBroker::new())).drop_every(3);
radio.connect().await?;

let mut moisture = Vec::new();
let mut delivered = 0;
loop {
    // A replay that has handed back every reading reports that it is closed.
    let clear = match ahead.read().await {
        Ok(clear) => clear,
        Err(error) => {
            println!(
                "ahead     ran out after {} readings: {error}",
                moisture.len()
            );
            break;
        }
    };
    let (speed, turn) = if clear > 1.0 { (1.0, 0.0) } else { (0.0, 1.0) };
    drive.apply(speed).await?;
    rover.apply(Twist::planar(speed, turn)).await?;
    let wet = soil.read().await?;
    let elapsed = moisture.len() as f32 * dt;
    moisture.push(wet);
    let report = format!("{wet:.1}");
    if radio
        .send_text("vineyard/row-4/soil", &report)
        .await
        .is_ok()
    {
        delivered += 1;
    }
    println!("{elapsed:.1} s     {clear:.1} m clear: drive {speed:.1}, turn {turn:.1}, soil {report}");
}

// The drive kept every command, which is how a test says what the loop decided
// rather than only what it ended up doing.
let commands: Vec<String> = told.commands().iter().map(|c| format!("{c:.1}")).collect();
println!("drive     recorded {}", commands.join(", "));

// Three half-second commands at 1 m/s reach 1.5 m along x. The last turns on the
// spot at 1 rad/s for half a second, which moves the rover nowhere.
let pose = rover.pose();
println!(
    "rover     ended at x {:.1} m, y {:.1} m, heading {:.1} rad",
    pose.x, pose.y, pose.theta
);

// A second probe with the same seed reads exactly the same values.
let mut twin = SimSensor::new(31.0)
    .with_drift(-0.5)
    .with_noise(0.3)
    .with_seed(7);
let mut again = Vec::new();
for _ in 0..moisture.len() {
    again.push(twin.read().await?);
}
let verdict = if again == moisture {
    "the same"
} else {
    "different"
};
println!(
    "soil      a probe with the same seed read {verdict} {} values",
    again.len()
);
println!(
    "radio     delivered {delivered} of {} soil reports and lost the third",
    moisture.len()
);
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/sim` has `new SimulatedSensor(baseline, driftPerRead?, noise?, seed?)`,
`new Replay(readings, repeating?)`, `new RecordingActuator()`, and `new SimulatedRobot(dt,
start?)`. `read()` on a sensor or a replay resolves with a number, and a finished one-shot replay
rejects with `resource is closed`. `apply(command)` gives the actuator a number and the robot a
twist, `{ vx, vy, omega }`, and `commands()`, `length()`, and `pose()` read them back, each as a
promise. `Transport.degraded(transport, { dropEvery?, up?, down? })` from `@pamoja/core` wraps any
transport in a degraded link, consuming it.

<!-- snippet: bindings/node/guides/sim.ts#example -->
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
<!-- end -->

## Python

In Python, `pamoja.sim` has `SimulatedSensor(baseline, *, drift_per_read=None, noise=None,
seed=None)`, `Replay(readings, *, repeating=False)`, `RecordingActuator()`, and
`SimulatedRobot(dt, *, x=0.0, y=0.0, theta=0.0)`. `read()` on a sensor or a replay is a
coroutine that returns a float, and a finished one-shot replay raises `PamojaError`.
`apply(command)` gives the actuator a number, `apply(vx=..., vy=..., omega=...)` gives the robot a
twist, and `commands()` and `pose()` read them back. `Transport.degraded(transport, drop_every=0,
up=0, down=0)` from `pamoja.core` wraps any transport in a degraded link, consuming it.

<!-- snippet: bindings/python/guides/sim.py#example -->
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
<!-- end -->

## C#

In C#, `Pamoja.Sim` has `new SimulatedSensor(baseline, drift, noise, seed)`, `new
Replay(readings, repeating)`, `new RecordingActuator()`, and `new SimulatedRobot(dt, start)`,
each disposable. `ReadAsync()` on a sensor or a replay returns a `float`, and a finished one-shot
replay throws `PamojaException`. `ApplyAsync(command)` gives the actuator a `float` and the robot
a `Twist`, and the `Commands`, `Count`, and `Pose` properties read them back.
`Transport.Degraded(transport, dropEvery, up, down)` in `Pamoja.Core` wraps any transport in a
degraded link, consuming it. Format a simulated reading with `CultureInfo.InvariantCulture`, as
the example does, so a test does not read `1,5` on a machine set to another culture.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/SimGuide.cs):

```csharp
// The clear distance ahead, in meters, replayed from an earlier survey of the
// row, so every run sees the same row.
using var ahead = new Replay([4.0f, 3.0f, 1.5f, 0.5f]);
// The drive keeps the commands it is given instead of turning a motor.
using var drive = new RecordingActuator();
// The rover's pose comes from integrating each command over half a second.
const float Dt = 0.5f;
using var rover = new SimulatedRobot(Dt);
// The soil probe reads around 31 percent, drying half a point a reading, with a
// wobble drawn from a seed, so the same seed gives the same readings every run.
using var soil = new SimulatedSensor(31.0f, -0.5f, 0.3f, 7);
// The radio loses every third report on its way to the base.
using var station = new LoopbackBroker();
using Transport radio = Transport.Degraded(station.Rung(), dropEvery: 3);
await radio.ConnectAsync();

List<float> moisture = [];
int delivered = 0;
while (true)
{
    // A replay that has handed back every reading reports that it is closed.
    float clear;
    try
    {
        clear = await ahead.ReadAsync();
    }
    catch (PamojaException error)
    {
        Console.WriteLine($"ahead     ran out after {moisture.Count} readings: {error.Message}");
        break;
    }

    (float speed, float turn) = clear > 1.0f ? (1.0f, 0.0f) : (0.0f, 1.0f);
    await drive.ApplyAsync(speed);
    await rover.ApplyAsync(new Twist(speed, Omega: turn));
    float wet = await soil.ReadAsync();
    float elapsed = moisture.Count * Dt;
    moisture.Add(wet);
    string report = Tenths(wet);
    try
    {
        await radio.SendAsync("vineyard/row-4/soil", report);
        delivered++;
    }
    catch (PamojaException)
    {
    }

    Console.WriteLine(
        $"{Tenths(elapsed)} s     {Tenths(clear)} m clear:"
        + $" drive {Tenths(speed)}, turn {Tenths(turn)}, soil {report}");
}

// The drive kept every command, which is how a test says what the loop decided
// rather than only what it ended up doing.
Console.WriteLine($"drive     recorded {string.Join(", ", drive.Commands.Select(Tenths))}");

// Three half-second commands at 1 m/s reach 1.5 m along x. The last turns on
// the spot at 1 rad/s for half a second, which moves the rover nowhere.
Pose pose = rover.Pose;
Console.WriteLine(
    $"rover     ended at x {Tenths(pose.X)} m, y {Tenths(pose.Y)} m, heading {Tenths(pose.Theta)} rad");

// A second probe with the same seed reads exactly the same values.
using var twin = new SimulatedSensor(31.0f, -0.5f, 0.3f, 7);
List<float> again = [];
foreach (float _ in moisture)
{
    again.Add(await twin.ReadAsync());
}

string verdict = again.SequenceEqual(moisture) ? "the same" : "different";
Console.WriteLine($"soil      a probe with the same seed read {verdict} {again.Count} values");
Console.WriteLine($"radio     delivered {delivered} of {moisture.Count} soil reports and lost the third");
```
<!-- end -->

## Values at a glance

**The simulators in each language:**

| Simulator | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| a noisy sensor | `SimSensor::new(baseline)` | `new SimulatedSensor(baseline)` | `SimulatedSensor(baseline)` | `new SimulatedSensor(baseline)` |
| a replay | `Replay::new(readings)` | `new Replay(readings)` | `Replay(readings)` | `new Replay(readings)` |
| a recording actuator | `RecordingActuator::new()` | `new RecordingActuator()` | `RecordingActuator()` | `new RecordingActuator()` |
| a simulated robot | `SimRobot::new(dt)` | `new SimulatedRobot(dt)` | `SimulatedRobot(dt)` | `new SimulatedRobot(dt)` |
| a degraded link | `DegradedLink::new(link).drop_every(n)` | `Transport.degraded(link, { dropEvery: n })` | `Transport.degraded(link, drop_every=n)` | `Transport.Degraded(link, dropEvery: n)` |

**The noisy sensor's settings.** A reading is the baseline, plus the drift so far, plus a
wobble:

| Setting | What it does | Unset |
| --- | --- | --- |
| baseline | the value the sensor starts at | required |
| drift per read | added to the baseline after each reading; negative to sag | 0 |
| noise | the wobble's largest size, above or below | 0, a clean signal |
| seed | where the wobble's sequence starts; the same seed gives the same readings | a fixed seed, the same every run |

**The degraded link's patterns,** counted in sends, so a test runs the same every time:

| Pattern | Rust | TypeScript | Python | C# | Refuses |
| --- | --- | --- | --- | --- | --- |
| lossy | `drop_every(3)` | `{ dropEvery: 3 }` | `drop_every=3` | `dropEvery: 3` | the 3rd, 6th, 9th send |
| comes and goes | `intermittent(2, 3)` | `{ up: 2, down: 3 }` | `up=2, down=3` | `up: 2, down: 3` | sends 3 to 5, then 8 to 10 |
| both | `drop_every(n).intermittent(u, d)` | `{ dropEvery, up, down }` | all three | all three | either pattern's sends |

A refused send fails with `transport error: packet lost on a lossy link` or `transport error:
link unreachable`, the errors a ladder or a drain handles the way it handles a real radio's.

**How the robot moves.** Each command is a twist held for `dt` seconds, integrated along the arc
it describes rather than in a straight line:

| Command | Over 0.5 s from x 0, heading 0 |
| --- | --- |
| drive at 1 m/s | 0.5 m along x |
| turn at 1 rad/s on the spot | the heading moves 0.5 rad, and x and y stay |
| drive at 1 m/s while turning at 1 rad/s | an arc that ends short of 0.5 m along x, turned 0.5 rad |

## When it goes wrong

What the simulators say:

| What happened | The message | What to check |
| --- | --- | --- |
| a one-shot replay read past its end | `resource is closed` | a loop that ends on it, as the example does, or `repeating` |
| an empty replay read at all | `resource is closed` | the series it was given |
| a send a lossy link dropped | `transport error: packet lost on a lossy link` | nothing, when the test is about loss |
| a send in a down window | `transport error: link unreachable` | nothing, when the test is about an outage |

The mistakes that cost an afternoon:

- **A test passes alone and fails beside another.** Two tests shared one simulator and each
  read part of its sequence. Give every test its own sensor, replay, and link.
- **An assertion on a noisy reading flickers.** It does not with a seed: the sequence is fixed
  by it. If a test flickers, it read the probe a different number of times on some runs.
- **The robot ends up somewhere the arithmetic says it should not.** `dt` is how long each
  command is held. A loop that runs faster or slower than `dt` in the real node disagrees with
  its simulation; match them.
- **A loss pattern drops a different send than the test expected.** Patterns count every send
  through the link, including any the code makes before the one the test is watching, a
  greeting on connect for one. Count them all.
- **A reading prints `1,5` in C#.** Formatting uses the machine's culture. Pass
  `CultureInfo.InvariantCulture`, as the example does.
- **Everything works in simulation and fails on the bench.** A simulator is as honest as what it
  was told. Record a replay from the real part, and set the drift and noise from its datasheet.

## Where next

<!-- table: next sim -->
- [Loopback](loopback.md): An in-process transport with topic matching, a fault injector, and outages on demand, for testing with no broker.
- [Your own device](device.md): A sensor and an actuator pamoja has never heard of, written against the core traits, run against a rule, and published with nothing plugged in.
- [Device profiles](profile.md): Named, ready-to-run device profiles from plain data or a JSON manifest, run as a node in every language.
- Also in Transports and testing: [MQTT](mqtt.md), [CoAP](coap.md), [Store and forward](sync.md), [Transport ladder](ladder.md), [Event bus](bus.md), [Engine surface](transport.md), [Your own link](link.md).
<!-- end -->

## Reference

<!-- table: reference sim -->
- Rust: [`pamoja-sim`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_sim/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-sim)
- TypeScript: [`@pamoja/sim`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_sim.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-sim)
- Python: [`pamoja.sim`](https://pamoja.molex.cloud/docs/reference/python/pamoja/sim.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-sim)
- C#: [`Pamoja.Sim`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Sim.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-sim)
<!-- end -->
