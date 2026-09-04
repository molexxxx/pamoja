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
the example asks it what to do at a healthy charge, a low one, and a nearly flat
one, then asks again with the panel delivering. The work window stays two seconds
throughout, so the duty fraction shows what stretching the cycle actually buys.

It proves:

- The default thresholds are the lower bound of each mode: saver below 50% charge,
  critical below 20%.
- The interval follows the mode, so a battery at 12% is asked for one reading an
  hour rather than sixty.
- A delivering panel eases the governor off by exactly one mode, and no further.
- The same two seconds of work is one part in thirty at the minute cadence and one
  part in 1800 at the hourly one, which is the sixtyfold cut in average draw the
  stretch was for.
- Splitting a period by a fraction gives the same schedule as naming the two
  durations.

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
assert_eq!(plan.mode(0.80), PowerMode::Active);
assert_eq!(plan.interval(0.80), Duration::from_secs(60));
assert_eq!(plan.mode(0.35), PowerMode::Saver);
assert_eq!(plan.interval(0.35), Duration::from_secs(600));
assert_eq!(plan.mode(0.12), PowerMode::Critical);
assert_eq!(plan.interval(0.12), Duration::from_secs(3600));

// A panel that is delivering buys back one mode, so the same flat battery keeps
// reporting on the ten-minute saver cadence while the sun is on it.
assert_eq!(plan.mode_while_charging(0.12, true), PowerMode::Saver);

// The work is the same two seconds whichever mode the node is in; stretching the
// cycle is what saves the energy. The duty fraction is the proxy for average draw,
// so the hourly cadence costs a sixtieth of what the one-minute cadence does.
let awake = Duration::from_secs(2);
let healthy = DutyCycle::new(awake, plan.interval(0.80) - awake);
let flat = DutyCycle::new(awake, plan.interval(0.12) - awake);
assert!((healthy.fraction() - 2.0 / 60.0).abs() < 1e-6);
assert!((flat.fraction() - 2.0 / 3600.0).abs() < 1e-6);

// Stating the budget as a fraction instead gives the awake time directly.
let quarter = DutyCycle::from_fraction(Duration::from_secs(1), 0.25);
assert_eq!(quarter.active(), Duration::from_millis(250));
assert_eq!(quarter.sleep(), Duration::from_millis(750));
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/power.ts#example -->
From [`bindings/node/guides/power.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/power.ts):

```typescript
import assert from 'node:assert/strict'

import { DutyCycle, PowerMode, PowerPlan } from '@pamoja/power'

// A solar node samples every minute while the charge is healthy, stretches to ten
// minutes to conserve, and to an hour once the battery is nearly flat. Durations cross
// the binding as microseconds.
const plan = new PowerPlan(60_000_000, 600_000_000, 3_600_000_000)

// The default thresholds enter saver mode below 50% charge and critical below 20%.
assert.equal(plan.mode(0.8), PowerMode.Active)
assert.equal(plan.intervalUs(0.8), 60_000_000)
assert.equal(plan.mode(0.35), PowerMode.Saver)
assert.equal(plan.intervalUs(0.35), 600_000_000)
assert.equal(plan.mode(0.12), PowerMode.Critical)
assert.equal(plan.intervalUs(0.12), 3_600_000_000)

// A panel that is delivering buys back one mode, so the same flat battery keeps
// reporting on the ten-minute saver cadence while the sun is on it.
assert.equal(plan.modeWhileCharging(0.12, true), PowerMode.Saver)

// The work is the same two seconds whichever mode the node is in; stretching the cycle
// is what saves the energy. The duty fraction is the proxy for average draw, so the
// hourly cadence costs a sixtieth of what the one-minute cadence does.
const awakeUs = 2_000_000
const healthy = new DutyCycle(awakeUs, plan.intervalUs(0.8) - awakeUs)
const flat = new DutyCycle(awakeUs, plan.intervalUs(0.12) - awakeUs)
assert.ok(Math.abs(healthy.fraction - 2 / 60) < 1e-6)
assert.ok(Math.abs(flat.fraction - 2 / 3600) < 1e-6)

// Stating the budget as a fraction instead gives the awake time directly.
const quarter = DutyCycle.fromFraction(1_000_000, 0.25)
assert.equal(quarter.activeUs, 250_000)
assert.equal(quarter.sleepUs, 750_000)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/power.py#example -->
From [`bindings/python/guides/power.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/power.py):

```python
from pamoja.power import DutyCycle, PowerMode, power_plan

# A solar node samples every minute while the charge is healthy, stretches to ten
# minutes to conserve, and to an hour once the battery is nearly flat. Durations cross
# the binding as microseconds.
plan = power_plan(60_000_000, 600_000_000, 3_600_000_000)

# The default thresholds enter saver mode below 50% charge and critical below 20%.
assert plan.mode(0.80) == PowerMode.ACTIVE
assert plan.interval_us(0.80) == 60_000_000
assert plan.mode(0.35) == PowerMode.SAVER
assert plan.interval_us(0.35) == 600_000_000
assert plan.mode(0.12) == PowerMode.CRITICAL
assert plan.interval_us(0.12) == 3_600_000_000

# A panel that is delivering buys back one mode, so the same flat battery keeps
# reporting on the ten-minute saver cadence while the sun is on it.
assert plan.mode_while_charging(0.12, True) == PowerMode.SAVER

# The work is the same two seconds whichever mode the node is in; stretching the cycle
# is what saves the energy. The duty fraction is the proxy for average draw, so the
# hourly cadence costs a sixtieth of what the one-minute cadence does.
awake_us = 2_000_000
healthy = DutyCycle(awake_us, plan.interval_us(0.80) - awake_us)
flat = DutyCycle(awake_us, plan.interval_us(0.12) - awake_us)
assert abs(healthy.fraction - 2 / 60) < 1e-6
assert abs(flat.fraction - 2 / 3600) < 1e-6

# Stating the budget as a fraction instead gives the awake time directly.
quarter = DutyCycle.from_fraction(1_000_000, 0.25)
assert quarter.active_us == 250_000
assert quarter.sleep_us == 750_000
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/PowerGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/PowerGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/PowerGuide.cs):

```csharp
// A solar node samples every minute while the charge is healthy, stretches to
// ten minutes to conserve, and to an hour once the battery is nearly flat.
// Durations cross the binding as microseconds.
PowerPlan plan = PowerPlan.Create(60_000_000, 600_000_000, 3_600_000_000);

// The default thresholds enter saver mode below 50% charge and critical below 20%.
Expect(plan.Mode(0.80f) == PowerMode.Active, "a healthy charge runs at full cadence");
Expect(plan.IntervalUs(0.80f) == 60_000_000, "which is a reading every minute");
Expect(plan.Mode(0.35f) == PowerMode.Saver, "a third of a charge conserves");
Expect(plan.IntervalUs(0.35f) == 600_000_000, "at ten minutes between readings");
Expect(plan.Mode(0.12f) == PowerMode.Critical, "and a nearly flat one survives");
Expect(plan.IntervalUs(0.12f) == 3_600_000_000, "at one reading an hour");

// A panel that is delivering buys back one mode, so the same flat battery keeps
// reporting on the ten-minute saver cadence while the sun is on it.
Expect(
    plan.ModeWhileCharging(0.12f, true) == PowerMode.Saver,
    "incoming charge eases the governor off by one mode");

// The work is the same two seconds whichever mode the node is in; stretching the
// cycle is what saves the energy. The duty fraction is the proxy for average
// draw, so the hourly cadence costs a sixtieth of what the one-minute one does.
const ulong awakeUs = 2_000_000;
DutyCycle healthy = new(awakeUs, plan.IntervalUs(0.80f) - awakeUs);
DutyCycle flat = new(awakeUs, plan.IntervalUs(0.12f) - awakeUs);
Expect(Math.Abs(healthy.Fraction - (2.0f / 60.0f)) < 1e-6f, "one part in thirty awake");
Expect(Math.Abs(flat.Fraction - (2.0f / 3600.0f)) < 1e-6f, "one part in 1800 awake");

// Stating the budget as a fraction instead gives the awake time directly.
DutyCycle quarter = DutyCycle.FromFraction(1_000_000, 0.25f);
Expect(quarter.ActiveUs == 250_000, "a quarter of a second of work");
Expect(quarter.SleepUs == 750_000, "and three quarters asleep");
```
<!-- end -->

## Reference

<!-- table: reference power -->
- Rust: [`pamoja-power`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_power/index.html)
- TypeScript: [`@pamoja/power`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_power.html)
- Python: [`pamoja.power`](https://pamoja.molex.cloud/docs/reference/python/pamoja/power.html)
- C#: [`Pamoja.Power`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Power.html)
<!-- end -->
