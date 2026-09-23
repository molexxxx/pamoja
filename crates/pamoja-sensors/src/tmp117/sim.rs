//! A TMP117 that is not there.
//!
//! [`part`] answers the way a TMP117 at 21.25 C does, and [`reporting`] at any temperature.
//! Its configuration register keeps the four flags the part sets for itself, HIGH_Alert,
//! LOW_Alert, Data_Ready, and EEPROM_Busy, whatever a driver writes there, with Data_Ready
//! set, so every conversion reads as finished.

use pamoja_hal::sim::WordPart;

use super::{raw_from_celsius, register, DEVICE_ID};

/// The temperature [`part`] reports.
pub const CELSIUS: f32 = 21.25;

/// The configuration register's flags, bits 15 to 12, which a write leaves alone.
const FLAGS: u16 = 0xF000;

/// Data_Ready, bit 13.
const DATA_READY: u16 = 1 << 13;

/// A TMP117 answering at an address, reading [`CELSIUS`].
///
/// # Arguments
///
/// * `address` - the address it answers to, 0x48 with ADD0 to ground.
///
/// # Returns
///
/// The part.
#[must_use]
pub fn part(address: u8) -> WordPart {
    reporting(address, CELSIUS)
}

/// A TMP117 that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `celsius` - the temperature it reports, which lands on the nearest 7.8125 m°C.
///
/// # Returns
///
/// The part.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::DelayLog;
/// use pamoja_sensors::tmp117::{sim, Tmp117};
///
/// let mut thermometer = Tmp117::new(sim::reporting(0x48, -18.5), 0x48, DelayLog::new());
/// let reading = thermometer.measure().expect("the part answers");
/// assert_eq!(reading.celsius(), -18.5);
/// ```
#[must_use]
pub fn reporting(address: u8, celsius: f32) -> WordPart {
    WordPart::new(address)
        .holding(register::DEVICE_ID, DEVICE_ID)
        .holding(register::CONFIGURATION, DATA_READY)
        .read_only(register::CONFIGURATION, FLAGS)
        .holding(register::TEMP_RESULT, raw_from_celsius(celsius) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmp117::{data_ready, eeprom_busy, Tmp117};
    use pamoja_hal::script::DelayLog;

    #[test]
    fn the_flags_are_the_ones_the_decoder_reads() {
        assert!(data_ready(DATA_READY));
        assert!(eeprom_busy(FLAGS));
    }

    #[test]
    fn the_driver_reads_the_temperature_the_part_holds() {
        let mut thermometer = Tmp117::new(part(0x48), 0x48, DelayLog::new());
        assert_eq!(thermometer.measure().expect("a reading").celsius(), CELSIUS);

        let mut cold = Tmp117::new(reporting(0x49, -40.0), 0x49, DelayLog::new());
        assert_eq!(cold.measure().expect("a reading").celsius(), -40.0);
    }
}
