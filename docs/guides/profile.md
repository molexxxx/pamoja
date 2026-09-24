# Device profiles

Most deployed nodes are one of a handful of shapes. Something is held near a
setpoint. Something is watched as it falls toward empty. Something is watched for
a change too fast to be real. A profile is that shape written down: what the node
publishes on, which policy it applies to each reading, and how often it samples as
its battery drains.

Writing it down as data rather than as code is what makes a fleet manageable. A
profile loads from and saves to JSON, so retuning a deadband across a hundred
nodes is a file that ships, not a firmware build. The same manifest also carries
what a dashboard needs to draw the node, so the device and the screen agree
without a second source.

A node runs the profile: it takes a reading, lets the profile's policy decide what
the output should do and whether the reading crossed a line worth raising, switches
the output, and publishes the reading, then waits as long as the battery allows. The
decisions are compiled into the library; the file only tunes them. A control kind
the library never shipped is decided by code the program registers under the kind's
name, so a manifest for it reads exactly like one for a setpoint. A manifest is
checked as it loads, so a value no node could run, or a misspelled field, is refused
with the reason rather than running quietly wrong.

## What the example does

It loads `profiles/brooder-heater.json`, the brooder profile the catalog ships, and
runs it as a node over a morning of five brooder temperatures: a list stands in for
the probe, a variable for the heat lamp, and an in-process broker for the link, with
a dashboard listening on it. It prints what the lamp does at each reading and what
the dashboard heard, asks the node how long it would wait at three charges, and
prints how the file says a dashboard draws the node.

The second part runs the other two built-in policies on the profiles the library
ships: a well whose level falls toward dry, and a river that rises too fast. The
third names a control kind of the program's own in the same file, and registers the
code that decides it. The fourth is what goes wrong: a probe that fails, a
controller built again for each reading, two manifests no node could run and one
with a misspelled field, and a custom kind with no code registered for it.

The five readings sit around the 32 C target: the lamp switches on at 31.5 C or
below and off at 32.5 C or above, and between the two it stays as it was.

It proves:

- The file parses into the profile a node runs: what it reads, the topic it
  reports on, the setpoint policy, the sampling schedule, and how a dashboard
  draws it.
- The node holds the lamp through the deadband in both directions, raises
  `OutOfRange` for a reading more than 4 C from the target, and publishes every
  reading, in order, to the topic the file names.
- At a charge of 80% the node samples every 2 minutes, at 30% every 10, and at 10%
  every 30, the cadences the file sets.
- A level warns once a fall puts dry within six samples, and a surge warns when
  one sample rises more than its limit.
- A kind the library never shipped runs the program's own code once it is
  registered under the kind's name, reading its parameters from the file and
  raising a condition of its own.
- A reading that is not a number raises `InvalidReading` and the lamp holds, and a
  controller built again for each reading forgets the lamp was on.
- A manifest with a hysteresis of zero, or intervals that shorten as the battery
  drains, is refused with the reason, and one with a misspelled field is refused
  with the field it was probably meant to be and where it sits. A custom kind with
  no code registered for it is refused rather than run as a node that never
  switches the lamp.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example profile" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example profile</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- profile" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- profile</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/profile.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/profile.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- profile" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- profile</code></div>
</div>
<!-- end -->

## Rust

In Rust, `Profile::from_json` loads a manifest and checks it, returning an `Err`
holding the reason for one no node could run. `Node::new` assembles the profile with
a `Sensor`, an `Actuator`, a `Transport`, and a `Codec`; each `tick` returns a `Tick`
with the reading and the `Reaction`, the output setting and an `Alert`, and `run`
repeats it at the cadence `schedule` gives, sleeping with the runtime's own timer. A
policy of your own implements `Policy`, and `Node::resolve` runs the one a
`PolicyRegistry` finds for a manifest's custom kind.

<!-- snippet: examples/guides/profile.rs#example -->
From [`examples/guides/profile.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/profile.rs):

```rust
use pamoja_codec::{Codec, JsonCodec};
use pamoja_core::{Receive, Transport};
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_profile::{Node, Profile};

// A profile is a file. This one ships in the catalog under profiles/: it holds a
// brooder at 32 C by switching a heat lamp, says what it reads, and says how a
// dashboard draws it.
let text = std::fs::read_to_string("profiles/brooder-heater.json")?;
let profile = Profile::from_json(&text)?;
let reads = profile
    .reads
    .clone()
    .expect("a catalog profile says what it reads");
println!(
    "profile   {} reads {} in {} and reports on {}",
    profile.name, reads.quantity, reads.unit, profile.topic
);
let topic = profile.topic.clone();
let element = profile
    .presentation
    .clone()
    .expect("a presentation")
    .elements[0]
    .clone();

// A node is the profile and the parts that make it run: a sensor, an output, and a
// link. A morning of readings stands in for the probe, and a dashboard listens on the
// same broker.
let broker = LoopbackBroker::new();
let mut link = LoopbackTransport::new(broker.clone());
let mut dashboard = LoopbackTransport::new(broker);
link.connect().await?;
dashboard.connect().await?;
dashboard.subscribe(&topic).await?;
let morning = Morning(vec![27.5, 31.8, 32.6, 32.1, 31.4]);
let mut node = Node::new(profile, morning, Lamp::default(), link, JsonCodec)?;

// Each tick reads, decides, switches the lamp, and publishes the reading. The lamp
// comes on at 31.5 C or below and goes off at 32.5 C or above, and in between it stays
// as it was; a reading more than 4 C from 32 raises an alert as well.
let mut was = false;
for _ in 0..5 {
    let tick = node.tick().await?;
    let on = tick.reaction.actuator == Some(true);
    let change = match (was, on) {
        (false, true) => "lamp on",
        (true, false) => "lamp off",
        (true, true) => "lamp stays on",
        (false, false) => "lamp stays off",
    };
    let alert = tick
        .reaction
        .alert
        .map(|alert| format!(", alert {}", alert.kind()))
        .unwrap_or_default();
    println!("{:<10}{change}{alert}", format!("{} C", tick.reading));
    was = on;
}

// The dashboard heard every reading the node published.
let mut heard = Vec::new();
for _ in 0..5 {
    let message = dashboard.recv().await?.expect("a reading");
    let reading: f32 = JsonCodec.decode(&message.payload)?;
    heard.push(reading.to_string());
}
println!("heard     {} on {topic}", heard.join(", "));

// Between ticks the node waits as long as its battery allows: often on a healthy
// charge, sparingly on a low one. run() does this until stopped, waiting each interval.
for charge in [0.8, 0.3, 0.1] {
    let (mode, wait) = node.schedule(charge, false);
    println!(
        "battery   at {:.0}% it runs {mode:?} and waits {} s",
        charge * 100.0,
        wait.as_secs()
    );
}

// The same file says how a dashboard draws the node.
let [low, high] = element.band.expect("a band");
println!(
    "draws     {} in {} on a {}, safe from {low} to {high}",
    element.key,
    element.unit,
    element.viz.name()
);
```
<!-- end -->

The other policies, continuing from above:

<!-- snippet: examples/guides/profile.rs#kinds -->
From [`examples/guides/profile.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/profile.rs):

```rust
use pamoja_profile::Alert;

// A level warns before a tank or a well runs dry. The shipped well profile counts
// 0.5 m as dry and warns once the last fall puts dry six samples away or nearer.
let mut well = Profile::well_level().controller()?;
for depth in [5.0, 4.4, 3.8] {
    match well.evaluate(depth).alert {
        Some(Alert::RunningOut { samples }) => {
            println!("well      {depth} m: dry in {samples} samples at this rate, RunningOut")
        }
        _ => println!("well      {depth} m: no warning yet"),
    }
}

// A surge warns when a reading moves too far in one sample. The shipped flood sensor
// warns when a river rises more than 0.3 m between two readings.
let mut river = Profile::flood_sensor().controller()?;
for gauge in [1.2, 1.35, 1.9] {
    match river.evaluate(gauge).alert {
        Some(Alert::ChangingFast { rate }) => {
            println!("river     {gauge} m: up {rate:.2} m in one sample, ChangingFast")
        }
        _ => println!("river     {gauge} m: no warning"),
    }
}
```
<!-- end -->

A control kind of the program's own, continuing from above:

<!-- snippet: examples/guides/profile.rs#custom -->
From [`examples/guides/profile.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/profile.rs):

```rust
use pamoja_profile::{BoxedPolicy, Params, Policy, PolicyRegistry, Reaction};

/// A policy of the program's own: the lamp on below the setpoint, and a condition of
/// its own when the chicks are chilled.
struct BrooderGuard {
    setpoint: f32,
    chilled_below: f32,
}

impl Policy for BrooderGuard {
    type Reading = f32;
    type Command = bool;

    fn evaluate(&mut self, reading: &f32) -> Reaction {
        let chilled = *reading < self.chilled_below;
        Reaction {
            actuator: Some(*reading < self.setpoint),
            alert: chilled.then_some(Alert::Custom {
                code: "Chilled",
                value: *reading,
            }),
        }
    }
}

// A manifest may name a control kind the library never shipped, with its parameters
// beside it. The program registers the code that decides it under that name, and the
// node runs whichever kind the file names.
let guarded =
    Profile::from_json(&text.replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""))?;
let registry = PolicyRegistry::new().register("brooder_guard", |params: &Params| {
    let setpoint = params.number("setpoint").unwrap_or(32.0) as f32;
    let band = params.number("safe_band").unwrap_or(4.0) as f32;
    Ok(Box::new(BrooderGuard {
        setpoint,
        chilled_below: setpoint - band,
    }) as BoxedPolicy)
});
println!(
    "custom    {} is decided by the program's own code, registered under its name",
    guarded.control.kind()
);
let mut link = LoopbackTransport::new(LoopbackBroker::new());
link.connect().await?;
let mut node = Node::resolve(
    guarded,
    &registry,
    Morning(vec![27.5]),
    Lamp::default(),
    link,
    JsonCodec,
)?;
let tick = node.tick().await?;
let lamp = if tick.reaction.actuator == Some(true) {
    "on"
} else {
    "off"
};
let alert = tick.reaction.alert.map(Alert::kind).unwrap_or("none");
println!(
    "{:<10}lamp {lamp}, alert {alert}",
    format!("{} C", tick.reading)
);
```
<!-- end -->

What goes wrong, continuing from above:

<!-- snippet: examples/guides/profile.rs#wrong -->
From [`examples/guides/profile.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/profile.rs):

```rust
// A probe that fails reports a reading that is not a number. The controller raises
// it rather than going quiet, and the lamp holds its state; what off means for the
// chicks is the node's call.
let profile = Profile::from_json(&text)?;
let mut controller = profile.controller()?;
controller.evaluate(27.5);
let failed = controller.evaluate(f32::NAN);
if let Some(alert) = failed.alert {
    let holds = if failed.actuator == Some(true) {
        "on"
    } else {
        "off"
    };
    println!(
        "probe     a reading of NaN raises {}, and the lamp holds {holds}",
        alert.kind()
    );
}

// A controller built again for each reading forgets the lamp was on, so inside the
// deadband it switches the lamp off.
let first = profile.controller()?.evaluate(27.5).actuator;
let then = profile.controller()?.evaluate(31.8).actuator;
if first == Some(true) && then == Some(false) {
    println!(
        "fresh     built again for each reading, the controller turns the lamp off at 31.8 C"
    );
}

// A manifest no node could run is refused as it loads, with the reason. So is a
// misspelled field, with the one it was probably meant to be and where it sits,
// rather than leaving the default in its place without a word.
for edited in [
    text.replace("\"hysteresis\": 0.5", "\"hysteresis\": 0.0"),
    text.replace("\"saver_secs\": 600", "\"saver_secs\": 60"),
    text.replace("\"saver_below\"", "\"saver_bellow\""),
] {
    match Profile::from_json(&edited) {
        Ok(_) => {
            println!("a manifest no node could run was accepted, which should never happen")
        }
        Err(error) => println!("refused   {error}"),
    }
}

// A kind the library does not ship loads with its parameters, but no built-in
// controller decides it, so a node without a registry that knows it is refused
// rather than running one that never switches the lamp.
let unknown =
    Profile::from_json(&text.replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""))?;
match unknown.controller() {
    Ok(_) => println!("a custom kind ran without its policy, which should never happen"),
    Err(error) => println!("refused   {error}"),
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/profile` loads a manifest with `Profile.fromJson`, which
throws an `Error` saying why for one no node could run. A `Node` takes the profile,
a `read` function, a `drive` function, and any link with `send`; `tick` resolves to
the reading and a reaction whose `actuator` is a boolean or undefined and whose
`alert` carries a `kind` from `AlertKind`, and `run` repeats it at the battery's
cadence until its `AbortSignal` fires. A `PolicyRegistry` maps a custom kind to a
function that builds the program's own policy.

<!-- snippet: bindings/node/guides/profile.ts#example -->
From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import { readFileSync } from 'node:fs'
import { LoopbackBroker } from '@pamoja/loopback'
import { Node, Profile } from '@pamoja/profile'

// A profile is a file. This one ships in the catalog under profiles/: it holds a brooder at
// 32 C by switching a heat lamp, says what it reads, and says how a dashboard draws it.
const text = readFileSync('profiles/brooder-heater.json', 'utf8')
const profile = Profile.fromJson(text)

async function example(): Promise<boolean> {
  const reads = profile.reads!
  console.log(`profile   ${profile.name} reads ${reads.quantity} in ${reads.unit} and reports on ${profile.topic}`)

  // A node is the profile and the parts that make it run: a sensor, an output, and a link.
  // A morning of readings stands in for the probe, and a dashboard listens on the same
  // broker.
  const broker = new LoopbackBroker()
  const link = broker.link()
  const dashboard = broker.link()
  await link.connect()
  await dashboard.connect()
  await dashboard.subscribe(profile.topic)
  const morning = [27.5, 31.8, 32.6, 32.1, 31.4]
  let lamp = false
  const node = new Node({
    profile,
    read: () => morning.shift()!,
    drive: (on) => {
      lamp = on
    },
    link,
  })

  // Each tick reads, decides, switches the lamp, and publishes the reading. The lamp comes on
  // at 31.5 C or below and goes off at 32.5 C or above, and in between it stays as it was; a
  // reading more than 4 C from 32 raises an alert as well.
  let was = false
  for (let at = 0; at < 5; at += 1) {
    const { reading, reaction } = await node.tick()
    const on = reaction.actuator === true
    const change = on ? (was ? 'lamp stays on' : 'lamp on') : was ? 'lamp off' : 'lamp stays off'
    const alert = reaction.alert ? `, alert ${reaction.alert.kind}` : ''
    console.log(`${`${reading} C`.padEnd(10)}${change}${alert}`)
    was = on
  }

  // The dashboard heard every reading the node published.
  const heard: number[] = []
  for (let at = 0; at < 5; at += 1) {
    heard.push((await dashboard.recv())!.number!)
  }
  console.log(`heard     ${heard.join(', ')} on ${profile.topic}`)

  // Between ticks the node waits as long as its battery allows: often on a healthy charge,
  // sparingly on a low one. run() does this until stopped, waiting each interval.
  for (const charge of [0.8, 0.3, 0.1]) {
    const { mode, waitMs } = node.schedule(charge)
    console.log(`battery   at ${(charge * 100).toFixed(0)}% it runs ${mode} and waits ${waitMs / 1000} s`)
  }

  // The same file says how a dashboard draws the node.
  const element = profile.presentation!.elements[0]
  const [low, high] = element.band!
  console.log(`draws     ${element.key} in ${element.unit} on a ${element.viz}, safe from ${low} to ${high}`)
  return lamp
}
```
<!-- end -->

The other policies, continuing from above:

<!-- snippet: bindings/node/guides/profile.ts#kinds -->
From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import { AlertKind } from '@pamoja/profile'

function kinds(): void {
  // A level warns before a tank or a well runs dry. The shipped well profile counts 0.5 m
  // as dry and warns once the last fall puts dry six samples away or nearer.
  const well = Profile.wellLevel().controller()
  for (const depth of [5.0, 4.4, 3.8]) {
    const alert = well.evaluate(depth).alert
    if (alert?.kind === AlertKind.RunningOut) {
      console.log(`well      ${depth} m: dry in ${alert.samples} samples at this rate, RunningOut`)
    } else {
      console.log(`well      ${depth} m: no warning yet`)
    }
  }

  // A surge warns when a reading moves too far in one sample. The shipped flood sensor warns
  // when a river rises more than 0.3 m between two readings.
  const river = Profile.floodSensor().controller()
  for (const gauge of [1.2, 1.35, 1.9]) {
    const alert = river.evaluate(gauge).alert
    if (alert?.kind === AlertKind.ChangingFast) {
      console.log(`river     ${gauge} m: up ${alert.rate?.toFixed(2)} m in one sample, ChangingFast`)
    } else {
      console.log(`river     ${gauge} m: no warning`)
    }
  }
}
```
<!-- end -->

A control kind of the program's own, continuing from above:

<!-- snippet: bindings/node/guides/profile.ts#custom -->
From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import { type Policy, PolicyRegistry } from '@pamoja/profile'

// A policy of the program's own: the lamp on below the setpoint, and a condition of its own
// when the chicks are chilled.
function brooderGuard(params: Record<string, number | boolean | string>): Policy {
  const setpoint = Number(params.setpoint ?? 32)
  const chilledBelow = setpoint - Number(params.safe_band ?? 4)
  return {
    evaluate: (reading) => ({
      actuator: reading < setpoint,
      alert: reading < chilledBelow ? { kind: AlertKind.Custom, code: 'Chilled', value: reading } : undefined,
    }),
  }
}

async function custom(): Promise<void> {
  // A manifest may name a control kind the library never shipped, with its parameters
  // beside it. The program registers the code that decides it under that name, and the node
  // runs whichever kind the file names.
  const guarded = Profile.fromJson(text.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
  const registry = new PolicyRegistry().register('brooder_guard', brooderGuard)
  console.log(`custom    ${guarded.control.customKind} is decided by the program's own code, registered under its name`)
  const link = new LoopbackBroker().link()
  await link.connect()
  let lamp = false
  const node = new Node({
    profile: guarded,
    policy: registry,
    read: () => 27.5,
    drive: (on) => {
      lamp = on
    },
    link,
  })
  const { reading, reaction } = await node.tick()
  const alert = reaction.alert?.code ?? reaction.alert?.kind ?? 'none'
  console.log(`${`${reading} C`.padEnd(10)}lamp ${lamp ? 'on' : 'off'}, alert ${alert}`)
}
```
<!-- end -->

What goes wrong, continuing from above:

<!-- snippet: bindings/node/guides/profile.ts#wrong -->
From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
function wrong(): void {
  // A probe that fails reports a reading that is not a number. The controller raises it
  // rather than going quiet, and the lamp holds its state; what off means for the chicks is
  // the node's call.
  const controller = profile.controller()
  controller.evaluate(27.5)
  const failed = controller.evaluate(Number.NaN)
  if (failed.alert) {
    const holds = failed.actuator === true ? 'on' : 'off'
    console.log(`probe     a reading of NaN raises ${failed.alert.kind}, and the lamp holds ${holds}`)
  }

  // A controller built again for each reading forgets the lamp was on, so inside the
  // deadband it switches the lamp off.
  const first = profile.controller().evaluate(27.5).actuator
  const then = profile.controller().evaluate(31.8).actuator
  if (first === true && then === false) {
    console.log('fresh     built again for each reading, the controller turns the lamp off at 31.8 C')
  }

  // A manifest no node could run is refused as it loads, with the reason. So is a misspelled
  // field, with the one it was probably meant to be and where it sits, rather than leaving
  // the default in its place without a word.
  for (const edited of [
    text.replace('"hysteresis": 0.5', '"hysteresis": 0.0'),
    text.replace('"saver_secs": 600', '"saver_secs": 60'),
    text.replace('"saver_below"', '"saver_bellow"'),
  ]) {
    try {
      Profile.fromJson(edited)
      console.log('a manifest no node could run was accepted, which should never happen')
    } catch (error) {
      console.log(`refused   ${(error as Error).message}`)
    }
  }

  // A kind the library does not ship loads with its parameters, but no built-in controller
  // decides it, so a node without a registry that knows it is refused rather than running
  // one that never switches the lamp.
  const unknown = Profile.fromJson(text.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
  try {
    unknown.controller()
    console.log('a custom kind ran without its policy, which should never happen')
  } catch (error) {
    console.log(`refused   ${(error as Error).message}`)
  }
}
```
<!-- end -->

## Python

In Python, `pamoja.profile` loads a manifest with `Profile.from_json`, which raises
`PamojaError` for one no node could run. A `Node` takes the profile, a `read`
callable, a `drive` callable, and any link with `send`, each plain or async; `tick`
returns the reading and a reaction whose `actuator` is `True`, `False`, or `None`,
and `run` repeats it at the battery's cadence until its task is cancelled. A
`PolicyRegistry` maps a custom kind to a class or function that builds the program's
own policy, which returns a `Decision`.

<!-- snippet: bindings/python/guides/profile.py#example -->
From [`bindings/python/guides/profile.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/profile.py):

```python
from pathlib import Path

from pamoja.loopback import LoopbackBroker
from pamoja.profile import Node, Profile

# A profile is a file. This one ships in the catalog under profiles/: it holds a brooder at
# 32 C by switching a heat lamp, says what it reads, and says how a dashboard draws it.
text = Path("profiles/brooder-heater.json").read_text(encoding="utf-8")
profile = Profile.from_json(text)
print(
    f"profile   {profile.name} reads {profile.reads.quantity} in {profile.reads.unit} "
    f"and reports on {profile.topic}"
)


async def example() -> bool:
    # A node is the profile and the parts that make it run: a sensor, an output, and a link.
    # A morning of readings stands in for the probe, and a dashboard listens on the same
    # broker.
    broker = LoopbackBroker()
    link = broker.link()
    dashboard = broker.link()
    await link.connect()
    await dashboard.connect()
    await dashboard.subscribe(profile.topic)
    morning = [27.5, 31.8, 32.6, 32.1, 31.4]
    lamp = []
    node = Node(profile, read=lambda: morning.pop(0), drive=lamp.append, link=link)

    # Each tick reads, decides, switches the lamp, and publishes the reading. The lamp comes
    # on at 31.5 C or below and goes off at 32.5 C or above, and in between it stays as it
    # was; a reading more than 4 C from 32 raises an alert as well.
    was = False
    for _ in range(5):
        tick = await node.tick()
        on = tick.reaction.actuator is True
        if on:
            change = "lamp stays on" if was else "lamp on"
        else:
            change = "lamp off" if was else "lamp stays off"
        alert = f", alert {tick.reaction.alert.kind}" if tick.reaction.alert else ""
        print(f"{f'{tick.reading:g} C':<10}{change}{alert}")
        was = on

    # The dashboard heard every reading the node published.
    heard = [f"{(await dashboard.recv()).number:g}" for _ in range(5)]
    print(f"heard     {', '.join(heard)} on {profile.topic}")

    # Between ticks the node waits as long as its battery allows: often on a healthy charge,
    # sparingly on a low one. run() does this until stopped, waiting each interval.
    for charge in [0.8, 0.3, 0.1]:
        mode, seconds = node.schedule(charge)
        print(f"battery   at {charge * 100:.0f}% it runs {mode} and waits {seconds:g} s")

    # The same file says how a dashboard draws the node.
    element = profile.presentation.elements[0]
    low, high = element.band
    print(
        f"draws     {element.key} in {element.unit} on a {element.viz}, "
        f"safe from {low:g} to {high:g}"
    )
    return lamp[-1]


lamp_on = asyncio.run(example())
```
<!-- end -->

The other policies, continuing from above:

<!-- snippet: bindings/python/guides/profile.py#kinds -->
From [`bindings/python/guides/profile.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/profile.py):

```python
from pamoja.profile import AlertKind

# A level warns before a tank or a well runs dry. The shipped well profile counts 0.5 m as
# dry and warns once the last fall puts dry six samples away or nearer.
well = Profile.well_level().controller()
for depth in [5.0, 4.4, 3.8]:
    alert = well.evaluate(depth).alert
    if alert and alert.kind == AlertKind.RUNNING_OUT:
        print(f"well      {depth:g} m: dry in {alert.samples} samples at this rate, RunningOut")
    else:
        print(f"well      {depth:g} m: no warning yet")

# A surge warns when a reading moves too far in one sample. The shipped flood sensor warns
# when a river rises more than 0.3 m between two readings.
river = Profile.flood_sensor().controller()
for gauge in [1.2, 1.35, 1.9]:
    alert = river.evaluate(gauge).alert
    if alert and alert.kind == AlertKind.CHANGING_FAST:
        print(f"river     {gauge:g} m: up {alert.rate:.2f} m in one sample, ChangingFast")
    else:
        print(f"river     {gauge:g} m: no warning")
```
<!-- end -->

A control kind of the program's own, continuing from above:

<!-- snippet: bindings/python/guides/profile.py#custom -->
From [`bindings/python/guides/profile.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/profile.py):

```python
from pamoja.profile import CustomAlert, Decision, PolicyRegistry


class BrooderGuard:
    """A policy of the program's own: the lamp on below the setpoint, and a condition of its
    own when the chicks are chilled."""

    def __init__(self, params: dict) -> None:
        self.setpoint = params.get("setpoint", 32.0)
        self.chilled_below = self.setpoint - params.get("safe_band", 4.0)

    def evaluate(self, reading: float) -> Decision:
        chilled = reading < self.chilled_below
        alert = CustomAlert("Chilled", reading) if chilled else None
        return Decision(actuator=reading < self.setpoint, alert=alert)


async def custom() -> None:
    # A manifest may name a control kind the library never shipped, with its parameters
    # beside it. The program registers the code that decides it under that name, and the
    # node runs whichever kind the file names.
    guarded = Profile.from_json(text.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
    registry = PolicyRegistry().register("brooder_guard", BrooderGuard)
    print(
        f"custom    {guarded.control.custom_kind} is decided by the program's own code, "
        "registered under its name"
    )
    link = LoopbackBroker().link()
    await link.connect()
    lamp = []
    node = Node(guarded, read=lambda: 27.5, drive=lamp.append, link=link, policy=registry)
    tick = await node.tick()
    alert = tick.reaction.alert.code if tick.reaction.alert else "none"
    print(f"{f'{tick.reading:g} C':<10}lamp {'on' if lamp[-1] else 'off'}, alert {alert}")


asyncio.run(custom())
```
<!-- end -->

What goes wrong, continuing from above:

<!-- snippet: bindings/python/guides/profile.py#wrong -->
From [`bindings/python/guides/profile.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/profile.py):

```python
from pamoja.core import PamojaError

# A probe that fails reports a reading that is not a number. The controller raises it
# rather than going quiet, and the lamp holds its state; what off means for the chicks is
# the node's call.
controller = profile.controller()
controller.evaluate(27.5)
failed = controller.evaluate(float("nan"))
if failed.alert:
    holds = "on" if failed.actuator is True else "off"
    print(f"probe     a reading of NaN raises {failed.alert.kind}, and the lamp holds {holds}")

# A controller built again for each reading forgets the lamp was on, so inside the deadband
# it switches the lamp off.
first = profile.controller().evaluate(27.5).actuator
then = profile.controller().evaluate(31.8).actuator
if first is True and then is False:
    print("fresh     built again for each reading, the controller turns the lamp off at 31.8 C")

# A manifest no node could run is refused as it loads, with the reason. So is a misspelled
# field, with the one it was probably meant to be and where it sits, rather than leaving the
# default in its place without a word.
for edited in [
    text.replace('"hysteresis": 0.5', '"hysteresis": 0.0'),
    text.replace('"saver_secs": 600', '"saver_secs": 60'),
    text.replace('"saver_below"', '"saver_bellow"'),
]:
    try:
        Profile.from_json(edited)
        print("a manifest no node could run was accepted, which should never happen")
    except PamojaError as error:
        print(f"refused   {error}")

# A kind the library does not ship loads with its parameters, but no built-in controller
# decides it, so a node without a registry that knows it is refused rather than running one
# that never switches the lamp.
unknown = Profile.from_json(text.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
try:
    unknown.controller()
    print("a custom kind ran without its policy, which should never happen")
except PamojaError as error:
    print(f"refused   {error}")
```
<!-- end -->

## C#

In C#, a `Profile`, a `Controller`, and a `Node` hold native state and are disposed
with `using`. `Profile.FromJson` throws `PamojaException` for a manifest no node could
run. A `Node` takes the profile, a `read` function, a `drive` function, and any
`ILink`; `TickAsync` returns a `Tick` with the reading and a `Reaction` record whose
`Actuator` is a `bool?` and whose `Alert` carries an `AlertKind`, and `RunAsync`
repeats it at the battery's cadence until its token is cancelled. A
`PolicyRegistry` maps a custom kind to a factory for the program's own `IPolicy`.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs):

```csharp
// A profile is a file. This one ships in the catalog under profiles/: it holds a
// brooder at 32 C by switching a heat lamp, says what it reads, and says how a
// dashboard draws it.
string text = File.ReadAllText("profiles/brooder-heater.json");
using var profile = Profile.FromJson(text);
Reads reads = profile.Reads!.Value;
Console.WriteLine($"profile   {profile.Name} reads {reads.Quantity} in {reads.Unit} and reports on {profile.Topic}");

// A node is the profile and the parts that make it run: a sensor, an output, and a
// link. A morning of readings stands in for the probe, and a dashboard listens on the
// same broker.
using var broker = new LoopbackBroker();
using var link = broker.Link();
using var dashboard = broker.Link();
await link.ConnectAsync();
await dashboard.ConnectAsync();
await dashboard.SubscribeAsync(profile.Topic);
var morning = new Queue<float>([27.5f, 31.8f, 32.6f, 32.1f, 31.4f]);
bool lamp = false;
using var node = new Node(
    profile,
    () => ValueTask.FromResult(morning.Dequeue()),
    link,
    drive: on =>
    {
        lamp = on;
        return ValueTask.CompletedTask;
    });

// Each tick reads, decides, switches the lamp, and publishes the reading. The lamp
// comes on at 31.5 C or below and goes off at 32.5 C or above, and in between it stays
// as it was; a reading more than 4 C from 32 raises an alert as well.
bool was = false;
for (int at = 0; at < 5; at++)
{
    Tick tick = await node.TickAsync();
    bool on = tick.Reaction.Actuator == true;
    string change = on ? (was ? "lamp stays on" : "lamp on") : was ? "lamp off" : "lamp stays off";
    string alert = tick.Reaction.Alert is { } raised ? $", alert {raised.Kind}" : string.Empty;
    Console.WriteLine(Invariant($"{Invariant($"{tick.Reading} C"),-10}{change}{alert}"));
    was = on;
}

// The dashboard heard every reading the node published.
var heard = new List<string>();
for (int at = 0; at < 5; at++)
{
    TransportMessage? message = await dashboard.ReceiveAsync(TimeSpan.FromSeconds(5));
    heard.Add(Invariant($"{message!.Number}"));
}

Console.WriteLine($"heard     {string.Join(", ", heard)} on {profile.Topic}");

// Between ticks the node waits as long as its battery allows: often on a healthy
// charge, sparingly on a low one. RunAsync does this until cancelled, waiting each
// interval.
foreach (float charge in new[] { 0.8f, 0.3f, 0.1f })
{
    (var mode, TimeSpan wait) = node.Schedule(charge);
    Console.WriteLine(Invariant($"battery   at {charge * 100:F0}% it runs {mode} and waits {wait.TotalSeconds} s"));
}

// The same file says how a dashboard draws the node.
ElementSpec element = profile.Presentation!.Elements[0];
string graphic = element.Viz.ToString().ToLowerInvariant();
Console.WriteLine(Invariant(
    $"draws     {element.Key} in {element.Unit} on a {graphic}, safe from {element.Band![0]} to {element.Band[1]}"));
```
<!-- end -->

The other policies, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs#kinds -->
From [`bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs):

```csharp
// A level warns before a tank or a well runs dry. The shipped well profile counts
// 0.5 m as dry and warns once the last fall puts dry six samples away or nearer.
using (var wellLevel = Profile.WellLevel())
using (Controller well = wellLevel.Controller())
{
    foreach (float depth in new[] { 5.0f, 4.4f, 3.8f })
    {
        Console.WriteLine(well.Evaluate(depth).Alert is { Kind: AlertKind.RunningOut } alert
            ? Invariant($"well      {depth} m: dry in {alert.Samples} samples at this rate, RunningOut")
            : Invariant($"well      {depth} m: no warning yet"));
    }
}

// A surge warns when a reading moves too far in one sample. The shipped flood sensor
// warns when a river rises more than 0.3 m between two readings.
using var floodSensor = Profile.FloodSensor();
using (Controller river = floodSensor.Controller())
{
    foreach (float gauge in new[] { 1.2f, 1.35f, 1.9f })
    {
        Console.WriteLine(river.Evaluate(gauge).Alert is { Kind: AlertKind.ChangingFast } alert
            ? Invariant($"river     {gauge} m: up {alert.Rate:F2} m in one sample, ChangingFast")
            : Invariant($"river     {gauge} m: no warning"));
    }
}
```
<!-- end -->

A control kind of the program's own, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs#custom -->
From [`bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs):

```csharp
/// <summary>
/// A policy of the program's own: the lamp on below the setpoint, and a condition of its
/// own when the chicks are chilled.
/// </summary>
private sealed class BrooderGuard(IReadOnlyDictionary<string, object> parameters) : IPolicy
{
    private readonly float _setpoint = Convert.ToSingle(parameters["setpoint"]);
    private readonly float _chilledBelow = Convert.ToSingle(parameters["setpoint"]) - Convert.ToSingle(parameters["safe_band"]);

    public Reaction Evaluate(float reading) => new(
        reading < _setpoint,
        reading < _chilledBelow ? new Alert(AlertKind.Custom, null, null, null, "Chilled", reading) : null);
}

private static async Task CustomAsync(string text)
{
    // A manifest may name a control kind the library never shipped, with its parameters
    // beside it. The program registers the code that decides it under that name, and the
    // node runs whichever kind the file names.
    using var guarded = Profile.FromJson(text.Replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""));
    var registry = new PolicyRegistry().Register("brooder_guard", parameters => new BrooderGuard(parameters));
    Console.WriteLine($"custom    {guarded.Control.CustomKind} is decided by the program's own code, registered under its name");
    using var broker = new LoopbackBroker();
    using var link = broker.Link();
    await link.ConnectAsync();
    bool lamp = false;
    using var node = new Node(
        guarded,
        () => ValueTask.FromResult(27.5f),
        link,
        drive: on =>
        {
            lamp = on;
            return ValueTask.CompletedTask;
        },
        policy: registry.Resolve(guarded));
    Tick tick = await node.TickAsync();
    string alert = tick.Reaction.Alert?.Code ?? tick.Reaction.Alert?.Kind.ToString() ?? "none";
    Console.WriteLine(Invariant($"{Invariant($"{tick.Reading} C"),-10}lamp {(lamp ? "on" : "off")}, alert {alert}"));
}
```
<!-- end -->

What goes wrong, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs#wrong -->
From [`bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs):

```csharp
// A probe that fails reports a reading that is not a number. The controller raises
// it rather than going quiet, and the lamp holds its state; what off means for the
// chicks is the node's call.
using (Controller controller = profile.Controller())
{
    controller.Evaluate(27.5f);
    Reaction failed = controller.Evaluate(float.NaN);
    if (failed.Alert is { } invalid)
    {
        string holds = failed.Actuator == true ? "on" : "off";
        Console.WriteLine($"probe     a reading of NaN raises {invalid.Kind}, and the lamp holds {holds}");
    }
}

// A controller built again for each reading forgets the lamp was on, so inside the
// deadband it switches the lamp off.
bool? first;
bool? then;
using (Controller once = profile.Controller())
{
    first = once.Evaluate(27.5f).Actuator;
}

using (Controller again = profile.Controller())
{
    then = again.Evaluate(31.8f).Actuator;
}

if (first == true && then == false)
{
    Console.WriteLine("fresh     built again for each reading, the controller turns the lamp off at 31.8 C");
}

// A manifest no node could run is refused as it loads, with the reason. So is a
// misspelled field, with the one it was probably meant to be and where it sits,
// rather than leaving the default in its place without a word.
foreach (string edited in new[]
{
    text.Replace("\"hysteresis\": 0.5", "\"hysteresis\": 0.0"),
    text.Replace("\"saver_secs\": 600", "\"saver_secs\": 60"),
    text.Replace("\"saver_below\"", "\"saver_bellow\""),
})
{
    try
    {
        using var accepted = Profile.FromJson(edited);
        Console.WriteLine("a manifest no node could run was accepted, which should never happen");
    }
    catch (PamojaException error)
    {
        Console.WriteLine($"refused   {error.Message}");
    }
}

// A kind the library does not ship loads with its parameters, but no built-in
// controller decides it, so a node without a registry that knows it is refused
// rather than running one that never switches the lamp.
using var unknown = Profile.FromJson(text.Replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""));
try
{
    using Controller inert = unknown.Controller();
    Console.WriteLine("a custom kind ran without its policy, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"refused   {error.Message}");
}
```
<!-- end -->

## Values at a glance

Every field a manifest may carry, with what it holds and its default, is under
[Every field](#every-field) below, generated from the published schema an editor
checks the file with.

**The control kinds:**

| `kind` | Fields | What it does with each reading | Alert |
| --- | --- | --- | --- |
| `setpoint` | `setpoint`, `hysteresis`, `cooling`, `safe_band` | a heater switches on at `setpoint - hysteresis` or below and off at `setpoint + hysteresis` or above, a cooler the other way round, and in between the output stays as it was | `OutOfRange` when the reading is more than `safe_band` from the setpoint |
| `level` | `empty`, `warn_within` | drives nothing; estimates how many more samples the last fall takes to reach `empty`, rounded up | `RunningOut` once that is `warn_within` or fewer |
| `surge` | `rising`, `limit` | drives nothing; compares each reading with the one before | `ChangingFast` when one sample moves more than `limit` the watched way |
| `monitor` | none | drives nothing and raises nothing | none |
| any other | its own, kept as parameters | whatever the code registered for the kind decides: through a `PolicyRegistry` in Rust, from `control.params` elsewhere; asking for a built-in controller is refused | whatever that policy raises |

A level and a surge need a reading before they can compare, so the first one never
warns.

**The alerts:**

| Kind | Raised when | Carries |
| --- | --- | --- |
| `OutOfRange` | a setpoint reading is further from the target than the safe band | the reading |
| `RunningOut` | a falling level is on course to reach empty within the warning | the samples left |
| `ChangingFast` | one sample moved further than the surge limit | the change |
| `InvalidReading` | a reading is not a finite number, under every kind but a monitor | the reading as it arrived |
| `Custom` | a policy of your own raised it | the code it chose, and a value |

**The power schedule.** A charge below `critical_below` is critical, below
`saver_below` is saver, and anything else is active. A charge that is not a number,
from a fuel gauge that failed to answer, is taken as critical. While the panel charges, the plan eases
the node up one step, from critical to saver and from saver to active. A running
node remembers its mode: it drops a cadence as soon as the charge crosses a
threshold, and climbs back only once the charge is `hysteresis` above it, so a
charge hovering at 50% does not switch the cadence on every sample.

**What a manifest is refused for,** as it loads and as each language's
constructor builds one:

| The manifest | The reason says |
| --- | --- |
| a field the format does not have, such as a misspelled `saver_bellow` | `` unknown field `saver_bellow` ``, and the field it was probably meant to be |
| a `$schema` for another format | `written in profile format 2`, or that it `is not a pamoja profile schema` |
| an empty name | `` `name` must not be empty `` |
| a `reads` quantity or unit that is not lowercase words joined by underscores | `must be lowercase words joined by underscores` |
| a custom kind with no name | `a custom control needs a kind to be named by` |
| a topic that is empty, or holds `+` or `#` | `the topic is empty`, or that it `is a filter` |
| a control value that is not a finite number | `must be a finite number` |
| a hysteresis of zero or less | `the output chatters at the setpoint` |
| a safe band narrower than the hysteresis | `an alert would fire inside the deadband` |
| a level that warns within no samples | `the warning never comes` |
| a surge limit of zero or less | `every sample is a surge` |
| an `active_secs` of zero | `` `active_secs` must be at least one second `` |
| intervals that shorten as the battery drains | `the intervals must not shorten as the battery drains` |
| a `saver_below` of 0 or less or above 1, or a `critical_below` not between 0 and it | the threshold and its value |
| a power `hysteresis` below 0, or one that puts `saver_below` plus it above 1 | `` the power `hysteresis` must be 0 or more `` and its value |
| a dashboard element with a band whose low end is not first, a key given twice, or a state that is not a `state.` code | the element and what is wrong with it |

The catalog in [Profiles](../profiles.md) holds its own files to more: a file
named for the profile, a description, a `reads`, and snake_case keys and kinds.

Asking a profile whose kind is custom for a built-in controller is refused too,
naming the kind: `` no policy decides the control kind `brooder_guard` ``, with
the built-in kind it is probably a misspelling of when there is one.

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| load and write | `Profile::from_json(text)`, `to_json()`, `check()` |
| start from a preset | `Profile::vaccine_fridge_monitor()`, `irrigation_node()`, `well_level()`, `flood_sensor()` |
| build one | `Profile::new(name, topic, ControlSpec, PowerSchedule::new(active, saver, critical))`, `with_reads`, `with_thresholds`, `with_description`, `with_presentation`, `with_element` |
| run it | `Node::new(profile, sensor, actuator, link, codec)`, `tick()` for a `Tick` of `reading` and `reaction`, `schedule(charge, charging)`, `run(battery, wait)` |
| decide a reading | `controller()`, then `evaluate(reading)`; `Controller::setpoint`, `level`, `surge`, `monitor` |
| read the reaction | `actuator`, `alert`, `Alert::kind()` |
| schedule sampling | `power.plan()`, then `mode(charge)`, `mode_while_charging(charge, charging)`, `interval(charge)` |
| supply a policy | `impl Policy`, `PolicyRegistry::new().register(kind, factory)`, `resolve(&profile.control)`, `Node::resolve`, `Node::with_policy` |

### TypeScript

| To | Call |
| --- | --- |
| load and write | `Profile.fromJson(text)`, `toJson()` |
| start from a preset | `Profile.vaccineFridgeMonitor()`, `irrigationNode()`, `wellLevel()`, `floodSensor()` |
| build one | `new Profile(name, topic, control, power)`, `withReads`, `withDescription`, `withPresentation` |
| run it | `new Node({ profile, read, drive, link })`, `tick()` for `reading` and `reaction`, `schedule(charge)`, `run({ signal, battery, onTick, onError })` |
| supply a policy | `new PolicyRegistry().register(kind, factory)`, `resolve(profile)`, or the registry as the node's `policy` |
| decide a reading | `controller()`, then `evaluate(reading)`; `Controller.setpoint`, `level`, `surge`, `monitor` |
| read the reaction | `actuator`, `alert.kind`, `AlertKind` |
| schedule sampling | `powerPlan()`, then `mode(charge)`, `modeWhileCharging(charge, charging)`, `intervalUs(charge)` |
| read what it reads | `reads?.quantity`, `reads?.unit` |
| read a custom kind | `control.kind === ControlKind.Custom`, `control.customKind`, `control.params` |

### Python

| To | Call |
| --- | --- |
| load and write | `Profile.from_json(text)`, `to_json()` |
| start from a preset | `Profile.vaccine_fridge_monitor()`, `irrigation_node()`, `well_level()`, `flood_sensor()` |
| build one | `Profile(name, topic, ControlPolicy(...), PowerScheduleSpec(...))`, `with_reads`, `with_description`, `with_presentation` |
| run it | `Node(profile, read=..., drive=..., link=...)`, `tick()` for `reading` and `reaction`, `schedule(charge)`, `run(battery=..., on_tick=..., on_error=...)` |
| supply a policy | `PolicyRegistry().register(kind, factory)`, `resolve(profile)`, or the registry as the node's `policy`; a policy returns a `Decision` |
| decide a reading | `controller()`, then `evaluate(reading)`; `Controller.setpoint`, `level`, `surge`, `monitor` |
| read the reaction | `actuator`, `alert.kind`, `AlertKind` |
| schedule sampling | `power_plan()`, then `mode(charge)`, `mode_while_charging(charge, charging)`, `interval_us(charge)` |
| read what it reads | `reads.quantity`, `reads.unit` |
| read a custom kind | `control.kind == ControlKind.CUSTOM`, `control.custom_kind`, `control.params` |

### C#

| To | Call |
| --- | --- |
| load and write | `Profile.FromJson(text)`, `ToJson()` |
| start from a preset | `Profile.VaccineFridgeMonitor()`, `IrrigationNode()`, `WellLevel()`, `FloodSensor()` |
| build one | `new Profile(name, topic, ControlPolicy, PowerSchedule)`, `WithReads`, `WithDescription`, `WithPresentation` |
| run it | `new Node(profile, read, link, drive)`, `TickAsync()` for a `Tick`, `Schedule(charge)`, `RunAsync(battery, onTick, onError, cancellationToken: token)` |
| supply a policy | `new PolicyRegistry().Register(kind, factory)`, `Resolve(profile)` as the node's `policy`, over an `IPolicy` |
| decide a reading | `Controller()`, then `Evaluate(reading)`; `Controller.Setpoint`, `Level`, `Surge`, `Monitor` |
| read the reaction | `Actuator`, `Alert?.Kind`, `AlertKind` |
| schedule sampling | `PowerPlan`, then `Mode(charge)`, `ModeWhileCharging(charge, charging)`, `IntervalUs(charge)` |
| read what it reads | `Reads?.Quantity`, `Reads?.Unit` |
| read a custom kind | `Control.Kind == ControlKind.Custom`, `Control.CustomKind`, `Control.Params` |

<!-- languages end -->

## Every field

These tables are generated from the published JSON Schema,
[`profile-1.json`](https://pamoja.molex.cloud/schema/profile-1.json), the same file
an editor checks a manifest against. Name it as the manifest's `$schema` and an
editor such as VS Code completes the fields and marks a wrong one as it is typed.
The schema cannot say that a safe band is at least as wide as its hysteresis, that
the sampling intervals lengthen as the battery drains, or that an element's key is
used once, so the library's own check is the last word on those.

<!-- table: schema profile -->
### The profile file {#profile-fields}

A node written down as data: what it reads, where it reports, how it decides each reading, how often it samples as its battery drains, and how a dashboard draws it.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `$schema` | text | no | The format the file is written in: this schema's address, or a copy of it by the same file name. An editor reads it to check the file as it is typed. |
| `name` | text | yes | A stable name, lowercase words joined by hyphens, such as brooder-heater. A shared profile's file carries the same name. |
| `description` | text | no | What the profile is for, in a sentence or two: what it watches or holds, and what it does when a reading crosses a line. |
| `reads` | object, see [reads](#profile-reads) | no | What the profile reads. The setpoint, bands, and limits under control are in this unit. |
| `topic` | text | yes | The topic each reading is published to, such as poultry/brooder/temperature. One topic, so no + or # wildcards. |
| `control` | object, one of the control kinds below | yes | How each reading is decided. The kind names the policy and the other fields tune it. |
| `power` | object, see [power](#profile-power) | yes | How often the node samples as its battery drains. The intervals may not shorten as the charge falls. |
| `presentation` | object, see [presentation](#profile-presentation) | no | How a dashboard draws the node: the readings it adds, a tint, and the words for its own states. |

### reads {#profile-reads}

What the profile reads. The setpoint, bands, and limits under control are in this unit.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `quantity` | text | yes | The quantity its control decides on, in lowercase words joined by underscores, such as temperature, relative_humidity, soil_moisture, water_level, or pressure. |
| `unit` | text | yes | The unit its numbers are in, in lowercase words joined by underscores, such as celsius, percent, meter, or bar. |

### control, kind setpoint {#profile-setpoint}

Holds a reading near a target by switching an output on and off, and alerts when the reading strays too far.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `kind` | `setpoint` | yes | Names this policy. |
| `setpoint` | number | yes | The target reading, such as 32 for a brooder in celsius. |
| `hysteresis` | number, above 0 | yes | Half the deadband around the setpoint. The output switches at the setpoint less and plus this, so it does not chatter. |
| `cooling` | true or false | yes | true for an output that switches on above the band, such as a cooler; false for one that switches on below it, such as a heater or a valve that adds water. |
| `safe_band` | number | yes | How far the reading may stray from the setpoint before an OutOfRange alert. No narrower than the hysteresis. |

### control, kind level {#profile-level}

Watches a falling level and warns before it runs out.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `kind` | `level` | yes | Names this policy. |
| `empty` | number | yes | The level treated as empty, such as 0.5 for a well in meters. |
| `warn_within` | whole number, at least 1 | yes | Raise RunningOut once the level is on course to reach empty within this many more samples. |

### control, kind surge {#profile-surge}

Warns when a reading changes faster than is safe, such as a river rising into a flash flood.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `kind` | `surge` | yes | Names this policy. |
| `rising` | true or false | yes | true to watch for a rapid rise, false for a rapid fall. |
| `limit` | number, above 0 | yes | The largest safe change between two samples. A bigger one raises ChangingFast. |

### control, kind monitor {#profile-monitor}

Reports readings and decides nothing: no output, no alerts.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `kind` | `monitor` | yes | Names this policy. |

### control, a kind of your own {#profile-custom}

A policy the library does not ship. Its parameters sit beside the kind, and the program that runs the profile registers the code that decides it.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `kind` | text | yes | The kind's own name, in lowercase words joined by underscores, such as frost_guard. |
| any other field | number, true or false, text | no | A parameter the kind's code reads: a number, true or false, or text. |

### power {#profile-power}

How often the node samples as its battery drains. The intervals may not shorten as the charge falls.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `active_secs` | whole number, at least 1 | yes | Seconds between samples on a healthy battery. |
| `saver_secs` | whole number, at least 1 | yes | Seconds between samples below saver_below. No shorter than active_secs. |
| `critical_secs` | whole number, at least 1 | yes | Seconds between samples below critical_below. No shorter than saver_secs. |
| `saver_below` | number, above 0, at most 1 | no, `0.5` | The state of charge, from 0 to 1, below which the node samples at saver_secs. |
| `critical_below` | number, above 0, at most 1 | no, `0.2` | The state of charge below which the node samples at critical_secs. Below saver_below. |
| `hysteresis` | number, at least 0 | no, `0.05` | How far above a threshold the charge must climb to leave the slower cadence, so a charge hovering at a threshold does not switch it every cycle. |

### presentation {#profile-presentation}

How a dashboard draws the node: the readings it adds, a tint, and the words for its own states.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `elements` | list of objects, see [presentation elements](#profile-element) | no, `[]` | The readings and node stats the dashboard draws, in the order they are offered. |
| `theme` | object, see [presentation theme](#profile-theme) | no | A tint for the dashboard. Each color is any CSS color. |
| `messages` | object of texts | no | The words for each state. or event. code the profile introduces, keyed by the code, such as state.lamp_on. |

### presentation elements {#profile-element}

One reading or node stat on the dashboard.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `key` | text | yes | The stable key the value travels under, in lowercase words joined by underscores, such as brooder_temperature. Used once per profile. |
| `unit` | text | yes | The unit's name, such as celsius, percent, ntu, or state for an on and off reading. |
| `label` | text | yes | The label shown when the reader's language has none of its own. |
| `labels` | object of texts | no | The label in other languages, keyed by locale tag, such as fr or sw. |
| `viz` | one of `spark`, `gauge`, `dial`, `bar`, `thermometer`, `droplet`, `battery`, `wind`, `sun`, `wave`, `switch`, `valve`, `chain`, `mesh`, `count` | yes | The graphic the value is drawn with. |
| `band` | list of 2 numbers | no | The safe band, [low, high] in the element's unit, drawn as the graphic's safe zone. The low end comes first. |
| `stat` | true or false | no, `false` | true for telemetry about the node itself, such as a battery or dropped packets, which the dashboard counts apart from the world it measures. |
| `scope` | `always`, or object | no, `always` | Which groups offer the element: always, or { "links": [...] } for groups on those link kinds only, such as mesh. |
| `span` | true or false | no, `false` | true to give a wide graphic two columns. |
| `value` | number | no | The value shown until the first real sample arrives. |
| `state` | text | no | The state shown until the first real sample arrives, for an on and off reading, such as state.lamp_off. Not together with value. |

### presentation theme {#profile-theme}

A tint for the dashboard. Each color is any CSS color.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `accent` | text | no | Links, focus, and the brand mark. |
| `ok` | text | no | A healthy reading, and an in-band gauge. |
| `warn` | text | no | A warning. |
| `alarm` | text | no | An alarm. |
| `track` | text | no | The unfilled rail behind gauges and bars. |
<!-- end -->

## When it goes wrong

A manifest a node could not run is refused as it loads. What gets past that shows
up as an output that does the wrong thing, or a warning that never comes. The ones
that cost an afternoon:

- **The output flickers, or a level never warns.** A controller is built fresh by
  every call to `controller`, and it remembers whether its output is on and what
  the reading before was. Build one for each reading and a heater switches off
  inside its deadband, and a level or a surge never has a reading to compare
  against. Build one when the node starts and hand it every reading.
- **A failed probe leaves the heater on.** A reading that is not a number raises
  `InvalidReading`, and the output holds its last state rather than acting on a
  number that is not there. Decide in the node what off means for its plant, and
  turn the output off, or raise an alarm, when the alert comes.
- **A manifest from a shared folder is refused for a field.** The format refuses a
  field it does not have rather than ignore it, so a misspelled `saver_bellow`
  cannot leave the default in place without a word. The reason names the field it
  was probably meant to be and where in the file it sits. Name the schema as the
  manifest's `$schema` and an editor marks the same field as it is typed.
- **A custom kind is refused as the node starts.** A kind the library does not ship
  loads with its fields as parameters, but no built-in controller decides it, so
  `controller` refuses it rather than hand back a node that reports readings and
  never drives its output. Register the code that decides it: through a
  `PolicyRegistry` in Rust, or in the program's own code from `control.params`
  elsewhere. A kind a letter away from a built-in one is named in the reason.
- **A warning means something else on a low battery.** `warn_within` counts
  samples, not minutes, so on the shipped well profile six samples are an hour
  ahead of dry at the active cadence and three hours at the saver one. A surge's
  limit is per sample too, so on the flood sensor a steady rise of 0.1 m a minute
  stays under the 0.3 m limit at the active 60 s and crosses it at the saver 300 s.
  Set both knowing the cadence stretches them, and check the numbers at each.
- **A glitch raises a flood warning.** A surge compares two readings and nothing
  else, so one bad sample from a noisy gauge looks like a jump. Pass the readings
  through a [median or a smoother](kit.md) first when the sensor is noisy.
- **A manifest is refused.** The reason names the field. A hysteresis has to be
  above zero or the output chatters, a safe band has to be at least as wide as the
  hysteresis, and the intervals have to lengthen as the battery drains.

## Where next

<!-- table: next profile -->
- [Rules](rules.md): Rules between nodes as a file.
- [Your own device](device.md): A sensor and an actuator pamoja has never heard of, written against the core traits, run against a rule, and published with nothing plugged in.
- [Sensor drivers](sensors.md): Datasheet-anchored drivers for eleven parts, from every language.
- Beside it: [Profiles](../profiles.md), [Running a profile](../run.md).
- Also in Profiles and robotics: [Robot motion](motion.md), [ROS 2 rules](ros2.md), [Zenoh keys](zenoh.md).
<!-- end -->

## Reference

<!-- table: reference profile -->
- Rust: [`pamoja-profile`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-profile)
- TypeScript: [`@pamoja/profile`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-profile)
- Python: [`pamoja.profile`](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-profile)
- C#: [`Pamoja.Profile`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-profile)
<!-- end -->
