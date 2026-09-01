//! Python bindings for the pamoja SDK, generated with PyO3.
//!
//! This crate is the generated low-level surface (the contract tier): it exposes
//! the Rust core and capability crates to Python one-to-one. A hand-written,
//! idiomatic facade wraps it for everyday use; see the `pamoja` Python package.
//!
//! The native module is imported as `pamoja._core` and re-exported verbatim at
//! `pamoja.raw`.

use pyo3::prelude::*;
use pyo3_stub_gen::{define_stub_info_gatherer, derive::gen_stub_pyfunction};

#[cfg(feature = "codec")]
mod codec;
#[cfg(feature = "kit")]
mod kit;
#[cfg(feature = "mqtt")]
mod mqtt;
#[cfg(feature = "security")]
mod security;

pyo3::create_exception!(
    pamoja,
    PamojaError,
    pyo3::exceptions::PyException,
    "Raised when a pamoja operation fails."
);

/// Returns the version of the native pamoja module.
#[gen_stub_pyfunction]
#[pyfunction]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// The generated low-level Python surface for the pamoja core.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add("PamojaError", m.py().get_type::<PamojaError>())?;
    #[cfg(feature = "mqtt")]
    {
        m.add_class::<mqtt::MqttClient>()?;
        m.add_class::<mqtt::MqttMessage>()?;
    }
    #[cfg(feature = "security")]
    {
        m.add_class::<security::DeviceIdentity>()?;
        m.add_function(wrap_pyfunction!(security::verify, m)?)?;
        m.add_function(wrap_pyfunction!(security::fingerprint, m)?)?;
    }
    #[cfg(feature = "codec")]
    {
        m.add_class::<codec::Quantizer>()?;
        m.add_function(wrap_pyfunction!(codec::json_to_cbor_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(codec::cbor_to_json_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(codec::encode_delta_samples, m)?)?;
        m.add_function(wrap_pyfunction!(codec::decode_delta_samples, m)?)?;
    }
    #[cfg(feature = "kit")]
    {
        m.add_class::<kit::Smoother>()?;
        m.add_class::<kit::Pid>()?;
        m.add_class::<kit::Thermostat>()?;
        m.add_class::<kit::Depletion>()?;
        m.add_class::<kit::Kalman>()?;
        m.add_class::<kit::Debounce>()?;
        m.add_class::<kit::Ramp>()?;
        m.add_class::<kit::Surge>()?;
        m.add_class::<kit::Calibration>()?;
        m.add_class::<kit::Geofence>()?;
        m.add_function(wrap_pyfunction!(kit::distance_between, m)?)?;
        m.add_function(wrap_pyfunction!(kit::bearing_between, m)?)?;
        m.add_function(wrap_pyfunction!(kit::deadband, m)?)?;
    }
    Ok(())
}

define_stub_info_gatherer!(stub_info);
