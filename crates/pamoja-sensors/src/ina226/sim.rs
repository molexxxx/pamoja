//! An INA226 that is not there.
//!
//! As with the [INA219](crate::ina219::sim), a current monitor's current and power registers
//! count in steps the calibration sets, so [`reporting`] takes the same shunt and largest
//! current a driver is given with [`with_shunt`](super::Ina226::with_shunt). The part carries
//! TI's manufacturer id and the INA226 die id, and its mask/enable register keeps the three
//! flags the part sets for itself, with the conversion-ready flag set, so every conversion
//! reads as finished. [`part`] carries 500 mA at 12 V through the 100 mΩ shunt sized for
//! 3.2 A a driver starts with.

use pamoja_hal::sim::WordPart;

use super::{
    bus_register, current_register, minimum_current_lsb_microamps, power_register, register,
    shunt_register, MaskEnable, DEVICE_ID, MANUFACTURER_ID,
};

/// The shunt [`part`] sits across, in milliohms.
pub const SHUNT_MILLIOHMS: u32 = 100;

/// The largest current [`part`] is sized for, in microamps.
pub const MAX_MICROAMPS: u32 = 3_200_000;

/// The bus voltage [`part`] reports, in microvolts.
pub const BUS_MICROVOLTS: u32 = 12_000_000;

/// The current [`part`] reports, in microamps.
pub const MICROAMPS: i32 = 500_000;

/// An INA226 answering at an address, carrying [`MICROAMPS`] at [`BUS_MICROVOLTS`] through
/// [`SHUNT_MILLIOHMS`].
///
/// # Arguments
///
/// * `address` - the address its A1 and A0 pins select.
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
        BUS_MICROVOLTS,
        MICROAMPS,
    )
}

/// An INA226 that reads what it is asked to.
///
/// # Arguments
///
/// * `address` - the address it answers to.
/// * `shunt_milliohms` - the shunt, as the driver is told with `with_shunt`.
/// * `max_microamps` - the largest current, as the driver is told with `with_shunt`.
/// * `bus_microvolts` - the bus voltage it reports.
/// * `microamps` - the current it reports; negative flows the other way through the shunt.
///
/// # Returns
///
/// The part. Its readings land on the steps its registers count in: 1.25 mV of bus,
/// 2.5 µV of shunt, and the calibration's current step.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::DelayLog;
/// use pamoja_sensors::ina226::{sim, Ina226, BASE_ADDRESS};
///
/// // 24 V on the bus and 1 A through a 100 mΩ shunt, for a part calibrated to 3.2 A.
/// let part = sim::reporting(BASE_ADDRESS, 100, 3_200_000, 24_000_000, 1_000_000);
/// let mut monitor = Ina226::new(part, BASE_ADDRESS, DelayLog::new());
/// let reading = monitor.measure().expect("the part answers");
/// assert_eq!(reading.bus_microvolts(), 24_000_000);
/// ```
#[must_use]
pub fn reporting(
    address: u8,
    shunt_milliohms: u32,
    max_microamps: u32,
    bus_microvolts: u32,
    microamps: i32,
) -> WordPart {
    let lsb = minimum_current_lsb_microamps(max_microamps);
    let shunt_nanovolts = i64::from(microamps) * i64::from(shunt_milliohms);
    let microwatts = i64::from(bus_microvolts) * i64::from(microamps).abs() / 1_000_000;
    let quiet = MaskEnable::from_register(0);
    let flags = MaskEnable {
        alert_function_flag: true,
        conversion_ready_flag: true,
        math_overflow: true,
        ..quiet
    }
    .to_register();
    let ready = MaskEnable {
        conversion_ready_flag: true,
        ..quiet
    }
    .to_register();
    WordPart::new(address)
        .holding(register::MANUFACTURER_ID, MANUFACTURER_ID)
        .holding(register::DIE_ID, DEVICE_ID << 4)
        .holding(register::MASK_ENABLE, ready)
        .read_only(register::MASK_ENABLE, flags)
        .holding(
            register::SHUNT_VOLTAGE,
            shunt_register(shunt_nanovolts as i32) as u16,
        )
        .holding(register::BUS_VOLTAGE, bus_register(bus_microvolts))
        .holding(register::CURRENT, current_register(microamps, lsb) as u16)
        .holding(register::POWER, power_register(microwatts as u32, lsb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ina226::{identify, Ina226};
    use pamoja_hal::script::DelayLog;

    #[test]
    fn the_part_identifies_as_an_ina226() {
        let part = part(0x40);
        assert!(identify(
            part.word(register::MANUFACTURER_ID),
            part.word(register::DIE_ID)
        )
        .is_ok());
    }

    #[test]
    fn a_driver_with_the_same_shunt_reads_the_load() {
        let mut monitor = Ina226::new(part(0x40), 0x40, DelayLog::new());
        let reading = monitor.measure().expect("a reading");
        let step = i64::from(minimum_current_lsb_microamps(MAX_MICROAMPS));
        assert_eq!(reading.bus_microvolts(), BUS_MICROVOLTS);
        assert!((i64::from(reading.current_microamps()) - i64::from(MICROAMPS)).abs() < step);

        let discharging = reporting(0x45, 2, 20_000_000, 3_300_000, -10_000_000);
        let mut monitor = Ina226::new(discharging, 0x45, DelayLog::new()).with_shunt(2, 20_000_000);
        let reading = monitor.measure().expect("a reading");
        let step = i64::from(minimum_current_lsb_microamps(20_000_000));
        assert!((i64::from(reading.current_microamps()) + 10_000_000).abs() < step);
        assert_eq!(reading.bus_microvolts(), 3_300_000);
    }
}
