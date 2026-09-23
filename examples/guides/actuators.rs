//! The actuator-driver guide example: a motion-control time-lapse rig, with a PCA9685 holding
//! the tilt servo and the status LED, a slider behind an A4988, and a pan head on a 28BYJ-48;
//! see docs/guides/actuators.md.
//!
//! Run: `cargo run -p pamoja-examples --example actuators`

use std::error::Error;

/// A camera rig that tilts, slides, and pans between frames, with nothing plugged in.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_actuators::pca9685::{self, mode1, register, Pca9685, Pwm, INTERNAL_OSC_HZ};
    use pamoja_actuators::stepper::{steps_for_degrees, Drive, FourWire, StepDir};
    use pamoja_actuators::DriverError;
    use pamoja_hal::bus::I2cBus;
    use pamoja_hal::digital::PinState;
    use pamoja_hal::i2c::I2c;
    use pamoja_hal::script::{DelayLog, PinScript};
    use pamoja_hal::sim::I2cPart;

    // Where the rig's parts connect: the PCA9685 answers at 0x40 with its six address pins
    // low, the tilt servo is on its channel 0, and the status LED on channel 15.
    let address = pca9685::DEFAULT_I2C_ADDRESS;
    const TILT: u8 = 0;
    const STATUS: u8 = 15;
    let glow = Pwm::duty(pca9685::COUNTS / 16);

    // The rig with nothing plugged in: a PCA9685 that powers up and keeps its datasheet's
    // rules the way the part does. On a Raspberry Pi the bus is I2cBus::open("/dev/i2c-1")
    // and nothing after this statement changes.
    let bus = I2cBus::simulated([pca9685::sim::part(address)]);

    // A hobby servo wants a pulse every 20 ms, 50 Hz. The driver works the prescale out from
    // that and writes it with the oscillator asleep, since only then does the part take it,
    // then wakes the oscillator and waits the 500 us it needs to settle.
    let mut controller = Pca9685::new(bus.clone(), address, bus.delay()).with_frequency(50);
    controller.init()?;
    println!(
        "controller   prescale {} for {:.1} Hz, awake after {} us",
        controller.prescale(),
        controller.frequency(),
        bus.waited_micros()
    );

    // A servo turns to the width of the pulse it is sent, and this one points the camera a
    // little below level at 1300 us. The LED glows at a sixteenth of full brightness while
    // the rig waits. Reading the channels back shows what the part now holds.
    controller.set_channel(TILT, Pwm::servo(1_300, 50))?;
    controller.set_channel(STATUS, glow)?;
    let tilt = controller.channel(TILT)?;
    let status = controller.channel(STATUS)?;
    println!(
        "tilt         1300 us pulse, low at count {} of {}",
        tilt.off(),
        pca9685::COUNTS
    );
    println!(
        "status       glowing, high for {} of {} counts",
        status.off(),
        pca9685::COUNTS
    );

    // The slider: a 1.8-degree motor, 200 steps a turn, behind an A4988 with MS1 to MS3
    // high, which splits each step into sixteen. A 20-tooth GT2 pulley pulls 40 mm of belt
    // a turn, so 5 mm between frames is an eighth of a turn.
    const SLIDER_STEPS_PER_TURN: u32 = 200 * 16;
    const BELT_MM_PER_TURN: f32 = 40.0;
    let slide = steps_for_degrees(360.0 * 5.0 / BELT_MM_PER_TURN, SLIDER_STEPS_PER_TURN);
    let mut slider = StepDir::new(PinScript::new([]), PinScript::new([]), DelayLog::new())
        .with_step_interval(500);

    // The pan head: a 28BYJ-48 through a ULN2003, half-stepped, 4096 half-steps a turn
    // through its gearbox. With a camera on it, it steps every 4 ms, half the 500 Hz its
    // pack is rated to start at with no load.
    const PAN_STEPS_PER_TURN: u32 = 4096;
    let pan_step = steps_for_degrees(2.0, PAN_STEPS_PER_TURN);
    let coils = (
        PinScript::new([]),
        PinScript::new([]),
        PinScript::new([]),
        PinScript::new([]),
    );
    let mut pan = FourWire::new(coils, DelayLog::new(), Drive::HalfStep).with_step_interval(4_000);

    // Four frames. The LED lights for each exposure, and between frames the rig slides and
    // pans while it glows. The stepper lines record every level, and the delays count every
    // wait without sleeping through it.
    for frame in 1..=4 {
        controller.set_channel(STATUS, Pwm::full_on())?;
        println!(
            "frame {frame}      slider {:.1} mm, pan {:.2} degrees",
            slider.position() as f32 * BELT_MM_PER_TURN / SLIDER_STEPS_PER_TURN as f32,
            pan.position() as f32 * 360.0 / PAN_STEPS_PER_TURN as f32
        );
        controller.set_channel(STATUS, glow)?;
        if frame < 4 {
            slider.steps(slide)?;
            pan.steps(pan_step)?;
        }
    }

    // A four-wire motor draws current for as long as its coils hold, so the pan head
    // drops them once the shoot is over.
    pan.idle()?;
    let slid = slider.position();
    let panned = pan.position();
    let ((step_line, direction_line), slider_delay) = slider.release();
    let ((a, b, c, d), pan_delay) = pan.release();
    let pulses = step_line
        .driven()
        .iter()
        .filter(|level| **level == PinState::High)
        .count();
    println!(
        "slider       {pulses} pulses on STEP, DIR {:?}",
        direction_line.level()
    );
    println!(
        "pan head     {panned} half-steps, coils {:?} {:?} {:?} {:?}",
        a.level(),
        b.level(),
        c.level(),
        d.level()
    );
    println!(
        "moving       slider {} ms, pan head {} ms",
        slider_delay.total_millis(),
        pan_delay.total_millis()
    );

    // The part takes a new prescale only while its oscillator sleeps. Written while it
    // runs, as a driver that skipped the sleep would write it, the value is dropped and the
    // servos stay at 50 Hz.
    let mut raw = bus.clone();
    let fast = pca9685::prescale_for_frequency(1_000, INTERNAL_OSC_HZ);
    raw.write(address, &[register::PRE_SCALE, fast])?;
    let held: I2cPart = bus.part(address).ok_or("the controller left the bus")?;
    println!(
        "prescale     written while awake, still {}",
        held.register(register::PRE_SCALE)
    );

    // A channel the part does not have is refused before anything reaches the bus.
    match controller.set_channel(16, Pwm::full_on()) {
        Err(DriverError::Command(reason)) => println!("channel 16   {reason}"),
        other => println!("channel 16   {other:?}, which should never happen"),
    }

    // The shoot is over: every channel off in one transfer through the ALL_LED registers,
    // then the oscillator asleep. The part keeps its registers while it sleeps.
    controller.set_all(Pwm::full_off())?;
    controller.sleep()?;
    let parked: I2cPart = bus.part(address).ok_or("the controller left the bus")?;
    let all_off = controller.channel(TILT)? == Pwm::full_off()
        && controller.channel(STATUS)? == Pwm::full_off();
    let asleep = parked.register(register::MODE1) & mode1::SLEEP != 0;
    println!(
        "parked       every channel {}, oscillator {}",
        if all_off { "off" } else { "still on" },
        if asleep { "asleep" } else { "running" }
    );
    // ANCHOR_END: example

    assert_eq!(controller.prescale(), 121, "round(25 MHz / 4096 / 50) - 1");
    assert_eq!(tilt, Pwm::servo(1_300, 50));
    assert_eq!(tilt.off(), 266, "1300 us of a 20 ms period in 4096 counts");
    assert_eq!(status.off(), 256);
    assert_eq!(slide, 400, "an eighth of 3200 microsteps");
    assert_eq!(pan_step, 23, "2 degrees of 4096 half-steps, rounded");
    assert_eq!(slid, 1_200);
    assert_eq!(panned, 69);
    assert_eq!(pulses, 1_200, "one rising edge a microstep");
    assert_eq!(direction_line.level(), PinState::High, "forward");
    assert_eq!(
        slider_delay.total_micros(),
        1_200 * (10 + 10 + 500),
        "direction setup, the pulse, and the step interval"
    );
    assert_eq!(pan_delay.total_micros(), 69 * 4_000);
    assert_eq!(held.register(register::PRE_SCALE), 121);
    assert_eq!(fast, 5, "round(25 MHz / 4096 / 1000) - 1");
    assert!(all_off && asleep);
    assert_eq!(
        parked.register(register::MODE1),
        mode1::AUTO_INCREMENT | mode1::SLEEP,
        "no channel was running, so RESTART stays clear"
    );
    assert!(
        (controller.frequency() - 50.0288).abs() < 1e-3,
        "25 MHz / (4096 * 122)"
    );
    assert_eq!(bus.waited_micros(), 500, "the oscillator's one start-up");

    Ok(())
}
