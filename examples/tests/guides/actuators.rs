//! The actuator-driver guide example; see docs/guides/actuators.md.

/// A servo pulse on a PCA9685 and a stepper walked through its coil patterns, checked
/// against the register values the datasheet fixes and the standard drive sequence, so the
/// encodings are pinned rather than round-tripped against themselves.
#[test]
fn a_servo_pulse_and_a_stepper_cycle() {
    // ANCHOR: example
    use pamoja_actuators::pca9685::{self, Pwm, INTERNAL_OSC_HZ};
    use pamoja_actuators::stepper::{steps_for_degrees, Direction, Drive, Sequencer};

    // The datasheet's worked example: 200 Hz off the 25 MHz internal oscillator is prescale
    // 0x1E, the value the part powers up holding. A servo bank wants 50 Hz instead.
    assert_eq!(pca9685::prescale_for_frequency(200, INTERNAL_OSC_HZ), 0x1E);
    assert_eq!(pca9685::prescale_for_frequency(50, INTERNAL_OSC_HZ), 0x79);

    // Each channel owns four consecutive registers from 0x06, so channel 3 starts at 0x12
    // and its four bytes go out in one bus transaction.
    assert_eq!(pca9685::channel_register(3), 0x12);

    // A centred hobby servo holds its output high for 1500 us, 7.5 % of the 20 ms period,
    // which is 307 of the 4096 counts. The bytes are on-low, on-high, off-low, off-high.
    assert_eq!(Pwm::servo(1500, 50).bytes(), [0x00, 0x00, 0x33, 0x01]);

    // Fully off carries its own bit rather than a zero duty, which still holds the output
    // high for the first count of every period.
    assert_eq!(Pwm::full_off().bytes(), [0x00, 0x00, 0x00, 0x10]);
    assert_eq!(Pwm::duty(0).bytes(), [0x00, 0x00, 0x00, 0x00]);

    // Half-step drive interleaves the one-coil and two-coil patterns; the most significant
    // of the four bits is the first coil.
    let mut motor = Sequencer::new(Drive::HalfStep);
    assert_eq!(motor.coils(), 0b1000);
    assert_eq!(motor.step(Direction::Forward), 0b1100);
    assert_eq!(motor.step(Direction::Forward), 0b0100);

    // The eight patterns of a cycle wrap, so the motor runs indefinitely either way, and an
    // angle converts to whole steps: a quarter turn of a 1.8-degree motor is 50 of them.
    for _ in 2..Drive::HalfStep.step_count() {
        motor.step(Direction::Forward);
    }
    assert_eq!(motor.coils(), 0b1000);
    assert_eq!(steps_for_degrees(90.0, 200), 50);
    // ANCHOR_END: example
}
