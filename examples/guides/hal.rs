//! The bus layer: one I2C bus a program and its drivers share, a BME280 read through a part
//! that answers from its registers, and the same driver held to the exact conversation its
//! datasheet prescribes.
//!
//! Run: `cargo run -p pamoja-examples --example hal`

use std::error::Error;

/// A part driven over a bus, with no part and no bus.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_hal::bus::I2cBus;
    use pamoja_hal::script::{I2cScript, I2cStep};
    use pamoja_hal::sim::I2cPart;
    use pamoja_sensors::bme280::{
        register, sim, Bme280, Config, CtrlHum, CtrlMeas, Mode, Oversampling, CHIP_ID,
        I2C_ADDRESS_PRIMARY, I2C_ADDRESS_SECONDARY, RESET_WORD,
    };
    use pamoja_sensors::DriverError;

    const BME280: u8 = I2C_ADDRESS_PRIMARY;

    // A bus with one part on it: a BME280 that is not there. It holds a real part's
    // calibration and one measurement that part took, and it answers from its registers, so
    // the driver runs its whole datasheet sequence against it. On a Raspberry Pi the bus is
    // `I2cBus::open("/dev/i2c-1")` and nothing after this line changes.
    let bus = I2cBus::simulated([sim::part(BME280)]);
    let mut sensor = Bme280::i2c(bus.clone(), BME280, bus.delay());

    // Reset, identify, calibrate, configure. The datasheet wants ctrl_hum written before
    // ctrl_meas, and the part left asleep until a measurement is forced. The part keeps what
    // the driver wrote, so the configuration reads back off the bus.
    sensor.init()?;
    let part: I2cPart = bus.part(BME280).ok_or("no part at the address")?;
    let humidity = CtrlHum::from_bits(part.register(register::CTRL_HUM)).humidity;
    let ctrl = CtrlMeas::from_bits(part.register(register::CTRL_MEAS));
    println!(
        "configured   humidity x{}, temperature x{}, pressure x{}, asleep: {}",
        humidity.factor(),
        ctrl.temperature.factor(),
        ctrl.pressure.factor(),
        ctrl.mode == Mode::Sleep
    );

    // One forced measurement. The driver waits the datasheet's longest measurement time for
    // these settings before it reads, and a simulated bus counts that wait rather than
    // sleeping through it.
    let reading = sensor.measure()?;
    println!(
        "measured     {:.2} C, {:.2} hPa, {:.2} %",
        reading.celsius(),
        reading.hectopascals(),
        reading.relative_humidity_percent()
    );
    println!(
        "waited       {:.2} ms across {} transfers",
        bus.waited_micros() as f64 / 1000.0,
        bus.transfers()
    );

    // A part reports whatever it is asked to. Putting one in the first one's place is how a
    // program meets a reading it would otherwise wait on the weather for, here a cold store
    // at four degrees, and the driver carries on without noticing.
    bus.attach(sim::reporting(BME280, 4.0, 1013.25, 80.0))?;
    let cold = sensor.measure()?;
    println!(
        "cold store   {:.2} C, {:.2} hPa, {:.2} %",
        cold.celsius(),
        cold.hectopascals(),
        cold.relative_humidity_percent()
    );

    // Nothing answers at the part's other address, and the driver says so rather than
    // returning a reading.
    match Bme280::i2c(bus.clone(), I2C_ADDRESS_SECONDARY, bus.delay()).init() {
        Ok(()) => println!("absent       a part answered"),
        Err(DriverError::Bus(error)) => println!("absent       {error}"),
        Err(error) => println!("absent       {error}"),
    }

    // The other half of the bus layer. A script plays one conversation and refuses anything
    // else, which proves a driver follows the datasheet rather than merely working: the
    // reset, the status once the calibration has loaded, the chip id, the two calibration
    // blocks, the three configuration writes in the order the part requires, then one forced
    // measurement.
    let settings = CtrlMeas {
        temperature: Oversampling::X1,
        pressure: Oversampling::X1,
        mode: Mode::Sleep,
    };
    let forced = CtrlMeas {
        mode: Mode::Forced,
        ..settings
    };
    let script = I2cBus::scripted(I2cScript::new([
        I2cStep::write(BME280, [register::RESET, RESET_WORD]),
        I2cStep::write_read(BME280, [register::STATUS], [sim::STATUS_IDLE]),
        I2cStep::write_read(BME280, [register::CHIP_ID], [CHIP_ID]),
        I2cStep::write_read(BME280, [register::CALIB_TEMP_PRESS], sim::CALIBRATION),
        I2cStep::write_read(
            BME280,
            [register::CALIB_HUMIDITY],
            sim::CALIBRATION_HUMIDITY,
        ),
        I2cStep::write(BME280, [register::CONFIG, Config::default().bits()]),
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
    ]));
    let checked = Bme280::i2c(script.clone(), BME280, script.delay()).measure()?;
    println!(
        "datasheet    {:.2} C after {} transfers, {} steps left",
        checked.celsius(),
        script.transfers(),
        script.remaining().unwrap_or_default()
    );
    // ANCHOR_END: example

    assert_eq!(reading.temperature_centi_celsius, 2044);
    assert_eq!(reading.pressure_centi_pascals, 8_480_523);
    assert_eq!(reading.humidity_q22_10, 45_725);
    assert_eq!(humidity, Oversampling::X1);
    assert_eq!(ctrl.mode, Mode::Sleep);

    assert_eq!(cold.celsius(), 4.0);
    assert!((cold.hectopascals() - 1013.25).abs() < 0.01);
    assert!((cold.relative_humidity_percent() - 80.0).abs() < 0.01);
    assert_eq!(
        bus.waited_micros(),
        2_000 + 2 * 9_300,
        "the start-up time and two measurements; the absent part failed before any wait"
    );

    assert_eq!(checked, reading, "the same part, read two ways");
    assert_eq!(script.transfers(), 11);
    assert_eq!(script.remaining(), Some(0));

    Ok(())
}
