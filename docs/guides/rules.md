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
publishes over the same link. In TypeScript, Python, and C# the file is the same
data, read with the language's own JSON, and the loop is the program's own: the
trigger decides the condition, and the program carries out the actions, which is
the same shape those languages drive a profile's controller in. A reading on a
topic is a number written out, so the message's `number` reads it and a `send`
takes the text of the next one; nothing here touches bytes.

## What the example does

It loads a rule that waters a raised bed when the soil dries below 30 and stops
once it is wetter than 35, feeds five readings through it from a probe on one link,
and watches the valve it drives and the messages it publishes from another.

It proves:

- A rule file parses into a topic, a line, a release band, and two lists of
  actions, and the same text loads in every language.
- The rule fires once on the way down, at 28, and once on the way back, at 36, and
  the readings in between change nothing: 31 is above the line, and 33 is inside
  the release band.
- Each edge drives the valve and publishes to the other topic in the order the
  file gives, so a watcher on that topic hears `open` and then `closed`.
- The valve switched exactly twice, so a reading that holds the state costs no
  relay cycle.

## Run it

The example below is a test that runs in CI, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo test -p pamoja-examples --test guides rules -- --nocapture" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo test -p pamoja-examples --test guides rules -- --nocapture</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run test:guides -- rules" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run test:guides -- rules</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/rules.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/rules.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- rules" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- rules</code></div>
</div>
<!-- end -->

## Rust

<!-- snippet: examples/tests/guides/rules.rs#example -->
From [`examples/tests/guides/rules.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/rules.rs):

```rust
use pamoja_codec::JsonCodec;
use pamoja_core::{Receive, Transport};
use pamoja_kit::Edge;
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use pamoja_profile::{Compare, RuleEngine, Rules};

// A rule is a file: the topic it watches, the line a reading crosses, the release
// band that stops it firing over and over, and what to do on the way down and on
// the way back. The same file runs in every language.
let rules = Rules::from_json(
    r#"{ "rules": [ {
        "name": "water-when-dry",
        "when": { "topic": "garden/bed-1/moisture", "compare": "below",
                  "threshold": 30.0, "hysteresis": 5.0 },
        "then": [ { "do": "drive", "actuator": "bed-valve", "on": true },
                  { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" } ],
        "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false },
                       { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" } ]
    } ] }"#,
)
.expect("a well-formed rule file");
let when = &rules.rules[0].when;
let side = match when.compare {
    Compare::Below => "below",
    Compare::Above => "above",
};
let clears = when.threshold + when.hysteresis;
println!(
    "the rule watches {} {side} {}, clearing above {clears}",
    when.topic, when.threshold
);

// Three parties on one broker: the node that reads the bed, the engine that holds
// the valve, and a watcher on the topic the rule publishes to.
let broker = LoopbackBroker::new();
let mut probe = LoopbackTransport::new(broker.clone());
let mut watcher = LoopbackTransport::new(broker.clone());
probe.connect().await.expect("the probe connects");
watcher.connect().await.expect("the watcher connects");
watcher
    .subscribe("garden/bed-1/valve")
    .await
    .expect("a subscription");

let valve = Valve::default();
let mut engine = RuleEngine::new(rules, LoopbackTransport::new(broker), JsonCodec)
    .with_actuator("bed-valve", valve.clone());
engine.connect().await.expect("the engine connects");

// The bed dries out and is watered back: the rule fires once on the way down and
// once on the way back, and holds its state for the readings in between.
for reading in [42.0f32, 31.0, 28.0, 33.0, 36.0] {
    probe
        .send_text("garden/bed-1/moisture", &reading.to_string())
        .await
        .expect("the probe publishes");
    let fired = engine
        .step()
        .await
        .expect("a step")
        .expect("the link is up");
    let edge = match fired.first().map(|fired| fired.edge) {
        Some(Edge::Set) => "set",
        Some(Edge::Cleared) => "cleared",
        None => "no edge",
    };
    let open = valve.switches.lock().expect("valve lock").last().copied();
    let state = if open == Some(true) { "on" } else { "off" };
    println!("{reading}: {edge}, valve {state}");
}

// The watcher on the other topic heard each edge as the rule published it.
let mut heard = Vec::new();
for _ in 0..2 {
    let message = watcher.recv().await.expect("recv").expect("a message");
    heard.push(message.text().expect("words").to_owned());
}
println!("the watcher heard {}", heard.join(", "));
let switched = valve.switches.lock().expect("valve lock").len();
println!("the valve switched {switched} times");
```
<!-- end -->

The valve is the same kind of struct the [your own device](device.md) guide
builds, an `Actuator` over a `bool`, and the engine owns it under the name the
file uses. `Rules::from_json` reads the file, `RuleEngine::new` takes the link and
the codec the readings arrive in, `with_actuator` hands over each output by name,
`connect` checks the file and subscribes, and each `step` handles one message and
returns what fired.

## TypeScript

<!-- snippet: bindings/node/guides/rules.ts#example -->
From [`bindings/node/guides/rules.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/rules.ts):

```typescript
import { Trigger } from '@pamoja/kit'
import { LoopbackBroker } from '@pamoja/loopback'

// A rule is a file: the topic it watches, the line a reading crosses, the release band
// that stops it firing over and over, and what to do on the way down and on the way
// back. The same file runs in every language; here the program reads it as data and
// drives the loop itself, with the kit's trigger deciding the condition.
const rules = JSON.parse(`{ "rules": [ {
  "name": "water-when-dry",
  "when": { "topic": "garden/bed-1/moisture", "compare": "below",
            "threshold": 30.0, "hysteresis": 5.0 },
  "then": [ { "do": "drive", "actuator": "bed-valve", "on": true },
            { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" } ],
  "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false },
                 { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" } ]
} ] }`)
const rule = rules.rules[0]
const when = rule.when
const clears = when.threshold + when.hysteresis
console.log(`the rule watches ${when.topic} ${when.compare} ${when.threshold}, clearing above ${clears}`)

async function main() {
  // Three parties on one broker: the node that reads the bed, the engine that holds the
  // valve, and a watcher on the topic the rule publishes to.
  const broker = new LoopbackBroker()
  const probe = broker.link()
  const engine = broker.link()
  const watcher = broker.link()
  await probe.connect()
  await engine.connect()
  await watcher.connect()
  await watcher.subscribe('garden/bed-1/valve')
  await engine.subscribe(when.topic)

  // The condition is the trigger; the actions are the program's own.
  const trigger =
    when.compare === 'below'
      ? Trigger.below(when.threshold, when.hysteresis)
      : Trigger.above(when.threshold, when.hysteresis)
  const valve = { open: false, switches: 0 }
  const run = async (actions: Array<Record<string, string | boolean>>) => {
    for (const action of actions) {
      if (action.do === 'drive') {
        valve.open = action.on as boolean
        valve.switches += 1
      } else {
        await engine.send(action.topic as string, action.payload as string)
      }
    }
  }

  // The bed dries out and is watered back: the rule fires once on the way down and once
  // on the way back, and holds its state for the readings in between.
  for (const reading of [42, 31, 28, 33, 36]) {
    await probe.send(when.topic, String(reading))
    const message = (await engine.recv())!
    const edge = trigger.update(message.number!)
    if (edge === 'set') await run(rule.then)
    else if (edge === 'cleared') await run(rule.otherwise)
    console.log(`${reading}: ${edge ?? 'no edge'}, valve ${valve.open ? 'on' : 'off'}`)
  }

  // The watcher on the other topic heard each edge as the rule published it.
  const heard: string[] = []
  for (let i = 0; i < 2; i += 1) {
    heard.push((await watcher.recv())!.text!)
  }
  console.log(`the watcher heard ${heard.join(', ')}`)
  console.log(`the valve switched ${valve.switches} times`)

  return { heard, valve }
}

main()
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/rules.py#example -->
From [`bindings/python/guides/rules.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/rules.py):

```python
import asyncio
import json

from pamoja.kit import Edge, Trigger
from pamoja.loopback import LoopbackBroker

# A rule is a file: the topic it watches, the line a reading crosses, the release band
# that stops it firing over and over, and what to do on the way down and on the way
# back. The same file runs in every language; here the program reads it as data and
# drives the loop itself, with the kit's trigger deciding the condition.
rules = json.loads("""{ "rules": [ {
  "name": "water-when-dry",
  "when": { "topic": "garden/bed-1/moisture", "compare": "below",
            "threshold": 30.0, "hysteresis": 5.0 },
  "then": [ { "do": "drive", "actuator": "bed-valve", "on": true },
            { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" } ],
  "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false },
                 { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" } ]
} ] }""")
rule = rules["rules"][0]
when = rule["when"]
clears = when["threshold"] + when["hysteresis"]
print(f"the rule watches {when['topic']} {when['compare']} {when['threshold']:g}, clearing above {clears:g}")


async def main() -> tuple[list[str], bool, int]:
    # Three parties on one broker: the node that reads the bed, the engine that holds
    # the valve, and a watcher on the topic the rule publishes to.
    broker = LoopbackBroker()
    probe = broker.link()
    engine = broker.link()
    watcher = broker.link()
    await probe.connect()
    await engine.connect()
    await watcher.connect()
    await watcher.subscribe("garden/bed-1/valve")
    await engine.subscribe(when["topic"])

    # The condition is the trigger; the actions are the program's own.
    trigger = (
        Trigger.below(when["threshold"], when["hysteresis"])
        if when["compare"] == "below"
        else Trigger.above(when["threshold"], when["hysteresis"])
    )
    valve = {"open": False, "switches": 0}

    async def run(actions: list[dict]) -> None:
        for action in actions:
            if action["do"] == "drive":
                valve["open"] = action["on"]
                valve["switches"] += 1
            else:
                await engine.send(action["topic"], action["payload"])

    # The bed dries out and is watered back: the rule fires once on the way down and
    # once on the way back, and holds its state for the readings in between.
    for reading in [42, 31, 28, 33, 36]:
        await probe.send(when["topic"], str(reading))
        message = await engine.recv()
        edge = trigger.update(message.number)
        if edge == Edge.SET:
            await run(rule["then"])
        elif edge == Edge.CLEARED:
            await run(rule["otherwise"])
        print(f"{reading}: {edge or 'no edge'}, valve {'on' if valve['open'] else 'off'}")

    # The watcher on the other topic heard each edge as the rule published it.
    heard = [(await watcher.recv()).text for _ in range(2)]
    print(f"the watcher heard {', '.join(heard)}")
    print(f"the valve switched {valve['switches']} times")
    return heard, valve["open"], valve["switches"]


heard, valve_open, switches = asyncio.run(main())
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/RulesGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/RulesGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/RulesGuide.cs):

```csharp
// A rule is a file: the topic it watches, the line a reading crosses, the release
// band that stops it firing over and over, and what to do on the way down and on
// the way back. The same file runs in every language; here the program reads it
// as data and drives the loop itself, with the kit's trigger deciding the
// condition.
using JsonDocument rules = JsonDocument.Parse("""
    { "rules": [ {
      "name": "water-when-dry",
      "when": { "topic": "garden/bed-1/moisture", "compare": "below",
                "threshold": 30.0, "hysteresis": 5.0 },
      "then": [ { "do": "drive", "actuator": "bed-valve", "on": true },
                { "do": "publish", "topic": "garden/bed-1/valve", "payload": "open" } ],
      "otherwise": [ { "do": "drive", "actuator": "bed-valve", "on": false },
                     { "do": "publish", "topic": "garden/bed-1/valve", "payload": "closed" } ]
    } ] }
    """);
JsonElement rule = rules.RootElement.GetProperty("rules")[0];
JsonElement when = rule.GetProperty("when");
string topic = when.GetProperty("topic").GetString()!;
string compare = when.GetProperty("compare").GetString()!;
float threshold = when.GetProperty("threshold").GetSingle();
float hysteresis = when.GetProperty("hysteresis").GetSingle();
Console.WriteLine(
    $"the rule watches {topic} {compare} {threshold}, clearing above {threshold + hysteresis}");

// Three parties on one broker: the node that reads the bed, the engine that holds
// the valve, and a watcher on the topic the rule publishes to.
using var broker = new LoopbackBroker();
using LoopbackTransport probe = broker.Link();
using LoopbackTransport engine = broker.Link();
using LoopbackTransport watcher = broker.Link();
await probe.ConnectAsync();
await engine.ConnectAsync();
await watcher.ConnectAsync();
await watcher.SubscribeAsync("garden/bed-1/valve");
await engine.SubscribeAsync(topic);

// The condition is the trigger; the actions are the program's own.
using Trigger trigger = compare == "below"
    ? Trigger.Below(threshold, hysteresis)
    : Trigger.Above(threshold, hysteresis);
bool valveOpen = false;
int switches = 0;
async Task Run(JsonElement actions)
{
    foreach (JsonElement action in actions.EnumerateArray())
    {
        if (action.GetProperty("do").GetString() == "drive")
        {
            valveOpen = action.GetProperty("on").GetBoolean();
            switches += 1;
        }
        else
        {
            await engine.SendAsync(
                action.GetProperty("topic").GetString()!,
                action.GetProperty("payload").GetString()!);
        }
    }
}

// The bed dries out and is watered back: the rule fires once on the way down and
// once on the way back, and holds its state for the readings in between.
foreach (float reading in new[] { 42f, 31f, 28f, 33f, 36f })
{
    await probe.SendAsync(topic, reading.ToString());
    TransportMessage message = (await engine.ReceiveAsync())!;
    Edge? edge = trigger.Update((float)message.Number!);
    if (edge == Edge.Set)
    {
        await Run(rule.GetProperty("then"));
    }
    else if (edge == Edge.Cleared)
    {
        await Run(rule.GetProperty("otherwise"));
    }

    string edgeText = edge?.ToString().ToLowerInvariant() ?? "no edge";
    Console.WriteLine($"{reading}: {edgeText}, valve {(valveOpen ? "on" : "off")}");
}

// The watcher on the other topic heard each edge as the rule published it.
var heard = new List<string>();
for (int i = 0; i < 2; i++)
{
    heard.Add((await watcher.ReceiveAsync())!.Text);
}

Console.WriteLine($"the watcher heard {string.Join(", ", heard)}");
Console.WriteLine($"the valve switched {switches} times");
```
<!-- end -->

## Reference

<!-- table: reference rules -->
- Rust: the `Transport` and `Receive` traits in [`pamoja-core`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_core/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-rules)
- TypeScript: [`@pamoja/kit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-rules)
- Python: [`pamoja.kit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-rules)
- C#: [`Pamoja.Kit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-rules)
<!-- end -->
