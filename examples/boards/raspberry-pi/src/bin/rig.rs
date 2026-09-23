//! A pan and tilt head for a time-lapse: a tilt servo and a status LED on a PCA9685 board on
//! the header's I2C bus, and a 28BYJ-48 pan motor on four GPIO lines through a ULN2003 board.
//!
//! Wire the PCA9685 board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, and turn
//! the I2C interface on. Plug the servo into channel 0 and an LED into channel 15, and feed the
//! board's V+ from a 5 V supply whose ground is joined to the Pi's. Wire the ULN2003 board's IN1
//! to IN4 to GPIO5, GPIO6, GPIO13, and GPIO26, and its power pins to the same supply. Then run
//! `cargo run --release --bin rig`. See docs/guides/actuators.md.

use std::error::Error;
use std::thread;
use std::time::Duration;

// ANCHOR: example
use pamoja_actuators::pca9685::{self, Pca9685, Pwm};
use pamoja_actuators::stepper::{steps_for_degrees, Drive, FourWire};
use pamoja_hal::bus::I2cBus;
use pamoja_hal::digital::PinState;
use pamoja_hal::linux;

/// The GPIO chip the header's lines live on, numbered as the BCM numbers.
const CHIP: &str = "/dev/gpiochip0";

/// The lines the ULN2003 board's IN1 to IN4 are wired to, coils A to D.
const PAN_LINES: [u32; 4] = [5, 6, 13, 26];

/// The PCA9685 channels the tilt servo and the LED are plugged into.
const TILT: u8 = 0;
const STATUS: u8 = 15;

/// The shoot: twelve frames across a 90-degree pan, one every five seconds.
const FRAMES: u32 = 12;
const SWEEP_DEGREES: f32 = 90.0;
const INTERVAL: Duration = Duration::from_secs(5);

fn main() -> Result<(), Box<dyn Error>> {
    // The PCA9685 on the header's I2C bus, at 50 Hz for the servo. The first channel written
    // runs the datasheet's start-up: sleep, prescale, wake, 500 us, restart.
    let bus = I2cBus::open("/dev/i2c-1")?;
    let mut controller =
        Pca9685::new(bus.clone(), pca9685::DEFAULT_I2C_ADDRESS, bus.delay()).with_frequency(50);
    let glow = Pwm::duty(pca9685::COUNTS / 16);
    controller.set_channel(TILT, Pwm::servo(1_300, 50))?;
    controller.set_channel(STATUS, glow)?;

    // Each coil line is taken low, so the motor holds nothing until its first step. The
    // 28BYJ-48 turns 4096 half-steps a turn through its gearbox, and with a camera on it,
    // it steps every 4 ms.
    let open = |line| linux::output(CHIP, line, "pamoja-pan", PinState::Low);
    let coils = (
        open(PAN_LINES[0])?,
        open(PAN_LINES[1])?,
        open(PAN_LINES[2])?,
        open(PAN_LINES[3])?,
    );
    let mut pan = FourWire::new(coils, linux::delay(), Drive::HalfStep).with_step_interval(4_000);
    let per_frame = steps_for_degrees(SWEEP_DEGREES / (FRAMES - 1) as f32, 4096);

    // The LED lights while the head holds still for the camera, and glows while it moves.
    for frame in 1..=FRAMES {
        controller.set_channel(STATUS, Pwm::full_on())?;
        println!(
            "frame {frame:2}  pan {:6.2} degrees",
            pan.position() as f32 * 360.0 / 4096.0
        );
        thread::sleep(INTERVAL);
        controller.set_channel(STATUS, glow)?;
        if frame < FRAMES {
            pan.steps(per_frame)?;
        }
    }

    // Back to the start, then everything off: the coils dropped, every channel off in one
    // transfer, and the oscillator asleep.
    pan.steps(-pan.position())?;
    pan.idle()?;
    controller.set_all(Pwm::full_off())?;
    controller.sleep()?;
    println!("parked at {} half-steps", pan.position());
    Ok(())
}
// ANCHOR_END: example
