# Helpers

Most of what a field node runs sits between the sensor and the actuator: turning
counts into units, discarding a bad sample, deciding whether to switch something
on. `pamoja-kit` is that layer. The helpers are named for the goal rather than
the technique, with the algorithm one level down: a `Smoother` is an exponential
moving average, a `Thermostat` is a bang-bang controller with hysteresis, a
`Depletion` is a projected countdown. They are synchronous and allocation-free, so
the same code runs on a gateway and on a microcontroller.

The rest of the set has the same shape. `Pid` and `Ramp` drive a continuous output,
`Debounce` cleans up a noisy contact, `Window` and `Trend` summarise recent
readings, `Surge` and `Anomaly` catch a reading that is moving or sitting where it
should not be, `Geofence` says where a tracked thing is, and the kinematics cover a
wheeled chassis and a jointed arm.

## What the example does

It reads a tank level off a 4-20 mA current loop. A two-point calibration converts
the loop current into a percentage, a median window discards a momentary dropout,
and a hysteresis controller decides whether the refill pump runs.

The calibration is built from the two ends of the loop, 4 mA empty and 20 mA full,
rather than from a slope and an offset worked out by hand, so a reader sees the
currents the loop is specified by. The level the pump acts on is not typed in
either; it is the filtered, calibrated reading carried down from the stage above.

It proves:

- 12 mA reads 50% and 4 mA reads 0%, because the span starts at 4 mA and not at
  zero; a map scaled from zero would put mid-scale at 60% and still be
  self-consistent.
- 0 mA reads -25%, off the bottom of the scale, which is what separates a broken
  loop from an empty tank.
- One dropout among five samples leaves the filtered level at 50%, where a mean
  over the same window would be dragged down by it.
- At the setpoint the pump stays off, and it starts only once the level falls below
  the deadband.
- Once running it keeps running at a level back inside the deadband, and stops only
  above the top of it.

The windowed helpers keep the most recent readings and nothing older. In Rust the
window length is a const generic chosen at the call site; the bindings fix it at 32
readings. Five readings fit in either window, so the same samples give the same
median in all four languages.

## Rust

<!-- snippet: examples/tests/guides/kit.rs#example -->
From [`examples/tests/guides/kit.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/kit.rs):

```rust
use pamoja_kit::{Calibration, Median, Thermostat};

// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is
// full, so the span is 16 mA and mid-scale is 12 mA, not 10.
let level = Calibration::two_point(4.0, 0.0, 20.0, 100.0);
let (mid, empty) = (level.apply(12.0), level.apply(4.0));
println!("12 mA is {mid}% full, 4 mA is {empty}%");

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
// scale rather than an empty tank.
let broken = level.apply(0.0);
println!("a dead loop reads {broken}%, which is not a level at all");

// A median window drops that sample outright, where an average would blend a quarter
// of the range into every reading after it.
let mut filtered = Median::<5>::new();
let mut percent = 0.0;
for milliamps in [12.0, 12.0, 0.0, 12.0, 12.0] {
    percent = level.apply(filtered.update(milliamps));
}
println!("through the dropout, the level held at {percent}%");

// A refill pump runs when the level falls below the deadband, which is the direction
// `heating` names; nothing about it is specific to temperature. The deadband stops a
// level sitting on the threshold from chattering the contactor.
let mut pump = Thermostat::heating(50.0, 10.0);
for reading in [percent, 38.0, 45.0, 62.0] {
    let running = if pump.update(reading) { "on" } else { "off" };
    println!("at {reading}% the pump is {running}");
}
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/kit.ts#example -->
From [`bindings/node/guides/kit.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/kit.ts):

```typescript
import { Calibration, Median, Thermostat } from '@pamoja/kit'

// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is full,
// so the span is 16 mA and mid-scale is 12 mA, not 10.
const level = Calibration.twoPoint(4, 0, 20, 100)
console.log(`12 mA is ${level.apply(12)}% full, 4 mA is ${level.apply(4)}%`)

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
// scale rather than an empty tank.
const broken = level.apply(0)
console.log(`a dead loop reads ${broken}%, which is not a level at all`)

// A median window drops that sample outright, where an average would blend a quarter of
// the range into every reading after it.
const filtered = new Median()
let percent = 0
for (const milliamps of [12, 12, 0, 12, 12]) {
  percent = level.apply(filtered.update(milliamps))
}
console.log(`through the dropout, the level held at ${percent}%`)

// A refill pump runs when the level falls below the deadband, which is the direction
// heating names; nothing about it is specific to temperature. The deadband stops a level
// sitting on the threshold from chattering the contactor.
const pump = Thermostat.heating(50, 10)
for (const reading of [percent, 38, 45, 62]) {
  console.log(`at ${reading}% the pump is ${pump.update(reading) ? 'on' : 'off'}`)
}
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/kit.py#example -->
From [`bindings/python/guides/kit.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/kit.py):

```python
from pamoja.kit import Calibration, Median, Thermostat

# A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is full,
# so the span is 16 mA and mid-scale is 12 mA, not 10.
level = Calibration.two_point(4.0, 0.0, 20.0, 100.0)
print(f"12 mA is {level.apply(12.0)}% full, 4 mA is {level.apply(4.0)}%")

# The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
# scale rather than an empty tank.
broken = level.apply(0.0)
print(f"a dead loop reads {broken}%, which is not a level at all")

# A median window drops that sample outright, where an average would blend a quarter of
# the range into every reading after it.
filtered = Median()
percent = 0.0
for milliamps in (12.0, 12.0, 0.0, 12.0, 12.0):
    percent = level.apply(filtered.update(milliamps))
print(f"through the dropout, the level held at {percent}%")

# A refill pump runs when the level falls below the deadband, which is the direction
# heating names; nothing about it is specific to temperature. The deadband stops a level
# sitting on the threshold from chattering the contactor.
pump = Thermostat.heating(50.0, 10.0)
for reading in (percent, 38.0, 45.0, 62.0):
    running = "on" if pump.update(reading) else "off"
    print(f"at {reading}% the pump is {running}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs):

```csharp
// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA
// is full, so the span is 16 mA and mid-scale is 12 mA, not 10.
Calibration level = Calibration.TwoPoint(4.0f, 0.0f, 20.0f, 100.0f);
Console.WriteLine($"12 mA is {level.Apply(12.0f)}% full, 4 mA is {level.Apply(4.0f)}%");

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of
// the scale rather than an empty tank.
float broken = level.Apply(0.0f);
Console.WriteLine($"a dead loop reads {broken}%, which is not a level at all");

// A median window drops that sample outright, where an average would blend a
// quarter of the range into every reading after it.
using var filtered = new Median();
float percent = 0.0f;
foreach (float milliamps in new[] { 12.0f, 12.0f, 0.0f, 12.0f, 12.0f })
{
    percent = level.Apply(filtered.Update(milliamps));
}

Console.WriteLine($"through the dropout, the level held at {percent}%");

// A refill pump runs when the level falls below the deadband, which is the
// direction heating names; nothing about it is specific to temperature. The
// deadband stops a level sitting on the threshold from chattering the contactor.
using var pump = Thermostat.Heating(50.0f, 10.0f);
foreach (float reading in new[] { percent, 38.0f, 45.0f, 62.0f })
{
    Console.WriteLine($"at {reading}% the pump is {(pump.Update(reading) ? "on" : "off")}");
}
```
<!-- end -->

## Reference

<!-- table: reference kit -->
- Rust: [`pamoja-kit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html)
- TypeScript: [`@pamoja/kit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html)
- Python: [`pamoja.kit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html)
- C#: [`Pamoja.Kit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html)
<!-- end -->
