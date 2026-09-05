//! The helpers guide example; see docs/guides/kit.md.

/// A tank level read off a 4-20 mA loop: scaled to a percentage, filtered so a broken loop
/// does not move it, and used to run the refill pump.
#[test]
fn a_reading_is_calibrated_filtered_and_acted_on() {
    // ANCHOR: example
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
    // ANCHOR_END: example

    assert_eq!(level.apply(12.0), 50.0);
    assert_eq!(level.apply(4.0), 0.0);
    assert_eq!(broken, -25.0);
    assert_eq!(percent, 50.0);

    let mut pump = Thermostat::heating(50.0, 10.0);
    assert!(!pump.update(50.0));
    assert!(pump.update(38.0));
    assert!(pump.update(45.0));
    assert!(!pump.update(62.0));
}
