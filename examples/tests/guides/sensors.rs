//! The sensor-driver guide example; see docs/guides/sensors.md.

/// Two parts on one battery node, each decoded the way its datasheet specifies. The
/// bytes they send are built by the same library that reads them, so a node can be
/// written and tested with nothing wired up.
#[test]
fn a_thermometer_and_a_power_monitor_decode_their_registers() {
    // ANCHOR: example
    use pamoja_sensors::ds18b20::{self, Resolution, Scratchpad};
    use pamoja_sensors::ina219;

    // Stand-ins for the two parts. On a running node the thermometer's nine bytes come
    // off the 1-Wire bus and the monitor's registers off I2C; here the library builds
    // what each would send, so the program runs with nothing plugged in.
    let thermometer = Scratchpad::new(
        ds18b20::temperature_from_celsius(25.0625, Resolution::Bits12),
        Resolution::Bits12,
        75,
        -10,
    )
    .to_bytes();

    // The monitor is set up for 1 mA per count across a 2 milliohm shunt, the load its
    // datasheet's worked design example describes: 11.98 V, 10 A, and 119.8 W.
    const CURRENT_LSB: u32 = 1_000;
    let bus = ina219::bus_register(11_980);
    let current = ina219::current_register(10_000_000, CURRENT_LSB);
    let power = ina219::power_register(119_800_000, CURRENT_LSB);

    // Everything below is the node's own code. The thermometer checksums every read, so
    // a reading is verified before it is believed.
    let reading = Scratchpad::parse(&thermometer).expect("the checksum matches");
    let celsius = reading.temperature_celsius();
    let bits = reading.resolution().bits();
    let (high, low) = (reading.alarm_high(), reading.alarm_low());
    println!("temperature  {celsius:.4} C");
    println!("resolution   {bits} bits");
    println!("alarms       {high} / {low} C");

    // The monitor computes nothing until it has been told what shunt it is across.
    let calibration = ina219::calibration(CURRENT_LSB, 2);
    let millivolts = ina219::bus_millivolts(bus);
    let milliamps = ina219::current_microamps(current, CURRENT_LSB) / 1_000;
    let milliwatts = ina219::power_microwatts(power, CURRENT_LSB) / 1_000;
    println!("calibration  {calibration:#06X}");
    println!("bus          {millivolts} mV");
    println!("current      {milliamps} mA");
    println!("power        {milliwatts} mW");

    // A bit flipped on a long 1-Wire run fails the checksum, so the node repeats the read
    // instead of logging a temperature a couple of degrees off.
    let mut corrupted = thermometer;
    corrupted[0] ^= 1;
    match Scratchpad::parse(&corrupted) {
        Ok(_) => println!("corrupt read accepted, which should never happen"),
        Err(error) => println!("corrupt read rejected: {error}"),
    }
    // ANCHOR_END: example

    assert_eq!(reading.raw_temperature(), 0x0191);
    assert_eq!(reading.temperature_micro_celsius(), 25_062_500);
    assert_eq!(reading.resolution().bits(), 12);
    assert_eq!(reading.alarm_high(), 75);
    assert_eq!(reading.alarm_low(), -10);
    assert!(Scratchpad::parse(&corrupted).is_err());

    // The datasheet's own figures for that design: calibration 0x5000, and registers
    // that read back 11.98 V, 10 A, and 119.8 W.
    assert_eq!(ina219::calibration(CURRENT_LSB, 2), 0x5000);
    assert_eq!(ina219::bus_millivolts(bus), 11_980);
    assert_eq!(ina219::current_microamps(current, CURRENT_LSB), 10_000_000);
    assert_eq!(ina219::power_microwatts(power, CURRENT_LSB), 119_800_000);

    // The published check value for CRC-8/MAXIM-DOW, the checksum every 1-Wire part
    // appends to what it sends.
    assert_eq!(ds18b20::crc8(b"123456789"), 0xA1);
}
