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

#[cfg(feature = "can")]
mod can;
#[cfg(feature = "codec")]
mod codec;
#[cfg(feature = "gpio")]
mod gpio;
#[cfg(feature = "kit")]
mod kit;
#[cfg(feature = "modbus")]
mod modbus;
#[cfg(feature = "mqtt")]
mod mqtt;
#[cfg(feature = "security")]
mod security;
#[cfg(feature = "serial")]
mod serial;

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
    #[cfg(feature = "serial")]
    {
        m.add_class::<serial::SlipDecoder>()?;
        m.add_class::<serial::CobsDecoder>()?;
        m.add_function(wrap_pyfunction!(serial::slip_encode, m)?)?;
        m.add_function(wrap_pyfunction!(serial::slip_decode, m)?)?;
        m.add_function(wrap_pyfunction!(serial::cobs_encode, m)?)?;
        m.add_function(wrap_pyfunction!(serial::cobs_decode, m)?)?;
        m.add_function(wrap_pyfunction!(serial::slip_max_encoded_len, m)?)?;
        m.add_function(wrap_pyfunction!(serial::cobs_max_encoded_len, m)?)?;
    }
    #[cfg(feature = "modbus")]
    {
        m.add_class::<modbus::ModbusFrame>()?;
        m.add_function(wrap_pyfunction!(modbus::modbus_crc16, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_read_coils, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_read_discrete_inputs, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_read_holding_registers, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_read_input_registers, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_write_single_coil, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_write_single_register, m)?)?;
        m.add_function(wrap_pyfunction!(
            modbus::modbus_write_multiple_registers,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_write_multiple_coils, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_raw, m)?)?;
        m.add_function(wrap_pyfunction!(modbus::modbus_parse_frame, m)?)?;
    }
    #[cfg(feature = "can")]
    {
        m.add_class::<can::CanFrame>()?;
        m.add_class::<can::J1939Message>()?;
        m.add_function(wrap_pyfunction!(can::can_frame, m)?)?;
        m.add_function(wrap_pyfunction!(can::can_fd_frame, m)?)?;
        m.add_function(wrap_pyfunction!(can::can_remote_frame, m)?)?;
        m.add_function(wrap_pyfunction!(can::can_len_to_dlc, m)?)?;
        m.add_function(wrap_pyfunction!(can::can_dlc_to_len, m)?)?;
        m.add_function(wrap_pyfunction!(can::j1939_decode, m)?)?;
        m.add_function(wrap_pyfunction!(can::j1939_compose, m)?)?;
    }
    #[cfg(feature = "gpio")]
    {
        m.add_class::<gpio::SpiClock>()?;
        m.add_function(wrap_pyfunction!(gpio::i2c_address_frame, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::i2c_address_frame_len, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::i2c_address_is_reserved, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::i2c_address_is_general_call, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::spi_mode_clock, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::spi_mode_from_clock, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_level_inverted, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_level_from_bool, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_edge_triggered_by, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_polarity_level, m)?)?;
        m.add_function(wrap_pyfunction!(gpio::pin_polarity_is_asserted, m)?)?;
    }
    Ok(())
}

define_stub_info_gatherer!(stub_info);
