//! The C ABI for the LoRa Basics Station protocol.
//!
//! These functions wrap [`pamoja_gateway::station`] for callers that reach the SDK through the
//! flat C boundary. A station and its network server exchange JSON messages over a websocket
//! the caller owns, so nothing here opens a socket: a message crosses as an opaque handle,
//! built from a frame the radio heard, read from text that arrived, or built from its fields,
//! and written back out as the text to send.
//!
//! A handle is used rather than one flat struct because the messages differ too much to share
//! one shape: a configuration carries a table of data rates and a schedule carries a list of
//! frames, while a join request carries eight fixed fields. [`PamojaGatewayStationFields`]
//! holds every fixed field of every kind, and the calls below add and read the text, the bytes
//! and the lists a kind carries besides.
//!
//! To build a message, fill the fields of its kind, call
//! [`pamoja_gateway_station_message_new`], then add its text, bytes and entries. To read one,
//! take its fields with [`pamoja_gateway_station_message_fields`], then read whichever text,
//! bytes and entries the counts there say it carries.

use std::os::raw::c_char;
use std::ptr;

use pamoja_gateway::station::{eui_of, id6, Broadcast, Discovery, Levels, Message, Router};
use pamoja_gateway::udp::Eui;

use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// A join request the station heard.
pub const PAMOJA_GATEWAY_STATION_JOIN_REQUEST: u8 = 0;

/// A data frame the station heard.
pub const PAMOJA_GATEWAY_STATION_UPLINK: u8 = 1;

/// A frame of a kind this protocol does not describe, carried whole.
pub const PAMOJA_GATEWAY_STATION_PROPRIETARY: u8 = 2;

/// What the station reports about itself when a session opens.
pub const PAMOJA_GATEWAY_STATION_VERSION: u8 = 3;

/// How the server tells the station to configure its radios.
pub const PAMOJA_GATEWAY_STATION_ROUTER_CONFIG: u8 = 4;

/// A frame the server asks the station to transmit.
pub const PAMOJA_GATEWAY_STATION_DOWNLINK: u8 = 5;

/// Frames the server asks the station to transmit to a group.
pub const PAMOJA_GATEWAY_STATION_SCHEDULE: u8 = 6;

/// What became of a frame the station was asked to transmit.
pub const PAMOJA_GATEWAY_STATION_TRANSMITTED: u8 = 7;

/// The clock the two keep between them.
pub const PAMOJA_GATEWAY_STATION_TIME_SYNC: u8 = 8;

/// A kind this build does not model, readable only as its text.
pub const PAMOJA_GATEWAY_STATION_OTHER: u8 = 9;

/// The protocol version a station reports.
pub const PAMOJA_GATEWAY_STATION_PROTOCOL_VERSION: u32 = 2;

// The header carries this as a literal, because cbindgen drops a constant whose value
// names another crate's. This holds it to what that crate says.
const _: () =
    assert!(PAMOJA_GATEWAY_STATION_PROTOCOL_VERSION == pamoja_gateway::station::PROTOCOL_VERSION);

/// The station software a version reports.
pub const PAMOJA_GATEWAY_STATION_TEXT_STATION: u8 = 0;

/// The firmware a version reports.
pub const PAMOJA_GATEWAY_STATION_TEXT_FIRMWARE: u8 = 1;

/// The package a version reports.
pub const PAMOJA_GATEWAY_STATION_TEXT_PACKAGE: u8 = 2;

/// The hardware model a version reports.
pub const PAMOJA_GATEWAY_STATION_TEXT_MODEL: u8 = 3;

/// What a version says the station can do, such as `gps`.
pub const PAMOJA_GATEWAY_STATION_TEXT_FEATURES: u8 = 4;

/// The region a configuration names, such as `EU868`.
pub const PAMOJA_GATEWAY_STATION_TEXT_REGION: u8 = 5;

/// The concentrator a configuration is written for, such as `sx1301/1`.
pub const PAMOJA_GATEWAY_STATION_TEXT_HWSPEC: u8 = 6;

/// The word a message calls itself on the wire, which is what names a kind this build does
/// not model.
pub const PAMOJA_GATEWAY_STATION_TEXT_MSGTYPE: u8 = 7;

/// A message either side of a session sends.
pub struct PamojaGatewayStationMessage {
    message: Message,
}

/// How a station heard a packet, as it reports it.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaGatewayStationLevels {
    /// The radio the packet arrived on, which an answer goes back out on.
    pub rctx: i64,
    /// The station clock, in microseconds.
    pub xtime: i64,
    /// The GPS time, when `has_gpstime`.
    pub gpstime: i64,
    /// Whether the station has a GPS time.
    pub has_gpstime: bool,
    /// The received signal strength, in dBm.
    pub rssi: f64,
    /// The signal-to-noise ratio, in dB.
    pub snr: f64,
}

/// A receive window, or the ping slot of a class B downlink: a data rate and a frequency.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaGatewayStationWindow {
    /// The data rate.
    pub data_rate: u8,
    /// The frequency in hertz.
    pub frequency_hz: u32,
    /// Whether the message names this window at all.
    pub present: bool,
}

/// One number of a configuration's data-rate table.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaGatewayStationDataRate {
    /// The spreading factor, 0 for FSK.
    pub spreading_factor: u8,
    /// The bandwidth in hertz.
    pub bandwidth_hz: u32,
    /// Whether the rate is used only for downlinks.
    pub downlink_only: bool,
    /// Whether the table defines this number at all. An undefined one keeps its place so the
    /// numbers after it keep theirs.
    pub defined: bool,
}

/// A range of join identifiers whose join requests a station forwards, both ends included.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaGatewayStationJoinRange {
    /// The first identifier, read as a big-endian number.
    pub first: u64,
    /// The last identifier, read the same way.
    pub last: u64,
}

/// One frame of a schedule, apart from its bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaGatewayStationBroadcast {
    /// The data rate to transmit at.
    pub data_rate: u8,
    /// The frequency to transmit on, in hertz.
    pub frequency_hz: u32,
    /// How urgent it is.
    pub priority: u8,
    /// When to transmit it, in microseconds since the GPS epoch, when `has_gpstime`.
    pub gpstime: i64,
    /// Whether it names a GPS time.
    pub has_gpstime: bool,
    /// The radio to transmit on, when `has_rctx`.
    pub rctx: i64,
    /// Whether it names a radio.
    pub has_rctx: bool,
}

/// The identities a discovery answer names.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaGatewayStationRouterIds {
    /// The station, as the server read it, when `has_router`.
    pub router: [u8; 8],
    /// Whether the answer names the station.
    pub has_router: bool,
    /// The server endpoint carrying the session, when `has_muxs`.
    pub muxs: [u8; 8],
    /// Whether the answer names the endpoint.
    pub has_muxs: bool,
}

/// Every fixed field a message carries, for any kind.
///
/// A kind uses the fields its protocol message names and leaves the rest at zero. The text,
/// the bytes and the lists a kind carries are read and added with the calls that take a
/// message handle.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaGatewayStationFields {
    /// Which kind this is, one of the `PAMOJA_GATEWAY_STATION_*` kind constants.
    pub kind: u8,
    /// The MAC header byte, for a join request or a data frame.
    pub mhdr: u8,
    /// The application being joined, for a join request.
    pub join_eui: [u8; 8],
    /// The device, for a join request, a downlink, or a transmission report.
    pub dev_eui: [u8; 8],
    /// The nonce a join request used.
    pub dev_nonce: u16,
    /// The address a data frame came from.
    pub dev_addr: i32,
    /// The frame control byte.
    pub fctrl: u8,
    /// The frame counter, as the sixteen bits on the air.
    pub fcnt: u16,
    /// The port a data frame was sent on, when `has_fport`.
    pub fport: u8,
    /// Whether the frame carried a port at all.
    pub has_fport: bool,
    /// The message integrity code.
    pub mic: i32,
    /// The data rate a frame arrived at.
    pub data_rate: u8,
    /// The frequency a frame arrived on, in hertz.
    pub frequency_hz: u32,
    /// How it was heard, for the kinds a station sends up.
    pub levels: PamojaGatewayStationLevels,
    /// Which class of downlink this is: 0 for A, 1 for B, 2 for C.
    pub class: u8,
    /// The identifier a downlink and its transmission report share.
    pub diid: i64,
    /// The delay before the first receive window, in seconds, when `has_rx_delay`.
    pub rx_delay: u8,
    /// Whether a downlink named a receive delay.
    pub has_rx_delay: bool,
    /// How urgent a downlink is.
    pub priority: u8,
    /// The protocol version a version message reports.
    pub protocol: u32,
    /// The highest radiated power a configuration allows, in dBm.
    pub max_eirp: f64,
    /// The lowest frequency a configuration allows, in hertz.
    pub freq_min_hz: u32,
    /// The highest frequency a configuration allows, in hertz.
    pub freq_max_hz: u32,
    /// Whether a configuration names the networks whose data frames are forwarded. When it
    /// does not, every network's are, and the list is written as `null`.
    pub filters_networks: bool,
    /// How many networks a configuration names.
    pub net_id_count: usize,
    /// How many join identifier ranges a configuration names.
    pub join_range_count: usize,
    /// How many numbers a configuration's data-rate table holds.
    pub data_rate_count: usize,
    /// How many frames a schedule carries.
    pub broadcast_count: usize,
    /// The first receive window a downlink names.
    pub rx1: PamojaGatewayStationWindow,
    /// The second receive window a downlink names.
    pub rx2: PamojaGatewayStationWindow,
    /// The ping slot a class B downlink goes out in.
    pub ping_slot: PamojaGatewayStationWindow,
    /// The station clock in microseconds, when `has_xtime`: the uplink a downlink answers,
    /// the moment a reported frame went out, or the one a time sync carries.
    pub xtime: i64,
    /// Whether the message carries a station clock.
    pub has_xtime: bool,
    /// The radio a downlink goes out on or a report says it went out on, when `has_rctx`.
    pub rctx: i64,
    /// Whether the message names a radio.
    pub has_rctx: bool,
    /// The GPS time in microseconds since the GPS epoch, when `has_gpstime`.
    pub gpstime: i64,
    /// Whether the message carries a GPS time.
    pub has_gpstime: bool,
    /// When a reported frame went out, in seconds, or the station time a time sync carries,
    /// in microseconds, when `has_txtime`.
    pub txtime: f64,
    /// Whether the message carries that time.
    pub has_txtime: bool,
}

/// Reads a frame the radio heard into the message that reports it.
///
/// # Arguments
///
/// * `frame` - the bytes as they arrived, header through integrity code.
/// * `frame_len` - how many bytes `frame` holds.
/// * `data_rate` - the data rate it arrived at.
/// * `frequency_hz` - the frequency it arrived on, in hertz.
/// * `levels` - how it was heard.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_gateway_station_message_free`], or null on
/// failure with the reason available from
/// [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `frame` must point to `frame_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_heard(
    frame: *const u8,
    frame_len: usize,
    data_rate: u8,
    frequency_hz: u32,
    levels: PamojaGatewayStationLevels,
) -> *mut PamojaGatewayStationMessage {
    let Ok(frame) = read_bytes(frame, frame_len) else {
        return ptr::null_mut();
    };
    match Message::heard(&frame, data_rate, frequency_hz, levels_of(levels)) {
        Ok(message) => held(message),
        Err(error) => {
            set_last_error(error.to_string());
            ptr::null_mut()
        }
    }
}

/// Reads a message that arrived over the websocket.
///
/// # Arguments
///
/// * `text` - the message text.
/// * `text_len` - how many bytes `text` holds.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_gateway_station_message_free`], or null on
/// failure.
///
/// # Safety
///
/// `text` must point to `text_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_parse(
    text: *const u8,
    text_len: usize,
) -> *mut PamojaGatewayStationMessage {
    let Ok(text) = read_bytes(text, text_len) else {
        return ptr::null_mut();
    };
    match Message::from_json(&text) {
        Ok(message) => held(message),
        Err(error) => {
            set_last_error(error.to_string());
            ptr::null_mut()
        }
    }
}

/// Builds a message of any kind from its fixed fields.
///
/// The message starts with no text, no bytes, and empty lists; add those with the
/// `pamoja_gateway_station_message_set_*` and `pamoja_gateway_station_message_add_*` calls.
/// The counts in `fields` are not read.
///
/// # Arguments
///
/// * `fields` - the fields, whose `kind` names the message.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_gateway_station_message_free`], or null
/// when `fields` is null or names no kind.
///
/// # Safety
///
/// `fields` must point to a readable [`PamojaGatewayStationFields`] or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_new(
    fields: *const PamojaGatewayStationFields,
) -> *mut PamojaGatewayStationMessage {
    let Some(fields) = fields.as_ref() else {
        set_last_error("fields must not be null".to_owned());
        return ptr::null_mut();
    };
    match message_from(fields) {
        Ok(message) => held(message),
        Err(why) => {
            set_last_error(why);
            ptr::null_mut()
        }
    }
}

/// Writes a message as the websocket carries it.
///
/// # Arguments
///
/// * `message` - the message.
/// * `out_text` - receives the text, which the caller releases with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written.
///
/// # Safety
///
/// `message` must be a live handle, and `out_text` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_json(
    message: *const PamojaGatewayStationMessage,
    out_text: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if message.is_null() || out_text.is_null() {
        set_last_error("message and out_text must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_text = PamojaBuffer::into_raw((*message).message.to_json().into_bytes());
    PamojaStatus::Ok
}

/// Returns which kind a message is.
///
/// # Arguments
///
/// * `message` - the message.
///
/// # Returns
///
/// One of the `PAMOJA_GATEWAY_STATION_*` kind constants, or
/// [`PAMOJA_GATEWAY_STATION_OTHER`] when `message` is null.
///
/// # Safety
///
/// `message` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_kind(
    message: *const PamojaGatewayStationMessage,
) -> u8 {
    message
        .as_ref()
        .map_or(PAMOJA_GATEWAY_STATION_OTHER, |held| kind_of(&held.message))
}

/// Reads the fixed fields a message carries, and how many entries each of its lists holds.
///
/// # Arguments
///
/// * `message` - the message.
/// * `out_fields` - receives the fields.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read.
///
/// # Safety
///
/// `message` must be a live handle, and `out_fields` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_fields(
    message: *const PamojaGatewayStationMessage,
    out_fields: *mut PamojaGatewayStationFields,
) -> PamojaStatus {
    if message.is_null() || out_fields.is_null() {
        set_last_error("message and out_fields must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_fields = fields_of(&(*message).message);
    PamojaStatus::Ok
}

/// Returns the bytes a message carries: the payload of a frame, or the frame to transmit.
///
/// # Arguments
///
/// * `message` - the message.
/// * `out_payload` - receives the bytes, which the caller releases with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written, with an empty buffer for a kind carrying none.
///
/// # Safety
///
/// `message` must be a live handle, and `out_payload` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_payload(
    message: *const PamojaGatewayStationMessage,
    out_payload: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if message.is_null() || out_payload.is_null() {
        set_last_error("message and out_payload must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match &(*message).message {
        Message::Uplink { payload, .. } | Message::Proprietary { payload, .. } => payload.clone(),
        Message::Downlink { pdu, .. } => pdu.clone(),
        _ => Vec::new(),
    };
    *out_payload = PamojaBuffer::into_raw(bytes);
    PamojaStatus::Ok
}

/// Returns the frame options a data frame carries.
///
/// # Arguments
///
/// * `message` - the message.
/// * `out_options` - receives the bytes, which the caller releases with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written, with an empty buffer for a kind carrying none.
///
/// # Safety
///
/// `message` must be a live handle, and `out_options` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_options(
    message: *const PamojaGatewayStationMessage,
    out_options: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if message.is_null() || out_options.is_null() {
        set_last_error("message and out_options must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match &(*message).message {
        Message::Uplink { fopts, .. } => fopts.clone(),
        _ => Vec::new(),
    };
    *out_options = PamojaBuffer::into_raw(bytes);
    PamojaStatus::Ok
}

/// Returns a piece of text a message carries.
///
/// # Arguments
///
/// * `message` - the message.
/// * `field` - which text, one of the `PAMOJA_GATEWAY_STATION_TEXT_*` constants.
/// * `out_text` - receives the text, which the caller releases with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written, with empty text for a field the kind does not carry, or
/// [`PamojaStatus::InvalidArgument`] when `field` names no text.
///
/// # Safety
///
/// `message` must be a live handle, and `out_text` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_text(
    message: *const PamojaGatewayStationMessage,
    field: u8,
    out_text: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if message.is_null() || out_text.is_null() {
        set_last_error("message and out_text must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if field > PAMOJA_GATEWAY_STATION_TEXT_MSGTYPE {
        set_last_error(format!("{field} names no text a message carries"));
        return PamojaStatus::InvalidArgument;
    }
    let text = text_of(&(*message).message, field);
    *out_text = PamojaBuffer::into_raw(text.as_bytes().to_vec());
    PamojaStatus::Ok
}

/// Sets a piece of text a message carries.
///
/// # Arguments
///
/// * `message` - the message.
/// * `field` - which text, one of the `PAMOJA_GATEWAY_STATION_TEXT_*` constants.
/// * `text` - the text, UTF-8.
/// * `text_len` - how many bytes `text` holds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once set, or [`PamojaStatus::InvalidArgument`] when the kind carries
/// no such text or the text is not UTF-8.
///
/// # Safety
///
/// `message` must be a live handle, and `text` must point to `text_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_set_text(
    message: *mut PamojaGatewayStationMessage,
    field: u8,
    text: *const u8,
    text_len: usize,
) -> PamojaStatus {
    let Some(held) = message.as_mut() else {
        set_last_error("message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Ok(bytes) = read_bytes(text, text_len) else {
        return PamojaStatus::InvalidArgument;
    };
    let Ok(text) = String::from_utf8(bytes) else {
        set_last_error("the text is not valid UTF-8".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let kind = held.message.msgtype().to_owned();
    match text_slot(&mut held.message, field) {
        Some(slot) => {
            *slot = text;
            PamojaStatus::Ok
        }
        None => {
            set_last_error(format!("a {kind} message carries no text field {field}"));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Sets the bytes a message carries: the payload of a data frame or a proprietary frame, or
/// the frame a downlink transmits.
///
/// # Arguments
///
/// * `message` - the message.
/// * `bytes` - the bytes.
/// * `bytes_len` - how many bytes `bytes` holds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once set, or [`PamojaStatus::InvalidArgument`] for a kind that
/// carries none.
///
/// # Safety
///
/// `message` must be a live handle, and `bytes` must point to `bytes_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_set_payload(
    message: *mut PamojaGatewayStationMessage,
    bytes: *const u8,
    bytes_len: usize,
) -> PamojaStatus {
    let Some(held) = message.as_mut() else {
        set_last_error("message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Ok(bytes) = read_bytes(bytes, bytes_len) else {
        return PamojaStatus::InvalidArgument;
    };
    match &mut held.message {
        Message::Uplink { payload, .. } | Message::Proprietary { payload, .. } => *payload = bytes,
        Message::Downlink { pdu, .. } => *pdu = bytes,
        other => return refused(other, "no payload"),
    }
    PamojaStatus::Ok
}

/// Sets the frame options a data frame carries.
///
/// # Arguments
///
/// * `message` - the message.
/// * `bytes` - the options.
/// * `bytes_len` - how many bytes `bytes` holds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once set, or [`PamojaStatus::InvalidArgument`] for a kind that
/// carries none.
///
/// # Safety
///
/// `message` must be a live handle, and `bytes` must point to `bytes_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_set_options(
    message: *mut PamojaGatewayStationMessage,
    bytes: *const u8,
    bytes_len: usize,
) -> PamojaStatus {
    let Some(held) = message.as_mut() else {
        set_last_error("message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Ok(bytes) = read_bytes(bytes, bytes_len) else {
        return PamojaStatus::InvalidArgument;
    };
    match &mut held.message {
        Message::Uplink { fopts, .. } => *fopts = bytes,
        other => return refused(other, "no frame options"),
    }
    PamojaStatus::Ok
}

/// Adds a network whose data frames a configuration forwards.
///
/// A configuration that names one network forwards no other, so the first call turns the
/// filter on.
///
/// # Arguments
///
/// * `message` - the configuration.
/// * `net_id` - the network identifier.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once added, or [`PamojaStatus::InvalidArgument`] for a message that
/// is not a configuration.
///
/// # Safety
///
/// `message` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_add_net_id(
    message: *mut PamojaGatewayStationMessage,
    net_id: u32,
) -> PamojaStatus {
    let Some(held) = message.as_mut() else {
        set_last_error("message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match &mut held.message {
        Message::RouterConfig { net_id: ids, .. } => {
            ids.get_or_insert_with(Vec::new).push(net_id);
            PamojaStatus::Ok
        }
        other => refused(other, "no network list"),
    }
}

/// Adds a range of join identifiers whose join requests a configuration forwards.
///
/// # Arguments
///
/// * `message` - the configuration.
/// * `range` - the range, both ends included.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once added, or [`PamojaStatus::InvalidArgument`] for a message that
/// is not a configuration.
///
/// # Safety
///
/// `message` must be a live handle, and `range` must point to a readable range.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_add_join_range(
    message: *mut PamojaGatewayStationMessage,
    range: *const PamojaGatewayStationJoinRange,
) -> PamojaStatus {
    let (Some(held), Some(range)) = (message.as_mut(), range.as_ref()) else {
        set_last_error("message and range must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match &mut held.message {
        Message::RouterConfig { join_eui, .. } => {
            join_eui.push((range.first, range.last));
            PamojaStatus::Ok
        }
        other => refused(other, "no join identifier ranges"),
    }
}

/// Adds the next number of a configuration's data-rate table.
///
/// # Arguments
///
/// * `message` - the configuration.
/// * `rate` - the data rate, whose `defined` false keeps the number without one.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once added, or [`PamojaStatus::InvalidArgument`] for a message that
/// is not a configuration.
///
/// # Safety
///
/// `message` must be a live handle, and `rate` must point to a readable data rate.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_add_data_rate(
    message: *mut PamojaGatewayStationMessage,
    rate: *const PamojaGatewayStationDataRate,
) -> PamojaStatus {
    let (Some(held), Some(rate)) = (message.as_mut(), rate.as_ref()) else {
        set_last_error("message and rate must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match &mut held.message {
        Message::RouterConfig { data_rates, .. } => {
            data_rates.push(rate.defined.then_some((
                rate.spreading_factor,
                rate.bandwidth_hz,
                rate.downlink_only,
            )));
            PamojaStatus::Ok
        }
        other => refused(other, "no data-rate table"),
    }
}

/// Adds a frame to a schedule.
///
/// # Arguments
///
/// * `message` - the schedule.
/// * `frame` - when and how to transmit it.
/// * `pdu` - the frame's bytes.
/// * `pdu_len` - how many bytes `pdu` holds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once added, or [`PamojaStatus::InvalidArgument`] for a message that
/// is not a schedule.
///
/// # Safety
///
/// `message` must be a live handle, `frame` must point to a readable frame, and `pdu` to
/// `pdu_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_add_broadcast(
    message: *mut PamojaGatewayStationMessage,
    frame: *const PamojaGatewayStationBroadcast,
    pdu: *const u8,
    pdu_len: usize,
) -> PamojaStatus {
    let (Some(held), Some(frame)) = (message.as_mut(), frame.as_ref()) else {
        set_last_error("message and frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Ok(pdu) = read_bytes(pdu, pdu_len) else {
        return PamojaStatus::InvalidArgument;
    };
    match &mut held.message {
        Message::Schedule { frames } => {
            frames.push(Broadcast {
                pdu,
                data_rate: frame.data_rate,
                frequency_hz: frame.frequency_hz,
                priority: frame.priority,
                gpstime: frame.has_gpstime.then_some(frame.gpstime),
                rctx: frame.has_rctx.then_some(frame.rctx),
            });
            PamojaStatus::Ok
        }
        other => refused(other, "no frames"),
    }
}

/// Reads one network a configuration names.
///
/// # Arguments
///
/// * `message` - the configuration.
/// * `index` - which one, below the `net_id_count` its fields report.
/// * `out_net_id` - receives the network identifier.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::InvalidArgument`] for an index past the
/// list or a message that is not a configuration.
///
/// # Safety
///
/// `message` must be a live handle, and `out_net_id` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_net_id(
    message: *const PamojaGatewayStationMessage,
    index: usize,
    out_net_id: *mut u32,
) -> PamojaStatus {
    if message.is_null() || out_net_id.is_null() {
        set_last_error("message and out_net_id must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let found = match &(*message).message {
        Message::RouterConfig { net_id, .. } => {
            net_id.as_deref().and_then(|ids| ids.get(index)).copied()
        }
        _ => None,
    };
    match found {
        Some(id) => {
            *out_net_id = id;
            PamojaStatus::Ok
        }
        None => missing(index, "network"),
    }
}

/// Reads one join identifier range a configuration names.
///
/// # Arguments
///
/// * `message` - the configuration.
/// * `index` - which one, below the `join_range_count` its fields report.
/// * `out_range` - receives the range.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::InvalidArgument`] for an index past the
/// list or a message that is not a configuration.
///
/// # Safety
///
/// `message` must be a live handle, and `out_range` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_join_range(
    message: *const PamojaGatewayStationMessage,
    index: usize,
    out_range: *mut PamojaGatewayStationJoinRange,
) -> PamojaStatus {
    if message.is_null() || out_range.is_null() {
        set_last_error("message and out_range must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let found = match &(*message).message {
        Message::RouterConfig { join_eui, .. } => join_eui.get(index).copied(),
        _ => None,
    };
    match found {
        Some((first, last)) => {
            *out_range = PamojaGatewayStationJoinRange { first, last };
            PamojaStatus::Ok
        }
        None => missing(index, "join identifier range"),
    }
}

/// Reads one number of a configuration's data-rate table.
///
/// # Arguments
///
/// * `message` - the configuration.
/// * `index` - the data-rate number, below the `data_rate_count` its fields report.
/// * `out_rate` - receives the data rate, with `defined` false for a number the table leaves
///   undefined.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::InvalidArgument`] for an index past the
/// table or a message that is not a configuration.
///
/// # Safety
///
/// `message` must be a live handle, and `out_rate` must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_data_rate(
    message: *const PamojaGatewayStationMessage,
    index: usize,
    out_rate: *mut PamojaGatewayStationDataRate,
) -> PamojaStatus {
    if message.is_null() || out_rate.is_null() {
        set_last_error("message and out_rate must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let found = match &(*message).message {
        Message::RouterConfig { data_rates, .. } => data_rates.get(index).copied(),
        _ => None,
    };
    match found {
        Some(entry) => {
            let (spreading_factor, bandwidth_hz, downlink_only) = entry.unwrap_or_default();
            *out_rate = PamojaGatewayStationDataRate {
                spreading_factor,
                bandwidth_hz,
                downlink_only,
                defined: entry.is_some(),
            };
            PamojaStatus::Ok
        }
        None => missing(index, "data rate"),
    }
}

/// Reads one frame of a schedule.
///
/// # Arguments
///
/// * `message` - the schedule.
/// * `index` - which frame, below the `broadcast_count` its fields report.
/// * `out_frame` - receives when and how to transmit it.
/// * `out_pdu` - receives its bytes, which the caller releases with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::InvalidArgument`] for an index past the
/// schedule or a message that is not one.
///
/// # Safety
///
/// `message` must be a live handle, and both out pointers must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_broadcast(
    message: *const PamojaGatewayStationMessage,
    index: usize,
    out_frame: *mut PamojaGatewayStationBroadcast,
    out_pdu: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if message.is_null() || out_frame.is_null() || out_pdu.is_null() {
        set_last_error("message, out_frame and out_pdu must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let found = match &(*message).message {
        Message::Schedule { frames } => frames.get(index),
        _ => None,
    };
    match found {
        Some(frame) => {
            *out_frame = PamojaGatewayStationBroadcast {
                data_rate: frame.data_rate,
                frequency_hz: frame.frequency_hz,
                priority: frame.priority,
                gpstime: frame.gpstime.unwrap_or_default(),
                has_gpstime: frame.gpstime.is_some(),
                rctx: frame.rctx.unwrap_or_default(),
                has_rctx: frame.rctx.is_some(),
            };
            *out_pdu = PamojaBuffer::into_raw(frame.pdu.clone());
            PamojaStatus::Ok
        }
        None => missing(index, "scheduled frame"),
    }
}

/// Releases a message.
///
/// # Arguments
///
/// * `message` - the message, or null.
///
/// # Safety
///
/// `message` must be a live handle from a call that produced one, or null, and must not be
/// used again afterwards.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_message_free(
    message: *mut PamojaGatewayStationMessage,
) {
    if !message.is_null() {
        drop(Box::from_raw(message));
    }
}

/// Writes the request a station sends to find its network server.
///
/// # Arguments
///
/// * `router` - the station asking, eight bytes.
/// * `out_text` - receives the text, which the caller releases with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written.
///
/// # Safety
///
/// `router` must point to eight readable bytes, and `out_text` to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_discovery(
    router: *const u8,
    out_text: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let (Some(router), false) = (eight(router), out_text.is_null()) else {
        set_last_error("router and out_text must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let asking = Discovery::new(Eui::new(router));
    *out_text = PamojaBuffer::into_raw(asking.to_json().into_bytes());
    PamojaStatus::Ok
}

/// Reads the request a station sent to find its network server, as the server does.
///
/// # Arguments
///
/// * `text` - the request text.
/// * `text_len` - how many bytes `text` holds.
/// * `out_router` - receives the station asking, eight bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::Codec`] when the text is not this
/// request.
///
/// # Safety
///
/// `text` must point to `text_len` readable bytes, and `out_router` to eight writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_discovery_parse(
    text: *const u8,
    text_len: usize,
    out_router: *mut u8,
) -> PamojaStatus {
    if out_router.is_null() {
        set_last_error("out_router must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Ok(text) = read_bytes(text, text_len) else {
        return PamojaStatus::InvalidArgument;
    };
    match Discovery::from_json(&text) {
        Ok(asked) => {
            ptr::copy_nonoverlapping(asked.router.bytes().as_ptr(), out_router, 8);
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Codec
        }
    }
}

/// Writes the answer that sends a station to the websocket its session runs on.
///
/// # Arguments
///
/// * `router` - the station being answered, eight bytes.
/// * `muxs` - the server endpoint that carries the session, eight bytes.
/// * `uri` - the websocket address, UTF-8.
/// * `uri_len` - how many bytes `uri` holds.
/// * `out_text` - receives the answer, which the caller releases with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written.
///
/// # Safety
///
/// `router` and `muxs` must point to eight readable bytes each, `uri` to `uri_len` readable
/// bytes, and `out_text` to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_router_accepted(
    router: *const u8,
    muxs: *const u8,
    uri: *const u8,
    uri_len: usize,
    out_text: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let (Some(router), Some(muxs), false) = (eight(router), eight(muxs), out_text.is_null()) else {
        set_last_error("router, muxs and out_text must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(uri) = utf8(uri, uri_len) else {
        return PamojaStatus::InvalidArgument;
    };
    let answer = Router::accepted(Eui::new(router), Eui::new(muxs), uri);
    *out_text = PamojaBuffer::into_raw(answer.to_json().into_bytes());
    PamojaStatus::Ok
}

/// Writes the answer that refuses a station, saying why.
///
/// # Arguments
///
/// * `router` - the station being refused, eight bytes.
/// * `error` - what is wrong, in words the operator can act on, UTF-8.
/// * `error_len` - how many bytes `error` holds.
/// * `out_text` - receives the answer, which the caller releases with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written.
///
/// # Safety
///
/// `router` must point to eight readable bytes, `error` to `error_len` readable bytes, and
/// `out_text` to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_router_refused(
    router: *const u8,
    error: *const u8,
    error_len: usize,
    out_text: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let (Some(router), false) = (eight(router), out_text.is_null()) else {
        set_last_error("router and out_text must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(error) = utf8(error, error_len) else {
        return PamojaStatus::InvalidArgument;
    };
    let answer = Router::refused(Eui::new(router), error);
    *out_text = PamojaBuffer::into_raw(answer.to_json().into_bytes());
    PamojaStatus::Ok
}

/// Reads the answer a discovery endpoint gives.
///
/// # Arguments
///
/// * `text` - the answer text.
/// * `text_len` - how many bytes `text` holds.
/// * `out_uri` - receives the websocket address to open, empty when the station was refused.
/// * `out_error` - receives why the station was refused, empty when it was not.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read. Both buffers are released by the caller with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Safety
///
/// `text` must point to `text_len` readable bytes, and both out pointers to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_router_parse(
    text: *const u8,
    text_len: usize,
    out_uri: *mut *mut PamojaBuffer,
    out_error: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if out_uri.is_null() || out_error.is_null() {
        set_last_error("out_uri and out_error must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Ok(text) = read_bytes(text, text_len) else {
        return PamojaStatus::InvalidArgument;
    };
    match Router::from_json(&text) {
        Ok(answer) => {
            *out_uri = PamojaBuffer::into_raw(answer.uri.unwrap_or_default().into_bytes());
            *out_error = PamojaBuffer::into_raw(answer.error.unwrap_or_default().into_bytes());
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Codec
        }
    }
}

/// Reads the identities a discovery answer names: the station, and the server endpoint that
/// carries its session.
///
/// # Arguments
///
/// * `text` - the answer text.
/// * `text_len` - how many bytes `text` holds.
/// * `out_ids` - receives the identities.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::Codec`] when the text is not JSON.
///
/// # Safety
///
/// `text` must point to `text_len` readable bytes, and `out_ids` to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_router_identities(
    text: *const u8,
    text_len: usize,
    out_ids: *mut PamojaGatewayStationRouterIds,
) -> PamojaStatus {
    if out_ids.is_null() {
        set_last_error("out_ids must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Ok(text) = read_bytes(text, text_len) else {
        return PamojaStatus::InvalidArgument;
    };
    match Router::from_json(&text) {
        Ok(answer) => {
            *out_ids = PamojaGatewayStationRouterIds {
                router: answer.router.map(|eui| eui.bytes()).unwrap_or_default(),
                has_router: answer.router.is_some(),
                muxs: answer.muxs.map(|eui| eui.bytes()).unwrap_or_default(),
                has_muxs: answer.muxs.is_some(),
            };
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Codec
        }
    }
}

/// Writes an identifier in the ID6 form the protocol prefers.
///
/// # Arguments
///
/// * `eui` - the identifier, eight bytes.
/// * `out_text` - receives the text, which the caller releases with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once written.
///
/// # Safety
///
/// `eui` must point to eight readable bytes, and `out_text` to writable storage.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_id6(
    eui: *const u8,
    out_text: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let (Some(eui), false) = (eight(eui), out_text.is_null()) else {
        set_last_error("eui and out_text must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_text = PamojaBuffer::into_raw(id6(Eui::new(eui)).into_bytes());
    PamojaStatus::Ok
}

/// Reads an identifier written in any form the protocol accepts.
///
/// # Arguments
///
/// * `text` - the identifier, null-terminated.
/// * `out_eui` - receives the eight bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once read, or [`PamojaStatus::Codec`] when the text is not an
/// identifier.
///
/// # Safety
///
/// `text` must be a valid null-terminated UTF-8 string, and `out_eui` must point to eight
/// writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_station_eui_of(
    text: *const c_char,
    out_eui: *mut u8,
) -> PamojaStatus {
    if text.is_null() || out_eui.is_null() {
        set_last_error("text and out_eui must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Ok(text) = std::ffi::CStr::from_ptr(text).to_str() else {
        set_last_error("the identifier is not valid UTF-8".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match eui_of(text) {
        Some(eui) => {
            ptr::copy_nonoverlapping(eui.bytes().as_ptr(), out_eui, 8);
            PamojaStatus::Ok
        }
        None => {
            set_last_error(format!("{text} is not an identifier"));
            PamojaStatus::Codec
        }
    }
}

/// Hands a message to the caller as a handle.
fn held(message: Message) -> *mut PamojaGatewayStationMessage {
    Box::into_raw(Box::new(PamojaGatewayStationMessage { message }))
}

/// Refuses an addition a kind does not carry, saying which.
fn refused(message: &Message, carries: &str) -> PamojaStatus {
    set_last_error(format!("a {} message carries {carries}", message.msgtype()));
    PamojaStatus::InvalidArgument
}

/// Refuses an index past the list asked for.
fn missing(index: usize, what: &str) -> PamojaStatus {
    set_last_error(format!("the message carries no {what} at index {index}"));
    PamojaStatus::InvalidArgument
}

/// Reads UTF-8 text from the boundary.
unsafe fn utf8(text: *const u8, text_len: usize) -> Option<String> {
    let bytes = read_bytes(text, text_len).ok()?;
    match String::from_utf8(bytes) {
        Ok(text) => Some(text),
        Err(_) => {
            set_last_error("the text is not valid UTF-8".to_owned());
            None
        }
    }
}

/// Reads how a packet was heard from the boundary.
fn levels_of(levels: PamojaGatewayStationLevels) -> Levels {
    Levels {
        rctx: levels.rctx,
        xtime: levels.xtime,
        gpstime: levels.has_gpstime.then_some(levels.gpstime),
        rssi: levels.rssi,
        snr: levels.snr,
    }
}

/// Writes how a packet was heard for the boundary.
fn levels_to_c(levels: Levels) -> PamojaGatewayStationLevels {
    PamojaGatewayStationLevels {
        rctx: levels.rctx,
        xtime: levels.xtime,
        gpstime: levels.gpstime.unwrap_or_default(),
        has_gpstime: levels.gpstime.is_some(),
        rssi: levels.rssi,
        snr: levels.snr,
    }
}

/// Reads a window from the boundary.
fn window_of(window: PamojaGatewayStationWindow) -> Option<(u8, u32)> {
    window
        .present
        .then_some((window.data_rate, window.frequency_hz))
}

/// Writes a window for the boundary.
fn window_to_c(window: Option<(u8, u32)>) -> PamojaGatewayStationWindow {
    let (data_rate, frequency_hz) = window.unwrap_or_default();
    PamojaGatewayStationWindow {
        data_rate,
        frequency_hz,
        present: window.is_some(),
    }
}

/// Names which kind a message is.
fn kind_of(message: &Message) -> u8 {
    match message {
        Message::JoinRequest { .. } => PAMOJA_GATEWAY_STATION_JOIN_REQUEST,
        Message::Uplink { .. } => PAMOJA_GATEWAY_STATION_UPLINK,
        Message::Proprietary { .. } => PAMOJA_GATEWAY_STATION_PROPRIETARY,
        Message::Version { .. } => PAMOJA_GATEWAY_STATION_VERSION,
        Message::RouterConfig { .. } => PAMOJA_GATEWAY_STATION_ROUTER_CONFIG,
        Message::Downlink { .. } => PAMOJA_GATEWAY_STATION_DOWNLINK,
        Message::Schedule { .. } => PAMOJA_GATEWAY_STATION_SCHEDULE,
        Message::Transmitted { .. } => PAMOJA_GATEWAY_STATION_TRANSMITTED,
        Message::TimeSync { .. } => PAMOJA_GATEWAY_STATION_TIME_SYNC,
        Message::Other { .. } => PAMOJA_GATEWAY_STATION_OTHER,
    }
}

/// Builds a message of the kind the fields name, with no text, bytes or entries yet.
fn message_from(fields: &PamojaGatewayStationFields) -> Result<Message, String> {
    let clock = |value: i64, present: bool| present.then_some(value);
    Ok(match fields.kind {
        PAMOJA_GATEWAY_STATION_JOIN_REQUEST => Message::JoinRequest {
            mhdr: fields.mhdr,
            join_eui: Eui::new(fields.join_eui),
            dev_eui: Eui::new(fields.dev_eui),
            dev_nonce: fields.dev_nonce,
            mic: fields.mic,
            data_rate: fields.data_rate,
            frequency_hz: fields.frequency_hz,
            levels: levels_of(fields.levels),
        },
        PAMOJA_GATEWAY_STATION_UPLINK => Message::Uplink {
            mhdr: fields.mhdr,
            dev_addr: fields.dev_addr,
            fctrl: fields.fctrl,
            fcnt: fields.fcnt,
            fopts: Vec::new(),
            fport: fields.has_fport.then_some(fields.fport),
            payload: Vec::new(),
            mic: fields.mic,
            data_rate: fields.data_rate,
            frequency_hz: fields.frequency_hz,
            levels: levels_of(fields.levels),
        },
        PAMOJA_GATEWAY_STATION_PROPRIETARY => Message::Proprietary {
            payload: Vec::new(),
            data_rate: fields.data_rate,
            frequency_hz: fields.frequency_hz,
            levels: levels_of(fields.levels),
        },
        PAMOJA_GATEWAY_STATION_VERSION => Message::Version {
            station: String::new(),
            firmware: String::new(),
            package: String::new(),
            model: String::new(),
            protocol: fields.protocol,
            features: String::new(),
        },
        PAMOJA_GATEWAY_STATION_ROUTER_CONFIG => Message::RouterConfig {
            net_id: fields.filters_networks.then(Vec::new),
            join_eui: Vec::new(),
            region: String::new(),
            max_eirp: fields.max_eirp,
            hwspec: String::new(),
            freq_range: (fields.freq_min_hz, fields.freq_max_hz),
            data_rates: Vec::new(),
        },
        PAMOJA_GATEWAY_STATION_DOWNLINK => Message::Downlink {
            dev_eui: Eui::new(fields.dev_eui),
            class: fields.class,
            diid: fields.diid,
            pdu: Vec::new(),
            rx_delay: fields.has_rx_delay.then_some(fields.rx_delay),
            rx1: window_of(fields.rx1),
            rx2: window_of(fields.rx2),
            ping_slot: window_of(fields.ping_slot),
            priority: fields.priority,
            xtime: clock(fields.xtime, fields.has_xtime),
            rctx: clock(fields.rctx, fields.has_rctx),
            gpstime: clock(fields.gpstime, fields.has_gpstime),
        },
        PAMOJA_GATEWAY_STATION_SCHEDULE => Message::Schedule { frames: Vec::new() },
        PAMOJA_GATEWAY_STATION_TRANSMITTED => Message::Transmitted {
            diid: fields.diid,
            dev_eui: Eui::new(fields.dev_eui),
            rctx: fields.rctx,
            xtime: fields.xtime,
            txtime: fields.txtime,
            gpstime: clock(fields.gpstime, fields.has_gpstime),
        },
        PAMOJA_GATEWAY_STATION_TIME_SYNC => Message::TimeSync {
            txtime: fields.has_txtime.then_some(fields.txtime.round() as i64),
            xtime: clock(fields.xtime, fields.has_xtime),
            gpstime: clock(fields.gpstime, fields.has_gpstime),
        },
        PAMOJA_GATEWAY_STATION_OTHER => Message::Other {
            msgtype: String::new(),
        },
        kind => return Err(format!("{kind} names no message kind")),
    })
}

/// Writes whichever fields the kind in hand carries, leaving the rest at zero.
fn fields_of(message: &Message) -> PamojaGatewayStationFields {
    let mut fields = PamojaGatewayStationFields {
        kind: kind_of(message),
        mhdr: 0,
        join_eui: [0; 8],
        dev_eui: [0; 8],
        dev_nonce: 0,
        dev_addr: 0,
        fctrl: 0,
        fcnt: 0,
        fport: 0,
        has_fport: false,
        mic: 0,
        data_rate: 0,
        frequency_hz: 0,
        levels: levels_to_c(Levels::default()),
        class: 0,
        diid: 0,
        rx_delay: 0,
        has_rx_delay: false,
        priority: 0,
        protocol: 0,
        max_eirp: 0.0,
        freq_min_hz: 0,
        freq_max_hz: 0,
        filters_networks: false,
        net_id_count: 0,
        join_range_count: 0,
        data_rate_count: 0,
        broadcast_count: 0,
        rx1: window_to_c(None),
        rx2: window_to_c(None),
        ping_slot: window_to_c(None),
        xtime: 0,
        has_xtime: false,
        rctx: 0,
        has_rctx: false,
        gpstime: 0,
        has_gpstime: false,
        txtime: 0.0,
        has_txtime: false,
    };

    match message {
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
            fields.mhdr = *mhdr;
            fields.join_eui = join_eui.bytes();
            fields.dev_eui = dev_eui.bytes();
            fields.dev_nonce = *dev_nonce;
            fields.mic = *mic;
            fields.data_rate = *data_rate;
            fields.frequency_hz = *frequency_hz;
            fields.levels = levels_to_c(*levels);
        }
        Message::Uplink {
            mhdr,
            dev_addr,
            fctrl,
            fcnt,
            fport,
            mic,
            data_rate,
            frequency_hz,
            levels,
            ..
        } => {
            fields.mhdr = *mhdr;
            fields.dev_addr = *dev_addr;
            fields.fctrl = *fctrl;
            fields.fcnt = *fcnt;
            fields.fport = fport.unwrap_or_default();
            fields.has_fport = fport.is_some();
            fields.mic = *mic;
            fields.data_rate = *data_rate;
            fields.frequency_hz = *frequency_hz;
            fields.levels = levels_to_c(*levels);
        }
        Message::Proprietary {
            data_rate,
            frequency_hz,
            levels,
            ..
        } => {
            fields.data_rate = *data_rate;
            fields.frequency_hz = *frequency_hz;
            fields.levels = levels_to_c(*levels);
        }
        Message::Version { protocol, .. } => fields.protocol = *protocol,
        Message::RouterConfig {
            net_id,
            join_eui,
            max_eirp,
            freq_range,
            data_rates,
            ..
        } => {
            fields.max_eirp = *max_eirp;
            fields.freq_min_hz = freq_range.0;
            fields.freq_max_hz = freq_range.1;
            fields.filters_networks = net_id.is_some();
            fields.net_id_count = net_id.as_ref().map_or(0, Vec::len);
            fields.join_range_count = join_eui.len();
            fields.data_rate_count = data_rates.len();
        }
        Message::Downlink {
            dev_eui,
            class,
            diid,
            rx_delay,
            rx1,
            rx2,
            ping_slot,
            priority,
            xtime,
            rctx,
            gpstime,
            ..
        } => {
            fields.dev_eui = dev_eui.bytes();
            fields.class = *class;
            fields.diid = *diid;
            fields.rx_delay = rx_delay.unwrap_or_default();
            fields.has_rx_delay = rx_delay.is_some();
            fields.rx1 = window_to_c(*rx1);
            fields.rx2 = window_to_c(*rx2);
            fields.ping_slot = window_to_c(*ping_slot);
            fields.priority = *priority;
            fields.xtime = xtime.unwrap_or_default();
            fields.has_xtime = xtime.is_some();
            fields.rctx = rctx.unwrap_or_default();
            fields.has_rctx = rctx.is_some();
            fields.gpstime = gpstime.unwrap_or_default();
            fields.has_gpstime = gpstime.is_some();
        }
        Message::Schedule { frames } => fields.broadcast_count = frames.len(),
        Message::Transmitted {
            diid,
            dev_eui,
            rctx,
            xtime,
            txtime,
            gpstime,
        } => {
            fields.diid = *diid;
            fields.dev_eui = dev_eui.bytes();
            fields.rctx = *rctx;
            fields.has_rctx = true;
            fields.xtime = *xtime;
            fields.has_xtime = true;
            fields.txtime = *txtime;
            fields.has_txtime = true;
            fields.gpstime = gpstime.unwrap_or_default();
            fields.has_gpstime = gpstime.is_some();
        }
        Message::TimeSync {
            txtime,
            xtime,
            gpstime,
        } => {
            fields.txtime = txtime.unwrap_or_default() as f64;
            fields.has_txtime = txtime.is_some();
            fields.xtime = xtime.unwrap_or_default();
            fields.has_xtime = xtime.is_some();
            fields.gpstime = gpstime.unwrap_or_default();
            fields.has_gpstime = gpstime.is_some();
        }
        Message::Other { .. } => {}
    }

    fields
}

/// Reads a piece of text a message carries, empty for one its kind does not.
fn text_of(message: &Message, field: u8) -> &str {
    match (message, field) {
        (_, PAMOJA_GATEWAY_STATION_TEXT_MSGTYPE) => message.msgtype(),
        (Message::Version { station, .. }, PAMOJA_GATEWAY_STATION_TEXT_STATION) => station,
        (Message::Version { firmware, .. }, PAMOJA_GATEWAY_STATION_TEXT_FIRMWARE) => firmware,
        (Message::Version { package, .. }, PAMOJA_GATEWAY_STATION_TEXT_PACKAGE) => package,
        (Message::Version { model, .. }, PAMOJA_GATEWAY_STATION_TEXT_MODEL) => model,
        (Message::Version { features, .. }, PAMOJA_GATEWAY_STATION_TEXT_FEATURES) => features,
        (Message::RouterConfig { region, .. }, PAMOJA_GATEWAY_STATION_TEXT_REGION) => region,
        (Message::RouterConfig { hwspec, .. }, PAMOJA_GATEWAY_STATION_TEXT_HWSPEC) => hwspec,
        _ => "",
    }
}

/// Finds where a piece of text a kind carries is kept.
fn text_slot(message: &mut Message, field: u8) -> Option<&mut String> {
    match (message, field) {
        (Message::Version { station, .. }, PAMOJA_GATEWAY_STATION_TEXT_STATION) => Some(station),
        (Message::Version { firmware, .. }, PAMOJA_GATEWAY_STATION_TEXT_FIRMWARE) => Some(firmware),
        (Message::Version { package, .. }, PAMOJA_GATEWAY_STATION_TEXT_PACKAGE) => Some(package),
        (Message::Version { model, .. }, PAMOJA_GATEWAY_STATION_TEXT_MODEL) => Some(model),
        (Message::Version { features, .. }, PAMOJA_GATEWAY_STATION_TEXT_FEATURES) => Some(features),
        (Message::RouterConfig { region, .. }, PAMOJA_GATEWAY_STATION_TEXT_REGION) => Some(region),
        (Message::RouterConfig { hwspec, .. }, PAMOJA_GATEWAY_STATION_TEXT_HWSPEC) => Some(hwspec),
        (Message::Other { msgtype }, PAMOJA_GATEWAY_STATION_TEXT_MSGTYPE) => Some(msgtype),
        _ => None,
    }
}

/// Reads an eight-byte identifier from the boundary.
unsafe fn eight(bytes: *const u8) -> Option<[u8; 8]> {
    if bytes.is_null() {
        return None;
    }
    let mut read = [0u8; 8];
    ptr::copy_nonoverlapping(bytes, read.as_mut_ptr(), read.len());
    Some(read)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    /// Takes the bytes out of a buffer the library handed over, and releases it.
    unsafe fn taken(buffer: *mut PamojaBuffer) -> Vec<u8> {
        let bytes =
            std::slice::from_raw_parts(pamoja_buffer_data(buffer), pamoja_buffer_len(buffer))
                .to_vec();
        pamoja_buffer_free(buffer);
        bytes
    }

    /// Writes a message through the boundary, and releases the handle.
    unsafe fn written(message: *mut PamojaGatewayStationMessage) -> String {
        assert!(!message.is_null(), "the message was built");
        let mut text = ptr::null_mut();
        assert_eq!(
            pamoja_gateway_station_message_json(message, &mut text),
            PamojaStatus::Ok
        );
        pamoja_gateway_station_message_free(message);
        String::from_utf8(taken(text)).expect("the text is UTF-8")
    }

    /// The fields of a message of one kind, every other field at zero.
    fn blank(kind: u8) -> PamojaGatewayStationFields {
        PamojaGatewayStationFields {
            kind,
            ..fields_of(&Message::Other {
                msgtype: String::new(),
            })
        }
    }

    #[test]
    fn a_configuration_built_through_the_boundary_matches_the_one_built_in_rust() {
        let expected = Message::RouterConfig {
            net_id: Some(vec![0x13]),
            join_eui: vec![(0x70b3_d57e_d000_0000, 0x70b3_d57e_d000_00ff)],
            region: "US915".to_owned(),
            max_eirp: 30.0,
            hwspec: "sx1301/1".to_owned(),
            freq_range: (902_000_000, 928_000_000),
            data_rates: vec![Some((10, 125_000, false)), None, Some((12, 500_000, true))],
        };

        unsafe {
            let mut fields = blank(PAMOJA_GATEWAY_STATION_ROUTER_CONFIG);
            fields.max_eirp = 30.0;
            fields.freq_min_hz = 902_000_000;
            fields.freq_max_hz = 928_000_000;
            let built = pamoja_gateway_station_message_new(&fields);
            for (field, text) in [
                (PAMOJA_GATEWAY_STATION_TEXT_REGION, "US915"),
                (PAMOJA_GATEWAY_STATION_TEXT_HWSPEC, "sx1301/1"),
            ] {
                assert_eq!(
                    pamoja_gateway_station_message_set_text(
                        built,
                        field,
                        text.as_ptr(),
                        text.len()
                    ),
                    PamojaStatus::Ok
                );
            }
            assert_eq!(
                pamoja_gateway_station_message_add_net_id(built, 0x13),
                PamojaStatus::Ok
            );
            let range = PamojaGatewayStationJoinRange {
                first: 0x70b3_d57e_d000_0000,
                last: 0x70b3_d57e_d000_00ff,
            };
            assert_eq!(
                pamoja_gateway_station_message_add_join_range(built, &range),
                PamojaStatus::Ok
            );
            for rate in [
                PamojaGatewayStationDataRate {
                    spreading_factor: 10,
                    bandwidth_hz: 125_000,
                    downlink_only: false,
                    defined: true,
                },
                PamojaGatewayStationDataRate {
                    spreading_factor: 0,
                    bandwidth_hz: 0,
                    downlink_only: false,
                    defined: false,
                },
                PamojaGatewayStationDataRate {
                    spreading_factor: 12,
                    bandwidth_hz: 500_000,
                    downlink_only: true,
                    defined: true,
                },
            ] {
                assert_eq!(
                    pamoja_gateway_station_message_add_data_rate(built, &rate),
                    PamojaStatus::Ok
                );
            }
            assert_eq!(written(built), expected.to_json());
        }
    }

    #[test]
    fn a_configuration_reads_back_entry_for_entry() {
        let config = Message::RouterConfig {
            net_id: None,
            join_eui: vec![(1, 2)],
            region: "EU868".to_owned(),
            max_eirp: 16.0,
            hwspec: "sx1301/1".to_owned(),
            freq_range: (863_000_000, 870_000_000),
            data_rates: vec![Some((12, 125_000, false)), None],
        };
        let text = config.to_json();

        unsafe {
            let read = pamoja_gateway_station_message_parse(text.as_ptr(), text.len());
            let mut fields = blank(PAMOJA_GATEWAY_STATION_OTHER);
            assert_eq!(
                pamoja_gateway_station_message_fields(read, &mut fields),
                PamojaStatus::Ok
            );
            assert_eq!(fields.kind, PAMOJA_GATEWAY_STATION_ROUTER_CONFIG);
            assert!(!fields.filters_networks, "no filter was written as null");
            assert_eq!(fields.join_range_count, 1);
            assert_eq!(fields.data_rate_count, 2);

            let mut rate = PamojaGatewayStationDataRate {
                spreading_factor: 0,
                bandwidth_hz: 0,
                downlink_only: false,
                defined: true,
            };
            assert_eq!(
                pamoja_gateway_station_message_data_rate(read, 1, &mut rate),
                PamojaStatus::Ok
            );
            assert!(!rate.defined, "the second number is left undefined");
            assert_eq!(
                pamoja_gateway_station_message_data_rate(read, 2, &mut rate),
                PamojaStatus::InvalidArgument
            );

            let mut region = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_station_message_text(
                    read,
                    PAMOJA_GATEWAY_STATION_TEXT_REGION,
                    &mut region
                ),
                PamojaStatus::Ok
            );
            assert_eq!(taken(region), b"EU868");
            pamoja_gateway_station_message_free(read);
        }
    }

    #[test]
    fn a_downlink_carries_its_windows_and_clock_both_ways() {
        let xtime = (0xa5_i64 << 48) | 1_000_000;
        let expected = Message::Downlink {
            dev_eui: Eui::new([0x11; 8]),
            class: 0,
            diid: 42,
            pdu: vec![0x60, 0x01],
            rx_delay: Some(1),
            rx1: Some((5, 868_100_000)),
            rx2: Some((0, 869_525_000)),
            ping_slot: None,
            priority: 0,
            xtime: Some(xtime),
            rctx: Some(0),
            gpstime: None,
        };

        unsafe {
            let mut fields = blank(PAMOJA_GATEWAY_STATION_DOWNLINK);
            fields.dev_eui = [0x11; 8];
            fields.diid = 42;
            fields.rx_delay = 1;
            fields.has_rx_delay = true;
            fields.rx1 = window_to_c(Some((5, 868_100_000)));
            fields.rx2 = window_to_c(Some((0, 869_525_000)));
            fields.xtime = xtime;
            fields.has_xtime = true;
            fields.has_rctx = true;
            let built = pamoja_gateway_station_message_new(&fields);
            let pdu = [0x60, 0x01];
            assert_eq!(
                pamoja_gateway_station_message_set_payload(built, pdu.as_ptr(), pdu.len()),
                PamojaStatus::Ok
            );
            let text = written(built);
            assert_eq!(text, expected.to_json());

            let read = pamoja_gateway_station_message_parse(text.as_ptr(), text.len());
            let mut back = blank(PAMOJA_GATEWAY_STATION_OTHER);
            pamoja_gateway_station_message_fields(read, &mut back);
            assert_eq!(back.xtime, xtime, "the clock comes back to the microsecond");
            assert!(back.rx1.present && back.rx2.present && !back.ping_slot.present);
            pamoja_gateway_station_message_free(read);
        }
    }

    #[test]
    fn what_a_kind_does_not_carry_is_refused() {
        unsafe {
            let built = pamoja_gateway_station_message_new(&blank(PAMOJA_GATEWAY_STATION_VERSION));
            let text = "EU868";
            assert_eq!(
                pamoja_gateway_station_message_set_text(
                    built,
                    PAMOJA_GATEWAY_STATION_TEXT_REGION,
                    text.as_ptr(),
                    text.len()
                ),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_gateway_station_message_add_net_id(built, 1),
                PamojaStatus::InvalidArgument
            );
            let mut out = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_station_message_text(built, 99, &mut out),
                PamojaStatus::InvalidArgument
            );
            pamoja_gateway_station_message_free(built);

            assert!(pamoja_gateway_station_message_new(&blank(42)).is_null());
            assert!(pamoja_gateway_station_message_new(ptr::null()).is_null());
        }
    }

    #[test]
    fn a_server_answers_a_station_and_reads_who_asked() {
        let station = [0xb8, 0x27, 0xeb, 0xff, 0xfe, 0x01, 0x02, 0x03];
        let asked = Discovery::new(Eui::new(station)).to_json();
        let uri = "ws://lns.example.invalid:3001/router";

        unsafe {
            let mut router = [0u8; 8];
            assert_eq!(
                pamoja_gateway_station_discovery_parse(
                    asked.as_ptr(),
                    asked.len(),
                    router.as_mut_ptr()
                ),
                PamojaStatus::Ok
            );
            assert_eq!(router, station);

            let muxs = [0u8; 8];
            let mut text = ptr::null_mut();
            assert_eq!(
                pamoja_gateway_station_router_accepted(
                    router.as_ptr(),
                    muxs.as_ptr(),
                    uri.as_ptr(),
                    uri.len(),
                    &mut text
                ),
                PamojaStatus::Ok
            );
            let answer = taken(text);
            assert_eq!(
                answer,
                Router::accepted(Eui::new(station), Eui::new(muxs), uri)
                    .to_json()
                    .into_bytes()
            );

            let mut ids = PamojaGatewayStationRouterIds {
                router: [0; 8],
                has_router: false,
                muxs: [1; 8],
                has_muxs: false,
            };
            assert_eq!(
                pamoja_gateway_station_router_identities(answer.as_ptr(), answer.len(), &mut ids),
                PamojaStatus::Ok
            );
            assert!(ids.has_router && ids.has_muxs);
            assert_eq!(ids.router, station);
            assert_eq!(ids.muxs, muxs);

            let why = "this gateway is not registered";
            assert_eq!(
                pamoja_gateway_station_router_refused(
                    router.as_ptr(),
                    why.as_ptr(),
                    why.len(),
                    &mut text
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                taken(text),
                Router::refused(Eui::new(station), why)
                    .to_json()
                    .into_bytes()
            );
        }
    }
}
