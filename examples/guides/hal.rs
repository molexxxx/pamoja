//! The bus layer: a BME280 read through a part that answers from its registers, then the
//! same driver held to the exact conversation its datasheet prescribes.
//!
//! Run: `cargo run -p pamoja-examples --example hal`

use std::error::Error;

/// A part driven over a bus, with no part and no bus.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_core::Sensor;
    use pamoja_hal::script::{block_on, DelayLog, I2cScript, I2cStep};
    use pamoja_sensors::bme280::{
        register, sim, Bme280, CtrlHum, CtrlMeas, Mode, Oversampling, CHIP_ID, I2C_ADDRESS_PRIMARY,
        RESET_WORD,
    };

    // A BME280 that is not there. It holds a real part's calibration and answers from its
    // registers, so the driver runs with nothing plugged in and no transfers written out.
    // On a Raspberry Pi the bus is `pamoja_hal::linux::i2c("/dev/i2c-1")` and the two
    // lines below do not change.
    const BME280: u8 = I2C_ADDRESS_PRIMARY;

    // Standing one up costs a line, so this one is asked only what the driver wrote to it.
    // The datasheet requires humidity to be set before the mode register, and the part to be
    // left asleep until a measurement is forced. A bench would tell you that; so does this.
    let mut configured = Bme280::i2c(sim::part(BME280), BME280, DelayLog::new());
    configured.init().expect("the part identifies itself");
    let (written, _) = configured.release();
    let part = written.release();
    let asleep = part.register(register::CTRL_MEAS) & 0x03 == Mode::Sleep.code();
    println!(
        "configured   ctrl_hum {:#04x}, left asleep: {asleep}",
        part.register(register::CTRL_HUM)
    );

    // Another one, read the way a node reads it.
    let mut sensor = Bme280::i2c(sim::part(BME280), BME280, DelayLog::new());
    let measurement = block_on(sensor.read()).expect("the part answers");

    let celsius = measurement.celsius();
    let hectopascals = measurement.hectopascals();
    let humidity = measurement.relative_humidity_percent();
    println!("measured     {celsius:.2} C, {hectopascals:.2} hPa, {humidity:.2} %");

    // A part reads whatever it is asked to, which is how a program meets a reading it would
    // otherwise have to wait for weather, or a cold store, to produce.
    let cold = sim::reporting(BME280, 4.0, 1013.25, 80.0);
    let mut store = Bme280::i2c(cold, BME280, DelayLog::new());
    let chilled = block_on(store.read()).expect("the part answers");
    println!(
        "cold store   {:.2} C, {:.2} hPa, {:.2} %",
        chilled.celsius(),
        chilled.hectopascals(),
        chilled.relative_humidity_percent()
    );

    // The other half of the bus layer. A script plays one conversation and refuses anything
    // else, which is what proves a driver follows the datasheet rather than merely working:
    // the reset, the status read once the calibration image has loaded, the chip id, the two
    // calibration blocks, the three configuration writes in the order the part requires,
    // then one forced measurement.
    let settings = CtrlMeas {
        temperature: Oversampling::X1,
        pressure: Oversampling::X1,
        mode: Mode::Sleep,
    };
    let forced = CtrlMeas {
        mode: Mode::Forced,
        ..settings
    };
    let bus = I2cScript::new([
        I2cStep::write(BME280, [register::RESET, RESET_WORD]),
        I2cStep::write_read(BME280, [register::STATUS], [sim::STATUS_IDLE]),
        I2cStep::write_read(BME280, [register::CHIP_ID], [CHIP_ID]),
        I2cStep::write_read(BME280, [register::CALIB_TEMP_PRESS], sim::CALIBRATION),
        I2cStep::write_read(
            BME280,
            [register::CALIB_HUMIDITY],
            sim::CALIBRATION_HUMIDITY,
        ),
        I2cStep::write(BME280, [register::CONFIG, 0x00]),
        I2cStep::write(
            BME280,
            [
                register::CTRL_HUM,
                CtrlHum {
                    humidity: Oversampling::X1,
                }
                .bits(),
            ],
        ),
        I2cStep::write(BME280, [register::CTRL_MEAS, settings.bits()]),
        I2cStep::write(BME280, [register::CTRL_MEAS, forced.bits()]),
        I2cStep::write_read(BME280, [register::STATUS], [sim::STATUS_IDLE]),
        I2cStep::write_read(BME280, [register::DATA], sim::BURST),
    ]);

    let mut checked = Bme280::i2c(bus, BME280, DelayLog::new());
    let from_script = block_on(checked.read()).expect("the datasheet sequence runs");
    let (registers, delay) = checked.release();
    let bus = registers.release();
    let transfers = bus.consumed();
    let unexpected = !bus.done();
    println!("datasheet    {transfers} transfers, unexpected: {unexpected}");
    // ANCHOR_END: example

    assert_eq!(measurement.temperature_centi_celsius, 2044);
    assert_eq!(measurement.pressure_centi_pascals, 8_480_523);
    assert_eq!(measurement.humidity_q22_10, 45_725);
    assert_eq!(part.register(register::CTRL_HUM), 0x01);
    assert!(asleep);

    assert_eq!(chilled.celsius(), 4.0);
    assert!((chilled.hectopascals() - 1013.25).abs() < 0.01);
    assert!((chilled.relative_humidity_percent() - 80.0).abs() < 0.01);

    assert_eq!(from_script, measurement, "the same part, read two ways");
    assert_eq!(transfers, 11);
    assert!(!unexpected);
    assert_eq!(delay.total_micros(), 2_000 + 9_300);

    Ok(())
}
