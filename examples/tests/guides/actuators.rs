//! The actuator-driver guide example; see docs/guides/actuators.md.

/// A servo bank on a PCA9685 and a stepper walked through its coil patterns, the two
/// things a node drives once it has decided what to do.
#[test]
fn a_servo_pulse_and_a_stepper_cycle() {
    // ANCHOR: example
    use pamoja_actuators::pca9685::{self, Pwm, INTERNAL_OSC_HZ};
    use pamoja_actuators::stepper::{steps_for_degrees, Direction, Drive, Sequencer};

    // A servo bank wants 50 Hz. The prescale register that produces it is derived from
    // the part's 25 MHz internal oscillator, so a caller names the rate it wants rather
    // than working the divider out.
    let prescale = pca9685::prescale_for_frequency(50, INTERNAL_OSC_HZ);
    let rate = pca9685::frequency_for_prescale(prescale, INTERNAL_OSC_HZ);
    println!("prescale  {prescale} gives {rate:.1} Hz");

    // Each channel owns four consecutive registers, so a whole channel is written in one
    // bus transaction rather than four.
    let first_register = pca9685::channel_register(3);
    println!("channel 3 starts at register {first_register:#04X}");

    // A centred hobby servo holds its output high for 1500 us of the 20 ms period. The
    // part counts in 4096 steps per period, so that is where the pulse ends.
    let centred = Pwm::servo(1500, 50);
    println!("centred servo goes low at count {} of 4096", centred.off());

    // Fully off carries its own flag rather than a zero duty, which would still hold the
    // output high for the first count of every period.
    let flagged = Pwm::full_off().off() != Pwm::duty(0).off();
    println!("full off flag set: {flagged}");

    // A stepper is driven by walking a pattern of coil states. Half-step drive
    // interleaves the one-coil and two-coil patterns, so it has twice as many.
    let mut motor = Sequencer::new(Drive::HalfStep);
    let at_rest = motor.coils();
    println!("coils     {at_rest:04b} at rest");
    for _ in 0..2 {
        let coils = motor.step(Direction::Forward);
        println!("coils     {coils:04b} after a step");
    }

    // The patterns wrap, so the motor runs indefinitely either way, and an angle converts
    // to whole steps: a quarter turn of a 1.8-degree motor is fifty of them.
    for _ in 2..Drive::HalfStep.step_count() {
        motor.step(Direction::Forward);
    }
    let wrapped = motor.coils();
    let quarter_turn = steps_for_degrees(90.0, 200);
    println!("coils     {wrapped:04b} back at the start of the cycle");
    println!("a quarter turn is {quarter_turn} steps");
    // ANCHOR_END: example

    assert_eq!(prescale, 0x79);
    assert_eq!(first_register, 0x12);
    assert_eq!(centred.off(), 307);
    assert!(flagged);
    assert_eq!(motor.coils(), 0b1000);
    assert_eq!(steps_for_degrees(90.0, 200), 50);
}
