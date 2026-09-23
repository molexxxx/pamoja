//! An ADS1115 that is not there.
//!
//! A conversion comes back as a count of the full-scale range the gain selects, so
//! [`reporting`] takes the same gain a driver is given with
//! [`with_gain`](super::Ads1115::with_gain), and holds the count that voltage converts to. The
//! part keeps whatever configuration a driver writes, so a conversion a driver starts reads as
//! finished. [`part`] reads 1.65 V, half a 3.3 V supply, at the gain a driver starts with.

use pamoja_hal::sim::WordPart;

use super::{register, Config, Pga};

/// The voltage [`part`] reports.
pub const VOLTS: f32 = 1.65;

/// An ADS1115 answering at an address, reading [`VOLTS`] at the gain a driver starts with.
///
/// # Arguments
///
/// * `address` - the address its ADDR pin selects.
///
/// # Returns
///
/// The part.
#[must_use]
pub fn part(address: u8) -> WordPart {
    reporting(address, Config::default().pga, VOLTS)
}

/// An ADS1115 that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `pga` - the gain the driver converts at.
/// * `volts` - the voltage it reports, held to the gain's full-scale range.
///
/// # Returns
///
/// The part. Its reading lands on the nearest of the 32768 steps either side of zero the gain
/// divides its range into.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::DelayLog;
/// use pamoja_sensors::ads1115::{address, sim, Ads1115, Pga};
///
/// let mut adc = Ads1115::new(sim::reporting(address::GND, Pga::Fsr4_096, 3.0), address::GND, DelayLog::new())
///     .with_gain(Pga::Fsr4_096);
/// let sample = adc.sample().expect("the part answers");
/// assert!((sample.volts() - 3.0).abs() < 0.001);
/// ```
#[must_use]
pub fn reporting(address: u8, pga: Pga, volts: f32) -> WordPart {
    let full_scale = pga.full_scale_microvolts() as f32;
    let counts = volts * 1_000_000.0 / full_scale * 32_768.0;
    let raw = if counts >= 0.0 {
        (counts + 0.5).min(f32::from(i16::MAX))
    } else {
        (counts - 0.5).max(f32::from(i16::MIN))
    } as i16;
    WordPart::new(address)
        .holding(register::CONFIG, Config::default().bits())
        .holding(register::CONVERSION, raw as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ads1115::{address, Ads1115};
    use pamoja_hal::script::DelayLog;

    #[test]
    fn a_driver_at_the_same_gain_reads_the_voltage() {
        let mut adc = Ads1115::new(part(address::VDD), address::VDD, DelayLog::new());
        let sample = adc.sample().expect("a sample");
        assert!((sample.volts() - VOLTS).abs() < 0.001);

        let mut adc = Ads1115::new(
            reporting(address::SDA, Pga::Fsr0_256, -0.1),
            address::SDA,
            DelayLog::new(),
        )
        .with_gain(Pga::Fsr0_256);
        let sample = adc.sample().expect("a sample");
        assert!((sample.volts() + 0.1).abs() < 0.0001);
    }
}
