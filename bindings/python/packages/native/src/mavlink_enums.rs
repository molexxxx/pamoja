//! Generated Python bindings for the MAVLink dialect's named values.
//!
//! An enum field rides on the wire as a plain integer, so setting `type` on a
//! heartbeat or reading `result` off an acknowledgment works in numbers. These
//! calls turn the names the dialect writes, such as `MAV_TYPE_QUADROTOR`, into
//! those numbers and back, for every enumeration a field of a typed message
//! names.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use pamoja_mavlink::dialect::{entry_value, enum_named, EnumDescriptor, ENUMS};

fn enumeration(name: &str) -> PyResult<&'static EnumDescriptor> {
    enum_named(name)
        .ok_or_else(|| PyValueError::new_err(format!("{name} is not a dialect enumeration")))
}

/// Returns the value a dialect entry name stands for, whichever enumeration it
/// belongs to.
///
/// Raises `ValueError` if no entry of any dialect enumeration has that name.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_enum_value(entry: &str) -> PyResult<u64> {
    entry_value(entry).ok_or_else(|| {
        PyValueError::new_err(format!(
            "{entry} is not an entry of any dialect enumeration"
        ))
    })
}

/// Names the entry of an enumeration that stands for a value, or `None` if
/// none does.
///
/// Raises `ValueError` if the enumeration is unknown.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_enum_entry(enumeration: &str, value: u64) -> PyResult<Option<&'static str>> {
    Ok(self::enumeration(enumeration)?.entry(value))
}

/// Names the entries a value is made of: for a bitmask, each entry whose bits
/// are set in it, in dialect order; otherwise the one entry that names it.
/// Empty when none applies.
///
/// Raises `ValueError` if the enumeration is unknown.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_enum_names(enumeration: &str, value: u64) -> PyResult<Vec<&'static str>> {
    Ok(self::enumeration(enumeration)?.names(value).collect())
}

/// Returns every entry of an enumeration as `(name, value)` pairs, in dialect
/// order.
///
/// Raises `ValueError` if the enumeration is unknown.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_enum_entries(enumeration: &str) -> PyResult<Vec<(&'static str, u64)>> {
    Ok(self::enumeration(enumeration)?
        .entries
        .iter()
        .map(|entry| (entry.name, entry.value))
        .collect())
}

/// Reports whether an enumeration's values combine as bits.
///
/// Raises `ValueError` if the enumeration is unknown.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_enum_is_bitmask(enumeration: &str) -> PyResult<bool> {
    Ok(self::enumeration(enumeration)?.bitmask)
}

/// Returns the names of every enumeration the dialect table holds, in the order
/// the dialect defines them.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn mavlink_known_enums() -> Vec<&'static str> {
    ENUMS.iter().map(|described| described.name).collect()
}
