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

It proves:

- 12 mA is exactly half of the 4-20 mA span, and 4 mA is empty rather than an
  absent signal, so a calibration that is wrong but self-consistent still fails.
- 0 mA maps to -25 on that scale, which is what makes a broken loop distinguishable
  from an empty tank.
- One dropout among five samples does not move the filtered reading at all.
- The controller holds its state inside the deadband instead of switching on every
  sample that crosses the setpoint.

The windowed helpers keep the most recent readings and nothing older. In Rust the
window length is a const generic chosen at the call site; the bindings fix it at 32
readings, so the same five samples give the same median in all four languages.

## Rust

<!-- snippet: examples/tests/guides/kit.rs#example -->
From [`examples/tests/guides/kit.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/kit.rs):

```rust
use pamoja_kit::{Calibration, Median, Thermostat};

// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is
// full, so the span is 16 mA and mid-scale is 12 mA, not 10.
let level = Calibration::two_point(4.0, 0.0, 20.0, 100.0);
assert_eq!(level.apply(12.0), 50.0);
assert_eq!(level.apply(4.0), 0.0);

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
// scale rather than an empty tank. A median window drops that sample outright, where
// an average would blend a quarter of the range into every reading after it.
assert_eq!(level.apply(0.0), -25.0);
let mut filtered = Median::<5>::new();
let mut percent = 0.0;
for milliamps in [12.0, 12.0, 0.0, 12.0, 12.0] {
    percent = level.apply(filtered.update(milliamps));
    assert_eq!(percent, 50.0);
}

// A refill pump runs when the level falls below the deadband, which is the direction
// `heating` names; nothing about it is specific to temperature. The deadband stops a
// level sitting on the threshold from chattering the contactor.
let mut pump = Thermostat::heating(50.0, 10.0);
assert!(!pump.update(percent));
assert!(pump.update(38.0));
assert!(pump.update(45.0));
assert!(!pump.update(62.0));
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/kit.ts#example -->
From [`bindings/node/guides/kit.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/kit.ts):

```typescript
import assert from 'node:assert/strict'

import { Calibration, Median, Thermostat } from '@pamoja/kit'

// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is
// full, so the span is 16 mA and mid-scale is 12 mA, not 10.
const level = Calibration.twoPoint(4, 0, 20, 100)
assert.equal(level.apply(12), 50)
assert.equal(level.apply(4), 0)

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
// scale rather than an empty tank. A median window drops that sample outright, where an
// average would blend a quarter of the range into every reading after it.
assert.equal(level.apply(0), -25)
const filtered = new Median()
let percent = 0
for (const milliamps of [12, 12, 0, 12, 12]) {
  percent = level.apply(filtered.update(milliamps))
  assert.equal(percent, 50)
}

// A refill pump runs when the level falls below the deadband, which is the direction
// heating names; nothing about it is specific to temperature. The deadband stops a level
// sitting on the threshold from chattering the contactor.
const pump = Thermostat.heating(50, 10)
assert.equal(pump.update(percent), false)
assert.equal(pump.update(38), true)
assert.equal(pump.update(45), true)
assert.equal(pump.update(62), false)
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/kit.py#example -->
From [`bindings/python/guides/kit.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/kit.py):

```python
from pamoja.kit import Calibration, Median, Thermostat

# A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA is
# full, so the span is 16 mA and mid-scale is 12 mA, not 10.
level = Calibration.two_point(4.0, 0.0, 20.0, 100.0)
assert level.apply(12.0) == 50.0
assert level.apply(4.0) == 0.0

# The live zero is what makes a broken loop detectable: 0 mA is off the bottom of the
# scale rather than an empty tank. A median window drops that sample outright, where an
# average would blend a quarter of the range into every reading after it.
assert level.apply(0.0) == -25.0
filtered = Median()
percent = 0.0
for milliamps in (12.0, 12.0, 0.0, 12.0, 12.0):
    percent = level.apply(filtered.update(milliamps))
    assert percent == 50.0

# A refill pump runs when the level falls below the deadband, which is the direction
# heating names; nothing about it is specific to temperature. The deadband stops a level
# sitting on the threshold from chattering the contactor.
pump = Thermostat.heating(50.0, 10.0)
assert pump.update(percent) is False
assert pump.update(38.0) is True
assert pump.update(45.0) is True
assert pump.update(62.0) is False
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs):

```csharp
// A 4-20 mA process loop carries the level as a current: 4 mA is empty and 20 mA
// is full, so the span is 16 mA and mid-scale is 12 mA, not 10.
using var level = Calibration.TwoPoint(4.0f, 0.0f, 20.0f, 100.0f);
Expect(level.Apply(12.0f) == 50.0f, "mid-scale current is half of the span");
Expect(level.Apply(4.0f) == 0.0f, "the live zero reads empty");

// The live zero is what makes a broken loop detectable: 0 mA is off the bottom of
// the scale rather than an empty tank. A median window drops that sample outright,
// where an average would blend a quarter of the range into every reading after it.
Expect(level.Apply(0.0f) == -25.0f, "a dead loop reads below the scale");
using var filtered = new Median();
float[] loop = [12.0f, 12.0f, 0.0f, 12.0f, 12.0f];
float percent = 0.0f;
foreach (float milliamps in loop)
{
    percent = level.Apply(filtered.Update(milliamps));
    Expect(percent == 50.0f, "the dropout never reaches the pump");
}

// A refill pump runs when the level falls below the deadband, which is the
// direction Heating names; nothing about it is specific to temperature. The
// deadband stops a level sitting on the threshold from chattering the contactor.
using var pump = Thermostat.Heating(50.0f, 10.0f);
Expect(!pump.Update(percent), "a half-full tank leaves the pump off");
Expect(pump.Update(38.0f), "below the deadband the pump runs");
Expect(pump.Update(45.0f), "inside the deadband it holds its state");
Expect(!pump.Update(62.0f), "above the deadband it stops");
```
<!-- end -->

## Reference

<!-- table: reference kit -->
- Rust: [`pamoja-kit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html)
- TypeScript: [`@pamoja/kit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html)
- Python: [`pamoja.kit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html)
- C#: [`Pamoja.Kit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html)
<!-- end -->
