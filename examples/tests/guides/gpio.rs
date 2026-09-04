//! The I2C, SPI, and GPIO guide example; see docs/guides/gpio.md.

/// The three on-board interfaces a node uses before it reaches any network, checked
/// against the address bytes UM10204 fixes, the mode numbers datasheets quote, and the
/// level an active-low relay is switched by.
#[test]
fn addressing_a_bus_and_driving_a_pin() {
    // ANCHOR: example
    use pamoja_gpio::i2c::{Address, Direction};
    use pamoja_gpio::pin::{Edge, Level, Polarity};
    use pamoja_gpio::spi::Mode;

    // A BME280 answers at 7-bit address 0x76, which is not the byte that goes on the wire:
    // the address shifts up one and the read/write bit fills the low bit.
    let mut frame = [0u8; 2];
    let sensor = Address::seven_bit(0x76).expect("a 7-bit address");
    assert_eq!(sensor.write_frame(Direction::Write, &mut frame), Ok(1));
    assert_eq!(frame[0], 0xEC);
    assert_eq!(sensor.write_frame(Direction::Read, &mut frame), Ok(1));
    assert_eq!(frame[0], 0xED);

    // UM10204 keeps 0x00..=0x07 and 0x78..=0x7F for itself, so an address in either range
    // is a wiring mistake rather than a device.
    assert!(!sensor.is_reserved());
    assert!(Address::seven_bit(0x78).expect("in range").is_reserved());

    // A 10-bit address spends the reserved 11110 prefix over two bytes: the prefix, the top
    // two address bits, and the read/write bit, then the low eight bits.
    let wide = Address::ten_bit(0x2A5).expect("a 10-bit address");
    assert_eq!(wide.frame_len(), 2);
    assert_eq!(wide.write_frame(Direction::Write, &mut frame), Ok(2));
    assert_eq!(frame, [0xF4, 0xA5]);
    assert_eq!(wide.write_frame(Direction::Read, &mut frame), Ok(2));
    assert_eq!(frame, [0xF5, 0xA5]);

    // Datasheets quote clock polarity and phase as one mode number, (CPOL << 1) | CPHA, so
    // mode 3 idles the clock high and samples on the trailing edge.
    assert_eq!(Mode::Mode3.cpol_cpha(), (true, true));
    assert_eq!(Mode::from_cpol_cpha(true, false).number(), 2);

    // A relay board sold as active low energises when its pin is driven low. The polarity
    // carries that inversion so no call site has to remember it.
    assert_eq!(Polarity::ActiveLow.level(true), Level::Low);
    assert!(Polarity::ActiveLow.is_asserted(Level::Low));

    // Releasing the relay drives the line back high, an edge a falling trigger ignores.
    assert!(Edge::Rising.triggered_by(Level::Low, Level::High));
    assert!(!Edge::Falling.triggered_by(Level::Low, Level::High));
    // ANCHOR_END: example
}
