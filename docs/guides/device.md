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

In TypeScript, Python, and C# there is no trait to implement. A class with a `read` or
an `apply` is the whole contract, and the loop that calls them is the program's own,
with the helpers, the controller, and the ladder taking plain numbers, booleans, and
bytes. In Rust the same two structs also drop into a profile `Node`, which runs the loop
for them.

## What the example does

It defines the probe and the valve, waters a raised bed from six readings of a bed that
dries out and is watered, and reports every reading to a gateway over a link that is
down for the first two.

The probe is calibrated from two measurements kept from the day it was built: 3200
counts bone dry and 1400 in a soaked pot. The rule is a hysteresis band, water below
30% and stop above 45%, which the kit's `Thermostat` provides once told that a valve
adds moisture the way a heater adds heat. The ladder buffers the two readings the link
refuses and replays them in order once a send goes through, so the gateway sees all six
in the order they were read.

It proves:

- A struct or class of the maker's own is a sensor the moment it can be read and an
  actuator the moment it can be told, with no registration and no base class.
- The probe reports percent because it does its own calibration; the converter
  underneath is any source of counts, a replay here and a driver on the node.
- The valve opens on the first reading, closes at 50%, and opens again at 25%: three
  changes, with the readings inside the band leaving it as it was.
- The first two readings are buffered rather than lost, both go out when the link
  returns, and the gateway receives all six in order.
- In Rust, the same two parts run under a hand-written profile through `Node`, which
  reads, decides, drives the valve, and publishes on every tick, and raises an alert
  once the bed is far from target.

## Rust

The parts:

<!-- snippet: examples/tests/guides/device.rs#parts -->
From [`examples/tests/guides/device.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/device.rs):

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

impl<A: Sensor<Reading = f32>> Sensor for SoilProbe<A> {
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

<!-- snippet: examples/tests/guides/device.rs#example -->
From [`examples/tests/guides/device.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/device.rs):

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
gateway.connect().await.expect("the gateway connects");
gateway.subscribe(topic).await.expect("the gateway listens");
let flaky = Faulty::new(LoopbackTransport::new(broker), 2);
let mut ladder = TransportLadder::new(MemoryStore::new()).rung(flaky);
ladder.connect().await.expect("the ladder connects");

// The loop: read, decide, act, publish. Nothing in it knows the probe is homemade.
for _ in 0..6 {
    let moisture = probe.read().await.expect("a reading");
    let wanted = rule.update(moisture);
    valve.apply(wanted).await.expect("the valve answers");
    let report = format!("{moisture:.1}");
    let payload = report.as_bytes();
    let delivery = ladder.send(topic, payload).await.expect("a delivery");
    let state = if valve.open { "open" } else { "closed" };
    println!("bed at {report}%, valve {state}, {delivery:?}");
    if ladder.buffered().await.expect("a count") > 0 {
        let caught_up = ladder.flush().await.expect("a flush");
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
    let message = gateway.recv().await.expect("recv").expect("a message");
    got.push(String::from_utf8_lossy(&message.payload).into_owned());
}
let readings = got.join(", ");
println!("gateway got {readings}");
```
<!-- end -->

The same parts under a profile of the maker's own:

<!-- snippet: examples/tests/guides/device.rs#node -->
From [`examples/tests/guides/device.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/device.rs):

```rust
use pamoja_codec::CborCodec;
use pamoja_core::Transport;
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_profile::{ControlSpec, Node, PowerSchedule, Profile};
use pamoja_sim::Replay;

// The same band as a manifest rather than a line of code, with an alert once the bed
// is more than 15 points from target. No preset is involved: this is the maker's own
// profile, and it saves to JSON the same as a shipped one.
let profile = Profile {
    name: "raised-bed-drip".to_owned(),
    description: None,
    topic: "garden/bed-1/moisture".to_owned(),
    control: ControlSpec::Setpoint {
        setpoint: 37.5,
        hysteresis: 7.5,
        cooling: false,
        safe_band: 15.0,
    },
    power: PowerSchedule::new(300, 1800, 3600),
    presentation: None,
};
let probe = SoilProbe {
    adc: Replay::new(vec![2900.0, 2300.0]),
    calibration: Calibration::two_point(3200.0, 0.0, 1400.0, 100.0),
};
let mut link = LoopbackTransport::new(LoopbackBroker::new());
link.connect().await.expect("the link connects");

// A node reads, decides, drives the valve, and publishes on every tick. The parts are
// the ones above; only the loop moved into the library.
let mut node = Node::new(profile, probe, Valve::default(), link, CborCodec);
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
    const delivery = await ladder.send(TOPIC, Buffer.from(report))
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
    got.push((await gateway.recv())!.payload.toString())
  }
  console.log(`gateway got ${got.join(', ')}`)

  return { got, switched: valve.switched, open: valve.open, left: await ladder.buffered() }
}

main()
```
<!-- end -->

## Python

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
    got = [(await gateway.recv()).payload.decode() for _ in range(6)]
    print(f"gateway got {', '.join(got)}")
    return got, valve, await ladder.buffered()


got, valve, left = asyncio.run(main())
```
<!-- end -->

## C#

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
    Delivery delivery = await ladder.SendAsync(Topic, Encoding.UTF8.GetBytes(report));
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
    got.Add(Encoding.UTF8.GetString(message.Payload));
}

Console.WriteLine($"gateway got {string.Join(", ", got)}");
```
<!-- end -->

## Reference

<!-- table: reference device -->
- Rust: the `Transport` and `Receive` traits in [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-device)
- TypeScript: [`@pamoja/core`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_core.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-device)
- Python: [`pamoja.core`](https://pamoja.molex.cloud/docs/reference/python/pamoja/core.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-device)
- C#: [`Pamoja.Core`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Core.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-device)
<!-- end -->
