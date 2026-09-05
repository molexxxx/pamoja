//! NXP PCA9685 16-channel, 12-bit PWM controller.
//!
//! The PCA9685 drives sixteen independent PWM outputs at one shared frequency, which
//! is what makes it the usual way to run a bank of servos, dimmable LEDs, or the
//! speed and direction inputs of motor drivers from a single I2C device. This module
//! builds the values that program it: the prescaler that sets the output frequency,
//! the address of each channel's registers, and the 12-bit on/off word that sets a
//! channel's phase and duty, with the full-on and full-off encodings the datasheet
//! defines.
//!
//! It is pure logic: a caller writes the bytes to the device over whatever performs
//! the I2C transfers.

/// The PCA9685 register addresses.
pub mod register {
    /// Mode register 1.
    pub const MODE1: u8 = 0x00;
    /// Mode register 2.
    pub const MODE2: u8 = 0x01;
    /// I2C-bus subaddress 1.
    pub const SUBADR1: u8 = 0x02;
    /// I2C-bus subaddress 2.
    pub const SUBADR2: u8 = 0x03;
    /// I2C-bus subaddress 3.
    pub const SUBADR3: u8 = 0x04;
    /// LED All Call I2C-bus address.
    pub const ALLCALL_ADDR: u8 = 0x05;
    /// First register of channel 0 (LED0_ON_L); each channel spans four registers.
    pub const LED0_ON_L: u8 = 0x06;
    /// Low byte of the on-count applied to all channels at once.
    pub const ALL_LED_ON_L: u8 = 0xFA;
    /// High byte of the on-count applied to all channels at once.
    pub const ALL_LED_ON_H: u8 = 0xFB;
    /// Low byte of the off-count applied to all channels at once.
    pub const ALL_LED_OFF_L: u8 = 0xFC;
    /// High byte of the off-count applied to all channels at once.
    pub const ALL_LED_OFF_H: u8 = 0xFD;
    /// Prescaler that sets the PWM output frequency.
    pub const PRE_SCALE: u8 = 0xFE;
}

/// Bit masks for the MODE1 register.
pub mod mode1 {
    /// Restart logic.
    pub const RESTART: u8 = 0x80;
    /// Use the external clock pin instead of the internal oscillator.
    pub const EXTCLK: u8 = 0x40;
    /// Auto-increment the register pointer, needed to write a channel's four bytes in
    /// one transfer.
    pub const AUTO_INCREMENT: u8 = 0x20;
    /// Low-power sleep: the oscillator is off and the prescaler can be written.
    pub const SLEEP: u8 = 0x10;
    /// Respond to I2C-bus subaddress 1.
    pub const SUB1: u8 = 0x08;
    /// Respond to I2C-bus subaddress 2.
    pub const SUB2: u8 = 0x04;
    /// Respond to I2C-bus subaddress 3.
    pub const SUB3: u8 = 0x02;
    /// Respond to the LED All Call address.
    pub const ALLCALL: u8 = 0x01;
}

/// The frequency of the internal oscillator, 25 MHz.
pub const INTERNAL_OSC_HZ: u32 = 25_000_000;
/// The number of PWM channels.
pub const CHANNELS: u8 = 16;
/// The number of counts in one PWM period (12-bit resolution).
pub const COUNTS: u16 = 4096;
/// The power-on value of the PRE_SCALE register (0x1E), about 200 Hz at 25 MHz.
pub const PRE_SCALE_RESET: u8 = 0x1E;
/// The power-on value of MODE1 (0x11): sleeping, responding to the All Call address.
pub const MODE1_RESET: u8 = 0x11;

/// Returns the address of a channel's first register (its on-count low byte).
///
/// Each channel occupies four consecutive registers (on low, on high, off low, off
/// high), starting at `0x06` for channel 0, so channel `n` begins at `0x06 + 4 * n`.
///
/// # Arguments
///
/// * `channel` - the channel number, `0..=15`.
///
/// # Returns
///
/// The address of `LEDn_ON_L`. Channels past 15 are clamped to 15.
pub fn channel_register(channel: u8) -> u8 {
    let channel = if channel >= CHANNELS {
        CHANNELS - 1
    } else {
        channel
    };
    register::LED0_ON_L + 4 * channel
}

/// Computes the PRE_SCALE value for a desired PWM frequency.
///
/// This is the datasheet's prescale formula `round(osc_clock / (4096 * update_rate)) - 1`,
/// clamped to the hardware's `3..=255` range (which bounds the frequency to roughly
/// 24 Hz to 1.5 kHz at 25 MHz).
///
/// # Arguments
///
/// * `update_rate_hz` - the desired output frequency in hertz.
/// * `osc_hz` - the oscillator frequency, [`INTERNAL_OSC_HZ`] unless an external clock
///   is used.
///
/// # Returns
///
/// The value to write to the PRE_SCALE register.
pub fn prescale_for_frequency(update_rate_hz: u32, osc_hz: u32) -> u8 {
    let divisor = COUNTS as u32 * update_rate_hz.max(1);
    let rounded = (osc_hz + divisor / 2) / divisor;
    rounded.saturating_sub(1).clamp(3, 255) as u8
}

/// Computes the PWM frequency a PRE_SCALE value produces.
///
/// # Arguments
///
/// * `prescale` - the PRE_SCALE register value.
/// * `osc_hz` - the oscillator frequency.
///
/// # Returns
///
/// The output frequency in hertz.
pub fn frequency_for_prescale(prescale: u8, osc_hz: u32) -> f32 {
    osc_hz as f32 / (COUNTS as f32 * (prescale as f32 + 1.0))
}

/// A channel's PWM setting: when in the period it turns on and when it turns off.
///
/// The PCA9685 counts from 0 to 4095 each period and lets a channel turn on at one
/// count and off at another, so duty and phase are both programmable. The special
/// full-on and full-off states are encoded in a dedicated bit rather than as counts.
///
/// # Examples
///
/// ```
/// use pamoja_actuators::pca9685::Pwm;
///
/// // Half brightness with no phase delay: on at count 0, off at the midpoint.
/// let half = Pwm::duty(2048);
/// assert_eq!(half.bytes(), [0x00, 0x00, 0x00, 0x08]);
///
/// // Fully off is its own encoding, not a zero duty.
/// assert_eq!(Pwm::full_off().bytes(), [0x00, 0x00, 0x00, 0x10]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pwm {
    on: u16,
    off: u16,
}

impl Pwm {
    /// Builds a setting from explicit on and off counts.
    ///
    /// # Arguments
    ///
    /// * `on` - the count at which the output goes high, `0..=4095`.
    /// * `off` - the count at which it goes low, `0..=4095`.
    ///
    /// # Returns
    ///
    /// The PWM setting; counts are masked to 12 bits.
    pub fn from_counts(on: u16, off: u16) -> Pwm {
        Pwm {
            on: on & 0x0FFF,
            off: off & 0x0FFF,
        }
    }

    /// Builds a setting with no phase delay: on at count 0, off at `off`.
    ///
    /// # Arguments
    ///
    /// * `off` - the count at which the output goes low, which sets the duty cycle.
    ///
    /// # Returns
    ///
    /// The PWM setting.
    pub fn duty(off: u16) -> Pwm {
        Pwm::from_counts(0, off)
    }

    /// Builds the setting that drives a hobby servo to a given pulse width.
    ///
    /// A servo reads the high-pulse width each period; the count is that width as a
    /// fraction of the period, `pulse * 4096 / period`. Typical travel is about 1000
    /// to 2000 microseconds at a 50 Hz update rate.
    ///
    /// # Arguments
    ///
    /// * `pulse_micros` - the high-pulse width in microseconds.
    /// * `update_rate_hz` - the PWM frequency the controller is set to.
    ///
    /// # Returns
    ///
    /// The PWM setting for that pulse width.
    pub fn servo(pulse_micros: u32, update_rate_hz: u32) -> Pwm {
        let counts = (pulse_micros as u64 * COUNTS as u64 * update_rate_hz as u64) / 1_000_000;
        Pwm::duty(counts.min(COUNTS as u64 - 1) as u16)
    }

    /// The setting that holds the output continuously high.
    ///
    /// # Returns
    ///
    /// The full-on setting.
    pub fn full_on() -> Pwm {
        Pwm { on: 0x1000, off: 0 }
    }

    /// The setting that holds the output continuously low.
    ///
    /// # Returns
    ///
    /// The full-off setting, the power-on state of every channel.
    pub fn full_off() -> Pwm {
        Pwm { on: 0, off: 0x1000 }
    }

    /// Returns the four register bytes for this setting.
    ///
    /// The order is on-low, on-high, off-low, off-high, matching the channel's four
    /// consecutive registers; the full-on and full-off flags ride in bit 4 of the
    /// high bytes.
    ///
    /// # Returns
    ///
    /// `[on_low, on_high, off_low, off_high]`.
    pub fn bytes(self) -> [u8; 4] {
        [
            self.on as u8,
            (self.on >> 8) as u8,
            self.off as u8,
            (self.off >> 8) as u8,
        ]
    }

    /// Returns the count at which the output goes high.
    ///
    /// # Returns
    ///
    /// The on count, including the full-on flag in bit 12 when the output is held high.
    pub fn on(self) -> u16 {
        self.on
    }

    /// Returns the count at which the output goes low.
    ///
    /// # Returns
    ///
    /// The off count, including the full-off flag in bit 12 when the output is held low.
    pub fn off(self) -> u16 {
        self.off
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prescale_matches_the_datasheet_example() {
        // The datasheet's worked example: 200 Hz at 25 MHz gives prescale 0x1E.
        assert_eq!(prescale_for_frequency(200, INTERNAL_OSC_HZ), 0x1E);
        assert_eq!(
            prescale_for_frequency(200, INTERNAL_OSC_HZ),
            PRE_SCALE_RESET
        );
        // The documented bounds: 1526 Hz is the fastest (prescale 3), and the value
        // is clamped to the hardware minimum.
        assert_eq!(prescale_for_frequency(1526, INTERNAL_OSC_HZ), 3);
        assert_eq!(prescale_for_frequency(100_000, INTERNAL_OSC_HZ), 3);
        // A common 50 Hz servo rate.
        assert_eq!(prescale_for_frequency(50, INTERNAL_OSC_HZ), 0x79);
    }

    #[test]
    fn frequency_and_prescale_round_trip() {
        for prescale in [3u8, 30, 0x79, 255] {
            let freq = frequency_for_prescale(prescale, INTERNAL_OSC_HZ);
            assert_eq!(
                prescale_for_frequency(freq as u32, INTERNAL_OSC_HZ),
                prescale
            );
        }
    }

    #[test]
    fn channel_registers_are_four_apart() {
        assert_eq!(channel_register(0), 0x06);
        assert_eq!(channel_register(1), 0x0A);
        assert_eq!(channel_register(15), 0x42);
        // Past the last channel clamps rather than running into the prescaler.
        assert_eq!(channel_register(20), 0x42);
    }

    #[test]
    fn pwm_bytes_pack_counts_little_endian() {
        // On at 0x199 (409), off at 0xCCC (3276): the 10 %/90 % example shape.
        let pwm = Pwm::from_counts(0x199, 0xCCC);
        assert_eq!(pwm.bytes(), [0x99, 0x01, 0xCC, 0x0C]);
    }

    #[test]
    fn full_on_and_full_off_set_the_flag_bit() {
        assert_eq!(Pwm::full_on().bytes(), [0x00, 0x10, 0x00, 0x00]);
        assert_eq!(Pwm::full_off().bytes(), [0x00, 0x00, 0x00, 0x10]);
    }

    #[test]
    fn servo_midpoint_is_a_centred_pulse() {
        // 1500 µs at 50 Hz is 7.5 % of the 20 ms period: 0.075 * 4096 = 307 counts.
        assert_eq!(Pwm::servo(1500, 50), Pwm::duty(307));
        // The travel extremes.
        assert_eq!(Pwm::servo(1000, 50), Pwm::duty(204));
        assert_eq!(Pwm::servo(2000, 50), Pwm::duty(409));
    }
}
