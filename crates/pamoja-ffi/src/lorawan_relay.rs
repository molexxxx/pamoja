//! The C ABI for a LoRaWAN relay, TS011-1.0.1: the wake-on-radio frames an end device and a
//! relay exchange, the uplinks a relay forwards, and the timing that keeps a wake-on-radio
//! preamble short.
//!
//! Everything here is a pure function of its arguments. Keys cross as a
//! [`PamojaLorawanWorKeys`] of two sixteen-byte arrays, carriers and states as small structs,
//! and frames as fixed-length arrays written in place, except a forwarded uplink, whose
//! length follows the end device's frame and crosses as a buffer.

use std::ptr;

use pamoja_lorawan::mac::relay_second_channel;
use pamoja_lorawan::relay::{
    self, open_wor_ack, t_offset_ms, unsynchronized_preamble_symbols, wor_ack, wor_join_request,
    wor_uplink, CadPeriodicity, CadToRx, Carrier, Forward, ForwardedUplink, StateSync,
    Synchronization, UplinkMetadata, Wor, WorChannel, WorKeys, XtalAccuracy,
};

use crate::lora_region::PamojaLoraRelayChannel;
use crate::lorawan::{failed, PamojaLorawanSession, PAMOJA_LORAWAN_KEY_LEN};
use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// The port every message between a relay and its network uses.
pub const PAMOJA_LORAWAN_LA_FPORT_RELAY: u8 = 226;
/// How many end devices a relay verifies wake-on-radio frames for.
pub const PAMOJA_LORAWAN_TRUSTED_ED_NUMBER: usize = 16;
/// How many WOR frames go without an acknowledgment before the uplink goes anyway.
pub const PAMOJA_LORAWAN_WOR_ATTEMPTS_WO_ACK: u8 = 8;
/// The gap between a WOR frame, or its acknowledgment, and the LoRaWAN frame after it.
pub const PAMOJA_LORAWAN_WOR_DATA_DELAY_US: u32 = 50_000;
/// The gap between a WOR frame and its acknowledgment.
pub const PAMOJA_LORAWAN_WOR_ACK_DELAY_US: u32 = 50_000;
/// The gap between a relay hearing an uplink and forwarding it.
pub const PAMOJA_LORAWAN_RELAY_FWD_DELAY_US: u32 = 50_000;
/// How long after an uplink an end device's RXR window opens at the latest.
pub const PAMOJA_LORAWAN_RXR_DELAY_US: u32 = 18_000_000;
/// The length of a WOR frame ahead of a join request.
pub const PAMOJA_LORAWAN_WOR_JOIN_REQUEST_LEN: usize = 5;
/// The length of a WOR frame ahead of a Class A uplink.
pub const PAMOJA_LORAWAN_WOR_UPLINK_LEN: usize = 15;
/// The length of a WOR ACK.
pub const PAMOJA_LORAWAN_WOR_ACK_LEN: usize = 7;
/// The bytes a forwarded uplink adds in front of the end device's frame.
pub const PAMOJA_LORAWAN_FORWARD_OVERHEAD: usize = 6;
/// The shortest WOR preamble, in symbols.
pub const PAMOJA_LORAWAN_MIN_WOR_PREAMBLE_SYMBOLS: u16 = 8;

// The header carries these as literals, because cbindgen drops a constant whose value
// names another crate's. These hold them to what those crates say.
const _: () = assert!(PAMOJA_LORAWAN_LA_FPORT_RELAY == relay::LA_FPORT_RELAY);
const _: () = assert!(PAMOJA_LORAWAN_TRUSTED_ED_NUMBER == relay::TRUSTED_ED_NUMBER);
const _: () = assert!(PAMOJA_LORAWAN_WOR_ATTEMPTS_WO_ACK == relay::WOR_ATTEMPTS_WO_ACK);
const _: () = assert!(PAMOJA_LORAWAN_WOR_DATA_DELAY_US == relay::WOR_DATA_DELAY_US);
const _: () = assert!(PAMOJA_LORAWAN_WOR_ACK_DELAY_US == relay::WOR_ACK_DELAY_US);
const _: () = assert!(PAMOJA_LORAWAN_RELAY_FWD_DELAY_US == relay::RELAY_FWD_DELAY_US);
const _: () = assert!(PAMOJA_LORAWAN_RXR_DELAY_US == relay::RXR_DELAY_US);
const _: () = assert!(PAMOJA_LORAWAN_WOR_JOIN_REQUEST_LEN == relay::WOR_JOIN_REQUEST_LEN);
const _: () = assert!(PAMOJA_LORAWAN_WOR_UPLINK_LEN == relay::WOR_UPLINK_LEN);
const _: () = assert!(PAMOJA_LORAWAN_WOR_ACK_LEN == relay::WOR_ACK_LEN);
const _: () = assert!(PAMOJA_LORAWAN_FORWARD_OVERHEAD == relay::FORWARD_OVERHEAD);
const _: () = assert!(PAMOJA_LORAWAN_MIN_WOR_PREAMBLE_SYMBOLS == relay::MIN_WOR_PREAMBLE_SYMBOLS);

/// A WOR frame ahead of a join request.
pub const PAMOJA_LORAWAN_WOR_JOIN_REQUEST: u8 = 0;
/// A WOR frame ahead of a Class A uplink.
pub const PAMOJA_LORAWAN_WOR_UPLINK: u8 = 1;

/// The integrity and encryption keys of one end device's wake-on-radio frames.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PamojaLorawanWorKeys {
    /// `WorSIntKey`.
    pub integrity: [u8; 16],
    /// `WorSEncKey`.
    pub encryption: [u8; 16],
}

/// Where a frame goes and how fast.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanCarrier {
    /// The frequency in hertz.
    pub frequency_hz: u32,
    /// The data rate.
    pub data_rate: u8,
}

/// A wake-on-radio frame, as a relay reads it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanWor {
    /// [`PAMOJA_LORAWAN_WOR_JOIN_REQUEST`] or [`PAMOJA_LORAWAN_WOR_UPLINK`].
    pub kind: u8,
    /// For a join request, where and how fast it follows; zero for an uplink, whose carrier
    /// is sealed until [`pamoja_lorawan_relay_wor_open`] reads it.
    pub uplink: PamojaLorawanCarrier,
    /// For an uplink, the address it names.
    pub dev_addr: u32,
    /// For an uplink, the low sixteen bits of its frame counter.
    pub wfcnt: u16,
}

/// What a relay tells an end device about itself in a WOR ACK, as TS011-1.0.1 table 14
/// codes each field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanStateSync {
    /// Symbols from detecting activity to receiving: 0 for 2, 1 for 4, 2 for 6, 3 for 8.
    pub cad_to_rx: u8,
    /// Whether the relay forwards: 0 yes, 1 and 2 retry in 30 or 60 minutes, 3 disabled.
    pub forward: u8,
    /// The data rate the relay forwards at.
    pub relay_data_rate: u8,
    /// Crystal accuracy: 0 to 3 for 10 to 40 parts per million.
    pub xtal_accuracy: u8,
    /// How often the relay scans: 0 to 5 for 1000, 500, 250, 100, 50 and 20 milliseconds.
    pub cad_periodicity: u8,
    /// Milliseconds from the start of the scan to the end of the WOR preamble.
    pub t_offset_ms: u16,
}

/// What a relay heard of an uplink it forwards.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanUplinkMetadata {
    /// The WOR channel: 0 for the default, 1 for the second.
    pub wor_channel: u8,
    /// The uplink's signal strength in dBm.
    pub rssi_dbm: i16,
    /// Its signal-to-noise ratio in dB.
    pub snr_db: i8,
    /// The data rate it arrived at.
    pub data_rate: u8,
}

/// What an end device knows of a relay's scans once a WOR ACK has arrived.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanSynchronization {
    /// When the relay scanned, in the device's microseconds.
    pub reference_us: u64,
    /// How often it scans, coded as in [`PamojaLorawanStateSync`].
    pub cad_periodicity: u8,
    /// Its crystal accuracy, coded.
    pub relay_xtal: u8,
    /// Its time to start receiving, coded.
    pub cad_to_rx: u8,
}

/// When a synchronized end device's next WOR frame goes out.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanWorSlot {
    /// When to start sending, in microseconds.
    pub start_us: u64,
    /// The preamble length in symbols.
    pub preamble_symbols: u16,
}

fn keys_in(keys: &PamojaLorawanWorKeys) -> WorKeys {
    WorKeys::new(keys.integrity, keys.encryption)
}

fn keys_out(keys: &WorKeys) -> PamojaLorawanWorKeys {
    PamojaLorawanWorKeys {
        integrity: *keys.integrity(),
        encryption: *keys.encryption(),
    }
}

fn carrier_in(carrier: &PamojaLorawanCarrier) -> Carrier {
    Carrier::new(carrier.frequency_hz, carrier.data_rate)
}

fn carrier_out(carrier: Carrier) -> PamojaLorawanCarrier {
    PamojaLorawanCarrier {
        frequency_hz: carrier.frequency_hz,
        data_rate: carrier.data_rate,
    }
}

fn two_bits(value: u8, what: &str) -> Result<u8, PamojaStatus> {
    if value > 3 {
        set_last_error(format!("{what} is coded in two bits, and {value} is not"));
        return Err(PamojaStatus::InvalidArgument);
    }
    Ok(value)
}

fn periodicity(code: u8) -> Result<CadPeriodicity, PamojaStatus> {
    CadPeriodicity::from_code(code).ok_or_else(|| {
        set_last_error(format!(
            "{code} is not a scan periodicity, which runs 0 to 5"
        ));
        PamojaStatus::InvalidArgument
    })
}

fn state_in(state: &PamojaLorawanStateSync) -> Result<StateSync, PamojaStatus> {
    Ok(StateSync {
        cad_to_rx: CadToRx::from_code(two_bits(state.cad_to_rx, "CadToRx")?),
        forward: Forward::from_code(two_bits(state.forward, "Forward")?),
        relay_data_rate: state.relay_data_rate,
        xtal_accuracy: XtalAccuracy::from_code(two_bits(state.xtal_accuracy, "XTALAccuracy")?),
        cad_periodicity: periodicity(state.cad_periodicity)?,
        t_offset_ms: state.t_offset_ms,
    })
}

fn state_out(state: StateSync) -> PamojaLorawanStateSync {
    PamojaLorawanStateSync {
        cad_to_rx: state.cad_to_rx.code(),
        forward: state.forward.code(),
        relay_data_rate: state.relay_data_rate,
        xtal_accuracy: state.xtal_accuracy.code(),
        cad_periodicity: state.cad_periodicity.code(),
        t_offset_ms: state.t_offset_ms,
    }
}

fn null(what: &str) -> PamojaStatus {
    set_last_error(format!("{what} must not be null"));
    PamojaStatus::InvalidArgument
}

/// Derives an end device's root relay session key from its network session key,
/// TS011-1.0.1 section 4.4.
///
/// # Arguments
///
/// * `network_key` - the network session key, `NwkSKey` under LoRaWAN 1.0.x.
/// * `network_key_len` - its length, which must be [`PAMOJA_LORAWAN_KEY_LEN`].
/// * `out_key` - receives the sixteen-byte root key.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer or a key of the wrong length.
///
/// # Safety
///
/// `network_key` must point to `network_key_len` readable bytes and `out_key` to sixteen
/// writable ones.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_root_wor_s_key(
    network_key: *const u8,
    network_key_len: usize,
    out_key: *mut u8,
) -> PamojaStatus {
    if out_key.is_null() {
        return null("out_key");
    }
    let key = match sixteen(network_key, network_key_len, "the network key") {
        Ok(key) => key,
        Err(status) => return status,
    };
    let root = relay::root_wor_s_key(&key);
    ptr::copy_nonoverlapping(root.as_ptr(), out_key, root.len());
    PamojaStatus::Ok
}

/// Derives an end device's wake-on-radio keys from its root relay session key, TS011-1.0.1
/// section 4.5.
///
/// # Arguments
///
/// * `root_key` - the root relay session key.
/// * `root_key_len` - its length, which must be [`PAMOJA_LORAWAN_KEY_LEN`].
/// * `dev_addr` - the device's address.
/// * `out_keys` - receives the keys.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer or a key of the wrong length.
///
/// # Safety
///
/// `root_key` must point to `root_key_len` readable bytes and `out_keys` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_wor_keys(
    root_key: *const u8,
    root_key_len: usize,
    dev_addr: u32,
    out_keys: *mut PamojaLorawanWorKeys,
) -> PamojaStatus {
    if out_keys.is_null() {
        return null("out_keys");
    }
    let root = match sixteen(root_key, root_key_len, "the root key") {
        Ok(root) => root,
        Err(status) => return status,
    };
    *out_keys = keys_out(&WorKeys::derive(&root, dev_addr));
    PamojaStatus::Ok
}

/// Returns the root relay session key of a session's device.
///
/// # Arguments
///
/// * `session` - the session.
/// * `out_key` - receives the sixteen-byte key.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `session` must be a live handle and `out_key` must point to sixteen writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_session_root_wor_s_key(
    session: *const PamojaLorawanSession,
    out_key: *mut u8,
) -> PamojaStatus {
    let (Some(session), false) = (session.as_ref(), out_key.is_null()) else {
        return null("session and out_key");
    };
    let root = session.session.root_wor_s_key();
    ptr::copy_nonoverlapping(root.as_ptr(), out_key, root.len());
    PamojaStatus::Ok
}

/// Returns the wake-on-radio keys of a session's device.
///
/// # Arguments
///
/// * `session` - the session.
/// * `out_keys` - receives the keys.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `session` must be a live handle and `out_keys` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_session_wor_keys(
    session: *const PamojaLorawanSession,
    out_keys: *mut PamojaLorawanWorKeys,
) -> PamojaStatus {
    let (Some(session), false) = (session.as_ref(), out_keys.is_null()) else {
        return null("session and out_keys");
    };
    *out_keys = keys_out(&session.session.wor_keys());
    PamojaStatus::Ok
}

/// Builds the WOR frame ahead of a join request, TS011-1.0.1 section 5.3.1.
///
/// # Arguments
///
/// * `uplink` - where and how fast the join request follows.
/// * `out_frame` - receives the [`PAMOJA_LORAWAN_WOR_JOIN_REQUEST_LEN`] bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, and [`PamojaStatus::Codec`]
/// for a carrier the fields cannot hold.
///
/// # Safety
///
/// `uplink` must be readable and `out_frame` must point to
/// [`PAMOJA_LORAWAN_WOR_JOIN_REQUEST_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_wor_join_request(
    uplink: *const PamojaLorawanCarrier,
    out_frame: *mut u8,
) -> PamojaStatus {
    let (Some(uplink), false) = (uplink.as_ref(), out_frame.is_null()) else {
        return null("uplink and out_frame");
    };
    match wor_join_request(carrier_in(uplink)) {
        Ok(frame) => {
            ptr::copy_nonoverlapping(frame.as_ptr(), out_frame, frame.len());
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Builds the WOR frame ahead of a Class A uplink, TS011-1.0.1 section 5.3.2.
///
/// # Arguments
///
/// * `keys` - the device's keys.
/// * `dev_addr` - its address.
/// * `wfcnt` - the WOR frame counter.
/// * `uplink` - where and how fast the uplink follows.
/// * `wor` - the carrier this WOR frame goes out on.
/// * `out_frame` - receives the [`PAMOJA_LORAWAN_WOR_UPLINK_LEN`] bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, and [`PamojaStatus::Codec`]
/// for a carrier the fields cannot hold.
///
/// # Safety
///
/// The pointers must be readable and `out_frame` must point to
/// [`PAMOJA_LORAWAN_WOR_UPLINK_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_wor_uplink(
    keys: *const PamojaLorawanWorKeys,
    dev_addr: u32,
    wfcnt: u32,
    uplink: *const PamojaLorawanCarrier,
    wor: *const PamojaLorawanCarrier,
    out_frame: *mut u8,
) -> PamojaStatus {
    let (Some(keys), Some(uplink), Some(wor), false) = (
        keys.as_ref(),
        uplink.as_ref(),
        wor.as_ref(),
        out_frame.is_null(),
    ) else {
        return null("keys, uplink, wor and out_frame");
    };
    match wor_uplink(
        &keys_in(keys),
        dev_addr,
        wfcnt,
        carrier_in(uplink),
        carrier_in(wor),
    ) {
        Ok(frame) => {
            ptr::copy_nonoverlapping(frame.as_ptr(), out_frame, frame.len());
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Reads a WOR frame.
///
/// # Arguments
///
/// * `frame` - the bytes a relay received.
/// * `frame_len` - their length.
/// * `out_wor` - receives what the frame is.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, and [`PamojaStatus::Codec`]
/// for a reserved or proprietary type, or a length that is not the type's.
///
/// # Safety
///
/// `frame` must point to `frame_len` readable bytes and `out_wor` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_wor_parse(
    frame: *const u8,
    frame_len: usize,
    out_wor: *mut PamojaLorawanWor,
) -> PamojaStatus {
    if out_wor.is_null() {
        return null("out_wor");
    }
    let bytes = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match Wor::parse(&bytes) {
        Ok(Wor::JoinRequest { uplink }) => {
            *out_wor = PamojaLorawanWor {
                kind: PAMOJA_LORAWAN_WOR_JOIN_REQUEST,
                uplink: carrier_out(uplink),
                dev_addr: 0,
                wfcnt: 0,
            };
            PamojaStatus::Ok
        }
        Ok(Wor::Uplink(sealed)) => {
            *out_wor = PamojaLorawanWor {
                kind: PAMOJA_LORAWAN_WOR_UPLINK,
                uplink: PamojaLorawanCarrier {
                    frequency_hz: 0,
                    data_rate: 0,
                },
                dev_addr: sealed.dev_addr(),
                wfcnt: sealed.wfcnt(),
            };
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Checks a WOR frame ahead of a Class A uplink and reads where the uplink follows.
///
/// # Arguments
///
/// * `frame` - the frame a relay received.
/// * `frame_len` - its length.
/// * `keys` - the keys of the device it names.
/// * `wfcnt` - the full 32-bit counter the relay takes it to carry.
/// * `wor` - the carrier it arrived on.
/// * `out_uplink` - receives where and how fast the uplink follows.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, [`PamojaStatus::Codec`] for
/// a frame that is not an uplink WOR, and [`PamojaStatus::Auth`] for a counter whose low bits
/// differ or an integrity code that does not verify.
///
/// # Safety
///
/// `frame` must point to `frame_len` readable bytes, and the other pointers must be readable
/// or writable as they are used.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_wor_open(
    frame: *const u8,
    frame_len: usize,
    keys: *const PamojaLorawanWorKeys,
    wfcnt: u32,
    wor: *const PamojaLorawanCarrier,
    out_uplink: *mut PamojaLorawanCarrier,
) -> PamojaStatus {
    let (Some(keys), Some(wor), false) = (keys.as_ref(), wor.as_ref(), out_uplink.is_null()) else {
        return null("keys, wor and out_uplink");
    };
    let bytes = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let sealed = match Wor::parse(&bytes) {
        Ok(Wor::Uplink(sealed)) => sealed,
        Ok(Wor::JoinRequest { .. }) => {
            set_last_error("a join request WOR carries nothing sealed".to_owned());
            return PamojaStatus::Codec;
        }
        Err(error) => return failed(error),
    };
    match sealed.open(&keys_in(keys), wfcnt, carrier_in(wor)) {
        Ok(uplink) => {
            *out_uplink = carrier_out(uplink);
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Builds a relay's WOR ACK, TS011-1.0.1 section 6.2.
///
/// # Arguments
///
/// * `keys` - the end device's keys.
/// * `dev_addr` - its address.
/// * `wfcnt` - the 32-bit counter of the acknowledged WOR frame.
/// * `ack` - the carrier the acknowledgment goes out on.
/// * `uplink` - the carrier the WOR frame named for the uplink.
/// * `state` - what the relay tells the device.
/// * `out_frame` - receives the [`PAMOJA_LORAWAN_WOR_ACK_LEN`] bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer or a coded field out of range,
/// and [`PamojaStatus::Codec`] for a state or carrier the fields cannot hold.
///
/// # Safety
///
/// The pointers must be readable and `out_frame` must point to
/// [`PAMOJA_LORAWAN_WOR_ACK_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_wor_ack(
    keys: *const PamojaLorawanWorKeys,
    dev_addr: u32,
    wfcnt: u32,
    ack: *const PamojaLorawanCarrier,
    uplink: *const PamojaLorawanCarrier,
    state: *const PamojaLorawanStateSync,
    out_frame: *mut u8,
) -> PamojaStatus {
    let (Some(keys), Some(ack), Some(uplink), Some(state), false) = (
        keys.as_ref(),
        ack.as_ref(),
        uplink.as_ref(),
        state.as_ref(),
        out_frame.is_null(),
    ) else {
        return null("keys, ack, uplink, state and out_frame");
    };
    let state = match state_in(state) {
        Ok(state) => state,
        Err(status) => return status,
    };
    match wor_ack(
        &keys_in(keys),
        dev_addr,
        wfcnt,
        carrier_in(ack),
        carrier_in(uplink),
        state,
    ) {
        Ok(frame) => {
            ptr::copy_nonoverlapping(frame.as_ptr(), out_frame, frame.len());
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Checks and reads a WOR ACK, TS011-1.0.1 section 6.2.
///
/// # Arguments
///
/// * `frame` - the acknowledgment an end device received.
/// * `frame_len` - its length.
/// * `keys` - the device's keys.
/// * `dev_addr` - its address.
/// * `wfcnt` - the counter of the WOR frame it sent.
/// * `ack` - the carrier the acknowledgment arrived on.
/// * `uplink` - the carrier the WOR frame named.
/// * `out_state` - receives what the relay said.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, [`PamojaStatus::Codec`] for
/// a frame of the wrong length or a reserved periodicity, and [`PamojaStatus::Auth`] when the
/// integrity code does not verify.
///
/// # Safety
///
/// `frame` must point to `frame_len` readable bytes, and the other pointers must be readable
/// or writable as they are used.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_wor_ack_open(
    frame: *const u8,
    frame_len: usize,
    keys: *const PamojaLorawanWorKeys,
    dev_addr: u32,
    wfcnt: u32,
    ack: *const PamojaLorawanCarrier,
    uplink: *const PamojaLorawanCarrier,
    out_state: *mut PamojaLorawanStateSync,
) -> PamojaStatus {
    let (Some(keys), Some(ack), Some(uplink), false) = (
        keys.as_ref(),
        ack.as_ref(),
        uplink.as_ref(),
        out_state.is_null(),
    ) else {
        return null("keys, ack, uplink and out_state");
    };
    let bytes = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match open_wor_ack(
        &bytes,
        &keys_in(keys),
        dev_addr,
        wfcnt,
        carrier_in(ack),
        carrier_in(uplink),
    ) {
        Ok(state) => {
            *out_state = state_out(state);
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Writes an uplink a relay forwards, TS011-1.0.1 section 9.1.
///
/// # Arguments
///
/// * `metadata` - what the relay heard of it.
/// * `frequency_hz` - the frequency it arrived on.
/// * `phy_payload` - the end device's frame.
/// * `phy_payload_len` - its length.
/// * `out_payload` - receives the relay uplink's payload for port 226, released with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer or a WOR channel past 1, and
/// [`PamojaStatus::Codec`] for a data rate or frequency the fields cannot hold.
///
/// # Safety
///
/// `metadata` must be readable, `phy_payload` must point to `phy_payload_len` readable bytes,
/// and `out_payload` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_forward_encode(
    metadata: *const PamojaLorawanUplinkMetadata,
    frequency_hz: u32,
    phy_payload: *const u8,
    phy_payload_len: usize,
    out_payload: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let (Some(metadata), false) = (metadata.as_ref(), out_payload.is_null()) else {
        return null("metadata and out_payload");
    };
    let wor_channel = match metadata.wor_channel {
        0 => WorChannel::Default,
        1 => WorChannel::Second,
        other => {
            set_last_error(format!("{other} is not a WOR channel, which is 0 or 1"));
            return PamojaStatus::InvalidArgument;
        }
    };
    let frame = match read_bytes(phy_payload, phy_payload_len) {
        Ok(frame) => frame,
        Err(status) => return status,
    };
    let forwarded = ForwardedUplink {
        metadata: UplinkMetadata {
            wor_channel,
            rssi_dbm: metadata.rssi_dbm,
            snr_db: metadata.snr_db,
            data_rate: metadata.data_rate,
        },
        frequency_hz,
        phy_payload: &frame,
    };
    let mut out = vec![0u8; relay::FORWARD_OVERHEAD + frame.len()];
    match forwarded.encode(&mut out) {
        Ok(_) => {
            *out_payload = PamojaBuffer::into_raw(out);
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Reads an uplink a relay forwarded, TS011-1.0.1 section 9.1.
///
/// # Arguments
///
/// * `payload` - the relay uplink's payload on port 226.
/// * `payload_len` - its length.
/// * `out_metadata` - receives what the relay heard.
/// * `out_frequency_hz` - receives the frequency the uplink arrived on.
/// * `out_phy_payload` - receives the end device's frame, released with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, and [`PamojaStatus::Codec`]
/// for a payload shorter than [`PAMOJA_LORAWAN_FORWARD_OVERHEAD`] or a reserved WOR channel.
///
/// # Safety
///
/// `payload` must point to `payload_len` readable bytes and the out pointers must be
/// writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_forward_parse(
    payload: *const u8,
    payload_len: usize,
    out_metadata: *mut PamojaLorawanUplinkMetadata,
    out_frequency_hz: *mut u32,
    out_phy_payload: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if out_metadata.is_null() || out_frequency_hz.is_null() || out_phy_payload.is_null() {
        return null("out_metadata, out_frequency_hz and out_phy_payload");
    }
    let bytes = match read_bytes(payload, payload_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match ForwardedUplink::parse(&bytes) {
        Ok(forwarded) => {
            *out_metadata = PamojaLorawanUplinkMetadata {
                wor_channel: match forwarded.metadata.wor_channel {
                    WorChannel::Default => 0,
                    WorChannel::Second => 1,
                },
                rssi_dbm: forwarded.metadata.rssi_dbm,
                snr_db: forwarded.metadata.snr_db,
                data_rate: forwarded.metadata.data_rate,
            };
            *out_frequency_hz = forwarded.frequency_hz;
            *out_phy_payload = PamojaBuffer::into_raw(forwarded.phy_payload.to_vec());
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Works out the WOR preamble of an end device that does not know when the relay scans,
/// TS011-1.0.1 section 5.2.
///
/// # Arguments
///
/// * `cad_periodicity` - how often the relay scans, coded 0 to 5.
/// * `symbol_us` - the symbol time of the WOR frame's data rate, in microseconds.
/// * `cad_to_rx` - the relay's time to start receiving, coded 0 to 3.
/// * `out_symbols` - receives the preamble length in symbols.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer or a code out of range.
///
/// # Safety
///
/// `out_symbols` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_unsynchronized_preamble(
    cad_periodicity: u8,
    symbol_us: u64,
    cad_to_rx: u8,
    out_symbols: *mut u16,
) -> PamojaStatus {
    if out_symbols.is_null() {
        return null("out_symbols");
    }
    let (period, receive) = match (periodicity(cad_periodicity), two_bits(cad_to_rx, "CadToRx")) {
        (Ok(period), Ok(receive)) => (period, CadToRx::from_code(receive)),
        (Err(status), _) | (_, Err(status)) => return status,
    };
    *out_symbols = unsynchronized_preamble_symbols(period, symbol_us, receive);
    PamojaStatus::Ok
}

/// Works out the offset a relay reports in a WOR ACK, TS011-1.0.1 appendix 1.
///
/// # Arguments
///
/// * `scan_start_us` - when the scan that detected the frame started.
/// * `preamble_end_us` - when the frame's preamble ended: when it finished arriving, less
///   the airtime of its sync word and payload.
/// * `out_offset_ms` - receives the offset in milliseconds.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, a preamble that ended
/// before the scan started, or an offset past the eleven bits a WOR ACK carries.
///
/// # Safety
///
/// `out_offset_ms` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_t_offset_ms(
    scan_start_us: u64,
    preamble_end_us: u64,
    out_offset_ms: *mut u16,
) -> PamojaStatus {
    if out_offset_ms.is_null() {
        return null("out_offset_ms");
    }
    match t_offset_ms(scan_start_us, preamble_end_us) {
        Some(offset) => {
            *out_offset_ms = offset;
            PamojaStatus::Ok
        }
        None => {
            set_last_error(
                "the preamble ended before the scan, or past the eleven bits a WOR ACK carries"
                    .to_owned(),
            );
            PamojaStatus::InvalidArgument
        }
    }
}

/// Works out when a relay scanned from the WOR ACK that answered a frame.
///
/// # Arguments
///
/// * `wor_start_us` - when the acknowledged WOR frame started going out.
/// * `preamble_symbols` - its preamble length.
/// * `symbol_us` - its symbol time.
/// * `state` - what the acknowledgment said.
/// * `out_synchronization` - receives the synchronization.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer or a coded field out of
/// range.
///
/// # Safety
///
/// `state` must be readable and `out_synchronization` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_synchronization(
    wor_start_us: u64,
    preamble_symbols: u16,
    symbol_us: u64,
    state: *const PamojaLorawanStateSync,
    out_synchronization: *mut PamojaLorawanSynchronization,
) -> PamojaStatus {
    let (Some(state), false) = (state.as_ref(), out_synchronization.is_null()) else {
        return null("state and out_synchronization");
    };
    let state = match state_in(state) {
        Ok(state) => state,
        Err(status) => return status,
    };
    let sync = Synchronization::from_ack(wor_start_us, preamble_symbols, symbol_us, &state);
    *out_synchronization = PamojaLorawanSynchronization {
        reference_us: sync.reference_us,
        cad_periodicity: sync.cad_periodicity.code(),
        relay_xtal: sync.relay_xtal.code(),
        cad_to_rx: sync.cad_to_rx.code(),
    };
    PamojaStatus::Ok
}

/// Picks the relay scan a synchronized end device aims its next WOR frame at, TS011-1.0.1
/// appendix 1.
///
/// # Arguments
///
/// * `synchronization` - what the device knows of the relay.
/// * `now_us` - the time.
/// * `device_xtal_ppm` - the device's crystal accuracy.
/// * `symbol_us` - the symbol time of the WOR frame's data rate.
/// * `other_channel` - `1` when the frame goes out on the relay's other channel.
/// * `out_slot` - receives when to send and with how long a preamble.
/// * `out_synchronized` - receives `1` with a slot, and `0` once the drift exceeds a period
///   and the device should send an unsynchronized preamble instead.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer or a coded field out of
/// range.
///
/// # Safety
///
/// `synchronization` must be readable and the out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_next_wor(
    synchronization: *const PamojaLorawanSynchronization,
    now_us: u64,
    device_xtal_ppm: u32,
    symbol_us: u64,
    other_channel: u8,
    out_slot: *mut PamojaLorawanWorSlot,
    out_synchronized: *mut u8,
) -> PamojaStatus {
    let (Some(sync), false, false) = (
        synchronization.as_ref(),
        out_slot.is_null(),
        out_synchronized.is_null(),
    ) else {
        return null("synchronization, out_slot and out_synchronized");
    };
    let sync = match (
        periodicity(sync.cad_periodicity),
        two_bits(sync.relay_xtal, "the relay crystal accuracy"),
        two_bits(sync.cad_to_rx, "CadToRx"),
    ) {
        (Ok(period), Ok(xtal), Ok(receive)) => Synchronization {
            reference_us: sync.reference_us,
            cad_periodicity: period,
            relay_xtal: XtalAccuracy::from_code(xtal),
            cad_to_rx: CadToRx::from_code(receive),
        },
        (Err(status), _, _) | (_, Err(status), _) | (_, _, Err(status)) => return status,
    };
    match sync.next_wor(now_us, device_xtal_ppm, symbol_us, other_channel != 0) {
        Some(slot) => {
            *out_slot = PamojaLorawanWorSlot {
                start_us: slot.start_us,
                preamble_symbols: slot.preamble_symbols,
            };
            *out_synchronized = 1;
        }
        None => {
            *out_slot = PamojaLorawanWorSlot {
                start_us: 0,
                preamble_symbols: 0,
            };
            *out_synchronized = 0;
        }
    }
    PamojaStatus::Ok
}

/// Reads the second channel a relay or end device configuration describes.
///
/// # Arguments
///
/// * `second_channel_index` - the coded index, 1 for a second channel.
/// * `data_rate` - its data rate.
/// * `ack_offset` - the coded acknowledgment offset, TS011-1.0.1 table 35.
/// * `frequency_hz` - its frequency.
/// * `out_channel` - receives the channel with its acknowledgment frequency.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, an index that is not 1, or a
/// reserved offset.
///
/// # Safety
///
/// `out_channel` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_second_channel(
    second_channel_index: u8,
    data_rate: u8,
    ack_offset: u8,
    frequency_hz: u32,
    out_channel: *mut PamojaLoraRelayChannel,
) -> PamojaStatus {
    if out_channel.is_null() {
        return null("out_channel");
    }
    match relay_second_channel(second_channel_index, data_rate, ack_offset, frequency_hz) {
        Some(channel) => {
            *out_channel = PamojaLoraRelayChannel {
                wor_frequency_hz: channel.wor_frequency_hz,
                ack_frequency_hz: channel.ack_frequency_hz,
                data_rate: channel.data_rate,
            };
            PamojaStatus::Ok
        }
        None => {
            set_last_error(format!(
                "index {second_channel_index} with offset {ack_offset} names no second channel"
            ));
            PamojaStatus::InvalidArgument
        }
    }
}

unsafe fn sixteen(bytes: *const u8, len: usize, what: &str) -> Result<[u8; 16], PamojaStatus> {
    let bytes = read_bytes(bytes, len)?;
    <[u8; PAMOJA_LORAWAN_KEY_LEN]>::try_from(&bytes[..]).map_err(|_| {
        set_last_error(format!(
            "{what} must be exactly {PAMOJA_LORAWAN_KEY_LEN} bytes"
        ));
        PamojaStatus::InvalidArgument
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lorawan::{pamoja_lorawan_session_free, pamoja_lorawan_session_new};
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    #[test]
    fn a_wor_goes_out_opens_and_is_acknowledged_across_the_boundary() {
        unsafe {
            let mut session = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_session_new(
                    0x2601_1BDA,
                    [0x2B; 16].as_ptr(),
                    16,
                    [0x99; 16].as_ptr(),
                    16,
                    &mut session
                ),
                PamojaStatus::Ok
            );
            let mut keys = PamojaLorawanWorKeys {
                integrity: [0; 16],
                encryption: [0; 16],
            };
            assert_eq!(
                pamoja_lorawan_session_wor_keys(session, &mut keys),
                PamojaStatus::Ok
            );
            let mut root = [0u8; 16];
            pamoja_lorawan_session_root_wor_s_key(session, root.as_mut_ptr());
            let mut derived = keys;
            derived.integrity = [0; 16];
            assert_eq!(
                pamoja_lorawan_relay_wor_keys(root.as_ptr(), 16, 0x2601_1BDA, &mut derived),
                PamojaStatus::Ok
            );
            assert!(derived == keys, "the session and the root key agree");

            let wor = PamojaLorawanCarrier {
                frequency_hz: 865_100_000,
                data_rate: 3,
            };
            let uplink = PamojaLorawanCarrier {
                frequency_hz: 868_100_000,
                data_rate: 5,
            };
            let mut frame = [0u8; PAMOJA_LORAWAN_WOR_UPLINK_LEN];
            assert_eq!(
                pamoja_lorawan_relay_wor_uplink(
                    &keys,
                    0x2601_1BDA,
                    1,
                    &uplink,
                    &wor,
                    frame.as_mut_ptr()
                ),
                PamojaStatus::Ok
            );
            assert_eq!(frame[..5], [0x01, 0xda, 0x1b, 0x01, 0x26]);

            let mut read = std::mem::zeroed::<PamojaLorawanWor>();
            assert_eq!(
                pamoja_lorawan_relay_wor_parse(frame.as_ptr(), frame.len(), &mut read),
                PamojaStatus::Ok
            );
            assert_eq!(
                (read.kind, read.dev_addr, read.wfcnt),
                (PAMOJA_LORAWAN_WOR_UPLINK, 0x2601_1BDA, 1)
            );
            let mut opened = std::mem::zeroed::<PamojaLorawanCarrier>();
            assert_eq!(
                pamoja_lorawan_relay_wor_open(
                    frame.as_ptr(),
                    frame.len(),
                    &keys,
                    1,
                    &wor,
                    &mut opened
                ),
                PamojaStatus::Ok
            );
            assert_eq!(opened, uplink);
            assert_eq!(
                pamoja_lorawan_relay_wor_open(
                    frame.as_ptr(),
                    frame.len(),
                    &keys,
                    0x0001_0001,
                    &wor,
                    &mut opened
                ),
                PamojaStatus::Auth
            );

            let ack = PamojaLorawanCarrier {
                frequency_hz: 865_300_000,
                data_rate: 3,
            };
            let state = PamojaLorawanStateSync {
                cad_to_rx: 1,
                forward: 0,
                relay_data_rate: 5,
                xtal_accuracy: 2,
                cad_periodicity: 1,
                t_offset_ms: 892,
            };
            let mut answer = [0u8; PAMOJA_LORAWAN_WOR_ACK_LEN];
            assert_eq!(
                pamoja_lorawan_relay_wor_ack(
                    &keys,
                    0x2601_1BDA,
                    1,
                    &ack,
                    &uplink,
                    &state,
                    answer.as_mut_ptr()
                ),
                PamojaStatus::Ok
            );
            assert_eq!(answer, [0xc0, 0x91, 0xac, 0x43, 0x19, 0x78, 0xef]);
            let mut heard = std::mem::zeroed::<PamojaLorawanStateSync>();
            assert_eq!(
                pamoja_lorawan_relay_wor_ack_open(
                    answer.as_ptr(),
                    answer.len(),
                    &keys,
                    0x2601_1BDA,
                    1,
                    &ack,
                    &uplink,
                    &mut heard
                ),
                PamojaStatus::Ok
            );
            assert_eq!(heard, state);

            let mut bad = state;
            bad.cad_periodicity = 6;
            assert_eq!(
                pamoja_lorawan_relay_wor_ack(
                    &keys,
                    0x2601_1BDA,
                    1,
                    &ack,
                    &uplink,
                    &bad,
                    answer.as_mut_ptr()
                ),
                PamojaStatus::InvalidArgument
            );
            pamoja_lorawan_session_free(session);
        }
    }

    #[test]
    fn a_forwarded_uplink_and_the_timing_cross_the_boundary() {
        unsafe {
            let metadata = PamojaLorawanUplinkMetadata {
                wor_channel: 1,
                rssi_dbm: -100,
                snr_db: 5,
                data_rate: 5,
            };
            let frame = [0x40u8, 0x01, 0x02];
            let mut payload = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_relay_forward_encode(
                    &metadata,
                    868_100_000,
                    frame.as_ptr(),
                    frame.len(),
                    &mut payload
                ),
                PamojaStatus::Ok
            );
            let bytes =
                std::slice::from_raw_parts(pamoja_buffer_data(payload), pamoja_buffer_len(payload))
                    .to_vec();
            pamoja_buffer_free(payload);
            assert_eq!(
                bytes,
                [0x95, 0xab, 0x01, 0x28, 0x76, 0x84, 0x40, 0x01, 0x02]
            );

            let mut read = std::mem::zeroed::<PamojaLorawanUplinkMetadata>();
            let mut frequency = 0u32;
            let mut phy = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_relay_forward_parse(
                    bytes.as_ptr(),
                    bytes.len(),
                    &mut read,
                    &mut frequency,
                    &mut phy
                ),
                PamojaStatus::Ok
            );
            assert_eq!((read, frequency), (metadata, 868_100_000));
            assert_eq!(pamoja_buffer_len(phy), 3);
            pamoja_buffer_free(phy);

            let mut symbols = 0u16;
            assert_eq!(
                pamoja_lorawan_relay_unsynchronized_preamble(1, 8_192, 1, &mut symbols),
                PamojaStatus::Ok
            );
            assert_eq!(symbols, 72);
            let mut offset = 0u16;
            assert_eq!(
                pamoja_lorawan_relay_t_offset_ms(87_654_000, 88_545_584, &mut offset),
                PamojaStatus::Ok
            );
            assert_eq!(offset, 892);

            let state = PamojaLorawanStateSync {
                cad_to_rx: 1,
                forward: 0,
                relay_data_rate: 3,
                xtal_accuracy: 2,
                cad_periodicity: 1,
                t_offset_ms: 892,
            };
            let mut sync = std::mem::zeroed::<PamojaLorawanSynchronization>();
            assert_eq!(
                pamoja_lorawan_relay_synchronization(1_234_000, 133, 8_192, &state, &mut sync),
                PamojaStatus::Ok
            );
            assert_eq!(sync.reference_us, 1_431_536);
            let mut slot = std::mem::zeroed::<PamojaLorawanWorSlot>();
            let mut synchronized = 0u8;
            assert_eq!(
                pamoja_lorawan_relay_next_wor(
                    &sync,
                    61_000_000,
                    20,
                    8_192,
                    0,
                    &mut slot,
                    &mut synchronized
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                (synchronized, slot.start_us, slot.preamble_symbols),
                (1, 61_430_036, 11)
            );

            let mut channel = std::mem::zeroed::<PamojaLoraRelayChannel>();
            assert_eq!(
                pamoja_lorawan_relay_second_channel(1, 3, 1, 868_100_000, &mut channel),
                PamojaStatus::Ok
            );
            assert_eq!(channel.ack_frequency_hz, 868_300_000);
        }
    }
}
