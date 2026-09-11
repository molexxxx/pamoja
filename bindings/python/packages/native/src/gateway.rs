//! Generated Python bindings for the LoRaWAN gateway protocols.
//!
//! These mirror the `pamoja-gateway` Rust API: the six UDP datagrams a gateway and a network
//! server exchange, and the objects they carry. A datagram crosses as a plain object with the
//! fields its kind uses, so a program builds one, encodes it, and sends it over a socket of
//! its own, and reads whatever arrives the same way.
//!
//! Frequencies are in hertz, payloads are bytes rather than base64, and a reception time is a
//! count of microseconds rather than the string the protocol writes, so nothing has to be
//! formatted by hand.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_gateway::udp::{
    CrcStatus, Eui, Modulation, Packet, PacketKind, Rxpk, Stat, TxStatus, Txpk, Uplink,
};
use pamoja_lora::LinkSettings;

use crate::lora::{db, decibels, LoraLink};
use crate::PamojaError;

/// A packet the gateway heard, with the metadata the protocol carries beside it.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayRxpk {
    /// The carrier it arrived on, in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// The packet itself, kept for the `payload` property.
    frame: Vec<u8>,
    /// The spreading factor, bandwidth, and coding rate, for a LoRa packet.
    #[pyo3(get)]
    link: Option<Py<LoraLink>>,
    /// The bitrate in bits per second, for an FSK packet.
    #[pyo3(get)]
    bitrate_bps: Option<u32>,
    /// What the CRC said: `"Ok"`, `"Failed"`, or `"Absent"`.
    #[pyo3(get)]
    crc: String,
    /// The received signal strength in dBm.
    #[pyo3(get)]
    rssi_dbm: f64,
    /// The signal-to-noise ratio in dB, for a LoRa packet.
    #[pyo3(get)]
    snr_db: Option<f64>,
    /// The concentrator channel it arrived on.
    #[pyo3(get)]
    channel: u8,
    /// The radio chain it arrived on.
    #[pyo3(get)]
    rf_chain: u8,
    /// The concentrator's own timestamp of the reception, in microseconds.
    #[pyo3(get)]
    timestamp_us: Option<u32>,
    /// When it arrived, in microseconds since 1970-01-01 UTC.
    #[pyo3(get)]
    received_at_us: Option<u64>,
    /// When it arrived on the GPS clock, in milliseconds since 6 January 1980.
    #[pyo3(get)]
    gps_millis: Option<u64>,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayRxpk {
    /// Describes a packet the gateway heard.
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        frequency_hz,
        payload,
        *,
        link=None,
        bitrate_bps=None,
        crc="Ok".to_owned(),
        rssi_dbm=0.0,
        snr_db=None,
        channel=0,
        rf_chain=0,
        timestamp_us=None,
        received_at_us=None,
        gps_millis=None
    ))]
    fn new(
        frequency_hz: u32,
        payload: Vec<u8>,
        link: Option<Py<LoraLink>>,
        bitrate_bps: Option<u32>,
        crc: String,
        rssi_dbm: f64,
        snr_db: Option<f64>,
        channel: u8,
        rf_chain: u8,
        timestamp_us: Option<u32>,
        received_at_us: Option<u64>,
        gps_millis: Option<u64>,
    ) -> GatewayRxpk {
        GatewayRxpk {
            frequency_hz,
            frame: payload,
            link,
            bitrate_bps,
            crc,
            rssi_dbm,
            snr_db,
            channel,
            rf_chain,
            timestamp_us,
            received_at_us,
            gps_millis,
        }
    }

    /// The packet itself.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.frame)
    }
}

/// A gateway's own status report.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayStat {
    /// The gateway's clock, in seconds since 1970-01-01 UTC.
    #[pyo3(get)]
    time_s: Option<u64>,
    /// Its latitude in degrees, north positive.
    #[pyo3(get)]
    latitude_deg: Option<f64>,
    /// Its longitude in degrees, east positive.
    #[pyo3(get)]
    longitude_deg: Option<f64>,
    /// Its altitude in meters.
    #[pyo3(get)]
    altitude_m: Option<i32>,
    /// How many packets its radio received.
    #[pyo3(get)]
    received: u32,
    /// How many of those had a good CRC.
    #[pyo3(get)]
    received_ok: u32,
    /// How many it forwarded.
    #[pyo3(get)]
    forwarded: u32,
    /// What share of its datagrams were acknowledged, as a percentage.
    #[pyo3(get)]
    acknowledged_percent: f64,
    /// How many downlink datagrams it received.
    #[pyo3(get)]
    downlinks: u32,
    /// How many packets it transmitted.
    #[pyo3(get)]
    transmitted: u32,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayStat {
    /// Describes a gateway's status report.
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        *,
        time_s=None,
        latitude_deg=None,
        longitude_deg=None,
        altitude_m=None,
        received=0,
        received_ok=0,
        forwarded=0,
        acknowledged_percent=0.0,
        downlinks=0,
        transmitted=0
    ))]
    fn new(
        time_s: Option<u64>,
        latitude_deg: Option<f64>,
        longitude_deg: Option<f64>,
        altitude_m: Option<i32>,
        received: u32,
        received_ok: u32,
        forwarded: u32,
        acknowledged_percent: f64,
        downlinks: u32,
        transmitted: u32,
    ) -> GatewayStat {
        GatewayStat {
            time_s,
            latitude_deg,
            longitude_deg,
            altitude_m,
            received,
            received_ok,
            forwarded,
            acknowledged_percent,
            downlinks,
            transmitted,
        }
    }
}

/// A packet the server asks the gateway to transmit.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayTxpk {
    /// The carrier to transmit on, in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// The packet itself, kept for the `payload` property.
    frame: Vec<u8>,
    /// The spreading factor, bandwidth, and coding rate, for a LoRa packet.
    #[pyo3(get)]
    link: Option<Py<LoraLink>>,
    /// The bitrate in bits per second, for an FSK packet.
    #[pyo3(get)]
    bitrate_bps: Option<u32>,
    /// Whether to transmit at once, which ignores the timestamps.
    #[pyo3(get)]
    immediate: bool,
    /// The concentrator timestamp to transmit at, in microseconds.
    #[pyo3(get)]
    timestamp_us: Option<u32>,
    /// The GPS time to transmit at, in milliseconds since 6 January 1980.
    #[pyo3(get)]
    gps_millis: Option<u64>,
    /// The radio chain to transmit from.
    #[pyo3(get)]
    rf_chain: u8,
    /// The power to transmit at, in dBm.
    #[pyo3(get)]
    power_dbm: i8,
    /// The FSK frequency deviation in hertz.
    #[pyo3(get)]
    frequency_deviation_hz: Option<u32>,
    /// Whether to invert the LoRa polarity, as a LoRaWAN downlink is sent.
    #[pyo3(get)]
    invert_polarity: bool,
    /// How long a preamble to send, in symbols.
    #[pyo3(get)]
    preamble_symbols: Option<u16>,
    /// Whether to leave the physical CRC off, as LoRaWAN downlinks are.
    #[pyo3(get)]
    without_crc: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayTxpk {
    /// Describes a packet to transmit.
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        frequency_hz,
        payload,
        *,
        link=None,
        bitrate_bps=None,
        immediate=None,
        timestamp_us=None,
        gps_millis=None,
        rf_chain=0,
        power_dbm=14,
        frequency_deviation_hz=None,
        invert_polarity=false,
        preamble_symbols=None,
        without_crc=false
    ))]
    fn new(
        frequency_hz: u32,
        payload: Vec<u8>,
        link: Option<Py<LoraLink>>,
        bitrate_bps: Option<u32>,
        immediate: Option<bool>,
        timestamp_us: Option<u32>,
        gps_millis: Option<u64>,
        rf_chain: u8,
        power_dbm: i8,
        frequency_deviation_hz: Option<u32>,
        invert_polarity: bool,
        preamble_symbols: Option<u16>,
        without_crc: bool,
    ) -> GatewayTxpk {
        GatewayTxpk {
            frequency_hz,
            frame: payload,
            link,
            bitrate_bps,
            immediate: immediate.unwrap_or(timestamp_us.is_none()),
            timestamp_us,
            gps_millis,
            rf_chain,
            power_dbm,
            frequency_deviation_hz,
            invert_polarity,
            preamble_symbols,
            without_crc,
        }
    }

    /// The packet itself.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.frame)
    }
}

/// One datagram of the protocol, with the fields its kind carries.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayPacket {
    /// Which kind of datagram: `"PushData"`, `"PushAck"`, `"PullData"`, `"PullResp"`,
    /// `"PullAck"`, or `"TxAck"`.
    #[pyo3(get)]
    kind: String,
    /// The token that pairs a datagram with its answer.
    #[pyo3(get)]
    token: u16,
    /// The gateway's identifier, as sixteen hexadecimal digits, for the kinds that carry one.
    #[pyo3(get)]
    gateway: Option<String>,
    /// The packets a PUSH_DATA forwards.
    #[pyo3(get)]
    packets: Vec<Py<GatewayRxpk>>,
    /// The report a PUSH_DATA carries.
    #[pyo3(get)]
    status: Option<Py<GatewayStat>>,
    /// What a PULL_RESP asks the gateway to transmit.
    #[pyo3(get)]
    transmit: Option<Py<GatewayTxpk>>,
    /// What a TX_ACK reports, such as `"NONE"` or `"COLLISION_PACKET"`.
    #[pyo3(get)]
    tx_status: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayPacket {
    /// Describes a datagram of the protocol.
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        kind,
        token,
        *,
        gateway=None,
        packets=Vec::new(),
        status=None,
        transmit=None,
        tx_status=None
    ))]
    fn new(
        kind: String,
        token: u16,
        gateway: Option<String>,
        packets: Vec<Py<GatewayRxpk>>,
        status: Option<Py<GatewayStat>>,
        transmit: Option<Py<GatewayTxpk>>,
        tx_status: Option<String>,
    ) -> GatewayPacket {
        GatewayPacket {
            kind,
            token,
            gateway,
            packets,
            status,
            transmit,
            tx_status,
        }
    }
}

/// Writes a datagram to send over a socket.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn gateway_encode<'py>(
    py: Python<'py>,
    packet: PyRef<'_, GatewayPacket>,
) -> PyResult<Bound<'py, PyBytes>> {
    Ok(PyBytes::new(py, &packet_of(py, &packet)?.to_bytes()))
}

/// Reads a datagram that arrived.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn gateway_parse(py: Python<'_>, datagram: Vec<u8>) -> PyResult<GatewayPacket> {
    let packet =
        Packet::parse(&datagram).map_err(|error| PamojaError::new_err(error.to_string()))?;
    packet_to_py(py, &packet)
}

/// Returns the acknowledgment a server owes a datagram, or `None` for one that needs none.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn gateway_acknowledgment(
    py: Python<'_>,
    packet: PyRef<'_, GatewayPacket>,
) -> PyResult<Option<GatewayPacket>> {
    match packet_of(py, &packet)?.acknowledgment() {
        Some(acknowledgment) => packet_to_py(py, &acknowledgment).map(Some),
        None => Ok(None),
    }
}

/// Reads a datagram Python describes.
fn packet_of(py: Python<'_>, packet: &GatewayPacket) -> PyResult<Packet> {
    let token = packet.token;
    Ok(match packet.kind.as_str() {
        "PushData" => Packet::PushData {
            token,
            gateway: gateway_of(packet.gateway.as_deref())?,
            uplink: Uplink {
                packets: packet
                    .packets
                    .iter()
                    .map(|heard| rxpk_of(py, heard.bind(py).borrow()))
                    .collect::<PyResult<Vec<_>>>()?,
                status: packet
                    .status
                    .as_ref()
                    .map(|report| stat_of(&report.bind(py).borrow())),
            },
        },
        "PushAck" => Packet::PushAck { token },
        "PullData" => Packet::PullData {
            token,
            gateway: gateway_of(packet.gateway.as_deref())?,
        },
        "PullAck" => Packet::PullAck { token },
        "PullResp" => Packet::PullResp {
            token,
            transmit: match &packet.transmit {
                Some(request) => txpk_of(py, request.bind(py).borrow())?,
                None => {
                    return Err(PyValueError::new_err(
                        "a PullResp carries what to transmit in `transmit`",
                    ))
                }
            },
        },
        "TxAck" => Packet::TxAck {
            token,
            gateway: gateway_of(packet.gateway.as_deref())?,
            status: match packet.tx_status.as_deref() {
                None => TxStatus::None,
                Some(status) => TxStatus::named(status).ok_or_else(|| {
                    PyValueError::new_err(format!("{status} is not a TX_ACK status"))
                })?,
            },
        },
        other => {
            return Err(PyValueError::new_err(format!(
                "no datagram is of kind {other}"
            )))
        }
    })
}

/// Describes a datagram for Python.
fn packet_to_py(py: Python<'_>, packet: &Packet) -> PyResult<GatewayPacket> {
    let (packets, status) = match packet {
        Packet::PushData { uplink, .. } => (
            uplink
                .packets
                .iter()
                .map(|heard| Py::new(py, rxpk_to_py(py, heard)?))
                .collect::<PyResult<Vec<_>>>()?,
            match &uplink.status {
                Some(report) => Some(Py::new(py, stat_to_py(report))?),
                None => None,
            },
        ),
        _ => (Vec::new(), None),
    };
    Ok(GatewayPacket {
        kind: kind_name(packet.kind()).to_owned(),
        token: packet.token(),
        gateway: packet.gateway().map(|gateway| gateway.to_hex()),
        packets,
        status,
        transmit: match packet {
            Packet::PullResp { transmit, .. } => Some(Py::new(py, txpk_to_py(py, transmit)?)?),
            _ => None,
        },
        tx_status: match packet {
            Packet::TxAck { status, .. } => Some(status.as_str().to_owned()),
            _ => None,
        },
    })
}

/// Names a kind for Python.
fn kind_name(kind: PacketKind) -> &'static str {
    match kind {
        PacketKind::PushData => "PushData",
        PacketKind::PushAck => "PushAck",
        PacketKind::PullData => "PullData",
        PacketKind::PullResp => "PullResp",
        PacketKind::PullAck => "PullAck",
        PacketKind::TxAck => "TxAck",
    }
}

/// Reads a gateway identifier Python passes.
fn gateway_of(text: Option<&str>) -> PyResult<Eui> {
    let text = text.ok_or_else(|| {
        PyValueError::new_err("this kind of datagram carries the gateway's identifier")
    })?;
    Eui::from_hex(text).ok_or_else(|| {
        PyValueError::new_err(format!(
            "a gateway identifier is sixteen hexadecimal digits, not {text}"
        ))
    })
}

/// Reads how a packet was modulated.
fn modulation_of(
    py: Python<'_>,
    link: Option<&Py<LoraLink>>,
    bitrate_bps: Option<u32>,
) -> Modulation {
    match (link, bitrate_bps) {
        (_, Some(bitrate)) => Modulation::Fsk(bitrate),
        (Some(link), None) => Modulation::Lora(link.bind(py).borrow().settings()),
        (None, None) => Modulation::Lora(LinkSettings::new(7, 125_000)),
    }
}

/// Describes how a packet was modulated.
fn modulation_to_py(
    py: Python<'_>,
    modulation: Modulation,
) -> PyResult<(Option<Py<LoraLink>>, Option<u32>)> {
    Ok(match modulation {
        Modulation::Lora(link) => (Some(Py::new(py, LoraLink::from_settings(link))?), None),
        Modulation::Fsk(bitrate) => (None, Some(bitrate)),
    })
}

/// Reads a forwarded packet Python describes.
fn rxpk_of(py: Python<'_>, heard: PyRef<'_, GatewayRxpk>) -> PyResult<Rxpk> {
    Ok(Rxpk {
        received_at: heard.received_at_us.map(pamoja_gateway::time::compact),
        gps_millis: heard.gps_millis,
        timestamp_us: heard.timestamp_us,
        frequency_hz: heard.frequency_hz,
        channel: heard.channel,
        rf_chain: heard.rf_chain,
        crc: match heard.crc.as_str() {
            "Ok" => CrcStatus::Ok,
            "Failed" => CrcStatus::Failed,
            "Absent" => CrcStatus::Absent,
            other => {
                return Err(PyValueError::new_err(format!(
                    "no CRC status is named {other}; use Ok, Failed, or Absent"
                )))
            }
        },
        modulation: modulation_of(py, heard.link.as_ref(), heard.bitrate_bps),
        rssi_dbm: decibels(heard.rssi_dbm),
        snr_db: heard.snr_db.map(decibels),
        payload: heard.frame.clone(),
    })
}

/// Describes a forwarded packet for Python.
fn rxpk_to_py(py: Python<'_>, heard: &Rxpk) -> PyResult<GatewayRxpk> {
    let (link, bitrate_bps) = modulation_to_py(py, heard.modulation)?;
    Ok(GatewayRxpk {
        frequency_hz: heard.frequency_hz,
        frame: heard.payload.clone(),
        link,
        bitrate_bps,
        crc: match heard.crc {
            CrcStatus::Ok => "Ok",
            CrcStatus::Failed => "Failed",
            CrcStatus::Absent => "Absent",
        }
        .to_owned(),
        rssi_dbm: db(heard.rssi_dbm),
        snr_db: heard.snr_db.map(db),
        channel: heard.channel,
        rf_chain: heard.rf_chain,
        timestamp_us: heard.timestamp_us,
        received_at_us: heard
            .received_at
            .as_deref()
            .and_then(pamoja_gateway::time::from_compact),
        gps_millis: heard.gps_millis,
    })
}

/// Reads a status report Python describes.
fn stat_of(report: &GatewayStat) -> Stat {
    Stat {
        time: report.time_s.map(pamoja_gateway::time::expanded),
        latitude_deg: report.latitude_deg,
        longitude_deg: report.longitude_deg,
        altitude_m: report.altitude_m,
        received: report.received,
        received_ok: report.received_ok,
        forwarded: report.forwarded,
        acknowledged_percent: report.acknowledged_percent,
        downlinks: report.downlinks,
        transmitted: report.transmitted,
    }
}

/// Describes a status report for Python.
fn stat_to_py(report: &Stat) -> GatewayStat {
    GatewayStat {
        time_s: report
            .time
            .as_deref()
            .and_then(pamoja_gateway::time::from_expanded),
        latitude_deg: report.latitude_deg,
        longitude_deg: report.longitude_deg,
        altitude_m: report.altitude_m,
        received: report.received,
        received_ok: report.received_ok,
        forwarded: report.forwarded,
        acknowledged_percent: report.acknowledged_percent,
        downlinks: report.downlinks,
        transmitted: report.transmitted,
    }
}

/// Reads a transmission request Python describes.
fn txpk_of(py: Python<'_>, request: PyRef<'_, GatewayTxpk>) -> PyResult<Txpk> {
    Ok(Txpk {
        immediate: request.immediate,
        timestamp_us: request.timestamp_us,
        gps_millis: request.gps_millis,
        frequency_hz: request.frequency_hz,
        rf_chain: request.rf_chain,
        power_dbm: request.power_dbm,
        modulation: modulation_of(py, request.link.as_ref(), request.bitrate_bps),
        frequency_deviation_hz: request.frequency_deviation_hz,
        invert_polarity: request.invert_polarity,
        preamble_symbols: request.preamble_symbols,
        without_crc: request.without_crc,
        payload: request.frame.clone(),
    })
}

/// Describes a transmission request for Python.
fn txpk_to_py(py: Python<'_>, request: &Txpk) -> PyResult<GatewayTxpk> {
    let (link, bitrate_bps) = modulation_to_py(py, request.modulation)?;
    Ok(GatewayTxpk {
        frequency_hz: request.frequency_hz,
        frame: request.payload.clone(),
        link,
        bitrate_bps,
        immediate: request.immediate,
        timestamp_us: request.timestamp_us,
        gps_millis: request.gps_millis,
        rf_chain: request.rf_chain,
        power_dbm: request.power_dbm,
        frequency_deviation_hz: request.frequency_deviation_hz,
        invert_polarity: request.invert_polarity,
        preamble_symbols: request.preamble_symbols,
        without_crc: request.without_crc,
    })
}
