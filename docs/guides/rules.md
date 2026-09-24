# Rules

A profile decides at the node that reads. A rule decides between nodes: it watches
one node's topic for a reading crossing a line and, the moment it does, drives an
actuator on another node or publishes a message, over the same link. Rules are
data. A rule file names the topic, the line, the release band that keeps the rule
from firing over and over as a reading hovers at the line, and what to do on the
way down and on the way back, so a fleet retunes a rule the way it retunes a
profile: a file that ships, not a firmware build.

The condition is the kit's trigger, a threshold with hysteresis that reports the
moment it is crossed and the moment the reading has come back far enough to
count, and nothing in between. In Rust the engine runs the whole file off any
link that can receive: it subscribes to each rule's topic, decodes each reading in
the codec the nodes publish in, switches the actuators it was given by name, and
publishes over the same link. In every language a rule evaluator is the engine's
deciding half on its own: the program hands it each reading with the topic it
arrived on, learns which rules set or cleared and what each calls for, and carries
that out with its own link and outputs. The two decide alike, because the engine
runs on one.

## What the example does

It loads a file of two rules on one raised bed. One waters the bed when the soil
dries below 30 and stops once it is wetter than 35. The other raises an alarm when
the bed is soaked past 60 and clears once it drops below 55. A probe on one link
sends six readings, and a watcher on another hears what the rules publish. In Rust
the engine runs the file off the broker and holds the valve; in the other
languages the evaluator judges each reading and the program runs the actions.

The second part is what goes wrong: a reading that is not a number, a topic no
rule watches, two files no engine could run, and a rule with no release band.

It proves:

- The same file loads in every language and says which topics to subscribe to and
  which outputs to hold.
- A rule fires once on the way down, at 28, and the readings in between change
  nothing: 31 is above the line, and 33 is inside the release band.
- One reading can fire two rules, in the order the file lists them: at 65 the
  watering rule clears and the alarm sets.
- Each rule that fires carries its actions in the file's order, and a rule with
  nothing under `otherwise` clears with nothing to do.
- The watcher hears every message the rules published, in order, and the valve
  switched exactly twice.
- A reading that is not a number is refused on a watched topic, where it would
  otherwise leave every rule as it was without a word, and a topic no rule watches
  is not judged at all.
- A file that watches a filter, or gives two rules one name, is refused with the
  rule and the reason.
- With no release band, four readings hovering at the line fire the rule four
  times; with a band of 5, once.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example rules" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example rules</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- rules" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- rules</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/rules.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/rules.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- rules" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- rules</code></div>
</div>
<!-- end -->

## Rust

In Rust, `Rules::from_json` reads the file and `Rules::check` says what no engine
could run. A `RuleEngine` runs the file off any link that can receive: `connect`
checks the rules, refuses a rule that drives an actuator it was not given, and
subscribes to every watched topic, and each `step` handles one message and returns
what fired, `Fired` values with the rule, the edge, the reading, and the actions it
carried out. A `RuleEvaluator` is the same decisions for a loop of your own.

<!-- snippet: examples/guides/rules.rs#example -->
From [`examples/guides/rules.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/rules.rs):

```rust
    use pamoja_codec::JsonCodec;
    use pamoja_core::{Receive, Transport};
    use pamoja_kit::Edge;
    use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
    use pamoja_profile::{Action, RuleEngine, Rules};

    // A rule is a file: the topic it watches, the line a reading crosses, the release
    // band that stops it firing over and over, and what to do on the way down and on the
    // way back. Two rules watch one bed here: one waters it when it dries past 30 and
    // stops once it is wetter than 35, and one raises an alarm when it is soaked past 60.
    let file = r#"{ "rules": [
  { "name": "water-when-dry",
    "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
    "then": [ { "drive": "bed-valve", "on": true },
              { "publish": "garden/bed-1/valve", "payload": "open" } ],
    "otherwise": [ { "drive": "bed-valve", "on": false },
                   { "publish": "garden/bed-1/valve", "payload": "closed" } ] },
  { "name": "flood-alarm",
    "when": { "topic": "garden/bed-1/moisture", "above": 60.0, "hysteresis": 5.0 },
    "then": [ { "publish": "garden/alarm", "payload": "waterlogged" } ] }
] }"#;

    // Three parties on one broker: the node that reads the bed, the engine that holds the
    // valve, and a watcher on the topics the rules publish to.
    let broker = LoopbackBroker::new();
    let mut probe = LoopbackTransport::new(broker.clone());
    let mut watcher = LoopbackTransport::new(broker.clone());
    probe.connect().await?;
    watcher.connect().await?;
    watcher.subscribe("garden/bed-1/valve").await?;
    watcher.subscribe("garden/alarm").await?;

    let valve = Valve::default();
    let mut engine = RuleEngine::new(
        Rules::from_json(file)?,
        LoopbackTransport::new(broker),
        JsonCodec,
    )
    .with_actuator("bed-valve", valve.clone());
    engine.connect().await?;
    println!(
        "watches   {}, and drives {}",
        engine.topics().join(", "),
        engine.actuators().join(", ")
    );

    // The bed dries out, is watered, and floods. A rule fires only as its condition sets
    // or clears, and the readings in between change nothing. At 65 two rules fire on one
    // reading, in the order the file lists them.
    for reading in [42.0f32, 31.0, 28.0, 33.0, 65.0, 50.0] {
        probe
            .send_text("garden/bed-1/moisture", &reading.to_string())
            .await?;
        let fired = engine.step().await?.expect("the link is up");
        if fired.is_empty() {
            println!("{reading:<10}nothing fired");
        }
        for one in &fired {
            let edge = match one.edge {
                Edge::Set => "set",
                Edge::Cleared => "cleared",
            };
            let actions: Vec<String> = one
                .actions
                .iter()
                .map(|action| match action {
                    Action::Drive { actuator, on } => {
                        format!("drive {actuator} {}", if *on { "on" } else { "off" })
                    }
                    Action::Publish { topic, payload } => format!("publish {payload} to {topic}"),
                })
                .collect();
            if actions.is_empty() {
                println!("{reading:<10}{} {edge}, with nothing to do", one.rule);
            } else {
                println!("{reading:<10}{} {edge}: {}", one.rule, actions.join(", "));
            }
        }
    }

    // The watcher heard every message the rules published, in the order they went out.
    let mut heard = Vec::new();
    for _ in 0..3 {
        let message = watcher.recv().await?.expect("a message");
        heard.push(message.text().expect("words").to_owned());
    }
    println!("heard     {}", heard.join(", "));
    let switches = valve.switches.lock().expect("valve lock").clone();
    let state = if switches.last() == Some(&true) {
        "on"
    } else {
        "off"
    };
    println!(
        "valve     switched {} times, and it is {state}",
        switches.len()
    );
```
<!-- end -->

What goes wrong, continuing from above:

<!-- snippet: examples/guides/rules.rs#wrong -->
From [`examples/guides/rules.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/rules.rs):

```rust
use pamoja_profile::RuleEvaluator;

// A program that moves its own messages hands each reading to an evaluator, the
// engine's deciding half on its own, and carries out what it says.
let mut evaluator = RuleEvaluator::new(Rules::from_json(file)?)?;

// A reading that is not a number, such as the NaN a failed probe reports, is refused
// on a watched topic rather than leaving every rule as it was with nothing to say why.
match evaluator.evaluate("garden/bed-1/moisture", f32::NAN) {
    Ok(_) => println!("a reading of NaN was judged, which should never happen"),
    Err(error) => println!("refused   {error}"),
}

// A topic no rule watches is not judged at all, so even a NaN there says nothing.
if evaluator
    .evaluate("garden/bed-2/moisture", f32::NAN)?
    .is_empty()
{
    println!(
        "ignored   no rule watches garden/bed-2/moisture, so even a NaN there is not judged"
    );
}

// A file no engine could run is refused as it loads, with the rule and the reason.
for edited in [
    file.replace("garden/bed-1/moisture", "garden/+/moisture"),
    file.replace("\"flood-alarm\"", "\"water-when-dry\""),
] {
    match Rules::from_json(&edited).and_then(RuleEvaluator::new) {
        Ok(_) => println!("a file no engine could run was accepted, which should never happen"),
        Err(error) => println!("refused   {error}"),
    }
}

// With no release band, readings that hover at the line set and clear the rule on
// every sample, and each edge switches the valve. The band of 5 holds it through them.
let fires = |text: &str| -> std::result::Result<usize, Box<dyn Error>> {
    let mut judge = RuleEvaluator::new(Rules::from_json(text)?)?;
    let mut count = 0;
    for reading in [29.9, 30.1, 29.8, 30.2] {
        count += judge.evaluate("garden/bed-1/moisture", reading)?.len();
    }
    Ok(count)
};
let bare = fires(&file.replace("\"hysteresis\": 5.0", "\"hysteresis\": 0.0"))?;
let banded = fires(file)?;
println!(
    "chatter   4 readings hovering at 30 fire the rule {bare} times with no release band, {banded} with a band of 5"
);
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/profile` loads the file with `RuleEvaluator.fromJson`, which
throws an `Error` saying why for a file no engine could run. `topics` and
`actuators` say what to subscribe to and which outputs to hold, and `evaluate`
returns what fired: each with the `rule`, the `edge` as `'set'` or `'cleared'`, the
`reading`, and the `actions`, each with a `kind` of `RuleActionKind.Drive` or
`RuleActionKind.Publish`. The program carries the actions out over its own link.

<!-- snippet: bindings/node/guides/rules.ts#example -->
From [`bindings/node/guides/rules.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/rules.ts):

```typescript
import { LoopbackBroker } from '@pamoja/loopback'
import { type RuleAction, RuleActionKind, RuleEvaluator } from '@pamoja/profile'

// A rule is a file: the topic it watches, the line a reading crosses, the release band
// that stops it firing over and over, and what to do on the way down and on the way back.
// Two rules watch one bed here: one waters it when it dries past 30 and stops once it is
// wetter than 35, and one raises an alarm when it is soaked past 60.
const file = `{ "rules": [
  { "name": "water-when-dry",
    "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
    "then": [ { "drive": "bed-valve", "on": true },
              { "publish": "garden/bed-1/valve", "payload": "open" } ],
    "otherwise": [ { "drive": "bed-valve", "on": false },
                   { "publish": "garden/bed-1/valve", "payload": "closed" } ] },
  { "name": "flood-alarm",
    "when": { "topic": "garden/bed-1/moisture", "above": 60.0, "hysteresis": 5.0 },
    "then": [ { "publish": "garden/alarm", "payload": "waterlogged" } ] }
] }`

// The evaluator judges each reading and says what the rules call for; the program moves
// the messages and holds the valve, which is the engine's work in Rust.
const evaluator = RuleEvaluator.fromJson(file)
console.log(`watches   ${evaluator.topics.join(', ')}, and drives ${evaluator.actuators.join(', ')}`)

async function main() {
  // Three parties on one broker: the node that reads the bed, the program that holds the
  // valve, and a watcher on the topics the rules publish to.
  const broker = new LoopbackBroker()
  const probe = broker.link()
  const link = broker.link()
  const watcher = broker.link()
  await probe.connect()
  await link.connect()
  await watcher.connect()
  await watcher.subscribe('garden/bed-1/valve')
  await watcher.subscribe('garden/alarm')
  for (const topic of evaluator.topics) {
    await link.subscribe(topic)
  }

  const valve: boolean[] = []
  const run = async (action: RuleAction) => {
    if (action.kind === RuleActionKind.Drive) {
      valve.push(action.on === true)
    } else {
      await link.send(action.topic!, action.payload!)
    }
  }
  const described = (action: RuleAction) =>
    action.kind === RuleActionKind.Drive
      ? `drive ${action.actuator} ${action.on ? 'on' : 'off'}`
      : `publish ${action.payload} to ${action.topic}`

  // The bed dries out, is watered, and floods. A rule fires only as its condition sets or
  // clears, and the readings in between change nothing. At 65 two rules fire on one
  // reading, in the order the file lists them.
  for (const reading of [42, 31, 28, 33, 65, 50]) {
    await probe.send('garden/bed-1/moisture', String(reading))
    const message = (await link.recv())!
    const fired = evaluator.evaluate(message.topic, message.number!)
    const at = String(reading).padEnd(10)
    if (fired.length === 0) {
      console.log(`${at}nothing fired`)
    }
    for (const one of fired) {
      for (const action of one.actions) {
        await run(action)
      }
      if (one.actions.length === 0) {
        console.log(`${at}${one.rule} ${one.edge}, with nothing to do`)
      } else {
        console.log(`${at}${one.rule} ${one.edge}: ${one.actions.map(described).join(', ')}`)
      }
    }
  }

  // The watcher heard every message the rules published, in the order they went out.
  const heard: string[] = []
  for (let i = 0; i < 3; i += 1) {
    heard.push((await watcher.recv())!.text!)
  }
  console.log(`heard     ${heard.join(', ')}`)
  console.log(`valve     switched ${valve.length} times, and it is ${valve[valve.length - 1] ? 'on' : 'off'}`)
  return { heard, valve }
}

main()
```
<!-- end -->

What goes wrong, continuing from above:

<!-- snippet: bindings/node/guides/rules.ts#wrong -->
From [`bindings/node/guides/rules.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/rules.ts):

```typescript
// A reading that is not a number, such as the NaN a failed probe reports, is refused on
// a watched topic rather than leaving every rule as it was with nothing to say why.
const judge = RuleEvaluator.fromJson(file)
try {
  judge.evaluate('garden/bed-1/moisture', Number.NaN)
  console.log('a reading of NaN was judged, which should never happen')
} catch (error) {
  console.log(`refused   ${(error as Error).message}`)
}

// A topic no rule watches is not judged at all, so even a NaN there says nothing.
if (judge.evaluate('garden/bed-2/moisture', Number.NaN).length === 0) {
  console.log('ignored   no rule watches garden/bed-2/moisture, so even a NaN there is not judged')
}

// A file no engine could run is refused as it loads, with the rule and the reason.
for (const edited of [
  file.split('garden/bed-1/moisture').join('garden/+/moisture'),
  file.replace('"flood-alarm"', '"water-when-dry"'),
]) {
  try {
    RuleEvaluator.fromJson(edited)
    console.log('a file no engine could run was accepted, which should never happen')
  } catch (error) {
    console.log(`refused   ${(error as Error).message}`)
  }
}

// With no release band, readings that hover at the line set and clear the rule on every
// sample, and each edge switches the valve. The band of 5 holds it through them.
const fires = (text: string) => {
  const rules = RuleEvaluator.fromJson(text)
  let count = 0
  for (const reading of [29.9, 30.1, 29.8, 30.2]) {
    count += rules.evaluate('garden/bed-1/moisture', reading).length
  }
  return count
}
const bare = fires(file.split('"hysteresis": 5.0').join('"hysteresis": 0.0'))
const banded = fires(file)
console.log(
  `chatter   4 readings hovering at 30 fire the rule ${bare} times with no release band, ${banded} with a band of 5`,
)
```
<!-- end -->

## Python

In Python, `pamoja.profile` loads the file with `RuleEvaluator.from_json`, which
raises `PamojaError` for a file no engine could run, as `evaluate` does for a
reading that is not a number on a watched topic. `evaluate` returns a list of
`RuleFired`, whose `edge` compares equal to a `pamoja.kit.Edge` member and whose
`actions` are `RuleAction` values with a `kind` equal to a `RuleActionKind` member.

<!-- snippet: bindings/python/guides/rules.py#example -->
From [`bindings/python/guides/rules.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/rules.py):

```python
import asyncio

from pamoja.loopback import LoopbackBroker
from pamoja.profile import RuleActionKind, RuleEvaluator

# A rule is a file: the topic it watches, the line a reading crosses, the release band that
# stops it firing over and over, and what to do on the way down and on the way back. Two
# rules watch one bed here: one waters it when it dries past 30 and stops once it is wetter
# than 35, and one raises an alarm when it is soaked past 60.
file = """{ "rules": [
  { "name": "water-when-dry",
    "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
    "then": [ { "drive": "bed-valve", "on": true },
              { "publish": "garden/bed-1/valve", "payload": "open" } ],
    "otherwise": [ { "drive": "bed-valve", "on": false },
                   { "publish": "garden/bed-1/valve", "payload": "closed" } ] },
  { "name": "flood-alarm",
    "when": { "topic": "garden/bed-1/moisture", "above": 60.0, "hysteresis": 5.0 },
    "then": [ { "publish": "garden/alarm", "payload": "waterlogged" } ] }
] }"""

# The evaluator judges each reading and says what the rules call for; the program moves the
# messages and holds the valve, which is the engine's work in Rust.
evaluator = RuleEvaluator.from_json(file)
print(f"watches   {', '.join(evaluator.topics)}, and drives {', '.join(evaluator.actuators)}")


def described(action) -> str:
    """Says what one action does, in the words the output uses."""
    if action.kind == RuleActionKind.DRIVE:
        return f"drive {action.actuator} {'on' if action.on else 'off'}"
    return f"publish {action.payload} to {action.topic}"


async def main() -> tuple[list[str], list[bool]]:
    # Three parties on one broker: the node that reads the bed, the program that holds the
    # valve, and a watcher on the topics the rules publish to.
    broker = LoopbackBroker()
    probe = broker.link()
    link = broker.link()
    watcher = broker.link()
    await probe.connect()
    await link.connect()
    await watcher.connect()
    await watcher.subscribe("garden/bed-1/valve")
    await watcher.subscribe("garden/alarm")
    for topic in evaluator.topics:
        await link.subscribe(topic)

    valve: list[bool] = []

    # The bed dries out, is watered, and floods. A rule fires only as its condition sets or
    # clears, and the readings in between change nothing. At 65 two rules fire on one
    # reading, in the order the file lists them.
    for reading in [42, 31, 28, 33, 65, 50]:
        await probe.send("garden/bed-1/moisture", str(reading))
        message = await link.recv()
        fired = evaluator.evaluate(message.topic, message.number)
        at = f"{reading:<10}"
        if not fired:
            print(f"{at}nothing fired")
        for one in fired:
            for action in one.actions:
                if action.kind == RuleActionKind.DRIVE:
                    valve.append(action.on)
                else:
                    await link.send(action.topic, action.payload)
            if not one.actions:
                print(f"{at}{one.rule} {one.edge}, with nothing to do")
            else:
                print(f"{at}{one.rule} {one.edge}: {', '.join(described(a) for a in one.actions)}")

    # The watcher heard every message the rules published, in the order they went out.
    heard = [(await watcher.recv()).text for _ in range(3)]
    print(f"heard     {', '.join(heard)}")
    print(f"valve     switched {len(valve)} times, and it is {'on' if valve[-1] else 'off'}")
    return heard, valve


heard, valve = asyncio.run(main())
```
<!-- end -->

What goes wrong, continuing from above:

<!-- snippet: bindings/python/guides/rules.py#wrong -->
From [`bindings/python/guides/rules.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/rules.py):

```python
from pamoja.core import PamojaError

# A reading that is not a number, such as the NaN a failed probe reports, is refused on a
# watched topic rather than leaving every rule as it was with nothing to say why.
judge = RuleEvaluator.from_json(file)
try:
    judge.evaluate("garden/bed-1/moisture", float("nan"))
    print("a reading of NaN was judged, which should never happen")
except PamojaError as error:
    print(f"refused   {error}")

# A topic no rule watches is not judged at all, so even a NaN there says nothing.
if not judge.evaluate("garden/bed-2/moisture", float("nan")):
    print("ignored   no rule watches garden/bed-2/moisture, so even a NaN there is not judged")

# A file no engine could run is refused as it loads, with the rule and the reason.
for edited in [
    file.replace("garden/bed-1/moisture", "garden/+/moisture"),
    file.replace('"flood-alarm"', '"water-when-dry"'),
]:
    try:
        RuleEvaluator.from_json(edited)
        print("a file no engine could run was accepted, which should never happen")
    except PamojaError as error:
        print(f"refused   {error}")


def fires(text: str) -> int:
    """Counts how often the rules fire as four readings hover at the line."""
    rules = RuleEvaluator.from_json(text)
    return sum(len(rules.evaluate("garden/bed-1/moisture", r)) for r in [29.9, 30.1, 29.8, 30.2])


# With no release band, readings that hover at the line set and clear the rule on every
# sample, and each edge switches the valve. The band of 5 holds it through them.
bare = fires(file.replace('"hysteresis": 5.0', '"hysteresis": 0.0'))
banded = fires(file)
print(
    f"chatter   4 readings hovering at 30 fire the rule {bare} times with no release band, "
    f"{banded} with a band of 5"
)
```
<!-- end -->

## C#

In C#, `RuleEvaluator.FromJson` loads the file and throws `PamojaException` for one
no engine could run; the evaluator holds native state and is disposed with `using`.
`Evaluate` returns `RuleFired` records, whose `Edge` is the kit's `Edge` and whose
`Actions` are `RuleAction` records with a `Kind` of `RuleActionKind.Drive` or
`RuleActionKind.Publish`, and the fields belonging to that kind.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RulesGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/RulesGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RulesGuide.cs):

```csharp
// A rule is a file: the topic it watches, the line a reading crosses, the release
// band that stops it firing over and over, and what to do on the way down and on
// the way back. Two rules watch one bed here: one waters it when it dries past 30
// and stops once it is wetter than 35, and one raises an alarm when it is soaked
// past 60.
const string file = """
    { "rules": [
      { "name": "water-when-dry",
        "when": { "topic": "garden/bed-1/moisture", "below": 30.0, "hysteresis": 5.0 },
        "then": [ { "drive": "bed-valve", "on": true },
                  { "publish": "garden/bed-1/valve", "payload": "open" } ],
        "otherwise": [ { "drive": "bed-valve", "on": false },
                       { "publish": "garden/bed-1/valve", "payload": "closed" } ] },
      { "name": "flood-alarm",
        "when": { "topic": "garden/bed-1/moisture", "above": 60.0, "hysteresis": 5.0 },
        "then": [ { "publish": "garden/alarm", "payload": "waterlogged" } ] }
    ] }
    """;

// The evaluator judges each reading and says what the rules call for; the program
// moves the messages and holds the valve, which is the engine's work in Rust.
using var evaluator = RuleEvaluator.FromJson(file);
Console.WriteLine(
    $"watches   {string.Join(", ", evaluator.Topics)}, and drives {string.Join(", ", evaluator.Actuators)}");

// Three parties on one broker: the node that reads the bed, the program that holds
// the valve, and a watcher on the topics the rules publish to.
using var broker = new LoopbackBroker();
using LoopbackTransport probe = broker.Link();
using LoopbackTransport link = broker.Link();
using LoopbackTransport watcher = broker.Link();
await probe.ConnectAsync();
await link.ConnectAsync();
await watcher.ConnectAsync();
await watcher.SubscribeAsync("garden/bed-1/valve");
await watcher.SubscribeAsync("garden/alarm");
foreach (string topic in evaluator.Topics)
{
    await link.SubscribeAsync(topic);
}

var valve = new List<bool>();
static string Described(RuleAction action) => action.Kind == RuleActionKind.Drive
    ? $"drive {action.Actuator} {(action.On == true ? "on" : "off")}"
    : $"publish {action.Payload} to {action.Topic}";

// The bed dries out, is watered, and floods. A rule fires only as its condition
// sets or clears, and the readings in between change nothing. At 65 two rules fire
// on one reading, in the order the file lists them.
foreach (int reading in new[] { 42, 31, 28, 33, 65, 50 })
{
    await probe.SendAsync("garden/bed-1/moisture", reading.ToString());
    TransportMessage message = (await link.ReceiveAsync())!;
    IReadOnlyList<RuleFired> fired = evaluator.Evaluate(message.Topic, (float)message.Number!);
    string at = $"{reading,-10}";
    if (fired.Count == 0)
    {
        Console.WriteLine($"{at}nothing fired");
    }

    foreach (RuleFired one in fired)
    {
        foreach (RuleAction action in one.Actions)
        {
            if (action.Kind == RuleActionKind.Drive)
            {
                valve.Add(action.On == true);
            }
            else
            {
                await link.SendAsync(action.Topic!, action.Payload!);
            }
        }

        string edge = one.Edge == Pamoja.Kit.Edge.Set ? "set" : "cleared";
        Console.WriteLine(one.Actions.Count == 0
            ? $"{at}{one.Rule} {edge}, with nothing to do"
            : $"{at}{one.Rule} {edge}: {string.Join(", ", one.Actions.Select(Described))}");
    }
}

// The watcher heard every message the rules published, in the order they went out.
var heard = new List<string>();
for (int i = 0; i < 3; i++)
{
    heard.Add((await watcher.ReceiveAsync())!.Text);
}

Console.WriteLine($"heard     {string.Join(", ", heard)}");
Console.WriteLine($"valve     switched {valve.Count} times, and it is {(valve[^1] ? "on" : "off")}");
```
<!-- end -->

What goes wrong, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RulesGuide.cs#wrong -->
From [`bindings/dotnet/samples/Pamoja.Guides/RulesGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RulesGuide.cs):

```csharp
// A reading that is not a number, such as the NaN a failed probe reports, is
// refused on a watched topic rather than leaving every rule as it was with nothing
// to say why.
using var judge = RuleEvaluator.FromJson(file);
try
{
    judge.Evaluate("garden/bed-1/moisture", float.NaN);
    Console.WriteLine("a reading of NaN was judged, which should never happen");
}
catch (PamojaException error)
{
    Console.WriteLine($"refused   {error.Message}");
}

// A topic no rule watches is not judged at all, so even a NaN there says nothing.
if (judge.Evaluate("garden/bed-2/moisture", float.NaN).Count == 0)
{
    Console.WriteLine("ignored   no rule watches garden/bed-2/moisture, so even a NaN there is not judged");
}

// A file no engine could run is refused as it loads, with the rule and the reason.
foreach (string edited in new[]
{
    file.Replace("garden/bed-1/moisture", "garden/+/moisture"),
    file.Replace("\"flood-alarm\"", "\"water-when-dry\""),
})
{
    try
    {
        using var accepted = RuleEvaluator.FromJson(edited);
        Console.WriteLine("a file no engine could run was accepted, which should never happen");
    }
    catch (PamojaException error)
    {
        Console.WriteLine($"refused   {error.Message}");
    }
}

// With no release band, readings that hover at the line set and clear the rule on
// every sample, and each edge switches the valve. The band of 5 holds it through
// them.
static int Fires(string text)
{
    using var rules = RuleEvaluator.FromJson(text);
    return new[] { 29.9f, 30.1f, 29.8f, 30.2f }
        .Sum(reading => rules.Evaluate("garden/bed-1/moisture", reading).Count);
}

int bare = Fires(file.Replace("\"hysteresis\": 5.0", "\"hysteresis\": 0.0"));
int banded = Fires(file);
Console.WriteLine(
    $"chatter   4 readings hovering at 30 fire the rule {bare} times with no release band, {banded} with a band of 5");
```
<!-- end -->

## Values at a glance

A rule reads as a sentence: when `garden/bed-1/moisture` is `below` 30, `drive`
`bed-valve` `on`, and otherwise `drive` it off. Every field of the file, with what
it holds and its default, is under [Every field](#every-field) below, generated
from the published schema an editor checks the file with.

**When a condition moves.** Every condition starts cleared:

| The line | Sets when a reading is | Clears when a reading is | Any other reading |
| --- | --- | --- | --- |
| `below` | below the line | above the line plus the hysteresis | leaves it as it was |
| `above` | above the line | below the line less the hysteresis | leaves it as it was |

A reading exactly on the threshold does not set the condition, and one exactly on
the far edge of the band does not clear it. The example's watering rule sets below
30 and clears above 35, and its alarm sets above 60 and clears below 55.

**What fired:**

| Field | Holds |
| --- | --- |
| rule | the rule's name |
| edge | `set` when the condition became true, `cleared` when it stopped holding |
| reading | the reading that moved it |
| actions | the rule's `then` actions when it set and its `otherwise` actions when it cleared, in the file's order |

Rules that fire on one reading come back in the order the file lists them.

**What a file is refused for,** with the rule named in the reason:

| The file | The reason says |
| --- | --- |
| a field the format does not have, such as a misspelled `hysterisis` | `` unknown field `hysterisis` ``, and the field it was probably meant to be |
| a `$schema` for another format | `written in rules format 2`, or that it `is not a pamoja rules schema` |
| a condition with both `above` and `below`, or neither | `names both`, or `needs a line to cross` |
| a drive with no `on`, a publish with no `payload`, or an action that does both | `needs`, and the field it lacks, or `not both` |
| a rule with no name | `a rule needs a name` |
| two rules with one name | `two rules share the name` |
| a rule with no topic | `needs a topic to watch` |
| a rule watching a topic with `+` or `#` | `a filter; a rule watches one topic exactly` |
| a threshold that is not a finite number | `needs a threshold that is a finite number` |
| a hysteresis below zero, or not a finite number | `needs a hysteresis that is a finite number of zero or more` |
| a rule with nothing under `then` or `otherwise` | `has nothing to do` |
| a drive with no actuator named | `drives an actuator with no name` |
| a publish with no topic, or to a filter | `publishes to no topic`, or `a filter rather than a topic` |

A reading is refused, as an error from `evaluate` or from the engine's `step`, when a
rule watches its topic and it is not a finite number. The engine also refuses a
payload its codec cannot decode, and, as it connects, a rule that drives an
actuator it was not given.

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| load and check a file | `Rules::from_json(text)`, `check()`, `to_json()` |
| build one in code | `Rules::new().with(Rule::new(name, Condition::below(topic, line).with_hysteresis(band)).then(action).otherwise(action))` |
| run it off a link | `RuleEngine::new(rules, link, codec)`, `with_actuator(name, output)`, `connect()`, `step()` |
| judge readings yourself | `RuleEvaluator::new(rules)`, `evaluate(topic, reading)` |
| see what it needs | `topics()`, `actuators()`, `watches(topic)`, `is_set(rule)` |

### TypeScript

| To | Call |
| --- | --- |
| load a file | `RuleEvaluator.fromJson(text)`, `toJson()` |
| judge a reading | `evaluate(topic, reading)`, then each fired `rule`, `edge`, `reading`, `actions` |
| read an action | `kind` of `RuleActionKind.Drive` with `actuator` and `on`, or `RuleActionKind.Publish` with `topic` and `payload` |
| see what it needs | `topics`, `actuators`, `watches(topic)`, `isSet(rule)` |

### Python

| To | Call |
| --- | --- |
| load a file | `RuleEvaluator.from_json(text)`, `to_json()` |
| judge a reading | `evaluate(topic, reading)`, then each fired `rule`, `edge`, `reading`, `actions` |
| read an action | `kind` of `RuleActionKind.DRIVE` with `actuator` and `on`, or `RuleActionKind.PUBLISH` with `topic` and `payload` |
| see what it needs | `topics`, `actuators`, `watches(topic)`, `is_set(rule)` |

### C#

| To | Call |
| --- | --- |
| load a file | `RuleEvaluator.FromJson(text)`, `ToJson()` |
| judge a reading | `Evaluate(topic, reading)`, then each fired `Rule`, `Edge`, `Reading`, `Actions` |
| read an action | `Kind` of `RuleActionKind.Drive` with `Actuator` and `On`, or `RuleActionKind.Publish` with `Topic` and `Payload` |
| see what it needs | `Topics`, `Actuators`, `Watches(topic)`, `IsSet(rule)` |

<!-- languages end -->

## Every field

These tables are generated from the published JSON Schema,
[`rules-1.json`](https://pamoja.molex.cloud/schema/rules-1.json), the same file an
editor checks a rule file against. Name it as the file's `$schema` and an editor
such as VS Code completes the fields and marks a wrong one as it is typed. The
schema cannot say that two rules have different names, so the library's own check
is the last word on that.

<!-- table: schema rules -->
### The rule file {#rules-fields}

Rules between nodes: when one topic's reading crosses a line, drive an output or publish a message, and undo it once the reading comes back.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `$schema` | text | no | The format the file is written in: this schema's address, or a copy of it by the same file name. An editor reads it to check the file as it is typed. |
| `rules` | list of objects, see [rules](#rules-rule) | yes | The rules, checked against each reading in the order they are listed. |

### rules {#rules-rule}

One rule: a line through one topic's readings, and what to do as a reading crosses it and comes back.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `name` | text | yes | A name no other rule in the file has, such as water-when-dry. The engine reports it when the rule fires. |
| `when` | object, see [when](#rules-when) | yes | The line a topic's readings cross. It names one side, above or below. |
| `then` | list of [drive](#rules-drive) and [publish](#rules-publish) actions | no | What to do once, the moment the condition becomes true. |
| `otherwise` | list of [drive](#rules-drive) and [publish](#rules-publish) actions | no | What to do once, the moment the condition stops holding. |

### when {#rules-when}

The line a topic's readings cross. It names one side, above or below.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `topic` | text | yes | The topic whose readings are watched, exactly as the node publishes to it. One topic, so no + or # wildcards. |
| `above` | number | no | The condition holds while readings are above this line. |
| `below` | number | no | The condition holds while readings are below this line. |
| `hysteresis` | number, at least 0 | no, `0` | How far back past the line a reading must come for the condition to clear, so readings hovering at the line do not fire it over and over. |

### then and otherwise, drive {#rules-drive}

Switches an output the program running the rules holds under this name.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `drive` | text | yes | The output's name, such as bed-valve. |
| `on` | true or false | yes | true to switch it on, false to switch it off. |

### then and otherwise, publish {#rules-publish}

Publishes a message over the link the rules run on.

| Field | Value | Required | What it does |
| --- | --- | --- | --- |
| `publish` | text | yes | The topic to publish to. One topic, so no + or # wildcards. |
| `payload` | text | yes | The text to publish. |
<!-- end -->

## When it goes wrong

A file no engine could run is refused as it loads. What gets past that shows up as a
rule that never fires, fires too often, or leaves an output in the wrong state. The
ones that cost an afternoon:

- **A rule never fires.** A rule compares each message's topic with its own
  exactly, so `garden/bed-1/moisture` misses `garden/Bed-1/moisture` and
  `garden/bed-1/moisture/`. Check the topic the node really publishes to. A reading
  exactly on the threshold does not set the condition either.
- **The valve switches on every reading.** A rule with no release band, which is
  what leaving out `hysteresis` gives, sets and clears each time a noisy reading
  crosses the line. Set the band wider than the sensor's noise.
- **A failed probe goes unnoticed, or stops the loop.** A reading that is not a
  number on a watched topic comes back as an error, so the failure is seen. A loop
  that stops at the first error then stops every rule with it: log the error and
  go on to the next message.
- **The valve stays open after a restart.** Conditions live in memory and start
  cleared, and a rule only clears from the set state. A node restarted with the
  valve open and the bed at 33 never closes it, because nothing sets the rule
  again until the bed dries past 30. Put every output in a known state as the
  program starts.
- **Two rules fight over one output.** Rules act independently, so two rules that
  drive one actuator leave it wherever the last edge put it. Give each output one
  rule.
- **The engine will not connect.** In Rust, a rule drives an actuator the engine was
  not given; give it with `with_actuator` under the name the file uses. Elsewhere,
  check the evaluator's `actuators` against the outputs the program holds.
- **Every reading fails to decode.** The engine reads each payload with the codec
  the nodes publish in, so a node sending text to an engine built for CBOR fails
  every reading. Build the engine with the codec the nodes use.

## Where next

<!-- table: next rules -->
- [Device profiles](profile.md): Named, ready-to-run device profiles from plain data or a JSON manifest.
- [Event bus](bus.md): An in-memory typed publish and subscribe event bus, with publishers that never wait and subscribers that count what they miss.
- [Helpers](kit.md): Plain-language helper math.
- Also in Profiles and robotics: [Robot motion](motion.md), [ROS 2 rules](ros2.md), [Zenoh keys](zenoh.md).
<!-- end -->

## Reference

<!-- table: reference rules -->
- Rust: `Rules`, `RuleEngine`, and `RuleEvaluator` in [`pamoja-profile`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-rules)
- TypeScript: [`@pamoja/profile`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-rules)
- Python: [`pamoja.profile`](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-rules)
- C#: [`Pamoja.Profile`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-rules)
<!-- end -->
