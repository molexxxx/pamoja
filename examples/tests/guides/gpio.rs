//! The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.

/// The three on-board interfaces a node uses before it reaches any network: addressing a
/// part on a bus, picking the clock mode its datasheet quotes, and driving a relay.
#[test]
fn addressing_a_bus_and_driving_a_pin() {
    // ANCHOR: example
    use pamoja_gpio::i2c::{Address, Direction, RESERVED_FROM};
    use pamoja_gpio::pin::{Edge, Level, Polarity};
    use pamoja_gpio::spi::Mode;

    // A BME280 answers at the 7-bit address its datasheet gives. That is not the byte
    // that goes on the wire: the address shifts up one and the low bit says whether this
    // transaction reads or writes, which is the step easiest to get wrong by hand.
    const BME280: u8 = 0x76;
    let sensor = Address::seven_bit(BME280).expect("a 7-bit address");
    let to_write = sensor.frame(Direction::Write);
    let to_read = sensor.frame(Direction::Read);
    println!("write to  {:#04X}", to_write.as_bytes()[0]);
    println!("read from {:#04X}", to_read.as_bytes()[0]);

    // The I2C specification keeps two ranges of addresses for itself, so a part answering
    // in either is a wiring mistake rather than a device.
    let sensor_reserved = sensor.is_reserved();
    let prefix = Address::seven_bit(RESERVED_FROM).expect("in range");
    let prefix_reserved = prefix.is_reserved();
    println!("{BME280:#04X} reserved: {sensor_reserved}, {RESERVED_FROM:#04X} reserved: {prefix_reserved}");

    // A 10-bit address spends a reserved prefix over two bytes rather than one, so a bus
    // driver has to send a different number of bytes depending on the address it holds.
    // This is the worked example UM10204 itself prints.
    const TEN_BIT_DEVICE: u16 = 0x2A5;
    let wide = Address::ten_bit(TEN_BIT_DEVICE).expect("a 10-bit address");
    let wide_frame = wide.frame(Direction::Write);
    println!("a 10-bit address takes {} bytes", wide_frame.len());

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

    // The addressing bytes the I2C specification fixes are pinned in the crate's own
    // tests, so a guide asserts behaviour instead.
    assert_eq!(to_write.len(), 1);
    assert_ne!(to_write.as_bytes(), to_read.as_bytes());
    assert!(!sensor_reserved);
    assert!(prefix.is_reserved());
    assert_eq!(wide_frame.len(), 2);
    assert_eq!((idles_high, trailing_edge), (true, true));
    assert_eq!(Mode::from_cpol_cpha(true, false).number(), 2);
    assert_eq!(energise, Level::Low);
    assert!(Polarity::ActiveLow.is_asserted(Level::Low));
    assert!(rising);
    assert!(!falling);
}
