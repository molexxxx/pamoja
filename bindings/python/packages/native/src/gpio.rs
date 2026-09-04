//! Generated Python bindings for on-board bus addressing and pin logic.
//!
//! These mirror the `pamoja-gpio` Rust API: I2C addressing per NXP UM10204, the
//! four SPI clock modes, and the pin model that maps a logical "asserted" onto a
//! physical level. Everything here is pure arithmetic over small values, so
//! nothing holds state.
//!
//! The pin enumerations cross as plain strings, which the facade turns back into
//! Python enum members.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_gpio::i2c::{Address, Direction};
use pamoja_gpio::pin::{Edge, Level, Polarity};
use pamoja_gpio::spi::Mode;
use pamoja_gpio::GpioError;

use crate::PamojaError;

/// The clock polarity and phase pair an SPI mode number names.
#[gen_stub_pyclass]
#[pyclass]
pub struct SpiClock {
    /// Whether the clock idles high (CPOL = 1), which is modes 2 and 3.
    #[pyo3(get)]
    cpol: bool,
    /// Whether data is sampled on the trailing edge (CPHA = 1), which is modes 1 and 3.
    #[pyo3(get)]
    cpha: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl SpiClock {
    /// Renders the pair the way a datasheet quotes it.
    fn __repr__(&self) -> String {
        format!("SpiClock(cpol={}, cpha={})", self.cpol, self.cpha)
    }
}

/// Returns the address bytes a controller puts on the bus for a transfer.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn i2c_address_frame<'py>(
    py: Python<'py>,
    address: u16,
    ten_bit: bool,
    read: bool,
) -> PyResult<Bound<'py, PyBytes>> {
    let address = validate(address, ten_bit)?;
    let direction = if read {
        Direction::Read
    } else {
        Direction::Write
    };
    let mut out = [0u8; 2];
    let written = address.write_frame(direction, &mut out).map_err(to_py)?;
    Ok(PyBytes::new(py, &out[..written]))
}

/// Reports whether a 7-bit address falls in a range the I2C specification reserves.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn i2c_address_is_reserved(address: u16, ten_bit: bool) -> PyResult<bool> {
    Ok(validate(address, ten_bit)?.is_reserved())
}

/// Reports whether an address is the general call address `0x00`.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn i2c_address_is_general_call(address: u16, ten_bit: bool) -> PyResult<bool> {
    Ok(validate(address, ten_bit)?.is_general_call())
}

/// Returns how many bytes an address frame occupies.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn i2c_address_frame_len(address: u16, ten_bit: bool) -> PyResult<usize> {
    Ok(validate(address, ten_bit)?.frame_len())
}

/// Returns the `(CPOL, CPHA)` pair an SPI mode number names.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn spi_mode_clock(mode: u8) -> PyResult<SpiClock> {
    let mode = Mode::from_number(mode)
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("SPI mode must be 0, 1, 2, or 3"))?;
    let (cpol, cpha) = mode.cpol_cpha();
    Ok(SpiClock { cpol, cpha })
}

/// Returns the SPI mode number a `(CPOL, CPHA)` pair names.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn spi_mode_from_clock(cpol: bool, cpha: bool) -> u8 {
    Mode::from_cpol_cpha(cpol, cpha).number()
}

/// Returns the opposite level, as `"Low"` or `"High"`.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pin_level_inverted(level: &str) -> PyResult<String> {
    Ok(name(read_level(level)?.inverted()).to_owned())
}

/// Returns the level a boolean names, as `"Low"` or `"High"`.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pin_level_from_bool(high: bool) -> String {
    name(Level::from_bool(high)).to_owned()
}

/// Reports whether a change from one level to another fires an interrupt trigger.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pin_edge_triggered_by(edge: &str, before: &str, after: &str) -> PyResult<bool> {
    let edge = match edge {
        "Rising" => Edge::Rising,
        "Falling" => Edge::Falling,
        "Both" => Edge::Both,
        _ => {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "edge must be Rising, Falling, or Both",
            ))
        }
    };
    Ok(edge.triggered_by(read_level(before)?, read_level(after)?))
}

/// Returns the physical level that represents a logical state under a polarity.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pin_polarity_level(polarity: &str, asserted: bool) -> PyResult<String> {
    Ok(name(read_polarity(polarity)?.level(asserted)).to_owned())
}

/// Reports whether a physical level means the signal is asserted.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn pin_polarity_is_asserted(polarity: &str, level: &str) -> PyResult<bool> {
    Ok(read_polarity(polarity)?.is_asserted(read_level(level)?))
}

/// Names a level as the string that crosses the boundary.
fn name(level: Level) -> &'static str {
    match level {
        Level::Low => "Low",
        Level::High => "High",
    }
}

/// Reads a level back from its name.
fn read_level(level: &str) -> PyResult<Level> {
    match level {
        "Low" => Ok(Level::Low),
        "High" => Ok(Level::High),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "level must be Low or High",
        )),
    }
}

/// Reads a polarity back from its name.
fn read_polarity(polarity: &str) -> PyResult<Polarity> {
    match polarity {
        "ActiveHigh" => Ok(Polarity::ActiveHigh),
        "ActiveLow" => Ok(Polarity::ActiveLow),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "polarity must be ActiveHigh or ActiveLow",
        )),
    }
}

/// Validates an address of the given width, rejecting one out of range.
fn validate(address: u16, ten_bit: bool) -> PyResult<Address> {
    if ten_bit {
        Address::ten_bit(address).map_err(to_py)
    } else {
        u8::try_from(address)
            .map_err(|_| to_py(GpioError::AddressOutOfRange))
            .and_then(|value| Address::seven_bit(value).map_err(to_py))
    }
}

/// Maps an addressing error onto the SDK's Python exception.
fn to_py(error: GpioError) -> PyErr {
    PamojaError::new_err(error.to_string())
}
