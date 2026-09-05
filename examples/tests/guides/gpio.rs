//! The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.

/// The three on-board interfaces a node uses before it reaches any network: addressing a
/// part on a bus, picking the clock mode its datasheet quotes, and driving a relay.
#[test]
fn addressing_a_bus_and_driving_a_pin() {
    // ANCHOR: example
    use pamoja_gpio::i2c::{Address, Direction};
    use pamoja_gpio::pin::{Edge, Level, Polarity};
    use pamoja_gpio::spi::Mode;

    // A BME280 answers at the 7-bit address its datasheet gives. That is not the byte
    // that goes on the wire: the address shifts up one and the low bit says whether this
    // transaction reads or writes, which is the step easiest to get wrong by hand.
    const BME280: u8 = 0x76;
    let sensor = Address::seven_bit(BME280).expect("a 7-bit address");
    let mut frame = [0u8; 2];
    sensor
        .write_frame(Direction::Write, &mut frame)
        .expect("one byte");
    println!("write to  {:#04X}", frame[0]);
    sensor
        .write_frame(Direction::Read, &mut frame)
        .expect("one byte");
    println!("read from {:#04X}", frame[0]);

    // The I2C specification keeps two ranges of addresses for itself, so a part answering
    // in either is a wiring mistake rather than a device.
    let sensor_reserved = sensor.is_reserved();
    let broadcast_reserved = Address::seven_bit(0x78).expect("in range").is_reserved();
    println!("{BME280:#04X} reserved: {sensor_reserved}, 0x78 reserved: {broadcast_reserved}");

    // A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
    // driver has to send a different number of bytes depending on the address it holds.
    let wide = Address::ten_bit(0x2A5).expect("a 10-bit address");
    println!("a 10-bit address takes {} bytes", wide.frame_len());

    // Datasheets quote clock polarity and phase as one mode number. Mode 3 idles the
    // clock high and samples on the trailing edge.
    let (idles_high, trailing_edge) = Mode::Mode3.cpol_cpha();
    println!("spi mode 3: idles high {idles_high}, samples on the trailing edge {trailing_edge}");

    // A relay board sold as active low energises when its pin is driven low. The polarity
    // carries that inversion, so no call site has to remember which way round it is.
    let energise = Polarity::ActiveLow.level(true);
    println!("to energise an active-low relay, drive the pin {energise:?}");

    // Releasing it drives the line back high, an edge a falling trigger ignores.
    let rising = Edge::Rising.triggered_by(Level::Low, Level::High);
    let falling = Edge::Falling.triggered_by(Level::Low, Level::High);
    println!("release seen by a rising trigger: {rising}, by a falling trigger: {falling}");
    // ANCHOR_END: example

    assert_eq!(frame[0], 0xED);
    assert!(!sensor_reserved);
    assert!(broadcast_reserved);
    assert_eq!(wide.frame_len(), 2);
    assert_eq!((idles_high, trailing_edge), (true, true));
    assert_eq!(Mode::from_cpol_cpha(true, false).number(), 2);
    assert_eq!(energise, Level::Low);
    assert!(Polarity::ActiveLow.is_asserted(Level::Low));
    assert!(rising);
    assert!(!falling);
}
