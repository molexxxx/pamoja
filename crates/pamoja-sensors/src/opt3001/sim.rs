//! An OPT3001 that is not there.
//!
//! [`part`] answers the way an OPT3001 reading 380 lux does, and [`reporting`] at any light
//! level. Its configuration register keeps the flags the part sets for itself, the overflow,
//! conversion-ready, high, and low flags, whatever a driver writes there, with the
//! conversion-ready flag set, so every conversion reads as finished.

use pamoja_hal::sim::WordPart;

use super::{raw_from_milli_lux, register, Configuration, DEVICE_ID, MANUFACTURER_ID};

/// The illuminance [`part`] reports.
pub const LUX: f32 = 380.0;

/// The configuration register's flags, which the part sets and a write leaves alone: the
/// overflow flag (bit 8), the conversion-ready flag (bit 7), and the high and low flags
/// (bits 6 and 5).
const FLAGS: u16 = 0x01E0;

/// The conversion-ready flag, bit 7.
const CONVERSION_READY: u16 = 1 << 7;

/// An OPT3001 answering at an address, reading [`LUX`].
///
/// # Arguments
///
/// * `address` - the address it answers to, from
///   [`I2C_ADDRESS_GND`](super::I2C_ADDRESS_GND) to
///   [`I2C_ADDRESS_SCL`](super::I2C_ADDRESS_SCL).
///
/// # Returns
///
/// The part.
#[must_use]
pub fn part(address: u8) -> WordPart {
    reporting(address, LUX)
}

/// An OPT3001 that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `lux` - the illuminance it reports, which lands on the nearest step the part's exponent
///   and mantissa can represent.
///
/// # Returns
///
/// The part.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::DelayLog;
/// use pamoja_sensors::opt3001::{sim, Opt3001, I2C_ADDRESS_GND};
///
/// let mut light = Opt3001::new(sim::reporting(I2C_ADDRESS_GND, 1200.0), I2C_ADDRESS_GND, DelayLog::new());
/// let reading = light.measure().expect("the part answers");
/// assert_eq!(reading.lux(), 1200.0);
/// ```
#[must_use]
pub fn reporting(address: u8, lux: f32) -> WordPart {
    let milli_lux = (lux.max(0.0) * 1000.0 + 0.5) as u32;
    WordPart::new(address)
        .holding(register::MANUFACTURER_ID, MANUFACTURER_ID)
        .holding(register::DEVICE_ID, DEVICE_ID)
        .holding(
            register::CONFIGURATION,
            Configuration::default().bits() | CONVERSION_READY,
        )
        .read_only(register::CONFIGURATION, FLAGS)
        .holding(register::RESULT, raw_from_milli_lux(milli_lux))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opt3001::{Opt3001, I2C_ADDRESS_VDD};
    use pamoja_hal::script::DelayLog;

    #[test]
    fn the_flags_are_the_ones_the_decoder_reads() {
        let flags = Configuration::from_bits(FLAGS);
        assert!(flags.overflow && flags.conversion_ready && flags.flag_high && flags.flag_low);
        assert!(Configuration::from_bits(CONVERSION_READY).conversion_ready);
        assert_eq!(
            Configuration::default().bits() & FLAGS,
            0,
            "a written configuration never carries a flag"
        );
    }

    #[test]
    fn the_driver_reads_the_light_the_part_holds() {
        let mut light = Opt3001::new(part(I2C_ADDRESS_VDD), I2C_ADDRESS_VDD, DelayLog::new());
        assert_eq!(light.measure().expect("a reading").lux(), LUX);

        let mut dim = Opt3001::new(
            reporting(I2C_ADDRESS_VDD, 0.64),
            I2C_ADDRESS_VDD,
            DelayLog::new(),
        );
        assert_eq!(dim.measure().expect("a reading").lux(), 0.64);
    }
}
