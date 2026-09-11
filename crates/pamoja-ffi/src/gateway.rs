//! The C ABI for the LoRaWAN gateway protocols.
//!
//! These functions wrap [`pamoja_gateway`] for callers that reach the SDK through the flat C
//! boundary: the six datagrams a gateway and a network server exchange over UDP, and the
//! objects they carry.
//!
//! A datagram carries a payload of variable length, so a packet crosses as an opaque handle:
//! build one, read it with the `pamoja_gateway_packet_*` calls, write it with
//! [`pamoja_gateway_packet_to_buffer`], and release it with [`pamoja_gateway_packet_free`].
//! The packets a PUSH_DATA forwards are collected in an uplink handle first, since there may
//! be any number of them.
//!
//! Times cross as counts rather than as the strings the protocol writes: a reception in
//! microseconds since the Unix epoch and a gateway's clock in seconds, each with a flag that
//! says whether the datagram carried one at all.

use std::ptr;

use pamoja_gateway::udp::{
    CrcStatus, Eui, Modulation, Packet, PacketKind, Rxpk, Stat, TxStatus, Txpk, Uplink,
};
use pamoja_lora::budget::Decibels;
use pamoja_lora::LinkSettings;

use crate::lora::{settings, PamojaLoraLink};
use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// The protocol version every datagram starts with.
pub const PAMOJA_GATEWAY_PROTOCOL_VERSION: u8 = 2;

/// The length of a gateway's unique identifier.
pub const PAMOJA_GATEWAY_EUI_LEN: usize = 8;

/// The gateway forwarding what it heard.
pub const PAMOJA_GATEWAY_PUSH_DATA: u8 = 0x00;

/// The server acknowledging a PUSH_DATA.
pub const PAMOJA_GATEWAY_PUSH_ACK: u8 = 0x01;

/// The gateway holding its route open.
pub const PAMOJA_GATEWAY_PULL_DATA: u8 = 0x02;

/// The server sending a packet to transmit.
pub const PAMOJA_GATEWAY_PULL_RESP: u8 = 0x03;

/// The server acknowledging a PULL_DATA.
pub const PAMOJA_GATEWAY_PULL_ACK: u8 = 0x04;

/// The gateway reporting what became of a PULL_RESP.
pub const PAMOJA_GATEWAY_TX_ACK: u8 = 0x05;

/// A LoRa packet, whose settings are in the link.
pub const PAMOJA_GATEWAY_MODULATION_LORA: u8 = 0;

/// An FSK packet, whose bitrate is in `bitrate_bps`.
pub const PAMOJA_GATEWAY_MODULATION_FSK: u8 = 1;

/// The CRC checked.
pub const PAMOJA_GATEWAY_CRC_OK: i8 = 1;

/// The CRC failed.
pub const PAMOJA_GATEWAY_CRC_FAILED: i8 = -1;

/// The packet carried no CRC.
pub const PAMOJA_GATEWAY_CRC_ABSENT: i8 = 0;

/// The downlink was scheduled, which the protocol writes as `NONE`.
pub const PAMOJA_GATEWAY_TX_NONE: u8 = 0;

/// It arrived too late to schedule.
pub const PAMOJA_GATEWAY_TX_TOO_LATE: u8 = 1;

/// Its timestamp is too far ahead.
pub const PAMOJA_GATEWAY_TX_TOO_EARLY: u8 = 2;

/// Another packet was already scheduled then.
pub const PAMOJA_GATEWAY_TX_COLLISION_PACKET: u8 = 3;

/// A beacon was already scheduled then.
pub const PAMOJA_GATEWAY_TX_COLLISION_BEACON: u8 = 4;

/// The radio chain cannot reach that frequency.
pub const PAMOJA_GATEWAY_TX_FREQ: u8 = 5;

/// The gateway cannot transmit at that power.
pub const PAMOJA_GATEWAY_TX_POWER: u8 = 6;

/// A GPS timestamp was asked for while the GPS is unlocked.
pub const PAMOJA_GATEWAY_TX_GPS_UNLOCKED: u8 = 7;

const _: () = assert!(
    PAMOJA_GATEWAY_PROTOCOL_VERSION == pamoja_gateway::udp::PROTOCOL_VERSION,
    "the protocol version must match the crate"
);
const _: () = assert!(
    PAMOJA_GATEWAY_EUI_LEN == pamoja_gateway::udp::EUI_LEN,
    "the identifier length must match the crate"
);
const _: () = assert!(PAMOJA_GATEWAY_PUSH_DATA == PacketKind::PushData.identifier());
const _: () = assert!(PAMOJA_GATEWAY_PUSH_ACK == PacketKind::PushAck.identifier());
const _: () = assert!(PAMOJA_GATEWAY_PULL_DATA == PacketKind::PullData.identifier());
const _: () = assert!(PAMOJA_GATEWAY_PULL_RESP == PacketKind::PullResp.identifier());
const _: () = assert!(PAMOJA_GATEWAY_PULL_ACK == PacketKind::PullAck.identifier());
const _: () = assert!(PAMOJA_GATEWAY_TX_ACK == PacketKind::TxAck.identifier());
const _: () = assert!(PAMOJA_GATEWAY_CRC_OK == CrcStatus::Ok.code());
const _: () = assert!(PAMOJA_GATEWAY_CRC_FAILED == CrcStatus::Failed.code());
const _: () = assert!(PAMOJA_GATEWAY_CRC_ABSENT == CrcStatus::Absent.code());

/// An opaque handle to one datagram of the protocol.
///
/// Read it with the `pamoja_gateway_packet_*` calls, then release it with
/// [`pamoja_gateway_packet_free`].
pub struct PamojaGatewayPacket {
    packet: Packet,
}

/// An opaque handle to what a PUSH_DATA carries: the packets heard, and the report.
///
/// Fill it with [`pamoja_gateway_uplink_add_rxpk`] and [`pamoja_gateway_uplink_set_stat`],
/// then hand it to [`pamoja_gateway_push_data`], which takes it over.
pub struct PamojaGatewayUplink {
    uplink: Uplink,
}

/// A packet the gateway heard, without its payload, which crosses beside it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PamojaGatewayRxpk {
    /// When it arrived, in microseconds since 1970-01-01 UTC, when `has_received_at`.
    pub received_at_us: u64,
    /// When it arrived on the GPS clock, in milliseconds, when `has_gps_millis`.
    pub gps_millis: u64,
    /// The carrier it arrived on, in hertz.
    pub frequency_hz: u32,
    /// The concentrator's own timestamp of the reception, when `has_timestamp`.
    pub timestamp_us: u32,
    /// The spreading factor, bandwidth, coding rate, header, and CRC, for a LoRa packet.
    pub link: PamojaLoraLink,
    /// The bitrate in bits per second, for an FSK packet.
    pub bitrate_bps: u32,
    /// The received signal strength, in hundredths of a dBm.
    pub rssi_centi_dbm: i32,
    /// The signal-to-noise ratio, in hundredths of a dB, when `has_snr`.
    pub snr_centi_db: i32,
    /// The concentrator channel it arrived on.
    pub channel: u8,
    /// The radio chain it arrived on.
    pub rf_chain: u8,
    /// What the CRC said: [`PAMOJA_GATEWAY_CRC_OK`], `_FAILED`, or `_ABSENT`.
    pub crc: i8,
    /// [`PAMOJA_GATEWAY_MODULATION_LORA`] or `_FSK`.
    pub modulation: u8,
    /// Whether the datagram carried a reception time.
    pub has_received_at: bool,
    /// Whether it carried a GPS time.
    pub has_gps_millis: bool,
    /// Whether it carried the concentrator's timestamp.
    pub has_timestamp: bool,
    /// Whether it carried a signal-to-noise ratio.
    pub has_snr: bool,
}

/// A gateway's own status report.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PamojaGatewayStat {
    /// The gateway's clock, in seconds since 1970-01-01 UTC, when `has_time`.
    pub time_s: u64,
    /// Its latitude in degrees, north positive, when `has_position`.
    pub latitude_deg: f64,
    /// Its longitude in degrees, east positive, when `has_position`.
    pub longitude_deg: f64,
    /// What share of its datagrams were acknowledged, as a percentage.
    pub acknowledged_percent: f64,
    /// Its altitude in meters, when `has_altitude`.
    pub altitude_m: i32,
    /// How many packets its radio received.
    pub received: u32,
    /// How many of those had a good CRC.
    pub received_ok: u32,
    /// How many it forwarded.
    pub forwarded: u32,
    /// How many downlink datagrams it received.
    pub downlinks: u32,
    /// How many packets it transmitted.
    pub transmitted: u32,
    /// Whether the report carried a clock reading.
    pub has_time: bool,
    /// Whether it carried a position.
    pub has_position: bool,
    /// Whether it carried an altitude.
    pub has_altitude: bool,
}

/// A packet the server asks the gateway to transmit, without its payload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PamojaGatewayTxpk {
    /// The GPS time to transmit at, in milliseconds, when `has_gps_millis`.
    pub gps_millis: u64,
    /// The carrier to transmit on, in hertz.
    pub frequency_hz: u32,
    /// The concentrator timestamp to transmit at, when `has_timestamp`.
    pub timestamp_us: u32,
    /// The spreading factor, bandwidth, and coding rate, for a LoRa packet.
    pub link: PamojaLoraLink,
    /// The bitrate in bits per second, for an FSK packet.
    pub bitrate_bps: u32,
    /// The FSK frequency deviation in hertz, when `has_deviation`.
    pub frequency_deviation_hz: u32,
    /// How long a preamble to send, in symbols, when `has_preamble`.
    pub preamble_symbols: u16,
    /// The radio chain to transmit from.
    pub rf_chain: u8,
    /// The power to transmit at, in dBm.
    pub power_dbm: i8,
    /// [`PAMOJA_GATEWAY_MODULATION_LORA`] or `_FSK`.
    pub modulation: u8,
    /// Whether to transmit at once, which ignores the timestamps.
    pub immediate: bool,
    /// Whether to invert the LoRa polarity, as a LoRaWAN downlink is sent.
    pub invert_polarity: bool,
    /// Whether to leave the physical CRC off, as LoRaWAN downlinks are.
    pub without_crc: bool,
    /// Whether a concentrator timestamp was given.
    pub has_timestamp: bool,
    /// Whether a GPS time was given.
    pub has_gps_millis: bool,
    /// Whether an FSK deviation was given.
    pub has_deviation: bool,
    /// Whether a preamble length was given.
    pub has_preamble: bool,
}

/// Creates an empty uplink for a PUSH_DATA to carry.
///
/// # Returns
///
/// A handle the caller releases with [`pamoja_gateway_uplink_free`], or hands to
/// [`pamoja_gateway_push_data`], which takes it over.
#[no_mangle]
pub extern "C" fn pamoja_gateway_uplink_new() -> *mut PamojaGatewayUplink {
    Box::into_raw(Box::new(PamojaGatewayUplink {
        uplink: Uplink::default(),
    }))
}

/// Adds a packet the gateway heard.
///
/// # Arguments
///
/// * `uplink` - the uplink being built.
/// * `packet` - the metadata, whose `modulation` says how to read its link or bitrate.
/// * `payload` - the packet itself.
/// * `payload_len` - its length.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null handle or a null
/// payload with a length.
///
/// # Safety
///
/// `uplink` must be a live handle from [`pamoja_gateway_uplink_new`], and `payload` must
/// point at `payload_len` readable bytes or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_uplink_add_rxpk(
    uplink: *mut PamojaGatewayUplink,
    packet: PamojaGatewayRxpk,
    payload: *const u8,
    payload_len: usize,
) -> PamojaStatus {
    let Some(uplink) = uplink.as_mut() else {
        return missing("uplink");
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    uplink.uplink.packets.push(rxpk_of(packet, payload));
    PamojaStatus::Ok
}

/// Sets the gateway's status report on an uplink.
///
/// # Arguments
///
/// * `uplink` - the uplink being built.
/// * `status` - the report.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null handle.
///
/// # Safety
///
/// `uplink` must be a live handle from [`pamoja_gateway_uplink_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_uplink_set_stat(
    uplink: *mut PamojaGatewayUplink,
    status: PamojaGatewayStat,
) -> PamojaStatus {
    let Some(uplink) = uplink.as_mut() else {
        return missing("uplink");
    };
    uplink.uplink.status = Some(stat_of(status));
    PamojaStatus::Ok
}

/// Releases an uplink that will not be sent.
///
/// # Arguments
///
/// * `uplink` - the uplink, which must not be used again.
///
/// # Safety
///
/// `uplink` must be a live handle from [`pamoja_gateway_uplink_new`] that was not handed to
/// [`pamoja_gateway_push_data`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_uplink_free(uplink: *mut PamojaGatewayUplink) {
    if !uplink.is_null() {
        drop(Box::from_raw(uplink));
    }
}

/// Builds the PUSH_DATA that forwards an uplink.
///
/// # Arguments
///
/// * `token` - the random token the acknowledgment carries back.
/// * `gateway` - the gateway's eight-byte identifier.
/// * `uplink` - the uplink, which this call takes over and releases.
/// * `out_packet` - receives the packet.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `gateway` must point at [`PAMOJA_GATEWAY_EUI_LEN`] readable bytes, `uplink` must be a live
/// handle that is not used again, and `out_packet` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_push_data(
    token: u16,
    gateway: *const u8,
    uplink: *mut PamojaGatewayUplink,
    out_packet: *mut *mut PamojaGatewayPacket,
) -> PamojaStatus {
    if !clear(out_packet) {
        return PamojaStatus::InvalidArgument;
    }
    let Some(identifier) = eui_of(gateway) else {
        return missing("gateway");
    };
    if uplink.is_null() {
        return missing("uplink");
    }
    let uplink = Box::from_raw(uplink).uplink;
    *out_packet = wrap(Packet::PushData {
        token,
        gateway: identifier,
        uplink,
    });
    PamojaStatus::Ok
}

/// Builds the PUSH_ACK that answers a PUSH_DATA.
///
/// # Arguments
///
/// * `token` - the token of the datagram being acknowledged.
///
/// # Returns
///
/// A handle the caller releases with [`pamoja_gateway_packet_free`].
#[no_mangle]
pub extern "C" fn pamoja_gateway_push_ack(token: u16) -> *mut PamojaGatewayPacket {
    wrap(Packet::PushAck { token })
}

/// Builds the PULL_DATA that holds a route open.
///
/// # Arguments
///
/// * `token` - the random token the acknowledgment carries back.
/// * `gateway` - the gateway's eight-byte identifier.
/// * `out_packet` - receives the packet.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `gateway` must point at [`PAMOJA_GATEWAY_EUI_LEN`] readable bytes and `out_packet` must be
/// writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_pull_data(
    token: u16,
    gateway: *const u8,
    out_packet: *mut *mut PamojaGatewayPacket,
) -> PamojaStatus {
    if !clear(out_packet) {
        return PamojaStatus::InvalidArgument;
    }
    let Some(identifier) = eui_of(gateway) else {
        return missing("gateway");
    };
    *out_packet = wrap(Packet::PullData {
        token,
        gateway: identifier,
    });
    PamojaStatus::Ok
}

/// Builds the PULL_ACK that answers a PULL_DATA.
///
/// # Arguments
///
/// * `token` - the token of the datagram being acknowledged.
///
/// # Returns
///
/// A handle the caller releases with [`pamoja_gateway_packet_free`].
#[no_mangle]
pub extern "C" fn pamoja_gateway_pull_ack(token: u16) -> *mut PamojaGatewayPacket {
    wrap(Packet::PullAck { token })
}

/// Builds the PULL_RESP that asks a gateway to transmit.
///
/// # Arguments
///
/// * `token` - the random token the TX_ACK carries back.
/// * `transmit` - what to transmit, and when.
/// * `payload` - the packet itself.
/// * `payload_len` - its length.
/// * `out_packet` - receives the packet.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `payload` must point at `payload_len` readable bytes or be null, and `out_packet` must be
/// writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_pull_resp(
    token: u16,
    transmit: PamojaGatewayTxpk,
    payload: *const u8,
    payload_len: usize,
    out_packet: *mut *mut PamojaGatewayPacket,
) -> PamojaStatus {
    if !clear(out_packet) {
        return PamojaStatus::InvalidArgument;
    }
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    *out_packet = wrap(Packet::PullResp {
        token,
        transmit: txpk_of(transmit, payload),
    });
    PamojaStatus::Ok
}

/// Builds the TX_ACK that reports what became of a PULL_RESP.
///
/// # Arguments
///
/// * `token` - the token of the PULL_RESP being answered.
/// * `gateway` - the gateway's eight-byte identifier.
/// * `status` - [`PAMOJA_GATEWAY_TX_NONE`] when it was scheduled, or why it was refused.
/// * `out_packet` - receives the packet.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument or a status
/// the protocol does not define.
///
/// # Safety
///
/// `gateway` must point at [`PAMOJA_GATEWAY_EUI_LEN`] readable bytes and `out_packet` must be
/// writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_tx_ack(
    token: u16,
    gateway: *const u8,
    status: u8,
    out_packet: *mut *mut PamojaGatewayPacket,
) -> PamojaStatus {
    if !clear(out_packet) {
        return PamojaStatus::InvalidArgument;
    }
    let Some(identifier) = eui_of(gateway) else {
        return missing("gateway");
    };
    let Some(status) = tx_status_of(status) else {
        set_last_error(format!("{status} is not a TX_ACK status"));
        return PamojaStatus::InvalidArgument;
    };
    *out_packet = wrap(Packet::TxAck {
        token,
        gateway: identifier,
        status,
    });
    PamojaStatus::Ok
}

/// Reads a datagram.
///
/// # Arguments
///
/// * `bytes` - the datagram as it arrived.
/// * `len` - its length.
/// * `out_packet` - receives the packet.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::Codec`] for a datagram this protocol does not
/// describe, with the reason in the last error message.
///
/// # Safety
///
/// `bytes` must point at `len` readable bytes or be null, and `out_packet` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_parse(
    bytes: *const u8,
    len: usize,
    out_packet: *mut *mut PamojaGatewayPacket,
) -> PamojaStatus {
    if !clear(out_packet) {
        return PamojaStatus::InvalidArgument;
    }
    let datagram = match read_bytes(bytes, len) {
        Ok(datagram) => datagram,
        Err(status) => return status,
    };
    match Packet::parse(&datagram) {
        Ok(packet) => {
            *out_packet = wrap(packet);
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Codec
        }
    }
}

/// Returns which kind of datagram a packet is.
///
/// # Arguments
///
/// * `packet` - the packet.
///
/// # Returns
///
/// One of [`PAMOJA_GATEWAY_PUSH_DATA`] through [`PAMOJA_GATEWAY_TX_ACK`], or 255 if `packet`
/// is null.
///
/// # Safety
///
/// `packet` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_kind(packet: *const PamojaGatewayPacket) -> u8 {
    match packet.as_ref() {
        Some(packet) => packet.packet.kind().identifier(),
        None => u8::MAX,
    }
}

/// Returns a packet's token, which pairs it with its answer.
///
/// # Arguments
///
/// * `packet` - the packet.
///
/// # Returns
///
/// The token, or 0 if `packet` is null.
///
/// # Safety
///
/// `packet` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_token(packet: *const PamojaGatewayPacket) -> u16 {
    packet.as_ref().map_or(0, |packet| packet.packet.token())
}

/// Writes the gateway's identifier, for the datagrams that carry one.
///
/// # Arguments
///
/// * `packet` - the packet.
/// * `out_gateway` - receives [`PAMOJA_GATEWAY_EUI_LEN`] bytes.
///
/// # Returns
///
/// `true` when the packet carries an identifier, which the datagrams a server sends do not.
///
/// # Safety
///
/// `packet` must be a live handle or null, and `out_gateway` must point at
/// [`PAMOJA_GATEWAY_EUI_LEN`] writable bytes or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_gateway(
    packet: *const PamojaGatewayPacket,
    out_gateway: *mut u8,
) -> bool {
    let Some(packet) = packet.as_ref() else {
        return false;
    };
    let Some(identifier) = packet.packet.gateway() else {
        return false;
    };
    if out_gateway.is_null() {
        return false;
    }
    ptr::copy_nonoverlapping(
        identifier.bytes().as_ptr(),
        out_gateway,
        PAMOJA_GATEWAY_EUI_LEN,
    );
    true
}

/// Returns how many packets a PUSH_DATA forwards.
///
/// # Arguments
///
/// * `packet` - the packet.
///
/// # Returns
///
/// The count, or 0 for any other kind.
///
/// # Safety
///
/// `packet` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_rxpk_count(
    packet: *const PamojaGatewayPacket,
) -> usize {
    match packet.as_ref().map(|packet| &packet.packet) {
        Some(Packet::PushData { uplink, .. }) => uplink.packets.len(),
        _ => 0,
    }
}

/// Reads one of the packets a PUSH_DATA forwards.
///
/// # Arguments
///
/// * `packet` - the packet.
/// * `index` - which forwarded packet, from zero.
/// * `out_rxpk` - receives its metadata.
///
/// # Returns
///
/// `true` when there is a packet at that index.
///
/// # Safety
///
/// `packet` must be a live handle or null, and `out_rxpk` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_rxpk(
    packet: *const PamojaGatewayPacket,
    index: usize,
    out_rxpk: *mut PamojaGatewayRxpk,
) -> bool {
    let Some(heard) = heard_at(packet, index) else {
        return false;
    };
    if out_rxpk.is_null() {
        return false;
    }
    *out_rxpk = rxpk_to_c(heard);
    true
}

/// Returns a pointer to one forwarded packet's payload.
///
/// Use [`pamoja_gateway_packet_rxpk_payload_len`] for its length. The pointer is valid until
/// the packet is freed.
///
/// # Arguments
///
/// * `packet` - the packet.
/// * `index` - which forwarded packet, from zero.
///
/// # Returns
///
/// The pointer, or null when there is no packet at that index.
///
/// # Safety
///
/// `packet` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_rxpk_payload(
    packet: *const PamojaGatewayPacket,
    index: usize,
) -> *const u8 {
    heard_at(packet, index).map_or(ptr::null(), |heard| heard.payload.as_ptr())
}

/// Returns the length of one forwarded packet's payload.
///
/// # Arguments
///
/// * `packet` - the packet.
/// * `index` - which forwarded packet, from zero.
///
/// # Returns
///
/// The length, or 0 when there is no packet at that index.
///
/// # Safety
///
/// `packet` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_rxpk_payload_len(
    packet: *const PamojaGatewayPacket,
    index: usize,
) -> usize {
    heard_at(packet, index).map_or(0, |heard| heard.payload.len())
}

/// Reads the gateway's status report, when a PUSH_DATA carries one.
///
/// # Arguments
///
/// * `packet` - the packet.
/// * `out_stat` - receives the report.
///
/// # Returns
///
/// `true` when the datagram carried a report.
///
/// # Safety
///
/// `packet` must be a live handle or null, and `out_stat` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_stat(
    packet: *const PamojaGatewayPacket,
    out_stat: *mut PamojaGatewayStat,
) -> bool {
    let report = match packet.as_ref().map(|packet| &packet.packet) {
        Some(Packet::PushData { uplink, .. }) => uplink.status.as_ref(),
        _ => None,
    };
    let Some(report) = report else {
        return false;
    };
    if out_stat.is_null() {
        return false;
    }
    *out_stat = stat_to_c(report);
    true
}

/// Reads what a PULL_RESP asks the gateway to transmit.
///
/// # Arguments
///
/// * `packet` - the packet.
/// * `out_txpk` - receives the request.
///
/// # Returns
///
/// `true` when the packet is a PULL_RESP.
///
/// # Safety
///
/// `packet` must be a live handle or null, and `out_txpk` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_txpk(
    packet: *const PamojaGatewayPacket,
    out_txpk: *mut PamojaGatewayTxpk,
) -> bool {
    let Some(request) = requested(packet) else {
        return false;
    };
    if out_txpk.is_null() {
        return false;
    }
    *out_txpk = txpk_to_c(request);
    true
}

/// Returns a pointer to the payload a PULL_RESP carries.
///
/// Use [`pamoja_gateway_packet_txpk_payload_len`] for its length. The pointer is valid until
/// the packet is freed.
///
/// # Arguments
///
/// * `packet` - the packet.
///
/// # Returns
///
/// The pointer, or null when the packet is not a PULL_RESP.
///
/// # Safety
///
/// `packet` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_txpk_payload(
    packet: *const PamojaGatewayPacket,
) -> *const u8 {
    requested(packet).map_or(ptr::null(), |request| request.payload.as_ptr())
}

/// Returns the length of the payload a PULL_RESP carries.
///
/// # Arguments
///
/// * `packet` - the packet.
///
/// # Returns
///
/// The length, or 0 when the packet is not a PULL_RESP.
///
/// # Safety
///
/// `packet` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_txpk_payload_len(
    packet: *const PamojaGatewayPacket,
) -> usize {
    requested(packet).map_or(0, |request| request.payload.len())
}

/// Returns what a TX_ACK reports.
///
/// # Arguments
///
/// * `packet` - the packet.
///
/// # Returns
///
/// One of [`PAMOJA_GATEWAY_TX_NONE`] through [`PAMOJA_GATEWAY_TX_GPS_UNLOCKED`], or 255 when
/// the packet is not a TX_ACK.
///
/// # Safety
///
/// `packet` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_tx_status(packet: *const PamojaGatewayPacket) -> u8 {
    match packet.as_ref().map(|packet| &packet.packet) {
        Some(Packet::TxAck { status, .. }) => tx_status_code(*status),
        _ => u8::MAX,
    }
}

/// Builds the acknowledgment a server owes a datagram.
///
/// # Arguments
///
/// * `packet` - the datagram that arrived.
/// * `out_packet` - receives the acknowledgment.
///
/// # Returns
///
/// `true` for a PUSH_DATA or a PULL_DATA, which are the datagrams a server acknowledges. A
/// PULL_RESP is answered with a TX_ACK, which names the gateway, so the gateway builds that
/// one with [`pamoja_gateway_tx_ack`].
///
/// # Safety
///
/// `packet` must be a live handle or null, and `out_packet` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_acknowledgment(
    packet: *const PamojaGatewayPacket,
    out_packet: *mut *mut PamojaGatewayPacket,
) -> bool {
    if !clear(out_packet) {
        return false;
    }
    let Some(packet) = packet.as_ref() else {
        return false;
    };
    match packet.packet.acknowledgment() {
        Some(acknowledgment) => {
            *out_packet = wrap(acknowledgment);
            true
        }
        None => false,
    }
}

/// Writes a packet as the datagram to send.
///
/// # Arguments
///
/// * `packet` - the packet.
///
/// # Returns
///
/// A buffer the caller releases with `pamoja_buffer_free`, or null if `packet` is null.
///
/// # Safety
///
/// `packet` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_to_buffer(
    packet: *const PamojaGatewayPacket,
) -> *mut PamojaBuffer {
    match packet.as_ref() {
        Some(packet) => PamojaBuffer::into_raw(packet.packet.to_bytes()),
        None => ptr::null_mut(),
    }
}

/// Releases a packet.
///
/// # Arguments
///
/// * `packet` - the packet, which must not be used again.
///
/// # Safety
///
/// `packet` must be a live handle from one of the builders or from
/// [`pamoja_gateway_packet_parse`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_packet_free(packet: *mut PamojaGatewayPacket) {
    if !packet.is_null() {
        drop(Box::from_raw(packet));
    }
}

/// Wraps a packet in a handle for the caller to own.
fn wrap(packet: Packet) -> *mut PamojaGatewayPacket {
    Box::into_raw(Box::new(PamojaGatewayPacket { packet }))
}

/// Nulls an out pointer before a handle is written into it, refusing a null pointer.
unsafe fn clear(out_packet: *mut *mut PamojaGatewayPacket) -> bool {
    if out_packet.is_null() {
        set_last_error("out_packet must not be null".to_owned());
        return false;
    }
    *out_packet = ptr::null_mut();
    true
}

/// Records a null argument and reports it.
fn missing(name: &str) -> PamojaStatus {
    set_last_error(format!("{name} must not be null"));
    PamojaStatus::InvalidArgument
}

/// Reads a gateway identifier from the boundary.
unsafe fn eui_of(gateway: *const u8) -> Option<Eui> {
    if gateway.is_null() {
        return None;
    }
    let mut bytes = [0u8; PAMOJA_GATEWAY_EUI_LEN];
    ptr::copy_nonoverlapping(gateway, bytes.as_mut_ptr(), PAMOJA_GATEWAY_EUI_LEN);
    Some(Eui::new(bytes))
}

/// Borrows one of the packets a PUSH_DATA forwards.
unsafe fn heard_at<'a>(packet: *const PamojaGatewayPacket, index: usize) -> Option<&'a Rxpk> {
    match packet.as_ref().map(|packet| &packet.packet) {
        Some(Packet::PushData { uplink, .. }) => uplink.packets.get(index),
        _ => None,
    }
}

/// Borrows what a PULL_RESP asks for.
unsafe fn requested<'a>(packet: *const PamojaGatewayPacket) -> Option<&'a Txpk> {
    match packet.as_ref().map(|packet| &packet.packet) {
        Some(Packet::PullResp { transmit, .. }) => Some(transmit),
        _ => None,
    }
}

/// Names the TX_ACK status a code selects.
fn tx_status_of(code: u8) -> Option<TxStatus> {
    Some(match code {
        PAMOJA_GATEWAY_TX_NONE => TxStatus::None,
        PAMOJA_GATEWAY_TX_TOO_LATE => TxStatus::TooLate,
        PAMOJA_GATEWAY_TX_TOO_EARLY => TxStatus::TooEarly,
        PAMOJA_GATEWAY_TX_COLLISION_PACKET => TxStatus::CollisionPacket,
        PAMOJA_GATEWAY_TX_COLLISION_BEACON => TxStatus::CollisionBeacon,
        PAMOJA_GATEWAY_TX_FREQ => TxStatus::TxFreq,
        PAMOJA_GATEWAY_TX_POWER => TxStatus::TxPower,
        PAMOJA_GATEWAY_TX_GPS_UNLOCKED => TxStatus::GpsUnlocked,
        _ => return None,
    })
}

/// Returns the code a TX_ACK status crosses as.
fn tx_status_code(status: TxStatus) -> u8 {
    match status {
        TxStatus::None => PAMOJA_GATEWAY_TX_NONE,
        TxStatus::TooLate => PAMOJA_GATEWAY_TX_TOO_LATE,
        TxStatus::TooEarly => PAMOJA_GATEWAY_TX_TOO_EARLY,
        TxStatus::CollisionPacket => PAMOJA_GATEWAY_TX_COLLISION_PACKET,
        TxStatus::CollisionBeacon => PAMOJA_GATEWAY_TX_COLLISION_BEACON,
        TxStatus::TxFreq => PAMOJA_GATEWAY_TX_FREQ,
        TxStatus::TxPower => PAMOJA_GATEWAY_TX_POWER,
        TxStatus::GpsUnlocked => PAMOJA_GATEWAY_TX_GPS_UNLOCKED,
    }
}

/// Reads a modulation from the boundary.
fn modulation_of(code: u8, link: PamojaLoraLink, bitrate_bps: u32) -> Modulation {
    if code == PAMOJA_GATEWAY_MODULATION_FSK {
        Modulation::Fsk(bitrate_bps)
    } else {
        Modulation::Lora(settings(link))
    }
}

/// Flattens a modulation for the boundary.
fn modulation_to_c(modulation: Modulation) -> (u8, PamojaLoraLink, u32) {
    match modulation {
        Modulation::Lora(link) => (PAMOJA_GATEWAY_MODULATION_LORA, link_to_c(link), 0),
        Modulation::Fsk(bitrate) => (
            PAMOJA_GATEWAY_MODULATION_FSK,
            link_to_c(LinkSettings::new(7, 125_000)),
            bitrate,
        ),
    }
}

/// Flattens link settings for the boundary.
fn link_to_c(link: LinkSettings) -> PamojaLoraLink {
    PamojaLoraLink {
        bandwidth_hz: link.bandwidth_hz(),
        preamble_symbols: link.preamble_symbols(),
        spreading_factor: link.spreading_factor(),
        coding_rate_denominator: link.coding_rate_denominator(),
        explicit_header: u8::from(link.explicit_header()),
        crc: u8::from(link.crc()),
    }
}

/// Reads a forwarded packet from the boundary.
fn rxpk_of(packet: PamojaGatewayRxpk, payload: Vec<u8>) -> Rxpk {
    Rxpk {
        received_at: packet
            .has_received_at
            .then(|| pamoja_gateway::time::compact(packet.received_at_us)),
        gps_millis: packet.has_gps_millis.then_some(packet.gps_millis),
        timestamp_us: packet.has_timestamp.then_some(packet.timestamp_us),
        frequency_hz: packet.frequency_hz,
        channel: packet.channel,
        rf_chain: packet.rf_chain,
        crc: CrcStatus::from_code(i64::from(packet.crc)).unwrap_or(CrcStatus::Absent),
        modulation: modulation_of(packet.modulation, packet.link, packet.bitrate_bps),
        rssi_dbm: Decibels::from_hundredths(packet.rssi_centi_dbm),
        snr_db: packet
            .has_snr
            .then(|| Decibels::from_hundredths(packet.snr_centi_db)),
        payload,
    }
}

/// Flattens a forwarded packet for the boundary.
fn rxpk_to_c(heard: &Rxpk) -> PamojaGatewayRxpk {
    let (modulation, link, bitrate_bps) = modulation_to_c(heard.modulation);
    let received_at = heard
        .received_at
        .as_deref()
        .and_then(pamoja_gateway::time::from_compact);
    PamojaGatewayRxpk {
        received_at_us: received_at.unwrap_or(0),
        gps_millis: heard.gps_millis.unwrap_or(0),
        frequency_hz: heard.frequency_hz,
        timestamp_us: heard.timestamp_us.unwrap_or(0),
        link,
        bitrate_bps,
        rssi_centi_dbm: heard.rssi_dbm.hundredths(),
        snr_centi_db: heard.snr_db.map_or(0, Decibels::hundredths),
        channel: heard.channel,
        rf_chain: heard.rf_chain,
        crc: heard.crc.code(),
        modulation,
        has_received_at: received_at.is_some(),
        has_gps_millis: heard.gps_millis.is_some(),
        has_timestamp: heard.timestamp_us.is_some(),
        has_snr: heard.snr_db.is_some(),
    }
}

/// Reads a status report from the boundary.
fn stat_of(report: PamojaGatewayStat) -> Stat {
    Stat {
        time: report
            .has_time
            .then(|| pamoja_gateway::time::expanded(report.time_s)),
        latitude_deg: report.has_position.then_some(report.latitude_deg),
        longitude_deg: report.has_position.then_some(report.longitude_deg),
        altitude_m: report.has_altitude.then_some(report.altitude_m),
        received: report.received,
        received_ok: report.received_ok,
        forwarded: report.forwarded,
        acknowledged_percent: report.acknowledged_percent,
        downlinks: report.downlinks,
        transmitted: report.transmitted,
    }
}

/// Flattens a status report for the boundary.
fn stat_to_c(report: &Stat) -> PamojaGatewayStat {
    let clock = report
        .time
        .as_deref()
        .and_then(pamoja_gateway::time::from_expanded);
    PamojaGatewayStat {
        time_s: clock.unwrap_or(0),
        latitude_deg: report.latitude_deg.unwrap_or(0.0),
        longitude_deg: report.longitude_deg.unwrap_or(0.0),
        acknowledged_percent: report.acknowledged_percent,
        altitude_m: report.altitude_m.unwrap_or(0),
        received: report.received,
        received_ok: report.received_ok,
        forwarded: report.forwarded,
        downlinks: report.downlinks,
        transmitted: report.transmitted,
        has_time: clock.is_some(),
        has_position: report.latitude_deg.is_some() && report.longitude_deg.is_some(),
        has_altitude: report.altitude_m.is_some(),
    }
}

/// Reads a transmission request from the boundary.
fn txpk_of(request: PamojaGatewayTxpk, payload: Vec<u8>) -> Txpk {
    Txpk {
        immediate: request.immediate,
        timestamp_us: request.has_timestamp.then_some(request.timestamp_us),
        gps_millis: request.has_gps_millis.then_some(request.gps_millis),
        frequency_hz: request.frequency_hz,
        rf_chain: request.rf_chain,
        power_dbm: request.power_dbm,
        modulation: modulation_of(request.modulation, request.link, request.bitrate_bps),
        frequency_deviation_hz: request
            .has_deviation
            .then_some(request.frequency_deviation_hz),
        invert_polarity: request.invert_polarity,
        preamble_symbols: request.has_preamble.then_some(request.preamble_symbols),
        without_crc: request.without_crc,
        payload,
    }
}

/// Flattens a transmission request for the boundary.
fn txpk_to_c(request: &Txpk) -> PamojaGatewayTxpk {
    let (modulation, link, bitrate_bps) = modulation_to_c(request.modulation);
    PamojaGatewayTxpk {
        gps_millis: request.gps_millis.unwrap_or(0),
        frequency_hz: request.frequency_hz,
        timestamp_us: request.timestamp_us.unwrap_or(0),
        link,
        bitrate_bps,
        frequency_deviation_hz: request.frequency_deviation_hz.unwrap_or(0),
        preamble_symbols: request.preamble_symbols.unwrap_or(0),
        rf_chain: request.rf_chain,
        power_dbm: request.power_dbm,
        modulation,
        immediate: request.immediate,
        invert_polarity: request.invert_polarity,
        without_crc: request.without_crc,
        has_timestamp: request.timestamp_us.is_some(),
        has_gps_millis: request.gps_millis.is_some(),
        has_deviation: request.frequency_deviation_hz.is_some(),
        has_preamble: request.preamble_symbols.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lora::pamoja_lora_link_default;
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    const GATEWAY: [u8; 8] = [0xB8, 0x27, 0xEB, 0xFF, 0xFE, 0x01, 0x02, 0x03];

    fn bytes_of(packet: *const PamojaGatewayPacket) -> Vec<u8> {
        unsafe {
            let buffer = pamoja_gateway_packet_to_buffer(packet);
            let len = pamoja_buffer_len(buffer);
            let bytes = std::slice::from_raw_parts(pamoja_buffer_data(buffer), len).to_vec();
            pamoja_buffer_free(buffer);
            bytes
        }
    }

    #[test]
    fn a_push_data_crosses_and_comes_back() {
        unsafe {
            let uplink = pamoja_gateway_uplink_new();
            let heard = PamojaGatewayRxpk {
                frequency_hz: 868_100_000,
                link: pamoja_lora_link_default(7, 125_000),
                rssi_centi_dbm: -3_500,
                snr_centi_db: 510,
                has_snr: true,
                timestamp_us: 3_512_348_611,
                has_timestamp: true,
                received_at_us: 1_364_746_877_528_002,
                has_received_at: true,
                crc: PAMOJA_GATEWAY_CRC_OK,
                channel: 2,
                ..PamojaGatewayRxpk::default()
            };
            let payload = b"TEST_PACKET_1234";
            assert_eq!(
                pamoja_gateway_uplink_add_rxpk(uplink, heard, payload.as_ptr(), payload.len()),
                PamojaStatus::Ok
            );

            let mut packet = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_push_data(0x1234, GATEWAY.as_ptr(), uplink, &mut packet),
                PamojaStatus::Ok
            );
            let datagram = bytes_of(packet);
            pamoja_gateway_packet_free(packet);

            let mut read = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_packet_parse(datagram.as_ptr(), datagram.len(), &mut read),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_gateway_packet_kind(read), PAMOJA_GATEWAY_PUSH_DATA);
            assert_eq!(pamoja_gateway_packet_token(read), 0x1234);
            let mut identifier = [0u8; 8];
            assert!(pamoja_gateway_packet_gateway(read, identifier.as_mut_ptr()));
            assert_eq!(identifier, GATEWAY);
            assert_eq!(pamoja_gateway_packet_rxpk_count(read), 1);

            let mut back = PamojaGatewayRxpk::default();
            assert!(pamoja_gateway_packet_rxpk(read, 0, &mut back));
            assert_eq!(back, heard);
            let len = pamoja_gateway_packet_rxpk_payload_len(read, 0);
            let carried =
                std::slice::from_raw_parts(pamoja_gateway_packet_rxpk_payload(read, 0), len);
            assert_eq!(carried, payload);

            let mut ack = ptr::null_mut();
            assert!(pamoja_gateway_packet_acknowledgment(read, &mut ack));
            assert_eq!(bytes_of(ack), [2, 0x12, 0x34, 0x01]);
            pamoja_gateway_packet_free(ack);
            pamoja_gateway_packet_free(read);
        }
    }

    #[test]
    fn a_pull_resp_and_its_tx_ack_cross() {
        unsafe {
            let request = PamojaGatewayTxpk {
                frequency_hz: 869_525_000,
                link: pamoja_lora_link_default(9, 125_000),
                power_dbm: 27,
                timestamp_us: 3_512_348_611,
                has_timestamp: true,
                invert_polarity: true,
                without_crc: true,
                ..PamojaGatewayTxpk::default()
            };
            let payload = b"downlink";
            let mut packet = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_pull_resp(
                    0x00AB,
                    request,
                    payload.as_ptr(),
                    payload.len(),
                    &mut packet
                ),
                PamojaStatus::Ok
            );
            let datagram = bytes_of(packet);
            pamoja_gateway_packet_free(packet);

            let mut read = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_packet_parse(datagram.as_ptr(), datagram.len(), &mut read),
                PamojaStatus::Ok
            );
            let mut back = PamojaGatewayTxpk::default();
            assert!(pamoja_gateway_packet_txpk(read, &mut back));
            assert_eq!(back, request);
            let len = pamoja_gateway_packet_txpk_payload_len(read);
            assert_eq!(
                std::slice::from_raw_parts(pamoja_gateway_packet_txpk_payload(read), len),
                payload
            );
            assert!(!pamoja_gateway_packet_acknowledgment(
                read,
                &mut ptr::null_mut()
            ));
            pamoja_gateway_packet_free(read);

            let mut ack = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_tx_ack(
                    0x00AB,
                    GATEWAY.as_ptr(),
                    PAMOJA_GATEWAY_TX_COLLISION_PACKET,
                    &mut ack
                ),
                PamojaStatus::Ok
            );
            let datagram = bytes_of(ack);
            pamoja_gateway_packet_free(ack);

            let mut read = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_packet_parse(datagram.as_ptr(), datagram.len(), &mut read),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_gateway_packet_tx_status(read),
                PAMOJA_GATEWAY_TX_COLLISION_PACKET
            );
            pamoja_gateway_packet_free(read);
        }
    }

    #[test]
    fn the_acknowledgments_are_four_bytes() {
        unsafe {
            let push = pamoja_gateway_push_ack(0x0102);
            let pull = pamoja_gateway_pull_ack(0x0304);
            assert_eq!(bytes_of(push), [2, 0x01, 0x02, 0x01]);
            assert_eq!(bytes_of(pull), [2, 0x03, 0x04, 0x04]);
            pamoja_gateway_packet_free(push);
            pamoja_gateway_packet_free(pull);

            let mut pull_data = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_pull_data(0x0506, GATEWAY.as_ptr(), &mut pull_data),
                PamojaStatus::Ok
            );
            assert_eq!(bytes_of(pull_data).len(), 12);
            pamoja_gateway_packet_free(pull_data);
        }
    }

    #[test]
    fn a_null_argument_or_a_bad_datagram_is_refused() {
        unsafe {
            assert_eq!(
                pamoja_gateway_uplink_add_rxpk(
                    ptr::null_mut(),
                    PamojaGatewayRxpk::default(),
                    ptr::null(),
                    0
                ),
                PamojaStatus::InvalidArgument
            );
            let mut packet = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_pull_data(1, ptr::null(), &mut packet),
                PamojaStatus::InvalidArgument
            );
            assert!(packet.is_null());
            assert_eq!(
                pamoja_gateway_tx_ack(1, GATEWAY.as_ptr(), 9, &mut packet),
                PamojaStatus::InvalidArgument
            );
            let datagram = [1u8, 0, 1, 0];
            assert_eq!(
                pamoja_gateway_packet_parse(datagram.as_ptr(), datagram.len(), &mut packet),
                PamojaStatus::Codec
            );
            assert_eq!(pamoja_gateway_packet_kind(ptr::null()), u8::MAX);
            assert_eq!(pamoja_gateway_packet_tx_status(ptr::null()), u8::MAX);
            pamoja_gateway_packet_free(ptr::null_mut());
            pamoja_gateway_uplink_free(ptr::null_mut());
        }
    }
}
