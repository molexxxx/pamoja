//! The C ABI for on-board bus addressing and pin logic.
//!
//! These functions wrap [`pamoja_gpio`] for callers that reach the SDK through
//! the flat C boundary: I2C addressing per NXP UM10204, the four SPI clock modes,
//! and the pin model that maps a logical "asserted" onto a physical level.
//!
//! Nothing here allocates or holds state, so nothing here is a handle. An I2C
//! address is two scalars and crosses by value as [`PamojaI2cAddress`]; the rest
//! are enumerations and small pure functions over them.

use pamoja_gpio::i2c::{Address, Direction};
use pamoja_gpio::pin::{Edge, Level, Polarity};
use pamoja_gpio::spi::Mode;
use pamoja_gpio::GpioError;

use crate::{set_last_error, PamojaStatus};

/// The largest I2C address frame, in bytes: the two a 10-bit address needs.
pub const PAMOJA_I2C_FRAME_MAX: usize = 2;

/// The lowest 7-bit address the I2C specification keeps for itself.
pub const PAMOJA_I2C_RESERVED_FROM: u8 = 0x78;

/// The first 7-bit address above the reserved block at the bottom of the range.
pub const PAMOJA_I2C_RESERVED_BELOW: u8 = 0x08;

// The header generator does not read the crates this one depends on, so these
// carry their value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_I2C_RESERVED_FROM == pamoja_gpio::i2c::RESERVED_FROM);
const _: () = assert!(PAMOJA_I2C_RESERVED_BELOW == pamoja_gpio::i2c::RESERVED_BELOW);

/// A validated I2C device address.
///
/// Build one with [`pamoja_i2c_address_seven_bit`] or
/// [`pamoja_i2c_address_ten_bit`], which reject a value outside the width's
/// range. Both fields are scalars, so this crosses the boundary by value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaI2cAddress {
    /// The address itself, without the read/write bit.
    pub value: u16,
    /// `1` for a 10-bit address, `0` for a 7-bit one.
    pub ten_bit: u8,
}

/// Which direction an I2C transfer runs, as the read/write bit encodes it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaI2cDirection {
    /// The controller writes to the device. Read/write bit `0`.
    Write = 0,
    /// The controller reads from the device. Read/write bit `1`.
    Read = 1,
}

/// The physical voltage level on a pin.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaPinLevel {
    /// A low level, near ground.
    Low = 0,
    /// A high level, near the supply voltage.
    High = 1,
}

/// The signal transition that triggers a pin interrupt.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaPinEdge {
    /// A low-to-high transition.
    Rising = 0,
    /// A high-to-low transition.
    Falling = 1,
    /// Either transition.
    Both = 2,
}

/// Whether a signal is asserted by a high or a low physical level.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaPinPolarity {
    /// A high level means asserted.
    ActiveHigh = 0,
    /// A low level means asserted, the wiring of most buttons and relay boards.
    ActiveLow = 1,
}

/// Validates a 7-bit I2C address.
///
/// The whole `0x00..=0x7F` range is accepted, reserved addresses included, since
/// those are still legal on the wire. Test for them with
/// [`pamoja_i2c_address_is_reserved`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_address` set, or
/// [`PamojaStatus::InvalidArgument`] if `address` is above `0x7F`.
///
/// # Safety
///
/// `out_address` must point to a writable `PamojaI2cAddress`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_address_seven_bit(
    address: u8,
    out_address: *mut PamojaI2cAddress,
) -> PamojaStatus {
    emit(Address::seven_bit(address), out_address)
}

/// Validates a 10-bit I2C address.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_address` set, or
/// [`PamojaStatus::InvalidArgument`] if `address` is above `0x3FF`.
///
/// # Safety
///
/// `out_address` must point to a writable `PamojaI2cAddress`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_address_ten_bit(
    address: u16,
    out_address: *mut PamojaI2cAddress,
) -> PamojaStatus {
    emit(Address::ten_bit(address), out_address)
}

/// Returns how many bytes an address frame occupies.
///
/// # Returns
///
/// `1` for a 7-bit address, `2` for a 10-bit one.
#[no_mangle]
pub extern "C" fn pamoja_i2c_address_frame_len(address: PamojaI2cAddress) -> usize {
    borrow(address).map_or(0, Address::frame_len)
}

/// Writes the address bytes a controller puts on the bus for a transfer.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_len` set to how many bytes of
/// `out_frame` were written, or [`PamojaStatus::InvalidArgument`] if the address
/// is not one this SDK produced or `out_frame_cap` is smaller than
/// [`pamoja_i2c_address_frame_len`].
///
/// # Safety
///
/// `out_frame` must point to at least `out_frame_cap` writable bytes, and
/// `out_len` must point to a writable `size_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_i2c_address_frame(
    address: PamojaI2cAddress,
    direction: PamojaI2cDirection,
    out_frame: *mut u8,
    out_frame_cap: usize,
    out_len: *mut usize,
) -> PamojaStatus {
    if out_frame.is_null() || out_len.is_null() {
        set_last_error("out_frame and out_len must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(address) = borrow(address) else {
        set_last_error("address is not a valid I2C address".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let out = std::slice::from_raw_parts_mut(out_frame, out_frame_cap);
    match address.write_frame(direction.into(), out) {
        Ok(written) => {
            *out_len = written;
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Reports whether a 7-bit address falls in a range the I2C specification reserves.
///
/// # Returns
///
/// `true` for a 7-bit address in `0x00..=0x07` or `0x78..=0x7F`, which leaves
/// `0x08..=0x77` for ordinary devices. A 10-bit address is never reserved in this
/// sense.
#[no_mangle]
pub extern "C" fn pamoja_i2c_address_is_reserved(address: PamojaI2cAddress) -> bool {
    borrow(address).is_some_and(Address::is_reserved)
}

/// Reports whether an address is the general call address `0x00`.
///
/// # Returns
///
/// `true` for the broadcast every device on the bus listens to.
#[no_mangle]
pub extern "C" fn pamoja_i2c_address_is_general_call(address: PamojaI2cAddress) -> bool {
    borrow(address).is_some_and(Address::is_general_call)
}

/// Returns the `(CPOL, CPHA)` pair an SPI mode number names.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_cpol` set to whether the clock
/// idles high and `*out_cpha` to whether data is sampled on the trailing edge, or
/// [`PamojaStatus::InvalidArgument`] if `mode` is above 3.
///
/// # Safety
///
/// `out_cpol` and `out_cpha` must each point to a writable `bool`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_spi_mode_cpol_cpha(
    mode: u8,
    out_cpol: *mut bool,
    out_cpha: *mut bool,
) -> PamojaStatus {
    if out_cpol.is_null() || out_cpha.is_null() {
        set_last_error("out_cpol and out_cpha must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(mode) = Mode::from_number(mode) else {
        set_last_error("SPI mode must be 0, 1, 2, or 3".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let (cpol, cpha) = mode.cpol_cpha();
    *out_cpol = cpol;
    *out_cpha = cpha;
    PamojaStatus::Ok
}

/// Returns the SPI mode number a `(CPOL, CPHA)` pair names.
///
/// # Returns
///
/// The mode number `0..=3`. Every pair names a mode, so this never fails.
#[no_mangle]
pub extern "C" fn pamoja_spi_mode_from_cpol_cpha(cpol: bool, cpha: bool) -> u8 {
    Mode::from_cpol_cpha(cpol, cpha).number()
}

/// Returns the level a boolean names.
///
/// # Returns
///
/// [`PamojaPinLevel::High`] for `true`, [`PamojaPinLevel::Low`] for `false`.
#[no_mangle]
pub extern "C" fn pamoja_pin_level_from_bool(high: bool) -> PamojaPinLevel {
    Level::from_bool(high).into()
}

/// Returns the opposite level.
///
/// # Returns
///
/// The inverted level.
#[no_mangle]
pub extern "C" fn pamoja_pin_level_inverted(level: PamojaPinLevel) -> PamojaPinLevel {
    Level::from(level).inverted().into()
}

/// Reports whether a change from one level to another fires an interrupt trigger.
///
/// # Returns
///
/// `true` if the transition matches `edge`; `false` for the other direction or
/// for no change at all.
#[no_mangle]
pub extern "C" fn pamoja_pin_edge_triggered_by(
    edge: PamojaPinEdge,
    from: PamojaPinLevel,
    to: PamojaPinLevel,
) -> bool {
    Edge::from(edge).triggered_by(from.into(), to.into())
}

/// Returns the physical level that represents a logical state under a polarity.
///
/// # Returns
///
/// The level to drive, which for active-low wiring is the inverse of `asserted`.
#[no_mangle]
pub extern "C" fn pamoja_pin_polarity_level(
    polarity: PamojaPinPolarity,
    asserted: bool,
) -> PamojaPinLevel {
    Polarity::from(polarity).level(asserted).into()
}

/// Reports whether a physical level means the signal is asserted.
///
/// # Returns
///
/// `true` if `level` asserts the signal under `polarity`.
#[no_mangle]
pub extern "C" fn pamoja_pin_polarity_is_asserted(
    polarity: PamojaPinPolarity,
    level: PamojaPinLevel,
) -> bool {
    Polarity::from(polarity).is_asserted(level.into())
}

impl From<PamojaI2cDirection> for Direction {
    fn from(value: PamojaI2cDirection) -> Self {
        match value {
            PamojaI2cDirection::Write => Direction::Write,
            PamojaI2cDirection::Read => Direction::Read,
        }
    }
}

impl From<PamojaPinLevel> for Level {
    fn from(value: PamojaPinLevel) -> Self {
        match value {
            PamojaPinLevel::Low => Level::Low,
            PamojaPinLevel::High => Level::High,
        }
    }
}

impl From<Level> for PamojaPinLevel {
    fn from(value: Level) -> Self {
        match value {
            Level::Low => PamojaPinLevel::Low,
            Level::High => PamojaPinLevel::High,
        }
    }
}

impl From<PamojaPinEdge> for Edge {
    fn from(value: PamojaPinEdge) -> Self {
        match value {
            PamojaPinEdge::Rising => Edge::Rising,
            PamojaPinEdge::Falling => Edge::Falling,
            PamojaPinEdge::Both => Edge::Both,
        }
    }
}

impl From<PamojaPinPolarity> for Polarity {
    fn from(value: PamojaPinPolarity) -> Self {
        match value {
            PamojaPinPolarity::ActiveHigh => Polarity::ActiveHigh,
            PamojaPinPolarity::ActiveLow => Polarity::ActiveLow,
        }
    }
}

/// Rebuilds the validated address a [`PamojaI2cAddress`] describes.
///
/// The value crossed the boundary as plain scalars, so a caller could have
/// changed it since; this puts it back through the same validation that produced
/// it rather than trusting the fields.
fn borrow(address: PamojaI2cAddress) -> Option<Address> {
    if address.ten_bit == 0 {
        u8::try_from(address.value)
            .ok()
            .and_then(|value| Address::seven_bit(value).ok())
    } else {
        Address::ten_bit(address.value).ok()
    }
}

/// Writes a validated address to the caller's slot, or reports why it is invalid.
///
/// # Safety
///
/// `out_address` must point to a writable `PamojaI2cAddress`.
unsafe fn emit(
    address: Result<Address, GpioError>,
    out_address: *mut PamojaI2cAddress,
) -> PamojaStatus {
    if out_address.is_null() {
        set_last_error("out_address must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match address {
        Ok(address) => {
            *out_address = PamojaI2cAddress {
                value: address.value(),
                ten_bit: u8::from(address.is_ten_bit()),
            };
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Records an addressing error and maps it onto its status.
fn failed(error: GpioError) -> PamojaStatus {
    set_last_error(error.to_string());
    PamojaStatus::InvalidArgument
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a 7-bit address, which the tests use as their ordinary case.
    fn seven_bit(value: u8) -> PamojaI2cAddress {
        let mut address = PamojaI2cAddress {
            value: 0,
            ten_bit: 0,
        };
        // Safety: the out-pointer is writable.
        let status = unsafe { pamoja_i2c_address_seven_bit(value, &mut address) };
        assert_eq!(status, PamojaStatus::Ok);
        address
    }

    #[test]
    fn a_seven_bit_address_frames_as_one_shifted_byte() {
        let bme = seven_bit(0x76);
        let mut frame = [0u8; PAMOJA_I2C_FRAME_MAX];
        let mut len = 0usize;

        // Safety: the buffer and the length slot are writable.
        unsafe {
            assert_eq!(
                pamoja_i2c_address_frame(
                    bme,
                    PamojaI2cDirection::Write,
                    frame.as_mut_ptr(),
                    frame.len(),
                    &mut len
                ),
                PamojaStatus::Ok
            );
            assert_eq!((len, frame[0]), (1, 0xEC), "(0x76 << 1) | 0");
            assert_eq!(
                pamoja_i2c_address_frame(
                    bme,
                    PamojaI2cDirection::Read,
                    frame.as_mut_ptr(),
                    frame.len(),
                    &mut len
                ),
                PamojaStatus::Ok
            );
            assert_eq!((len, frame[0]), (1, 0xED), "(0x76 << 1) | 1");
        }
    }

    #[test]
    fn a_ten_bit_address_frames_as_the_reserved_prefix_and_the_low_byte() {
        let mut address = PamojaI2cAddress {
            value: 0,
            ten_bit: 0,
        };
        let mut frame = [0u8; PAMOJA_I2C_FRAME_MAX];
        let mut len = 0usize;

        // Safety: the out-pointers and the buffer are writable.
        unsafe {
            assert_eq!(
                pamoja_i2c_address_ten_bit(0x2A5, &mut address),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_i2c_address_frame_len(address), 2);
            assert_eq!(
                pamoja_i2c_address_frame(
                    address,
                    PamojaI2cDirection::Write,
                    frame.as_mut_ptr(),
                    frame.len(),
                    &mut len
                ),
                PamojaStatus::Ok
            );
            assert_eq!(len, 2);
            assert_eq!(frame, [0xF4, 0xA5], "11110 then the top two bits, then r/w");
        }
    }

    #[test]
    fn an_out_of_range_address_is_refused() {
        let mut address = PamojaI2cAddress {
            value: 0,
            ten_bit: 0,
        };
        // Safety: the out-pointer is writable.
        let status = unsafe { pamoja_i2c_address_ten_bit(0x400, &mut address) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn a_buffer_too_small_for_the_frame_is_refused() {
        let mut address = PamojaI2cAddress {
            value: 0,
            ten_bit: 0,
        };
        let mut frame = [0u8; 1];
        let mut len = 0usize;

        // Safety: the out-pointers and the buffer are writable.
        unsafe {
            assert_eq!(
                pamoja_i2c_address_ten_bit(0x2A5, &mut address),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_i2c_address_frame(
                    address,
                    PamojaI2cDirection::Write,
                    frame.as_mut_ptr(),
                    frame.len(),
                    &mut len
                ),
                PamojaStatus::InvalidArgument
            );
        }
    }

    #[test]
    fn the_reserved_ranges_are_recognised() {
        assert!(pamoja_i2c_address_is_reserved(seven_bit(0x00)));
        assert!(pamoja_i2c_address_is_general_call(seven_bit(0x00)));
        assert!(pamoja_i2c_address_is_reserved(seven_bit(0x07)));
        assert!(!pamoja_i2c_address_is_reserved(seven_bit(0x08)));
        assert!(!pamoja_i2c_address_is_reserved(seven_bit(0x77)));
        assert!(pamoja_i2c_address_is_reserved(seven_bit(0x78)));
    }

    #[test]
    fn the_spi_modes_match_the_pairs_datasheets_quote() {
        let mut cpol = false;
        let mut cpha = false;
        for (number, expected) in [
            (0u8, (false, false)),
            (1, (false, true)),
            (2, (true, false)),
            (3, (true, true)),
        ] {
            // Safety: both out-pointers are writable.
            unsafe {
                assert_eq!(
                    pamoja_spi_mode_cpol_cpha(number, &mut cpol, &mut cpha),
                    PamojaStatus::Ok
                );
            }
            assert_eq!((cpol, cpha), expected, "mode {number}");
            assert_eq!(pamoja_spi_mode_from_cpol_cpha(cpol, cpha), number);
        }
    }

    #[test]
    fn a_mode_number_above_three_is_refused() {
        let mut cpol = false;
        let mut cpha = false;
        // Safety: both out-pointers are writable.
        let status = unsafe { pamoja_spi_mode_cpol_cpha(4, &mut cpol, &mut cpha) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn an_active_low_relay_is_energised_by_a_low_level() {
        assert_eq!(
            pamoja_pin_polarity_level(PamojaPinPolarity::ActiveLow, true),
            PamojaPinLevel::Low
        );
        assert_eq!(
            pamoja_pin_polarity_level(PamojaPinPolarity::ActiveHigh, true),
            PamojaPinLevel::High
        );
        assert!(pamoja_pin_polarity_is_asserted(
            PamojaPinPolarity::ActiveLow,
            PamojaPinLevel::Low
        ));
    }

    #[test]
    fn an_edge_fires_only_on_its_own_transition() {
        assert!(pamoja_pin_edge_triggered_by(
            PamojaPinEdge::Rising,
            PamojaPinLevel::Low,
            PamojaPinLevel::High
        ));
        assert!(!pamoja_pin_edge_triggered_by(
            PamojaPinEdge::Rising,
            PamojaPinLevel::High,
            PamojaPinLevel::Low
        ));
        assert!(pamoja_pin_edge_triggered_by(
            PamojaPinEdge::Both,
            PamojaPinLevel::High,
            PamojaPinLevel::Low
        ));
        assert!(!pamoja_pin_edge_triggered_by(
            PamojaPinEdge::Both,
            PamojaPinLevel::High,
            PamojaPinLevel::High
        ));
    }

    #[test]
    fn a_level_inverts() {
        assert_eq!(
            pamoja_pin_level_inverted(pamoja_pin_level_from_bool(true)),
            PamojaPinLevel::Low
        );
    }
}
