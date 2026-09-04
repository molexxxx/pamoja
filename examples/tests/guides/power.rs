//! The power-scheduling guide example; see docs/guides/power.md.

/// The cadence a solar node runs at as its battery drains, and what the resulting duty
/// fraction says about the average draw each cadence costs.
#[test]
fn a_falling_charge_stretches_the_work_interval() {
    // ANCHOR: example
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
    // ANCHOR_END: example
}
