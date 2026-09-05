//! Generated Node bindings for on-board bus addressing and pin logic.
//!
//! These mirror the `pamoja-gpio` Rust API: I2C addressing per NXP UM10204, the
//! four SPI clock modes, and the pin model that maps a logical "asserted" onto a
//! physical level. Everything here is pure arithmetic over small values, so
//! nothing holds state and nothing needs releasing.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_gpio::i2c::{self, Address, Direction};
use pamoja_gpio::pin::{Edge, Level, Polarity};
use pamoja_gpio::spi::Mode;
use pamoja_gpio::GpioError;

/// The physical voltage level on a pin.
#[napi(string_enum)]
pub enum PinLevel {
    /// A low level, near ground.
    Low,
    /// A high level, near the supply voltage.
    High,
}

/// The signal transition that triggers a pin interrupt.
#[napi(string_enum)]
pub enum PinEdge {
    /// A low-to-high transition.
    Rising,
    /// A high-to-low transition.
    Falling,
    /// Either transition.
    Both,
}

/// Whether a signal is asserted by a high or a low physical level.
#[napi(string_enum)]
pub enum PinPolarity {
    /// A high level means asserted.
    ActiveHigh,
    /// A low level means asserted, the wiring of most buttons and relay boards.
    ActiveLow,
}

/// The clock polarity and phase pair an SPI mode number names.
#[napi(object)]
pub struct SpiClock {
    /// Whether the clock idles high (CPOL = 1), which is modes 2 and 3.
    pub cpol: bool,
    /// Whether data is sampled on the trailing edge (CPHA = 1), which is modes 1 and 3.
    pub cpha: bool,
}

/// Returns the address bytes a controller puts on the bus for a transfer.
///
/// A 7-bit address frames as the single byte `(address << 1) | r/w`; a 10-bit one
/// frames as two, the reserved `11110` prefix carrying the top two bits and the
/// read/write bit, then the low eight.
#[napi(js_name = "i2cAddressFrame")]
pub fn i2c_address_frame(address: u16, ten_bit: bool, read: bool) -> napi::Result<Buffer> {
    let address = validate(address, ten_bit)?;
    let direction = if read {
        Direction::Read
    } else {
        Direction::Write
    };
    let mut out = [0u8; 2];
    let written = address.write_frame(direction, &mut out).map_err(to_napi)?;
    Ok(out[..written].into())
}

/// The lowest 7-bit address the I2C specification keeps for itself.
#[napi]
pub const I2C_RESERVED_FROM: u8 = i2c::RESERVED_FROM;

/// The first 7-bit address above the reserved block at the bottom of the range.
#[napi]
pub const I2C_RESERVED_BELOW: u8 = i2c::RESERVED_BELOW;

/// Reports whether a 7-bit address falls in a range the I2C specification reserves.
///
/// UM10204 reserves `0x00..=0x07` and `0x78..=0x7F`, leaving `0x08..=0x77` for
/// ordinary devices. A 10-bit address is never reserved in this sense.
#[napi(js_name = "i2cAddressIsReserved")]
pub fn i2c_address_is_reserved(address: u16, ten_bit: bool) -> napi::Result<bool> {
    Ok(validate(address, ten_bit)?.is_reserved())
}

/// Reports whether an address is the general call address `0x00`, the broadcast
/// every device on the bus listens to.
#[napi(js_name = "i2cAddressIsGeneralCall")]
pub fn i2c_address_is_general_call(address: u16, ten_bit: bool) -> napi::Result<bool> {
    Ok(validate(address, ten_bit)?.is_general_call())
}

/// Returns how many bytes an address frame occupies: one for a 7-bit address, two
/// for a 10-bit one.
#[napi(js_name = "i2cAddressFrameLen")]
pub fn i2c_address_frame_len(address: u16, ten_bit: bool) -> napi::Result<u32> {
    Ok(validate(address, ten_bit)?.frame_len() as u32)
}

/// Returns the `(CPOL, CPHA)` pair an SPI mode number names.
#[napi]
pub fn spi_mode_clock(mode: u8) -> napi::Result<SpiClock> {
    let mode = Mode::from_number(mode)
        .ok_or_else(|| napi::Error::from_reason("SPI mode must be 0, 1, 2, or 3"))?;
    let (cpol, cpha) = mode.cpol_cpha();
    Ok(SpiClock { cpol, cpha })
}

/// Returns the SPI mode number a `(CPOL, CPHA)` pair names.
#[napi]
pub fn spi_mode_from_clock(cpol: bool, cpha: bool) -> u8 {
    Mode::from_cpol_cpha(cpol, cpha).number()
}

/// Returns the opposite level.
#[napi]
pub fn pin_level_inverted(level: PinLevel) -> PinLevel {
    Level::from(level).inverted().into()
}

/// Returns the level a boolean names.
#[napi]
pub fn pin_level_from_bool(high: bool) -> PinLevel {
    Level::from_bool(high).into()
}

/// Reports whether a change from one level to another fires an interrupt trigger.
#[napi]
pub fn pin_edge_triggered_by(edge: PinEdge, from: PinLevel, to: PinLevel) -> bool {
    Edge::from(edge).triggered_by(from.into(), to.into())
}

/// Returns the physical level that represents a logical state under a polarity.
#[napi]
pub fn pin_polarity_level(polarity: PinPolarity, asserted: bool) -> PinLevel {
    Polarity::from(polarity).level(asserted).into()
}

/// Reports whether a physical level means the signal is asserted.
#[napi]
pub fn pin_polarity_is_asserted(polarity: PinPolarity, level: PinLevel) -> bool {
    Polarity::from(polarity).is_asserted(level.into())
}

impl From<PinLevel> for Level {
    fn from(value: PinLevel) -> Self {
        match value {
            PinLevel::Low => Level::Low,
            PinLevel::High => Level::High,
        }
    }
}

impl From<Level> for PinLevel {
    fn from(value: Level) -> Self {
        match value {
            Level::Low => PinLevel::Low,
            Level::High => PinLevel::High,
        }
    }
}

impl From<PinEdge> for Edge {
    fn from(value: PinEdge) -> Self {
        match value {
            PinEdge::Rising => Edge::Rising,
            PinEdge::Falling => Edge::Falling,
            PinEdge::Both => Edge::Both,
        }
    }
}

impl From<PinPolarity> for Polarity {
    fn from(value: PinPolarity) -> Self {
        match value {
            PinPolarity::ActiveHigh => Polarity::ActiveHigh,
            PinPolarity::ActiveLow => Polarity::ActiveLow,
        }
    }
}

/// Validates an address of the given width, rejecting one out of range.
fn validate(address: u16, ten_bit: bool) -> napi::Result<Address> {
    if ten_bit {
        Address::ten_bit(address).map_err(to_napi)
    } else {
        u8::try_from(address)
            .map_err(|_| napi::Error::from_reason(GpioError::AddressOutOfRange.to_string()))
            .and_then(|value| Address::seven_bit(value).map_err(to_napi))
    }
}

/// Maps an addressing error onto a thrown exception.
fn to_napi(error: GpioError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
