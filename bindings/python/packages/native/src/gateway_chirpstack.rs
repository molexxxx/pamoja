//! Generated Python bindings for the uplink events a ChirpStack network server publishes.
//!
//! ChirpStack publishes every uplink it deduplicates to an MQTT topic as the JSON form of its
//! `UplinkEvent` message. These read one into the device, the counter, the payload and every
//! gateway that heard it.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_gateway::chirpstack::{uplink_topic, Reception, UplinkEvent};

use crate::PamojaError;

/// One gateway that heard an uplink.
#[gen_stub_pyclass]
#[pyclass]
pub struct ChirpstackReception {
    /// The gateway's EUI, as lowercase hex.
    #[pyo3(get)]
    gateway: String,
    /// The received signal strength, in dBm.
    #[pyo3(get)]
    rssi_dbm: i32,
    /// The signal-to-noise ratio, in dB.
    #[pyo3(get)]
    snr_db: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl ChirpstackReception {
    fn __repr__(&self) -> String {
        format!(
            "ChirpstackReception(gateway={:?}, rssi_dbm={}, snr_db={})",
            self.gateway, self.rssi_dbm, self.snr_db
        )
    }
}

impl From<&Reception> for ChirpstackReception {
    fn from(reception: &Reception) -> Self {
        Self {
            gateway: reception.gateway.to_hex(),
            rssi_dbm: reception.rssi_dbm,
            snr_db: f64::from(reception.snr_db),
        }
    }
}

/// An uplink as ChirpStack reports it.
#[gen_stub_pyclass]
#[pyclass]
pub struct ChirpstackUplinkEvent {
    inner: UplinkEvent,
}

#[gen_stub_pymethods]
#[pymethods]
impl ChirpstackUplinkEvent {
    /// Reads an uplink event from the JSON ChirpStack published.
    ///
    /// Fields protobuf's JSON mapping leaves out when they hold their default read as that
    /// default: a frame counter of zero, ADR off, unconfirmed. Raises `PamojaError` for text
    /// that is not a JSON object, an event with no device EUI, or a field that does not read as
    /// what it should.
    #[staticmethod]
    fn from_json(text: &str) -> PyResult<Self> {
        UplinkEvent::from_json(text)
            .map(|inner| Self { inner })
            .map_err(|error| PamojaError::new_err(error.to_string()))
    }

    /// The identifier ChirpStack gave the uplink once it deduplicated the gateways' copies.
    #[getter]
    fn deduplication_id(&self) -> String {
        self.inner.deduplication_id.clone()
    }

    /// When the uplink was received, as ChirpStack wrote it, or `None`.
    #[getter]
    fn time(&self) -> Option<String> {
        self.inner.time.clone()
    }

    /// The application the device belongs to.
    #[getter]
    fn application_id(&self) -> String {
        self.inner.application_id.clone()
    }

    /// The name the device was given in ChirpStack.
    #[getter]
    fn device_name(&self) -> String {
        self.inner.device_name.clone()
    }

    /// The device EUI, as lowercase hex.
    #[getter]
    fn dev_eui(&self) -> String {
        self.inner.dev_eui.to_hex()
    }

    /// The device's address, or `None` when the event names none.
    #[getter]
    fn dev_addr(&self) -> Option<u32> {
        self.inner.dev_addr
    }

    /// Whether the device had adaptive data rate on.
    #[getter]
    fn adr(&self) -> bool {
        self.inner.adr
    }

    /// The data rate, as the region numbers them.
    #[getter]
    fn data_rate(&self) -> u8 {
        self.inner.data_rate
    }

    /// The uplink frame counter.
    #[getter]
    fn fcnt(&self) -> u32 {
        self.inner.fcnt
    }

    /// The application port, or `None` for a frame that carried none.
    #[getter]
    fn fport(&self) -> Option<u8> {
        self.inner.fport
    }

    /// Whether the uplink was confirmed.
    #[getter]
    fn confirmed(&self) -> bool {
        self.inner.confirmed
    }

    /// The application payload, decoded from base64.
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.data)
    }

    /// The carrier it was heard on, in hertz, or `None`.
    #[getter]
    fn frequency_hz(&self) -> Option<u32> {
        self.inner.frequency_hz
    }

    /// Every gateway that heard it.
    #[getter]
    fn receptions(&self) -> Vec<ChirpstackReception> {
        self.inner
            .receptions
            .iter()
            .map(ChirpstackReception::from)
            .collect()
    }

    /// The gateway that heard it with the highest signal-to-noise ratio, or `None` when none
    /// did.
    fn best_reception(&self) -> Option<ChirpstackReception> {
        self.inner.best_reception().map(ChirpstackReception::from)
    }

    fn __repr__(&self) -> String {
        format!(
            "ChirpstackUplinkEvent(dev_eui={:?}, fcnt={}, fport={:?})",
            self.inner.dev_eui.to_hex(),
            self.inner.fcnt,
            self.inner.fport
        )
    }
}

/// Builds the MQTT topic an application's uplink events are published on, with a wildcard in
/// place of the device: `application/<id>/device/+/event/up`.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn chirpstack_uplink_topic(application_id: &str) -> String {
    uplink_topic(application_id)
}
