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

The controller is the decision half. Hand it a reading and it says what the output
should do and whether the reading crossed a threshold worth raising. A manifest is
checked as it loads, so a value no node could run is refused with the reason
rather than running quietly wrong, and serializing writes the defaulted fields out
in full, so a manifest that has been round-tripped leaves nothing for the next
reader to infer.

## What the example does

It loads a brooder-heater profile from a JSON manifest and asks its power plan how
often the node samples at three charges. It then walks one controller through a
morning of five brooder temperatures, printing what the heat lamp does at each. It
writes the profile back out, and declares how a dashboard draws the node.

The second part runs the other two policies on the profiles the library ships: a
well whose level falls toward dry, and a river that rises too fast.

The third part is what goes wrong: a probe that fails, a controller built again
for each reading, two manifests no node could run, a misspelled field, and a
control kind the library does not ship.

The manifest sets the three sampling intervals but neither battery threshold, so
the 50% and 20% printed come from the library's defaults rather than from the
file. The five readings sit around the 32 C target: the lamp switches on at
31.5 C or below and off at 32.5 C or above, and between the two it stays as it
was.

It proves:

- A manifest parses into the name, topic, setpoint policy and sampling schedule
  the node runs on, and a threshold the file leaves out takes its documented
  default.
- The power plan puts a charge of 80% in the active mode, 30% in saver, and 10% in
  critical, each with its own interval.
- One controller holds the lamp through the deadband in both directions, and
  raises `OutOfRange` for a reading more than 4 C from the target.
- Written back out, the manifest names the defaulted thresholds and loads as the
  same profile, and a dashboard element travels in it with its key, unit, graphic,
  and band intact.
- A level warns once a fall puts dry within six samples, and a surge warns when
  one sample rises more than its limit.
- A reading that is not a number raises `InvalidReading` and the lamp holds, and a
  controller built again for each reading forgets the lamp was on.
- A manifest with a hysteresis of zero, or intervals that shorten as the battery
  drains, is refused with the reason. A misspelled field is not, and its default
  stays. A control kind the library does not ship decides nothing until the node
  supplies the policy.

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
holding the reason for one no node could run, and `check` runs the same check on
a profile built in code. `controller` builds a `Controller` whose `evaluate`
returns a `Reaction`: the actuator setting, if the profile drives one, and an
`Alert`. `power.plan()` turns the schedule into the `pamoja-power` governor. A
policy of your own implements `Policy`, and a `PolicyRegistry` resolves a
manifest's custom kind to it.

<!-- snippet: examples/guides/profile.rs#example -->
From [`examples/guides/profile.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/profile.rs):

```rust
use pamoja_profile::{ElementSpec, Presentation, Profile, Viz};

// A profile is plain data, so a fleet ships one as a file rather than as code. This
// manifest names no battery thresholds, so the documented defaults apply.
let manifest = r#"{
    "name": "brooder-heater",
    "topic": "poultry/brooder/temperature",
    "control": {
        "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
        "cooling": false, "safe_band": 4.0
    },
    "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}"#;
let profile = Profile::from_json(manifest)?;
println!("profile   {} reports on {}", profile.name, profile.topic);
println!(
    "defaults  the file names no battery thresholds, so saver starts below {:.0}% and critical below {:.0}%",
    profile.power.saver_below * 100.0,
    profile.power.critical_below * 100.0
);

// The schedule becomes a power plan, which says what mode a charge puts the node in
// and how long it waits between samples there.
let plan = profile.power.plan();
for charge in [0.8, 0.3, 0.1] {
    println!(
        "battery   at {:.0}% it runs {:?} and samples every {} s",
        charge * 100.0,
        plan.mode(charge),
        plan.interval(charge).as_secs()
    );
}

// One controller runs for the life of the node, because it remembers whether the
// lamp is on. The lamp switches on at 31.5 C or below and off at 32.5 C or above, the
// setpoint less and plus the hysteresis, and in between it stays as it was. A reading
// more than 4 C from the setpoint raises an alert as well.
let mut controller = profile.controller();
let mut lamp = false;
for reading in [27.5, 31.8, 32.6, 32.1, 31.4] {
    let reaction = controller.evaluate(reading);
    let on = reaction.actuator.expect("this profile drives a lamp");
    let change = match (lamp, on) {
        (false, true) => "lamp on",
        (true, false) => "lamp off",
        (true, true) => "lamp stays on",
        (false, false) => "lamp stays off",
    };
    let alert = reaction
        .alert
        .map(|alert| format!(", alert {}", alert.kind()))
        .unwrap_or_default();
    println!("{:<10}{change}{alert}", format!("{reading} C"));
    lamp = on;
}

// Written back out, the manifest names the thresholds the file left to their
// defaults, so the next reader has nothing to infer, and it loads as the same profile.
let shared = profile.to_json()?;
if shared.contains("saver_below") && Profile::from_json(&shared)?.to_json()? == shared {
    println!("shared    written back out, it names saver_below and loads as the same profile");
}

// The manifest also carries how a dashboard draws the node: one element here, the
// brooder's temperature on a thermometer with the band the chicks are safe in.
let drawn = profile.clone().with_presentation(
    Presentation::new().with_element(
        ElementSpec::new(
            "brooder_temperature",
            "celsius",
            "Brooder temperature",
            Viz::Thermometer,
        )
        .with_band(28.0, 36.0),
    ),
);
let element = &drawn
    .presentation
    .as_ref()
    .expect("a declared presentation")
    .elements[0];
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
let mut well = Profile::well_level().controller();
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
let mut river = Profile::flood_sensor().controller();
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

What goes wrong, continuing from above:

<!-- snippet: examples/guides/profile.rs#wrong -->
From [`examples/guides/profile.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/profile.rs):

```rust
use pamoja_profile::ControlSpec;

// A probe that fails reports a reading that is not a number. The controller raises
// it rather than going quiet, and the lamp holds its state; what off means for the
// chicks is the node's call.
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
let first = profile.controller().evaluate(27.5).actuator;
let then = profile.controller().evaluate(31.8).actuator;
if first == Some(true) && then == Some(false) {
    println!(
        "fresh     built again for each reading, the controller turns the lamp off at 31.8 C"
    );
}

// A manifest no node could run is refused as it loads, with the reason.
for edited in [
    manifest.replace("\"hysteresis\": 0.5", "\"hysteresis\": 0.0"),
    manifest.replace("\"saver_secs\": 600", "\"saver_secs\": 60"),
] {
    match Profile::from_json(&edited) {
        Ok(_) => {
            println!("a manifest no node could run was accepted, which should never happen")
        }
        Err(error) => println!("refused   {error}"),
    }
}

// A misspelled optional field is not an error: it names no field, so the default
// stays. Writing the profile back out shows what the node understood.
let misspelled = manifest.replace(
    "\"critical_secs\": 1800 }",
    "\"critical_secs\": 1800, \"saver_bellow\": 0.3 }",
);
let understood = Profile::from_json(&misspelled)?;
println!(
    "typo      saver_bellow names no field, so saver still starts below {:.0}%",
    understood.power.saver_below * 100.0
);

// A kind the library does not ship loads with its parameters and runs as a monitor
// until the node supplies the policy, so it drives nothing and raises nothing.
let custom = Profile::from_json(
    &manifest.replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""),
)?;
if let ControlSpec::Custom { kind, params } = &custom.control {
    let reaction = custom.controller().evaluate(27.5);
    if reaction.actuator.is_none() && reaction.alert.is_none() {
        println!(
            "custom    {kind} loads with {} parameters, and with no policy behind it drives nothing",
            params.len()
        );
    }
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/profile` loads a manifest with `Profile.fromJson`, which
throws an `Error` saying why for one no node could run, as the constructor does
for parts that could not. A reaction is a plain object whose `actuator` is a
boolean or undefined and whose `alert` carries a `kind` from `AlertKind`.
`powerPlan` returns the power governor, whose intervals are in microseconds.

<!-- snippet: bindings/node/guides/profile.ts#example -->
From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import { Profile, Viz } from '@pamoja/profile'

// A profile is plain data, so a fleet ships one as a file rather than as code. This
// manifest names no battery thresholds, so the documented defaults apply.
const manifest = `{
  "name": "brooder-heater",
  "topic": "poultry/brooder/temperature",
  "control": {
    "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
    "cooling": false, "safe_band": 4.0
  },
  "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}`
const profile = Profile.fromJson(manifest)
console.log(`profile   ${profile.name} reports on ${profile.topic}`)
console.log(
  `defaults  the file names no battery thresholds, so saver starts below ${(profile.power.saverBelow * 100).toFixed(0)}% and critical below ${(profile.power.criticalBelow * 100).toFixed(0)}%`,
)

// The schedule becomes a power plan, which says what mode a charge puts the node in and
// how long it waits between samples there, in microseconds.
const plan = profile.powerPlan()
for (const charge of [0.8, 0.3, 0.1]) {
  console.log(
    `battery   at ${(charge * 100).toFixed(0)}% it runs ${plan.mode(charge)} and samples every ${plan.intervalUs(charge) / 1_000_000} s`,
  )
}

// One controller runs for the life of the node, because it remembers whether the lamp is
// on. The lamp switches on at 31.5 C or below and off at 32.5 C or above, the setpoint
// less and plus the hysteresis, and in between it stays as it was. A reading more than
// 4 C from the setpoint raises an alert as well.
const controller = profile.controller()
let lamp = false
for (const reading of [27.5, 31.8, 32.6, 32.1, 31.4]) {
  const reaction = controller.evaluate(reading)
  const on = reaction.actuator === true
  const change = on ? (lamp ? 'lamp stays on' : 'lamp on') : lamp ? 'lamp off' : 'lamp stays off'
  const alert = reaction.alert ? `, alert ${reaction.alert.kind}` : ''
  console.log(`${`${reading} C`.padEnd(10)}${change}${alert}`)
  lamp = on
}

// Written back out, the manifest names the thresholds the file left to their defaults,
// so the next reader has nothing to infer, and it loads as the same profile.
const shared = profile.toJson()
if (shared.includes('saver_below') && Profile.fromJson(shared).toJson() === shared) {
  console.log('shared    written back out, it names saver_below and loads as the same profile')
}

// The manifest also carries how a dashboard draws the node: one element here, the
// brooder's temperature on a thermometer with the band the chicks are safe in.
const drawn = profile.withPresentation({
  elements: [
    {
      key: 'brooder_temperature',
      unit: 'celsius',
      label: 'Brooder temperature',
      viz: Viz.Thermometer,
      band: [28, 36],
    },
  ],
})
const element = drawn.presentation?.elements[0]
console.log(
  `draws     ${element?.key} in ${element?.unit} on a ${element?.viz}, safe from ${element?.band?.[0]} to ${element?.band?.[1]}`,
)
```
<!-- end -->

The other policies, continuing from above:

<!-- snippet: bindings/node/guides/profile.ts#kinds -->
From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import { AlertKind } from '@pamoja/profile'

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
```
<!-- end -->

What goes wrong, continuing from above:

<!-- snippet: bindings/node/guides/profile.ts#wrong -->
From [`bindings/node/guides/profile.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/profile.ts):

```typescript
import { ControlKind } from '@pamoja/profile'

// A probe that fails reports a reading that is not a number. The controller raises it
// rather than going quiet, and the lamp holds its state; what off means for the chicks is
// the node's call.
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
  console.log(
    'fresh     built again for each reading, the controller turns the lamp off at 31.8 C',
  )
}

// A manifest no node could run is refused as it loads, with the reason.
for (const edited of [
  manifest.replace('"hysteresis": 0.5', '"hysteresis": 0.0'),
  manifest.replace('"saver_secs": 600', '"saver_secs": 60'),
]) {
  try {
    Profile.fromJson(edited)
    console.log('a manifest no node could run was accepted, which should never happen')
  } catch (error) {
    console.log(`refused   ${(error as Error).message}`)
  }
}

// A misspelled optional field is not an error: it names no field, so the default stays.
// Writing the profile back out shows what the node understood.
const misspelled = manifest.replace(
  '"critical_secs": 1800 }',
  '"critical_secs": 1800, "saver_bellow": 0.3 }',
)
const understood = Profile.fromJson(misspelled)
console.log(
  `typo      saver_bellow names no field, so saver still starts below ${(understood.power.saverBelow * 100).toFixed(0)}%`,
)

// A kind the library does not ship loads with its parameters and runs as a monitor until
// the node supplies the policy, so it drives nothing and raises nothing.
const custom = Profile.fromJson(manifest.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
if (custom.control.kind === ControlKind.Custom) {
  const reaction = custom.controller().evaluate(27.5)
  if (reaction.actuator == null && reaction.alert == null) {
    const count = Object.keys(custom.control.params ?? {}).length
    console.log(
      `custom    ${custom.control.customKind} loads with ${count} parameters, and with no policy behind it drives nothing`,
    )
  }
}
```
<!-- end -->

## Python

In Python, `pamoja.profile` loads a manifest with `Profile.from_json`, which raises
`PamojaError` for one no node could run, as the constructor does for such parts.
`ValueError` is kept for an argument of the wrong shape, such as a setpoint
control with no hysteresis. A reaction's `actuator` is `True`, `False`, or `None`,
and its `alert` has a `kind` that compares equal to an `AlertKind` member.
`power_plan` returns the power governor, whose intervals are in microseconds.

<!-- snippet: bindings/python/guides/profile.py#example -->
From [`bindings/python/guides/profile.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/profile.py):

```python
from pamoja.profile import ElementSpec, Presentation, Profile, Viz

# A profile is plain data, so a fleet ships one as a file rather than as code. This
# manifest names no battery thresholds, so the documented defaults apply.
manifest = """{
    "name": "brooder-heater",
    "topic": "poultry/brooder/temperature",
    "control": {
        "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
        "cooling": false, "safe_band": 4.0
    },
    "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}"""
profile = Profile.from_json(manifest)
print(f"profile   {profile.name} reports on {profile.topic}")
print(
    "defaults  the file names no battery thresholds, so saver starts below "
    f"{profile.power.saver_below * 100:.0f}% and critical below "
    f"{profile.power.critical_below * 100:.0f}%"
)

# The schedule becomes a power plan, which says what mode a charge puts the node in and
# how long it waits between samples there, in microseconds.
plan = profile.power_plan()
for charge in [0.8, 0.3, 0.1]:
    print(
        f"battery   at {charge * 100:.0f}% it runs {plan.mode(charge)} "
        f"and samples every {plan.interval_us(charge) // 1_000_000} s"
    )

# One controller runs for the life of the node, because it remembers whether the lamp is
# on. The lamp switches on at 31.5 C or below and off at 32.5 C or above, the setpoint
# less and plus the hysteresis, and in between it stays as it was. A reading more than 4 C
# from the setpoint raises an alert as well.
controller = profile.controller()
lamp = False
for reading in [27.5, 31.8, 32.6, 32.1, 31.4]:
    reaction = controller.evaluate(reading)
    on = reaction.actuator is True
    if on:
        change = "lamp stays on" if lamp else "lamp on"
    else:
        change = "lamp off" if lamp else "lamp stays off"
    alert = f", alert {reaction.alert.kind}" if reaction.alert else ""
    print(f"{f'{reading:g} C':<10}{change}{alert}")
    lamp = on

# Written back out, the manifest names the thresholds the file left to their defaults, so
# the next reader has nothing to infer, and it loads as the same profile.
shared = profile.to_json()
if "saver_below" in shared and Profile.from_json(shared).to_json() == shared:
    print("shared    written back out, it names saver_below and loads as the same profile")

# The manifest also carries how a dashboard draws the node: one element here, the brooder's
# temperature on a thermometer with the band the chicks are safe in.
drawn = profile.with_presentation(
    Presentation(
        [
            ElementSpec(
                "brooder_temperature", "celsius", "Brooder temperature", Viz.THERMOMETER,
                band=(28, 36),
            ),
        ]
    )
)
element = drawn.presentation.elements[0]
low, high = element.band
print(
    f"draws     {element.key} in {element.unit} on a {element.viz}, "
    f"safe from {low:g} to {high:g}"
)
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

What goes wrong, continuing from above:

<!-- snippet: bindings/python/guides/profile.py#wrong -->
From [`bindings/python/guides/profile.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/profile.py):

```python
from pamoja.core import PamojaError
from pamoja.profile import ControlKind

# A probe that fails reports a reading that is not a number. The controller raises it
# rather than going quiet, and the lamp holds its state; what off means for the chicks is
# the node's call.
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

# A manifest no node could run is refused as it loads, with the reason.
for edited in [
    manifest.replace('"hysteresis": 0.5', '"hysteresis": 0.0'),
    manifest.replace('"saver_secs": 600', '"saver_secs": 60'),
]:
    try:
        Profile.from_json(edited)
        print("a manifest no node could run was accepted, which should never happen")
    except PamojaError as error:
        print(f"refused   {error}")

# A misspelled optional field is not an error: it names no field, so the default stays.
# Writing the profile back out shows what the node understood.
misspelled = manifest.replace(
    '"critical_secs": 1800 }', '"critical_secs": 1800, "saver_bellow": 0.3 }'
)
understood = Profile.from_json(misspelled)
print(
    "typo      saver_bellow names no field, so saver still starts below "
    f"{understood.power.saver_below * 100:.0f}%"
)

# A kind the library does not ship loads with its parameters and runs as a monitor until
# the node supplies the policy, so it drives nothing and raises nothing.
custom = Profile.from_json(manifest.replace('"kind": "setpoint"', '"kind": "brooder_guard"'))
if custom.control.kind == ControlKind.CUSTOM:
    reaction = custom.controller().evaluate(27.5)
    if reaction.actuator is None and reaction.alert is None:
        print(
            f"custom    {custom.control.custom_kind} loads with {len(custom.control.params)} "
            "parameters, and with no policy behind it drives nothing"
        )
```
<!-- end -->

## C#

In C#, a `Profile` and a `Controller` hold native state and are disposed with
`using`. `Profile.FromJson` throws `PamojaException` for a manifest no node could
run, as the constructor does for such parts. `Evaluate` returns a `Reaction`
record whose `Actuator` is a `bool?` and whose `Alert` carries an `AlertKind`.
The `PowerPlan` property is the power governor, whose intervals are in
microseconds.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs):

```csharp
// A profile is plain data, so a fleet ships one as a file rather than as code.
// This manifest names no battery thresholds, so the documented defaults apply.
const string manifest = """
{
    "name": "brooder-heater",
    "topic": "poultry/brooder/temperature",
    "control": {
        "kind": "setpoint", "setpoint": 32.0, "hysteresis": 0.5,
        "cooling": false, "safe_band": 4.0
    },
    "power": { "active_secs": 120, "saver_secs": 600, "critical_secs": 1800 }
}
""";
using var profile = Profile.FromJson(manifest);
Console.WriteLine($"profile   {profile.Name} reports on {profile.Topic}");
Console.WriteLine(Invariant(
    $"defaults  the file names no battery thresholds, so saver starts below {profile.Power.SaverBelow * 100:F0}% and critical below {profile.Power.CriticalBelow * 100:F0}%"));

// The schedule becomes a power plan, which says what mode a charge puts the node
// in and how long it waits between samples there, in microseconds.
PowerPlan plan = profile.PowerPlan;
foreach (float charge in new[] { 0.8f, 0.3f, 0.1f })
{
    Console.WriteLine(Invariant(
        $"battery   at {charge * 100:F0}% it runs {plan.Mode(charge)} and samples every {plan.IntervalUs(charge) / 1_000_000} s"));
}

// One controller runs for the life of the node, because it remembers whether the
// lamp is on. The lamp switches on at 31.5 C or below and off at 32.5 C or above,
// the setpoint less and plus the hysteresis, and in between it stays as it was. A
// reading more than 4 C from the setpoint raises an alert as well.
using Controller controller = profile.Controller();
bool lamp = false;
foreach (float reading in new[] { 27.5f, 31.8f, 32.6f, 32.1f, 31.4f })
{
    Reaction reaction = controller.Evaluate(reading);
    bool on = reaction.Actuator == true;
    string change = on
        ? lamp ? "lamp stays on" : "lamp on"
        : lamp ? "lamp off" : "lamp stays off";
    string alert = reaction.Alert is { } raised ? $", alert {raised.Kind}" : "";
    string at = Invariant($"{reading} C");
    Console.WriteLine($"{at,-10}{change}{alert}");
    lamp = on;
}

// Written back out, the manifest names the thresholds the file left to their
// defaults, so the next reader has nothing to infer, and it loads as the same
// profile.
string shared = profile.ToJson();
using (var reloaded = Profile.FromJson(shared))
{
    if (shared.Contains("saver_below") && reloaded.ToJson() == shared)
    {
        Console.WriteLine("shared    written back out, it names saver_below and loads as the same profile");
    }
}

// The manifest also carries how a dashboard draws the node: one element here, the
// brooder's temperature on a thermometer with the band the chicks are safe in.
using var drawn = profile.WithPresentation(new Presentation(
[
    new ElementSpec("brooder_temperature", "celsius", "Brooder temperature", Viz.Thermometer)
    {
        Band = [28f, 36f],
    },
]));
ElementSpec element = drawn.Presentation!.Elements[0];
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

// A surge warns when a reading moves too far in one sample. The shipped flood
// sensor warns when a river rises more than 0.3 m between two readings.
using (var floodSensor = Profile.FloodSensor())
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

What goes wrong, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs#wrong -->
From [`bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ProfileGuide.cs):

```csharp
// A probe that fails reports a reading that is not a number. The controller raises
// it rather than going quiet, and the lamp holds its state; what off means for the
// chicks is the node's call.
Reaction failed = controller.Evaluate(float.NaN);
if (failed.Alert is { } invalid)
{
    string holds = failed.Actuator == true ? "on" : "off";
    Console.WriteLine($"probe     a reading of NaN raises {invalid.Kind}, and the lamp holds {holds}");
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

// A manifest no node could run is refused as it loads, with the reason.
foreach (string edited in new[]
{
    manifest.Replace("\"hysteresis\": 0.5", "\"hysteresis\": 0.0"),
    manifest.Replace("\"saver_secs\": 600", "\"saver_secs\": 60"),
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

// A misspelled optional field is not an error: it names no field, so the default
// stays. Writing the profile back out shows what the node understood.
string misspelled = manifest.Replace(
    "\"critical_secs\": 1800 }",
    "\"critical_secs\": 1800, \"saver_bellow\": 0.3 }");
using (var understood = Profile.FromJson(misspelled))
{
    Console.WriteLine(Invariant(
        $"typo      saver_bellow names no field, so saver still starts below {understood.Power.SaverBelow * 100:F0}%"));
}

// A kind the library does not ship loads with its parameters and runs as a monitor
// until the node supplies the policy, so it drives nothing and raises nothing.
using var custom = Profile.FromJson(manifest.Replace("\"kind\": \"setpoint\"", "\"kind\": \"brooder_guard\""));
ControlPolicy policy = custom.Control;
if (policy.Kind == ControlKind.Custom)
{
    using Controller inert = custom.Controller();
    Reaction reaction = inert.Evaluate(27.5f);
    if (reaction.Actuator is null && reaction.Alert is null)
    {
        Console.WriteLine(
            $"custom    {policy.CustomKind} loads with {policy.Params!.Count} parameters, and with no policy behind it drives nothing");
    }
}
```
<!-- end -->

## Values at a glance

**A manifest:**

| Field | Required | Holds |
| --- | --- | --- |
| `name` | yes | a stable name, such as `brooder-heater` |
| `description` | no | what the profile is for, in a sentence or two |
| `topic` | yes | the one topic each reading is published to |
| `control` | yes | the policy, named by `kind`, with that kind's fields beside it |
| `power` | yes | the sampling schedule |
| `presentation` | no | the elements, theme, and wording a dashboard draws the node with |

**The control kinds:**

| `kind` | Fields | What it does with each reading | Alert |
| --- | --- | --- | --- |
| `setpoint` | `setpoint`, `hysteresis`, `cooling`, `safe_band` | a heater switches on at `setpoint - hysteresis` or below and off at `setpoint + hysteresis` or above, a cooler the other way round, and in between the output stays as it was | `OutOfRange` when the reading is more than `safe_band` from the setpoint |
| `level` | `empty`, `warn_within` | drives nothing; estimates how many more samples the last fall takes to reach `empty`, rounded up | `RunningOut` once that is `warn_within` or fewer |
| `surge` | `rising`, `limit` | drives nothing; compares each reading with the one before | `ChangingFast` when one sample moves more than `limit` the watched way |
| `monitor` | none | drives nothing and raises nothing | none |
| any other | its own, kept as parameters | runs as a monitor until the node supplies the policy: through a `PolicyRegistry` in Rust, from `control.params` elsewhere | whatever that policy raises |

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

**The power schedule:**

| Field | Holds | When the file leaves it out |
| --- | --- | --- |
| `active_secs` | seconds between samples at a healthy charge | required |
| `saver_secs` | seconds between samples while conserving | required |
| `critical_secs` | seconds between samples when critically low | required |
| `saver_below` | the charge, from 0 to 1, below which the node conserves | 0.5 |
| `critical_below` | the charge below which it does the least | 0.2 |
| `hysteresis` | how far above a threshold the charge must climb before the node leaves the lower cadence | 0.05 |

A charge below `critical_below` is critical, below `saver_below` is saver, and
anything else is active. A charge that is not a number, from a fuel gauge that
failed to answer, is taken as critical. While the panel charges, the plan eases
the node up one step, from critical to saver and from saver to active. A running
node remembers its mode: it drops a cadence as soon as the charge crosses a
threshold, and climbs back only once the charge is `hysteresis` above it, so a
charge hovering at 50% does not switch the cadence on every sample.

**What a manifest is refused for,** as it loads and as each language's
constructor builds one:

| The manifest | The reason says |
| --- | --- |
| an empty name | `` `name` must not be empty `` |
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
named for the profile, a description, and snake_case keys and kinds.

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| load and write | `Profile::from_json(text)`, `to_json()`, `check()` |
| start from a preset | `Profile::vaccine_fridge_monitor()`, `irrigation_node()`, `well_level()`, `flood_sensor()` |
| build one | `Profile::new(name, topic, ControlSpec, PowerSchedule::new(active, saver, critical))`, `with_thresholds`, `with_description`, `with_presentation`, `with_element` |
| decide a reading | `controller()`, then `evaluate(reading)`; `Controller::setpoint`, `level`, `surge`, `monitor` |
| read the reaction | `actuator`, `alert`, `Alert::kind()` |
| schedule sampling | `power.plan()`, then `mode(charge)`, `mode_while_charging(charge, charging)`, `interval(charge)` |
| supply a policy | `impl Policy`, `PolicyRegistry::new().register(kind, factory)`, `resolve(&profile.control)`, `Node::with_policy` |

### TypeScript

| To | Call |
| --- | --- |
| load and write | `Profile.fromJson(text)`, `toJson()` |
| start from a preset | `Profile.vaccineFridgeMonitor()`, `irrigationNode()`, `wellLevel()`, `floodSensor()` |
| build one | `new Profile(name, topic, control, power)`, `withDescription`, `withPresentation` |
| decide a reading | `controller()`, then `evaluate(reading)`; `Controller.setpoint`, `level`, `surge`, `monitor` |
| read the reaction | `actuator`, `alert.kind`, `AlertKind` |
| schedule sampling | `powerPlan()`, then `mode(charge)`, `modeWhileCharging(charge, charging)`, `intervalUs(charge)` |
| read a custom kind | `control.kind === ControlKind.Custom`, `control.customKind`, `control.params` |

### Python

| To | Call |
| --- | --- |
| load and write | `Profile.from_json(text)`, `to_json()` |
| start from a preset | `Profile.vaccine_fridge_monitor()`, `irrigation_node()`, `well_level()`, `flood_sensor()` |
| build one | `Profile(name, topic, ControlPolicy(...), PowerScheduleSpec(...))`, `with_description`, `with_presentation` |
| decide a reading | `controller()`, then `evaluate(reading)`; `Controller.setpoint`, `level`, `surge`, `monitor` |
| read the reaction | `actuator`, `alert.kind`, `AlertKind` |
| schedule sampling | `power_plan()`, then `mode(charge)`, `mode_while_charging(charge, charging)`, `interval_us(charge)` |
| read a custom kind | `control.kind == ControlKind.CUSTOM`, `control.custom_kind`, `control.params` |

### C#

| To | Call |
| --- | --- |
| load and write | `Profile.FromJson(text)`, `ToJson()` |
| start from a preset | `Profile.VaccineFridgeMonitor()`, `IrrigationNode()`, `WellLevel()`, `FloodSensor()` |
| build one | `new Profile(name, topic, ControlPolicy, PowerSchedule)`, `WithDescription`, `WithPresentation` |
| decide a reading | `Controller()`, then `Evaluate(reading)`; `Controller.Setpoint`, `Level`, `Surge`, `Monitor` |
| read the reaction | `Actuator`, `Alert?.Kind`, `AlertKind` |
| schedule sampling | `PowerPlan`, then `Mode(charge)`, `ModeWhileCharging(charge, charging)`, `IntervalUs(charge)` |
| read a custom kind | `Control.Kind == ControlKind.Custom`, `Control.CustomKind`, `Control.Params` |

<!-- languages end -->

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
- **A setting in the file makes no difference.** A field the library does not know,
  such as a misspelled `saver_bellow`, names nothing, so it is ignored and the
  default stays. Write the loaded profile back out with `to_json` and read what
  the node understood.
- **A custom kind does nothing.** A kind the library does not ship loads with its
  fields as parameters and runs as a monitor, so it drives nothing and raises
  nothing until the node supplies the policy: through a `PolicyRegistry` in Rust,
  or in the program's own code from `control.params` elsewhere.
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
- Beside it: [Profiles](../profiles.md).
- Also in Profiles and robotics: [Robot motion](motion.md), [ROS 2 rules](ros2.md), [Zenoh keys](zenoh.md).
<!-- end -->

## Reference

<!-- table: reference profile -->
- Rust: [`pamoja-profile`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_profile/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-profile)
- TypeScript: [`@pamoja/profile`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_profile.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-profile)
- Python: [`pamoja.profile`](https://pamoja.molex.cloud/docs/reference/python/pamoja/profile.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-profile)
- C#: [`Pamoja.Profile`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Profile.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-profile)
<!-- end -->
