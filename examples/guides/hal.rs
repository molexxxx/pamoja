//! The bus layer: a BME280 read through its driver over a scripted I2C bus, the same
//! conversation its datasheet prescribes and the one a gateway's `/dev/i2c-1` carries.
//!
//! Run: `cargo run -p pamoja-examples --example hal`

use std::error::Error;

/// A part driven over a bus that answers the way its datasheet says it does.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_core::Sensor;
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
    use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

    // On a Raspberry Pi the bus is `pamoja_hal::linux::i2c("/dev/i2c-1")` and nothing
    // below changes. Here a script plays the part's side: what a BME280 answers to the
    // reset, the status and chip id reads, the two calibration reads, the three
    // configuration writes, and one forced measurement, in the order the datasheet
    // lists them. A transfer the script does not expect is refused.
    const BME280: u8 = I2C_ADDRESS_PRIMARY;
    let calibration_a = [
        0x45, 0x6F, 0x6F, 0x68, 0x32, 0x00, 0x46, 0x91, 0x6A, 0xD6, 0xD0, 0x0B, 0x4E, 0x1E, 0x88,
        0xFF, 0xF9, 0xFF, 0xAC, 0x26, 0x0A, 0xD8, 0xBD, 0x10, 0x00, 0x4B,
    ];
    let calibration_b = [0x62, 0x01, 0x00, 0x15, 0x23, 0x03, 0x1E];
    let burst = [0x65, 0x5A, 0xC0, 0x7E, 0xED, 0x00, 0x75, 0x30];
    let bus = I2cScript::new([
        I2cStep::write(BME280, [0xE0, 0xB6]),
        I2cStep::write_read(BME280, [0xF3], [0x00]),
        I2cStep::write_read(BME280, [0xD0], [0x60]),
        I2cStep::write_read(BME280, [0x88], calibration_a),
        I2cStep::write_read(BME280, [0xE1], calibration_b),
        I2cStep::write(BME280, [0xF5, 0x00]),
        I2cStep::write(BME280, [0xF2, 0x01]),
        I2cStep::write(BME280, [0xF4, 0x24]),
        I2cStep::write(BME280, [0xF4, 0x25]),
        I2cStep::write_read(BME280, [0xF3], [0x00]),
        I2cStep::write_read(BME280, [0xF7], burst),
    ]);

    // The driver runs that sequence, checks the chip id, and keeps the calibration.
    let mut sensor = Bme280::i2c(bus, BME280, DelayLog::new());
    sensor.init().expect("the part answers");
    let calibrated = sensor.calibration().is_some();
    println!("calibration  read once: {calibrated}");

    // A reading is the whole measurement, compensated with that calibration.
    let measurement = block_on(sensor.read()).expect("a measurement");
    let celsius = measurement.celsius();
    let hectopascals = measurement.hectopascals();
    let humidity = measurement.relative_humidity_percent();
    println!("measured     {celsius:.2} C, {hectopascals:.2} hPa, {humidity:.2} %");

    // The script is spent: every transfer the datasheet lists was made, and no other.
    let (registers, delay) = sensor.release();
    let bus = registers.release();
    let transfers = bus.consumed();
    let unexpected = !bus.done();
    println!("bus          {transfers} transfers, unexpected: {unexpected}");
    // ANCHOR_END: example

    assert!(calibrated);
    assert_eq!(measurement.temperature_centi_celsius, 2044);
    assert_eq!(measurement.pressure_centi_pascals, 8_480_523);
    assert_eq!(measurement.humidity_q22_10, 45_725);
    assert_eq!(transfers, 11);
    assert!(!unexpected);
    assert_eq!(delay.total_micros(), 2_000 + 9_300);

    Ok(())
}
