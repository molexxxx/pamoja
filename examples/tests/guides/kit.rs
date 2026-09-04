//! The helper-math guide example; see docs/guides/kit.md.

/// A level reading off a 4-20 mA loop, calibrated, filtered for a dropout, and turned into
/// a pump decision, checked against the currents the loop standard fixes so a calibration
/// that is wrong but self-consistent still fails.
#[test]
fn a_reading_is_calibrated_filtered_and_acted_on() {
    // ANCHOR: example
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
    // ANCHOR_END: example
}
