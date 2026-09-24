# Power

A node on a battery or a panel lives or dies by how much it sleeps. Two decisions
carry most of that: how long to stay awake in a cycle, and how long the cycle
should be. `pamoja-power` holds both as plain arithmetic. A `DutyCycle` splits a
period between working and sleeping and reports the share spent awake, which is the
usual first proxy for average draw. A `PowerPlan` is the governor above it: give it
a battery state of charge and it names a `PowerMode` and the interval to wait before
the next cycle, so a node that would otherwise go dark in a cloudy week keeps
reporting, less often.

Neither type reads a battery, drives a sleep, or owns a clock. They decide and the
caller acts, which is what lets the same policy run on a microcontroller and be
checked on a server. The crate is `no_std` and allocation-free.

A state of charge measured in the field is noisy, and a governor that looks only at
the latest reading flaps back and forth across a threshold. `next_mode` takes the mode
the node is already in and applies hysteresis: the node drops a mode as soon as the
charge falls below its threshold, and climbs back only once the charge is a margin
clear of it, five points unless the plan is told otherwise. Smoothing the reading
first, with a `Smoother` or a `Median` from `pamoja-kit`, steadies it further.

## What the example does

It runs a solar node through a draining battery. One plan holds three cadences, and
the example prints the mode and the sampling interval at a healthy charge, a low
one and a nearly flat one, then asks again with the panel delivering, and once more
with a fuel gauge that did not answer.

A second plan is the first with its thresholds moved for winter, when the battery
has to carry the node through long nights. The example reads the thresholds back
from it and compares the two plans at the same charge.

Then it walks a charge that wanders around 50%, the way a fuel gauge's reading does
from one cycle to the next, through `next_mode` twice: once on a copy of the plan with
no margin, and once on the plan as it is. It prints the mode after each reading, and
the mode once the charge climbs to 56%.

The work window stays two seconds throughout. The sleep half of each duty cycle is
the plan's own interval less that window, so the two fractions weigh the same job
at the cadence the governor picked rather than at a period typed in by hand. The
last cycles are sized from what a panel harvests instead: the share of a minute the
harvest pays for, in cloud, in full sun, and from a meter that could not be read.

It proves:

- With the default thresholds, 80% charge is active, 35% is saver and 12% is
  critical, and the interval follows the mode, so a battery at 12% is asked for one
  reading an hour where a healthy one gives sixty.
- A delivering panel eases the governor off by one mode and no further, so the flat
  battery reports on the saver cadence rather than the active one. `interval_for`
  gives the cadence for that mode; the interval for a charge alone would still be
  the hourly one.
- A charge that is not a number is taken as critical, so a node that cannot tell
  what it has left samples hourly until it can.
- Moving the thresholds to 70% and 30% puts a 60% battery in saver mode, where the
  default plan keeps it active.
- With no margin, readings of 49%, 51%, 50%, 53%, 48% and 52% change the mode four
  times, since every reading at or above 50% is active again. With the default five
  point margin the node drops to saver at 49% and stays there, because none of the
  later readings reaches 55%. At 56% it is active again.
- Two seconds of work is one part in thirty at the minute cadence and one part in
  1800 at the hourly one, the sixtyfold cut in average draw the stretch buys. The
  fraction is the awake share of the whole period, so two seconds awake and 58
  asleep is one in thirty, not one in twenty-nine.
- A 12 mW harvest against a 120 mW draw pays for a tenth of a minute, 6000ms. A
  harvest above the draw clamps to the whole minute awake, and one that is not a
  number to the whole minute asleep.

Durations are a `Duration` in Rust and microseconds across the three bindings, so
the same intervals appear scaled by a million in the other three snippets.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example power" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example power</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- power" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- power</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/power.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/power.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- power" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- power</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-power` is `no_std` and allocation-free, and both types are `Copy`.
`PowerPlan::new` takes the three intervals as `Duration`s and starts with the default
thresholds; `thresholds` returns a copy with them moved, and `saver_below` and
`critical_below` read them back. `mode` and `interval` answer for a charge,
`mode_while_charging` for a charge and the panel, and `interval_for` for a mode
already chosen. `next_mode` and `next_mode_while_charging` take the mode the node is
in as well, and apply the margin `with_hysteresis` sets and `hysteresis` reads back;
`DEFAULT_HYSTERESIS` is the 0.05 a new plan starts with. A charge is an `f32` from 0.0
to 1.0. `DutyCycle::new` takes the two
halves and `DutyCycle::from_fraction` splits a period; `active`, `sleep`, `period`,
and `fraction` read one back, and `period` holds at `Duration::MAX` rather than
overflow.

<!-- snippet: examples/guides/power.rs#example -->
From [`examples/guides/power.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/power.rs):

```rust
use core::time::Duration;

use pamoja_power::{DutyCycle, PowerMode, PowerPlan};

// A solar node samples every minute while the charge is healthy, stretches to ten
// minutes to conserve, and to an hour once the battery is nearly flat.
let plan = PowerPlan::new(
    Duration::from_secs(60),
    Duration::from_secs(600),
    Duration::from_secs(3600),
);

// The default thresholds enter saver mode below 50% charge and critical below 20%.
for charge in [0.80, 0.35, 0.12] {
    let mode = plan.mode(charge);
    let every = plan.interval(charge).as_secs();
    let percent = charge * 100.0;
    println!("at {percent:.0}% charge: {mode:?}, sampling every {every}s");
}

// A panel that is delivering buys back one mode. The interval for a charge knows
// nothing of the panel, so the cadence comes from the mode the panel bought.
let charging = plan.mode_while_charging(0.12, true);
let every = plan.interval_for(charging).as_secs();
println!("at 12% charge while charging: {charging:?}, sampling every {every}s");

// A charge worked out from a fuel gauge that did not answer is not a number. The
// plan takes it as critical, so a node that cannot tell what it has left does the
// least until it can.
let unknown = f32::NAN;
let (mode, every) = (plan.mode(unknown), plan.interval(unknown).as_secs());
println!("with no reading from the gauge: {mode:?}, sampling every {every}s");

// The thresholds say how long the battery must carry the node without sun. Winter
// nights are long, so a winter plan starts saving sooner and goes critical sooner.
let winter = plan.thresholds(0.70, 0.30);
let saver = winter.saver_below() * 100.0;
let critical = winter.critical_below() * 100.0;
println!("the winter plan saves below {saver:.0}% and goes critical below {critical:.0}%");
let (cold, mild) = (winter.mode(0.60), plan.mode(0.60));
println!("at 60% charge: {cold:?} in winter, {mild:?} by default");

// A fuel gauge wanders a point or two between readings, so a charge sitting at a
// threshold would change the cadence on every cycle. `next_mode` takes the mode the
// node is in: it drops as soon as the charge falls below a threshold, and climbs
// back only once the charge is the plan's hysteresis margin clear of it.
let wandering = [0.49, 0.51, 0.50, 0.53, 0.48, 0.52];
let walk = |governor: PowerPlan| {
    let mut mode = PowerMode::Active;
    let mut modes = Vec::new();
    for charge in wandering {
        mode = governor.next_mode(mode, charge);
        modes.push(format!("{mode:?}"));
    }
    modes.join(", ")
};
let flapping = walk(plan.with_hysteresis(0.0));
println!("a charge wandering around 50% with no margin: {flapping}");
let margin = plan.hysteresis() * 100.0;
let settled = walk(plan);
println!("and with the {margin:.0} point margin: {settled}");
let back = plan.next_mode(PowerMode::Saver, 0.56);
println!("at 56% the charge has cleared the margin: {back:?}");

// The work is the same two seconds whichever mode the node is in; stretching the cycle
// is what saves the energy. The duty fraction is the proxy for average draw, so the
// hourly cadence costs a sixtieth of what the one-minute cadence does.
let awake = Duration::from_secs(2);
let healthy = DutyCycle::new(awake, plan.interval(0.80) - awake);
let flat = DutyCycle::new(awake, plan.interval(0.12) - awake);
let (healthy_duty, flat_duty) = (healthy.fraction() * 100.0, flat.fraction() * 100.0);
println!("awake {healthy_duty:.2}% of the time when healthy");
println!("awake {flat_duty:.3}% of the time when flat");

// A node that lives on its panel can stay awake for the share of the time the harvest
// pays for. Asleep it draws next to nothing, so that share is the harvest over what it
// draws awake, and the duty cycle turns it into time.
let minute = Duration::from_secs(60);
let awake_mw = 120.0;
let cloudy = DutyCycle::from_fraction(minute, 12.0 / awake_mw);
let paid = cloudy.active().as_millis();
println!("a 12 mW harvest pays for {paid}ms awake in each minute");

// The share is clamped, so a harvest above the draw keeps the node awake throughout,
// and a harvest the meter could not read keeps it asleep until one can be.
let sunny = DutyCycle::from_fraction(minute, 150.0 / awake_mw);
let unread = DutyCycle::from_fraction(minute, f32::NAN);
let awake_all = sunny.active().as_millis();
let asleep_all = unread.sleep().as_millis();
println!("a 150 mW harvest keeps it awake all {awake_all}ms");
println!("an unread harvest keeps it asleep all {asleep_all}ms");
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/power` exports `PowerPlan`, `DutyCycle`, and `PowerMode`, an
object whose values are the mode names that `mode` returns. Every duration is a whole
number of microseconds in a `number`, and a constructor or `fromFraction` throws on
one that is negative, fractional, or past `Number.MAX_SAFE_INTEGER`.
`withThresholds` and `withHysteresis` return a new plan, and `saverBelow`,
`criticalBelow`, and `hysteresis` are properties; `mode`, `modeWhileCharging`,
`nextMode`, `nextModeWhileCharging`, `intervalUs`, and `intervalForUs` are methods. A `DutyCycle` has `activeUs`, `sleepUs`, `periodUs`, and `fraction` as
properties.

<!-- snippet: bindings/node/guides/power.ts#example -->
From [`bindings/node/guides/power.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/power.ts):

```typescript
import { DutyCycle, PowerMode, PowerPlan } from '@pamoja/power'

// A solar node samples every minute while the charge is healthy, stretches to ten minutes
// to conserve, and to an hour once the battery is nearly flat. Durations cross the binding
// as microseconds.
const plan = new PowerPlan(60_000_000, 600_000_000, 3_600_000_000)

// The default thresholds enter saver mode below 50% charge and critical below 20%.
for (const charge of [0.8, 0.35, 0.12]) {
  const every = plan.intervalUs(charge) / 1_000_000
  console.log(
    `at ${(charge * 100).toFixed(0)}% charge: ${plan.mode(charge)}, sampling every ${every}s`,
  )
}

// A panel that is delivering buys back one mode. The interval for a charge knows nothing of
// the panel, so the cadence comes from the mode the panel bought.
const charging = plan.modeWhileCharging(0.12, true)
const chargingEvery = plan.intervalForUs(charging) / 1_000_000
console.log(`at 12% charge while charging: ${charging}, sampling every ${chargingEvery}s`)

// A charge worked out from a fuel gauge that did not answer is not a number. The plan takes
// it as critical, so a node that cannot tell what it has left does the least until it can.
const unknown = Number.NaN
const unknownEvery = plan.intervalUs(unknown) / 1_000_000
console.log(
  `with no reading from the gauge: ${plan.mode(unknown)}, sampling every ${unknownEvery}s`,
)

// The thresholds say how long the battery must carry the node without sun. Winter nights
// are long, so a winter plan starts saving sooner and goes critical sooner.
const winter = plan.withThresholds(0.7, 0.3)
const saver = (winter.saverBelow * 100).toFixed(0)
const critical = (winter.criticalBelow * 100).toFixed(0)
console.log(`the winter plan saves below ${saver}% and goes critical below ${critical}%`)
const cold = winter.mode(0.6)
const mild = plan.mode(0.6)
console.log(`at 60% charge: ${cold} in winter, ${mild} by default`)

// A fuel gauge wanders a point or two between readings, so a charge sitting at a threshold
// would change the cadence on every cycle. `nextMode` takes the mode the node is in: it drops
// as soon as the charge falls below a threshold, and climbs back only once the charge is the
// plan's hysteresis margin clear of it.
const wandering = [0.49, 0.51, 0.5, 0.53, 0.48, 0.52]
const walk = (governor: PowerPlan): string => {
  let mode: PowerMode = PowerMode.Active
  const modes: string[] = []
  for (const charge of wandering) {
    mode = governor.nextMode(mode, charge)
    modes.push(mode)
  }
  return modes.join(', ')
}
const flapping = walk(plan.withHysteresis(0))
console.log(`a charge wandering around 50% with no margin: ${flapping}`)
const margin = (plan.hysteresis * 100).toFixed(0)
const settled = walk(plan)
console.log(`and with the ${margin} point margin: ${settled}`)
const back = plan.nextMode(PowerMode.Saver, 0.56)
console.log(`at 56% the charge has cleared the margin: ${back}`)

// The work is the same two seconds whichever mode the node is in; stretching the cycle is
// what saves the energy. The duty fraction is the proxy for average draw, so the hourly
// cadence costs a sixtieth of what the one-minute cadence does.
const awakeUs = 2_000_000
const healthy = new DutyCycle(awakeUs, plan.intervalUs(0.8) - awakeUs)
const flat = new DutyCycle(awakeUs, plan.intervalUs(0.12) - awakeUs)
console.log(`awake ${(healthy.fraction * 100).toFixed(2)}% of the time when healthy`)
console.log(`awake ${(flat.fraction * 100).toFixed(3)}% of the time when flat`)

// A node that lives on its panel can stay awake for the share of the time the harvest pays
// for. Asleep it draws next to nothing, so that share is the harvest over what it draws
// awake, and the duty cycle turns it into time.
const minuteUs = 60_000_000
const awakeMw = 120
const cloudy = DutyCycle.fromFraction(minuteUs, 12 / awakeMw)
console.log(`a 12 mW harvest pays for ${cloudy.activeUs / 1000}ms awake in each minute`)

// The share is clamped, so a harvest above the draw keeps the node awake throughout, and a
// harvest the meter could not read keeps it asleep until one can be.
const sunny = DutyCycle.fromFraction(minuteUs, 150 / awakeMw)
const unread = DutyCycle.fromFraction(minuteUs, Number.NaN)
console.log(`a 150 mW harvest keeps it awake all ${sunny.activeUs / 1000}ms`)
console.log(`an unread harvest keeps it asleep all ${unread.sleepUs / 1000}ms`)
```
<!-- end -->

## Python

In Python, `pamoja.power` exports `PowerPlan` and `DutyCycle`, the `power_plan` and
`duty_cycle` builders, and `PowerMode`, a string enum. Durations are `int`
microseconds: a negative one raises `OverflowError` and a `float` raises `TypeError`.
A mode comes back as its name, a `str` that compares equal to its `PowerMode`
member, and `interval_for_us`, `next_mode`, and `next_mode_while_charging` raise
`PamojaError` for a name that is not a mode. `with_thresholds` and `with_hysteresis`
return a new plan, and `saver_below`, `critical_below`, and `hysteresis` are
properties. A `DutyCycle` has `active_us`, `sleep_us`, `period_us`, and `fraction`.

<!-- snippet: bindings/python/guides/power.py#example -->
From [`bindings/python/guides/power.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/power.py):

```python
from pamoja.power import DutyCycle, PowerMode, power_plan

# A solar node samples every minute while the charge is healthy, stretches to ten minutes
# to conserve, and to an hour once the battery is nearly flat. Durations cross the binding
# as microseconds.
plan = power_plan(60_000_000, 600_000_000, 3_600_000_000)

# The default thresholds enter saver mode below 50% charge and critical below 20%.
for charge in (0.80, 0.35, 0.12):
    every = plan.interval_us(charge) // 1_000_000
    print(f"at {charge * 100:.0f}% charge: {plan.mode(charge)}, sampling every {every}s")

# A panel that is delivering buys back one mode. The interval for a charge knows nothing of
# the panel, so the cadence comes from the mode the panel bought.
charging = plan.mode_while_charging(0.12, True)
every = plan.interval_for_us(charging) // 1_000_000
print(f"at 12% charge while charging: {charging}, sampling every {every}s")

# A charge worked out from a fuel gauge that did not answer is not a number. The plan takes
# it as critical, so a node that cannot tell what it has left does the least until it can.
unknown = float("nan")
every = plan.interval_us(unknown) // 1_000_000
print(f"with no reading from the gauge: {plan.mode(unknown)}, sampling every {every}s")

# The thresholds say how long the battery must carry the node without sun. Winter nights
# are long, so a winter plan starts saving sooner and goes critical sooner.
winter = plan.with_thresholds(0.70, 0.30)
saver, critical = winter.saver_below * 100, winter.critical_below * 100
print(f"the winter plan saves below {saver:.0f}% and goes critical below {critical:.0f}%")
cold, mild = winter.mode(0.60), plan.mode(0.60)
print(f"at 60% charge: {cold} in winter, {mild} by default")

# A fuel gauge wanders a point or two between readings, so a charge sitting at a threshold
# would change the cadence on every cycle. `next_mode` takes the mode the node is in: it
# drops as soon as the charge falls below a threshold, and climbs back only once the charge
# is the plan's hysteresis margin clear of it.
wandering = (0.49, 0.51, 0.50, 0.53, 0.48, 0.52)


def walk(governor):
    mode = PowerMode.ACTIVE
    modes = []
    for charge in wandering:
        mode = governor.next_mode(mode, charge)
        modes.append(mode)
    return ", ".join(modes)


flapping = walk(plan.with_hysteresis(0))
print(f"a charge wandering around 50% with no margin: {flapping}")
margin = plan.hysteresis * 100
settled = walk(plan)
print(f"and with the {margin:.0f} point margin: {settled}")
back = plan.next_mode(PowerMode.SAVER, 0.56)
print(f"at 56% the charge has cleared the margin: {back}")

# The work is the same two seconds whichever mode the node is in; stretching the cycle is
# what saves the energy. The duty fraction is the proxy for average draw, so the hourly
# cadence costs a sixtieth of what the one-minute cadence does.
awake_us = 2_000_000
healthy = DutyCycle(awake_us, plan.interval_us(0.80) - awake_us)
flat = DutyCycle(awake_us, plan.interval_us(0.12) - awake_us)
print(f"awake {healthy.fraction * 100:.2f}% of the time when healthy")
print(f"awake {flat.fraction * 100:.3f}% of the time when flat")

# A node that lives on its panel can stay awake for the share of the time the harvest pays
# for. Asleep it draws next to nothing, so that share is the harvest over what it draws
# awake, and the duty cycle turns it into time.
minute_us = 60_000_000
awake_mw = 120
cloudy = DutyCycle.from_fraction(minute_us, 12 / awake_mw)
print(f"a 12 mW harvest pays for {cloudy.active_us // 1000}ms awake in each minute")

# The share is clamped, so a harvest above the draw keeps the node awake throughout, and a
# harvest the meter could not read keeps it asleep until one can be.
sunny = DutyCycle.from_fraction(minute_us, 150 / awake_mw)
unread = DutyCycle.from_fraction(minute_us, float("nan"))
print(f"a 150 mW harvest keeps it awake all {sunny.active_us // 1000}ms")
print(f"an unread harvest keeps it asleep all {unread.sleep_us // 1000}ms")
```
<!-- end -->

## C#

In C#, `Pamoja.Power` holds `PowerPlan` and `DutyCycle`, both readonly record
structs, and the `PowerMode` enum. Durations are `ulong` microseconds.
`PowerPlan.Create` starts with the default thresholds and margin, which the record
carries as `SaverBelow`, `CriticalBelow`, and `Hysteresis`, and `WithThresholds` and
`WithHysteresis` return a new plan; `PowerPlan.DefaultHysteresis` is the 0.05 it starts
with. `Mode`, `ModeWhileCharging`, `NextMode`, `NextModeWhileCharging`, `IntervalUs`,
and `IntervalForUs` are methods. A `DutyCycle` is
its `ActiveUs` and `SleepUs`, with `PeriodUs` and `Fraction` worked out from them,
and `DutyCycle.FromFraction` splits a period.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/PowerGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/PowerGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/PowerGuide.cs):

```csharp
// A solar node samples every minute while the charge is healthy, stretches to ten
// minutes to conserve, and to an hour once the battery is nearly flat. Durations
// cross the binding as microseconds.
PowerPlan plan = PowerPlan.Create(60_000_000, 600_000_000, 3_600_000_000);

// The default thresholds enter saver mode below 50% charge and critical below 20%.
foreach (float charge in new[] { 0.80f, 0.35f, 0.12f })
{
    ulong every = plan.IntervalUs(charge) / 1_000_000;
    Console.WriteLine(Invariant(
        $"at {charge * 100:F0}% charge: {plan.Mode(charge)}, sampling every {every}s"));
}

// A panel that is delivering buys back one mode. The interval for a charge knows
// nothing of the panel, so the cadence comes from the mode the panel bought.
PowerMode charging = plan.ModeWhileCharging(0.12f, true);
ulong chargingEvery = plan.IntervalForUs(charging) / 1_000_000;
Console.WriteLine(
    $"at 12% charge while charging: {charging}, sampling every {chargingEvery}s");

// A charge worked out from a fuel gauge that did not answer is not a number. The
// plan takes it as critical, so a node that cannot tell what it has left does the
// least until it can.
float unknown = float.NaN;
ulong unknownEvery = plan.IntervalUs(unknown) / 1_000_000;
Console.WriteLine(
    $"with no reading from the gauge: {plan.Mode(unknown)}, sampling every {unknownEvery}s");

// The thresholds say how long the battery must carry the node without sun. Winter
// nights are long, so a winter plan starts saving sooner and goes critical sooner.
PowerPlan winter = plan.WithThresholds(0.70f, 0.30f);
Console.WriteLine(Invariant(
    $"the winter plan saves below {winter.SaverBelow * 100:F0}% and goes critical below {winter.CriticalBelow * 100:F0}%"));
PowerMode cold = winter.Mode(0.60f);
PowerMode mild = plan.Mode(0.60f);
Console.WriteLine($"at 60% charge: {cold} in winter, {mild} by default");

// A fuel gauge wanders a point or two between readings, so a charge sitting at a
// threshold would change the cadence on every cycle. NextMode takes the mode the
// node is in: it drops as soon as the charge falls below a threshold, and climbs
// back only once the charge is the plan's hysteresis margin clear of it.
float[] wandering = { 0.49f, 0.51f, 0.50f, 0.53f, 0.48f, 0.52f };
string Walk(PowerPlan governor)
{
    PowerMode mode = PowerMode.Active;
    var modes = new List<string>();
    foreach (float charge in wandering)
    {
        mode = governor.NextMode(mode, charge);
        modes.Add(mode.ToString());
    }
    return string.Join(", ", modes);
}
string flapping = Walk(plan.WithHysteresis(0f));
Console.WriteLine($"a charge wandering around 50% with no margin: {flapping}");
float margin = plan.Hysteresis * 100;
string settled = Walk(plan);
Console.WriteLine(Invariant($"and with the {margin:F0} point margin: {settled}"));
PowerMode back = plan.NextMode(PowerMode.Saver, 0.56f);
Console.WriteLine($"at 56% the charge has cleared the margin: {back}");

// The work is the same two seconds whichever mode the node is in; stretching the
// cycle is what saves the energy. The duty fraction is the proxy for average draw,
// so the hourly cadence costs a sixtieth of what the one-minute cadence does.
const ulong AwakeUs = 2_000_000;
var healthy = new DutyCycle(AwakeUs, plan.IntervalUs(0.80f) - AwakeUs);
var flat = new DutyCycle(AwakeUs, plan.IntervalUs(0.12f) - AwakeUs);
Console.WriteLine(Invariant($"awake {healthy.Fraction * 100:F2}% of the time when healthy"));
Console.WriteLine(Invariant($"awake {flat.Fraction * 100:F3}% of the time when flat"));

// A node that lives on its panel can stay awake for the share of the time the
// harvest pays for. Asleep it draws next to nothing, so that share is the harvest
// over what it draws awake, and the duty cycle turns it into time.
const ulong MinuteUs = 60_000_000;
const float AwakeMw = 120f;
DutyCycle cloudy = DutyCycle.FromFraction(MinuteUs, 12f / AwakeMw);
Console.WriteLine($"a 12 mW harvest pays for {cloudy.ActiveUs / 1000}ms awake in each minute");

// The share is clamped, so a harvest above the draw keeps the node awake
// throughout, and a harvest the meter could not read keeps it asleep until one
// can be.
DutyCycle sunny = DutyCycle.FromFraction(MinuteUs, 150f / AwakeMw);
DutyCycle unread = DutyCycle.FromFraction(MinuteUs, float.NaN);
Console.WriteLine($"a 150 mW harvest keeps it awake all {sunny.ActiveUs / 1000}ms");
Console.WriteLine($"an unread harvest keeps it asleep all {unread.SleepUs / 1000}ms");
```
<!-- end -->

## Values at a glance

**The modes,** from the most work to the least. A threshold is the lower bound of
the mode above it, so a battery at exactly 50% is still active:

| Mode | Entered when the charge is | Interval in the example |
| --- | --- | --- |
| Active | at or above the saver threshold, 0.5 by default | 60 s |
| Saver | below the saver threshold, at or above the critical one | 600 s |
| Critical | below the critical threshold, 0.2 by default, or not a number | 3600 s |

**Moving with `next_mode`,** a node falls at a threshold and climbs back only past
the threshold plus the margin, 0.05 by default:

| From | To | When the charge |
| --- | --- | --- |
| Active | Saver | falls below the saver threshold, 0.5 |
| Saver | Critical | falls below the critical threshold, 0.2 |
| Critical | Saver | reaches the critical threshold plus the margin, 0.25 |
| Saver | Active | reaches the saver threshold plus the margin, 0.55 |
| any mode | the mode `mode` gives | with a margin of 0, at every charge |

A climb of two modes at once happens when the charge clears both bars, so a critical
node at 0.9 goes straight to active. A charge that is not a number drops the node to
critical, as it does for `mode`. `next_mode_while_charging` eases the result one step
while the panel delivers, and counts that step as a climb, so near the critical
threshold it too waits for the margin.

**While the panel is delivering,** the mode moves one step toward full duty:

| Mode by charge alone | Mode while charging |
| --- | --- |
| Critical | Saver |
| Saver | Active |
| Active | Active |

**A duty cycle's parts:**

| Part | What it is |
| --- | --- |
| active | how long the node is awake each period |
| sleep | how long it is asleep each period |
| period | active plus sleep |
| fraction | active over period, from 0 to 1, and 0 for a zero-length period |

**What `from_fraction` makes of the fraction it is given:**

| Fraction | Awake | Asleep |
| --- | --- | --- |
| between 0 and 1 | that share of the period | the rest |
| 1 or more | the whole period | none |
| 0 or less | none | the whole period |
| not a number | none | the whole period |

Across the bindings the awake time is rounded down to a whole microsecond and the
rest of the period is asleep, so the two always add up to the period and the awake
share never runs over the budget.

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| make a plan | `PowerPlan::new(active, saver, critical)` |
| move the thresholds | `thresholds(saver_below, critical_below)`; `saver_below()`, `critical_below()` |
| set the margin | `with_hysteresis(margin)`; `hysteresis()`, `DEFAULT_HYSTERESIS` |
| choose a mode | `mode(soc)`, `mode_while_charging(soc, charging)` |
| move from the mode it is in | `next_mode(current, soc)`, `next_mode_while_charging(current, soc, charging)` |
| look up an interval | `interval(soc)`, `interval_for(mode)` |
| make a duty cycle | `DutyCycle::new(active, sleep)`, `DutyCycle::from_fraction(period, fraction)` |
| read a duty cycle | `active()`, `sleep()`, `period()`, `fraction()` |

### TypeScript

| To | Call |
| --- | --- |
| make a plan | `new PowerPlan(activeUs, saverUs, criticalUs)` |
| move the thresholds | `withThresholds(saverBelow, criticalBelow)`; `saverBelow`, `criticalBelow` |
| set the margin | `withHysteresis(margin)`; `hysteresis` |
| choose a mode | `mode(soc)`, `modeWhileCharging(soc, charging)` |
| move from the mode it is in | `nextMode(current, soc)`, `nextModeWhileCharging(current, soc, charging)` |
| look up an interval | `intervalUs(soc)`, `intervalForUs(mode)` |
| make a duty cycle | `new DutyCycle(activeUs, sleepUs)`, `DutyCycle.fromFraction(periodUs, fraction)` |
| read a duty cycle | `activeUs`, `sleepUs`, `periodUs`, `fraction` |

### Python

| To | Call |
| --- | --- |
| make a plan | `power_plan(active_us, saver_us, critical_us)` |
| move the thresholds | `with_thresholds(saver_below, critical_below)`; `saver_below`, `critical_below` |
| set the margin | `with_hysteresis(margin)`; `hysteresis` |
| choose a mode | `mode(soc)`, `mode_while_charging(soc, charging)` |
| move from the mode it is in | `next_mode(current, soc)`, `next_mode_while_charging(current, soc, charging)` |
| look up an interval | `interval_us(soc)`, `interval_for_us(mode)` |
| make a duty cycle | `duty_cycle(active_us, sleep_us)`, `DutyCycle.from_fraction(period_us, fraction)` |
| read a duty cycle | `active_us`, `sleep_us`, `period_us`, `fraction` |

### C#

| To | Call |
| --- | --- |
| make a plan | `PowerPlan.Create(activeUs, saverUs, criticalUs)` |
| move the thresholds | `WithThresholds(saverBelow, criticalBelow)`; `SaverBelow`, `CriticalBelow` |
| set the margin | `WithHysteresis(margin)`; `Hysteresis`, `PowerPlan.DefaultHysteresis` |
| choose a mode | `Mode(soc)`, `ModeWhileCharging(soc, charging)` |
| move from the mode it is in | `NextMode(current, soc)`, `NextModeWhileCharging(current, soc, charging)` |
| look up an interval | `IntervalUs(soc)`, `IntervalForUs(mode)` |
| make a duty cycle | `new DutyCycle(activeUs, sleepUs)`, `DutyCycle.FromFraction(periodUs, fraction)` |
| read a duty cycle | `ActiveUs`, `SleepUs`, `PeriodUs`, `Fraction` |

<!-- languages end -->

## When it goes wrong

Neither type refuses a charge or a fraction. Each has an answer for every value, so
a mistake shows up as a node that behaves oddly rather than one that stops. The
mistakes that cost an afternoon:

- **The node never leaves active mode.** A charge is a fraction from 0 to 1, not a
  percentage. A gauge that reports 35 for 35% reads as more than full; divide by 100
  before the plan sees it.
- **The cadence stays slow while the panel is charging.** `interval` looks at the
  charge alone. Take the mode from `mode_while_charging` and ask `interval_for` for
  its cadence, as the example does.
- **The mode flips back and forth.** The node asks `mode` for each reading, which
  looks at the charge alone, so a charge hovering at 50% crosses the saver threshold
  on every sample. Keep the mode the node is in and pass it to `next_mode`, as the
  example does. If it still flips, the margin is smaller than the gauge's noise:
  widen it with `with_hysteresis`, or smooth the reading first with a `Smoother` or a
  `Median` from `pamoja-kit`.
- **The node never climbs back to active.** The saver threshold plus the margin is
  above 1, so no charge reaches it: a saver threshold of 0.98 with the default margin
  asks for 103%. A profile refuses such a schedule; a plan built by hand does not, so
  keep the two under 1.
- **Saver mode never comes.** The thresholds are the wrong way round: a critical
  threshold above the saver one takes the node straight from active to critical.
  Keep the critical threshold below the saver one.
- **Readings drift later every cycle.** The interval runs from one start of work to
  the next. A node that sleeps the whole interval after its work adds the work to
  every cycle, so a two-second job on a minute's cadence reports every 62 seconds.
  Sleep for the interval less the work, as the example's duty cycles do.
- **The sleep comes out absurdly long, or the program stops.** The work took longer
  than the interval, and the subtraction went below zero. Rust's `Duration` panics,
  TypeScript's constructors throw and Python's raise `OverflowError`, but C#'s
  `ulong` wraps to about 585,000 years unless the code is `checked`. Compare the two
  before subtracting.
- **The node sleeps through every period.** The fraction handed to `from_fraction`
  was zero, negative, or not a number. A budget worked out from an energy meter that
  did not answer is not a number, and a harvest below the sleep draw leaves nothing
  to spend.
- **A TypeScript constructor throws.** Durations are whole microseconds:
  `activeUs must be a whole number of microseconds, not 1.5`. Multiply seconds by a
  million and round before building the plan.

## Where next

<!-- table: next power -->
- [LoRa airtime and range](lora.md): Time-on-air, duty-cycle off-time, the regional channel plans a LoRa node must keep to, and the link budget that sets its range.
- [Telemetry](telemetry.md): Observability that ships only what is worth the bytes as link cost rises, while counting everything.
- [Store and forward](sync.md): Offline-first queues in memory or on disk, bounded or not, the on-disk one surviving power loss, and the drain that forwards them in order when a link returns.
- Also in Trust and operation: [Audit log](audit.md), [Secured session](session.md), [Signed updates](update.md).
<!-- end -->

## Reference

<!-- table: reference power -->
- Rust: [`pamoja-power`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-power)
- TypeScript: [`@pamoja/power`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-power)
- Python: [`pamoja.power`](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-power)
- C#: [`Pamoja.Power`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-power)
<!-- end -->
