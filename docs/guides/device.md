# Your own device

pamoja does not need to have heard of a part to run it. Everything above the wire is a
trait: a `Sensor` is anything that can be asked for a reading, an `Actuator` is anything
that can be told a command, and the reading and command types are the maker's to
choose. The shipped decoders, the kit helpers, the profiles, and the ladder are written
against those two traits, so a probe built on a kitchen table gets the same treatment as
the BME280 in the sensors guide.

Writing one is the whole of what this guide shows. The parts here are a capacitive
soil-moisture probe on an analog-to-digital converter and a solenoid valve on a relay,
neither of which pamoja ships. What reads the converter is the maker's business, so a
replay stands in for it; on a running node the converter's own driver hands back the
same counts, and nothing downstream changes.

## What the example does

It defines the probe and the valve, waters a raised bed from six readings of a bed that
dries out and is watered, and reports every reading to a gateway over a link that is
down for the first two. Then it hands the same two parts to a profile of the maker's own,
built from its parts, which decides from two more readings.

The probe is calibrated from two measurements kept from the day it was built: 3200
counts bone dry and 1400 in a soaked pot. The rule is a hysteresis band, water below
30% and stop above 45%, which the kit's `Thermostat` provides once told that a valve
adds moisture the way a heater adds heat. The ladder buffers the two readings the link
refuses and replays them in order once a send goes through, so the gateway sees all six
in the order they were read. The profile holds the same band as data, with an alert once
the bed is more than 15 points from its 37.5% target.

It proves:

- A struct or class of the maker's own is a sensor the moment it can be read and an
  actuator the moment it can be told, with no registration and no base class.
- The probe reports percent because it does its own calibration; the converter
  underneath is any source of counts, a replay here and a driver on the node.
- 2900 counts is 16.7% and 2300 is 50.0%: a capacitive probe reads higher the drier the
  soil, so bone dry is the high end of the calibration and soaked the low end.
- The valve opens on the first reading, closes at 50%, and opens again at 25%: three
  changes, with the readings inside the band leaving it as it was.
- The first two readings are buffered rather than lost, both go out when the link
  returns, and the gateway receives all six in order.
- Under the profile, 16.7% opens the valve and raises an out-of-range alert, 20.8 points
  from target, and 50.0% closes it with no alert, 12.5 points from it. In Rust a `Node`
  runs that loop and publishes each reading; in the other languages the profile's
  controller decides and the program acts.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example device" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example device</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- device" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- device</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/device.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/device.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- device" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- device</code></div>
</div>
<!-- end -->

## Rust

In Rust, a part is a type that implements `Sensor` or `Actuator` from `pamoja-core`. Each
names its own reading or command type and has one async method, `read` or `apply`,
returning a `pamoja_core::Result`. A read that cannot happen returns `Error::Io`, and
one on a part that has gone away `Error::Closed`, which is what a `Replay` returns once it
runs out. The futures are `Send`, so a part generic over a bus or a converter needs a
`Send` bound on it, as `SoilProbe` has; that is what lets a profile's `Node` own the parts
and run from a spawned task. A driver whose reading or command has another shape plugs in
without a wrapper type: `map` selects or converts each reading, and `map_command`
converts each command.

The parts:

<!-- snippet: examples/guides/device.rs#parts -->
From [`examples/guides/device.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/device.rs):

```rust
use pamoja_core::{Actuator, Result, Sensor};
use pamoja_kit::Calibration;

/// A capacitive soil probe on an analog-to-digital converter. Whatever reads the chip is
/// `adc`: any sensor that hands back counts, so a replay stands in for it here and the
/// converter's own driver does on the node. The probe turns counts into percent, and
/// nothing downstream needs to know there was a chip at all.
struct SoilProbe<A> {
    adc: A,
    calibration: Calibration,
}

impl<A: Sensor<Reading = f32> + Send> Sensor for SoilProbe<A> {
    type Reading = f32;

    async fn read(&mut self) -> Result<f32> {
        let counts = self.adc.read().await?;
        Ok(self.calibration.apply(counts))
    }
}

/// A solenoid valve on a relay. It keeps what it was last told and counts the changes,
/// which is what a test needs and what a real one does before it drives the pin.
#[derive(Default)]
struct Valve {
    open: bool,
    switched: u32,
}

impl Actuator for Valve {
    type Command = bool;

    async fn apply(&mut self, open: bool) -> Result<()> {
        if open != self.open {
            self.open = open;
            self.switched += 1;
        }
        Ok(())
    }
}
```
<!-- end -->

The loop:

<!-- snippet: examples/guides/device.rs#example -->
From [`examples/guides/device.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/device.rs):

```rust
use pamoja_core::{Receive, Transport};
use pamoja_kit::Thermostat;
use pamoja_ladder::TransportLadder;
use pamoja_loopback::{Faulty, LoopbackBroker, LoopbackTransport};
use pamoja_sim::Replay;
use pamoja_sync::MemoryStore;

let topic = "garden/bed-1/moisture";

// Bone dry read 3200 counts on this probe and a soaked pot read 1400, measured once
// and kept. The replay hands back the counts a bed reads as it dries and is watered.
let counts = Replay::new(vec![2900.0, 2700.0, 2300.0, 2450.0, 2750.0, 2500.0]);
let mut probe = SoilProbe {
    adc: counts,
    calibration: Calibration::two_point(3200.0, 0.0, 1400.0, 100.0),
};
let mut valve = Valve::default();

// Water below 30% and stop above 45%. A valve that adds water is what `heating`
// names, so the band sits at 37.5 with 7.5 either side.
let mut rule = Thermostat::heating(37.5, 7.5);

// The link, with its first two sends failing the way a radio does at dusk. The ladder
// keeps what it could not send and replays it in order once a send goes through.
let broker = LoopbackBroker::new();
let mut gateway = LoopbackTransport::new(broker.clone());
gateway.connect().await?;
gateway.subscribe(topic).await?;
let flaky = Faulty::new(LoopbackTransport::new(broker), 2);
let mut ladder = TransportLadder::new(MemoryStore::new()).rung(flaky);
ladder.connect().await?;

// The loop: read, decide, act, publish. Nothing in it knows the probe is homemade.
for _ in 0..6 {
    let moisture = probe.read().await.expect("a reading");
    let wanted = rule.update(moisture);
    valve.apply(wanted).await.expect("the valve answers");
    let report = format!("{moisture:.1}");
    let payload = report.as_bytes();
    let delivery = ladder.send(topic, payload).await?;
    let state = if valve.open { "open" } else { "closed" };
    println!("bed at {report}%, valve {state}, {delivery:?}");
    if ladder.buffered().await.expect("a count") > 0 {
        let caught_up = ladder.flush().await?;
        if caught_up > 0 {
            println!("link back, {caught_up} readings caught up");
        }
    }
}
let switched = valve.switched;
println!("the valve switched {switched} times");

// On the gateway, in the order they were read, outage included.
let mut got = Vec::new();
for _ in 0..6 {
    let message = gateway.recv().await?.expect("a message");
    got.push(message.text().expect("text").to_owned());
}
let readings = got.join(", ");
println!("gateway got {readings}");
```
<!-- end -->

The same parts under a profile of the maker's own:

<!-- snippet: examples/guides/device.rs#node -->
From [`examples/guides/device.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/device.rs):

```rust
use pamoja_codec::CborCodec;
use pamoja_core::Transport;
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_profile::{ControlSpec, Node, PowerSchedule, Profile};
use pamoja_sim::Replay;

// The same band as a profile rather than a line of code, with an alert once the bed
// is more than 15 points from target. No preset is involved: this is the maker's own
// profile, sampling every 5 minutes, every 30 as the battery runs low, and hourly
// when it is nearly flat, and it saves to JSON the same as a shipped one.
let band = ControlSpec::Setpoint {
    setpoint: 37.5,
    hysteresis: 7.5,
    cooling: false,
    safe_band: 15.0,
};
let schedule = PowerSchedule::new(300, 1800, 3600);
let profile = Profile::new("raised-bed-drip", "garden/bed-1/moisture", band, schedule);
let probe = SoilProbe {
    adc: Replay::new(vec![2900.0, 2300.0]),
    calibration: Calibration::two_point(3200.0, 0.0, 1400.0, 100.0),
};
let mut link = LoopbackTransport::new(LoopbackBroker::new());
link.connect().await?;

// A node reads, decides, drives the valve, and publishes on every tick. The parts are
// the ones above; only the loop moved into the library.
let mut node = Node::new(profile, probe, Valve::default(), link, CborCodec)
    .expect("the profile names a built-in kind");
let mut reactions = Vec::new();
for _ in 0..2 {
    let reaction = node.tick().await.expect("a tick");
    let valve = match reaction.actuator {
        Some(true) => "open",
        Some(false) => "closed",
        None => "untouched",
    };
    let alert = reaction.alert.map_or("none", |alert| alert.kind());
    println!("under the profile: valve {valve}, alert {alert}");
    reactions.push(reaction);
}
```
<!-- end -->

## TypeScript

In TypeScript there is no trait to implement. Any object with `read(): Promise<number>`
is a sensor to the program, and any object with `apply(command): Promise<void>` an
actuator, because the loop that calls them is the program's own; the helpers, the
controller, and the ladder take plain numbers, booleans, and strings. A read that fails
throws, and a `Replay` that has run out throws `resource is closed`. A profile of the
program's own is `new Profile(name, topic, control, power)`, where the control is a plain
object such as `{ kind: ControlKind.Setpoint, setpoint, hysteresis, safeBand }` and the
power `{ activeSecs, saverSecs, criticalSecs }`. Its `controller()` decides, and in place
of Rust's `Node` the program reads, drives, and publishes around it.

The parts:

<!-- snippet: bindings/node/guides/device.ts#parts -->
From [`bindings/node/guides/device.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/device.ts):

```typescript
import { Transport } from '@pamoja/core'
import { Calibration, Thermostat } from '@pamoja/kit'
import { Ladder } from '@pamoja/ladder'
import { LoopbackBroker } from '@pamoja/loopback'
import { Replay } from '@pamoja/sim'
import { Store } from '@pamoja/sync'

// A capacitive soil probe on an analog-to-digital converter. Whatever reads the chip is
// `adc`: anything with a `read` that hands back counts, so a replay stands in for it here
// and the converter's own driver does on the node. The probe turns counts into percent,
// and nothing downstream needs to know there was a chip at all.
class SoilProbe {
  constructor(
    private readonly adc: { read(): Promise<number> },
    private readonly calibration: Calibration,
  ) {}

  async read(): Promise<number> {
    return this.calibration.apply(await this.adc.read())
  }
}

// A solenoid valve on a relay. It keeps what it was last told and counts the changes,
// which is what a test needs and what a real one does before it drives the pin.
class Valve {
  open = false
  switched = 0

  async apply(open: boolean): Promise<void> {
    if (open !== this.open) {
      this.open = open
      this.switched += 1
    }
  }
}
```
<!-- end -->

The loop:

<!-- snippet: bindings/node/guides/device.ts#example -->
From [`bindings/node/guides/device.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/device.ts):

```typescript
const TOPIC = 'garden/bed-1/moisture'

async function main() {
  // Bone dry read 3200 counts on this probe and a soaked pot read 1400, measured once and
  // kept. The replay hands back the counts a bed reads as it dries and is watered.
  const counts = new Replay([2900, 2700, 2300, 2450, 2750, 2500])
  const probe = new SoilProbe(counts, Calibration.twoPoint(3200, 0, 1400, 100))
  const valve = new Valve()

  // Water below 30% and stop above 45%. A valve that adds water is what `heating` names,
  // so the band sits at 37.5 with 7.5 either side.
  const rule = Thermostat.heating(37.5, 7.5)

  // The link, with its first two sends failing the way a radio does at dusk. The ladder
  // keeps what it could not send and replays it in order once a send goes through.
  const broker = new LoopbackBroker()
  const gateway = broker.link()
  await gateway.connect()
  await gateway.subscribe(TOPIC)
  const ladder = new Ladder(Store.memory())
  await ladder.rung(Transport.faulty(broker.rung(), 2))
  await ladder.connect()

  // The loop: read, decide, act, publish. Nothing in it knows the probe is homemade.
  for (let sample = 0; sample < 6; sample += 1) {
    const moisture = await probe.read()
    await valve.apply(rule.update(moisture))
    const report = moisture.toFixed(1)
    const delivery = await ladder.send(TOPIC, report)
    console.log(`bed at ${report}%, valve ${valve.open ? 'open' : 'closed'}, ${delivery}`)
    if ((await ladder.buffered()) > 0) {
      const caughtUp = await ladder.flush()
      if (caughtUp > 0) console.log(`link back, ${caughtUp} readings caught up`)
    }
  }
  console.log(`the valve switched ${valve.switched} times`)

  // On the gateway, in the order they were read, outage included.
  const got: string[] = []
  for (let n = 0; n < 6; n += 1) {
    got.push((await gateway.recv())!.text!)
  }
  console.log(`gateway got ${got.join(', ')}`)

  return { got, switched: valve.switched, open: valve.open, left: await ladder.buffered() }
}

main()
```
<!-- end -->

The same parts under a profile of the maker's own:

<!-- snippet: bindings/node/guides/device.ts#profile -->
From [`bindings/node/guides/device.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/device.ts):

```typescript
import { ControlKind, Profile, type Reaction } from '@pamoja/profile'

async function underAProfile(): Promise<Reaction[]> {
  // The same band as a profile rather than a line of code, with an alert once the bed is
  // more than 15 points from target. No preset is involved: this is the maker's own
  // profile, sampling every 5 minutes, every 30 as the battery runs low, and hourly when it
  // is nearly flat, and it saves to JSON the same as a shipped one.
  const band = { kind: ControlKind.Setpoint, setpoint: 37.5, hysteresis: 7.5, safeBand: 15 }
  const schedule = { activeSecs: 300, saverSecs: 1800, criticalSecs: 3600 }
  const profile = new Profile('raised-bed-drip', TOPIC, band, schedule)
  const probe = new SoilProbe(new Replay([2900, 2300]), Calibration.twoPoint(3200, 0, 1400, 100))
  const valve = new Valve()

  // In Rust a Node runs this loop. Here the profile's controller decides, and the program
  // reads the probe and drives the valve itself.
  const controller = profile.controller()
  const reactions: Reaction[] = []
  for (let tick = 0; tick < 2; tick += 1) {
    const reaction = controller.evaluate(await probe.read())
    if (reaction.actuator != null) await valve.apply(reaction.actuator)
    const state = reaction.actuator == null ? 'untouched' : reaction.actuator ? 'open' : 'closed'
    console.log(`under the profile: valve ${state}, alert ${reaction.alert?.kind ?? 'none'}`)
    reactions.push(reaction)
  }
  return reactions
}
```
<!-- end -->

## Python

In Python, a part is any class with an `async def read(self)` or an
`async def apply(self, command)`; nothing checks for a base class, and the loop is the
program's own. A read that fails raises, and a `Replay` that has run out raises
`PamojaError` with `resource is closed`. A profile of the program's own is
`Profile(name, topic, control, power)`, with the control a
`ControlPolicy(ControlKind.SETPOINT, setpoint=..., hysteresis=..., safe_band=...)` and
the power a `PowerScheduleSpec(active_secs, saver_secs, critical_secs)`, whose thresholds
are 50% and 20% charge unless given. Its `controller()` decides, and the program acts on
what it says.

The parts:

<!-- snippet: bindings/python/guides/device.py#parts -->
From [`bindings/python/guides/device.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/device.py):

```python
import asyncio

from pamoja.core import Transport
from pamoja.kit import Calibration, Thermostat
from pamoja.ladder import Ladder
from pamoja.loopback import LoopbackBroker
from pamoja.sim import Replay
from pamoja.sync import Store


class SoilProbe:
    """A capacitive soil probe on an analog-to-digital converter.

    Whatever reads the chip is ``adc``: anything with a ``read`` that hands back counts,
    so a replay stands in for it here and the converter's own driver does on the node.
    The probe turns counts into percent, and nothing downstream needs to know there was
    a chip at all.
    """

    def __init__(self, adc, calibration: Calibration) -> None:
        self.adc = adc
        self.calibration = calibration

    async def read(self) -> float:
        return self.calibration.apply(await self.adc.read())


class Valve:
    """A solenoid valve on a relay.

    It keeps what it was last told and counts the changes, which is what a test needs
    and what a real one does before it drives the pin.
    """

    def __init__(self) -> None:
        self.open = False
        self.switched = 0

    async def apply(self, open: bool) -> None:
        if open != self.open:
            self.open = open
            self.switched += 1
```
<!-- end -->

The loop:

<!-- snippet: bindings/python/guides/device.py#example -->
From [`bindings/python/guides/device.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/device.py):

```python
TOPIC = "garden/bed-1/moisture"


async def main():
    # Bone dry read 3200 counts on this probe and a soaked pot read 1400, measured once
    # and kept. The replay hands back the counts a bed reads as it dries and is watered.
    counts = Replay([2900.0, 2700.0, 2300.0, 2450.0, 2750.0, 2500.0])
    probe = SoilProbe(counts, Calibration.two_point(3200.0, 0.0, 1400.0, 100.0))
    valve = Valve()

    # Water below 30% and stop above 45%. A valve that adds water is what `heating`
    # names, so the band sits at 37.5 with 7.5 either side.
    rule = Thermostat.heating(37.5, 7.5)

    # The link, with its first two sends failing the way a radio does at dusk. The ladder
    # keeps what it could not send and replays it in order once a send goes through.
    broker = LoopbackBroker()
    gateway = broker.link()
    await gateway.connect()
    await gateway.subscribe(TOPIC)
    ladder = Ladder(Store.memory())
    await ladder.rung(Transport.faulty(broker.rung(), 2))
    await ladder.connect()

    # The loop: read, decide, act, publish. Nothing in it knows the probe is homemade.
    for _ in range(6):
        moisture = await probe.read()
        await valve.apply(rule.update(moisture))
        report = f"{moisture:.1f}"
        delivery = await ladder.send(TOPIC, report.encode())
        state = "open" if valve.open else "closed"
        print(f"bed at {report}%, valve {state}, {delivery}")
        if await ladder.buffered() > 0:
            caught_up = await ladder.flush()
            if caught_up > 0:
                print(f"link back, {caught_up} readings caught up")
    print(f"the valve switched {valve.switched} times")

    # On the gateway, in the order they were read, outage included.
    got = [(await gateway.recv()).text for _ in range(6)]
    print(f"gateway got {', '.join(got)}")
    return got, valve, await ladder.buffered()


got, valve, left = asyncio.run(main())
```
<!-- end -->

The same parts under a profile of the maker's own:

<!-- snippet: bindings/python/guides/device.py#profile -->
From [`bindings/python/guides/device.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/device.py):

```python
async def under_a_profile():
    from pamoja.profile import ControlKind, ControlPolicy, PowerScheduleSpec, Profile

    # The same band as a profile rather than a line of code, with an alert once the bed is
    # more than 15 points from target. No preset is involved: this is the maker's own
    # profile, sampling every 5 minutes, every 30 as the battery runs low, and hourly when
    # it is nearly flat, and it saves to JSON the same as a shipped one.
    band = ControlPolicy(ControlKind.SETPOINT, setpoint=37.5, hysteresis=7.5, safe_band=15.0)
    schedule = PowerScheduleSpec(300, 1800, 3600)
    profile = Profile("raised-bed-drip", TOPIC, band, schedule)
    probe = SoilProbe(Replay([2900.0, 2300.0]), Calibration.two_point(3200.0, 0.0, 1400.0, 100.0))
    valve = Valve()

    # In Rust a Node runs this loop. Here the profile's controller decides, and the program
    # reads the probe and drives the valve itself.
    controller = profile.controller()
    reactions = []
    for _ in range(2):
        reaction = controller.evaluate(await probe.read())
        if reaction.actuator is not None:
            await valve.apply(reaction.actuator)
        state = {None: "untouched", True: "open", False: "closed"}[reaction.actuator]
        alert = reaction.alert.kind if reaction.alert else "none"
        print(f"under the profile: valve {state}, alert {alert}")
        reactions.append(reaction)
    return reactions


reactions = asyncio.run(under_a_profile())
```
<!-- end -->

## C#

In C#, a part is whatever the program makes it: here the probe wraps a
`Func<Task<float>>` that reads counts and the valve has an `ApplyAsync(bool)`, and the
loop is the program's own. A failed read throws, and a `Replay` that has run out throws
`PamojaException` with `resource is closed`. A profile of the program's own is
`new Profile(name, topic, control, power)`, with the control a `ControlPolicy` record
built with named values, `new ControlPolicy(ControlKind.Setpoint, Setpoint: 37.5f,
Hysteresis: 7.5f, SafeBand: 15f)`, and the power a `PowerSchedule(ActiveSecs, SaverSecs,
CriticalSecs)` whose thresholds are 0.5 and 0.2 unless given. The profile and its
`Controller` hold native handles and belong in a `using`.

The parts:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/DeviceGuide.cs#parts -->
From [`bindings/dotnet/samples/Pamoja.Guides/DeviceGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/DeviceGuide.cs):

```csharp
/// <summary>
/// A capacitive soil probe on an analog-to-digital converter. Whatever reads the chip
/// is <paramref name="readCounts"/>: a replay here, the converter's own driver on the
/// node. The probe turns counts into percent, and nothing downstream needs to know
/// there was a chip at all.
/// </summary>
private sealed class SoilProbe(Func<Task<float>> readCounts, Calibration calibration)
{
    public async Task<float> ReadAsync() => calibration.Apply(await readCounts());
}

/// <summary>
/// A solenoid valve on a relay. It keeps what it was last told and counts the
/// changes, which is what a test needs and what a real one does before it drives
/// the pin.
/// </summary>
private sealed class Valve
{
    public bool Open { get; private set; }

    public int Switched { get; private set; }

    public Task ApplyAsync(bool open)
    {
        if (open != Open)
        {
            Open = open;
            Switched++;
        }

        return Task.CompletedTask;
    }
}
```
<!-- end -->

The loop:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/DeviceGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/DeviceGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/DeviceGuide.cs):

```csharp
// Bone dry read 3200 counts on this probe and a soaked pot read 1400, measured
// once and kept. The replay hands back the counts a bed reads as it dries and is
// watered.
using var counts = new Replay([2900f, 2700f, 2300f, 2450f, 2750f, 2500f]);
var probe = new SoilProbe(counts.ReadAsync, Calibration.TwoPoint(3200f, 0f, 1400f, 100f));
var valve = new Valve();

// Water below 30% and stop above 45%. A valve that adds water is what `Heating`
// names, so the band sits at 37.5 with 7.5 either side.
using var rule = Thermostat.Heating(37.5f, 7.5f);

// The link, with its first two sends failing the way a radio does at dusk. The
// ladder keeps what it could not send and replays it in order once a send goes
// through.
using var broker = new LoopbackBroker();
using LoopbackTransport gateway = broker.Link();
await gateway.ConnectAsync();
await gateway.SubscribeAsync(Topic);
using var ladder = new Ladder(Store.Memory());
ladder.Rung(Transport.Faulty(broker.Rung(), 2));
await ladder.ConnectAsync();

// The loop: read, decide, act, publish. Nothing in it knows the probe is homemade.
for (int sample = 0; sample < 6; sample++)
{
    float moisture = await probe.ReadAsync();
    await valve.ApplyAsync(rule.Update(moisture));
    string report = moisture.ToString("F1", CultureInfo.InvariantCulture);
    Delivery delivery = await ladder.SendAsync(Topic, report);
    string state = valve.Open ? "open" : "closed";
    Console.WriteLine($"bed at {report}%, valve {state}, {delivery}");
    if (await ladder.BufferedAsync() > 0)
    {
        int caughtUp = await ladder.FlushAsync();
        if (caughtUp > 0)
        {
            Console.WriteLine($"link back, {caughtUp} readings caught up");
        }
    }
}

Console.WriteLine($"the valve switched {valve.Switched} times");

// On the gateway, in the order they were read, outage included.
List<string> got = [];
for (int n = 0; n < 6; n++)
{
    TransportMessage message = (await gateway.ReceiveAsync())!;
    got.Add(message.Text);
}

Console.WriteLine($"gateway got {string.Join(", ", got)}");
```
<!-- end -->

The same parts under a profile of the maker's own:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/DeviceGuide.cs#profile -->
From [`bindings/dotnet/samples/Pamoja.Guides/DeviceGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/DeviceGuide.cs):

```csharp
// The same band as a profile rather than a line of code, with an alert once the
// bed is more than 15 points from target. No preset is involved: this is the
// maker's own profile, sampling every 5 minutes, every 30 as the battery runs low,
// and hourly when it is nearly flat, and it saves to JSON the same as a shipped one.
var band = new ControlPolicy(ControlKind.Setpoint, Setpoint: 37.5f, Hysteresis: 7.5f, SafeBand: 15f);
var schedule = new PowerSchedule(300, 1800, 3600);
using var profile = new Profile("raised-bed-drip", Topic, band, schedule);
using var counts = new Replay([2900f, 2300f]);
var probe = new SoilProbe(counts.ReadAsync, Calibration.TwoPoint(3200f, 0f, 1400f, 100f));
var valve = new Valve();

// In Rust a Node runs this loop. Here the profile's controller decides, and the
// program reads the probe and drives the valve itself.
using Controller controller = profile.Controller();
List<Reaction> reactions = [];
for (int tick = 0; tick < 2; tick++)
{
    Reaction reaction = controller.Evaluate(await probe.ReadAsync());
    if (reaction.Actuator is bool open)
    {
        await valve.ApplyAsync(open);
    }

    string state = reaction.Actuator switch { true => "open", false => "closed", null => "untouched" };
    string alert = reaction.Alert?.Kind.ToString() ?? "none";
    Console.WriteLine($"under the profile: valve {state}, alert {alert}");
    reactions.Add(reaction);
}
```
<!-- end -->

## Values at a glance

**What makes a part.** Each language's contract, and what a part that cannot do its job
should do:

| Language | A sensor | An actuator | When it cannot |
| --- | --- | --- | --- |
| Rust | `impl Sensor` with a `Reading` type and `async fn read(&mut self) -> Result<Reading>` | `impl Actuator` with a `Command` type and `async fn apply(&mut self, command) -> Result<()>` | return `Error::Io`, or `Error::Closed` once the part has gone away |
| TypeScript | any object with `read(): Promise<number>` | any object with `apply(command): Promise<void>` | throw |
| Python | any object with `async def read(self)` | any object with `async def apply(self, command)` | raise |
| C# | any method that returns a `Task<float>` | any method that takes the command | throw |

**The pieces the loop uses,** each covered in a guide of its own:

| Piece | What it does | Its guide |
| --- | --- | --- |
| replay | hands back recorded readings one per read, then reports closed, or loops when repeating | [Simulators](sim.md) |
| calibration | maps a raw count onto real units from two known points | [Helpers](kit.md) |
| thermostat | on/off control with a band either side of a setpoint | [Helpers](kit.md) |
| loopback broker and faulty link | a link with nothing plugged in, and one that refuses its first sends | [Loopback](loopback.md) |
| ladder | buffers what no link could carry and replays it in order | [Transport ladder](ladder.md) |
| profile and controller | a rule as data, and the decisions it makes about each reading | [Device profiles](profile.md) |

**A profile built from its parts.** A name, a topic, a control policy, and a power
schedule; the description and dashboard presentation are added afterward, as the
profiles guide shows. In the bindings, `cooling` and `rising` are false unless given:

| Part | Means | In a manifest |
| --- | --- | --- |
| name | a stable, human-readable name, such as `raised-bed-drip` | `name` |
| topic | where each reading is published | `topic` |
| setpoint control | `setpoint`, `hysteresis` either side of it, `cooling` for an output that cools rather than heats, and `safe band`, how far a reading may stray before an alert | `kind: setpoint` and its four fields |
| level control | `empty`, the level treated as empty, and `warn within`, how many samples ahead to warn | `kind: level` |
| surge control | `limit`, the largest safe change per sample, and `rising` to watch a rise rather than a fall | `kind: surge` |
| monitor control | reports readings and decides nothing | `kind: monitor` |
| custom control | a kind of the program's own, named, with its parameters as numbers, flags, and text | the kind and every parameter beside it |
| power schedule | seconds between samples at a healthy charge, while conserving, and when critically low; the saver cadence starts below 50% charge and the critical one below 20% unless given | `power` |

**The calls in each language:**

### Rust

| To | Write |
| --- | --- |
| build a profile | `Profile::new(name, topic, control, PowerSchedule::new(active, saver, critical))` |
| a setpoint control | `ControlSpec::Setpoint { setpoint, hysteresis, cooling, safe_band }` |
| a custom control | `ControlSpec::custom(kind, Params::new().with(name, value))` |
| other thresholds | `PowerSchedule::new(..).with_thresholds(saver_below, critical_below)` |
| another margin | `PowerSchedule::new(..).with_hysteresis(margin)` |
| run the parts under it | `Node::new(profile, sensor, actuator, link, codec)`, then `tick()` gives a `Reaction` |
| decide by hand | `profile.controller()`, then `evaluate(reading)` gives a `Reaction` |

### TypeScript

| To | Write |
| --- | --- |
| build a profile | `new Profile(name, topic, control, { activeSecs, saverSecs, criticalSecs })` |
| a setpoint control | `{ kind: ControlKind.Setpoint, setpoint, hysteresis, cooling?, safeBand }` |
| a custom control | `{ kind: ControlKind.Custom, customKind, params: { name: value } }` |
| other thresholds | `{ ..., saverBelow, criticalBelow }` in the power schedule |
| another margin | `{ ..., hysteresis }` in the power schedule |
| decide | `profile.controller()`, then `evaluate(reading)` gives `{ actuator, alert }` |

### Python

| To | Write |
| --- | --- |
| build a profile | `Profile(name, topic, control, PowerScheduleSpec(active, saver, critical))` |
| a setpoint control | `ControlPolicy(ControlKind.SETPOINT, setpoint=..., hysteresis=..., cooling=False, safe_band=...)` |
| a custom control | `ControlPolicy(ControlKind.CUSTOM, custom_kind=..., params={...})` |
| other thresholds | `PowerScheduleSpec(active, saver, critical, saver_below=..., critical_below=...)` |
| another margin | `PowerScheduleSpec(active, saver, critical, hysteresis=...)` |
| decide | `profile.controller()`, then `evaluate(reading)` gives a `Reaction` with `actuator` and `alert` |

### C#

| To | Write |
| --- | --- |
| build a profile | `new Profile(name, topic, control, new PowerSchedule(active, saver, critical))` |
| a setpoint control | `new ControlPolicy(ControlKind.Setpoint, Setpoint: .., Hysteresis: .., SafeBand: ..)` |
| a custom control | `new ControlPolicy(ControlKind.Custom, CustomKind: .., Params: new Dictionary<string, object> { .. })` |
| other thresholds | `new PowerSchedule(active, saver, critical, SaverBelow: .., CriticalBelow: ..)` |
| another margin | `new PowerSchedule(active, saver, critical, Hysteresis: ..)` |
| decide | `profile.Controller()`, then `Evaluate(reading)` gives a `Reaction` with `Actuator` and `Alert` |

<!-- languages end -->

**What a reaction says.** The controller's answer to one reading, the same in every
language:

| Field | Means |
| --- | --- |
| actuator | the setting the output should take, or none when the profile observes rather than controls |
| alert | none, or the threshold the reading crossed: out of range with the reading, running out with the samples left, changing fast with the rate, or a custom code with its value |

## When it goes wrong

What is refused, building a profile from its parts or reading past a replay's end:

| What happened | The message | Where |
| --- | --- | --- |
| a control without a field its kind needs | `a Setpoint control needs hysteresis` | every binding: an `ArgumentException` in C#, which names `Hysteresis`, and a `ValueError` in Python |
| a custom control named like a built-in one | `codec error: level is a built-in control kind, so it takes its own fields rather than parameters` | every language, from Rust's `ControlSpec::custom` down |
| a custom parameter named `kind` | `codec error: a custom control cannot have a parameter named kind, which its manifest keeps for the kind itself` | every language |
| a custom parameter that is not a number, a flag, or text | `parameter zones must be a number, True or False, or text, not list` | Python; C# says the same with `true or false` |
| seconds with a fraction | `activeSecs must be a whole number of seconds, not 2.5` | TypeScript; Python raises `TypeError`, and Rust and C# take whole numbers |
| a kind that is not one of the five | `kind must be "Setpoint", "Level", "Surge", "Monitor", or "Custom", not "Sideways"` | Python; TypeScript's types refuse it before the call |
| a replay read past its end | `resource is closed` | every language: `Error::Closed` in Rust |

The mistakes that cost an afternoon:

- **The valve never opens, or never closes.** A thermostat's `cooling` and `heating` name
  a direction, not a use. A valve that adds water acts on a falling reading the way a
  heater does, so it is `heating`; `cooling` opens it on a rising one.
- **The moisture reads backward.** A capacitive probe reads higher the drier the soil, so
  the dry count is the top of the calibration and the wet count the bottom. Measure both
  once, in dry air and in water, and keep the numbers.
- **One bad read stops the node.** A read that fails throws, raises, or returns an error,
  and a loop that does not catch it ends there. Skip that reading and carry on, or let the
  profile's controller hold what it decided last.
- **The backlog is gone after a restart.** A memory store holds what the link refused for
  as long as the process runs. Give the ladder a file store to keep it across a restart or
  a power cut.
- **A profile decides strangely.** A controller keeps state between readings, because a
  level estimate and a rate of change both need the previous one. Evaluate readings
  through one controller, in the order they were taken.
- **A Rust node will not spawn.** Every core trait's future is `Send`, so a part that holds
  something that is not, such as a bus without a `Send` bound, cannot move into a spawned
  task. Bound it, as `SoilProbe` bounds its converter.

## Where next

<!-- table: next device -->
- [Device profiles](profile.md): Named, ready-to-run device profiles from plain data or a JSON manifest.
- [Simulators](sim.md): Noisy and replay sensors, a recording actuator, a simulated robot that dead-reckons its pose, and a link that loses sends on a pattern.
- [Sensor drivers](sensors.md): Datasheet-anchored drivers for eleven parts, from every language.
- Also in Sensing and actuation: [Actuator drivers](actuators.md).
<!-- end -->

## Reference

<!-- table: reference device -->
- Rust: `Sensor` and `Actuator` in [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-device)
- TypeScript: [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-device)
- Python: [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-device)
- C#: [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-device)
<!-- end -->
