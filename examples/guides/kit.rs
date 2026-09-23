//! The helpers guide example; see docs/guides/kit.md.
//!
//! Run: `cargo run -p pamoja-examples --example kit`

use std::error::Error;

/// A village water system run on the helpers: the tower level read off a 4-20 mA loop, the
/// refill pump and its float switch, a low-water alarm, a booster pump holding the mains
/// pressure, the warnings that come before trouble, and the tanker truck's district.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
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
    // ANCHOR_END: example

    assert_eq!((mid, empty, dead), (50.0, 0.0, -25.0));
    assert_eq!(held, 50.0);
    assert!((mean - 35.0).abs() < 1e-3);
    assert_eq!(states, ["50% off", "39% on", "45% on", "61% off"]);
    assert_eq!((raw_changes, settled_changes), (5, 1));
    assert!(float.state());
    assert_eq!(hours, 13);
    assert!((slope + 0.4457).abs() < 1e-3);
    assert!(slow < 56.0 && fast > 58.0);
    assert!((at_rest.roll - 0.401).abs() < 1e-3);
    assert!((steady_lean - 0.426).abs() < 1e-3);
    assert!(dew > 24.0 && dew < 25.0);
    assert!(hydrant && failed);
    assert_eq!(flagged, 0);
    assert_eq!(
        crossings,
        ["inside", "inside", "exited", "outside", "entered"]
    );

    Ok(())
}
