//! Generated Python bindings for the LoRa Basics Station protocol.
//!
//! These mirror the `pamoja_gateway::station` Rust API: the messages a station and its network
//! server exchange over a websocket, and the frame split a station does before it reports what
//! it heard. Nothing here opens a socket, so a program brings its own websocket, sends what
//! :func:`station_encode` writes, and reads whatever arrives with :func:`station_parse`.
//!
//! Frequencies are in hertz and payloads are ``bytes`` rather than hexadecimal text, so nothing
//! has to be formatted by hand.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_gateway::station::{eui_of, id6, Broadcast, Discovery, Levels, Message, Router};
use pamoja_gateway::udp::Eui;

use crate::PamojaError;

/// How a station heard a packet, as it reports it.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayStationLevels {
    /// The radio the packet arrived on, which an answer goes back out on.
    #[pyo3(get)]
    rctx: i64,
    /// The station clock, in microseconds.
    #[pyo3(get)]
    xtime: i64,
    /// The GPS time, when the station has one.
    #[pyo3(get)]
    gpstime: Option<i64>,
    /// The received signal strength, in dBm.
    #[pyo3(get)]
    rssi: f64,
    /// The signal-to-noise ratio, in dB.
    #[pyo3(get)]
    snr: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayStationLevels {
    /// Describes how a packet was heard.
    #[new]
    #[pyo3(signature = (rctx=0, xtime=0, gpstime=None, rssi=0.0, snr=0.0))]
    fn new(rctx: i64, xtime: i64, gpstime: Option<i64>, rssi: f64, snr: f64) -> Self {
        Self {
            rctx,
            xtime,
            gpstime,
            rssi,
            snr,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "GatewayStationLevels(rctx={}, xtime={}, rssi={}, snr={})",
            self.rctx, self.xtime, self.rssi, self.snr
        )
    }
}

/// A receive window, or the ping slot a class B frame goes out in.
#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq)]
pub struct GatewayStationWindow {
    /// The data rate, as the network's table numbers it.
    #[pyo3(get)]
    data_rate: u8,
    /// The frequency in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayStationWindow {
    /// Describes a window by its data rate and frequency.
    #[new]
    #[pyo3(signature = (data_rate, frequency_hz))]
    fn new(data_rate: u8, frequency_hz: u32) -> Self {
        Self {
            data_rate,
            frequency_hz,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "GatewayStationWindow(data_rate={}, frequency_hz={})",
            self.data_rate, self.frequency_hz
        )
    }
}

/// One number of a configuration's data-rate table.
#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq)]
pub struct GatewayStationDataRate {
    /// The spreading factor, 0 for FSK.
    #[pyo3(get)]
    spreading_factor: u8,
    /// The bandwidth in hertz.
    #[pyo3(get)]
    bandwidth_hz: u32,
    /// Whether the rate is used only for downlinks.
    #[pyo3(get)]
    downlink_only: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayStationDataRate {
    /// Describes a data rate by its spreading factor and bandwidth.
    #[new]
    #[pyo3(signature = (spreading_factor, bandwidth_hz, downlink_only=false))]
    fn new(spreading_factor: u8, bandwidth_hz: u32, downlink_only: bool) -> Self {
        Self {
            spreading_factor,
            bandwidth_hz,
            downlink_only,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "GatewayStationDataRate(spreading_factor={}, bandwidth_hz={}, downlink_only={})",
            self.spreading_factor,
            self.bandwidth_hz,
            if self.downlink_only { "True" } else { "False" }
        )
    }
}

/// A range of join identifiers whose join requests a station forwards, both ends included.
#[gen_stub_pyclass]
#[pyclass(frozen, eq, skip_from_py_object)]
#[derive(Clone, PartialEq)]
pub struct GatewayStationJoinRange {
    /// The first identifier, as sixteen hexadecimal digits.
    #[pyo3(get)]
    first: String,
    /// The last identifier, as sixteen hexadecimal digits.
    #[pyo3(get)]
    last: String,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayStationJoinRange {
    /// Describes a range by its first and last identifier, each sixteen hexadecimal digits.
    #[new]
    #[pyo3(signature = (first, last))]
    fn new(first: String, last: String) -> PyResult<Self> {
        Ok(Self {
            first: identifier(&first)?.to_hex(),
            last: identifier(&last)?.to_hex(),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "GatewayStationJoinRange(first={}, last={})",
            self.first, self.last
        )
    }
}

/// One frame of a schedule, transmitted to a group rather than a device.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayStationBroadcast {
    /// The frame to transmit, kept for the `pdu` property.
    frame: Vec<u8>,
    /// The data rate to transmit at.
    #[pyo3(get)]
    data_rate: u8,
    /// The frequency to transmit on, in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// How urgent it is.
    #[pyo3(get)]
    priority: u8,
    /// The GPS time to transmit at.
    #[pyo3(get)]
    gpstime: Option<i64>,
    /// The radio to transmit on.
    #[pyo3(get)]
    rctx: Option<i64>,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayStationBroadcast {
    /// Describes one frame of a schedule.
    #[new]
    #[pyo3(signature = (pdu, data_rate=0, frequency_hz=0, priority=0, gpstime=None, rctx=None))]
    fn new(
        pdu: Vec<u8>,
        data_rate: u8,
        frequency_hz: u32,
        priority: u8,
        gpstime: Option<i64>,
        rctx: Option<i64>,
    ) -> Self {
        Self {
            frame: pdu,
            data_rate,
            frequency_hz,
            priority,
            gpstime,
            rctx,
        }
    }

    /// The frame to transmit.
    #[getter]
    fn pdu<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.frame)
    }

    fn __repr__(&self) -> String {
        format!(
            "GatewayStationBroadcast(len={}, data_rate={}, frequency_hz={})",
            self.frame.len(),
            self.data_rate,
            self.frequency_hz
        )
    }
}

/// A message either side of a session sends.
///
/// Every message names its kind in `msgtype`, and carries the fields that kind uses. The rest
/// read as ``None``. Build one with the keyword arguments of its kind and write it with
/// :func:`station_encode`.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayStationMessage {
    /// The kind as the protocol writes it, such as `jreq` or `updf`.
    #[pyo3(get)]
    msgtype: String,
    /// The station software, for a version.
    #[pyo3(get)]
    station: Option<String>,
    /// Its firmware, for a version.
    #[pyo3(get)]
    firmware: Option<String>,
    /// The package it came from, for a version.
    #[pyo3(get)]
    package: Option<String>,
    /// The hardware model, for a version.
    #[pyo3(get)]
    model: Option<String>,
    /// The protocol version it speaks, for a version.
    #[pyo3(get)]
    protocol: Option<u32>,
    /// What it can do, for a version.
    #[pyo3(get)]
    features: Option<String>,
    /// The networks whose data frames a configuration forwards, ``None`` to forward every
    /// network's. A station matches each against the top seven bits of a device address, so an
    /// empty list forwards no data frame at all.
    #[pyo3(get)]
    net_id: Option<Vec<u32>>,
    /// The join identifier ranges a configuration forwards; empty forwards every join.
    #[pyo3(get)]
    join_eui_ranges: Option<Vec<Py<GatewayStationJoinRange>>>,
    /// The region name, for a configuration.
    #[pyo3(get)]
    region: Option<String>,
    /// The highest radiated power the region allows, in dBm.
    #[pyo3(get)]
    max_eirp: Option<f64>,
    /// The concentrator the configuration is written for.
    #[pyo3(get)]
    hwspec: Option<String>,
    /// The lowest frequency the station may use, in hertz.
    #[pyo3(get)]
    freq_min: Option<u32>,
    /// The highest frequency the station may use, in hertz.
    #[pyo3(get)]
    freq_max: Option<u32>,
    /// A configuration's data rates, indexed by data-rate number, with ``None`` for a number
    /// the table leaves undefined.
    #[pyo3(get)]
    data_rates: Option<Vec<Option<Py<GatewayStationDataRate>>>>,
    /// The MAC header byte, for a join request or a data frame.
    #[pyo3(get)]
    mhdr: Option<u8>,
    /// The application being joined, as sixteen hexadecimal digits.
    #[pyo3(get)]
    join_eui: Option<String>,
    /// The device, as sixteen hexadecimal digits.
    #[pyo3(get)]
    dev_eui: Option<String>,
    /// The nonce a join request used.
    #[pyo3(get)]
    dev_nonce: Option<u16>,
    /// The address a data frame came from.
    #[pyo3(get)]
    dev_addr: Option<i32>,
    /// The frame control byte.
    #[pyo3(get)]
    fctrl: Option<u8>,
    /// The frame counter, as the sixteen bits on the air.
    #[pyo3(get)]
    fcnt: Option<u16>,
    /// The port, ``None`` for a frame carrying only options.
    #[pyo3(get)]
    fport: Option<u8>,
    /// The message integrity code.
    #[pyo3(get)]
    mic: Option<i32>,
    /// The data rate a frame arrived at.
    #[pyo3(get)]
    data_rate: Option<u8>,
    /// The frequency in hertz.
    #[pyo3(get)]
    frequency_hz: Option<u32>,
    /// How it was heard, for the kinds a station sends up.
    #[pyo3(get)]
    levels: Option<Py<GatewayStationLevels>>,
    /// Which class of downlink this is: 0 for A, 1 for B, 2 for C.
    #[pyo3(get)]
    class_: Option<u8>,
    /// The identifier a downlink and its transmission report share.
    #[pyo3(get)]
    diid: Option<i64>,
    /// The delay before the first receive window, in seconds.
    #[pyo3(get)]
    rx_delay: Option<u8>,
    /// The first receive window a downlink names.
    #[pyo3(get)]
    rx1: Option<Py<GatewayStationWindow>>,
    /// The second receive window a downlink names.
    #[pyo3(get)]
    rx2: Option<Py<GatewayStationWindow>>,
    /// The ping slot a class B downlink goes out in.
    #[pyo3(get)]
    ping_slot: Option<Py<GatewayStationWindow>>,
    /// How urgent a downlink is.
    #[pyo3(get)]
    priority: Option<u8>,
    /// The station clock in microseconds: the uplink a downlink answers, the moment a reported
    /// frame went out, or the one a time sync carries.
    #[pyo3(get)]
    xtime: Option<i64>,
    /// The radio, for a downlink or a report.
    #[pyo3(get)]
    rctx: Option<i64>,
    /// When a frame went out, in seconds, or the station time a time sync carries, in
    /// microseconds.
    #[pyo3(get)]
    txtime: Option<f64>,
    /// The GPS time in microseconds since the GPS epoch.
    #[pyo3(get)]
    gpstime: Option<i64>,
    /// What to transmit to a group, for a schedule.
    #[pyo3(get)]
    schedule: Option<Vec<Py<GatewayStationBroadcast>>>,
    /// The frame options, kept for the `fopts` property.
    options: Vec<u8>,
    /// The payload or frame, kept for the `payload` property.
    body: Vec<u8>,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayStationMessage {
    /// Describes a message of any kind, named by `msgtype` as the protocol writes it.
    ///
    /// A downlink's frame goes in `payload`, as does the whole of a proprietary frame.
    #[new]
    #[pyo3(signature = (
        msgtype, *, station=None, firmware=None, package=None, model=None, protocol=None,
        features=None, net_id=None, join_eui_ranges=None, region=None, max_eirp=None,
        hwspec=None, freq_min=None, freq_max=None, data_rates=None, mhdr=None, join_eui=None,
        dev_eui=None, dev_nonce=None, dev_addr=None, fctrl=None, fcnt=None, fopts=None,
        fport=None, payload=None, mic=None, data_rate=None, frequency_hz=None, levels=None,
        class_=None, diid=None, rx_delay=None, rx1=None, rx2=None, ping_slot=None,
        priority=None, xtime=None, rctx=None, txtime=None, gpstime=None, schedule=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        msgtype: String,
        station: Option<String>,
        firmware: Option<String>,
        package: Option<String>,
        model: Option<String>,
        protocol: Option<u32>,
        features: Option<String>,
        net_id: Option<Vec<u32>>,
        join_eui_ranges: Option<Vec<Py<GatewayStationJoinRange>>>,
        region: Option<String>,
        max_eirp: Option<f64>,
        hwspec: Option<String>,
        freq_min: Option<u32>,
        freq_max: Option<u32>,
        data_rates: Option<Vec<Option<Py<GatewayStationDataRate>>>>,
        mhdr: Option<u8>,
        join_eui: Option<String>,
        dev_eui: Option<String>,
        dev_nonce: Option<u16>,
        dev_addr: Option<i32>,
        fctrl: Option<u8>,
        fcnt: Option<u16>,
        fopts: Option<Vec<u8>>,
        fport: Option<u8>,
        payload: Option<Vec<u8>>,
        mic: Option<i32>,
        data_rate: Option<u8>,
        frequency_hz: Option<u32>,
        levels: Option<Py<GatewayStationLevels>>,
        class_: Option<u8>,
        diid: Option<i64>,
        rx_delay: Option<u8>,
        rx1: Option<Py<GatewayStationWindow>>,
        rx2: Option<Py<GatewayStationWindow>>,
        ping_slot: Option<Py<GatewayStationWindow>>,
        priority: Option<u8>,
        xtime: Option<i64>,
        rctx: Option<i64>,
        txtime: Option<f64>,
        gpstime: Option<i64>,
        schedule: Option<Vec<Py<GatewayStationBroadcast>>>,
    ) -> PyResult<Self> {
        if msgtype.is_empty() {
            return Err(PamojaError::new_err(
                "a message names its kind in msgtype, such as updf",
            ));
        }
        Ok(Self {
            msgtype,
            station,
            firmware,
            package,
            model,
            protocol,
            features,
            net_id,
            join_eui_ranges,
            region,
            max_eirp,
            hwspec,
            freq_min,
            freq_max,
            data_rates,
            mhdr,
            join_eui,
            dev_eui,
            dev_nonce,
            dev_addr,
            fctrl,
            fcnt,
            fport,
            mic,
            data_rate,
            frequency_hz,
            levels,
            class_,
            diid,
            rx_delay,
            rx1,
            rx2,
            ping_slot,
            priority,
            xtime,
            rctx,
            txtime,
            gpstime,
            schedule,
            options: fopts.unwrap_or_default(),
            body: payload.unwrap_or_default(),
        })
    }

    /// The frame options a data frame carries.
    #[getter]
    fn fopts<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.options)
    }

    /// The payload a frame carries, still encrypted, the whole of a proprietary frame, or the
    /// frame a downlink transmits.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.body)
    }

    fn __repr__(&self) -> String {
        format!("GatewayStationMessage(msgtype={})", self.msgtype)
    }
}

/// The answer a discovery endpoint gives.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayStationRouter {
    /// The station, as the server read it.
    #[pyo3(get)]
    router: Option<String>,
    /// The server endpoint carrying the session.
    #[pyo3(get)]
    muxs: Option<String>,
    /// The websocket to open, absolute.
    #[pyo3(get)]
    uri: Option<String>,
    /// Why the station was refused, when it was.
    #[pyo3(get)]
    error: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayStationRouter {
    fn __repr__(&self) -> String {
        match (&self.uri, &self.error) {
            (_, Some(error)) => format!("GatewayStationRouter(error={error})"),
            (Some(uri), _) => format!("GatewayStationRouter(uri={uri})"),
            _ => "GatewayStationRouter()".to_owned(),
        }
    }
}

/// Reads a frame the radio heard into the message that reports it.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (frame, data_rate, frequency_hz, levels=None))]
pub fn station_heard(
    py: Python<'_>,
    frame: Vec<u8>,
    data_rate: u8,
    frequency_hz: u32,
    levels: Option<PyRef<'_, GatewayStationLevels>>,
) -> PyResult<GatewayStationMessage> {
    let heard = levels.map_or_else(Levels::default, |levels| Levels {
        rctx: levels.rctx,
        xtime: levels.xtime,
        gpstime: levels.gpstime,
        rssi: levels.rssi,
        snr: levels.snr,
    });
    let message = Message::heard(&frame, data_rate, frequency_hz, heard)
        .map_err(|error| PamojaError::new_err(error.to_string()))?;
    message_to_py(py, &message)
}

/// Reads a message that arrived over the websocket.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn station_parse(py: Python<'_>, text: String) -> PyResult<GatewayStationMessage> {
    let message = Message::from_json(text.as_bytes())
        .map_err(|error| PamojaError::new_err(error.to_string()))?;
    message_to_py(py, &message)
}

/// Writes a message of any kind the way the websocket carries it.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn station_encode(
    py: Python<'_>,
    message: PyRef<'_, GatewayStationMessage>,
) -> PyResult<String> {
    Ok(message_of(py, &message)?.to_json())
}

/// Writes the request a station sends to find its network server.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn station_discovery(router: String) -> PyResult<String> {
    Ok(Discovery::new(identifier(&router)?).to_json())
}

/// Reads the request a station sent to find its network server, as the server does, and
/// returns the station asking as sixteen hexadecimal digits.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn station_discovery_parse(text: String) -> PyResult<String> {
    let asked = Discovery::from_json(text.as_bytes())
        .map_err(|error| PamojaError::new_err(error.to_string()))?;
    Ok(asked.router.to_hex())
}

/// Writes the answer that sends a station to the websocket its session runs on.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn station_router_accepted(router: String, muxs: String, uri: String) -> PyResult<String> {
    Ok(Router::accepted(identifier(&router)?, identifier(&muxs)?, uri).to_json())
}

/// Writes the answer that refuses a station, saying why.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn station_router_refused(router: String, error: String) -> PyResult<String> {
    Ok(Router::refused(identifier(&router)?, error).to_json())
}

/// Reads the answer a discovery endpoint gives.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn station_router_parse(text: String) -> PyResult<GatewayStationRouter> {
    let answer = Router::from_json(text.as_bytes())
        .map_err(|error| PamojaError::new_err(error.to_string()))?;
    Ok(GatewayStationRouter {
        router: answer.router.map(|eui| eui.to_hex()),
        muxs: answer.muxs.map(|eui| eui.to_hex()),
        uri: answer.uri,
        error: answer.error,
    })
}

/// Writes an identifier in the ID6 form the protocol prefers.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn station_id6(eui: String) -> PyResult<String> {
    Ok(id6(identifier(&eui)?))
}

/// Reads an identifier written in any form the protocol accepts.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn station_eui_of(text: String) -> PyResult<String> {
    eui_of(&text)
        .map(|eui| eui.to_hex())
        .ok_or_else(|| PamojaError::new_err(format!("{text} is not an identifier")))
}

/// Reads an identifier written as sixteen hexadecimal digits.
fn identifier(text: &str) -> PyResult<Eui> {
    Eui::from_hex(text)
        .ok_or_else(|| PamojaError::new_err(format!("{text} is not sixteen hexadecimal digits")))
}

/// Writes how a packet was heard, for Python to read.
fn levels_to_py(py: Python<'_>, levels: Levels) -> PyResult<Py<GatewayStationLevels>> {
    Py::new(
        py,
        GatewayStationLevels {
            rctx: levels.rctx,
            xtime: levels.xtime,
            gpstime: levels.gpstime,
            rssi: levels.rssi,
            snr: levels.snr,
        },
    )
}

/// Writes a window, for Python to read.
fn window_to_py(
    py: Python<'_>,
    window: Option<(u8, u32)>,
) -> PyResult<Option<Py<GatewayStationWindow>>> {
    window
        .map(|(data_rate, frequency_hz)| {
            Py::new(
                py,
                GatewayStationWindow {
                    data_rate,
                    frequency_hz,
                },
            )
        })
        .transpose()
}

/// Reads a window Python describes.
fn window_of(py: Python<'_>, window: Option<&Py<GatewayStationWindow>>) -> Option<(u8, u32)> {
    window.map(|window| {
        let window = window.bind(py).get();
        (window.data_rate, window.frequency_hz)
    })
}

/// An empty message of the given kind, which each arm then fills in.
fn blank(msgtype: &str) -> GatewayStationMessage {
    GatewayStationMessage {
        msgtype: msgtype.to_owned(),
        station: None,
        firmware: None,
        package: None,
        model: None,
        protocol: None,
        features: None,
        net_id: None,
        join_eui_ranges: None,
        region: None,
        max_eirp: None,
        hwspec: None,
        freq_min: None,
        freq_max: None,
        data_rates: None,
        mhdr: None,
        join_eui: None,
        dev_eui: None,
        dev_nonce: None,
        dev_addr: None,
        fctrl: None,
        fcnt: None,
        fport: None,
        mic: None,
        data_rate: None,
        frequency_hz: None,
        levels: None,
        class_: None,
        diid: None,
        rx_delay: None,
        rx1: None,
        rx2: None,
        ping_slot: None,
        priority: None,
        xtime: None,
        rctx: None,
        txtime: None,
        gpstime: None,
        schedule: None,
        options: Vec::new(),
        body: Vec::new(),
    }
}

/// Writes a message Python reads.
fn message_to_py(py: Python<'_>, message: &Message) -> PyResult<GatewayStationMessage> {
    let mut held = blank(message.msgtype());
    match message {
        Message::Version {
            station,
            firmware,
            package,
            model,
            protocol,
            features,
        } => {
            held.station = Some(station.clone());
            held.firmware = Some(firmware.clone());
            held.package = Some(package.clone());
            held.model = Some(model.clone());
            held.protocol = Some(*protocol);
            held.features = Some(features.clone());
        }
        Message::RouterConfig {
            net_id,
            join_eui,
            region,
            max_eirp,
            hwspec,
            freq_range,
            data_rates,
        } => {
            held.net_id = net_id.clone();
            held.join_eui_ranges = Some(
                join_eui
                    .iter()
                    .map(|(first, last)| {
                        Py::new(
                            py,
                            GatewayStationJoinRange {
                                first: Eui::new(first.to_be_bytes()).to_hex(),
                                last: Eui::new(last.to_be_bytes()).to_hex(),
                            },
                        )
                    })
                    .collect::<PyResult<_>>()?,
            );
            held.region = Some(region.clone());
            held.max_eirp = Some(*max_eirp);
            held.hwspec = Some(hwspec.clone());
            held.freq_min = Some(freq_range.0);
            held.freq_max = Some(freq_range.1);
            held.data_rates = Some(
                data_rates
                    .iter()
                    .map(|entry| {
                        entry
                            .map(|(spreading_factor, bandwidth_hz, downlink_only)| {
                                Py::new(
                                    py,
                                    GatewayStationDataRate {
                                        spreading_factor,
                                        bandwidth_hz,
                                        downlink_only,
                                    },
                                )
                            })
                            .transpose()
                    })
                    .collect::<PyResult<_>>()?,
            );
        }
        Message::JoinRequest {
            mhdr,
            join_eui,
            dev_eui,
            dev_nonce,
            mic,
            data_rate,
            frequency_hz,
            levels,
        } => {
            held.mhdr = Some(*mhdr);
            held.join_eui = Some(join_eui.to_hex());
            held.dev_eui = Some(dev_eui.to_hex());
            held.dev_nonce = Some(*dev_nonce);
            held.mic = Some(*mic);
            held.data_rate = Some(*data_rate);
            held.frequency_hz = Some(*frequency_hz);
            held.levels = Some(levels_to_py(py, *levels)?);
        }
        Message::Uplink {
            mhdr,
            dev_addr,
            fctrl,
            fcnt,
            fopts,
            fport,
            payload,
            mic,
            data_rate,
            frequency_hz,
            levels,
        } => {
            held.mhdr = Some(*mhdr);
            held.dev_addr = Some(*dev_addr);
            held.fctrl = Some(*fctrl);
            held.fcnt = Some(*fcnt);
            held.options = fopts.clone();
            held.fport = *fport;
            held.body = payload.clone();
            held.mic = Some(*mic);
            held.data_rate = Some(*data_rate);
            held.frequency_hz = Some(*frequency_hz);
            held.levels = Some(levels_to_py(py, *levels)?);
        }
        Message::Proprietary {
            payload,
            data_rate,
            frequency_hz,
            levels,
        } => {
            held.body = payload.clone();
            held.data_rate = Some(*data_rate);
            held.frequency_hz = Some(*frequency_hz);
            held.levels = Some(levels_to_py(py, *levels)?);
        }
        Message::Downlink {
            dev_eui,
            class,
            diid,
            pdu,
            rx_delay,
            rx1,
            rx2,
            ping_slot,
            priority,
            xtime,
            rctx,
            gpstime,
        } => {
            held.dev_eui = Some(dev_eui.to_hex());
            held.class_ = Some(*class);
            held.diid = Some(*diid);
            held.body = pdu.clone();
            held.rx_delay = *rx_delay;
            held.rx1 = window_to_py(py, *rx1)?;
            held.rx2 = window_to_py(py, *rx2)?;
            held.ping_slot = window_to_py(py, *ping_slot)?;
            held.priority = Some(*priority);
            held.xtime = *xtime;
            held.rctx = *rctx;
            held.gpstime = *gpstime;
        }
        Message::Schedule { frames } => {
            let mut schedule = Vec::with_capacity(frames.len());
            for frame in frames {
                schedule.push(Py::new(
                    py,
                    GatewayStationBroadcast {
                        frame: frame.pdu.clone(),
                        data_rate: frame.data_rate,
                        frequency_hz: frame.frequency_hz,
                        priority: frame.priority,
                        gpstime: frame.gpstime,
                        rctx: frame.rctx,
                    },
                )?);
            }
            held.schedule = Some(schedule);
        }
        Message::Transmitted {
            diid,
            dev_eui,
            rctx,
            xtime,
            txtime,
            gpstime,
        } => {
            held.diid = Some(*diid);
            held.dev_eui = Some(dev_eui.to_hex());
            held.rctx = Some(*rctx);
            held.xtime = Some(*xtime);
            held.txtime = Some(*txtime);
            held.gpstime = *gpstime;
        }
        Message::TimeSync {
            txtime,
            xtime,
            gpstime,
        } => {
            held.txtime = txtime.map(|value| value as f64);
            held.xtime = *xtime;
            held.gpstime = *gpstime;
        }
        Message::Other { .. } => {}
    }
    Ok(held)
}

/// Reads a message Python describes.
fn message_of(py: Python<'_>, message: &GatewayStationMessage) -> PyResult<Message> {
    // A message a station heard keeps its levels in the object beside it, while the downlink
    // kinds carry the clock in the flat fields, so read whichever the message has.
    let levels = match &message.levels {
        Some(held) => {
            let held = held.bind(py).get();
            Levels {
                rctx: held.rctx,
                xtime: held.xtime,
                gpstime: held.gpstime,
                rssi: held.rssi,
                snr: held.snr,
            }
        }
        None => Levels {
            rctx: message.rctx.unwrap_or_default(),
            xtime: message.xtime.unwrap_or_default(),
            gpstime: message.gpstime,
            rssi: 0.0,
            snr: 0.0,
        },
    };
    Ok(match message.msgtype.as_str() {
        "version" => Message::Version {
            station: message.station.clone().unwrap_or_default(),
            firmware: message.firmware.clone().unwrap_or_default(),
            package: message.package.clone().unwrap_or_default(),
            model: message.model.clone().unwrap_or_default(),
            protocol: message
                .protocol
                .unwrap_or(pamoja_gateway::station::PROTOCOL_VERSION),
            features: message.features.clone().unwrap_or_default(),
        },
        "router_config" => Message::RouterConfig {
            net_id: message.net_id.clone(),
            join_eui: message
                .join_eui_ranges
                .iter()
                .flatten()
                .map(|range| {
                    let range = range.bind(py).get();
                    let bound =
                        |text: &str| identifier(text).map(|eui| u64::from_be_bytes(eui.bytes()));
                    Ok((bound(&range.first)?, bound(&range.last)?))
                })
                .collect::<PyResult<_>>()?,
            region: message.region.clone().unwrap_or_default(),
            max_eirp: message.max_eirp.unwrap_or_default(),
            hwspec: message.hwspec.clone().unwrap_or_default(),
            freq_range: (
                message.freq_min.unwrap_or_default(),
                message.freq_max.unwrap_or_default(),
            ),
            data_rates: message
                .data_rates
                .iter()
                .flatten()
                .map(|entry| {
                    entry.as_ref().map(|rate| {
                        let rate = rate.bind(py).get();
                        (rate.spreading_factor, rate.bandwidth_hz, rate.downlink_only)
                    })
                })
                .collect(),
        },
        "jreq" => Message::JoinRequest {
            mhdr: message.mhdr.unwrap_or_default(),
            join_eui: identifier(message.join_eui.as_deref().unwrap_or_default())?,
            dev_eui: identifier(message.dev_eui.as_deref().unwrap_or_default())?,
            dev_nonce: message.dev_nonce.unwrap_or_default(),
            mic: message.mic.unwrap_or_default(),
            data_rate: message.data_rate.unwrap_or_default(),
            frequency_hz: message.frequency_hz.unwrap_or_default(),
            levels,
        },
        "updf" => Message::Uplink {
            mhdr: message.mhdr.unwrap_or_default(),
            dev_addr: message.dev_addr.unwrap_or_default(),
            fctrl: message.fctrl.unwrap_or_default(),
            fcnt: message.fcnt.unwrap_or_default(),
            fopts: message.options.clone(),
            fport: message.fport,
            payload: message.body.clone(),
            mic: message.mic.unwrap_or_default(),
            data_rate: message.data_rate.unwrap_or_default(),
            frequency_hz: message.frequency_hz.unwrap_or_default(),
            levels,
        },
        "propdf" => Message::Proprietary {
            payload: message.body.clone(),
            data_rate: message.data_rate.unwrap_or_default(),
            frequency_hz: message.frequency_hz.unwrap_or_default(),
            levels,
        },
        "dnmsg" => Message::Downlink {
            dev_eui: identifier(message.dev_eui.as_deref().unwrap_or_default())?,
            class: message.class_.unwrap_or_default(),
            diid: message.diid.unwrap_or_default(),
            pdu: message.body.clone(),
            rx_delay: message.rx_delay,
            rx1: window_of(py, message.rx1.as_ref()),
            rx2: window_of(py, message.rx2.as_ref()),
            ping_slot: window_of(py, message.ping_slot.as_ref()),
            priority: message.priority.unwrap_or_default(),
            xtime: message.xtime,
            rctx: message.rctx,
            gpstime: message.gpstime,
        },
        "dnsched" => Message::Schedule {
            frames: message
                .schedule
                .iter()
                .flatten()
                .map(|frame| broadcast_of(frame.bind(py).get()))
                .collect(),
        },
        "dntxed" => Message::Transmitted {
            diid: message.diid.unwrap_or_default(),
            dev_eui: identifier(message.dev_eui.as_deref().unwrap_or_default())?,
            rctx: message.rctx.unwrap_or_default(),
            xtime: message.xtime.unwrap_or_default(),
            txtime: message.txtime.unwrap_or_default(),
            gpstime: message.gpstime,
        },
        "timesync" => Message::TimeSync {
            txtime: message.txtime.map(|value| value.round() as i64),
            xtime: message.xtime,
            gpstime: message.gpstime,
        },
        other => Message::Other {
            msgtype: other.to_owned(),
        },
    })
}

/// Reads one frame of a schedule Python describes.
fn broadcast_of(frame: &GatewayStationBroadcast) -> Broadcast {
    Broadcast {
        pdu: frame.frame.clone(),
        data_rate: frame.data_rate,
        frequency_hz: frame.frequency_hz,
        priority: frame.priority,
        gpstime: frame.gpstime,
        rctx: frame.rctx,
    }
}
