//! An INA219 that is not there.
//!
//! A current monitor's current and power registers count in steps the calibration sets, and
//! the calibration comes from the shunt and the largest current a driver is told about. So
//! [`reporting`] takes the same shunt and largest current a driver is given with
//! [`with_shunt`](super::Ina219::with_shunt), and holds the registers a part across that load
//! reports once calibrated for them. [`part`] is the common breakout carrying 500 mA at 12 V,
//! with the 100 mΩ shunt sized for 3.2 A a driver starts with.

use pamoja_hal::sim::WordPart;

use super::{
    bus_register, current_register, minimum_current_lsb_microamps, power_register, register,
    shunt_register,
};

/// The shunt [`part`] sits across, in milliohms: the common breakout's.
pub const SHUNT_MILLIOHMS: u32 = 100;

/// The largest current [`part`] is sized for, in microamps.
pub const MAX_MICROAMPS: u32 = 3_200_000;

/// The bus voltage [`part`] reports, in millivolts.
pub const BUS_MILLIVOLTS: u32 = 12_000;

/// The current [`part`] reports, in microamps.
pub const MICROAMPS: i32 = 500_000;

/// The conversion-ready flag, CNVR, bit 1 of the bus voltage register.
const CONVERSION_READY: u16 = 1 << 1;

/// An INA219 answering at an address, carrying [`MICROAMPS`] at [`BUS_MILLIVOLTS`] through
/// [`SHUNT_MILLIOHMS`].
///
/// # Arguments
///
/// * `address` - the address it answers to, [`BASE_ADDRESS`](super::BASE_ADDRESS) plus what
///   its A1 and A0 pins add.
///
/// # Returns
///
/// The part, which a driver with its default shunt reads.
#[must_use]
pub fn part(address: u8) -> WordPart {
    reporting(
        address,
        SHUNT_MILLIOHMS,
        MAX_MICROAMPS,
        BUS_MILLIVOLTS,
        MICROAMPS,
    )
}

/// An INA219 that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `shunt_milliohms` - the shunt, as the driver is told with `with_shunt`.
/// * `max_microamps` - the largest current, as the driver is told with `with_shunt`.
/// * `bus_millivolts` - the bus voltage it reports.
/// * `microamps` - the current it reports; negative flows the other way through the shunt.
///
/// # Returns
///
/// The part. Its readings land on the steps its registers count in: 4 mV of bus, 10 µV of
/// shunt, and the calibration's current step.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::DelayLog;
/// use pamoja_sensors::ina219::{sim, Ina219, BASE_ADDRESS};
///
/// let part = sim::reporting(BASE_ADDRESS, 100, 3_200_000, 5_000, 250_000);
/// let mut monitor = Ina219::new(part, BASE_ADDRESS, DelayLog::new());
/// let reading = monitor.measure().expect("the part answers");
/// assert_eq!(reading.bus_millivolts(), 5_000);
/// ```
#[must_use]
pub fn reporting(
    address: u8,
    shunt_milliohms: u32,
    max_microamps: u32,
    bus_millivolts: u32,
    microamps: i32,
) -> WordPart {
    let lsb = minimum_current_lsb_microamps(max_microamps);
    let shunt_microvolts = i64::from(microamps) * i64::from(shunt_milliohms) / 1_000;
    let microwatts = i64::from(bus_millivolts) * i64::from(microamps).abs() / 1_000;
    WordPart::new(address)
        .holding(
            register::SHUNT_VOLTAGE,
            shunt_register(shunt_microvolts as i32) as u16,
        )
        .holding(
            register::BUS_VOLTAGE,
            bus_register(bus_millivolts) | CONVERSION_READY,
        )
        .holding(register::CURRENT, current_register(microamps, lsb) as u16)
        .holding(register::POWER, power_register(microwatts as u32, lsb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ina219::{conversion_ready, Ina219, BASE_ADDRESS};
    use pamoja_hal::script::DelayLog;

    #[test]
    fn the_flag_is_the_one_the_decoder_reads() {
        assert!(conversion_ready(CONVERSION_READY));
    }

    #[test]
    fn a_driver_with_the_same_shunt_reads_the_load() {
        let mut monitor = Ina219::new(part(BASE_ADDRESS), BASE_ADDRESS, DelayLog::new());
        let reading = monitor.measure().expect("a reading");
        let step = i64::from(minimum_current_lsb_microamps(MAX_MICROAMPS));
        assert_eq!(reading.bus_millivolts(), BUS_MILLIVOLTS);
        assert!((i64::from(reading.current_microamps()) - i64::from(MICROAMPS)).abs() < step);
        assert_eq!(reading.shunt_microvolts(), 50_000);

        let charging = reporting(0x41, 20, 8_000_000, 13_800, -2_500_000);
        let mut monitor = Ina219::new(charging, 0x41, DelayLog::new()).with_shunt(20, 8_000_000);
        let reading = monitor.measure().expect("a reading");
        let step = i64::from(minimum_current_lsb_microamps(8_000_000));
        assert!((i64::from(reading.current_microamps()) + 2_500_000).abs() < step);
        assert_eq!(reading.bus_millivolts(), 13_800);
    }
}
