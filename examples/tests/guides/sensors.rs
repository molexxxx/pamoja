//! The sensor-driver guide example; see docs/guides/sensors.md.

/// Two parts wired to the same node decoded from their register bytes, checked against the
/// values their datasheets publish, so the conversions are pinned rather than round-tripped
/// against themselves.
#[test]
fn a_thermometer_and_a_power_monitor_decode_their_registers() {
    // ANCHOR: example
    use pamoja_sensors::ds18b20::{self, Scratchpad};
    use pamoja_sensors::ina219;

    // Every Maxim 1-Wire part checks itself with CRC-8/MAXIM-DOW, whose published check
    // value over the ASCII digits 1 to 9 is 0xA1.
    assert_eq!(ds18b20::crc8(b"123456789"), 0xA1);

    // A DS18B20 answers a read with nine scratchpad bytes, the ninth that CRC over the
    // other eight, so a reading is verified before it is believed.
    let mut scratchpad = [0x91, 0x01, 0x4B, 0xF6, 0x7F, 0xFF, 0x0C, 0x10, 0x00];
    scratchpad[8] = ds18b20::crc8(&scratchpad[..8]);
    let reading = Scratchpad::parse(&scratchpad).expect("the CRC matches");

    // Register 0x0191 is the +25.0625 degree row of the datasheet's temperature table, each
    // count a sixteenth of a degree, so micro-degrees stay exact in integer arithmetic.
    assert_eq!(reading.raw_temperature(), 0x0191);
    assert_eq!(reading.temperature_micro_celsius(), 25_062_500);
    assert_eq!(reading.resolution().bits(), 12);
    assert_eq!(reading.alarm_high(), 75);

    // A bit flipped on a long 1-Wire run fails the CRC instead of arriving as a plausible
    // temperature a few degrees off.
    let mut corrupt = scratchpad;
    corrupt[0] ^= 0x01;
    assert!(Scratchpad::parse(&corrupt).is_err());

    // The INA219 datasheet's worked design example: 1 mA per count across a 2 milliohm
    // shunt calibrates to 0x5000, and its registers then read 11.98 V, 10 A, and 119.8 W.
    const CURRENT_LSB: u32 = 1_000;
    assert_eq!(ina219::calibration(CURRENT_LSB, 2), 0x5000);
    assert_eq!(ina219::bus_millivolts(0x5D98), 11_980);
    assert_eq!(ina219::current_microamps(0x2710, CURRENT_LSB), 10_000_000);
    assert_eq!(ina219::power_microwatts(0x1766, CURRENT_LSB), 119_800_000);
    // ANCHOR_END: example
}
