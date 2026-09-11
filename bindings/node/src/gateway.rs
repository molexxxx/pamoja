//! Generated Node bindings for the LoRaWAN gateway protocols.
//!
//! These mirror the `pamoja-gateway` Rust API: the six UDP datagrams a gateway and a network
//! server exchange, and the objects they carry. A datagram crosses as a plain object with the
//! fields its kind uses, so a program builds one, encodes it, and sends it over a socket of
//! its own, and parses whatever arrives the same way.
//!
//! Frequencies are in hertz, payloads are buffers rather than base64, and a reception time is
//! a count of microseconds rather than the string the protocol writes, so nothing has to be
//! formatted by hand.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_gateway::udp::{
    CrcStatus, Eui, Modulation, Packet, PacketKind, Rxpk, Stat, TxStatus, Txpk, Uplink,
};
use pamoja_lora::budget::Decibels;

use crate::lora::{db, decibels, settings, LoraLink};

/// Which kind of datagram this is.
#[napi(string_enum, js_name = "GatewayPacketKind")]
pub enum GatewayPacketKind {
    /// The gateway forwarding what it heard.
    PushData,
    /// The server acknowledging a PUSH_DATA.
    PushAck,
    /// The gateway holding its route open.
    PullData,
    /// The server sending a packet to transmit.
    PullResp,
    /// The server acknowledging a PULL_DATA.
    PullAck,
    /// The gateway reporting what became of a PULL_RESP.
    TxAck,
}

/// What the CRC of a received packet said.
#[napi(string_enum, js_name = "GatewayCrc")]
pub enum GatewayCrc {
    /// The CRC checked.
    Ok,
    /// The CRC failed.
    Failed,
    /// The packet carried no CRC.
    Absent,
}

/// A packet the gateway heard, with the metadata the protocol carries beside it.
#[napi(object, js_name = "GatewayRxpk")]
pub struct GatewayRxpk {
    /// The carrier it arrived on, in hertz.
    pub frequency_hz: u32,
    /// The packet itself.
    pub payload: Buffer,
    /// The spreading factor, bandwidth, and coding rate, for a LoRa packet.
    pub link: Option<LoraLink>,
    /// The bitrate in bits per second, for an FSK packet.
    pub bitrate_bps: Option<u32>,
    /// What the CRC said; `Ok` when omitted.
    pub crc: Option<GatewayCrc>,
    /// The received signal strength in dBm.
    pub rssi_dbm: Option<f64>,
    /// The signal-to-noise ratio in dB.
    pub snr_db: Option<f64>,
    /// The concentrator channel it arrived on.
    pub channel: Option<u8>,
    /// The radio chain it arrived on.
    pub rf_chain: Option<u8>,
    /// The concentrator's own timestamp of the reception, in microseconds.
    pub timestamp_us: Option<u32>,
    /// When it arrived, in microseconds since 1970-01-01 UTC.
    pub received_at_us: Option<f64>,
    /// When it arrived on the GPS clock, in milliseconds since 6 January 1980.
    pub gps_millis: Option<f64>,
}

/// A gateway's own status report.
#[napi(object, js_name = "GatewayStat")]
pub struct GatewayStat {
    /// The gateway's clock, in seconds since 1970-01-01 UTC.
    pub time_s: Option<f64>,
    /// Its latitude in degrees, north positive.
    pub latitude_deg: Option<f64>,
    /// Its longitude in degrees, east positive.
    pub longitude_deg: Option<f64>,
    /// Its altitude in meters.
    pub altitude_m: Option<i32>,
    /// How many packets its radio received.
    pub received: u32,
    /// How many of those had a good CRC.
    pub received_ok: u32,
    /// How many it forwarded.
    pub forwarded: u32,
    /// What share of its datagrams were acknowledged, as a percentage.
    pub acknowledged_percent: Option<f64>,
    /// How many downlink datagrams it received.
    pub downlinks: u32,
    /// How many packets it transmitted.
    pub transmitted: u32,
}

/// A packet the server asks the gateway to transmit.
#[napi(object, js_name = "GatewayTxpk")]
pub struct GatewayTxpk {
    /// The carrier to transmit on, in hertz.
    pub frequency_hz: u32,
    /// The packet itself.
    pub payload: Buffer,
    /// The spreading factor, bandwidth, and coding rate, for a LoRa packet.
    pub link: Option<LoraLink>,
    /// The bitrate in bits per second, for an FSK packet.
    pub bitrate_bps: Option<u32>,
    /// Whether to transmit at once, which ignores the timestamps.
    pub immediate: Option<bool>,
    /// The concentrator timestamp to transmit at, in microseconds.
    pub timestamp_us: Option<u32>,
    /// The GPS time to transmit at, in milliseconds since 6 January 1980.
    pub gps_millis: Option<f64>,
    /// The radio chain to transmit from.
    pub rf_chain: Option<u8>,
    /// The power to transmit at, in dBm; 14 when omitted.
    pub power_dbm: Option<i32>,
    /// The FSK frequency deviation in hertz.
    pub frequency_deviation_hz: Option<u32>,
    /// Whether to invert the LoRa polarity, as a LoRaWAN downlink is sent.
    pub invert_polarity: Option<bool>,
    /// How long a preamble to send, in symbols.
    pub preamble_symbols: Option<u16>,
    /// Whether to leave the physical CRC off, as LoRaWAN downlinks are.
    pub without_crc: Option<bool>,
}

/// One datagram of the protocol, with the fields its kind carries.
#[napi(object, js_name = "GatewayPacket")]
pub struct GatewayPacket {
    /// Which kind of datagram.
    pub kind: GatewayPacketKind,
    /// The token that pairs a datagram with its answer.
    pub token: u16,
    /// The gateway's identifier, as sixteen hexadecimal digits, for the kinds that carry one.
    pub gateway: Option<String>,
    /// The packets a PUSH_DATA forwards.
    pub packets: Option<Vec<GatewayRxpk>>,
    /// The report a PUSH_DATA carries.
    pub status: Option<GatewayStat>,
    /// What a PULL_RESP asks the gateway to transmit.
    pub transmit: Option<GatewayTxpk>,
    /// What a TX_ACK reports, in the protocol's own words, such as `NONE` or
    /// `COLLISION_PACKET`.
    pub tx_status: Option<String>,
}

/// Writes a datagram to send over a socket.
#[napi(js_name = "gatewayEncode")]
pub fn gateway_encode(packet: GatewayPacket) -> napi::Result<Buffer> {
    Ok(Buffer::from(packet_of(packet)?.to_bytes()))
}

/// Reads a datagram that arrived.
#[napi(js_name = "gatewayParse")]
pub fn gateway_parse(datagram: Buffer) -> napi::Result<GatewayPacket> {
    let packet =
        Packet::parse(&datagram).map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(packet_to_js(&packet))
}

/// Returns the acknowledgment a server owes a datagram, or `null` for one that needs none.
#[napi(js_name = "gatewayAcknowledgment")]
pub fn gateway_acknowledgment(packet: GatewayPacket) -> napi::Result<Option<GatewayPacket>> {
    Ok(packet_of(packet)?
        .acknowledgment()
        .as_ref()
        .map(packet_to_js))
}

/// Reads a datagram JavaScript describes.
fn packet_of(packet: GatewayPacket) -> napi::Result<Packet> {
    let token = packet.token;
    Ok(match packet.kind {
        GatewayPacketKind::PushData => Packet::PushData {
            token,
            gateway: gateway_of(packet.gateway.as_deref())?,
            uplink: Uplink {
                packets: packet
                    .packets
                    .unwrap_or_default()
                    .into_iter()
                    .map(rxpk_of)
                    .collect(),
                status: packet.status.map(stat_of),
            },
        },
        GatewayPacketKind::PushAck => Packet::PushAck { token },
        GatewayPacketKind::PullData => Packet::PullData {
            token,
            gateway: gateway_of(packet.gateway.as_deref())?,
        },
        GatewayPacketKind::PullAck => Packet::PullAck { token },
        GatewayPacketKind::PullResp => Packet::PullResp {
            token,
            transmit: txpk_of(packet.transmit.ok_or_else(|| {
                napi::Error::from_reason("a PullResp carries what to transmit in `transmit`")
            })?),
        },
        GatewayPacketKind::TxAck => Packet::TxAck {
            token,
            gateway: gateway_of(packet.gateway.as_deref())?,
            status: match packet.tx_status.as_deref() {
                None => TxStatus::None,
                Some(status) => TxStatus::named(status).ok_or_else(|| {
                    napi::Error::from_reason(format!("{status} is not a TX_ACK status"))
                })?,
            },
        },
    })
}

/// Describes a datagram for JavaScript.
fn packet_to_js(packet: &Packet) -> GatewayPacket {
    let (packets, status) = match packet {
        Packet::PushData { uplink, .. } => (
            Some(uplink.packets.iter().map(rxpk_to_js).collect()),
            uplink.status.as_ref().map(stat_to_js),
        ),
        _ => (None, None),
    };
    GatewayPacket {
        kind: kind_to_js(packet.kind()),
        token: packet.token(),
        gateway: packet.gateway().map(|gateway| gateway.to_hex()),
        packets,
        status,
        transmit: match packet {
            Packet::PullResp { transmit, .. } => Some(txpk_to_js(transmit)),
            _ => None,
        },
        tx_status: match packet {
            Packet::TxAck { status, .. } => Some(status.as_str().to_owned()),
            _ => None,
        },
    }
}

/// Reads a gateway identifier JavaScript passes.
fn gateway_of(text: Option<&str>) -> napi::Result<Eui> {
    let text = text.ok_or_else(|| {
        napi::Error::from_reason("this kind of datagram carries the gateway's identifier")
    })?;
    Eui::from_hex(text).ok_or_else(|| {
        napi::Error::from_reason(format!(
            "a gateway identifier is sixteen hexadecimal digits, not {text}"
        ))
    })
}

/// Names the kind for JavaScript.
fn kind_to_js(kind: PacketKind) -> GatewayPacketKind {
    match kind {
        PacketKind::PushData => GatewayPacketKind::PushData,
        PacketKind::PushAck => GatewayPacketKind::PushAck,
        PacketKind::PullData => GatewayPacketKind::PullData,
        PacketKind::PullResp => GatewayPacketKind::PullResp,
        PacketKind::PullAck => GatewayPacketKind::PullAck,
        PacketKind::TxAck => GatewayPacketKind::TxAck,
    }
}

/// Reads how a packet was modulated.
fn modulation_of(link: Option<LoraLink>, bitrate_bps: Option<u32>) -> Modulation {
    match (link, bitrate_bps) {
        (_, Some(bitrate)) => Modulation::Fsk(bitrate),
        (Some(link), None) => Modulation::Lora(settings(&link)),
        (None, None) => Modulation::Lora(pamoja_lora::LinkSettings::new(7, 125_000)),
    }
}

/// Describes how a packet was modulated.
fn modulation_to_js(modulation: Modulation) -> (Option<LoraLink>, Option<u32>) {
    match modulation {
        Modulation::Lora(link) => (Some(crate::lora::lora_link_of(link)), None),
        Modulation::Fsk(bitrate) => (None, Some(bitrate)),
    }
}

/// Reads a forwarded packet JavaScript describes.
fn rxpk_of(heard: GatewayRxpk) -> Rxpk {
    Rxpk {
        received_at: heard
            .received_at_us
            .map(|micros| pamoja_gateway::time::compact(micros as u64)),
        gps_millis: heard.gps_millis.map(|millis| millis as u64),
        timestamp_us: heard.timestamp_us,
        frequency_hz: heard.frequency_hz,
        channel: heard.channel.unwrap_or(0),
        rf_chain: heard.rf_chain.unwrap_or(0),
        crc: match heard.crc {
            None | Some(GatewayCrc::Ok) => CrcStatus::Ok,
            Some(GatewayCrc::Failed) => CrcStatus::Failed,
            Some(GatewayCrc::Absent) => CrcStatus::Absent,
        },
        modulation: modulation_of(heard.link, heard.bitrate_bps),
        rssi_dbm: heard.rssi_dbm.map_or(Decibels::ZERO, decibels),
        snr_db: heard.snr_db.map(decibels),
        payload: heard.payload.to_vec(),
    }
}

/// Describes a forwarded packet for JavaScript.
fn rxpk_to_js(heard: &Rxpk) -> GatewayRxpk {
    let (link, bitrate_bps) = modulation_to_js(heard.modulation);
    GatewayRxpk {
        frequency_hz: heard.frequency_hz,
        payload: Buffer::from(heard.payload.clone()),
        link,
        bitrate_bps,
        crc: Some(match heard.crc {
            CrcStatus::Ok => GatewayCrc::Ok,
            CrcStatus::Failed => GatewayCrc::Failed,
            CrcStatus::Absent => GatewayCrc::Absent,
        }),
        rssi_dbm: Some(db(heard.rssi_dbm)),
        snr_db: heard.snr_db.map(db),
        channel: Some(heard.channel),
        rf_chain: Some(heard.rf_chain),
        timestamp_us: heard.timestamp_us,
        received_at_us: heard
            .received_at
            .as_deref()
            .and_then(pamoja_gateway::time::from_compact)
            .map(|micros| micros as f64),
        gps_millis: heard.gps_millis.map(|millis| millis as f64),
    }
}

/// Reads a status report JavaScript describes.
fn stat_of(report: GatewayStat) -> Stat {
    Stat {
        time: report
            .time_s
            .map(|seconds| pamoja_gateway::time::expanded(seconds as u64)),
        latitude_deg: report.latitude_deg,
        longitude_deg: report.longitude_deg,
        altitude_m: report.altitude_m,
        received: report.received,
        received_ok: report.received_ok,
        forwarded: report.forwarded,
        acknowledged_percent: report.acknowledged_percent.unwrap_or(0.0),
        downlinks: report.downlinks,
        transmitted: report.transmitted,
    }
}

/// Describes a status report for JavaScript.
fn stat_to_js(report: &Stat) -> GatewayStat {
    GatewayStat {
        time_s: report
            .time
            .as_deref()
            .and_then(pamoja_gateway::time::from_expanded)
            .map(|seconds| seconds as f64),
        latitude_deg: report.latitude_deg,
        longitude_deg: report.longitude_deg,
        altitude_m: report.altitude_m,
        received: report.received,
        received_ok: report.received_ok,
        forwarded: report.forwarded,
        acknowledged_percent: Some(report.acknowledged_percent),
        downlinks: report.downlinks,
        transmitted: report.transmitted,
    }
}

/// Reads a transmission request JavaScript describes.
fn txpk_of(request: GatewayTxpk) -> Txpk {
    Txpk {
        immediate: request.immediate.unwrap_or(request.timestamp_us.is_none()),
        timestamp_us: request.timestamp_us,
        gps_millis: request.gps_millis.map(|millis| millis as u64),
        frequency_hz: request.frequency_hz,
        rf_chain: request.rf_chain.unwrap_or(0),
        power_dbm: request
            .power_dbm
            .unwrap_or(14)
            .clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8,
        modulation: modulation_of(request.link, request.bitrate_bps),
        frequency_deviation_hz: request.frequency_deviation_hz,
        invert_polarity: request.invert_polarity.unwrap_or(false),
        preamble_symbols: request.preamble_symbols,
        without_crc: request.without_crc.unwrap_or(false),
        payload: request.payload.to_vec(),
    }
}

/// Describes a transmission request for JavaScript.
fn txpk_to_js(request: &Txpk) -> GatewayTxpk {
    let (link, bitrate_bps) = modulation_to_js(request.modulation);
    GatewayTxpk {
        frequency_hz: request.frequency_hz,
        payload: Buffer::from(request.payload.clone()),
        link,
        bitrate_bps,
        immediate: Some(request.immediate),
        timestamp_us: request.timestamp_us,
        gps_millis: request.gps_millis.map(|millis| millis as f64),
        rf_chain: Some(request.rf_chain),
        power_dbm: Some(i32::from(request.power_dbm)),
        frequency_deviation_hz: request.frequency_deviation_hz,
        invert_polarity: Some(request.invert_polarity),
        preamble_symbols: request.preamble_symbols,
        without_crc: Some(request.without_crc),
    }
}
