//! The inspection rover's arm: a shoulder servo and an elbow servo on a PCA9685 board on the
//! header's I2C bus, reaching for the controls on an inverter cabinet's panel.
//!
//! Wire the PCA9685 board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, and turn
//! the I2C interface on. Plug the shoulder servo into channel 0 and the elbow servo into channel
//! 1, and feed the board's V+ from a 5 V supply whose ground is joined to the Pi's. Then run
//! `cargo run --release --bin arm`. See docs/guides/motion.md.

use std::error::Error;
use std::thread;
use std::time::Duration;

// ANCHOR: example
use pamoja_actuators::pca9685::{self, Pca9685, Pwm};
use pamoja_hal::bus::I2cBus;
use pamoja_kit::{Elbow, ServoMap, TwoLinkArm};

/// The PCA9685 channels the shoulder and elbow servos are plugged into.
const SHOULDER_CHANNEL: u8 = 0;
const ELBOW_CHANNEL: u8 = 1;

/// The panel's controls, each in meters out from the shoulder and up from it.
const PANEL: [(&str, f32, f32); 4] = [
    ("reset button", 0.35, 0.20),
    ("breaker", 0.45, 0.10),
    ("door latch", 0.20, 0.35),
    ("fan switch", 0.70, 0.00),
];

fn main() -> Result<(), Box<dyn Error>> {
    // The PCA9685 on the header's I2C bus, at 50 Hz for the servos.
    let bus = I2cBus::open("/dev/i2c-1")?;
    let mut controller =
        Pca9685::new(bus.clone(), pca9685::DEFAULT_I2C_ADDRESS, bus.delay()).with_frequency(50);

    // Links of 0.30 m and 0.25 m. The servos were fitted with both joints at 0, so a joint
    // angle of 0 is each servo's center, 90 degrees.
    let arm = TwoLinkArm::new(0.30, 0.25);
    let servo = ServoMap::standard();
    let pulse = |joint: f32| u32::from(servo.pulse(90.0 + joint.to_degrees()));

    // Each control the arm can reach, it holds for two seconds; one it cannot, it skips rather
    // than drive a servo into its end stop.
    for (name, x, y) in PANEL {
        let Some((shoulder, elbow)) = arm.joints_for(x, y, Elbow::Up) else {
            println!("{name:12}  out of reach, skipped");
            continue;
        };
        controller.set_channel(SHOULDER_CHANNEL, Pwm::servo(pulse(shoulder), 50))?;
        controller.set_channel(ELBOW_CHANNEL, Pwm::servo(pulse(elbow), 50))?;
        println!(
            "{name:12}  shoulder {} us, elbow {} us",
            pulse(shoulder),
            pulse(elbow)
        );
        thread::sleep(Duration::from_secs(2));
    }

    // Back to the pose the arm was fitted in. The PCA9685 goes on sending both pulses after
    // the program exits, so the servos hold the arm there.
    controller.set_channel(SHOULDER_CHANNEL, Pwm::servo(pulse(0.0), 50))?;
    controller.set_channel(ELBOW_CHANNEL, Pwm::servo(pulse(0.0), 50))?;
    println!("parked        both servos at {} us", pulse(0.0));
    Ok(())
}
// ANCHOR_END: example
