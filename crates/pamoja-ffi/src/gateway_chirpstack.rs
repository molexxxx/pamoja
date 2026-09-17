//! The C ABI for the uplink events a ChirpStack network server publishes.
//!
//! ChirpStack publishes every uplink it deduplicates to an MQTT topic,
//! `application/<id>/device/<dev_eui>/event/up`, as the JSON form of its `UplinkEvent`
//! message. [`pamoja_chirpstack_uplink_parse`] reads one into a handle, and the calls here
//! read the device, the counter, the payload and every gateway that heard it back out.
//!
//! Fields protobuf's JSON mapping leaves out when they hold their default read as that default:
//! a frame counter of zero, ADR off, unconfirmed.

use std::ffi::c_char;
use std::ptr;

use pamoja_gateway::chirpstack::{uplink_topic, UplinkEvent};

use crate::{read_bytes, read_str, set_last_error, PamojaBuffer, PamojaStatus, PamojaString};

/// The scalar fields of an uplink event, read in one call.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaChirpstackUplinkSummary {
    /// The device's address, meaningful when `has_dev_addr` is `1`.
    pub dev_addr: u32,
    /// The uplink frame counter.
    pub fcnt: u32,
    /// The carrier it was heard on, in hertz, meaningful when `has_frequency` is `1`.
    pub frequency_hz: u32,
    /// How many gateways heard it.
    pub reception_count: u32,
    /// The device EUI, most-significant byte first.
    pub dev_eui: [u8; 8],
    /// `1` when the event names the device's address.
    pub has_dev_addr: u8,
    /// `1` when the device had adaptive data rate on.
    pub adr: u8,
    /// The data rate, as the region numbers them.
    pub data_rate: u8,
    /// `1` when the frame carried an application port.
    pub has_fport: u8,
    /// The application port.
    pub fport: u8,
    /// `1` for a confirmed uplink.
    pub confirmed: u8,
    /// `1` when the event names the carrier.
    pub has_frequency: u8,
}

/// One gateway that heard an uplink.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaChirpstackReception {
    /// The received signal strength, in dBm.
    pub rssi_dbm: i32,
    /// The signal-to-noise ratio, in dB.
    pub snr_db: f32,
    /// The gateway's EUI, most-significant byte first.
    pub gateway: [u8; 8],
}

/// An opaque handle to a parsed uplink event.
///
/// Release it with [`pamoja_chirpstack_uplink_free`].
pub struct PamojaChirpstackUplink {
    event: UplinkEvent,
}

/// Reads an uplink event from the JSON ChirpStack published.
///
/// # Arguments
///
/// * `text` - the MQTT message's payload, UTF-8 JSON.
/// * `text_len` - its length.
/// * `out_uplink` - receives the event.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_uplink` set to a handle the caller must release
/// with [`pamoja_chirpstack_uplink_free`].
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, and [`PamojaStatus::Codec`]
/// for text that is not a JSON object, an event with no device EUI, or a field that does not
/// read as what it should, with the field named by `pamoja_last_error`.
///
/// # Safety
///
/// `text` must point to `text_len` readable bytes when that is non-zero, and `out_uplink` must
/// be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_chirpstack_uplink_parse(
    text: *const u8,
    text_len: usize,
    out_uplink: *mut *mut PamojaChirpstackUplink,
) -> PamojaStatus {
    if out_uplink.is_null() {
        set_last_error("out_uplink must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_uplink;
    *slot = ptr::null_mut();
    let bytes = match read_bytes(text, text_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        set_last_error("the event is not UTF-8".to_owned());
        return PamojaStatus::Codec;
    };
    match UplinkEvent::from_json(text) {
        Ok(event) => {
            *slot = Box::into_raw(Box::new(PamojaChirpstackUplink { event }));
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Codec
        }
    }
}

/// Releases an uplink event handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `uplink` must be a handle from [`pamoja_chirpstack_uplink_parse`] that has not already been
/// freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_chirpstack_uplink_free(uplink: *mut PamojaChirpstackUplink) {
    if !uplink.is_null() {
        drop(Box::from_raw(uplink));
    }
}

/// Reads the scalar fields of an uplink event.
///
/// # Arguments
///
/// * `uplink` - the event.
/// * `out_summary` - receives the fields.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `uplink` must be a live handle and `out_summary` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_chirpstack_uplink_summary(
    uplink: *const PamojaChirpstackUplink,
    out_summary: *mut PamojaChirpstackUplinkSummary,
) -> PamojaStatus {
    let (Some(uplink), false) = (uplink.as_ref(), out_summary.is_null()) else {
        set_last_error("uplink and out_summary must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let event = &uplink.event;
    *out_summary = PamojaChirpstackUplinkSummary {
        dev_addr: event.dev_addr.unwrap_or(0),
        fcnt: event.fcnt,
        frequency_hz: event.frequency_hz.unwrap_or(0),
        reception_count: event.receptions.len() as u32,
        dev_eui: event.dev_eui.bytes(),
        has_dev_addr: u8::from(event.dev_addr.is_some()),
        adr: u8::from(event.adr),
        data_rate: event.data_rate,
        has_fport: u8::from(event.fport.is_some()),
        fport: event.fport.unwrap_or(0),
        confirmed: u8::from(event.confirmed),
        has_frequency: u8::from(event.frequency_hz.is_some()),
    };
    PamojaStatus::Ok
}

/// Which text field of an event to read.
fn text_of(event: &UplinkEvent, field: u8) -> Option<Option<&str>> {
    match field {
        PAMOJA_CHIRPSTACK_DEDUPLICATION_ID => Some(Some(&event.deduplication_id)),
        PAMOJA_CHIRPSTACK_TIME => Some(event.time.as_deref()),
        PAMOJA_CHIRPSTACK_APPLICATION_ID => Some(Some(&event.application_id)),
        PAMOJA_CHIRPSTACK_DEVICE_NAME => Some(Some(&event.device_name)),
        _ => None,
    }
}

/// The identifier ChirpStack gave the uplink once it deduplicated the gateways' copies.
pub const PAMOJA_CHIRPSTACK_DEDUPLICATION_ID: u8 = 0;
/// When the uplink was received, as ChirpStack wrote it.
pub const PAMOJA_CHIRPSTACK_TIME: u8 = 1;
/// The application the device belongs to.
pub const PAMOJA_CHIRPSTACK_APPLICATION_ID: u8 = 2;
/// The name the device was given in ChirpStack.
pub const PAMOJA_CHIRPSTACK_DEVICE_NAME: u8 = 3;

/// Reads one of an uplink event's text fields.
///
/// # Arguments
///
/// * `uplink` - the event.
/// * `field` - one of the `PAMOJA_CHIRPSTACK_*` field constants.
///
/// # Returns
///
/// The text, which the caller releases with [`pamoja_string_free`](crate::pamoja_string_free);
/// an empty string for a field the event left out; or null for a time the event left out, a
/// null handle, or a field constant that names nothing.
///
/// # Safety
///
/// `uplink` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_chirpstack_uplink_text(
    uplink: *const PamojaChirpstackUplink,
    field: u8,
) -> *mut PamojaString {
    let Some(uplink) = uplink.as_ref() else {
        set_last_error("uplink must not be null".to_owned());
        return ptr::null_mut();
    };
    match text_of(&uplink.event, field) {
        Some(Some(text)) => PamojaString::into_raw(text.to_owned()),
        Some(None) => ptr::null_mut(),
        None => {
            set_last_error(format!("{field} names no text field of an uplink event"));
            ptr::null_mut()
        }
    }
}

/// Copies out the application payload an uplink carried, decoded from base64.
///
/// # Arguments
///
/// * `uplink` - the event.
/// * `out_data` - receives the payload, empty when the event carried none.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_data` set to a buffer the caller releases with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `uplink` must be a live handle and `out_data` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_chirpstack_uplink_data(
    uplink: *const PamojaChirpstackUplink,
    out_data: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let (Some(uplink), false) = (uplink.as_ref(), out_data.is_null()) else {
        set_last_error("uplink and out_data must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_data = PamojaBuffer::into_raw(uplink.event.data.clone());
    PamojaStatus::Ok
}

/// Reads one gateway that heard an uplink.
///
/// # Arguments
///
/// * `uplink` - the event.
/// * `index` - the reception's position, below the count the summary reports.
/// * `out_reception` - receives the reception.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null or `index` is past the end.
///
/// # Safety
///
/// `uplink` must be a live handle and `out_reception` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_chirpstack_uplink_reception(
    uplink: *const PamojaChirpstackUplink,
    index: u32,
    out_reception: *mut PamojaChirpstackReception,
) -> PamojaStatus {
    let (Some(uplink), false) = (uplink.as_ref(), out_reception.is_null()) else {
        set_last_error("uplink and out_reception must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(reception) = uplink.event.receptions.get(index as usize) else {
        set_last_error(format!("the event has no reception {index}"));
        return PamojaStatus::InvalidArgument;
    };
    *out_reception = PamojaChirpstackReception {
        rssi_dbm: reception.rssi_dbm,
        snr_db: reception.snr_db,
        gateway: reception.gateway.bytes(),
    };
    PamojaStatus::Ok
}

/// Finds the gateway that heard an uplink best.
///
/// # Arguments
///
/// * `uplink` - the event.
/// * `out_index` - receives the position of the reception with the highest signal-to-noise
///   ratio.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null or the event named no
/// gateway.
///
/// # Safety
///
/// `uplink` must be a live handle and `out_index` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_chirpstack_uplink_best_reception(
    uplink: *const PamojaChirpstackUplink,
    out_index: *mut u32,
) -> PamojaStatus {
    let (Some(uplink), false) = (uplink.as_ref(), out_index.is_null()) else {
        set_last_error("uplink and out_index must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let event = &uplink.event;
    let Some(best) = event.best_reception() else {
        set_last_error("the event named no gateway".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let index = event
        .receptions
        .iter()
        .position(|reception| ptr::eq(reception, best))
        .unwrap_or(0);
    *out_index = index as u32;
    PamojaStatus::Ok
}

/// Builds the MQTT topic an application's uplink events are published on, with a wildcard in
/// place of the device.
///
/// # Arguments
///
/// * `application_id` - the application's identifier, as ChirpStack shows it, or `+` for every
///   application's events.
///
/// # Returns
///
/// The topic, `application/<id>/device/+/event/up`, which the caller releases with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null for a null or non-UTF-8 argument.
///
/// # Safety
///
/// `application_id` must be a NUL-terminated string or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_chirpstack_uplink_topic(
    application_id: *const c_char,
) -> *mut PamojaString {
    let Some(application_id) = read_str(application_id, "application_id") else {
        return ptr::null_mut();
    };
    PamojaString::into_raw(uplink_topic(application_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len, pamoja_string_free};

    const TWO: &str = r#"{
        "deduplicationId": "3ac7e3c4-4401-4b8d-9386-a5c902f9202d",
        "deviceInfo": { "applicationId": "17c8", "deviceName": "Test device", "devEui": "0101010101010101" },
        "devAddr": "00189440", "dr": 1, "fPort": 1, "fCnt": 7, "confirmed": true, "data": "qg==",
        "rxInfo": [
            { "gatewayId": "0202020202020202", "rssi": -110, "snr": -4.25 },
            { "gatewayId": "0016c001f153a14c", "rssi": -36, "snr": 10.5 }
        ],
        "txInfo": { "frequency": 867100000 }
    }"#;

    #[test]
    fn an_event_reads_back_across_the_boundary() {
        unsafe {
            let mut uplink = ptr::null_mut();
            assert_eq!(
                pamoja_chirpstack_uplink_parse(TWO.as_ptr(), TWO.len(), &mut uplink),
                PamojaStatus::Ok
            );
            let mut summary = std::mem::zeroed::<PamojaChirpstackUplinkSummary>();
            assert_eq!(
                pamoja_chirpstack_uplink_summary(uplink, &mut summary),
                PamojaStatus::Ok
            );
            assert_eq!(summary.dev_eui, [1; 8]);
            assert_eq!((summary.has_dev_addr, summary.dev_addr), (1, 0x0018_9440));
            assert_eq!((summary.fcnt, summary.confirmed, summary.adr), (7, 1, 0));
            assert_eq!(
                (summary.has_fport, summary.fport, summary.data_rate),
                (1, 1, 1)
            );
            assert_eq!(
                (summary.has_frequency, summary.frequency_hz),
                (1, 867_100_000)
            );
            assert_eq!(summary.reception_count, 2);

            let name = pamoja_chirpstack_uplink_text(uplink, PAMOJA_CHIRPSTACK_DEVICE_NAME);
            assert!(!name.is_null());
            pamoja_string_free(name);
            assert!(pamoja_chirpstack_uplink_text(uplink, PAMOJA_CHIRPSTACK_TIME).is_null());
            assert!(pamoja_chirpstack_uplink_text(uplink, 9).is_null());

            let mut data = ptr::null_mut();
            assert_eq!(
                pamoja_chirpstack_uplink_data(uplink, &mut data),
                PamojaStatus::Ok
            );
            assert_eq!(
                std::slice::from_raw_parts(pamoja_buffer_data(data), pamoja_buffer_len(data)),
                [0xAA]
            );
            pamoja_buffer_free(data);

            let mut best = 9;
            assert_eq!(
                pamoja_chirpstack_uplink_best_reception(uplink, &mut best),
                PamojaStatus::Ok
            );
            assert_eq!(best, 1);
            let mut reception = std::mem::zeroed::<PamojaChirpstackReception>();
            assert_eq!(
                pamoja_chirpstack_uplink_reception(uplink, best, &mut reception),
                PamojaStatus::Ok
            );
            assert_eq!((reception.rssi_dbm, reception.snr_db), (-36, 10.5));
            assert_eq!(
                reception.gateway,
                [0x00, 0x16, 0xC0, 0x01, 0xF1, 0x53, 0xA1, 0x4C]
            );
            assert_eq!(
                pamoja_chirpstack_uplink_reception(uplink, 2, &mut reception),
                PamojaStatus::InvalidArgument
            );
            pamoja_chirpstack_uplink_free(uplink);

            let broken = r#"{ "deviceInfo": {} }"#;
            assert_eq!(
                pamoja_chirpstack_uplink_parse(broken.as_ptr(), broken.len(), &mut uplink),
                PamojaStatus::Codec
            );
            assert!(uplink.is_null());
        }
    }
}
