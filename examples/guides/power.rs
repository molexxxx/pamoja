//! The power-budget guide example; see docs/guides/power.md.
//!
//! Run: `cargo run -p pamoja-examples --example power`

use std::error::Error;

/// A solar node deciding how often to wake as its battery falls, and how long it can
/// afford to stay awake on what its panel harvests.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use core::time::Duration;

    use pamoja_power::{DutyCycle, PowerPlan};

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
    // ANCHOR_END: example

    use pamoja_power::PowerMode;

    assert_eq!(plan.mode(0.80), PowerMode::Active);
    assert_eq!(plan.interval(0.80), Duration::from_secs(60));
    assert_eq!(plan.mode(0.35), PowerMode::Saver);
    assert_eq!(plan.mode(0.12), PowerMode::Critical);
    assert_eq!(plan.interval(0.12), Duration::from_secs(3600));
    assert_eq!(charging, PowerMode::Saver);
    assert_eq!(plan.interval_for(charging), Duration::from_secs(600));
    assert_eq!(plan.mode(unknown), PowerMode::Critical);
    assert_eq!(plan.interval(unknown), Duration::from_secs(3600));
    assert_eq!(cold, PowerMode::Saver);
    assert_eq!(mild, PowerMode::Active);
    assert!((healthy.fraction() - 2.0 / 60.0).abs() < 1e-6);
    assert!((flat.fraction() - 2.0 / 3600.0).abs() < 1e-6);
    assert_eq!(paid, 6000);
    assert_eq!(cloudy.period(), minute);
    assert_eq!(sunny.active(), minute);
    assert_eq!(sunny.sleep(), Duration::ZERO);
    assert_eq!(unread.active(), Duration::ZERO);
    assert_eq!(unread.sleep(), minute);

    Ok(())
}
