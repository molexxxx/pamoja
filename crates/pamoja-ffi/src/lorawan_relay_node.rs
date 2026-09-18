//! A LoRaWAN relay across the C ABI, TS011-1.0.1.
//!
//! [`pamoja_lorawan::relay::Relay`] is an end device that also listens for others: it scans
//! its channels, verifies the wake-on-radio frames of the devices its network trusts it
//! with, answers them, takes the uplinks behind them, forwards those to the network on port
//! 226, and sends the answers on in each device's relay window. It owns no radio and no
//! clock, so every call takes the time and hands back what to put on the air.
//!
//! A relay is opened on a plan and a session or a set of credentials, and released with
//! [`pamoja_lorawan_relay_free`]. Its own device joins and sends through
//! [`pamoja_lorawan_relay_join`] and [`pamoja_lorawan_relay_send`], and every downlink goes
//! through [`pamoja_lorawan_relay_heard_in`], which acts on the relay commands in it.

use std::ptr;

use crate::lora::PamojaLoraLink;
use crate::lorawan_device::{
    heard_out, link_out, made_from, next_out, status_out, transmitted, window_in,
    PamojaLorawanDeviceSettings, PamojaLorawanEndDeviceStatus, PamojaLorawanHeard,
    PamojaLorawanTransmission,
};
use crate::lorawan_relay::PAMOJA_LORAWAN_WOR_ACK_LEN;
use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};
use pamoja_lora::region::RelayChannel;
use pamoja_lorawan::device::{DeviceError, Heard, Transmission};
use pamoja_lorawan::relay::{
    Acknowledgment, CadPeriodicity, CadToRx, Carrier, Listen, Relay, RelayConfig, RelayError,
    RelayHeard, RelaySettings, RxrDownlink, Scan, Wake, WorChannel, XtalAccuracy,
};

/// A join request follows the wake-on-radio frame; listen for it.
pub const PAMOJA_LORAWAN_WAKE_JOIN_REQUEST: u8 = 0;
/// A trusted device's uplink follows.
pub const PAMOJA_LORAWAN_WAKE_UPLINK: u8 = 1;
/// A device the relay does not trust woke it, and its network will hear of it.
pub const PAMOJA_LORAWAN_WAKE_NOTIFIED: u8 = 2;

/// The frame was for the relay's own device.
pub const PAMOJA_LORAWAN_RELAY_HEARD_DEVICE: u8 = 0;
/// It carried a downlink for an end device, to send on in its relay window.
pub const PAMOJA_LORAWAN_RELAY_HEARD_DOWNLINK: u8 = 1;
/// It carried one the relay cannot send on.
pub const PAMOJA_LORAWAN_RELAY_HEARD_UNDELIVERABLE: u8 = 2;

/// An opaque handle to a LoRaWAN relay, which owns the end device it was made from.
///
/// Release it with [`pamoja_lorawan_relay_free`].
pub struct PamojaLorawanRelay {
    relay: Relay<'static>,
    error: Option<RelayError>,
}

/// A scan for wake-on-radio frames, due next.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanScan {
    /// When to start detecting, in microseconds.
    pub start_us: u64,
    /// The frequency to listen on, in hertz.
    pub frequency_hz: u32,
    /// The data rate to listen at.
    pub data_rate: u8,
    /// `0` for the default channel, `1` for the second.
    pub channel: u8,
    /// The LoRa settings of a wake-on-radio frame, heard with inverted IQ.
    pub link: PamojaLoraLink,
    /// The longest preamble an end device sends on the channel, in symbols.
    pub preamble_symbols: u16,
}

/// What a wake-on-radio frame led a relay to do.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanWake {
    /// When to start sending the acknowledgment, in microseconds.
    pub ack_start_us: u64,
    /// How long it holds the air, in microseconds.
    pub ack_airtime_us: u64,
    /// When the uplink starts arriving, in microseconds.
    pub listen_start_us: u64,
    /// [`PAMOJA_LORAWAN_WAKE_JOIN_REQUEST`], [`PAMOJA_LORAWAN_WAKE_UPLINK`] or
    /// [`PAMOJA_LORAWAN_WAKE_NOTIFIED`].
    pub kind: u8,
    /// The device the frame came from, for an uplink or a notification.
    pub dev_addr: u32,
    /// The wake-on-radio frame counter it carried, for an uplink.
    pub wfcnt: u32,
    /// Whether the relay forwards the uplink, as TS011-1.0.1 table 16 codes it.
    pub forward: u8,
    /// `1` when there is an acknowledgment to send.
    pub has_ack: u8,
    /// The acknowledgment, seven bytes.
    pub ack_frame: [u8; PAMOJA_LORAWAN_WOR_ACK_LEN],
    /// Where it goes, in hertz.
    pub ack_frequency_hz: u32,
    /// The data rate it goes out at.
    pub ack_data_rate: u8,
    /// Its LoRa settings, sent with inverted IQ.
    pub ack_link: PamojaLoraLink,
    /// The power to ask of the radio for it, conducted, in dBm.
    pub ack_output_dbm: i8,
    /// `1` when the relay listens for the uplink behind the frame.
    pub has_listen: u8,
    /// Where the uplink arrives, in hertz.
    pub listen_frequency_hz: u32,
    /// The data rate it arrives at.
    pub listen_data_rate: u8,
    /// Its LoRa settings, heard with standard IQ.
    pub listen_link: PamojaLoraLink,
    /// The longest frame to receive; stop at anything longer.
    pub listen_max_len: usize,
}

/// A downlink for an end device, to send in its relay window.
///
/// The frame's bytes cross beside it as a [`PamojaBuffer`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanRxr {
    /// When to start sending it, in microseconds.
    pub start_us: u64,
    /// How long it holds the air, in microseconds.
    pub airtime_us: u64,
    /// The frequency, in hertz: the one the wake-on-radio frame arrived on.
    pub frequency_hz: u32,
    /// The data rate.
    pub data_rate: u8,
    /// The LoRa settings, sent with inverted IQ and a payload CRC.
    pub link: PamojaLoraLink,
    /// The power to ask of the radio, conducted, in dBm.
    pub output_dbm: i8,
}

/// Makes a relay whose own device is activated by personalization.
///
/// # Arguments
///
/// * `plan` - a published channel plan.
/// * `session` - the relay's own address and session keys.
/// * `settings` - what its radio can do.
/// * `xtal_accuracy` - how accurate the relay's crystal is, as TS011-1.0.1 table 17 codes
///   it.
/// * `cad_to_rx` - how many symbols it takes from detecting a preamble to receiving, as
///   table 15 codes it.
/// * `out_relay` - receives the relay.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, a plan that is not a
/// published one, or settings out of range.
///
/// # Safety
///
/// `plan` and `session` must be live handles, `settings` must point to a readable
/// [`PamojaLorawanDeviceSettings`], and `out_relay` to a writable pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_personalized(
    plan: *const crate::lora_region::PamojaLoraPlan,
    session: *const crate::lorawan::PamojaLorawanSession,
    settings: *const PamojaLorawanDeviceSettings,
    xtal_accuracy: u8,
    cad_to_rx: u8,
    out_relay: *mut *mut PamojaLorawanRelay,
) -> PamojaStatus {
    let (Some(session), false) = (session.as_ref(), out_relay.is_null()) else {
        set_last_error("session and out_relay must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let (plan, settings) = match made_from(plan, settings) {
        Ok(parts) => parts,
        Err(status) => return status,
    };
    match pamoja_lorawan::device::EndDevice::personalized(plan, session.session, settings) {
        Ok(device) => {
            *out_relay = Box::into_raw(Box::new(PamojaLorawanRelay {
                relay: Relay::new(device, relay_settings(xtal_accuracy, cad_to_rx)),
                error: None,
            }));
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::InvalidArgument
        }
    }
}

/// Makes a relay whose own device joins over the air.
///
/// # Arguments
///
/// * `plan` - a published channel plan.
/// * `credentials` - the relay's own identifiers and root key.
/// * `settings` - what its radio can do.
/// * `xtal_accuracy` - how accurate its crystal is, table 17.
/// * `cad_to_rx` - how long it takes to start receiving, table 15.
/// * `out_relay` - receives the relay.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// As [`pamoja_lorawan_relay_personalized`].
///
/// # Safety
///
/// `plan` and `credentials` must be live handles, `settings` must point to a readable
/// [`PamojaLorawanDeviceSettings`], and `out_relay` to a writable pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_new(
    plan: *const crate::lora_region::PamojaLoraPlan,
    credentials: *const crate::lorawan::PamojaLorawanDevice,
    settings: *const PamojaLorawanDeviceSettings,
    xtal_accuracy: u8,
    cad_to_rx: u8,
    out_relay: *mut *mut PamojaLorawanRelay,
) -> PamojaStatus {
    let (Some(credentials), false) = (credentials.as_ref(), out_relay.is_null()) else {
        set_last_error("credentials and out_relay must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let (plan, settings) = match made_from(plan, settings) {
        Ok(parts) => parts,
        Err(status) => return status,
    };
    match pamoja_lorawan::device::EndDevice::new(plan, credentials.device.clone(), settings) {
        Ok(device) => {
            *out_relay = Box::into_raw(Box::new(PamojaLorawanRelay {
                relay: Relay::new(device, relay_settings(xtal_accuracy, cad_to_rx)),
                error: None,
            }));
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::InvalidArgument
        }
    }
}

/// The relay settings two codes describe.
fn relay_settings(xtal_accuracy: u8, cad_to_rx: u8) -> RelaySettings {
    RelaySettings::new(
        XtalAccuracy::from_code(xtal_accuracy),
        CadToRx::from_code(cad_to_rx),
    )
}

/// Releases a relay and its own device.
///
/// # Arguments
///
/// * `relay` - the relay, or null.
///
/// # Safety
///
/// `relay` must come from [`pamoja_lorawan_relay_new`] or
/// [`pamoja_lorawan_relay_personalized`] and not be used afterward.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_free(relay: *mut PamojaLorawanRelay) {
    if !relay.is_null() {
        drop(Box::from_raw(relay));
    }
}

/// Makes the relay's own join request, as a device's own would be.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `dev_nonce` - a nonce this device has never used.
/// * `now_us` - the time, in microseconds.
/// * `out_frame` - receives the frame to transmit.
/// * `out_transmission` - receives where and how to transmit it, and the accept windows.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a buffer the caller must
/// release with [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_relay_error`].
///
/// # Safety
///
/// `relay` must be a live handle and the out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_join(
    relay: *mut PamojaLorawanRelay,
    dev_nonce: u16,
    now_us: u64,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
) -> PamojaStatus {
    relay_transmit(relay, out_frame, out_transmission, |device| {
        device.join(dev_nonce, now_us)
    })
}

/// Sends one of the relay's own uplinks, which also carries what it owes its network.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `port` - the application port, or 0 for a frame with no payload.
/// * `payload` - the payload, which may be null when `port` is 0.
/// * `payload_len` - its length.
/// * `confirmed` - `1` to ask the network to acknowledge it.
/// * `now_us` - the time, in microseconds.
/// * `out_frame` - receives the frame to transmit.
/// * `out_transmission` - receives where and how to transmit it, and its windows.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a buffer the caller must
/// release with [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_relay_error`].
///
/// # Safety
///
/// `relay` must be a live handle, `payload` must point to `payload_len` readable bytes when
/// that is non-zero, and the out pointers must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_lorawan_relay_send(
    relay: *mut PamojaLorawanRelay,
    port: u8,
    payload: *const u8,
    payload_len: usize,
    confirmed: u8,
    now_us: u64,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
) -> PamojaStatus {
    let payload = match read_bytes(payload, payload_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    relay_transmit(relay, out_frame, out_transmission, |device| {
        if port == 0 {
            device.send_empty(now_us)
        } else {
            device.send(port, &payload, confirmed != 0, now_us)
        }
    })
}

/// Reports what the relay's own device is doing.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `out_status` - receives the device's session, counters, channels and windows.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle or out pointer.
///
/// # Safety
///
/// `relay` must be a live handle and `out_status` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_status(
    relay: *const PamojaLorawanRelay,
    out_status: *mut PamojaLorawanEndDeviceStatus,
) -> PamojaStatus {
    let (Some(relay), false) = (relay.as_ref(), out_status.is_null()) else {
        set_last_error("relay and out_status must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_status = status_out(relay.relay.device());
    PamojaStatus::Ok
}

/// Runs a call that puts one of the relay's own frames on the air.
///
/// # Safety
///
/// `relay` must be a live handle and the out pointers writable.
unsafe fn relay_transmit(
    relay: *mut PamojaLorawanRelay,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
    call: impl FnOnce(
        &mut pamoja_lorawan::device::EndDevice<'static>,
    ) -> Result<Transmission, DeviceError>,
) -> PamojaStatus {
    let (Some(relay), false, false) = (
        relay.as_mut(),
        out_frame.is_null(),
        out_transmission.is_null(),
    ) else {
        set_last_error("relay, out_frame and out_transmission must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_frame = ptr::null_mut();
    let result = call(relay.relay.device_mut()).map_err(RelayError::Device);
    match relay.settle(result) {
        Ok(transmission) => {
            transmitted(transmission, out_frame, out_transmission);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Returns why the last relay call failed.
///
/// # Arguments
///
/// * `relay` - the relay.
///
/// # Returns
///
/// The reason, as a string the caller releases with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null when the last call succeeded.
///
/// # Safety
///
/// `relay` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_error(
    relay: *mut PamojaLorawanRelay,
) -> *mut crate::PamojaString {
    match relay.as_ref().and_then(|relay| relay.error) {
        Some(error) => crate::PamojaString::into_raw(error.to_string()),
        None => ptr::null_mut(),
    }
}

/// Starts the relay scanning, or changes what a running one scans.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `cad_periodicity` - how often it scans each channel, as TS011-1.0.1 table 18 codes it.
/// * `default_channel_index` - which of the region's wake-on-radio channels is the default
///   one.
/// * `has_second_channel` - `1` to scan a second channel as well.
/// * `second_wor_frequency_hz` - where wake-on-radio frames arrive on it.
/// * `second_ack_frequency_hz` - where the relay answers them.
/// * `second_data_rate` - the data rate of both.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_relay_error`], for a
/// reserved scan period, a channel the region does not define or the radio cannot tune, or
/// scans too close together for the data rate.
///
/// # Safety
///
/// `relay` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_start(
    relay: *mut PamojaLorawanRelay,
    cad_periodicity: u8,
    default_channel_index: u8,
    has_second_channel: u8,
    second_wor_frequency_hz: u32,
    second_ack_frequency_hz: u32,
    second_data_rate: u8,
) -> PamojaStatus {
    let Some(relay) = relay.as_mut() else {
        set_last_error("relay must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(periodicity) = CadPeriodicity::from_code(cad_periodicity) else {
        set_last_error("the scan period is reserved".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(default_channel) = relay.relay.region_channel(default_channel_index) else {
        set_last_error("the region does not define that wake-on-radio channel".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let mut config = RelayConfig::new(periodicity, default_channel);
    if has_second_channel != 0 {
        config = config.with_second_channel(RelayChannel::new(
            second_wor_frequency_hz,
            second_ack_frequency_hz,
            second_data_rate,
        ));
    }
    let started = relay.relay.start(config);
    relay.settle(started).err().unwrap_or(PamojaStatus::Ok)
}

/// Stops the relay scanning.
///
/// # Arguments
///
/// * `relay` - the relay.
///
/// # Safety
///
/// `relay` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_stop(relay: *mut PamojaLorawanRelay) {
    if let Some(relay) = relay.as_mut() {
        relay.relay.stop();
    }
}

/// Reports whether the relay is scanning.
///
/// # Arguments
///
/// * `relay` - the relay.
///
/// # Returns
///
/// `1` while it scans, and `0` while it is stopped or the handle is null.
///
/// # Safety
///
/// `relay` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_running(relay: *const PamojaLorawanRelay) -> u8 {
    match relay.as_ref() {
        Some(relay) => u8::from(relay.relay.config().is_some()),
        None => 0,
    }
}

/// Trusts an end device, as an `UpdateUplinkListReq` with the same fields does.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `index` - the entry, 0 to 15.
/// * `dev_addr` - the device's address.
/// * `root_wor_s_key` - its sixteen-byte root relay session key.
/// * `next_wfcnt` - the wake-on-radio frame counter to expect from it next.
/// * `reload_rate` - its uplinks forwarded an hour, 63 for no limit.
/// * `bucket_size` - the coded bucket size multiplier, TS011-1.0.1 table 55.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle or key, or an index past 15.
///
/// # Safety
///
/// `relay` must be a live handle and `root_wor_s_key` point to sixteen readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_trust(
    relay: *mut PamojaLorawanRelay,
    index: u8,
    dev_addr: u32,
    root_wor_s_key: *const u8,
    next_wfcnt: u32,
    reload_rate: u8,
    bucket_size: u8,
) -> PamojaStatus {
    let (Some(relay), false) = (relay.as_mut(), root_wor_s_key.is_null()) else {
        set_last_error("relay and root_wor_s_key must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let mut key = [0u8; 16];
    key.copy_from_slice(std::slice::from_raw_parts(root_wor_s_key, 16));
    if relay
        .relay
        .trust(index, dev_addr, &key, next_wfcnt, reload_rate, bucket_size)
    {
        PamojaStatus::Ok
    } else {
        set_last_error("a relay trusts sixteen devices, at indexes 0 to 15".to_owned());
        PamojaStatus::InvalidArgument
    }
}

/// Says when and where to scan next.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `now_us` - the time, in microseconds.
/// * `out_scan` - receives the scan.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_relay_error`], while the
/// relay is not running.
///
/// # Safety
///
/// `relay` must be a live handle and `out_scan` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_next_scan(
    relay: *mut PamojaLorawanRelay,
    now_us: u64,
    out_scan: *mut PamojaLorawanScan,
) -> PamojaStatus {
    let (Some(relay), false) = (relay.as_mut(), out_scan.is_null()) else {
        set_last_error("relay and out_scan must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match relay.relay.next_scan(now_us) {
        Some(scan) => {
            *out_scan = scan_out(&scan);
            relay.error = None;
            PamojaStatus::Ok
        }
        None => {
            relay.error = Some(RelayError::Stopped);
            set_last_error(RelayError::Stopped.to_string());
            PamojaStatus::Other
        }
    }
}

/// Reads a wake-on-radio frame a scan heard.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `scan` - the scan that heard it, as [`pamoja_lorawan_relay_next_scan`] gave it.
/// * `frame` - the frame.
/// * `frame_len` - its length.
/// * `rssi_dbm` - its signal strength.
/// * `snr_db` - its signal-to-noise ratio.
/// * `ended_us` - when it finished arriving, in microseconds.
/// * `out_wake` - receives what to do next.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_relay_error`]: stopped, a
/// forward still waiting, a frame that does not decode or verify, a carrier the relay
/// cannot hear, or a forwarding limit.
///
/// # Safety
///
/// `relay` must be a live handle, `scan` and `out_wake` must be readable and writable, and
/// `frame` must point to `frame_len` readable bytes.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_lorawan_relay_heard_wor(
    relay: *mut PamojaLorawanRelay,
    scan: *const PamojaLorawanScan,
    frame: *const u8,
    frame_len: usize,
    rssi_dbm: i16,
    snr_db: i8,
    ended_us: u64,
    out_wake: *mut PamojaLorawanWake,
) -> PamojaStatus {
    let (Some(relay), Some(scan), false) = (relay.as_mut(), scan.as_ref(), out_wake.is_null())
    else {
        set_last_error("relay, scan and out_wake must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let scan = match scan_in(scan) {
        Some(scan) => scan,
        None => {
            set_last_error("the scan names a data rate the region does not define".to_owned());
            return PamojaStatus::InvalidArgument;
        }
    };
    let heard = relay
        .relay
        .heard_wor(&scan, &frame, rssi_dbm, snr_db, ended_us);
    match relay.settle(heard) {
        Ok(wake) => {
            *out_wake = wake_out(&wake, &scan);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Reads the uplink a wake-on-radio frame announced, and holds it to forward.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `frame` - the uplink.
/// * `frame_len` - its length.
/// * `rssi_dbm` - its signal strength.
/// * `snr_db` - its signal-to-noise ratio.
/// * `ended_us` - when it finished arriving, in microseconds.
/// * `out_due_us` - receives when to forward it.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_relay_error`]: no uplink
/// announced, a frame that is not the kind announced or is from another device, a join
/// request the filter drops, or a forwarding limit.
///
/// # Safety
///
/// `relay` must be a live handle, `frame` must point to `frame_len` readable bytes, and
/// `out_due_us` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_heard_uplink(
    relay: *mut PamojaLorawanRelay,
    frame: *const u8,
    frame_len: usize,
    rssi_dbm: i16,
    snr_db: i8,
    ended_us: u64,
    out_due_us: *mut u64,
) -> PamojaStatus {
    let (Some(relay), false) = (relay.as_mut(), out_due_us.is_null()) else {
        set_last_error("relay and out_due_us must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let held = relay.relay.heard_uplink(&frame, rssi_dbm, snr_db, ended_us);
    match relay.settle(held) {
        Ok(due_us) => {
            *out_due_us = due_us;
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Clears the uplink a wake-on-radio frame announced, once listening for it heard nothing.
///
/// # Arguments
///
/// * `relay` - the relay.
///
/// # Safety
///
/// `relay` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_uplink_missed(relay: *mut PamojaLorawanRelay) {
    if let Some(relay) = relay.as_mut() {
        relay.relay.uplink_missed();
    }
}

/// Sends the uplink the relay is holding, in its own uplink on port 226.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `now_us` - the time, in microseconds.
/// * `out_frame` - receives the frame to transmit.
/// * `out_transmission` - receives where and how to transmit it, and its windows.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a buffer the caller must
/// release with [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_relay_error`]: nothing
/// held, or the relay's own device not free to send yet.
///
/// # Safety
///
/// `relay` must be a live handle and the out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_forward(
    relay: *mut PamojaLorawanRelay,
    now_us: u64,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
) -> PamojaStatus {
    let (Some(relay), false, false) = (
        relay.as_mut(),
        out_frame.is_null(),
        out_transmission.is_null(),
    ) else {
        set_last_error("relay, out_frame and out_transmission must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_frame = ptr::null_mut();
    let sent = relay.relay.forward(now_us);
    match relay.settle(sent) {
        Ok(transmission) => {
            transmitted(transmission, out_frame, out_transmission);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Returns when the forwarded uplink waiting to go out is due.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `out_due_us` - receives the time.
///
/// # Returns
///
/// `1` when one is waiting, `0` when none is.
///
/// # Safety
///
/// `relay` must be a live handle and `out_due_us` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_forward_due(
    relay: *mut PamojaLorawanRelay,
    out_due_us: *mut u64,
) -> u8 {
    let (Some(relay), false) = (relay.as_ref(), out_due_us.is_null()) else {
        return 0;
    };
    match relay.relay.forward_due() {
        Some(due_us) => {
            *out_due_us = due_us;
            1
        }
        None => 0,
    }
}

/// Reads a frame the relay's own device heard, acting on the relay commands in it.
///
/// A downlink on port 226 answering a forwarded uplink becomes a frame for the end device,
/// handed back through `out_frame` with the window to send it in.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `window` - which window it arrived in, as
///   [`PAMOJA_LORAWAN_WINDOW_RX1`](crate::lorawan_device::PAMOJA_LORAWAN_WINDOW_RX1) codes
///   it.
/// * `frame` - the bytes the radio received.
/// * `frame_len` - their length.
/// * `snr_db` - the frame's signal-to-noise ratio.
/// * `out_kind` - receives [`PAMOJA_LORAWAN_RELAY_HEARD_DEVICE`],
///   [`PAMOJA_LORAWAN_RELAY_HEARD_DOWNLINK`] or
///   [`PAMOJA_LORAWAN_RELAY_HEARD_UNDELIVERABLE`].
/// * `out_heard` - receives what the relay's own device made of it.
/// * `out_payload` - receives the payload of that, as a buffer to release.
/// * `out_frame` - receives the frame to send the end device, as a buffer to release.
/// * `out_downlink` - receives when and where to send it.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_relay_error`], for
/// whatever the relay's own device refuses.
///
/// # Safety
///
/// `relay` must be a live handle, `frame` must point to `frame_len` readable bytes, and the
/// out pointers must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_lorawan_relay_heard_in(
    relay: *mut PamojaLorawanRelay,
    window: u8,
    frame: *const u8,
    frame_len: usize,
    snr_db: i8,
    out_kind: *mut u8,
    out_heard: *mut PamojaLorawanHeard,
    out_payload: *mut *mut PamojaBuffer,
    out_frame: *mut *mut PamojaBuffer,
    out_downlink: *mut PamojaLorawanRxr,
) -> PamojaStatus {
    let (Some(relay), false, false, false, false, false) = (
        relay.as_mut(),
        out_kind.is_null(),
        out_heard.is_null(),
        out_payload.is_null(),
        out_frame.is_null(),
        out_downlink.is_null(),
    ) else {
        set_last_error("relay and every out pointer must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let bytes = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let dev_addr = relay.relay.device().dev_addr().unwrap_or(0);
    let Some(window) = window_in(window) else {
        set_last_error("the window must be RX1, RX2 or RXR".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_payload = ptr::null_mut();
    *out_frame = ptr::null_mut();

    let heard = relay.relay.heard_in(window, &bytes, snr_db);
    match relay.settle(heard) {
        Ok(RelayHeard::Device(heard)) => {
            *out_kind = PAMOJA_LORAWAN_RELAY_HEARD_DEVICE;
            *out_heard = heard_out(heard, dev_addr, out_payload);
            *out_downlink = empty_rxr();
            PamojaStatus::Ok
        }
        Ok(RelayHeard::Downlink { delivery, downlink }) => {
            *out_kind = PAMOJA_LORAWAN_RELAY_HEARD_DOWNLINK;
            *out_heard = heard_out(Heard::Data(delivery), dev_addr, out_payload);
            *out_frame = PamojaBuffer::into_raw(downlink.frame().to_vec());
            *out_downlink = rxr_out(&downlink);
            PamojaStatus::Ok
        }
        Ok(RelayHeard::Undeliverable { delivery, reason }) => {
            *out_kind = PAMOJA_LORAWAN_RELAY_HEARD_UNDELIVERABLE;
            *out_heard = heard_out(Heard::Data(delivery), dev_addr, out_payload);
            *out_downlink = empty_rxr();
            relay.error = Some(reason);
            set_last_error(reason.to_string());
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Says what comes next once the relay's own receive windows closed with nothing in them.
///
/// # Arguments
///
/// * `relay` - the relay.
/// * `now_us` - the time the second window closed, in microseconds.
/// * `out_next` - receives the outcome, as
///   [`PAMOJA_LORAWAN_NEXT_REPEAT`](crate::lorawan_device::PAMOJA_LORAWAN_NEXT_REPEAT) codes
///   it.
/// * `out_not_before_us` - receives the earliest time to send again, where one applies.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_relay_error`], when no
/// transmission waits on its windows.
///
/// # Safety
///
/// `relay` must be a live handle and the out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_relay_nothing_heard(
    relay: *mut PamojaLorawanRelay,
    now_us: u64,
    out_next: *mut u8,
    out_not_before_us: *mut u64,
) -> PamojaStatus {
    let (Some(relay), false, false) = (
        relay.as_mut(),
        out_next.is_null(),
        out_not_before_us.is_null(),
    ) else {
        set_last_error("relay, out_next and out_not_before_us must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let next = relay.relay.nothing_heard(now_us);
    match relay.settle(next) {
        Ok(next) => {
            let (code, not_before_us) = next_out(next);
            *out_next = code;
            *out_not_before_us = not_before_us;
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

impl PamojaLorawanRelay {
    /// Keeps the reason a call failed, and maps it onto a status.
    fn settle<T>(&mut self, result: Result<T, RelayError>) -> Result<T, PamojaStatus> {
        match result {
            Ok(value) => {
                self.error = None;
                Ok(value)
            }
            Err(error) => {
                self.error = Some(error);
                set_last_error(error.to_string());
                Err(PamojaStatus::Other)
            }
        }
    }
}

/// A scan, as C sees it.
fn scan_out(scan: &Scan) -> PamojaLorawanScan {
    PamojaLorawanScan {
        start_us: scan.start_us,
        frequency_hz: scan.carrier.frequency_hz,
        data_rate: scan.carrier.data_rate,
        channel: match scan.channel {
            WorChannel::Default => 0,
            WorChannel::Second => 1,
        },
        link: link_out(scan.link),
        preamble_symbols: scan.preamble_symbols,
    }
}

/// A scan, as the relay reads it back.
#[allow(clippy::unnecessary_wraps)]
fn scan_in(scan: &PamojaLorawanScan) -> Option<Scan> {
    Some(Scan {
        start_us: scan.start_us,
        channel: match scan.channel {
            0 => WorChannel::Default,
            1 => WorChannel::Second,
            _ => return None,
        },
        carrier: Carrier::new(scan.frequency_hz, scan.data_rate),
        link: crate::lora::settings(scan.link),
        preamble_symbols: scan.preamble_symbols,
    })
}

/// What a wake-on-radio frame led to, as C sees it.
fn wake_out(wake: &Wake, scan: &Scan) -> PamojaLorawanWake {
    let empty_link = link_out(scan.link);
    let mut out = PamojaLorawanWake {
        ack_start_us: 0,
        ack_airtime_us: 0,
        listen_start_us: 0,
        kind: PAMOJA_LORAWAN_WAKE_JOIN_REQUEST,
        dev_addr: 0,
        wfcnt: 0,
        forward: 0,
        has_ack: 0,
        ack_frame: [0; PAMOJA_LORAWAN_WOR_ACK_LEN],
        ack_frequency_hz: 0,
        ack_data_rate: 0,
        ack_link: empty_link,
        ack_output_dbm: 0,
        has_listen: 0,
        listen_frequency_hz: 0,
        listen_data_rate: 0,
        listen_link: empty_link,
        listen_max_len: 0,
    };
    match wake {
        Wake::JoinRequest { listen } => {
            out.kind = PAMOJA_LORAWAN_WAKE_JOIN_REQUEST;
            listen_out(&mut out, listen);
        }
        Wake::Uplink {
            dev_addr,
            wfcnt,
            forward,
            acknowledgment,
            listen,
        } => {
            out.kind = PAMOJA_LORAWAN_WAKE_UPLINK;
            out.dev_addr = *dev_addr;
            out.wfcnt = *wfcnt;
            out.forward = forward.code();
            if let Some(acknowledgment) = acknowledgment {
                ack_out(&mut out, acknowledgment);
            }
            if let Some(listen) = listen {
                listen_out(&mut out, listen);
            }
        }
        Wake::Notified { dev_addr } => {
            out.kind = PAMOJA_LORAWAN_WAKE_NOTIFIED;
            out.dev_addr = *dev_addr;
        }
    }
    out
}

/// Fills in the acknowledgment a relay answers with.
fn ack_out(out: &mut PamojaLorawanWake, acknowledgment: &Acknowledgment) {
    out.has_ack = 1;
    out.ack_frame = acknowledgment.frame;
    out.ack_start_us = acknowledgment.start_us;
    out.ack_airtime_us = acknowledgment.airtime_us;
    out.ack_frequency_hz = acknowledgment.carrier.frequency_hz;
    out.ack_data_rate = acknowledgment.carrier.data_rate;
    out.ack_link = link_out(acknowledgment.link);
    out.ack_output_dbm = acknowledgment.output_dbm;
}

/// Fills in where the uplink behind a frame arrives.
fn listen_out(out: &mut PamojaLorawanWake, listen: &Listen) {
    out.has_listen = 1;
    out.listen_start_us = listen.start_us;
    out.listen_frequency_hz = listen.carrier.frequency_hz;
    out.listen_data_rate = listen.carrier.data_rate;
    out.listen_link = link_out(listen.link);
    out.listen_max_len = listen.max_len;
}

/// A downlink for an end device, as C sees it.
fn rxr_out(downlink: &RxrDownlink) -> PamojaLorawanRxr {
    PamojaLorawanRxr {
        start_us: downlink.start_us,
        airtime_us: downlink.airtime_us,
        frequency_hz: downlink.carrier.frequency_hz,
        data_rate: downlink.carrier.data_rate,
        link: link_out(downlink.link),
        output_dbm: downlink.output_dbm,
    }
}

/// The window field of a frame that carries none.
fn empty_rxr() -> PamojaLorawanRxr {
    PamojaLorawanRxr {
        start_us: 0,
        airtime_us: 0,
        frequency_hz: 0,
        data_rate: 0,
        link: link_out(pamoja_lora::LinkSettings::new(7, 125_000)),
        output_dbm: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pamoja_lorawan::relay::{root_wor_s_key, wor_uplink, WorKeys, LA_FPORT_RELAY};
    use pamoja_lorawan::{Downlink, Session, Uplink};

    use crate::lora_region::{
        pamoja_lora_plan_for_region, PamojaLoraPlan, PAMOJA_LORA_REGION_EU868,
    };
    use crate::lorawan::{pamoja_lorawan_session_new, PamojaLorawanSession};
    use crate::lorawan_device::{pamoja_lorawan_device_settings, PAMOJA_LORAWAN_WINDOW_RX1};

    /// The bytes of a buffer the boundary handed back.
    unsafe fn buffer_bytes(buffer: *const PamojaBuffer) -> Vec<u8> {
        let len = crate::pamoja_buffer_len(buffer);
        std::slice::from_raw_parts(crate::pamoja_buffer_data(buffer), len).to_vec()
    }

    const RELAY_ADDR: u32 = 0x2601_0001;
    const SENSOR_ADDR: u32 = 0x2601_1BDA;
    const SENSOR_NWK_S_KEY: [u8; 16] = [0x2B; 16];

    /// The region's plan, the relay's session, and a relay handle built on both.
    unsafe fn relay() -> (
        *mut PamojaLoraPlan,
        *mut PamojaLorawanSession,
        *mut PamojaLorawanRelay,
    ) {
        let mut plan: *mut PamojaLoraPlan = ptr::null_mut();
        assert_eq!(
            pamoja_lora_plan_for_region(PAMOJA_LORA_REGION_EU868, &mut plan),
            PamojaStatus::Ok
        );
        let mut session: *mut PamojaLorawanSession = ptr::null_mut();
        assert_eq!(
            pamoja_lorawan_session_new(
                RELAY_ADDR,
                [0x11; 16].as_ptr(),
                16,
                [0x22; 16].as_ptr(),
                16,
                &mut session
            ),
            PamojaStatus::Ok
        );
        let mut settings = core::mem::zeroed::<PamojaLorawanDeviceSettings>();
        assert_eq!(
            pamoja_lorawan_device_settings(2, 14, &mut settings),
            PamojaStatus::Ok
        );
        settings.lowest_hz = 863_000_000;
        settings.highest_hz = 870_000_000;
        let mut relay: *mut PamojaLorawanRelay = ptr::null_mut();
        assert_eq!(
            pamoja_lorawan_relay_personalized(plan, session, &settings, 0, 0, &mut relay),
            PamojaStatus::Ok
        );
        assert_eq!(
            pamoja_lorawan_relay_start(relay, 0, 0, 0, 0, 0, 0),
            PamojaStatus::Ok
        );
        let root = root_wor_s_key(&SENSOR_NWK_S_KEY);
        assert_eq!(
            pamoja_lorawan_relay_trust(relay, 0, SENSOR_ADDR, root.as_ptr(), 0, 63, 0),
            PamojaStatus::Ok
        );
        (plan, session, relay)
    }

    #[test]
    fn a_relay_scans_answers_and_forwards_across_the_boundary() {
        unsafe {
            let (plan, session, relay) = relay();

            // The first scan is on the region's first wake-on-radio channel.
            let mut scan = PamojaLorawanScan {
                start_us: 0,
                frequency_hz: 0,
                data_rate: 0,
                channel: 0,
                link: crate::lorawan_device::link_out(pamoja_lora::LinkSettings::new(7, 125_000)),
                preamble_symbols: 0,
            };
            assert_eq!(
                pamoja_lorawan_relay_next_scan(relay, 0, &mut scan),
                PamojaStatus::Ok
            );
            assert_eq!(scan.frequency_hz, 865_100_000);
            assert_eq!(scan.data_rate, 3);

            // A trusted sensor wakes it, and the relay answers and listens.
            let sensor = Session::new(SENSOR_ADDR, SENSOR_NWK_S_KEY, [0x99; 16]);
            let keys = WorKeys::derive(&root_wor_s_key(&SENSOR_NWK_S_KEY), SENSOR_ADDR);
            let uplink = pamoja_lorawan::relay::Carrier::new(868_100_000, 5);
            let wor = wor_uplink(
                &keys,
                SENSOR_ADDR,
                0,
                uplink,
                pamoja_lorawan::relay::Carrier::new(scan.frequency_hz, scan.data_rate),
            )
            .expect("a wake-on-radio frame");

            let mut wake = core::mem::zeroed::<PamojaLorawanWake>();
            assert_eq!(
                pamoja_lorawan_relay_heard_wor(
                    relay,
                    &scan,
                    wor.as_ptr(),
                    wor.len(),
                    -90,
                    4,
                    scan.start_us + 900_000,
                    &mut wake,
                ),
                PamojaStatus::Ok
            );
            assert_eq!(wake.kind, PAMOJA_LORAWAN_WAKE_UPLINK);
            assert_eq!(wake.dev_addr, SENSOR_ADDR);
            assert_eq!(wake.has_ack, 1);
            assert_eq!(wake.ack_frequency_hz, 865_300_000);
            assert_eq!(wake.has_listen, 1);
            assert_eq!(wake.listen_frequency_hz, 868_100_000);

            // The uplink behind it is forwarded to the network on port 226.
            let frame = sensor
                .encode_uplink(&Uplink::new(0, 1, b"21.5"))
                .expect("an uplink");
            let mut due_us = 0u64;
            assert_eq!(
                pamoja_lorawan_relay_heard_uplink(
                    relay,
                    frame.as_bytes().as_ptr(),
                    frame.as_bytes().len(),
                    -88,
                    6,
                    wake.listen_start_us + 60_000,
                    &mut due_us,
                ),
                PamojaStatus::Ok
            );
            let mut waiting = 0u64;
            assert_eq!(pamoja_lorawan_relay_forward_due(relay, &mut waiting), 1);
            assert_eq!(waiting, due_us);

            let mut forwarded: *mut PamojaBuffer = ptr::null_mut();
            let mut transmission = core::mem::zeroed::<PamojaLorawanTransmission>();
            assert_eq!(
                pamoja_lorawan_relay_forward(relay, due_us, &mut forwarded, &mut transmission),
                PamojaStatus::Ok
            );
            let sent = buffer_bytes(forwarded);
            crate::pamoja_buffer_free(forwarded);
            let relay_session = Session::new(RELAY_ADDR, [0x11; 16], [0x22; 16]);
            let read = relay_session.decode(&sent, 0).expect("it decodes");
            assert_eq!(read.fport(), Some(LA_FPORT_RELAY));

            // The network answers the sensor through the relay, which sends it on.
            let answer = sensor
                .encode_downlink(&Downlink::new(0, 1, b"ok"))
                .expect("a downlink");
            let reply = relay_session
                .encode_downlink(&Downlink::new(0, LA_FPORT_RELAY, answer.as_bytes()))
                .expect("a downlink");
            let mut kind = 0u8;
            let mut heard = core::mem::zeroed::<PamojaLorawanHeard>();
            let mut payload: *mut PamojaBuffer = ptr::null_mut();
            let mut out_frame: *mut PamojaBuffer = ptr::null_mut();
            let mut downlink = core::mem::zeroed::<PamojaLorawanRxr>();
            assert_eq!(
                pamoja_lorawan_relay_heard_in(
                    relay,
                    PAMOJA_LORAWAN_WINDOW_RX1,
                    reply.as_bytes().as_ptr(),
                    reply.as_bytes().len(),
                    7,
                    &mut kind,
                    &mut heard,
                    &mut payload,
                    &mut out_frame,
                    &mut downlink,
                ),
                PamojaStatus::Ok
            );
            assert_eq!(kind, PAMOJA_LORAWAN_RELAY_HEARD_DOWNLINK);
            assert_eq!(downlink.frequency_hz, 865_100_000);
            assert_eq!(buffer_bytes(out_frame), answer.as_bytes());
            crate::pamoja_buffer_free(payload);
            crate::pamoja_buffer_free(out_frame);

            pamoja_lorawan_relay_free(relay);
            crate::lorawan::pamoja_lorawan_session_free(session);
            crate::lora_region::pamoja_lora_plan_free(plan);
        }
    }

    #[test]
    fn a_stopped_relay_says_so_and_a_bad_channel_is_refused() {
        unsafe {
            let (plan, session, relay) = relay();
            pamoja_lorawan_relay_stop(relay);
            let mut scan = core::mem::zeroed::<PamojaLorawanScan>();
            assert_eq!(
                pamoja_lorawan_relay_next_scan(relay, 0, &mut scan),
                PamojaStatus::Other
            );
            let reason = pamoja_lorawan_relay_error(relay);
            assert!(!reason.is_null());
            crate::pamoja_string_free(reason);

            assert_eq!(
                pamoja_lorawan_relay_start(relay, 7, 0, 0, 0, 0, 0),
                PamojaStatus::InvalidArgument,
                "a reserved scan period",
            );
            assert_eq!(
                pamoja_lorawan_relay_start(relay, 0, 9, 0, 0, 0, 0),
                PamojaStatus::InvalidArgument,
                "a channel the region does not define",
            );

            pamoja_lorawan_relay_free(relay);
            crate::lorawan::pamoja_lorawan_session_free(session);
            crate::lora_region::pamoja_lora_plan_free(plan);
        }
    }
}
