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

A state of charge measured in the field is noisy, and a governor fed a raw reading
will flap back and forth across a threshold. Smooth it first, with a `Smoother` or a
`Median` from `pamoja-kit`, and give the governor the filtered value.

## What the example does

It runs a solar node through a draining battery. One plan holds three cadences, and
the example prints the mode and the sampling interval at a healthy charge, a low
one and a nearly flat one, then asks again with the panel delivering.

The work window stays two seconds throughout. The sleep half of each duty cycle is
the plan's own interval less that window, so the two fractions weigh the same job
at the cadence the governor picked rather than at a period typed in by hand. A last
cycle states the budget as a fraction of a second instead of as two durations.

It proves:

- With the default thresholds, 80% charge is active, 35% is saver and 12% is
  critical.
- The interval follows the mode, so a battery at 12% is asked for one reading an
  hour where a healthy one gives sixty.
- A delivering panel eases the governor off by one mode and no further, so the flat
  battery reports on the saver cadence rather than the active one.
- Two seconds of work is one part in thirty at the minute cadence and one part in
  1800 at the hourly one, the sixtyfold cut in average draw the stretch buys.
- The fraction is the awake share of the whole period, so two seconds awake and 58
  asleep is one in thirty, not one in twenty-nine.
- `from_fraction` divides the period it is given, so a quarter-duty second is 250ms
  awake and 750ms asleep.

Durations are a `Duration` in Rust and microseconds across the three bindings, so
the same intervals appear scaled by a million in the other three snippets.

## Rust

<!-- snippet: examples/tests/guides/power.rs#example -->
From [`examples/tests/guides/power.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/power.rs):

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

// A panel that is delivering buys back one mode, so the same flat battery keeps
// reporting on the ten-minute saver cadence while the sun is on it.
let charging = plan.mode_while_charging(0.12, true);
println!("the same flat battery, while charging: {charging:?}");

// The work is the same two seconds whichever mode the node is in; stretching the cycle
// is what saves the energy. The duty fraction is the proxy for average draw, so the
// hourly cadence costs a sixtieth of what the one-minute cadence does.
let awake = Duration::from_secs(2);
let healthy = DutyCycle::new(awake, plan.interval(0.80) - awake);
let flat = DutyCycle::new(awake, plan.interval(0.12) - awake);
let (healthy_duty, flat_duty) = (healthy.fraction() * 100.0, flat.fraction() * 100.0);
println!("awake {healthy_duty:.2}% of the time when healthy");
println!("awake {flat_duty:.3}% of the time when flat");

// Stating the budget as a fraction instead gives the awake time directly.
let quarter = DutyCycle::from_fraction(Duration::from_secs(1), 0.25);
println!("a quarter-duty second is {:?} awake", quarter.active());
```
<!-- end -->

## TypeScript

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
  console.log(`at ${(charge * 100).toFixed(0)}% charge: ${plan.mode(charge)}, every ${every}s`)
}

// A panel that is delivering buys back one mode, so the same flat battery keeps reporting
// on the ten-minute saver cadence while the sun is on it.
const charging = plan.modeWhileCharging(0.12, true)
console.log(`the same flat battery, while charging: ${charging}`)

// The work is the same two seconds whichever mode the node is in; stretching the cycle is
// what saves the energy. The duty fraction is the proxy for average draw, so the hourly
// cadence costs a sixtieth of what the one-minute cadence does.
const awakeUs = 2_000_000
const healthy = new DutyCycle(awakeUs, plan.intervalUs(0.8) - awakeUs)
const flat = new DutyCycle(awakeUs, plan.intervalUs(0.12) - awakeUs)
console.log(`awake ${(healthy.fraction * 100).toFixed(2)}% of the time when healthy`)
console.log(`awake ${(flat.fraction * 100).toFixed(3)}% of the time when flat`)

// Stating the budget as a fraction instead gives the awake time directly.
const quarter = DutyCycle.fromFraction(1_000_000, 0.25)
console.log(`a quarter-duty second is ${quarter.activeUs / 1000}ms awake`)
```
<!-- end -->

## Python

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

# A panel that is delivering buys back one mode, so the same flat battery keeps reporting
# on the ten-minute saver cadence while the sun is on it.
charging = plan.mode_while_charging(0.12, True)
print(f"the same flat battery, while charging: {charging}")

# The work is the same two seconds whichever mode the node is in; stretching the cycle is
# what saves the energy. The duty fraction is the proxy for average draw, so the hourly
# cadence costs a sixtieth of what the one-minute cadence does.
awake_us = 2_000_000
healthy = DutyCycle(awake_us, plan.interval_us(0.80) - awake_us)
flat = DutyCycle(awake_us, plan.interval_us(0.12) - awake_us)
print(f"awake {healthy.fraction * 100:.2f}% of the time when healthy")
print(f"awake {flat.fraction * 100:.3f}% of the time when flat")

# Stating the budget as a fraction instead gives the awake time directly.
quarter = DutyCycle.from_fraction(1_000_000, 0.25)
print(f"a quarter-duty second is {quarter.active_us / 1000:.0f}ms awake")
```
<!-- end -->

## C#

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
    Console.WriteLine(
        $"at {charge * 100:F0}% charge: {plan.Mode(charge)}, sampling every {every}s");
}

// A panel that is delivering buys back one mode, so the same flat battery keeps
// reporting on the ten-minute saver cadence while the sun is on it.
PowerMode charging = plan.ModeWhileCharging(0.12f, true);
Console.WriteLine($"the same flat battery, while charging: {charging}");

// The work is the same two seconds whichever mode the node is in; stretching the
// cycle is what saves the energy. The duty fraction is the proxy for average draw,
// so the hourly cadence costs a sixtieth of what the one-minute cadence does.
const ulong AwakeUs = 2_000_000;
var healthy = new DutyCycle(AwakeUs, plan.IntervalUs(0.80f) - AwakeUs);
var flat = new DutyCycle(AwakeUs, plan.IntervalUs(0.12f) - AwakeUs);
Console.WriteLine($"awake {healthy.Fraction * 100:F2}% of the time when healthy");
Console.WriteLine($"awake {flat.Fraction * 100:F3}% of the time when flat");

// Stating the budget as a fraction instead gives the awake time directly.
DutyCycle quarter = DutyCycle.FromFraction(1_000_000, 0.25f);
Console.WriteLine($"a quarter-duty second is {quarter.ActiveUs / 1000}ms awake");
```
<!-- end -->

## Reference

<!-- table: reference power -->
- Rust: [`pamoja-power`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html)
- TypeScript: [`@pamoja/power`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html)
- Python: [`pamoja.power`](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html)
- C#: [`Pamoja.Power`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html)
<!-- end -->
