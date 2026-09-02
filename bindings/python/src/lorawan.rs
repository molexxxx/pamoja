//! Generated Python bindings for LoRaWAN 1.0.x MAC framing.
//!
//! These mirror the `pamoja-lorawan` Rust API: the secured frame a long-range node
//! puts on the air, and the over-the-air activation that hands it its session
//! keys.
//!
//! A session and a device hold key material, so they are classes and the keys
//! never come back out. An encoded frame crosses as the bytes to transmit, and a
//! decoded one as a read-only object carrying its header fields and its recovered
//! payload.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_lorawan::{
    Device as CoreDevice, Direction, Downlink, FrameHeader, JoinAccept as CoreJoinAccept,
    JoinGrant, JoinRequest, LorawanError, MessageType, RxData, Session as CoreSession, Uplink,
};

use crate::PamojaError;

/// A decoded data frame, with its payload decrypted.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanRxData {
    /// The direction the frame travelled: `Uplink` or `Downlink`.
    #[pyo3(get)]
    direction: String,
    /// The device address the frame carries.
    #[pyo3(get)]
    dev_addr: u32,
    /// The low 16 bits of the frame counter.
    #[pyo3(get)]
    fcnt: u16,
    /// Whether the frame asks to be acknowledged.
    #[pyo3(get)]
    confirmed: bool,
    /// Whether the frame takes part in adaptive data rate.
    #[pyo3(get)]
    adr: bool,
    /// Whether the frame acknowledges the last confirmed one.
    #[pyo3(get)]
    ack: bool,
    /// Whether the network has more downlink data waiting.
    #[pyo3(get)]
    fpending: bool,
    /// The port the frame was sent on, or `None` when it carries only options.
    #[pyo3(get)]
    fport: Option<u8>,
    fopts: Vec<u8>,
    payload: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanRxData {
    /// The MAC commands the header carried.
    #[getter]
    fn fopts<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.fopts)
    }

    /// The decrypted application payload.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.payload)
    }
}

/// An activated LoRaWAN session: a device address and its two session keys.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanSession {
    inner: CoreSession,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanSession {
    /// Creates a session from a device address and its two 16-byte session keys.
    ///
    /// `nwk_skey` authenticates frames and `app_skey` encrypts payloads.
    #[new]
    fn new(dev_addr: u32, nwk_skey: Vec<u8>, app_skey: Vec<u8>) -> PyResult<Self> {
        Ok(LorawanSession {
            inner: CoreSession::new(
                dev_addr,
                key(&nwk_skey, "nwk_skey")?,
                key(&app_skey, "app_skey")?,
            ),
        })
    }

    /// The device address this session is bound to.
    #[getter]
    fn dev_addr(&self) -> u32 {
        self.inner.dev_addr()
    }

    /// Encodes an uplink, encrypting the payload and appending the MIC.
    #[pyo3(signature = (
        fcnt,
        fport,
        payload,
        confirmed = false,
        adr = false,
        ack = false,
        fopts = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn encode_uplink<'py>(
        &self,
        py: Python<'py>,
        fcnt: u32,
        fport: u8,
        payload: Vec<u8>,
        confirmed: bool,
        adr: bool,
        ack: bool,
        fopts: Option<Vec<u8>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let fopts = fopts.unwrap_or_default();
        let mut uplink = Uplink::new(fcnt, fport, &payload).with_fopts(&fopts);
        if confirmed {
            uplink = uplink.confirmed();
        }
        if adr {
            uplink = uplink.with_adr();
        }
        if ack {
            uplink = uplink.with_ack();
        }
        self.inner
            .encode_uplink(&uplink)
            .map(|frame| PyBytes::new(py, frame.as_bytes()))
            .map_err(to_py)
    }

    /// Encodes a downlink, encrypting the payload and appending the MIC.
    #[pyo3(signature = (
        fcnt,
        fport,
        payload,
        confirmed = false,
        adr = false,
        ack = false,
        fpending = false,
        fopts = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn encode_downlink<'py>(
        &self,
        py: Python<'py>,
        fcnt: u32,
        fport: u8,
        payload: Vec<u8>,
        confirmed: bool,
        adr: bool,
        ack: bool,
        fpending: bool,
        fopts: Option<Vec<u8>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let fopts = fopts.unwrap_or_default();
        let mut downlink = Downlink::new(fcnt, fport, &payload).with_fopts(&fopts);
        if confirmed {
            downlink = downlink.confirmed();
        }
        if adr {
            downlink = downlink.with_adr();
        }
        if ack {
            downlink = downlink.with_ack();
        }
        if fpending {
            downlink = downlink.with_fpending();
        }
        self.inner
            .encode_downlink(&downlink)
            .map(|frame| PyBytes::new(py, frame.as_bytes()))
            .map_err(to_py)
    }

    /// Verifies a received frame, then decrypts it.
    ///
    /// `fcnt` is the full 32-bit counter expected for this frame; its low 16 bits
    /// must match the counter the frame carries.
    fn decode(&self, bytes: Vec<u8>, fcnt: u32) -> PyResult<LorawanRxData> {
        self.inner.decode(&bytes, fcnt).map(describe).map_err(to_py)
    }
}

/// The root credentials over-the-air activation is built on.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanDevice {
    inner: CoreDevice,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanDevice {
    /// Creates a device from its two 8-byte EUIs and its 16-byte application key.
    #[new]
    fn new(dev_eui: Vec<u8>, app_eui: Vec<u8>, app_key: Vec<u8>) -> PyResult<Self> {
        Ok(LorawanDevice {
            inner: CoreDevice::new(
                eui(&dev_eui, "dev_eui")?,
                eui(&app_eui, "app_eui")?,
                key(&app_key, "app_key")?,
            ),
        })
    }

    /// Builds the join request this device broadcasts to activate.
    ///
    /// `dev_nonce` must never repeat for a device, since the network rejects a
    /// replayed one.
    fn join_request<'py>(&self, py: Python<'py>, dev_nonce: u16) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.join_request(dev_nonce).as_bytes())
    }

    /// Turns the join accept a network sent into the settings it grants.
    ///
    /// `dev_nonce` is the nonce the matching join request carried.
    fn accept_join(&self, bytes: Vec<u8>, dev_nonce: u16) -> PyResult<LorawanJoinAccept> {
        self.inner
            .accept_join(&bytes, dev_nonce)
            .map(|accept| LorawanJoinAccept { inner: accept })
            .map_err(to_py)
    }
}

/// An accepted join: the network settings, and the session it grants.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanJoinAccept {
    inner: CoreJoinAccept,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanJoinAccept {
    /// The device address the network assigned.
    #[getter]
    fn dev_addr(&self) -> u32 {
        self.inner.dev_addr()
    }

    /// The identifier of the network that accepted the join.
    #[getter]
    fn net_id(&self) -> u32 {
        self.inner.net_id()
    }

    /// The downlink settings byte, carrying the second receive window data rate
    /// and the first window offset.
    #[getter]
    fn dl_settings(&self) -> u8 {
        self.inner.dl_settings()
    }

    /// The delay before the first receive window, in seconds.
    #[getter]
    fn rx_delay(&self) -> u8 {
        self.inner.rx_delay()
    }

    /// The activated session this join grants, with its keys already derived.
    fn session(&self) -> LorawanSession {
        LorawanSession {
            inner: self.inner.session(),
        }
    }
}

/// Reads every field off a decoded frame into the object Python receives.
fn describe(rx: RxData) -> LorawanRxData {
    LorawanRxData {
        direction: match rx.direction() {
            Direction::Uplink => "Uplink".to_owned(),
            Direction::Downlink => "Downlink".to_owned(),
        },
        dev_addr: rx.dev_addr(),
        fcnt: rx.fcnt(),
        confirmed: rx.confirmed(),
        adr: rx.adr(),
        ack: rx.ack(),
        fpending: rx.fpending(),
        fport: rx.fport(),
        fopts: rx.fopts().to_vec(),
        payload: rx.payload().to_vec(),
    }
}

/// Copies a 16-byte key, rejecting anything else.
fn key(bytes: &[u8], what: &str) -> PyResult<[u8; 16]> {
    <[u8; 16]>::try_from(bytes)
        .map_err(|_| PamojaError::new_err(format!("{what} must be exactly 16 bytes")))
}

/// Copies an 8-byte EUI, rejecting anything else.
fn eui(bytes: &[u8], what: &str) -> PyResult<[u8; 8]> {
    <[u8; 8]>::try_from(bytes)
        .map_err(|_| PamojaError::new_err(format!("{what} must be exactly 8 bytes")))
}

/// Turns a LoRaWAN error into the Python exception a caller sees.
fn to_py(error: LorawanError) -> PyErr {
    PamojaError::new_err(error.to_string())
}

/// What a frame says about itself before any key is involved.
///
/// Nothing here is authenticated, since checking the MIC needs the session key.
/// Treat it as a routing hint until `decode` has verified the frame.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanHeader {
    /// What kind of message the frame is.
    #[pyo3(get)]
    message_type: String,
    /// Whether this is a data frame rather than part of a join exchange.
    #[pyo3(get)]
    is_data: bool,
    /// The device address, or `None` for a join frame.
    #[pyo3(get)]
    dev_addr: Option<u32>,
    /// The low 16 bits of the frame counter, or `None` for a join frame.
    #[pyo3(get)]
    fcnt: Option<u16>,
    /// The port, or `None` for a join frame or one carrying only options.
    #[pyo3(get)]
    fport: Option<u8>,
    /// Whether the frame asks to be acknowledged.
    #[pyo3(get)]
    confirmed: bool,
    /// Whether the frame takes part in adaptive data rate.
    #[pyo3(get)]
    adr: bool,
    /// Whether the frame acknowledges the last confirmed one.
    #[pyo3(get)]
    ack: bool,
    /// Whether the network has more downlink data waiting.
    #[pyo3(get)]
    fpending: bool,
    /// How many bytes of frame options the header carries.
    #[pyo3(get)]
    fopts_len: usize,
    /// The length of the still-encrypted payload.
    #[pyo3(get)]
    payload_len: usize,
}

/// A join-request a device broadcast, with its integrity already verified.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanJoinRequest {
    /// The nonce the request carried, which a network must not accept twice.
    #[pyo3(get)]
    dev_nonce: u16,
    dev_eui: Vec<u8>,
    app_eui: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanJoinRequest {
    /// The device identifier, most-significant byte first.
    #[getter]
    fn dev_eui<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.dev_eui)
    }

    /// The application identifier, most-significant byte first.
    #[getter]
    fn app_eui<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.app_eui)
    }
}

/// What a network grants a device that joined.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanGrant {
    inner: JoinGrant,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanGrant {
    /// Creates a grant of an address and the settings to answer on.
    ///
    /// `app_nonce` and `net_id` carry their low 24 bits only.
    #[new]
    #[pyo3(signature = (app_nonce, net_id, dev_addr, dl_settings = 0, rx_delay = 0, cflist = None))]
    fn new(
        app_nonce: u32,
        net_id: u32,
        dev_addr: u32,
        dl_settings: u8,
        rx_delay: u8,
        cflist: Option<Vec<u8>>,
    ) -> PyResult<Self> {
        let mut inner = JoinGrant::new(app_nonce, net_id, dev_addr)
            .with_dl_settings(dl_settings)
            .with_rx_delay(rx_delay);
        if let Some(cflist) = cflist {
            let cflist = <[u8; 16]>::try_from(&cflist[..])
                .map_err(|_| PamojaError::new_err("cflist must be exactly 16 bytes".to_owned()))?;
            inner = inner.with_cflist(cflist);
        }
        Ok(LorawanGrant { inner })
    }

    /// The address this grant assigns.
    #[getter]
    fn dev_addr(&self) -> u32 {
        self.inner.dev_addr()
    }

    /// The network identifier this grant carries.
    #[getter]
    fn net_id(&self) -> u32 {
        self.inner.net_id()
    }

    /// Builds the signed join-accept to transmit.
    fn accept<'py>(
        &self,
        py: Python<'py>,
        app_key: Vec<u8>,
        dev_nonce: u16,
    ) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(
            py,
            self.inner
                .accept(&key(&app_key, "app_key")?, dev_nonce)
                .as_bytes(),
        ))
    }

    /// Derives the session this grant activates, the same one the device computes.
    fn session(&self, app_key: Vec<u8>, dev_nonce: u16) -> PyResult<LorawanSession> {
        Ok(LorawanSession {
            inner: self.inner.session(&key(&app_key, "app_key")?, dev_nonce),
        })
    }
}

/// Reads a frame far enough to route it, without any key.
///
/// A receiver holding many sessions uses this to find which one a frame belongs
/// to: the device address travels in the clear.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_parse_header(bytes: Vec<u8>) -> PyResult<LorawanHeader> {
    let header = FrameHeader::parse(&bytes).map_err(to_py)?;
    Ok(LorawanHeader {
        message_type: match header.message_type() {
            MessageType::JoinRequest => "JoinRequest",
            MessageType::JoinAccept => "JoinAccept",
            MessageType::UnconfirmedUp => "UnconfirmedUp",
            MessageType::ConfirmedUp => "ConfirmedUp",
            MessageType::UnconfirmedDown => "UnconfirmedDown",
            MessageType::ConfirmedDown => "ConfirmedDown",
        }
        .to_owned(),
        is_data: header.message_type().is_data(),
        dev_addr: header.dev_addr(),
        fcnt: header.fcnt(),
        fport: header.fport(),
        confirmed: header.confirmed(),
        adr: header.adr(),
        ack: header.ack(),
        fpending: header.fpending(),
        fopts_len: header.fopts_len(),
        payload_len: header.payload_len(),
    })
}

/// Verifies a join-request and reads the identifiers out of it.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_parse_join_request(
    bytes: Vec<u8>,
    app_key: Vec<u8>,
) -> PyResult<LorawanJoinRequest> {
    let request = JoinRequest::parse(&bytes, &key(&app_key, "app_key")?).map_err(to_py)?;
    Ok(LorawanJoinRequest {
        dev_nonce: request.dev_nonce(),
        dev_eui: request.dev_eui().to_vec(),
        app_eui: request.app_eui().to_vec(),
    })
}
