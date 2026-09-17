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

use pamoja_lorawan::mac::{MacCommand, MacCommands};
use pamoja_lorawan::{
    Device as CoreDevice, Direction, Downlink, FrameHeader, JoinAccept as CoreJoinAccept,
    JoinGrant, JoinRequest, LorawanError, MessageType, RxData, Session as CoreSession, Uplink,
};

use crate::PamojaError;

/// A decoded data frame, with its payload decrypted.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanRxData {
    /// The direction the frame traveled: `Uplink` or `Downlink`.
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
    /// Whether an uplink asks the network to answer.
    #[pyo3(get)]
    adr_ack_req: bool,
    /// Whether an uplink comes from a device running Class B.
    #[pyo3(get)]
    class_b: bool,
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
    pub(crate) inner: CoreSession,
}

/// The integrity and encryption keys of one end device's wake-on-radio frames, TS011-1.0.1.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct LorawanWorKeys {
    /// `WorSIntKey`, 16 bytes.
    #[pyo3(get)]
    pub(crate) integrity: Vec<u8>,
    /// `WorSEncKey`, 16 bytes.
    #[pyo3(get)]
    pub(crate) encryption: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanWorKeys {
    /// Holds keys derived earlier, such as ones a relay keeps for an end device it trusts.
    #[new]
    fn new(integrity: Vec<u8>, encryption: Vec<u8>) -> PyResult<Self> {
        key(&integrity, "integrity")?;
        key(&encryption, "encryption")?;
        Ok(Self {
            integrity,
            encryption,
        })
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.integrity == other.integrity && self.encryption == other.encryption
    }

    fn __repr__(&self) -> String {
        "LorawanWorKeys(..)".to_owned()
    }
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

    /// The root relay session key a network sends a relay for this device, TS011-1.0.1
    /// section 4.4.
    fn root_wor_s_key<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.root_wor_s_key())
    }

    /// The keys this device's wake-on-radio frames are protected with, section 4.5.
    fn wor_keys(&self) -> LorawanWorKeys {
        let keys = self.inner.wor_keys();
        LorawanWorKeys {
            integrity: keys.integrity().to_vec(),
            encryption: keys.encryption().to_vec(),
        }
    }

    /// Encodes an uplink, encrypting the payload and appending the MIC.
    ///
    /// `adr_ack_req` asks the network to answer, which a device running adaptive data
    /// rate does once it has gone too long without hearing it.
    #[pyo3(signature = (
        fcnt,
        fport,
        payload,
        confirmed = false,
        adr = false,
        ack = false,
        fopts = None,
        adr_ack_req = false,
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
        adr_ack_req: bool,
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
        if adr_ack_req {
            uplink = uplink.with_adr_ack_req();
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
    pub(crate) inner: CoreDevice,
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

    /// The 8-byte device EUI, most-significant byte first.
    #[getter]
    fn dev_eui<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.dev_eui())
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

    /// The delay byte before the first receive window, as it arrived.
    #[getter]
    fn rx_delay(&self) -> u8 {
        self.inner.rx_delay()
    }

    /// How far below the uplink's data rate the first receive window listens, the
    /// RX1DROffset of the downlink settings.
    #[getter]
    fn rx1_dr_offset(&self) -> u8 {
        self.inner.rx1_dr_offset()
    }

    /// The data rate the second receive window listens at.
    #[getter]
    fn rx2_data_rate(&self) -> u8 {
        self.inner.rx2_data_rate()
    }

    /// The delay from the end of an uplink to the first receive window, in microseconds,
    /// where a delay byte of zero means one second.
    #[getter]
    fn receive_delay_us(&self) -> u32 {
        self.inner.receive_delay_us()
    }

    /// The channel list the accept carried, or `None` when it carried none.
    #[getter]
    fn cflist(&self) -> Option<crate::lorawan_link::LorawanCfList> {
        self.inner
            .cflist()
            .map(crate::lorawan_link::LorawanCfList::from_core)
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
        adr_ack_req: rx.adr_ack_req(),
        class_b: rx.class_b(),
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
    /// Whether an uplink asks the network to answer.
    #[pyo3(get)]
    adr_ack_req: bool,
    /// Whether an uplink comes from a device running Class B.
    #[pyo3(get)]
    class_b: bool,
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
    /// `app_nonce` and `net_id` carry their low 24 bits only. `cflist` is the 16-byte
    /// channel list, such as `LorawanCfList.bytes`.
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
        adr_ack_req: header.adr_ack_req(),
        class_b: header.class_b(),
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

/// One of the commands a network and a device configure each other with.
///
/// `kind` names the command and `cid` is the identifier it travels under. Only the fields
/// that command carries are set; the rest are `None`. The same identifier means a different
/// command in each direction, so `direction` decides which one this is.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanMacCommand {
    /// Which command this is, as a name.
    #[pyo3(get)]
    kind: String,
    /// The identifier it travels under.
    #[pyo3(get)]
    cid: u8,
    /// Which way it travels: `Uplink` or `Downlink`.
    #[pyo3(get)]
    direction: String,
    /// How far above the floor a link check arrived, in dB.
    #[pyo3(get)]
    margin: Option<u8>,
    /// How many gateways heard it.
    #[pyo3(get)]
    gateways: Option<u8>,
    /// The data rate a network asks a device to use.
    #[pyo3(get)]
    data_rate: Option<u8>,
    /// The transmit power it may use, as a ceiling.
    #[pyo3(get)]
    tx_power: Option<u8>,
    /// Which channels may carry an uplink.
    #[pyo3(get)]
    channel_mask: Option<u16>,
    /// Which block of sixteen channels that mask applies to.
    #[pyo3(get)]
    mask_control: Option<u8>,
    /// How many times to send an unconfirmed uplink.
    #[pyo3(get)]
    transmissions: Option<u8>,
    /// Whether the power was set.
    #[pyo3(get)]
    power_ack: Option<bool>,
    /// Whether the data rate was set.
    #[pyo3(get)]
    data_rate_ack: Option<bool>,
    /// Whether the channel mask was usable.
    #[pyo3(get)]
    channel_mask_ack: Option<bool>,
    /// The share of the air a device is held to, as one over two to this.
    #[pyo3(get)]
    max_duty_cycle: Option<u8>,
    /// How far the first receive window sits below the uplink rate.
    #[pyo3(get)]
    rx1_offset: Option<u8>,
    /// The rate of the second receive window.
    #[pyo3(get)]
    rx2_data_rate: Option<u8>,
    /// A frequency in hertz, for the receive window and the channel commands.
    #[pyo3(get)]
    frequency_hz: Option<u32>,
    /// Whether the window offset was in range.
    #[pyo3(get)]
    rx1_offset_ack: Option<bool>,
    /// Whether the window rate was known.
    #[pyo3(get)]
    rx2_data_rate_ack: Option<bool>,
    /// Whether the frequency was usable.
    #[pyo3(get)]
    channel_ack: Option<bool>,
    /// A device battery level: 0 on external power, 255 when it cannot tell.
    #[pyo3(get)]
    battery: Option<u8>,
    /// The signal-to-noise ratio of the last request, in dB.
    #[pyo3(get)]
    snr_margin: Option<i8>,
    /// Which channel a channel command names.
    #[pyo3(get)]
    index: Option<u8>,
    /// The fastest rate allowed on it.
    #[pyo3(get)]
    max_data_rate: Option<u8>,
    /// The slowest rate allowed on it.
    #[pyo3(get)]
    min_data_rate: Option<u8>,
    /// Whether the device can run that range of rates.
    #[pyo3(get)]
    data_rate_range_ok: Option<bool>,
    /// Whether its radio can reach that frequency.
    #[pyo3(get)]
    frequency_ok: Option<bool>,
    /// How long a device waits before its first receive window, as the command codes it.
    #[pyo3(get)]
    delay: Option<u8>,
    /// The coded transmit power ceiling a region imposes.
    #[pyo3(get)]
    max_eirp: Option<u8>,
    /// Whether an uplink is held to 400 ms of air time.
    #[pyo3(get)]
    uplink_dwell: Option<bool>,
    /// Whether a downlink is.
    #[pyo3(get)]
    downlink_dwell: Option<bool>,
    /// Whether the channel already had an uplink frequency to pair a downlink with.
    #[pyo3(get)]
    uplink_frequency_exists: Option<bool>,
    /// Seconds since the GPS epoch.
    #[pyo3(get)]
    seconds: Option<u32>,
    /// The fraction of that second, in steps of one part in 256.
    #[pyo3(get)]
    fraction: Option<u8>,
    /// Whether a relay runs.
    #[pyo3(get)]
    enabled: Option<bool>,
    /// How often a relay scans, as TS011-1.0.1 table 18 codes it.
    #[pyo3(get)]
    cad_periodicity: Option<u8>,
    /// Which of the region's relay channels is a relay's default one.
    #[pyo3(get)]
    default_channel_index: Option<u8>,
    /// Whether a relay configuration sets a second channel, 1 for yes.
    #[pyo3(get)]
    second_channel_index: Option<u8>,
    /// The second channel's data rate; its frequency is the frequency field.
    #[pyo3(get)]
    second_channel_data_rate: Option<u8>,
    /// How far above its frequency the second channel is acknowledged, as table 35 codes it.
    #[pyo3(get)]
    second_channel_ack_offset: Option<u8>,
    /// Whether the scan period was valid.
    #[pyo3(get)]
    cad_periodicity_ack: Option<bool>,
    /// Whether the default channel was valid.
    #[pyo3(get)]
    default_channel_index_ack: Option<bool>,
    /// Whether the second channel index was valid.
    #[pyo3(get)]
    second_channel_index_ack: Option<bool>,
    /// Whether the second channel's data rate was valid.
    #[pyo3(get)]
    second_channel_data_rate_ack: Option<bool>,
    /// Whether its acknowledgment offset was valid.
    #[pyo3(get)]
    second_channel_ack_offset_ack: Option<bool>,
    /// Whether its frequency was valid.
    #[pyo3(get)]
    second_channel_frequency_ack: Option<bool>,
    /// How an end device uses a relay, as TS011-1.0.1 table 40 codes it.
    #[pyo3(get)]
    relay_mode: Option<u8>,
    /// How many unanswered uplinks turn relaying on, as table 41 codes it.
    #[pyo3(get)]
    smart_enable_level: Option<u8>,
    /// How many WOR frames without an acknowledgment before an uplink goes anyway.
    #[pyo3(get)]
    back_off: Option<u8>,
    /// What a join filter rule does, or whether a trusted end device is read or removed.
    #[pyo3(get)]
    action: Option<u8>,
    /// How many leading bytes of JoinEUI and DevEUI a join filter rule matches.
    #[pyo3(get)]
    eui_len: Option<u8>,
    /// Those bytes, most significant first, with the rest zero.
    #[pyo3(get)]
    eui: Option<Vec<u8>>,
    /// Whether a join filter rule was one to create, change or remove.
    #[pyo3(get)]
    combined_rules_ack: Option<bool>,
    /// Whether its length was valid.
    #[pyo3(get)]
    eui_len_ack: Option<bool>,
    /// Whether its action was valid.
    #[pyo3(get)]
    action_ack: Option<bool>,
    /// Tokens a trusted end device earns an hour, 63 for no limit.
    #[pyo3(get)]
    reload_rate: Option<u8>,
    /// Its bucket size multiplier, as TS011-1.0.1 table 55 codes it.
    #[pyo3(get)]
    bucket_size: Option<u8>,
    /// An end device address a relay command names.
    #[pyo3(get)]
    dev_addr: Option<u32>,
    /// A wake-on-radio frame counter.
    #[pyo3(get)]
    wfcnt: Option<u32>,
    /// An end device's root relay session key.
    #[pyo3(get)]
    root_wor_s_key: Option<Vec<u8>>,
    /// Whether a trusted list entry was in use.
    #[pyo3(get)]
    index_ack: Option<bool>,
    /// What a forwarding limit command does to a relay's token counters, as table 63 codes it.
    #[pyo3(get)]
    reset_limit_counters: Option<u8>,
    /// Join requests a relay forwards an hour, 127 for no limit.
    #[pyo3(get)]
    join_request_reload_rate: Option<u8>,
    /// New end device notifications a relay sends an hour.
    #[pyo3(get)]
    notify_reload_rate: Option<u8>,
    /// Uplinks a relay forwards an hour across every trusted end device.
    #[pyo3(get)]
    global_uplink_reload_rate: Option<u8>,
    /// Every message a relay sends an hour.
    #[pyo3(get)]
    overall_reload_rate: Option<u8>,
    /// The join request bucket size multiplier.
    #[pyo3(get)]
    join_request_bucket_size: Option<u8>,
    /// The notification bucket size multiplier.
    #[pyo3(get)]
    notify_bucket_size: Option<u8>,
    /// The global uplink bucket size multiplier.
    #[pyo3(get)]
    global_uplink_bucket_size: Option<u8>,
    /// The overall bucket size multiplier.
    #[pyo3(get)]
    overall_bucket_size: Option<u8>,
    /// The signal strength of a WOR frame a relay could not verify, in dBm.
    #[pyo3(get)]
    rssi_dbm: Option<i16>,
    /// Its signal-to-noise ratio, in dB.
    #[pyo3(get)]
    snr_db: Option<i8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanMacCommand {
    /// Builds a command to write out.
    ///
    /// The identifier and the direction decide which command this is, and therefore which of
    /// the other arguments are read. The rest may be left off.
    #[new]
    #[pyo3(signature = (
        cid,
        direction,
        margin = None,
        gateways = None,
        data_rate = None,
        tx_power = None,
        channel_mask = None,
        mask_control = None,
        transmissions = None,
        power_ack = None,
        data_rate_ack = None,
        channel_mask_ack = None,
        max_duty_cycle = None,
        rx1_offset = None,
        rx2_data_rate = None,
        frequency_hz = None,
        rx1_offset_ack = None,
        rx2_data_rate_ack = None,
        channel_ack = None,
        battery = None,
        snr_margin = None,
        index = None,
        max_data_rate = None,
        min_data_rate = None,
        data_rate_range_ok = None,
        frequency_ok = None,
        delay = None,
        max_eirp = None,
        uplink_dwell = None,
        downlink_dwell = None,
        uplink_frequency_exists = None,
        seconds = None,
        fraction = None,
        enabled = None,
        cad_periodicity = None,
        default_channel_index = None,
        second_channel_index = None,
        second_channel_data_rate = None,
        second_channel_ack_offset = None,
        cad_periodicity_ack = None,
        default_channel_index_ack = None,
        second_channel_index_ack = None,
        second_channel_data_rate_ack = None,
        second_channel_ack_offset_ack = None,
        second_channel_frequency_ack = None,
        relay_mode = None,
        smart_enable_level = None,
        back_off = None,
        action = None,
        eui_len = None,
        eui = None,
        combined_rules_ack = None,
        eui_len_ack = None,
        action_ack = None,
        reload_rate = None,
        bucket_size = None,
        dev_addr = None,
        wfcnt = None,
        root_wor_s_key = None,
        index_ack = None,
        reset_limit_counters = None,
        join_request_reload_rate = None,
        notify_reload_rate = None,
        global_uplink_reload_rate = None,
        overall_reload_rate = None,
        join_request_bucket_size = None,
        notify_bucket_size = None,
        global_uplink_bucket_size = None,
        overall_bucket_size = None,
        rssi_dbm = None,
        snr_db = None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        cid: u8,
        direction: &str,
        margin: Option<u8>,
        gateways: Option<u8>,
        data_rate: Option<u8>,
        tx_power: Option<u8>,
        channel_mask: Option<u16>,
        mask_control: Option<u8>,
        transmissions: Option<u8>,
        power_ack: Option<bool>,
        data_rate_ack: Option<bool>,
        channel_mask_ack: Option<bool>,
        max_duty_cycle: Option<u8>,
        rx1_offset: Option<u8>,
        rx2_data_rate: Option<u8>,
        frequency_hz: Option<u32>,
        rx1_offset_ack: Option<bool>,
        rx2_data_rate_ack: Option<bool>,
        channel_ack: Option<bool>,
        battery: Option<u8>,
        snr_margin: Option<i8>,
        index: Option<u8>,
        max_data_rate: Option<u8>,
        min_data_rate: Option<u8>,
        data_rate_range_ok: Option<bool>,
        frequency_ok: Option<bool>,
        delay: Option<u8>,
        max_eirp: Option<u8>,
        uplink_dwell: Option<bool>,
        downlink_dwell: Option<bool>,
        uplink_frequency_exists: Option<bool>,
        seconds: Option<u32>,
        fraction: Option<u8>,
        enabled: Option<bool>,
        cad_periodicity: Option<u8>,
        default_channel_index: Option<u8>,
        second_channel_index: Option<u8>,
        second_channel_data_rate: Option<u8>,
        second_channel_ack_offset: Option<u8>,
        cad_periodicity_ack: Option<bool>,
        default_channel_index_ack: Option<bool>,
        second_channel_index_ack: Option<bool>,
        second_channel_data_rate_ack: Option<bool>,
        second_channel_ack_offset_ack: Option<bool>,
        second_channel_frequency_ack: Option<bool>,
        relay_mode: Option<u8>,
        smart_enable_level: Option<u8>,
        back_off: Option<u8>,
        action: Option<u8>,
        eui_len: Option<u8>,
        eui: Option<Vec<u8>>,
        combined_rules_ack: Option<bool>,
        eui_len_ack: Option<bool>,
        action_ack: Option<bool>,
        reload_rate: Option<u8>,
        bucket_size: Option<u8>,
        dev_addr: Option<u32>,
        wfcnt: Option<u32>,
        root_wor_s_key: Option<Vec<u8>>,
        index_ack: Option<bool>,
        reset_limit_counters: Option<u8>,
        join_request_reload_rate: Option<u8>,
        notify_reload_rate: Option<u8>,
        global_uplink_reload_rate: Option<u8>,
        overall_reload_rate: Option<u8>,
        join_request_bucket_size: Option<u8>,
        notify_bucket_size: Option<u8>,
        global_uplink_bucket_size: Option<u8>,
        overall_bucket_size: Option<u8>,
        rssi_dbm: Option<i16>,
        snr_db: Option<i8>,
    ) -> LorawanMacCommand {
        LorawanMacCommand {
            kind: String::new(),
            cid,
            direction: direction.to_owned(),
            margin,
            gateways,
            data_rate,
            tx_power,
            channel_mask,
            mask_control,
            transmissions,
            power_ack,
            data_rate_ack,
            channel_mask_ack,
            max_duty_cycle,
            rx1_offset,
            rx2_data_rate,
            frequency_hz,
            rx1_offset_ack,
            rx2_data_rate_ack,
            channel_ack,
            battery,
            snr_margin,
            index,
            max_data_rate,
            min_data_rate,
            data_rate_range_ok,
            frequency_ok,
            delay,
            max_eirp,
            uplink_dwell,
            downlink_dwell,
            uplink_frequency_exists,
            seconds,
            fraction,
            enabled,
            cad_periodicity,
            default_channel_index,
            second_channel_index,
            second_channel_data_rate,
            second_channel_ack_offset,
            cad_periodicity_ack,
            default_channel_index_ack,
            second_channel_index_ack,
            second_channel_data_rate_ack,
            second_channel_ack_offset_ack,
            second_channel_frequency_ack,
            relay_mode,
            smart_enable_level,
            back_off,
            action,
            eui_len,
            eui,
            combined_rules_ack,
            eui_len_ack,
            action_ack,
            reload_rate,
            bucket_size,
            dev_addr,
            wfcnt,
            root_wor_s_key,
            index_ack,
            reset_limit_counters,
            join_request_reload_rate,
            notify_reload_rate,
            global_uplink_reload_rate,
            overall_reload_rate,
            join_request_bucket_size,
            notify_bucket_size,
            global_uplink_bucket_size,
            overall_bucket_size,
            rssi_dbm,
            snr_db,
        }
    }

    /// Writes this command out.
    ///
    /// # Returns
    ///
    /// The bytes it goes out as.
    ///
    /// # Errors
    ///
    /// When the identifier and the direction name no command, or a field will not fit what
    /// carries it.
    fn encode<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let built = rebuild_command(self)?;
        let mut out = [0u8; pamoja_lorawan::mac::MAX_COMMAND];
        let written = built
            .encode(&mut out)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        Ok(PyBytes::new(py, &out[..written]))
    }
}

fn blank_mac(kind: &str, command: &MacCommand) -> LorawanMacCommand {
    LorawanMacCommand {
        kind: kind.to_owned(),
        cid: command.cid(),
        direction: match command.direction() {
            Direction::Uplink => "Uplink".to_owned(),
            Direction::Downlink => "Downlink".to_owned(),
        },
        margin: None,
        gateways: None,
        data_rate: None,
        tx_power: None,
        channel_mask: None,
        mask_control: None,
        transmissions: None,
        power_ack: None,
        data_rate_ack: None,
        channel_mask_ack: None,
        max_duty_cycle: None,
        rx1_offset: None,
        rx2_data_rate: None,
        frequency_hz: None,
        rx1_offset_ack: None,
        rx2_data_rate_ack: None,
        channel_ack: None,
        battery: None,
        snr_margin: None,
        index: None,
        max_data_rate: None,
        min_data_rate: None,
        data_rate_range_ok: None,
        frequency_ok: None,
        delay: None,
        max_eirp: None,
        uplink_dwell: None,
        downlink_dwell: None,
        uplink_frequency_exists: None,
        seconds: None,
        fraction: None,
        enabled: None,
        cad_periodicity: None,
        default_channel_index: None,
        second_channel_index: None,
        second_channel_data_rate: None,
        second_channel_ack_offset: None,
        cad_periodicity_ack: None,
        default_channel_index_ack: None,
        second_channel_index_ack: None,
        second_channel_data_rate_ack: None,
        second_channel_ack_offset_ack: None,
        second_channel_frequency_ack: None,
        relay_mode: None,
        smart_enable_level: None,
        back_off: None,
        action: None,
        eui_len: None,
        eui: None,
        combined_rules_ack: None,
        eui_len_ack: None,
        action_ack: None,
        reload_rate: None,
        bucket_size: None,
        dev_addr: None,
        wfcnt: None,
        root_wor_s_key: None,
        index_ack: None,
        reset_limit_counters: None,
        join_request_reload_rate: None,
        notify_reload_rate: None,
        global_uplink_reload_rate: None,
        overall_reload_rate: None,
        join_request_bucket_size: None,
        notify_bucket_size: None,
        global_uplink_bucket_size: None,
        overall_bucket_size: None,
        rssi_dbm: None,
        snr_db: None,
    }
}

fn describe_mac(command: MacCommand) -> LorawanMacCommand {
    match command {
        MacCommand::LinkCheckReq => blank_mac("link_check_req", &command),
        MacCommand::LinkCheckAns { margin, gateways } => {
            let mut out = blank_mac("link_check_ans", &command);
            out.margin = Some(margin);
            out.gateways = Some(gateways);
            out
        }
        MacCommand::LinkAdrReq {
            data_rate,
            tx_power,
            channel_mask,
            mask_control,
            transmissions,
        } => {
            let mut out = blank_mac("link_adr_req", &command);
            out.data_rate = Some(data_rate);
            out.tx_power = Some(tx_power);
            out.channel_mask = Some(channel_mask);
            out.mask_control = Some(mask_control);
            out.transmissions = Some(transmissions);
            out
        }
        MacCommand::LinkAdrAns {
            power_ack,
            data_rate_ack,
            channel_mask_ack,
        } => {
            let mut out = blank_mac("link_adr_ans", &command);
            out.power_ack = Some(power_ack);
            out.data_rate_ack = Some(data_rate_ack);
            out.channel_mask_ack = Some(channel_mask_ack);
            out
        }
        MacCommand::DutyCycleReq { max_duty_cycle } => {
            let mut out = blank_mac("duty_cycle_req", &command);
            out.max_duty_cycle = Some(max_duty_cycle);
            out
        }
        MacCommand::DutyCycleAns => blank_mac("duty_cycle_ans", &command),
        MacCommand::RxParamSetupReq {
            rx1_offset,
            rx2_data_rate,
            frequency_hz,
        } => {
            let mut out = blank_mac("rx_param_setup_req", &command);
            out.rx1_offset = Some(rx1_offset);
            out.rx2_data_rate = Some(rx2_data_rate);
            out.frequency_hz = Some(frequency_hz);
            out
        }
        MacCommand::RxParamSetupAns {
            rx1_offset_ack,
            rx2_data_rate_ack,
            channel_ack,
        } => {
            let mut out = blank_mac("rx_param_setup_ans", &command);
            out.rx1_offset_ack = Some(rx1_offset_ack);
            out.rx2_data_rate_ack = Some(rx2_data_rate_ack);
            out.channel_ack = Some(channel_ack);
            out
        }
        MacCommand::DevStatusReq => blank_mac("dev_status_req", &command),
        MacCommand::DevStatusAns { battery, margin } => {
            let mut out = blank_mac("dev_status_ans", &command);
            out.battery = Some(battery);
            out.snr_margin = Some(margin);
            out
        }
        MacCommand::NewChannelReq {
            index,
            frequency_hz,
            max_data_rate,
            min_data_rate,
        } => {
            let mut out = blank_mac("new_channel_req", &command);
            out.index = Some(index);
            out.frequency_hz = Some(frequency_hz);
            out.max_data_rate = Some(max_data_rate);
            out.min_data_rate = Some(min_data_rate);
            out
        }
        MacCommand::NewChannelAns {
            data_rate_range_ok,
            frequency_ok,
        } => {
            let mut out = blank_mac("new_channel_ans", &command);
            out.data_rate_range_ok = Some(data_rate_range_ok);
            out.frequency_ok = Some(frequency_ok);
            out
        }
        MacCommand::RxTimingSetupReq { delay } => {
            let mut out = blank_mac("rx_timing_setup_req", &command);
            out.delay = Some(delay);
            out
        }
        MacCommand::RxTimingSetupAns => blank_mac("rx_timing_setup_ans", &command),
        MacCommand::TxParamSetupReq {
            max_eirp,
            uplink_dwell,
            downlink_dwell,
        } => {
            let mut out = blank_mac("tx_param_setup_req", &command);
            out.max_eirp = Some(max_eirp);
            out.uplink_dwell = Some(uplink_dwell);
            out.downlink_dwell = Some(downlink_dwell);
            out
        }
        MacCommand::TxParamSetupAns => blank_mac("tx_param_setup_ans", &command),
        MacCommand::DlChannelReq {
            index,
            frequency_hz,
        } => {
            let mut out = blank_mac("dl_channel_req", &command);
            out.index = Some(index);
            out.frequency_hz = Some(frequency_hz);
            out
        }
        MacCommand::DlChannelAns {
            uplink_frequency_exists,
            frequency_ok,
        } => {
            let mut out = blank_mac("dl_channel_ans", &command);
            out.uplink_frequency_exists = Some(uplink_frequency_exists);
            out.frequency_ok = Some(frequency_ok);
            out
        }
        MacCommand::DeviceTimeReq => blank_mac("device_time_req", &command),
        MacCommand::DeviceTimeAns { seconds, fraction } => {
            let mut out = blank_mac("device_time_ans", &command);
            out.seconds = Some(seconds);
            out.fraction = Some(fraction);
            out
        }
        MacCommand::RelayConfReq {
            enabled,
            cad_periodicity,
            default_channel_index,
            second_channel_index,
            second_channel_data_rate,
            second_channel_ack_offset,
            second_channel_frequency_hz,
        } => {
            let mut out = blank_mac("relay_conf_req", &command);
            out.enabled = Some(enabled);
            out.cad_periodicity = Some(cad_periodicity);
            out.default_channel_index = Some(default_channel_index);
            out.second_channel_index = Some(second_channel_index);
            out.second_channel_data_rate = Some(second_channel_data_rate);
            out.second_channel_ack_offset = Some(second_channel_ack_offset);
            out.frequency_hz = Some(second_channel_frequency_hz);
            out
        }
        MacCommand::RelayConfAns {
            cad_periodicity_ack,
            default_channel_index_ack,
            second_channel_index_ack,
            second_channel_data_rate_ack,
            second_channel_ack_offset_ack,
            second_channel_frequency_ack,
        } => {
            let mut out = blank_mac("relay_conf_ans", &command);
            out.cad_periodicity_ack = Some(cad_periodicity_ack);
            out.default_channel_index_ack = Some(default_channel_index_ack);
            out.second_channel_index_ack = Some(second_channel_index_ack);
            out.second_channel_data_rate_ack = Some(second_channel_data_rate_ack);
            out.second_channel_ack_offset_ack = Some(second_channel_ack_offset_ack);
            out.second_channel_frequency_ack = Some(second_channel_frequency_ack);
            out
        }
        MacCommand::EndDeviceConfReq {
            relay_mode,
            smart_enable_level,
            back_off,
            second_channel_index,
            second_channel_data_rate,
            second_channel_ack_offset,
            second_channel_frequency_hz,
        } => {
            let mut out = blank_mac("end_device_conf_req", &command);
            out.relay_mode = Some(relay_mode);
            out.smart_enable_level = Some(smart_enable_level);
            out.back_off = Some(back_off);
            out.second_channel_index = Some(second_channel_index);
            out.second_channel_data_rate = Some(second_channel_data_rate);
            out.second_channel_ack_offset = Some(second_channel_ack_offset);
            out.frequency_hz = Some(second_channel_frequency_hz);
            out
        }
        MacCommand::EndDeviceConfAns {
            second_channel_ack_offset_ack,
            second_channel_index_ack,
            second_channel_data_rate_ack,
            second_channel_frequency_ack,
        } => {
            let mut out = blank_mac("end_device_conf_ans", &command);
            out.second_channel_ack_offset_ack = Some(second_channel_ack_offset_ack);
            out.second_channel_index_ack = Some(second_channel_index_ack);
            out.second_channel_data_rate_ack = Some(second_channel_data_rate_ack);
            out.second_channel_frequency_ack = Some(second_channel_frequency_ack);
            out
        }
        MacCommand::FilterListReq {
            index,
            action,
            eui_len,
            eui,
        } => {
            let mut out = blank_mac("filter_list_req", &command);
            out.index = Some(index);
            out.action = Some(action);
            out.eui_len = Some(eui_len);
            out.eui = Some(eui.to_vec());
            out
        }
        MacCommand::FilterListAns {
            combined_rules_ack,
            eui_len_ack,
            action_ack,
        } => {
            let mut out = blank_mac("filter_list_ans", &command);
            out.combined_rules_ack = Some(combined_rules_ack);
            out.eui_len_ack = Some(eui_len_ack);
            out.action_ack = Some(action_ack);
            out
        }
        MacCommand::UpdateUplinkListReq {
            index,
            reload_rate,
            bucket_size,
            dev_addr,
            wfcnt,
            root_wor_s_key,
        } => {
            let mut out = blank_mac("update_uplink_list_req", &command);
            out.index = Some(index);
            out.reload_rate = Some(reload_rate);
            out.bucket_size = Some(bucket_size);
            out.dev_addr = Some(dev_addr);
            out.wfcnt = Some(wfcnt);
            out.root_wor_s_key = Some(root_wor_s_key.to_vec());
            out
        }
        MacCommand::UpdateUplinkListAns => blank_mac("update_uplink_list_ans", &command),
        MacCommand::CtrlUplinkListReq { index, action } => {
            let mut out = blank_mac("ctrl_uplink_list_req", &command);
            out.index = Some(index);
            out.action = Some(action);
            out
        }
        MacCommand::CtrlUplinkListAns { index_ack, wfcnt } => {
            let mut out = blank_mac("ctrl_uplink_list_ans", &command);
            out.index_ack = Some(index_ack);
            out.wfcnt = Some(wfcnt);
            out
        }
        MacCommand::ConfigureFwdLimitReq {
            reset_limit_counters,
            join_request_reload_rate,
            notify_reload_rate,
            global_uplink_reload_rate,
            overall_reload_rate,
            join_request_bucket_size,
            notify_bucket_size,
            global_uplink_bucket_size,
            overall_bucket_size,
        } => {
            let mut out = blank_mac("configure_fwd_limit_req", &command);
            out.reset_limit_counters = Some(reset_limit_counters);
            out.join_request_reload_rate = Some(join_request_reload_rate);
            out.notify_reload_rate = Some(notify_reload_rate);
            out.global_uplink_reload_rate = Some(global_uplink_reload_rate);
            out.overall_reload_rate = Some(overall_reload_rate);
            out.join_request_bucket_size = Some(join_request_bucket_size);
            out.notify_bucket_size = Some(notify_bucket_size);
            out.global_uplink_bucket_size = Some(global_uplink_bucket_size);
            out.overall_bucket_size = Some(overall_bucket_size);
            out
        }
        MacCommand::ConfigureFwdLimitAns => blank_mac("configure_fwd_limit_ans", &command),
        MacCommand::NotifyNewEndDeviceReq {
            dev_addr,
            rssi_dbm,
            snr_db,
        } => {
            let mut out = blank_mac("notify_new_end_device_req", &command);
            out.dev_addr = Some(dev_addr);
            out.rssi_dbm = Some(rssi_dbm);
            out.snr_db = Some(snr_db);
            out
        }
    }
}

// Sixteen bytes of a relay command, a filter prefix or a key, zero when absent.
fn sixteen(bytes: Option<&[u8]>, name: &str) -> PyResult<[u8; 16]> {
    match bytes {
        None => Ok([0; 16]),
        Some(bytes) => <[u8; 16]>::try_from(bytes)
            .map_err(|_| PamojaError::new_err(format!("{name} must be exactly 16 bytes"))),
    }
}

fn rebuild_command(command: &LorawanMacCommand) -> PyResult<MacCommand> {
    use pamoja_lorawan::mac;

    let down = command.direction.eq_ignore_ascii_case("downlink");
    let byte = |value: Option<u8>| value.unwrap_or(0);
    let flag = |value: Option<bool>| value.unwrap_or(false);

    let built = match (command.cid, down) {
        (mac::CID_LINK_CHECK, false) => MacCommand::LinkCheckReq,
        (mac::CID_LINK_CHECK, true) => MacCommand::LinkCheckAns {
            margin: byte(command.margin),
            gateways: byte(command.gateways),
        },
        (mac::CID_LINK_ADR, true) => MacCommand::LinkAdrReq {
            data_rate: byte(command.data_rate),
            tx_power: byte(command.tx_power),
            channel_mask: command.channel_mask.unwrap_or(0),
            mask_control: byte(command.mask_control),
            transmissions: byte(command.transmissions),
        },
        (mac::CID_LINK_ADR, false) => MacCommand::LinkAdrAns {
            power_ack: flag(command.power_ack),
            data_rate_ack: flag(command.data_rate_ack),
            channel_mask_ack: flag(command.channel_mask_ack),
        },
        (mac::CID_DUTY_CYCLE, true) => MacCommand::DutyCycleReq {
            max_duty_cycle: byte(command.max_duty_cycle),
        },
        (mac::CID_DUTY_CYCLE, false) => MacCommand::DutyCycleAns,
        (mac::CID_RX_PARAM_SETUP, true) => MacCommand::RxParamSetupReq {
            rx1_offset: byte(command.rx1_offset),
            rx2_data_rate: byte(command.rx2_data_rate),
            frequency_hz: command.frequency_hz.unwrap_or(0),
        },
        (mac::CID_RX_PARAM_SETUP, false) => MacCommand::RxParamSetupAns {
            rx1_offset_ack: flag(command.rx1_offset_ack),
            rx2_data_rate_ack: flag(command.rx2_data_rate_ack),
            channel_ack: flag(command.channel_ack),
        },
        (mac::CID_DEV_STATUS, true) => MacCommand::DevStatusReq,
        (mac::CID_DEV_STATUS, false) => MacCommand::DevStatusAns {
            battery: byte(command.battery),
            margin: command.snr_margin.unwrap_or(0),
        },
        (mac::CID_NEW_CHANNEL, true) => MacCommand::NewChannelReq {
            index: byte(command.index),
            frequency_hz: command.frequency_hz.unwrap_or(0),
            max_data_rate: byte(command.max_data_rate),
            min_data_rate: byte(command.min_data_rate),
        },
        (mac::CID_NEW_CHANNEL, false) => MacCommand::NewChannelAns {
            data_rate_range_ok: flag(command.data_rate_range_ok),
            frequency_ok: flag(command.frequency_ok),
        },
        (mac::CID_RX_TIMING_SETUP, true) => MacCommand::RxTimingSetupReq {
            delay: byte(command.delay),
        },
        (mac::CID_RX_TIMING_SETUP, false) => MacCommand::RxTimingSetupAns,
        (mac::CID_TX_PARAM_SETUP, true) => MacCommand::TxParamSetupReq {
            max_eirp: byte(command.max_eirp),
            uplink_dwell: flag(command.uplink_dwell),
            downlink_dwell: flag(command.downlink_dwell),
        },
        (mac::CID_TX_PARAM_SETUP, false) => MacCommand::TxParamSetupAns,
        (mac::CID_DL_CHANNEL, true) => MacCommand::DlChannelReq {
            index: byte(command.index),
            frequency_hz: command.frequency_hz.unwrap_or(0),
        },
        (mac::CID_DL_CHANNEL, false) => MacCommand::DlChannelAns {
            uplink_frequency_exists: flag(command.uplink_frequency_exists),
            frequency_ok: flag(command.frequency_ok),
        },
        (mac::CID_DEVICE_TIME, false) => MacCommand::DeviceTimeReq,
        (mac::CID_DEVICE_TIME, true) => MacCommand::DeviceTimeAns {
            seconds: command.seconds.unwrap_or(0),
            fraction: byte(command.fraction),
        },
        (mac::CID_RELAY_CONF, true) => MacCommand::RelayConfReq {
            enabled: flag(command.enabled),
            cad_periodicity: byte(command.cad_periodicity),
            default_channel_index: byte(command.default_channel_index),
            second_channel_index: byte(command.second_channel_index),
            second_channel_data_rate: byte(command.second_channel_data_rate),
            second_channel_ack_offset: byte(command.second_channel_ack_offset),
            second_channel_frequency_hz: command.frequency_hz.unwrap_or(0),
        },
        (mac::CID_RELAY_CONF, false) => MacCommand::RelayConfAns {
            cad_periodicity_ack: flag(command.cad_periodicity_ack),
            default_channel_index_ack: flag(command.default_channel_index_ack),
            second_channel_index_ack: flag(command.second_channel_index_ack),
            second_channel_data_rate_ack: flag(command.second_channel_data_rate_ack),
            second_channel_ack_offset_ack: flag(command.second_channel_ack_offset_ack),
            second_channel_frequency_ack: flag(command.second_channel_frequency_ack),
        },
        (mac::CID_END_DEVICE_CONF, true) => MacCommand::EndDeviceConfReq {
            relay_mode: byte(command.relay_mode),
            smart_enable_level: byte(command.smart_enable_level),
            back_off: byte(command.back_off),
            second_channel_index: byte(command.second_channel_index),
            second_channel_data_rate: byte(command.second_channel_data_rate),
            second_channel_ack_offset: byte(command.second_channel_ack_offset),
            second_channel_frequency_hz: command.frequency_hz.unwrap_or(0),
        },
        (mac::CID_END_DEVICE_CONF, false) => MacCommand::EndDeviceConfAns {
            second_channel_ack_offset_ack: flag(command.second_channel_ack_offset_ack),
            second_channel_index_ack: flag(command.second_channel_index_ack),
            second_channel_data_rate_ack: flag(command.second_channel_data_rate_ack),
            second_channel_frequency_ack: flag(command.second_channel_frequency_ack),
        },
        (mac::CID_FILTER_LIST, true) => MacCommand::FilterListReq {
            index: byte(command.index),
            action: byte(command.action),
            eui_len: byte(command.eui_len),
            eui: sixteen(command.eui.as_deref(), "eui")?,
        },
        (mac::CID_FILTER_LIST, false) => MacCommand::FilterListAns {
            combined_rules_ack: flag(command.combined_rules_ack),
            eui_len_ack: flag(command.eui_len_ack),
            action_ack: flag(command.action_ack),
        },
        (mac::CID_UPDATE_UPLINK_LIST, true) => MacCommand::UpdateUplinkListReq {
            index: byte(command.index),
            reload_rate: byte(command.reload_rate),
            bucket_size: byte(command.bucket_size),
            dev_addr: command.dev_addr.unwrap_or(0),
            wfcnt: command.wfcnt.unwrap_or(0),
            root_wor_s_key: sixteen(command.root_wor_s_key.as_deref(), "root_wor_s_key")?,
        },
        (mac::CID_UPDATE_UPLINK_LIST, false) => MacCommand::UpdateUplinkListAns,
        (mac::CID_CTRL_UPLINK_LIST, true) => MacCommand::CtrlUplinkListReq {
            index: byte(command.index),
            action: byte(command.action),
        },
        (mac::CID_CTRL_UPLINK_LIST, false) => MacCommand::CtrlUplinkListAns {
            index_ack: flag(command.index_ack),
            wfcnt: command.wfcnt.unwrap_or(0),
        },
        (mac::CID_CONFIGURE_FWD_LIMIT, true) => MacCommand::ConfigureFwdLimitReq {
            reset_limit_counters: byte(command.reset_limit_counters),
            join_request_reload_rate: byte(command.join_request_reload_rate),
            notify_reload_rate: byte(command.notify_reload_rate),
            global_uplink_reload_rate: byte(command.global_uplink_reload_rate),
            overall_reload_rate: byte(command.overall_reload_rate),
            join_request_bucket_size: byte(command.join_request_bucket_size),
            notify_bucket_size: byte(command.notify_bucket_size),
            global_uplink_bucket_size: byte(command.global_uplink_bucket_size),
            overall_bucket_size: byte(command.overall_bucket_size),
        },
        (mac::CID_CONFIGURE_FWD_LIMIT, false) => MacCommand::ConfigureFwdLimitAns,
        (mac::CID_NOTIFY_NEW_END_DEVICE, false) => MacCommand::NotifyNewEndDeviceReq {
            dev_addr: command.dev_addr.unwrap_or(0),
            rssi_dbm: command.rssi_dbm.unwrap_or(0),
            snr_db: command.snr_db.unwrap_or(0),
        },
        _ => {
            return Err(PamojaError::new_err(format!(
                "identifier {:#04x} names no command in that direction",
                command.cid
            )))
        }
    };
    Ok(built)
}

/// Reads the commands packed into a frame options field, or a payload sent on port 0.
///
/// The same identifier means a different command in each direction, so the direction decides
/// what is read and there is no default.
///
/// A command does not carry its own length, so one this build does not know cannot be
/// stepped over. Reading stops there and returns what came before it.
///
/// # Arguments
///
/// * `direction` - `Uplink` or `Downlink`.
/// * `data` - the options field, or the payload.
///
/// # Returns
///
/// The commands that were readable, in order.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_mac_parse(direction: &str, data: Vec<u8>) -> Vec<LorawanMacCommand> {
    let travel = if direction.eq_ignore_ascii_case("downlink") {
        Direction::Downlink
    } else {
        Direction::Uplink
    };
    MacCommands::new(travel, &data)
        .take_while(Result::is_ok)
        .filter_map(Result::ok)
        .map(describe_mac)
        .collect()
}
