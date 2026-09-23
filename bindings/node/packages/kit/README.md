# @pamoja/kit

Plain-language helper math: smoothing, calibration, PID and thermostat control, a trigger with hysteresis, trend and surge prediction, rolling windows, unit conversions, geo, and robot motion. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/kit.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html)

## Install

```sh
npm install @pamoja/kit
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/kit.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/kit.ts):

```typescript
import {
  Anomaly,
  bearingBetween,
  Calibration,
  Complementary,
  type Coord,
  deadband,
  Debounce,
  Depletion,
  dewPoint,
  distanceBetween,
  Edge,
  fahrenheitToCelsius,
  Geofence,
  Kalman,
  Median,
  Pid,
  Ramp,
  Smoother,
  Surge,
  Thermostat,
  tiltFromAccel,
  Trend,
  Trigger,
  Window,
} from '@pamoja/kit'

// The tower's level transmitter reports on a 4-20 mA loop: 4 mA is empty and 20 mA is
// full, so mid-scale is 12 mA, and a dead loop reads below empty rather than as empty.
const level = Calibration.twoPoint(4, 0, 20, 100)
const [mid, empty, dead] = [level.apply(12), level.apply(4), level.apply(0)]
console.log(
  `level     12 mA reads ${mid.toFixed(1)}%, 4 mA reads ${empty.toFixed(1)}%, a dead loop ${dead.toFixed(1)}%`,
)

// One dropout in five readings: the median of the five ignores it, the mean does not.
const median = new Median(5)
const recent = new Window(5)
let held = 0
for (const milliamps of [12, 12, 0, 12, 12]) {
  held = median.update(milliamps)
  recent.push(milliamps)
}
const heldPercent = level.apply(held)
const meanPercent = level.apply(recent.mean()!)
console.log(
  `level     through a dropout the median holds ${heldPercent.toFixed(1)}%, the mean falls to ${meanPercent.toFixed(1)}%`,
)

// Water sloshing in the tower swings the reading. A smoother moves a quarter of the way
// from its last value toward each new reading, so the swing mostly cancels out.
const smoother = new Smoother(0.25)
const swing = new Window(6)
for (const percent of [50, 53, 48, 52, 49, 51]) {
  smoother.update(percent)
  swing.push(percent)
}
console.log(
  `level     sloshing readings from ${swing.min()!.toFixed(1)}% to ${swing.max()!.toFixed(1)}% smooth to ${smoother.value!.toFixed(1)}%`,
)

// A Kalman filter is told how noisy the sensor is and how fast the level can really move.
// When the pump starts and the level climbs from 50% to 60%, the one told the level barely
// moves takes the climb for noise and lags; the one told it moves keeps up.
const expectsSteady = new Kalman(0.01, 2, 50)
const expectsMotion = new Kalman(0.5, 2, 50)
for (const percent of [50, 50, 50, 60, 60, 60, 60]) {
  expectsSteady.update(percent)
  expectsMotion.update(percent)
}
const [slow, fast] = [expectsSteady.estimate, expectsMotion.estimate]
console.log(
  `level     four readings into a rise to 60%, a Kalman filter expecting a steady level reads ${slow.toFixed(1)}%, one expecting motion ${fast.toFixed(1)}%`,
)

// An accelerometer on the tank watches the tower's lean. Standing still, only gravity pulls
// on it, so the direction of the pull, in g, gives the tilt.
const atRest = tiltFromAccel(0, 0.007, 1)
console.log(`tower     at rest the accelerometer reads a lean of ${atRest.roll.toFixed(2)} degrees`)

// In wind the tower sways, and the sway's own acceleration swings the accelerometer's tilt.
// A gyro's rate of turn does not swing, but it drifts. A complementary filter trusts the gyro
// from one tenth of a second to the next and the accelerometer over time.
const lean = new Complementary(0.98, atRest.roll)
const gusts = new Window(5)
for (const [rate, tilt] of [
  [0.4, 2.1],
  [-0.6, -1.3],
  [0.5, 1.8],
  [-0.3, -0.9],
  [0.1, 1.2],
]) {
  lean.update(rate, tilt, 0.1)
  gusts.push(tilt)
}
const steadyLean = lean.estimate
console.log(
  `tower     in wind the accelerometer swings from ${gusts.min()!.toFixed(1)} to ${gusts.max()!.toFixed(1)} degrees; fused with the gyro the lean reads ${steadyLean.toFixed(1)}`,
)

// The refill pump starts at 40% and stops at 60%: on/off control with a band either side
// of 50. Starting when the level falls is the direction heating names.
const pump = Thermostat.heating(50, 10)
const states = [50, 39, 45, 61].map(
  (percent) => `${percent.toFixed(0)}% ${pump.update(percent) ? 'on' : 'off'}`,
)
console.log(`pump      ${states.join(', ')}`)

// The high-level float switch bounces as the water sloshes at the top. It has to read full
// three times running before the pump controller believes it.
const float = new Debounce(3, false)
let [rawChanges, settledChanges, lastRaw] = [0, 0, false]
for (const raw of [true, false, true, true, true, false, true]) {
  rawChanges += raw !== lastRaw ? 1 : 0
  lastRaw = raw
  const before = float.state
  settledChanges += float.update(raw) !== before ? 1 : 0
}
console.log(
  `float     ${rawChanges} raw changes settled into ${settledChanges}: the tower reads ${float.state ? 'full' : 'not full'}`,
)

// The low-water alarm is sent once when the level drops under 20% and not again until it
// has come back above 25%, however long it hovers near the line.
const lowWater = Trigger.below(20, 5)
for (const percent of [24, 19, 18, 21, 19, 26]) {
  const edge = lowWater.update(percent)
  if (edge === Edge.Set) {
    console.log(`alarm     low water at ${percent.toFixed(0)}%: alarm sent`)
  } else if (edge === Edge.Cleared) {
    console.log(`alarm     back to ${percent.toFixed(0)}%: all clear sent`)
  }
}

// A booster pump holds the mains at 3.0 bar. The PID asks for a speed, the ramp lets the
// pump change by at most 25% a second so the pipes never take a water hammer, and within
// 0.05 bar of 3.0 the reading counts as on target.
const pressureHold = Pid.withLimits(40, 8, 0, 0, 100)
const softStart = new Ramp(0, 25)
for (const bar of [1.0, 1.8, 2.5, 2.9, 3.02]) {
  const steady = deadband(bar, 3.0, 0.05)
  const asked = pressureHold.update(3.0, steady, 1.0)
  const given = softStart.update(asked)
  console.log(
    `booster   at ${bar.toFixed(2)} bar the PID asks for ${asked.toFixed(0)}%, the pump is given ${given.toFixed(0)}%`,
  )
}

// The booster's controller hangs in the pump house above the mains. The pump house
// thermometer reads Fahrenheit, and a pipe colder than the air's dew point sweats.
const air = fahrenheitToCelsius(84)
const dew = dewPoint(air, 78)
const sweats = 18 < dew ? 'sweat' : 'stay dry'
console.log(
  `pumphouse 84 F is ${air.toFixed(1)} C, and at 78% humidity it dews at ${dew.toFixed(1)} C, so the 18 C mains ${sweats}`,
)

// A power cut stops the borehole pump. From the hourly level, the countdown says how long
// until the tower reaches its 20% reserve.
const reserve = new Depletion(20)
let hoursLeft: number | null = null
for (const percent of [80, 76, 72]) {
  hoursLeft = reserve.update(percent)
}
console.log(`outage    at the rate it is falling, the tower reaches 20% in ${hoursLeft} hours`)

// With the outlet shut overnight the level should hold. A steady fall is a leak.
const overnight = new Trend(6)
for (const percent of [78.0, 77.6, 77.1, 76.7, 76.2, 75.8]) {
  overnight.push(percent)
}
const slope = overnight.slope!
console.log(`leak      with the outlet shut the level falls ${(-slope).toFixed(2)}% an hour`)

// A burst main shows as pressure falling faster than any demand could pull it.
const burst = Surge.falling(0.5)
for (const bar of [3.0, 2.9, 1.7]) {
  const fall = burst.update(bar)
  if (fall !== null) {
    console.log(`burst     the pressure fell ${fall.toFixed(1)} bar in one reading`)
  }
}

// The flow meter's readings set their own baseline. A hydrant opened stands out, and so
// does a reading the meter could not make.
const flow = new Anomaly(3, 8)
const normal = new Window(8)
let flagged = 0
for (const cubicMeters of [12.1, 11.8, 12.4, 12.0, 11.9, 12.2, 12.0, 12.3]) {
  flagged += flow.check(cubicMeters) ? 1 : 0
  normal.push(cubicMeters)
}
console.log(
  `meter     ${normal.len} readings from ${normal.min()!.toFixed(1)} to ${normal.max()!.toFixed(1)} m3/h, ${flagged} flagged`,
)
const verdict = (standsOut: boolean) => (standsOut ? 'stands out' : 'passes')
const hydrant = flow.check(30.5)
const failed = flow.check(NaN)
console.log(`meter     a reading of 30.5 m3/h ${verdict(hydrant)}; a failed reading ${verdict(failed)}`)

// The tanker truck delivers inside a 20 km district around its depot.
const depot: Coord = { latitude: -1.5177, longitude: 37.2634 }
const village: Coord = { latitude: -1.448, longitude: 37.339 }
const km = distanceBetween(depot, village) / 1000
const bearing = bearingBetween(depot, village)
console.log(`truck     the village is ${km.toFixed(1)} km from the depot, bearing ${bearing.toFixed(0)} degrees`)
const district = new Geofence(depot, 20_000)
const roadOut: Coord = { latitude: -1.3, longitude: 37.45 }
const further: Coord = { latitude: -1.25, longitude: 37.5 }
const crossings = [depot, village, roadOut, further, village].map((fix) =>
  district.update(fix).toLowerCase(),
)
console.log(`truck     ${crossings.join(', ')}`)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-kit`](https://crates.io/crates/pamoja-kit) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html), [docs.rs](https://docs.rs/pamoja-kit), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-kit) |
| TypeScript | [`@pamoja/kit`](https://www.npmjs.com/package/@pamoja/kit) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-kit) |
| Python | [`pamoja-kit`](https://pypi.org/project/pamoja-kit/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-kit) |
| C# | [`Pamoja.Kit`](https://www.nuget.org/packages/Pamoja.Kit) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-kit) |

## Documentation

- [`@pamoja/kit` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html), every class, function, and type this package exports.
- [The Helpers guide](https://pamoja.molex.cloud/docs/guides/kit.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
