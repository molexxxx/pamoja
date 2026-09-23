# Helpers

Most of what a field node runs sits between the sensor and the actuator: turning a
current into a level, throwing out a bad sample, deciding whether a pump should run,
noticing that something is about to go wrong. `pamoja-kit` is that layer. Each helper is
named for the job rather than the technique, with the algorithm one level down: a
`Smoother` is an exponential moving average, a `Thermostat` is on/off control with
hysteresis, a `Depletion` is a projected countdown, an `Anomaly` is the three-sigma rule.
They are synchronous and allocation-free, so the same code runs on a gateway and on a
microcontroller, and every language gets the same numbers from them.

The helpers fall into five families. Reading helpers turn raw values into trustworthy
ones: `Calibration`, `Median`, `Smoother`, `Kalman`, `Complementary`, and `deadband`.
Deciding helpers turn a reading into an action: `Thermostat`, `Trigger`, `Debounce`, `Pid`,
and `Ramp`. Warning helpers look at recent readings for trouble on its way: `Window`,
`Trend`, `Surge`, `Depletion`, and `Anomaly`. Place helpers work on fixes: `Geofence` and
the distance and bearing between two coordinates. Conversion helpers turn one quantity into
another: the unit conversions, the tilt an accelerometer reads, and the dew point of air.
The motion helpers for robots, from wheel kinematics to a safety gate, have
[a guide of their own](motion.md).

## What the example does

It runs the water system of a small village: a tower fed by a borehole pump, a booster
pump holding the pressure in the mains, a flow meter, and a tanker truck serving the
outlying houses. Each part of the system uses the helpers for one job, and the example
prints what each decided.

- **The tower level** arrives on a 4-20 mA loop. A two-point calibration turns the current
  into a percentage, a median of five readings rides over a dropout, a smoother takes the
  slosh out, and two Kalman filters tuned differently follow a real rise.
- **The tower's lean** is watched by an accelerometer on the tank, and in wind a
  complementary filter steadies it with a gyro.
- **The refill pump** starts at 40% and stops at 60%, and the tower's float switch has to
  read full three times running before the controller believes it.
- **The low-water alarm** is sent once when the level drops under 20% and cleared once it
  is back above 25%.
- **The booster pump** holds the mains at 3.0 bar with a PID, soft-started by a ramp so the
  pipes never take a water hammer. Its controller hangs in the pump house, where the
  thermometer reads Fahrenheit and the dew point says whether the cold mains sweat.
- **The warnings**: a countdown to the reserve level during a power cut, a leak seen as a
  steady fall overnight, a burst main seen as a sudden pressure drop, and a flow meter
  whose odd readings stand out from their own baseline.
- **The truck** has a 20 km district around its depot, and its route takes it out of the
  district and back.

The pressure and level readings are typed into the example, so every language sees the
same ones; on a real system they come from the transmitters.

It proves:

- 12 mA reads 50% and 4 mA reads 0%, because the span starts at 4 mA, not zero, and a dead
  loop reads -25%, which is how a broken wire tells itself apart from an empty tank.
- One dropout in five readings leaves the median at 50% while it drags the mean to 35%.
- Readings swinging between 48% and 53% smooth to 50.4%.
- Four readings into a rise from 50% to 60%, a Kalman filter told the level barely moves
  reads 55.2% and one told it moves reads 58.6%. Its process noise is the whole
  difference.
- A pull of 0.007 g sideways against 1 g down is a lean of 0.40 degrees. In wind the
  accelerometer's tilt swings from -1.3 to 2.1 degrees, and fused with the gyro the lean
  still reads 0.4.
- The pump holds its state inside the band: off at 50%, on at 39%, still on at 45%, off
  at 61%.
- Five raw changes on the float switch settle into one debounced change.
- Six readings hovering around the low-water line send one alarm and one all clear.
- The PID asks for 96% at 1.0 bar while the ramp gives the pump 25%, and at 3.02 bar,
  inside the 0.05 bar deadband, the PID's integral alone holds the pump at 30%.
- 84 F is 28.9 C, and at 78% humidity that air dews at 24.7 C, so mains at 18 C sweat.
- A level falling 4% an hour from 72% reaches the 20% reserve in 13 hours.
- A least-squares line through six overnight levels falls 0.45% an hour.
- A 1.2 bar drop between two readings is a surge past the 0.5 bar limit.
- Eight normal flows raise no flag; a 30.5 m3/h reading and a failed reading both stand
  out.
- The village is 11.4 km from the depot on a bearing of 47 degrees, and the truck's route
  reports inside, inside, exited, outside, entered: one alert on the way out, one on the
  way back.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example kit" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example kit</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- kit" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- kit</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/kit.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/kit.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- kit" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- kit</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-kit` is `no_std` and allocation-free. Each helper is a plain value you own
and update: construct it, feed it readings with `update`, `push`, or `check`, and read its
state back through a method such as `value()` or `is_on()`. An answer that may not exist
yet, such as a median before the first reading, is an `Option`. Nothing returns a
`Result`: a helper never fails, and a parameter outside its range falls back as the
parameter table below says. The windowed helpers take their capacity as a const generic,
`Median::<5>::new()`, and `with_capacity(n)` keeps fewer than that at run time. The geo
helpers are methods on `Coordinate`: `distance_to` and `bearing_to`. The tilt and the dew
point live in the `imu` and `weather` modules, and the conversions in `units`. The default features
are `geo`, `imu`, `weather`, and `robotics`; each pulls in `libm` for float math, and
turning them off leaves a crate with no dependencies at all.

<!-- snippet: examples/guides/kit.rs#example -->
From [`examples/guides/kit.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/kit.rs):

```rust
use pamoja_kit::imu::tilt_from_accel;
use pamoja_kit::weather::dew_point;
use pamoja_kit::{
    deadband, units, Anomaly, Boundary, Calibration, Complementary, Coordinate, Debounce,
    Depletion, Edge, Geofence, Kalman, Median, Pid, Ramp, Smoother, Surge, Thermostat, Trend,
    Trigger, Window,
};

// The tower's level transmitter reports on a 4-20 mA loop: 4 mA is empty and 20 mA is
// full, so mid-scale is 12 mA, and a dead loop reads below empty rather than as empty.
let level = Calibration::two_point(4.0, 0.0, 20.0, 100.0);
let (mid, empty, dead) = (level.apply(12.0), level.apply(4.0), level.apply(0.0));
println!("level     12 mA reads {mid:.1}%, 4 mA reads {empty:.1}%, a dead loop {dead:.1}%");

// One dropout in five readings: the median of the five ignores it, the mean does not.
let mut median = Median::<5>::new();
let mut recent = Window::<5>::new();
let mut held = 0.0;
for milliamps in [12.0, 12.0, 0.0, 12.0, 12.0] {
    held = median.update(milliamps);
    recent.push(milliamps);
}
let mean = recent.mean().expect("the window holds five readings");
let (held, mean) = (level.apply(held), level.apply(mean));
println!(
    "level     through a dropout the median holds {held:.1}%, the mean falls to {mean:.1}%"
);

// Water sloshing in the tower swings the reading. A smoother moves a quarter of the way
// from its last value toward each new reading, so the swing mostly cancels out.
let mut smoother = Smoother::new(0.25);
let mut swing = Window::<6>::new();
for percent in [50.0, 53.0, 48.0, 52.0, 49.0, 51.0] {
    smoother.update(percent);
    swing.push(percent);
}
let (low, high) = (swing.min().unwrap_or(0.0), swing.max().unwrap_or(0.0));
let smoothed = smoother.value().unwrap_or(0.0);
println!("level     sloshing readings from {low:.1}% to {high:.1}% smooth to {smoothed:.1}%");

// A Kalman filter is told how noisy the sensor is and how fast the level can really
// move. When the pump starts and the level climbs from 50% to 60%, the one told the
// level barely moves takes the climb for noise and lags; the one told it moves keeps up.
let mut expects_steady = Kalman::new(0.01, 2.0, 50.0);
let mut expects_motion = Kalman::new(0.5, 2.0, 50.0);
for percent in [50.0, 50.0, 50.0, 60.0, 60.0, 60.0, 60.0] {
    expects_steady.update(percent);
    expects_motion.update(percent);
}
let (slow, fast) = (expects_steady.estimate(), expects_motion.estimate());
println!("level     four readings into a rise to 60%, a Kalman filter expecting a steady level reads {slow:.1}%, one expecting motion {fast:.1}%");

// An accelerometer on the tank watches the tower's lean. Standing still, only gravity
// pulls on it, so the direction of the pull, in g, gives the tilt.
let at_rest = tilt_from_accel(0.0, 0.007, 1.0);
println!(
    "tower     at rest the accelerometer reads a lean of {:.2} degrees",
    at_rest.roll
);

// In wind the tower sways, and the sway's own acceleration swings the accelerometer's
// tilt. A gyro's rate of turn does not swing, but it drifts. A complementary filter
// trusts the gyro from one tenth of a second to the next and the accelerometer over time.
let mut lean = Complementary::new(0.98, at_rest.roll as f32);
let mut gusts = Window::<5>::new();
for (rate, tilt) in [
    (0.4, 2.1),
    (-0.6, -1.3),
    (0.5, 1.8),
    (-0.3, -0.9),
    (0.1, 1.2),
] {
    lean.update(rate, tilt, 0.1);
    gusts.push(tilt);
}
let (low, high) = (gusts.min().unwrap_or(0.0), gusts.max().unwrap_or(0.0));
let steady_lean = lean.estimate();
println!("tower     in wind the accelerometer swings from {low:.1} to {high:.1} degrees; fused with the gyro the lean reads {steady_lean:.1}");

// The refill pump starts at 40% and stops at 60%: on/off control with a band either
// side of 50. Starting when the level falls is the direction `heating` names.
let mut pump = Thermostat::heating(50.0, 10.0);
let states: Vec<String> = [50.0, 39.0, 45.0, 61.0]
    .iter()
    .map(|&percent| {
        let running = if pump.update(percent) { "on" } else { "off" };
        format!("{percent:.0}% {running}")
    })
    .collect();
println!("pump      {}", states.join(", "));

// The high-level float switch bounces as the water sloshes at the top. It has to read
// full three times running before the pump controller believes it.
let mut float = Debounce::new(3, false);
let (mut raw_changes, mut settled_changes, mut last_raw) = (0, 0, false);
for raw in [true, false, true, true, true, false, true] {
    raw_changes += usize::from(raw != last_raw);
    last_raw = raw;
    let before = float.state();
    settled_changes += usize::from(float.update(raw) != before);
}
let full = if float.state() { "full" } else { "not full" };
println!("float     {raw_changes} raw changes settled into {settled_changes}: the tower reads {full}");

// The low-water alarm is sent once when the level drops under 20% and not again until
// it has come back above 25%, however long it hovers near the line.
let mut low_water = Trigger::below(20.0, 5.0);
for percent in [24.0, 19.0, 18.0, 21.0, 19.0, 26.0] {
    match low_water.update(percent) {
        Some(Edge::Set) => println!("alarm     low water at {percent:.0}%: alarm sent"),
        Some(Edge::Cleared) => println!("alarm     back to {percent:.0}%: all clear sent"),
        None => {}
    }
}

// A booster pump holds the mains at 3.0 bar. The PID asks for a speed, the ramp lets
// the pump change by at most 25% a second so the pipes never take a water hammer, and
// within 0.05 bar of 3.0 the reading counts as on target.
let mut pressure_hold = Pid::new(40.0, 8.0, 0.0).with_limits(0.0, 100.0);
let mut soft_start = Ramp::new(0.0, 25.0);
for bar in [1.0, 1.8, 2.5, 2.9, 3.02] {
    let steady = deadband(bar, 3.0, 0.05);
    let asked = pressure_hold.update(3.0, steady, 1.0);
    let given = soft_start.update(asked);
    println!(
        "booster   at {bar:.2} bar the PID asks for {asked:.0}%, the pump is given {given:.0}%"
    );
}

// The booster's controller hangs in the pump house above the mains. The pump house
// thermometer reads Fahrenheit, and a pipe colder than the air's dew point sweats.
let air = units::fahrenheit_to_celsius(84.0);
let dew = dew_point(f64::from(air), 78.0);
let sweats = if 18.0 < dew { "sweat" } else { "stay dry" };
println!("pumphouse 84 F is {air:.1} C, and at 78% humidity it dews at {dew:.1} C, so the 18 C mains {sweats}");

// A power cut stops the borehole pump. From the hourly level, the countdown says how
// long until the tower reaches its 20% reserve.
let mut reserve = Depletion::new(20.0);
let mut hours_left = None;
for percent in [80.0, 76.0, 72.0] {
    hours_left = reserve.update(percent);
}
let hours = hours_left.expect("the level is falling");
println!("outage    at the rate it is falling, the tower reaches 20% in {hours} hours");

// With the outlet shut overnight the level should hold. A steady fall is a leak.
let mut overnight = Trend::<6>::new();
for percent in [78.0, 77.6, 77.1, 76.7, 76.2, 75.8] {
    overnight.push(percent);
}
let slope = overnight.slope().expect("six readings fit a line");
println!(
    "leak      with the outlet shut the level falls {:.2}% an hour",
    -slope
);

// A burst main shows as pressure falling faster than any demand could pull it.
let mut burst = Surge::falling(0.5);
for bar in [3.0, 2.9, 1.7] {
    if let Some(fall) = burst.update(bar) {
        println!("burst     the pressure fell {fall:.1} bar in one reading");
    }
}

// The flow meter's readings set their own baseline. A hydrant opened stands out, and
// so does a reading the meter could not make.
let mut flow = Anomaly::<8>::new(3.0);
let mut normal = Window::<8>::new();
let mut flagged = 0;
for cubic_meters in [12.1, 11.8, 12.4, 12.0, 11.9, 12.2, 12.0, 12.3] {
    flagged += usize::from(flow.check(cubic_meters));
    normal.push(cubic_meters);
}
let (low, high) = (normal.min().unwrap_or(0.0), normal.max().unwrap_or(0.0));
println!(
    "meter     {} readings from {low:.1} to {high:.1} m3/h, {flagged} flagged",
    normal.len()
);
let verdict = |flagged: bool| if flagged { "stands out" } else { "passes" };
let hydrant = flow.check(30.5);
let failed = flow.check(f32::NAN);
println!(
    "meter     a reading of 30.5 m3/h {}; a failed reading {}",
    verdict(hydrant),
    verdict(failed)
);

// The tanker truck delivers inside a 20 km district around its depot.
let depot = Coordinate::new(-1.5177, 37.2634);
let village = Coordinate::new(-1.4480, 37.3390);
let km = depot.distance_to(village) / 1000.0;
let bearing = depot.bearing_to(village);
println!("truck     the village is {km:.1} km from the depot, bearing {bearing:.0} degrees");
let mut district = Geofence::new(depot, 20_000.0);
let road_out = Coordinate::new(-1.3000, 37.4500);
let further = Coordinate::new(-1.2500, 37.5000);
let crossings: Vec<&str> = [depot, village, road_out, further, village]
    .iter()
    .map(|&fix| match district.update(fix) {
        Boundary::Inside => "inside",
        Boundary::Outside => "outside",
        Boundary::Exited => "exited",
        Boundary::Entered => "entered",
    })
    .collect();
println!("truck     {}", crossings.join(", "));
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/kit` exposes each helper as a class. Stateful helpers are built
with `new` or a static factory, such as `Thermostat.heating(setpoint, hysteresis)`, and
their state reads as a property: `smoother.value`, `pump.isOn`, `trend.slope`. An answer
that may not exist yet is `null`. `Trigger.update` returns `Edge.Set`, `Edge.Cleared`, or
`null`, and `Geofence.update` returns `'Inside'`, `'Outside'`, `'Exited'`, or `'Entered'`.
A coordinate is a plain `{ latitude, longitude }` object. The conversions, the tilt, and the
dew point are plain functions, such as `fahrenheitToCelsius(84)`. The windowed helpers take
an optional capacity, `new Median(5)`, up to `WINDOW_CAPACITY` (32), and throw on one they
cannot keep. Readings are narrowed to 32-bit floats on their way in, as they are in every
language, so a result carries about seven significant digits; coordinates stay 64-bit.

<!-- snippet: bindings/node/guides/kit.ts#example -->
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
<!-- end -->

## Python

In Python, `pamoja.kit` works the same way, with state as properties: `smoother.value`,
`pump.is_on`, `trend.slope`. An answer that may not exist yet is `None`. `Trigger.update`
returns the `Edge` enum or `None`, and `Geofence.update` returns the `Boundary` enum.
`Coordinate(latitude, longitude)` is a named tuple the geo helpers take, and the
conversions, the tilt, and the dew point are plain functions. `Pid` takes its
output limits as keywords, and either one alone leaves the other side open:
`Pid(40.0, 8.0, 0.0, min=0.0, max=100.0)`. The windowed helpers take an optional capacity,
`Median(5)`, up to `WINDOW_CAPACITY` (32); one out of range raises `ValueError`, and a
float where a whole number belongs raises `TypeError`, as Python does.

<!-- snippet: bindings/python/guides/kit.py#example -->
From [`bindings/python/guides/kit.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/kit.py):

```python
import math

from pamoja.kit import (
    Anomaly,
    Calibration,
    Complementary,
    Coordinate,
    Debounce,
    Depletion,
    Edge,
    Geofence,
    Kalman,
    Median,
    Pid,
    Ramp,
    Smoother,
    Surge,
    Thermostat,
    Trend,
    Trigger,
    Window,
    bearing_between,
    deadband,
    dew_point,
    distance_between,
    fahrenheit_to_celsius,
    tilt_from_accel,
)

# The tower's level transmitter reports on a 4-20 mA loop: 4 mA is empty and 20 mA is
# full, so mid-scale is 12 mA, and a dead loop reads below empty rather than as empty.
level = Calibration.two_point(4.0, 0.0, 20.0, 100.0)
mid, empty, dead = level.apply(12.0), level.apply(4.0), level.apply(0.0)
print(f"level     12 mA reads {mid:.1f}%, 4 mA reads {empty:.1f}%, a dead loop {dead:.1f}%")

# One dropout in five readings: the median of the five ignores it, the mean does not.
median = Median(5)
recent = Window(5)
held = 0.0
for milliamps in (12.0, 12.0, 0.0, 12.0, 12.0):
    held = median.update(milliamps)
    recent.push(milliamps)
held_percent, mean_percent = level.apply(held), level.apply(recent.mean())
print(
    f"level     through a dropout the median holds {held_percent:.1f}%, "
    f"the mean falls to {mean_percent:.1f}%"
)

# Water sloshing in the tower swings the reading. A smoother moves a quarter of the way
# from its last value toward each new reading, so the swing mostly cancels out.
smoother = Smoother(0.25)
swing = Window(6)
for percent in (50.0, 53.0, 48.0, 52.0, 49.0, 51.0):
    smoother.update(percent)
    swing.push(percent)
print(
    f"level     sloshing readings from {swing.min():.1f}% to {swing.max():.1f}% "
    f"smooth to {smoother.value:.1f}%"
)

# A Kalman filter is told how noisy the sensor is and how fast the level can really move.
# When the pump starts and the level climbs from 50% to 60%, the one told the level barely
# moves takes the climb for noise and lags; the one told it moves keeps up.
expects_steady = Kalman(0.01, 2.0, 50.0)
expects_motion = Kalman(0.5, 2.0, 50.0)
for percent in (50.0, 50.0, 50.0, 60.0, 60.0, 60.0, 60.0):
    expects_steady.update(percent)
    expects_motion.update(percent)
slow, fast = expects_steady.estimate, expects_motion.estimate
print(
    f"level     four readings into a rise to 60%, a Kalman filter expecting a steady level "
    f"reads {slow:.1f}%, one expecting motion {fast:.1f}%"
)

# An accelerometer on the tank watches the tower's lean. Standing still, only gravity pulls
# on it, so the direction of the pull, in g, gives the tilt.
at_rest = tilt_from_accel(0.0, 0.007, 1.0)
print(f"tower     at rest the accelerometer reads a lean of {at_rest.roll:.2f} degrees")

# In wind the tower sways, and the sway's own acceleration swings the accelerometer's tilt.
# A gyro's rate of turn does not swing, but it drifts. A complementary filter trusts the gyro
# from one tenth of a second to the next and the accelerometer over time.
lean = Complementary(0.98, at_rest.roll)
gusts = Window(5)
for rate, tilt in ((0.4, 2.1), (-0.6, -1.3), (0.5, 1.8), (-0.3, -0.9), (0.1, 1.2)):
    lean.update(rate, tilt, 0.1)
    gusts.push(tilt)
steady_lean = lean.estimate
print(
    f"tower     in wind the accelerometer swings from {gusts.min():.1f} to {gusts.max():.1f} "
    f"degrees; fused with the gyro the lean reads {steady_lean:.1f}"
)

# The refill pump starts at 40% and stops at 60%: on/off control with a band either side
# of 50. Starting when the level falls is the direction heating names.
pump = Thermostat.heating(50.0, 10.0)
states = []
for percent in (50.0, 39.0, 45.0, 61.0):
    running = "on" if pump.update(percent) else "off"
    states.append(f"{percent:.0f}% {running}")
print(f"pump      {', '.join(states)}")

# The high-level float switch bounces as the water sloshes at the top. It has to read full
# three times running before the pump controller believes it.
float_switch = Debounce(3, False)
raw_changes, settled_changes, last_raw = 0, 0, False
for raw in (True, False, True, True, True, False, True):
    raw_changes += raw != last_raw
    last_raw = raw
    before = float_switch.state
    settled_changes += float_switch.update(raw) != before
full = "full" if float_switch.state else "not full"
print(
    f"float     {raw_changes} raw changes settled into {settled_changes}: "
    f"the tower reads {full}"
)

# The low-water alarm is sent once when the level drops under 20% and not again until it
# has come back above 25%, however long it hovers near the line.
low_water = Trigger.below(20.0, 5.0)
for percent in (24.0, 19.0, 18.0, 21.0, 19.0, 26.0):
    edge = low_water.update(percent)
    if edge is Edge.SET:
        print(f"alarm     low water at {percent:.0f}%: alarm sent")
    elif edge is Edge.CLEARED:
        print(f"alarm     back to {percent:.0f}%: all clear sent")

# A booster pump holds the mains at 3.0 bar. The PID asks for a speed, the ramp lets the
# pump change by at most 25% a second so the pipes never take a water hammer, and within
# 0.05 bar of 3.0 the reading counts as on target.
pressure_hold = Pid(40.0, 8.0, 0.0, min=0.0, max=100.0)
soft_start = Ramp(0.0, 25.0)
for bar in (1.0, 1.8, 2.5, 2.9, 3.02):
    steady = deadband(bar, 3.0, 0.05)
    asked = pressure_hold.update(3.0, steady, 1.0)
    given = soft_start.update(asked)
    print(
        f"booster   at {bar:.2f} bar the PID asks for {asked:.0f}%, "
        f"the pump is given {given:.0f}%"
    )

# The booster's controller hangs in the pump house above the mains. The pump house
# thermometer reads Fahrenheit, and a pipe colder than the air's dew point sweats.
air = fahrenheit_to_celsius(84.0)
dew = dew_point(air, 78.0)
sweats = "sweat" if 18.0 < dew else "stay dry"
print(
    f"pumphouse 84 F is {air:.1f} C, and at 78% humidity it dews at {dew:.1f} C, "
    f"so the 18 C mains {sweats}"
)

# A power cut stops the borehole pump. From the hourly level, the countdown says how long
# until the tower reaches its 20% reserve.
reserve = Depletion(20.0)
hours_left = None
for percent in (80.0, 76.0, 72.0):
    hours_left = reserve.update(percent)
print(f"outage    at the rate it is falling, the tower reaches 20% in {hours_left} hours")

# With the outlet shut overnight the level should hold. A steady fall is a leak.
overnight = Trend(6)
for percent in (78.0, 77.6, 77.1, 76.7, 76.2, 75.8):
    overnight.push(percent)
slope = overnight.slope
print(f"leak      with the outlet shut the level falls {-slope:.2f}% an hour")

# A burst main shows as pressure falling faster than any demand could pull it.
burst = Surge.falling(0.5)
for bar in (3.0, 2.9, 1.7):
    fall = burst.update(bar)
    if fall is not None:
        print(f"burst     the pressure fell {fall:.1f} bar in one reading")

# The flow meter's readings set their own baseline. A hydrant opened stands out, and so
# does a reading the meter could not make.
flow = Anomaly(3.0, 8)
normal = Window(8)
flagged = 0
for cubic_meters in (12.1, 11.8, 12.4, 12.0, 11.9, 12.2, 12.0, 12.3):
    flagged += flow.check(cubic_meters)
    normal.push(cubic_meters)
print(
    f"meter     {len(normal)} readings from {normal.min():.1f} to {normal.max():.1f} m3/h, "
    f"{flagged} flagged"
)


def verdict(stands_out: bool) -> str:
    return "stands out" if stands_out else "passes"


hydrant = flow.check(30.5)
failed = flow.check(math.nan)
print(f"meter     a reading of 30.5 m3/h {verdict(hydrant)}; a failed reading {verdict(failed)}")

# The tanker truck delivers inside a 20 km district around its depot.
depot = Coordinate(-1.5177, 37.2634)
village = Coordinate(-1.4480, 37.3390)
km = distance_between(depot, village) / 1000.0
bearing = bearing_between(depot, village)
print(f"truck     the village is {km:.1f} km from the depot, bearing {bearing:.0f} degrees")
district = Geofence(depot, 20_000.0)
road_out = Coordinate(-1.3000, 37.4500)
further = Coordinate(-1.2500, 37.5000)
route = (depot, village, road_out, further, village)
crossings = [district.update(fix).value.lower() for fix in route]
print(f"truck     {', '.join(crossings)}")
```
<!-- end -->

## C#

In C#, `Pamoja.Kit` holds each stateful helper over a native handle, so each is
`IDisposable` and belongs in a `using`. State reads as a property (`Value`, `IsOn`,
`Slope`), and an answer that may not exist yet is a nullable value type such as `float?`.
`Trigger.Update` returns `Edge?`, and `Geofence.Update` returns `Boundary`. The stateless
helpers are static methods on `Kit`, such as `Kit.Deadband`, `Kit.DistanceBetween`, and
`Kit.DewPoint`, and the unit conversions are static methods on `Units`. The windowed
helpers take a capacity, `new Median(5)`, up to
`Kit.WindowCapacity` (32), and throw `ArgumentOutOfRangeException` on one they cannot keep.
Every helper is safe to call from more than one thread; calls on the same helper run one at
a time.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/KitGuide.cs):

```csharp
// The tower's level transmitter reports on a 4-20 mA loop: 4 mA is empty and 20 mA
// is full, so mid-scale is 12 mA, and a dead loop reads below empty rather than as
// empty.
Calibration level = Calibration.TwoPoint(4.0f, 0.0f, 20.0f, 100.0f);
(float mid, float empty, float dead) = (level.Apply(12.0f), level.Apply(4.0f), level.Apply(0.0f));
Console.WriteLine(Invariant($"level     12 mA reads {mid:F1}%, 4 mA reads {empty:F1}%, a dead loop {dead:F1}%"));

// One dropout in five readings: the median of the five ignores it, the mean does
// not.
using var median = new Median(5);
using var recent = new Window(5);
float held = 0.0f;
foreach (float milliamps in new[] { 12.0f, 12.0f, 0.0f, 12.0f, 12.0f })
{
    held = median.Update(milliamps);
    recent.Push(milliamps);
}

float heldPercent = level.Apply(held);
float meanPercent = level.Apply(recent.Mean()!.Value);
Console.WriteLine(Invariant(
    $"level     through a dropout the median holds {heldPercent:F1}%, the mean falls to {meanPercent:F1}%"));

// Water sloshing in the tower swings the reading. A smoother moves a quarter of
// the way from its last value toward each new reading, so the swing mostly cancels
// out.
using var smoother = new Smoother(0.25f);
using var swing = new Window(6);
foreach (float percent in new[] { 50.0f, 53.0f, 48.0f, 52.0f, 49.0f, 51.0f })
{
    smoother.Update(percent);
    swing.Push(percent);
}

Console.WriteLine(Invariant(
    $"level     sloshing readings from {swing.Min():F1}% to {swing.Max():F1}% smooth to {smoother.Value:F1}%"));

// A Kalman filter is told how noisy the sensor is and how fast the level can really
// move. When the pump starts and the level climbs from 50% to 60%, the one told the
// level barely moves takes the climb for noise and lags; the one told it moves
// keeps up.
using var expectsSteady = new Kalman(0.01f, 2.0f, 50.0f);
using var expectsMotion = new Kalman(0.5f, 2.0f, 50.0f);
foreach (float percent in new[] { 50.0f, 50.0f, 50.0f, 60.0f, 60.0f, 60.0f, 60.0f })
{
    expectsSteady.Update(percent);
    expectsMotion.Update(percent);
}

(float slow, float fast) = (expectsSteady.Estimate, expectsMotion.Estimate);
Console.WriteLine(Invariant(
    $"level     four readings into a rise to 60%, a Kalman filter expecting a steady level reads {slow:F1}%, one expecting motion {fast:F1}%"));

// An accelerometer on the tank watches the tower's lean. Standing still, only gravity
// pulls on it, so the direction of the pull, in g, gives the tilt.
Tilt atRest = Kit.TiltFromAccel(0.0, 0.007, 1.0);
Console.WriteLine(Invariant($"tower     at rest the accelerometer reads a lean of {atRest.Roll:F2} degrees"));

// In wind the tower sways, and the sway's own acceleration swings the accelerometer's
// tilt. A gyro's rate of turn does not swing, but it drifts. A complementary filter
// trusts the gyro from one tenth of a second to the next and the accelerometer over
// time.
using var lean = new Complementary(0.98f, (float)atRest.Roll);
using var gusts = new Window(5);
foreach ((float rate, float tilt) in new[] { (0.4f, 2.1f), (-0.6f, -1.3f), (0.5f, 1.8f), (-0.3f, -0.9f), (0.1f, 1.2f) })
{
    lean.Update(rate, tilt, 0.1f);
    gusts.Push(tilt);
}

float steadyLean = lean.Estimate;
Console.WriteLine(Invariant(
    $"tower     in wind the accelerometer swings from {gusts.Min():F1} to {gusts.Max():F1} degrees; fused with the gyro the lean reads {steadyLean:F1}"));

// The refill pump starts at 40% and stops at 60%: on/off control with a band either
// side of 50. Starting when the level falls is the direction Heating names.
using var pump = Thermostat.Heating(50.0f, 10.0f);
var states = new List<string>();
foreach (float percent in new[] { 50.0f, 39.0f, 45.0f, 61.0f })
{
    string running = pump.Update(percent) ? "on" : "off";
    states.Add(Invariant($"{percent:F0}% {running}"));
}

Console.WriteLine($"pump      {string.Join(", ", states)}");

// The high-level float switch bounces as the water sloshes at the top. It has to
// read full three times running before the pump controller believes it.
using var floatSwitch = new Debounce(3, false);
(int rawChanges, int settledChanges, bool lastRaw) = (0, 0, false);
foreach (bool raw in new[] { true, false, true, true, true, false, true })
{
    rawChanges += raw != lastRaw ? 1 : 0;
    lastRaw = raw;
    bool before = floatSwitch.State;
    settledChanges += floatSwitch.Update(raw) != before ? 1 : 0;
}

string full = floatSwitch.State ? "full" : "not full";
Console.WriteLine($"float     {rawChanges} raw changes settled into {settledChanges}: the tower reads {full}");

// The low-water alarm is sent once when the level drops under 20% and not again
// until it has come back above 25%, however long it hovers near the line.
using var lowWater = Trigger.Below(20.0f, 5.0f);
foreach (float percent in new[] { 24.0f, 19.0f, 18.0f, 21.0f, 19.0f, 26.0f })
{
    switch (lowWater.Update(percent))
    {
        case Edge.Set:
            Console.WriteLine(Invariant($"alarm     low water at {percent:F0}%: alarm sent"));
            break;
        case Edge.Cleared:
            Console.WriteLine(Invariant($"alarm     back to {percent:F0}%: all clear sent"));
            break;
    }
}

// A booster pump holds the mains at 3.0 bar. The PID asks for a speed, the ramp lets
// the pump change by at most 25% a second so the pipes never take a water hammer,
// and within 0.05 bar of 3.0 the reading counts as on target.
using var pressureHold = Pid.WithLimits(40.0f, 8.0f, 0.0f, 0.0f, 100.0f);
using var softStart = new Ramp(0.0f, 25.0f);
foreach (float bar in new[] { 1.0f, 1.8f, 2.5f, 2.9f, 3.02f })
{
    float steady = Kit.Deadband(bar, 3.0f, 0.05f);
    float asked = pressureHold.Update(3.0f, steady, 1.0f);
    float given = softStart.Update(asked);
    Console.WriteLine(Invariant(
        $"booster   at {bar:F2} bar the PID asks for {asked:F0}%, the pump is given {given:F0}%"));
}

// The booster's controller hangs in the pump house above the mains. The pump house
// thermometer reads Fahrenheit, and a pipe colder than the air's dew point sweats.
float air = Units.FahrenheitToCelsius(84.0f);
double dew = Kit.DewPoint(air, 78.0);
string sweats = 18.0 < dew ? "sweat" : "stay dry";
Console.WriteLine(Invariant(
    $"pumphouse 84 F is {air:F1} C, and at 78% humidity it dews at {dew:F1} C, so the 18 C mains {sweats}"));

// A power cut stops the borehole pump. From the hourly level, the countdown says how
// long until the tower reaches its 20% reserve.
using var reserve = new Depletion(20.0f);
uint? hoursLeft = null;
foreach (float percent in new[] { 80.0f, 76.0f, 72.0f })
{
    hoursLeft = reserve.Update(percent);
}

Console.WriteLine($"outage    at the rate it is falling, the tower reaches 20% in {hoursLeft} hours");

// With the outlet shut overnight the level should hold. A steady fall is a leak.
using var overnight = new Trend(6);
foreach (float percent in new[] { 78.0f, 77.6f, 77.1f, 76.7f, 76.2f, 75.8f })
{
    overnight.Push(percent);
}

float slope = overnight.Slope!.Value;
Console.WriteLine(Invariant($"leak      with the outlet shut the level falls {-slope:F2}% an hour"));

// A burst main shows as pressure falling faster than any demand could pull it.
using var burst = Surge.Falling(0.5f);
foreach (float bar in new[] { 3.0f, 2.9f, 1.7f })
{
    if (burst.Update(bar) is float fall)
    {
        Console.WriteLine(Invariant($"burst     the pressure fell {fall:F1} bar in one reading"));
    }
}

// The flow meter's readings set their own baseline. A hydrant opened stands out,
// and so does a reading the meter could not make.
using var flow = new Anomaly(3.0f, 8);
using var normal = new Window(8);
int flagged = 0;
foreach (float cubicMeters in new[] { 12.1f, 11.8f, 12.4f, 12.0f, 11.9f, 12.2f, 12.0f, 12.3f })
{
    flagged += flow.Check(cubicMeters) ? 1 : 0;
    normal.Push(cubicMeters);
}

Console.WriteLine(Invariant(
    $"meter     {normal.Count} readings from {normal.Min():F1} to {normal.Max():F1} m3/h, {flagged} flagged"));
static string Verdict(bool standsOut) => standsOut ? "stands out" : "passes";
bool hydrant = flow.Check(30.5f);
bool failed = flow.Check(float.NaN);
Console.WriteLine($"meter     a reading of 30.5 m3/h {Verdict(hydrant)}; a failed reading {Verdict(failed)}");

// The tanker truck delivers inside a 20 km district around its depot.
var depot = new Coordinate(-1.5177, 37.2634);
var village = new Coordinate(-1.4480, 37.3390);
double km = Kit.DistanceBetween(depot, village) / 1000.0;
double bearing = Kit.BearingBetween(depot, village);
Console.WriteLine(Invariant($"truck     the village is {km:F1} km from the depot, bearing {bearing:F0} degrees"));
using var district = new Geofence(depot, 20_000.0);
var roadOut = new Coordinate(-1.3000, 37.4500);
var further = new Coordinate(-1.2500, 37.5000);
string[] crossings = new[] { depot, village, roadOut, further, village }
    .Select(fix => district.Update(fix).ToString().ToLowerInvariant())
    .ToArray();
Console.WriteLine($"truck     {string.Join(", ", crossings)}");
```
<!-- end -->

## Values at a glance

**Reading helpers** turn a raw value into one worth acting on:

| Helper | What it does |
| --- | --- |
| calibration | maps a raw reading onto real units along a line, built from two known points or from a scale and offset |
| median | the middle of the recent readings, which a single spike cannot move |
| smoother | an exponential moving average: each output moves part of the way toward the new sample |
| Kalman filter | an estimate that weighs how far the true value can move against how noisy the sensor is |
| complementary filter | fuses a fast rate that drifts, such as a gyro's, with a slow absolute reading that does not, such as an accelerometer's tilt |
| deadband | holds a reading at a center while it stays close to it |

**Deciding helpers** turn a reading into an action:

| Helper | What it does |
| --- | --- |
| thermostat | on/off control with a band either side of a setpoint |
| trigger | one event when a reading crosses a line, and one when it comes back past a release band |
| debounce | accepts a change only once it has held for so many readings |
| PID | a continuous output that holds a reading at a setpoint |
| ramp | moves a value toward a target by at most a step each update |

**Warning helpers** look at recent readings for trouble on its way:

| Helper | What it does |
| --- | --- |
| window | the recent readings, with their mean, spread, and ends |
| trend | the least-squares slope of the recent readings |
| surge | a change between two readings past a limit |
| depletion | how many readings until a falling level reaches a threshold |
| anomaly | a reading far from the mean of the ones before it |

**Place helpers** work on fixes:

| Helper | What it does |
| --- | --- |
| coordinate | a latitude and longitude in degrees |
| distance and bearing | the great-circle distance in meters, and the initial bearing in degrees clockwise from north |
| geofence | inside or outside a circle, and the one fix that crossed |

**Conversion helpers** turn one quantity into another:

| Helper | What it does |
| --- | --- |
| units | Celsius to and from Fahrenheit and kelvin; pascals to and from hectopascals, kilopascals, and psi; a ratio to and from a percentage |
| tilt | the roll and pitch, in degrees, of a three-axis accelerometer at rest, from any units it reads in |
| dew point | the temperature air must cool to before its water condenses, by the Magnus formula with the WMO coefficients |

**The calls in each language.** Each language makes, feeds, and reads the helpers as its
own conventions do:

### Rust

Constructors are associated functions, and state reads through a method:

| Helper | Make it | Feed it, then read it back |
| --- | --- | --- |
| calibration | `Calibration::two_point(raw_low, value_low, raw_high, value_high)` or `Calibration::linear(scale, offset)` | `apply(raw)` |
| median | `Median::<N>::new()` or `Median::<N>::with_capacity(n)` | `update(reading)`; `median()`, `capacity()` |
| smoother | `Smoother::new(weight)` | `update(sample)`; `value()`, `reset()` |
| Kalman filter | `Kalman::new(process_noise, measurement_noise, initial)` | `update(reading)`; `estimate()` |
| complementary filter | `Complementary::new(alpha, initial)` | `update(rate, absolute, dt)`; `estimate()` |
| deadband | `deadband(value, center, width)` | |
| thermostat | `Thermostat::cooling(setpoint, hysteresis)` or `Thermostat::heating(setpoint, hysteresis)` | `update(reading)`; `is_on()` |
| trigger | `Trigger::above(threshold, hysteresis)` or `Trigger::below(threshold, hysteresis)` | `update(reading)` gives `Option<Edge>`; `is_set()`, `threshold()`, `hysteresis()`, `watches_above()` |
| debounce | `Debounce::new(samples, initial)` | `update(raw)`; `state()` |
| PID | `Pid::new(kp, ki, kd)`, then `.with_limits(min, max)` | `update(setpoint, measurement, dt)`; `reset()` |
| ramp | `Ramp::new(start, max_step)` | `update(target)`, `update_capped(target, max_step)`; `value()`, `set(value)` |
| window | `Window::<N>::new()` or `Window::<N>::with_capacity(n)` | `push(reading)`; `len()`, `is_empty()`, `is_full()`, `capacity()`, `latest()`, `oldest()`, `mean()`, `min()`, `max()`, `range()`, `variance()` |
| trend | `Trend::<N>::new()` or `Trend::<N>::with_capacity(n)` | `push(reading)`; `slope()`, `len()`, `capacity()` |
| surge | `Surge::rising(limit)` or `Surge::falling(limit)` | `update(value)` gives `Option<f32>` |
| depletion | `Depletion::new(threshold)` | `update(level)` gives `Option<u32>` |
| anomaly | `Anomaly::<N>::new(sigmas)` or `Anomaly::<N>::with_capacity(sigmas, n)` | `check(reading)`; `len()`, `capacity()` |
| coordinate | `Coordinate::new(latitude, longitude)` | `distance_to(other)`, `bearing_to(other)` |
| geofence | `Geofence::new(center, radius_m)` | `update(fix)` gives a `Boundary`; `contains(fix)` |
| units | `units::fahrenheit_to_celsius(fahrenheit)`, and the rest named the same way | |
| tilt | `imu::tilt_from_accel(ax, ay, az)` gives a `Tilt` | `roll`, `pitch` |
| dew point | `weather::dew_point(celsius, humidity_percent)` | |

### TypeScript

Stateful helpers are classes, and state reads as a property:

| Helper | Make it | Feed it, then read it back |
| --- | --- | --- |
| calibration | `Calibration.twoPoint(rawLow, valueLow, rawHigh, valueHigh)` or `Calibration.linear(scale, offset)` | `apply(raw)` |
| median | `new Median(capacity?)` | `update(reading)`; `value`, `capacity` |
| smoother | `new Smoother(weight)` | `update(sample)`; `value`, `reset()` |
| Kalman filter | `new Kalman(processNoise, measurementNoise, initial)` | `update(reading)`; `estimate` |
| complementary filter | `new Complementary(alpha, initial)` | `update(rate, absolute, dt)`; `estimate` |
| deadband | `deadband(value, center, width)` | |
| thermostat | `Thermostat.cooling(setpoint, hysteresis)` or `Thermostat.heating(setpoint, hysteresis)` | `update(reading)`; `isOn` |
| trigger | `Trigger.above(threshold, hysteresis)` or `Trigger.below(threshold, hysteresis)` | `update(reading)` gives `Edge.Set`, `Edge.Cleared`, or `null`; `isSet`, `threshold`, `hysteresis`, `watchesAbove` |
| debounce | `new Debounce(samples, initial)` | `update(raw)`; `state` |
| PID | `new Pid(kp, ki, kd)` or `Pid.withLimits(kp, ki, kd, min, max)` | `update(setpoint, measurement, dt)`; `reset()` |
| ramp | `new Ramp(start, maxStep)` | `update(target)`; `value`, `set(value)` |
| window | `new Window(capacity?)` | `push(reading)`; `len`, `isEmpty`, `isFull`, `capacity`, `latest()`, `oldest()`, `mean()`, `min()`, `max()`, `range()`, `variance()` |
| trend | `new Trend(capacity?)` | `push(reading)`; `slope`, `capacity` |
| surge | `Surge.rising(limit)` or `Surge.falling(limit)` | `update(value)` gives a number or `null` |
| depletion | `new Depletion(threshold)` | `update(level)` gives a number or `null` |
| anomaly | `new Anomaly(sigmas, capacity?)` | `check(reading)`; `capacity` |
| coordinate | `{ latitude, longitude }`, typed `Coord` | `distanceBetween(from, to)`, `bearingBetween(from, to)` |
| geofence | `new Geofence(center, radiusM)` | `update(point)` gives `'Inside'`, `'Outside'`, `'Exited'`, or `'Entered'`; `contains(point)` |
| units | `fahrenheitToCelsius(fahrenheit)`, and the rest named the same way | |
| tilt | `tiltFromAccel(ax, ay, az)` gives `{ roll, pitch }` | |
| dew point | `dewPoint(celsius, humidityPercent)` | |

### Python

Stateful helpers are classes, and state reads as a property:

| Helper | Make it | Feed it, then read it back |
| --- | --- | --- |
| calibration | `Calibration.two_point(raw_low, value_low, raw_high, value_high)` or `Calibration.linear(scale, offset)` | `apply(raw)` |
| median | `Median(capacity=None)` | `update(reading)`; `value`, `capacity` |
| smoother | `Smoother(weight)` | `update(sample)`; `value`, `reset()` |
| Kalman filter | `Kalman(process_noise, measurement_noise, initial)` | `update(reading)`; `estimate` |
| complementary filter | `Complementary(alpha, initial)` | `update(rate, absolute, dt)`; `estimate` |
| deadband | `deadband(value, center, width)` | |
| thermostat | `Thermostat.cooling(setpoint, hysteresis)` or `Thermostat.heating(setpoint, hysteresis)` | `update(reading)`; `is_on` |
| trigger | `Trigger.above(threshold, hysteresis)` or `Trigger.below(threshold, hysteresis)` | `update(reading)` gives `Edge.SET`, `Edge.CLEARED`, or `None`; `is_set`, `threshold`, `hysteresis`, `watches_above` |
| debounce | `Debounce(samples, initial)` | `update(raw)`; `state` |
| PID | `Pid(kp, ki, kd, *, min=None, max=None)` | `update(setpoint, measurement, dt)`; `reset()` |
| ramp | `Ramp(start, max_step)` | `update(target)`; `value`, `set(value)` |
| window | `Window(capacity=None)` | `push(reading)`; `len(window)`, `is_full`, `capacity`, `latest()`, `oldest()`, `mean()`, `min()`, `max()`, `range()`, `variance()` |
| trend | `Trend(capacity=None)` | `push(reading)`; `slope`, `capacity` |
| surge | `Surge.rising(limit)` or `Surge.falling(limit)` | `update(value)` gives a `float` or `None` |
| depletion | `Depletion(threshold)` | `update(level)` gives an `int` or `None` |
| anomaly | `Anomaly(sigmas, capacity=None)` | `check(reading)`; `capacity` |
| coordinate | `Coordinate(latitude, longitude)` | `distance_between(origin, destination)`, `bearing_between(origin, destination)` |
| geofence | `Geofence(center, radius_m)` | `update(point)` gives a `Boundary`; `contains(point)` |
| units | `fahrenheit_to_celsius(fahrenheit)`, and the rest named the same way | |
| tilt | `tilt_from_accel(ax, ay, az)` gives a `Tilt` | `roll`, `pitch` |
| dew point | `dew_point(celsius, humidity_percent)` | |

### C#

Stateful helpers are `IDisposable` classes, and state reads as a property:

| Helper | Make it | Feed it, then read it back |
| --- | --- | --- |
| calibration | `Calibration.TwoPoint(rawLow, valueLow, rawHigh, valueHigh)` or `Calibration.Linear(scale, offset)` | `Apply(raw)` |
| median | `new Median(capacity)` | `Update(reading)`; `Value`, `Capacity` |
| smoother | `new Smoother(weight)` | `Update(sample)`; `Value`, `Reset()` |
| Kalman filter | `new Kalman(processNoise, measurementNoise, initial)` | `Update(reading)`; `Estimate` |
| complementary filter | `new Complementary(alpha, initial)` | `Update(rate, absolute, dt)`; `Estimate` |
| deadband | `Kit.Deadband(value, center, width)` | |
| thermostat | `Thermostat.Cooling(setpoint, hysteresis)` or `Thermostat.Heating(setpoint, hysteresis)` | `Update(reading)`; `IsOn` |
| trigger | `Trigger.Above(threshold, hysteresis)` or `Trigger.Below(threshold, hysteresis)` | `Update(reading)` gives an `Edge?`; `IsSet`, `Threshold`, `Hysteresis`, `WatchesAbove` |
| debounce | `new Debounce(samples, initial)` | `Update(raw)`; `State` |
| PID | `new Pid(kp, ki, kd)` or `Pid.WithLimits(kp, ki, kd, min, max)` | `Update(setpoint, measurement, dt)`; `Reset()` |
| ramp | `new Ramp(start, maxStep)` | `Update(target)`; `Value`, `Set(value)` |
| window | `new Window(capacity)` | `Push(reading)`; `Count`, `IsFull`, `Capacity`, `Latest()`, `Oldest()`, `Mean()`, `Min()`, `Max()`, `Range()`, `Variance()` |
| trend | `new Trend(capacity)` | `Push(reading)`; `Slope`, `Capacity` |
| surge | `Surge.Rising(limit)` or `Surge.Falling(limit)` | `Update(value)` gives a `float?` |
| depletion | `new Depletion(threshold)` | `Update(level)` gives a `uint?` |
| anomaly | `new Anomaly(sigmas, capacity)` | `Check(reading)`; `Capacity` |
| coordinate | `new Coordinate(latitude, longitude)` | `Kit.DistanceBetween(origin, destination)`, `Kit.BearingBetween(origin, destination)` |
| geofence | `new Geofence(center, radiusM)` | `Update(point)` gives a `Boundary`; `Contains(point)` |
| units | `Units.FahrenheitToCelsius(fahrenheit)`, and the rest named the same way | |
| tilt | `Kit.TiltFromAccel(ax, ay, az)` gives a `Tilt` | `Roll`, `Pitch` |
| dew point | `Kit.DewPoint(celsius, humidityPercent)` | |

<!-- languages end -->

**The parameters, and what a value out of range does.** A helper never refuses a
parameter; it falls back to the behavior the right-hand column describes, so a mistake
shows up as behavior you can see rather than as a crash in the field:

| Parameter | Means | Out of range |
| --- | --- | --- |
| smoother `weight` | how much a new sample counts, 0 to 1; 1 follows the input, near 0 barely moves | clamped to 0 to 1; NaN is 1, no smoothing |
| Kalman `process_noise` | how much the true value can move between readings | its magnitude is used; not finite is 0 |
| Kalman `measurement_noise` | how noisy the sensor is; larger trusts each reading less | its magnitude is used; not finite, or 0, trusts every reading outright |
| Kalman `initial` | the estimate before the first reading, which replaces it | |
| complementary `alpha` | the weight on the integrated rate, 0 to 1; near 1 trusts the rate and corrects slowly toward the absolute reading | clamped to 0 to 1; NaN is 0, which follows the absolute reading, since that one cannot drift |
| complementary `dt` | the time since the last update, in the unit the rate is per | used as given; not finite skips the update |
| `deadband` `width` | how far either side of the center counts as on target | its magnitude is used |
| thermostat `hysteresis` | half the band: a `heating` thermostat turns on at `setpoint - hysteresis` and off at `setpoint + hysteresis` | its magnitude is used; NaN is 0 |
| trigger `hysteresis` | how far back past the line a reading must come to clear | its magnitude is used; NaN is 0 |
| trigger `threshold` | the line | NaN never fires |
| debounce `samples` | how many agreeing readings accept a change; 0 and 1 both accept the first | Rust and C# take a `u16` or `ushort`; TypeScript and Python refuse anything but a whole number from 0 to 65535 |
| PID `kp`, `ki`, `kd` | the proportional, integral, and derivative gains | not finite is 0 |
| PID `min`, `max` | the output limits; the integral is held inside them too | NaN leaves that side open; swapped if `max` is below `min` |
| PID `dt` | the time since the last update, in the unit the gains assume | 0, negative, or not finite holds the integral where it was and drops the derivative for that update |
| ramp `max_step` | the largest change per update | its magnitude is used; NaN holds still; infinity sets no limit |
| surge `limit` | the largest safe change per reading; a step past it is reported | its magnitude is used; NaN is 0, so every change in its direction is reported |
| depletion `threshold` | the level to count down to | NaN never reports |
| anomaly `sigmas` | how many standard deviations away stands out; 3 is the usual choice | its magnitude is used; NaN is 0, so nearly every reading stands out |
| geofence `radius_m` | the fence radius, in meters | its magnitude is used; NaN contains nothing |
| dew point `humidity_percent` | the relative humidity, above 0 and up to 100 | 0 or below is taken as a trace, so the logarithm stays defined |

**The windowed helpers keep a fixed number of readings.** In Rust the storage is a const
generic, and `with_capacity(n)` keeps fewer. The other languages build every windowed
helper with room for 32 and keep what the constructor asks for:

| Helper | Fewest | Most | Unless told | Why the fewest |
| --- | --- | --- | --- | --- |
| window | 1 | 32 | 32 | |
| median | 1 | 32 | 32 | |
| trend | 2 | 32 | 32 | a line needs two points |
| anomaly | 2 | 32 | 32 | a spread needs two readings |

A window's size is a trade between steadiness and lag. A median of `n` readings follows a
real step change about `n / 2` readings late, so a median of 32 is 16 readings behind; five
or seven is the usual choice for knocking out a spike.

**What a reading that is not a number does.** A failed sensor often reports NaN, and one
NaN folded into an average or an integral stays there. So every helper that keeps state
ignores a reading that is not a finite number:

| Helper | Given NaN or infinity |
| --- | --- |
| smoother, Kalman filter, complementary filter, median, ramp | ignores it and returns the value it already held |
| thermostat, debounce | ignores it and keeps its output: a pump that was running keeps running |
| PID | ignores a setpoint or measurement that is not finite and returns its last output |
| trigger, surge, depletion | ignores it and reports nothing |
| window, trend | does not keep it |
| anomaly | flags it, and keeps it out of the baseline |
| geofence | ignores the fix and reports where the last fix was, with no crossing |
| calibration, `deadband`, units, tilt, dew point | keep nothing, so NaN in gives NaN out, as arithmetic does |

**Tuning, in one table:**

| To get | Turn |
| --- | --- |
| a smoother that follows faster | `weight` up; the lag is roughly `1 / weight` readings |
| a Kalman filter that follows a real change | `process_noise` up, toward the size of change you expect between readings |
| a Kalman filter that ignores more noise | `measurement_noise` up, toward the variance of the sensor's noise |
| a pump or heater that switches less often | a wider hysteresis |
| fewer false anomalies | `sigmas` up, or more readings in the baseline |
| a gentler actuator | a smaller ramp step |
| a complementary filter that settles on the absolute reading sooner | `alpha` down; it settles over about `alpha * dt / (1 - alpha)` |

## When it goes wrong

What the helpers refuse:

| What happened | The message | Where |
| --- | --- | --- |
| a window or median asked to keep 0, or more than 32 | `capacity must be a whole number from 1 to 32, not 0` | TypeScript and Python; C# throws `ArgumentOutOfRangeException`, whose message starts the same way |
| a trend or anomaly asked to keep fewer than 2 | `capacity must be a whole number from 2 to 32, not 1` | the same |
| a capacity with a fraction | `capacity must be a whole number from 2 to 32, not 2.5` | TypeScript; Python raises `TypeError` before the helper sees it, and C# will not compile it |
| a debounce count below 0 or above 65535 | `samples must be a whole number from 0 to 65535, not -1` | TypeScript and Python |
| a debounce count with a fraction | `samples must be a whole number from 0 to 65535, not 2.9` | TypeScript; Python raises `TypeError` |

Rust has no refusals to show: a const generic fixes the storage when the program compiles,
and `with_capacity` takes the smaller of what it is given and `N`.

The mistakes that cost an afternoon:

- **The pump never stops, or never starts.** `cooling` and `heating` name a direction, not
  a use. A cooler runs when the reading is high; a refill pump, like a heater, runs when it
  is low. Use `heating` for anything that should switch on as the reading falls, as this
  guide's refill pump does.
- **The band is twice as wide as intended.** A thermostat's `hysteresis` is half the band,
  measured from the setpoint: `heating(50, 10)` switches at 40 and 60. A trigger's
  `hysteresis` is the whole release band, on one side of the line: `below(20, 5)` fires
  under 20 and clears above 25.
- **The alarm is sent on every reading.** A thermostat reports a level to hold, so it
  answers `true` on every reading while the load runs. Send an alert from a trigger, which
  reports only the edges.
- **The filtered value lags a real change.** A median of 32, a smoother with a small
  weight, and a Kalman filter with a tiny process noise all trade lag for steadiness. The
  Kalman filter expecting a steady level in this guide was still 4.8 points short of the
  new level four readings into the rise.
- **The PID winds up, or does nothing.** `dt` is the time since the last update, in the
  unit the integral and derivative gains assume. Pass seconds to gains tuned in seconds.
  Passing milliseconds makes the integral a thousand times too strong; passing 0 freezes
  the integral and drops the derivative, so the controller stops correcting a steady
  error.
- **Every change is an anomaly.** With a perfectly flat baseline the spread is zero, so any
  change at all stands out. Real sensor noise gives a baseline some spread; a simulated or
  quantized one may not.
- **The slope is in the wrong units.** A trend's slope is per reading, and a depletion
  countdown is in readings. With a reading every ten minutes, a slope of 0.5 is 3 an hour,
  and a countdown of 12 is two hours.
- **The tilt jumps whenever the board moves.** An accelerometer feels every acceleration,
  not only gravity, so the tilt it gives is right only at rest. Fuse it with a gyro's rate
  through a complementary filter, as the tower does.
- **A pump keeps running on a dead sensor.** A thermostat that was on stays on while its
  reading is NaN, because the helper cannot know what off means for your plant. A node
  that must fail safe checks the reading itself before it trusts the decision.

## Where next

<!-- table: next kit -->
- [Sensor drivers](sensors.md): Datasheet-anchored drivers for eleven parts, from every language.
- [Rules](rules.md): Rules between nodes as a file.
- [Telemetry](telemetry.md): Observability that ships only what is worth the bytes as link cost rises, while counting everything.
<!-- end -->

## Reference

<!-- table: reference kit -->
- Rust: [`pamoja-kit`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_kit/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-kit)
- TypeScript: [`@pamoja/kit`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_kit.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-kit)
- Python: [`pamoja.kit`](https://pamoja.molex.cloud/docs/reference/python/pamoja/kit.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-kit)
- C#: [`Pamoja.Kit`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Kit.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-kit)
<!-- end -->
