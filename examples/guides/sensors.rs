//! The sensor-driver guide example: a greenhouse bench with every I2C part pamoja drives on
//! one bus, and a DS18B20 read the way Linux serves one; see docs/guides/sensors.md.
//!
//! Run: `cargo run -p pamoja-examples --example sensors`

use std::error::Error;

/// Ten sensors on one bench, each read by its own driver, with nothing plugged in.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_hal::bus::I2cBus;
    use pamoja_hal::sim::Part;
    use pamoja_sensors::ds18b20::{self, linux::Thermometer, Resolution, Scratchpad};
    use pamoja_sensors::ina226::AddressPin;
    use pamoja_sensors::{ads1115, bmp280, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117};

    // The address plan. Every part answers at the address its pins choose, and two parts on
    // one address garble each other, so a bench of nine is planned around the two that
    // cannot move: the HDC1080 and the SCD41 have one address each.
    let air = sht3x::I2C_ADDRESS_A;
    let pressure = bmp280::I2C_ADDRESS_SECONDARY;
    let light = opt3001::I2C_ADDRESS_SCL;
    let soil = ads1115::address::SDA;
    let enclosure = tmp117::address::ADD0_VPLUS;
    let panel = ina219::address(AddressPin::Ground, AddressPin::Supply);
    let battery = ina226::address(AddressPin::Supply, AddressPin::Supply);

    // The bench with nothing plugged in: each part answers the way its datasheet says, with
    // the reading it is given here, a warm and humid afternoon. A bus holds parts of three
    // kinds, and a `Part` is any of them. On a Raspberry Pi the bus is
    // `I2cBus::open("/dev/i2c-1")` and nothing after this statement changes.
    let bus = I2cBus::simulated::<Part>([
        sht3x::sim::reporting(air, 24.1, 62.0).into(),
        bmp280::sim::reporting(pressure, 24.1, 1003.2).into(),
        scd4x::sim::reporting(1_180, 24.1, 62.0).into(),
        opt3001::sim::reporting(light, 4_200.0).into(),
        ads1115::sim::reporting(soil, ads1115::Pga::Fsr4_096, 2.35).into(),
        tmp117::sim::reporting(enclosure, 31.25).into(),
        hdc1080::sim::reporting(31.25, 38.0).into(),
        ina219::sim::reporting(panel, 100, 3_200_000, 18_400, 1_250_000).into(),
        ina226::sim::reporting(battery, 2, 20_000_000, 12_800_000, -350_000).into(),
    ]);

    // One driver per part. Each holds its own share of the bus and a delay that sleeps only
    // when a real part is on the other end, and each runs its datasheet's whole conversation
    // on the first measurement: reset, identify, configure, convert, read.
    let air_now = sht3x::Sht3x::new(bus.clone(), air, bus.delay()).measure()?;
    println!(
        "air          {:.2} C, {:.2} %",
        air_now.temperature_celsius(),
        air_now.relative_humidity()
    );

    let weather = bmp280::Bmp280::i2c(bus.clone(), pressure, bus.delay()).measure()?;
    println!("pressure     {:.1} hPa", weather.hectopascals());

    let co2 = scd4x::Scd4x::new(bus.clone(), bus.delay()).measure()?;
    println!("co2          {} ppm", co2.co2_ppm);

    let sun = opt3001::Opt3001::new(bus.clone(), light, bus.delay()).measure()?;
    println!("light        {:.0} lux", sun.lux());

    // A capacitive probe's voltage falls as the soil wets and nears 3 V in dry soil, past
    // the ADS1115's default range of 2.048 V, so the driver is given the 4.096 V range.
    let probe = ads1115::Ads1115::new(bus.clone(), soil, bus.delay())
        .with_input(ads1115::Mux::Ain0Gnd)
        .with_gain(ads1115::Pga::Fsr4_096)
        .sample()?;
    println!("soil         {:.3} V", probe.volts());

    let case = tmp117::Tmp117::new(bus.clone(), enclosure, bus.delay()).measure()?;
    let case_air = hdc1080::Hdc1080::new(bus.clone(), bus.delay()).measure()?;
    println!(
        "enclosure    {:.2} C, {:.1} %",
        case.celsius(),
        case_air.relative_humidity()
    );

    // Two current monitors, each calibrated for its own shunt. The fans and the pump draw
    // more than the panel gives, so the battery makes up the rest and its current reads
    // negative: current through a shunt is signed by its direction.
    let charge = ina219::Ina219::new(bus.clone(), panel, bus.delay())
        .with_shunt(100, 3_200_000)
        .measure()?;
    println!(
        "panel        {:.2} V, {:.2} A, {:.2} W",
        f64::from(charge.bus_millivolts()) / 1e3,
        f64::from(charge.current_microamps()) / 1e6,
        f64::from(charge.power_microwatts()) / 1e6
    );
    let drain = ina226::Ina226::new(bus.clone(), battery, bus.delay())
        .with_shunt(2, 20_000_000)
        .measure()?;
    println!(
        "battery      {:.2} V, {:.2} A, {:.2} W",
        drain.bus_volts(),
        drain.current_amps(),
        drain.power_watts()
    );

    // Every driver waited as its datasheet asks, the OPT3001's 800 ms integration and the
    // SCD41's command times most of all. The simulated bus counted the waits and slept none.
    println!(
        "waited       {:.2} s across {} transfers",
        bus.waited_micros() as f64 / 1e6,
        bus.transfers()
    );

    // The same probe read at the default range. Past 2.048 V the converter pins at its top
    // code, and the sample says so rather than passing the edge of the range off as a reading.
    bus.attach(ads1115::sim::reporting(soil, ads1115::Pga::Fsr2_048, 2.35))?;
    let pinned = ads1115::Ads1115::new(bus.clone(), soil, bus.delay())
        .with_input(ads1115::Mux::Ain0Gnd)
        .sample()?;
    println!(
        "default gain {:.3} V, clipped: {}",
        pinned.volts(),
        pinned.clipped()
    );

    // A driver aimed at the wrong address meets whatever answers there. The TMP117 reads its
    // device id before anything else, so pointed at the soil probe's converter it refuses
    // rather than reporting that part's registers as a temperature.
    match tmp117::Tmp117::new(bus.clone(), soil, bus.delay()).init() {
        Ok(()) => println!("wrong part   accepted, which should never happen"),
        Err(error) => println!("wrong part   {error}"),
    }

    // A DS18B20 on a lead into a pot, read the way a Linux board reads one: the kernel's
    // 1-Wire driver serves each probe as a file under /sys/bus/w1/devices. The program writes
    // that file itself, with the text the kernel prints, so it runs anywhere.
    let devices = std::env::temp_dir().join(format!("pamoja-bench-{}", std::process::id()));
    let directory = devices.join(format!("{:02x}-000005e2fdc3", ds18b20::FAMILY_CODE));
    std::fs::create_dir_all(&directory)?;
    let scratchpad = Scratchpad::new(
        ds18b20::temperature_from_celsius(19.5, Resolution::Bits12),
        Resolution::Bits12,
        30,
        5,
    );
    std::fs::write(
        directory.join(ds18b20::linux::W1_SLAVE),
        ds18b20::w1_slave_text(&scratchpad),
    )?;
    let mut found = Vec::new();
    for thermometer in Thermometer::discover_in(&devices)? {
        let reading = thermometer.read_scratchpad()?;
        println!(
            "soil probe   {:.4} C, alarms at {} and {} C",
            reading.temperature_celsius(),
            reading.alarm_low(),
            reading.alarm_high()
        );
        found.push(reading);
    }
    std::fs::remove_dir_all(&devices)?;
    // ANCHOR_END: example

    assert!((air_now.temperature_celsius() - 24.1).abs() < 0.003);
    assert!((air_now.relative_humidity() - 62.0).abs() < 0.002);
    assert!((weather.hectopascals() - 1003.2).abs() < 0.01);
    assert_eq!(co2.co2_ppm, 1_180);
    assert!(
        (sun.lux() - 4_200.0).abs() < 1.28,
        "the step at that exponent"
    );
    assert_eq!(probe.raw, 18_800, "2.35 V at 125 uV a count");
    assert!(!probe.clipped());
    assert_eq!(case.celsius(), 31.25);
    assert!((case_air.relative_humidity() - 38.0).abs() < 0.002);
    assert_eq!(charge.bus_millivolts(), 18_400);
    assert!((charge.current_microamps() - 1_250_000).abs() < 98);
    assert!((drain.current_microamps() + 350_000).abs() < 611);
    assert!(pinned.clipped());
    assert_eq!(pinned.raw, i16::MAX, "the top code of Table 7-3");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].temperature_micro_celsius(), 19_500_000);
    assert_eq!(found[0], scratchpad);
    assert_eq!(
        panel, 0x41,
        "Table 1 of the INA219 datasheet: A1 to GND, A0 to VS+"
    );
    assert_eq!(battery, 0x45, "and of the INA226's: A1 and A0 to VS");

    // The decode half under every driver, anchored to its datasheets: the INA219's worked
    // design example calibrates 1 mA a count across 2 milliohms to 0x5000, and the 1-Wire
    // checksum gives the published CRC-8/MAXIM-DOW check value over the digits 1 to 9.
    assert_eq!(ina219::calibration(1_000, 2), 0x5000);
    assert_eq!(ds18b20::crc8(b"123456789"), 0xA1);

    Ok(())
}
