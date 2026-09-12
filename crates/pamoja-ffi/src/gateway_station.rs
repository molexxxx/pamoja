//! The C ABI for the LoRa Basics Station protocol.
//!
//! These functions wrap [`pamoja_gateway::station`] for callers that reach the SDK through the
//! flat C boundary. A station and its network server exchange JSON messages over a websocket
//! the caller owns, so nothing here opens a socket: a message crosses as an opaque handle,
//! built from a frame the radio heard or read from text that arrived, and written back out as
//! the text to send.
//!
//! A handle is used rather than one flat struct because the messages differ too much to share
//! one shape: a configuration carries a list of channels and a schedule carries a list of
//! frames, while a join request carries eight fixed fields. The accessors below read whichever
//! fields the kind in hand actually has.

use std::os::raw::c_char;
use std::ptr;

use pamoja_gateway::station::{eui_of, id6, Discovery, Levels, Message, Router};
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
pub const PAMOJA_GATEWAY_STATION_PROTOCOL_VERSION: u32 = pamoja_gateway::station::PROTOCOL_VERSION;

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

/// The fields a message carries, for the kinds built from fixed fields.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaGatewayStationFields {
    /// Which kind this is, one of the `PAMOJA_GATEWAY_STATION_*` constants.
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
    /// The data rate it arrived at, or is to be sent at.
    pub data_rate: u8,
    /// The frequency in hertz.
    pub frequency_hz: u32,
    /// How it was heard, for the kinds a station sends up.
    pub levels: PamojaGatewayStationLevels,
    /// Which class of downlink this is.
    pub class: u8,
    /// The identifier a transmission report carries back.
    pub diid: i64,
    /// The delay before the first receive window, in seconds, when `has_rx_delay`.
    pub rx_delay: u8,
    /// Whether a downlink named a receive delay.
    pub has_rx_delay: bool,
    /// How urgent a downlink is.
    pub priority: u8,
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
        Ok(message) => Box::into_raw(Box::new(PamojaGatewayStationMessage { message })),
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
        Ok(message) => Box::into_raw(Box::new(PamojaGatewayStationMessage { message })),
        Err(error) => {
            set_last_error(error.to_string());
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
/// One of the `PAMOJA_GATEWAY_STATION_*` constants, or
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

/// Reads the fields a message carries.
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
        Message::Downlink {
            dev_eui,
            class,
            diid,
            rx_delay,
            priority,
            ..
        } => {
            fields.dev_eui = dev_eui.bytes();
            fields.class = *class;
            fields.diid = *diid;
            fields.rx_delay = rx_delay.unwrap_or_default();
            fields.has_rx_delay = rx_delay.is_some();
            fields.priority = *priority;
        }
        Message::Transmitted {
            diid,
            dev_eui,
            rctx,
            xtime,
            gpstime,
            ..
        } => {
            fields.diid = *diid;
            fields.dev_eui = dev_eui.bytes();
            fields.levels = levels_to_c(Levels {
                rctx: *rctx,
                xtime: *xtime,
                gpstime: *gpstime,
                rssi: 0.0,
                snr: 0.0,
            });
        }
        _ => {}
    }

    fields
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
