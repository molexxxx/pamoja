//! The C ABI for LoRaWAN 1.0.x MAC framing.
//!
//! These functions wrap [`pamoja_lorawan`] for callers that reach the SDK through
//! the flat C boundary: the secured frame a long-range node puts on the air, and
//! the over-the-air activation that hands it its session keys.
//!
//! A session and a device hold key material, so they cross as opaque handles and
//! the keys never leave the library once set. An encoded frame comes back as a
//! [`PamojaBuffer`], and a decoded one as a handle carrying its recovered payload.
//! The header flags a sender chooses are only scalars, so they cross by value as
//! [`PamojaLorawanFlags`].

use std::ptr;

use pamoja_lorawan::{
    Device, Direction, Downlink, FrameHeader, JoinAccept, JoinGrant, JoinRequest, LorawanError,
    MessageType, PhyPayload, RxData, Session, Uplink,
};

use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// The largest LoRaWAN frame, in bytes, this build accepts.
pub const PAMOJA_LORAWAN_FRAME_MAX: usize = 256;

/// The largest application payload, in bytes, a single frame can carry.
pub const PAMOJA_LORAWAN_PAYLOAD_MAX: usize = 243;

/// The length of a LoRaWAN key, in bytes.
pub const PAMOJA_LORAWAN_KEY_LEN: usize = 16;

/// The length of a LoRaWAN EUI, in bytes.
pub const PAMOJA_LORAWAN_EUI_LEN: usize = 8;

/// The direction a frame travelled, which its MIC and encryption both fold in.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaLorawanDirection {
    /// From an end device up to the network.
    Uplink = 0,
    /// From the network down to an end device.
    Downlink = 1,
}

/// The header flags a sender sets on a data frame.
///
/// Each is `1` for on and `0` for off. `fpending` applies to a downlink only and
/// is ignored when encoding an uplink.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanFlags {
    /// Ask the far end to acknowledge this frame.
    pub confirmed: u8,
    /// Mark the frame as taking part in adaptive data rate.
    pub adr: u8,
    /// Acknowledge the last confirmed frame from the far end.
    pub ack: u8,
    /// Tell the device more downlink data is waiting.
    pub fpending: u8,
}

/// An opaque handle to an activated LoRaWAN session.
///
/// Holds a device address and the two session keys, and never hands them back.
/// Release it with [`pamoja_lorawan_session_free`].
pub struct PamojaLorawanSession {
    session: Session,
}

/// An opaque handle to the root credentials of a device.
///
/// Holds the EUIs and the application key that over-the-air activation is built
/// on. Release it with [`pamoja_lorawan_device_free`].
pub struct PamojaLorawanDevice {
    device: Device,
}

/// An opaque handle to an accepted join.
///
/// Read the network settings off it, then take the session it grants with
/// [`pamoja_lorawan_join_accept_session`]. Release it with
/// [`pamoja_lorawan_join_accept_free`].
pub struct PamojaLorawanJoinAccept {
    accept: JoinAccept,
}

/// An opaque handle to a decoded data frame.
///
/// What a successful [`pamoja_lorawan_session_decode`] produces: the header fields
/// and the decrypted payload. Release it with [`pamoja_lorawan_rx_free`].
pub struct PamojaLorawanRx {
    rx: RxData,
}

/// Creates a session from a device address and its two session keys.
///
/// # Arguments
///
/// * `dev_addr` - the device address the network assigned.
/// * `nwk_skey` - the 16-byte network session key, which authenticates frames.
/// * `nwk_skey_len` - its length, which must be [`PAMOJA_LORAWAN_KEY_LEN`].
/// * `app_skey` - the 16-byte application session key, which encrypts payloads.
/// * `app_skey_len` - its length, which must be [`PAMOJA_LORAWAN_KEY_LEN`].
/// * `out_session` - receives the new session.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_session` set to a handle the caller
/// must release with [`pamoja_lorawan_session_free`], or
/// [`PamojaStatus::InvalidArgument`] if either key is the wrong length.
///
/// # Safety
///
/// Each key pointer must point to at least its stated length in readable bytes,
/// and `out_session` must point to a writable `*mut PamojaLorawanSession`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_session_new(
    dev_addr: u32,
    nwk_skey: *const u8,
    nwk_skey_len: usize,
    app_skey: *const u8,
    app_skey_len: usize,
    out_session: *mut *mut PamojaLorawanSession,
) -> PamojaStatus {
    if out_session.is_null() {
        set_last_error("out_session must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_session;
    *slot = ptr::null_mut();

    let nwk_skey = match key(nwk_skey, nwk_skey_len, "the network session key") {
        Ok(key) => key,
        Err(status) => return status,
    };
    let app_skey = match key(app_skey, app_skey_len, "the application session key") {
        Ok(key) => key,
        Err(status) => return status,
    };

    *slot = Box::into_raw(Box::new(PamojaLorawanSession {
        session: Session::new(dev_addr, nwk_skey, app_skey),
    }));
    PamojaStatus::Ok
}

/// Returns the device address a session is bound to.
///
/// # Returns
///
/// The device address, or 0 if `session` is null.
///
/// # Safety
///
/// `session` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_session_dev_addr(
    session: *const PamojaLorawanSession,
) -> u32 {
    if session.is_null() {
        return 0;
    }
    (*session).session.dev_addr()
}

/// Encodes an uplink data frame, encrypting the payload and appending the MIC.
///
/// # Arguments
///
/// * `session` - the activated session to send from.
/// * `fcnt` - the frame counter for this uplink.
/// * `fport` - the port; `0` for MAC commands, otherwise an application port.
/// * `payload` - the application payload to carry.
/// * `payload_len` - its length.
/// * `fopts` - the frame options to carry in the header, at most 15 bytes.
/// * `fopts_len` - their length.
/// * `flags` - the header flags to set; `fpending` is ignored on an uplink.
/// * `out_frame` - receives the encoded frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a buffer the caller
/// must release with [`pamoja_buffer_free`](crate::pamoja_buffer_free), or
/// [`PamojaStatus::InvalidArgument`] if the payload and options do not fit one
/// frame.
///
/// # Safety
///
/// `payload` and `fopts` must each point to at least their stated lengths in
/// readable bytes when those lengths are non-zero, and `out_frame` must point to a
/// writable `*mut PamojaBuffer`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_lorawan_session_encode_uplink(
    session: *const PamojaLorawanSession,
    fcnt: u32,
    fport: u8,
    payload: *const u8,
    payload_len: usize,
    fopts: *const u8,
    fopts_len: usize,
    flags: PamojaLorawanFlags,
    out_frame: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    encode(
        session,
        payload,
        payload_len,
        fopts,
        fopts_len,
        out_frame,
        |session, payload, fopts| {
            let mut uplink = Uplink::new(fcnt, fport, payload).with_fopts(fopts);
            if flags.confirmed != 0 {
                uplink = uplink.confirmed();
            }
            if flags.adr != 0 {
                uplink = uplink.with_adr();
            }
            if flags.ack != 0 {
                uplink = uplink.with_ack();
            }
            session.encode_uplink(&uplink)
        },
    )
}

/// Encodes a downlink data frame, encrypting the payload and appending the MIC.
///
/// # Arguments
///
/// * `session` - the session the frame is addressed to.
/// * `fcnt` - the frame counter for this downlink.
/// * `fport` - the port; `0` for MAC commands, otherwise an application port.
/// * `payload` - the application payload to carry.
/// * `payload_len` - its length.
/// * `fopts` - the frame options to carry in the header, at most 15 bytes.
/// * `fopts_len` - their length.
/// * `flags` - the header flags to set.
/// * `out_frame` - receives the encoded frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a buffer the caller
/// must release with [`pamoja_buffer_free`](crate::pamoja_buffer_free), or
/// [`PamojaStatus::InvalidArgument`] if the payload and options do not fit one
/// frame.
///
/// # Safety
///
/// `payload` and `fopts` must each point to at least their stated lengths in
/// readable bytes when those lengths are non-zero, and `out_frame` must point to a
/// writable `*mut PamojaBuffer`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_lorawan_session_encode_downlink(
    session: *const PamojaLorawanSession,
    fcnt: u32,
    fport: u8,
    payload: *const u8,
    payload_len: usize,
    fopts: *const u8,
    fopts_len: usize,
    flags: PamojaLorawanFlags,
    out_frame: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    encode(
        session,
        payload,
        payload_len,
        fopts,
        fopts_len,
        out_frame,
        |session, payload, fopts| {
            let mut downlink = Downlink::new(fcnt, fport, payload).with_fopts(fopts);
            if flags.confirmed != 0 {
                downlink = downlink.confirmed();
            }
            if flags.adr != 0 {
                downlink = downlink.with_adr();
            }
            if flags.ack != 0 {
                downlink = downlink.with_ack();
            }
            if flags.fpending != 0 {
                downlink = downlink.with_fpending();
            }
            session.encode_downlink(&downlink)
        },
    )
}

/// Decodes a received data frame: verifies the MIC, then decrypts the payload.
///
/// # Arguments
///
/// * `session` - the session the frame belongs to.
/// * `bytes` - the frame exactly as it came off the radio.
/// * `bytes_len` - its length.
/// * `fcnt` - the full 32-bit frame counter expected for this frame; its low 16
///   bits must match the counter the frame carries.
/// * `out_rx` - receives the decoded frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_rx` set to a handle the caller must
/// release with [`pamoja_lorawan_rx_free`], [`PamojaStatus::Auth`] if the MIC does
/// not verify or the counter does not match, or [`PamojaStatus::Codec`] if the
/// frame is truncated or is not a data frame.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes when that length is
/// non-zero, and `out_rx` must point to a writable `*mut PamojaLorawanRx`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_session_decode(
    session: *const PamojaLorawanSession,
    bytes: *const u8,
    bytes_len: usize,
    fcnt: u32,
    out_rx: *mut *mut PamojaLorawanRx,
) -> PamojaStatus {
    if out_rx.is_null() {
        set_last_error("out_rx must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_rx;
    *slot = ptr::null_mut();

    if session.is_null() {
        set_last_error("session must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match (*session).session.decode(&bytes, fcnt) {
        Ok(rx) => {
            *slot = Box::into_raw(Box::new(PamojaLorawanRx { rx }));
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Releases a session handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `session` must be a handle from a call that produced one and that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_session_free(session: *mut PamojaLorawanSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// Returns the direction a decoded frame travelled.
///
/// # Returns
///
/// The direction, or [`PamojaLorawanDirection::Uplink`] if `rx` is null.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_direction(
    rx: *const PamojaLorawanRx,
) -> PamojaLorawanDirection {
    if rx.is_null() {
        return PamojaLorawanDirection::Uplink;
    }
    match (*rx).rx.direction() {
        Direction::Uplink => PamojaLorawanDirection::Uplink,
        Direction::Downlink => PamojaLorawanDirection::Downlink,
    }
}

/// Returns the device address a decoded frame carries.
///
/// # Returns
///
/// The device address, or 0 if `rx` is null.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_dev_addr(rx: *const PamojaLorawanRx) -> u32 {
    if rx.is_null() {
        return 0;
    }
    (*rx).rx.dev_addr()
}

/// Returns the low 16 bits of the frame counter a decoded frame carries.
///
/// # Returns
///
/// The counter, or 0 if `rx` is null.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_fcnt(rx: *const PamojaLorawanRx) -> u16 {
    if rx.is_null() {
        return 0;
    }
    (*rx).rx.fcnt()
}

/// Reports whether a decoded frame asks to be acknowledged.
///
/// # Returns
///
/// `true` when the frame is confirmed, or `false` if `rx` is null.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_confirmed(rx: *const PamojaLorawanRx) -> bool {
    !rx.is_null() && (*rx).rx.confirmed()
}

/// Reports whether a decoded frame takes part in adaptive data rate.
///
/// # Returns
///
/// `true` when the ADR bit is set, or `false` if `rx` is null.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_adr(rx: *const PamojaLorawanRx) -> bool {
    !rx.is_null() && (*rx).rx.adr()
}

/// Reports whether a decoded frame acknowledges the last confirmed one.
///
/// # Returns
///
/// `true` when the ACK bit is set, or `false` if `rx` is null.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_ack(rx: *const PamojaLorawanRx) -> bool {
    !rx.is_null() && (*rx).rx.ack()
}

/// Reports whether the network has more downlink data waiting.
///
/// # Returns
///
/// `true` when the frame-pending bit is set, or `false` if `rx` is null.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_fpending(rx: *const PamojaLorawanRx) -> bool {
    !rx.is_null() && (*rx).rx.fpending()
}

/// Returns the port a decoded frame was sent on.
///
/// # Arguments
///
/// * `rx` - the decoded frame.
/// * `out_fport` - receives the port.
///
/// # Returns
///
/// `true` when the frame carries a port, with `*out_fport` written, or `false`
/// for a frame that carries only frame options and so has none.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null,
/// and `out_fport` must point to a writable `uint8_t`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_fport(
    rx: *const PamojaLorawanRx,
    out_fport: *mut u8,
) -> bool {
    if rx.is_null() || out_fport.is_null() {
        return false;
    }
    match (*rx).rx.fport() {
        Some(fport) => {
            *out_fport = fport;
            true
        }
        None => false,
    }
}

/// Returns a pointer to the frame options a decoded frame carries.
///
/// Use [`pamoja_lorawan_rx_fopts_len`] for the length. The pointer is valid until
/// the frame is freed.
///
/// # Returns
///
/// A pointer to the options, or null if `rx` is null or there are none.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_fopts(rx: *const PamojaLorawanRx) -> *const u8 {
    if rx.is_null() {
        return ptr::null();
    }
    let fopts = (*rx).rx.fopts();
    if fopts.is_empty() {
        ptr::null()
    } else {
        fopts.as_ptr()
    }
}

/// Returns the length in bytes of the frame options a decoded frame carries.
///
/// # Returns
///
/// The length, or 0 if `rx` is null.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_fopts_len(rx: *const PamojaLorawanRx) -> usize {
    if rx.is_null() {
        return 0;
    }
    (*rx).rx.fopts().len()
}

/// Returns a pointer to the decrypted payload of a decoded frame.
///
/// Use [`pamoja_lorawan_rx_payload_len`] for the length. The pointer is valid
/// until the frame is freed.
///
/// # Returns
///
/// A pointer to the payload, or null if `rx` is null or the payload is empty.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_payload(rx: *const PamojaLorawanRx) -> *const u8 {
    if rx.is_null() {
        return ptr::null();
    }
    let payload = (*rx).rx.payload();
    if payload.is_empty() {
        ptr::null()
    } else {
        payload.as_ptr()
    }
}

/// Returns the length in bytes of the decrypted payload of a decoded frame.
///
/// # Returns
///
/// The payload length, or 0 if `rx` is null.
///
/// # Safety
///
/// `rx` must be a live handle from [`pamoja_lorawan_session_decode`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_payload_len(rx: *const PamojaLorawanRx) -> usize {
    if rx.is_null() {
        return 0;
    }
    (*rx).rx.payload().len()
}

/// Releases a decoded frame handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `rx` must be a handle from [`pamoja_lorawan_session_decode`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_rx_free(rx: *mut PamojaLorawanRx) {
    if !rx.is_null() {
        drop(Box::from_raw(rx));
    }
}

/// Creates a device from the root credentials over-the-air activation uses.
///
/// # Arguments
///
/// * `dev_eui` - the 8-byte device EUI.
/// * `dev_eui_len` - its length, which must be [`PAMOJA_LORAWAN_EUI_LEN`].
/// * `app_eui` - the 8-byte application (join) EUI.
/// * `app_eui_len` - its length, which must be [`PAMOJA_LORAWAN_EUI_LEN`].
/// * `app_key` - the 16-byte application key the join exchange is secured with.
/// * `app_key_len` - its length, which must be [`PAMOJA_LORAWAN_KEY_LEN`].
/// * `out_device` - receives the new device.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_device` set to a handle the caller
/// must release with [`pamoja_lorawan_device_free`], or
/// [`PamojaStatus::InvalidArgument`] if any credential is the wrong length.
///
/// # Safety
///
/// Each pointer must point to at least its stated length in readable bytes, and
/// `out_device` must point to a writable `*mut PamojaLorawanDevice`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_device_new(
    dev_eui: *const u8,
    dev_eui_len: usize,
    app_eui: *const u8,
    app_eui_len: usize,
    app_key: *const u8,
    app_key_len: usize,
    out_device: *mut *mut PamojaLorawanDevice,
) -> PamojaStatus {
    if out_device.is_null() {
        set_last_error("out_device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_device;
    *slot = ptr::null_mut();

    let dev_eui = match eui(dev_eui, dev_eui_len, "the device EUI") {
        Ok(eui) => eui,
        Err(status) => return status,
    };
    let app_eui = match eui(app_eui, app_eui_len, "the application EUI") {
        Ok(eui) => eui,
        Err(status) => return status,
    };
    let app_key = match key(app_key, app_key_len, "the application key") {
        Ok(key) => key,
        Err(status) => return status,
    };

    *slot = Box::into_raw(Box::new(PamojaLorawanDevice {
        device: Device::new(dev_eui, app_eui, app_key),
    }));
    PamojaStatus::Ok
}

/// Builds the join request a device broadcasts to activate.
///
/// # Arguments
///
/// * `device` - the device to activate.
/// * `dev_nonce` - a nonce that must never repeat for this device, since the
///   network rejects a replayed one.
/// * `out_frame` - receives the encoded join request.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a buffer the caller
/// must release with [`pamoja_buffer_free`](crate::pamoja_buffer_free), or
/// [`PamojaStatus::InvalidArgument`] if `device` is null.
///
/// # Safety
///
/// `device` must be a live handle from [`pamoja_lorawan_device_new`], or null, and
/// `out_frame` must point to a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_device_join_request(
    device: *const PamojaLorawanDevice,
    dev_nonce: u16,
    out_frame: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = ptr::null_mut();

    if device.is_null() {
        set_last_error("device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let request = (*device).device.join_request(dev_nonce);
    *slot = PamojaBuffer::into_raw(request.as_bytes().to_vec());
    PamojaStatus::Ok
}

/// Turns the join accept a network sent into the settings it grants.
///
/// # Arguments
///
/// * `device` - the device that sent the join request.
/// * `bytes` - the join accept exactly as it arrived.
/// * `bytes_len` - its length.
/// * `dev_nonce` - the nonce the matching join request carried.
/// * `out_accept` - receives the accepted join.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_accept` set to a handle the caller
/// must release with [`pamoja_lorawan_join_accept_free`],
/// [`PamojaStatus::Auth`] if the MIC does not verify, or
/// [`PamojaStatus::Codec`] if the frame is truncated or is not a join accept.
///
/// # Safety
///
/// `device` must be a live handle from [`pamoja_lorawan_device_new`], or null,
/// `bytes` must point to at least `bytes_len` readable bytes when that length is
/// non-zero, and `out_accept` must point to a writable
/// `*mut PamojaLorawanJoinAccept`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_device_accept_join(
    device: *const PamojaLorawanDevice,
    bytes: *const u8,
    bytes_len: usize,
    dev_nonce: u16,
    out_accept: *mut *mut PamojaLorawanJoinAccept,
) -> PamojaStatus {
    if out_accept.is_null() {
        set_last_error("out_accept must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_accept;
    *slot = ptr::null_mut();

    if device.is_null() {
        set_last_error("device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match (*device).device.accept_join(&bytes, dev_nonce) {
        Ok(accept) => {
            *slot = Box::into_raw(Box::new(PamojaLorawanJoinAccept { accept }));
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Releases a device handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `device` must be a handle from [`pamoja_lorawan_device_new`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_device_free(device: *mut PamojaLorawanDevice) {
    if !device.is_null() {
        drop(Box::from_raw(device));
    }
}

/// Returns the device address a join grants.
///
/// # Returns
///
/// The device address, or 0 if `accept` is null.
///
/// # Safety
///
/// `accept` must be a live handle from [`pamoja_lorawan_device_accept_join`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_accept_dev_addr(
    accept: *const PamojaLorawanJoinAccept,
) -> u32 {
    if accept.is_null() {
        return 0;
    }
    (*accept).accept.dev_addr()
}

/// Returns the identifier of the network that accepted a join.
///
/// # Returns
///
/// The network identifier, or 0 if `accept` is null.
///
/// # Safety
///
/// `accept` must be a live handle from [`pamoja_lorawan_device_accept_join`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_accept_net_id(
    accept: *const PamojaLorawanJoinAccept,
) -> u32 {
    if accept.is_null() {
        return 0;
    }
    (*accept).accept.net_id()
}

/// Returns the downlink settings byte a join grants.
///
/// # Returns
///
/// The settings byte, which carries the second receive window data rate and the
/// first window offset, or 0 if `accept` is null.
///
/// # Safety
///
/// `accept` must be a live handle from [`pamoja_lorawan_device_accept_join`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_accept_dl_settings(
    accept: *const PamojaLorawanJoinAccept,
) -> u8 {
    if accept.is_null() {
        return 0;
    }
    (*accept).accept.dl_settings()
}

/// Returns the delay before the first receive window, in seconds.
///
/// # Returns
///
/// The delay, or 0 if `accept` is null.
///
/// # Safety
///
/// `accept` must be a live handle from [`pamoja_lorawan_device_accept_join`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_accept_rx_delay(
    accept: *const PamojaLorawanJoinAccept,
) -> u8 {
    if accept.is_null() {
        return 0;
    }
    (*accept).accept.rx_delay()
}

/// Takes the activated session a join grants.
///
/// # Arguments
///
/// * `accept` - the accepted join.
/// * `out_session` - receives the session.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_session` set to a handle the caller
/// must release with [`pamoja_lorawan_session_free`], or
/// [`PamojaStatus::InvalidArgument`] if `accept` is null.
///
/// # Safety
///
/// `accept` must be a live handle from [`pamoja_lorawan_device_accept_join`], or
/// null, and `out_session` must point to a writable `*mut PamojaLorawanSession`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_accept_session(
    accept: *const PamojaLorawanJoinAccept,
    out_session: *mut *mut PamojaLorawanSession,
) -> PamojaStatus {
    if out_session.is_null() {
        set_last_error("out_session must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_session;
    *slot = ptr::null_mut();

    if accept.is_null() {
        set_last_error("accept must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *slot = Box::into_raw(Box::new(PamojaLorawanSession {
        session: (*accept).accept.session(),
    }));
    PamojaStatus::Ok
}

/// Releases an accepted join handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `accept` must be a handle from [`pamoja_lorawan_device_accept_join`] that has
/// not already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_accept_free(accept: *mut PamojaLorawanJoinAccept) {
    if !accept.is_null() {
        drop(Box::from_raw(accept));
    }
}

/// Encodes a data frame with whichever builder the caller asked for.
///
/// # Safety
///
/// `payload` and `fopts` must each point to at least their stated lengths in
/// readable bytes when those lengths are non-zero, and `out_frame` must point to a
/// writable `*mut PamojaBuffer`.
unsafe fn encode(
    session: *const PamojaLorawanSession,
    payload: *const u8,
    payload_len: usize,
    fopts: *const u8,
    fopts_len: usize,
    out_frame: *mut *mut PamojaBuffer,
    build: impl FnOnce(&Session, &[u8], &[u8]) -> Result<PhyPayload, LorawanError>,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = ptr::null_mut();

    if session.is_null() {
        set_last_error("session must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let payload = match read_bytes(payload, payload_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let fopts = match read_bytes(fopts, fopts_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match build(&(*session).session, &payload, &fopts) {
        Ok(frame) => {
            *slot = PamojaBuffer::into_raw(frame.as_bytes().to_vec());
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Copies a borrowed 16-byte key.
///
/// # Safety
///
/// `bytes` must point to at least `len` readable bytes when that length is
/// non-zero.
unsafe fn key(bytes: *const u8, len: usize, what: &str) -> Result<[u8; 16], PamojaStatus> {
    let bytes = read_bytes(bytes, len)?;
    <[u8; PAMOJA_LORAWAN_KEY_LEN]>::try_from(&bytes[..]).map_err(|_| {
        set_last_error(format!(
            "{what} must be exactly {PAMOJA_LORAWAN_KEY_LEN} bytes"
        ));
        PamojaStatus::InvalidArgument
    })
}

/// Copies a borrowed 8-byte EUI.
///
/// # Safety
///
/// `bytes` must point to at least `len` readable bytes when that length is
/// non-zero.
unsafe fn eui(bytes: *const u8, len: usize, what: &str) -> Result<[u8; 8], PamojaStatus> {
    let bytes = read_bytes(bytes, len)?;
    <[u8; PAMOJA_LORAWAN_EUI_LEN]>::try_from(&bytes[..]).map_err(|_| {
        set_last_error(format!(
            "{what} must be exactly {PAMOJA_LORAWAN_EUI_LEN} bytes"
        ));
        PamojaStatus::InvalidArgument
    })
}

/// Records a LoRaWAN error and classifies it.
///
/// # Arguments
///
/// * `error` - the failure the LoRaWAN crate reported.
///
/// # Returns
///
/// [`PamojaStatus::Auth`] when a frame failed its integrity or counter check,
/// [`PamojaStatus::InvalidArgument`] when the caller asked for a frame that cannot
/// be built, and [`PamojaStatus::Codec`] when a received frame could not be read.
fn failed(error: LorawanError) -> PamojaStatus {
    set_last_error(error.to_string());
    match error {
        LorawanError::MicMismatch | LorawanError::FcntMismatch => PamojaStatus::Auth,
        LorawanError::PayloadTooLong => PamojaStatus::InvalidArgument,
        _ => PamojaStatus::Codec,
    }
}

/// What kind of message a frame is, read from its header.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaLorawanMessageType {
    /// A device asking to join a network.
    JoinRequest = 0,
    /// A network admitting a device.
    JoinAccept = 1,
    /// Data from a device that does not need acknowledging.
    UnconfirmedUp = 2,
    /// Data from a device that asks to be acknowledged.
    ConfirmedUp = 3,
    /// Data to a device that does not need acknowledging.
    UnconfirmedDown = 4,
    /// Data to a device that asks to be acknowledged.
    ConfirmedDown = 5,
}

/// What a frame says about itself before any key is involved.
///
/// Every field is a scalar, so this crosses the boundary by value. `is_data` is
/// `1` when `dev_addr` and `fcnt` are meaningful, which is every message type
/// except the two join frames, and `has_fport` is `1` when `fport` is.
///
/// Nothing here is authenticated, since checking the MIC needs the session key.
/// Treat it as a routing hint until [`pamoja_lorawan_session_decode`] has verified
/// the frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanHeader {
    /// The length of the still-encrypted payload, in bytes.
    pub payload_len: usize,
    /// The device address, meaningful only when `is_data` is `1`.
    pub dev_addr: u32,
    /// The low 16 bits of the frame counter, meaningful only when `is_data` is `1`.
    pub fcnt: u16,
    /// What kind of message the frame is.
    pub message_type: PamojaLorawanMessageType,
    /// The port the frame was sent on, meaningful only when `has_fport` is `1`.
    pub fport: u8,
    /// `1` for a data frame, `0` for one of the two join frames.
    pub is_data: u8,
    /// `1` when the frame carries a port rather than only frame options.
    pub has_fport: u8,
    /// `1` when the frame asks to be acknowledged.
    pub confirmed: u8,
    /// `1` when the frame takes part in adaptive data rate.
    pub adr: u8,
    /// `1` when the frame acknowledges the last confirmed one.
    pub ack: u8,
    /// `1` when the network has more downlink data waiting.
    pub fpending: u8,
    /// How many bytes of frame options the header carries, from 0 to 15.
    pub fopts_len: u8,
}

/// What a network grants a device that joined.
///
/// Every field is a scalar, so this crosses the boundary by value. The optional
/// channel list is passed alongside it, since it is bytes rather than a scalar.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanGrant {
    /// A nonce this network must not reuse for the device; low 24 bits only.
    pub app_nonce: u32,
    /// The network identifier; low 24 bits only.
    pub net_id: u32,
    /// The address to assign the device.
    pub dev_addr: u32,
    /// The downlink settings byte.
    pub dl_settings: u8,
    /// The delay before the first receive window, in seconds.
    pub rx_delay: u8,
}

/// An opaque handle to a verified join-request.
///
/// Read it with the `pamoja_lorawan_join_request_*` calls, then release it with
/// [`pamoja_lorawan_join_request_free`].
pub struct PamojaLorawanJoinRequest {
    request: JoinRequest,
}

/// Reads a frame far enough to route it, without any key.
///
/// A receiver holding many sessions uses this to find which one a frame belongs
/// to: the device address travels in the clear, so it can be read before the
/// session that would verify the frame is even known.
///
/// # Arguments
///
/// * `bytes` - the raw frame as it came off the radio.
/// * `bytes_len` - its length.
/// * `out_header` - receives the header fields.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_header` filled in, or
/// [`PamojaStatus::Codec`] if the frame is truncated, carries a message type this
/// build does not read, or declares more frame options than it holds.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes when that length is
/// non-zero, and `out_header` must point to a writable [`PamojaLorawanHeader`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_header_parse(
    bytes: *const u8,
    bytes_len: usize,
    out_header: *mut PamojaLorawanHeader,
) -> PamojaStatus {
    if out_header.is_null() {
        set_last_error("out_header must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let header = match FrameHeader::parse(&bytes) {
        Ok(header) => header,
        Err(error) => return failed(error),
    };

    *out_header = PamojaLorawanHeader {
        payload_len: header.payload_len(),
        dev_addr: header.dev_addr().unwrap_or(0),
        fcnt: header.fcnt().unwrap_or(0),
        message_type: match header.message_type() {
            MessageType::JoinRequest => PamojaLorawanMessageType::JoinRequest,
            MessageType::JoinAccept => PamojaLorawanMessageType::JoinAccept,
            MessageType::UnconfirmedUp => PamojaLorawanMessageType::UnconfirmedUp,
            MessageType::ConfirmedUp => PamojaLorawanMessageType::ConfirmedUp,
            MessageType::UnconfirmedDown => PamojaLorawanMessageType::UnconfirmedDown,
            MessageType::ConfirmedDown => PamojaLorawanMessageType::ConfirmedDown,
        },
        fport: header.fport().unwrap_or(0),
        is_data: u8::from(header.message_type().is_data()),
        has_fport: u8::from(header.fport().is_some()),
        confirmed: u8::from(header.confirmed()),
        adr: u8::from(header.adr()),
        ack: u8::from(header.ack()),
        fpending: u8::from(header.fpending()),
        fopts_len: header.fopts_len() as u8,
    };
    PamojaStatus::Ok
}

/// Verifies a join-request and reads the identifiers out of it.
///
/// # Arguments
///
/// * `bytes` - the raw join-request as it came off the radio.
/// * `bytes_len` - its length.
/// * `app_key` - the 16-byte application root key the device shares.
/// * `app_key_len` - its length, which must be [`PAMOJA_LORAWAN_KEY_LEN`].
/// * `out_request` - receives the verified request.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_request` set to a handle the caller
/// must release with [`pamoja_lorawan_join_request_free`], [`PamojaStatus::Auth`]
/// if the MIC does not verify, or [`PamojaStatus::Codec`] if the frame is not a
/// well-formed join-request.
///
/// # Safety
///
/// `bytes` and `app_key` must each point to at least their stated lengths in
/// readable bytes, and `out_request` must point to a writable
/// `*mut PamojaLorawanJoinRequest`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_request_parse(
    bytes: *const u8,
    bytes_len: usize,
    app_key: *const u8,
    app_key_len: usize,
    out_request: *mut *mut PamojaLorawanJoinRequest,
) -> PamojaStatus {
    if out_request.is_null() {
        set_last_error("out_request must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_request;
    *slot = ptr::null_mut();

    let bytes = match read_bytes(bytes, bytes_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let app_key = match key(app_key, app_key_len, "the application key") {
        Ok(key) => key,
        Err(status) => return status,
    };
    match JoinRequest::parse(&bytes, &app_key) {
        Ok(request) => {
            *slot = Box::into_raw(Box::new(PamojaLorawanJoinRequest { request }));
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Copies the device identifier out of a verified join-request.
///
/// # Arguments
///
/// * `request` - the verified request.
/// * `out_dev_eui` - receives [`PAMOJA_LORAWAN_EUI_LEN`] bytes, most-significant
///   byte first.
///
/// # Returns
///
/// `true` when the identifier was written, or `false` if either pointer is null.
///
/// # Safety
///
/// `request` must be a live handle from [`pamoja_lorawan_join_request_parse`], or
/// null, and `out_dev_eui` must point to at least
/// [`PAMOJA_LORAWAN_EUI_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_request_dev_eui(
    request: *const PamojaLorawanJoinRequest,
    out_dev_eui: *mut u8,
) -> bool {
    if request.is_null() || out_dev_eui.is_null() {
        return false;
    }
    let eui = (*request).request.dev_eui();
    ptr::copy_nonoverlapping(eui.as_ptr(), out_dev_eui, PAMOJA_LORAWAN_EUI_LEN);
    true
}

/// Copies the application identifier out of a verified join-request.
///
/// # Arguments
///
/// * `request` - the verified request.
/// * `out_app_eui` - receives [`PAMOJA_LORAWAN_EUI_LEN`] bytes, most-significant
///   byte first.
///
/// # Returns
///
/// `true` when the identifier was written, or `false` if either pointer is null.
///
/// # Safety
///
/// `request` must be a live handle from [`pamoja_lorawan_join_request_parse`], or
/// null, and `out_app_eui` must point to at least
/// [`PAMOJA_LORAWAN_EUI_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_request_app_eui(
    request: *const PamojaLorawanJoinRequest,
    out_app_eui: *mut u8,
) -> bool {
    if request.is_null() || out_app_eui.is_null() {
        return false;
    }
    let eui = (*request).request.app_eui();
    ptr::copy_nonoverlapping(eui.as_ptr(), out_app_eui, PAMOJA_LORAWAN_EUI_LEN);
    true
}

/// Returns the nonce a verified join-request carried.
///
/// A network must remember the nonces a device has used and refuse a repeat, since
/// replaying one would re-derive the same session keys.
///
/// # Returns
///
/// The DevNonce, or 0 if `request` is null.
///
/// # Safety
///
/// `request` must be a live handle from [`pamoja_lorawan_join_request_parse`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_request_dev_nonce(
    request: *const PamojaLorawanJoinRequest,
) -> u16 {
    if request.is_null() {
        return 0;
    }
    (*request).request.dev_nonce()
}

/// Releases a verified join-request handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `request` must be a handle from [`pamoja_lorawan_join_request_parse`] that has
/// not already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_join_request_free(request: *mut PamojaLorawanJoinRequest) {
    if !request.is_null() {
        drop(Box::from_raw(request));
    }
}

/// Builds the signed join-accept a network sends to admit a device.
///
/// # Arguments
///
/// * `grant` - the address and settings to grant.
/// * `cflist` - the optional 16-byte channel list, or null for none.
/// * `cflist_len` - its length, either 0 or 16.
/// * `app_key` - the 16-byte application root key the device shares.
/// * `app_key_len` - its length, which must be [`PAMOJA_LORAWAN_KEY_LEN`].
/// * `dev_nonce` - the nonce the matching join-request carried.
/// * `out_frame` - receives the encoded join-accept.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a buffer the caller
/// must release with [`pamoja_buffer_free`](crate::pamoja_buffer_free), or
/// [`PamojaStatus::InvalidArgument`] if the key or the channel list is the wrong
/// length.
///
/// # Safety
///
/// `cflist` and `app_key` must each point to at least their stated lengths in
/// readable bytes, and `out_frame` must point to a writable `*mut PamojaBuffer`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_grant_accept(
    grant: PamojaLorawanGrant,
    cflist: *const u8,
    cflist_len: usize,
    app_key: *const u8,
    app_key_len: usize,
    dev_nonce: u16,
    out_frame: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = ptr::null_mut();

    let (grant, app_key) = match granted(grant, cflist, cflist_len, app_key, app_key_len) {
        Ok(pair) => pair,
        Err(status) => return status,
    };
    *slot = PamojaBuffer::into_raw(grant.accept(&app_key, dev_nonce).as_bytes().to_vec());
    PamojaStatus::Ok
}

/// Derives the session a grant activates, the same one the device computes.
///
/// # Arguments
///
/// * `grant` - the address and settings granted.
/// * `cflist` - the optional 16-byte channel list, or null for none.
/// * `cflist_len` - its length, either 0 or 16.
/// * `app_key` - the 16-byte application root key the device shares.
/// * `app_key_len` - its length, which must be [`PAMOJA_LORAWAN_KEY_LEN`].
/// * `dev_nonce` - the nonce the matching join-request carried.
/// * `out_session` - receives the session.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_session` set to a handle the caller
/// must release with [`pamoja_lorawan_session_free`], or
/// [`PamojaStatus::InvalidArgument`] if the key or the channel list is the wrong
/// length.
///
/// # Safety
///
/// `cflist` and `app_key` must each point to at least their stated lengths in
/// readable bytes, and `out_session` must point to a writable
/// `*mut PamojaLorawanSession`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_grant_session(
    grant: PamojaLorawanGrant,
    cflist: *const u8,
    cflist_len: usize,
    app_key: *const u8,
    app_key_len: usize,
    dev_nonce: u16,
    out_session: *mut *mut PamojaLorawanSession,
) -> PamojaStatus {
    if out_session.is_null() {
        set_last_error("out_session must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_session;
    *slot = ptr::null_mut();

    let (grant, app_key) = match granted(grant, cflist, cflist_len, app_key, app_key_len) {
        Ok(pair) => pair,
        Err(status) => return status,
    };
    *slot = Box::into_raw(Box::new(PamojaLorawanSession {
        session: grant.session(&app_key, dev_nonce),
    }));
    PamojaStatus::Ok
}

/// Rebuilds a Rust grant and the key it is used with from what crossed the ABI.
///
/// # Safety
///
/// `cflist` and `app_key` must each point to at least their stated lengths in
/// readable bytes when those lengths are non-zero.
unsafe fn granted(
    grant: PamojaLorawanGrant,
    cflist: *const u8,
    cflist_len: usize,
    app_key: *const u8,
    app_key_len: usize,
) -> Result<(JoinGrant, [u8; 16]), PamojaStatus> {
    let app_key = key(app_key, app_key_len, "the application key")?;
    let mut built = JoinGrant::new(grant.app_nonce, grant.net_id, grant.dev_addr)
        .with_dl_settings(grant.dl_settings)
        .with_rx_delay(grant.rx_delay);

    let cflist = read_bytes(cflist, cflist_len)?;
    if !cflist.is_empty() {
        let Ok(cflist) = <[u8; 16]>::try_from(&cflist[..]) else {
            set_last_error("the channel list must be exactly 16 bytes".to_owned());
            return Err(PamojaStatus::InvalidArgument);
        };
        built = built.with_cflist(cflist);
    }
    Ok((built, app_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    const NWK_SKEY: [u8; 16] = [0x2B; 16];
    const APP_SKEY: [u8; 16] = [0x99; 16];

    /// Every flag off, which is a plain unconfirmed frame.
    fn quiet() -> PamojaLorawanFlags {
        PamojaLorawanFlags {
            confirmed: 0,
            adr: 0,
            ack: 0,
            fpending: 0,
        }
    }

    /// Creates a session over the test keys.
    ///
    /// # Safety
    ///
    /// The returned handle must be released with [`pamoja_lorawan_session_free`].
    unsafe fn session() -> *mut PamojaLorawanSession {
        let mut session = ptr::null_mut();
        assert_eq!(
            pamoja_lorawan_session_new(
                0x2601_1BDA,
                NWK_SKEY.as_ptr(),
                NWK_SKEY.len(),
                APP_SKEY.as_ptr(),
                APP_SKEY.len(),
                &mut session
            ),
            PamojaStatus::Ok
        );
        session
    }

    #[test]
    fn the_constants_match_the_lorawan_crate() {
        assert_eq!(PAMOJA_LORAWAN_FRAME_MAX, pamoja_lorawan::MAX_FRAME);
        assert_eq!(PAMOJA_LORAWAN_PAYLOAD_MAX, pamoja_lorawan::MAX_PAYLOAD);
    }

    #[test]
    fn a_confirmed_uplink_round_trips_through_the_boundary() {
        // Safety: every pointer below is valid and every handle is released.
        unsafe {
            let session = session();
            assert_eq!(pamoja_lorawan_session_dev_addr(session), 0x2601_1BDA);

            let payload = b"temp=4.8";
            let mut frame = ptr::null_mut();
            let flags = PamojaLorawanFlags {
                confirmed: 1,
                adr: 1,
                ..quiet()
            };
            assert_eq!(
                pamoja_lorawan_session_encode_uplink(
                    session,
                    42,
                    1,
                    payload.as_ptr(),
                    payload.len(),
                    ptr::null(),
                    0,
                    flags,
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            let on_air =
                std::slice::from_raw_parts(pamoja_buffer_data(frame), pamoja_buffer_len(frame))
                    .to_vec();
            pamoja_buffer_free(frame);

            let mut rx = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_session_decode(session, on_air.as_ptr(), on_air.len(), 42, &mut rx),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_rx_direction(rx),
                PamojaLorawanDirection::Uplink
            );
            assert!(pamoja_lorawan_rx_confirmed(rx));
            assert!(pamoja_lorawan_rx_adr(rx));
            assert_eq!(pamoja_lorawan_rx_fcnt(rx), 42);
            assert_eq!(pamoja_lorawan_rx_dev_addr(rx), 0x2601_1BDA);

            let mut fport = 0u8;
            assert!(pamoja_lorawan_rx_fport(rx, &mut fport));
            assert_eq!(fport, 1);

            let recovered = std::slice::from_raw_parts(
                pamoja_lorawan_rx_payload(rx),
                pamoja_lorawan_rx_payload_len(rx),
            );
            assert_eq!(recovered, payload);

            pamoja_lorawan_rx_free(rx);
            pamoja_lorawan_session_free(session);
        }
    }

    #[test]
    fn a_downlink_carries_its_frame_options() {
        // Safety: every pointer below is valid and every handle is released.
        unsafe {
            let session = session();
            let fopts = [0x03u8, 0x50, 0x00];
            let mut frame = ptr::null_mut();
            let flags = PamojaLorawanFlags {
                fpending: 1,
                ..quiet()
            };
            assert_eq!(
                pamoja_lorawan_session_encode_downlink(
                    session,
                    7,
                    2,
                    ptr::null(),
                    0,
                    fopts.as_ptr(),
                    fopts.len(),
                    flags,
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            let on_air =
                std::slice::from_raw_parts(pamoja_buffer_data(frame), pamoja_buffer_len(frame))
                    .to_vec();
            pamoja_buffer_free(frame);

            let mut rx = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_session_decode(session, on_air.as_ptr(), on_air.len(), 7, &mut rx),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_rx_direction(rx),
                PamojaLorawanDirection::Downlink
            );
            assert!(pamoja_lorawan_rx_fpending(rx));
            let recovered = std::slice::from_raw_parts(
                pamoja_lorawan_rx_fopts(rx),
                pamoja_lorawan_rx_fopts_len(rx),
            );
            assert_eq!(recovered, fopts);
            assert_eq!(pamoja_lorawan_rx_payload_len(rx), 0);
            assert!(pamoja_lorawan_rx_payload(rx).is_null());

            pamoja_lorawan_rx_free(rx);
            pamoja_lorawan_session_free(session);
        }
    }

    #[test]
    fn a_forged_frame_fails_its_integrity_check() {
        // Safety: every pointer below is valid and every handle is released.
        unsafe {
            let session = session();
            let payload = b"reading";
            let mut frame = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_session_encode_uplink(
                    session,
                    1,
                    1,
                    payload.as_ptr(),
                    payload.len(),
                    ptr::null(),
                    0,
                    quiet(),
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            let mut on_air =
                std::slice::from_raw_parts(pamoja_buffer_data(frame), pamoja_buffer_len(frame))
                    .to_vec();
            pamoja_buffer_free(frame);
            let last = on_air.len() - 1;
            on_air[last] ^= 0xFF;

            let mut rx = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_session_decode(session, on_air.as_ptr(), on_air.len(), 1, &mut rx),
                PamojaStatus::Auth
            );
            assert!(rx.is_null());
            pamoja_lorawan_session_free(session);
        }
    }

    #[test]
    fn a_counter_that_does_not_match_is_refused() {
        // Safety: every pointer below is valid and every handle is released.
        unsafe {
            let session = session();
            let payload = b"reading";
            let mut frame = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_session_encode_uplink(
                    session,
                    1,
                    1,
                    payload.as_ptr(),
                    payload.len(),
                    ptr::null(),
                    0,
                    quiet(),
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            let on_air =
                std::slice::from_raw_parts(pamoja_buffer_data(frame), pamoja_buffer_len(frame))
                    .to_vec();
            pamoja_buffer_free(frame);

            let mut rx = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_session_decode(session, on_air.as_ptr(), on_air.len(), 2, &mut rx),
                PamojaStatus::Auth
            );
            assert!(rx.is_null());
            pamoja_lorawan_session_free(session);
        }
    }

    #[test]
    fn a_key_of_the_wrong_length_is_refused() {
        let short = [0u8; 8];
        let mut session = ptr::null_mut();
        // Safety: the buffers and out-pointer are valid.
        unsafe {
            assert_eq!(
                pamoja_lorawan_session_new(
                    1,
                    short.as_ptr(),
                    short.len(),
                    APP_SKEY.as_ptr(),
                    APP_SKEY.len(),
                    &mut session
                ),
                PamojaStatus::InvalidArgument
            );
            assert!(session.is_null());
        }
    }

    #[test]
    fn a_join_request_crosses_the_boundary_byte_for_byte() {
        let dev_eui = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let app_eui = [0x11u8, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18];
        let app_key = [0x2Bu8; 16];
        // Safety: every pointer below is valid and every handle is released.
        unsafe {
            let mut device = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_device_new(
                    dev_eui.as_ptr(),
                    dev_eui.len(),
                    app_eui.as_ptr(),
                    app_eui.len(),
                    app_key.as_ptr(),
                    app_key.len(),
                    &mut device
                ),
                PamojaStatus::Ok
            );

            let mut request = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_device_join_request(device, 0x0102, &mut request),
                PamojaStatus::Ok
            );
            let bytes =
                std::slice::from_raw_parts(pamoja_buffer_data(request), pamoja_buffer_len(request));
            let reference = Device::new(dev_eui, app_eui, app_key).join_request(0x0102);
            assert_eq!(bytes, reference.as_bytes());
            pamoja_buffer_free(request);

            // A join accept the network never signed must not activate a session.
            let forged = [0x20u8; 17];
            let mut accept = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_device_accept_join(
                    device,
                    forged.as_ptr(),
                    forged.len(),
                    0x0102,
                    &mut accept
                ),
                PamojaStatus::Auth
            );
            assert!(accept.is_null());

            pamoja_lorawan_device_free(device);
        }
    }

    #[test]
    fn null_handles_are_tolerated() {
        // Safety: every call below is documented to accept null.
        unsafe {
            assert_eq!(pamoja_lorawan_session_dev_addr(ptr::null()), 0);
            assert_eq!(pamoja_lorawan_rx_fcnt(ptr::null()), 0);
            assert!(!pamoja_lorawan_rx_confirmed(ptr::null()));
            assert!(pamoja_lorawan_rx_payload(ptr::null()).is_null());
            assert_eq!(pamoja_lorawan_join_accept_dev_addr(ptr::null()), 0);
            pamoja_lorawan_session_free(ptr::null_mut());
            pamoja_lorawan_rx_free(ptr::null_mut());
            pamoja_lorawan_device_free(ptr::null_mut());
            pamoja_lorawan_join_accept_free(ptr::null_mut());
        }
    }

    #[test]
    fn a_header_routes_a_frame_before_any_key_is_known() {
        // Safety: every pointer below is valid and every handle is released.
        unsafe {
            let session = session();
            let payload = b"temp=4.8";
            let mut frame = ptr::null_mut();
            let flags = PamojaLorawanFlags {
                confirmed: 1,
                adr: 1,
                ..quiet()
            };
            assert_eq!(
                pamoja_lorawan_session_encode_uplink(
                    session,
                    42,
                    1,
                    payload.as_ptr(),
                    payload.len(),
                    ptr::null(),
                    0,
                    flags,
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            let on_air =
                std::slice::from_raw_parts(pamoja_buffer_data(frame), pamoja_buffer_len(frame))
                    .to_vec();
            pamoja_buffer_free(frame);

            let mut header = std::mem::zeroed::<PamojaLorawanHeader>();
            assert_eq!(
                pamoja_lorawan_header_parse(on_air.as_ptr(), on_air.len(), &mut header),
                PamojaStatus::Ok
            );
            assert_eq!(header.message_type, PamojaLorawanMessageType::ConfirmedUp);
            assert_eq!(header.is_data, 1);
            assert_eq!(header.dev_addr, 0x2601_1BDA);
            assert_eq!(header.fcnt, 42);
            assert_eq!(header.has_fport, 1);
            assert_eq!(header.fport, 1);
            assert_eq!(header.confirmed, 1);
            assert_eq!(header.adr, 1);
            assert_eq!(header.payload_len, payload.len());
            pamoja_lorawan_session_free(session);
        }
    }

    #[test]
    fn a_network_completes_an_activation_across_the_boundary() {
        let dev_eui = [0x00u8, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
        let app_eui = [0x88u8, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let app_key = [0xABu8; 16];
        let grant = PamojaLorawanGrant {
            app_nonce: 0x0003_0201,
            net_id: 0x0006_0504,
            dev_addr: 0x2601_1BDA,
            dl_settings: 0x00,
            rx_delay: 0x01,
        };
        // Safety: every pointer below is valid and every handle is released.
        unsafe {
            let mut device = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_device_new(
                    dev_eui.as_ptr(),
                    dev_eui.len(),
                    app_eui.as_ptr(),
                    app_eui.len(),
                    app_key.as_ptr(),
                    app_key.len(),
                    &mut device
                ),
                PamojaStatus::Ok
            );

            // The device asks, and this network reads the request back.
            let mut request_frame = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_device_join_request(device, 0x1234, &mut request_frame),
                PamojaStatus::Ok
            );
            let on_air = std::slice::from_raw_parts(
                pamoja_buffer_data(request_frame),
                pamoja_buffer_len(request_frame),
            )
            .to_vec();
            pamoja_buffer_free(request_frame);

            let mut request = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_join_request_parse(
                    on_air.as_ptr(),
                    on_air.len(),
                    app_key.as_ptr(),
                    app_key.len(),
                    &mut request
                ),
                PamojaStatus::Ok
            );
            let mut read_eui = [0u8; 8];
            assert!(pamoja_lorawan_join_request_dev_eui(
                request,
                read_eui.as_mut_ptr()
            ));
            assert_eq!(read_eui, dev_eui);
            assert!(pamoja_lorawan_join_request_app_eui(
                request,
                read_eui.as_mut_ptr()
            ));
            assert_eq!(read_eui, app_eui);
            let dev_nonce = pamoja_lorawan_join_request_dev_nonce(request);
            assert_eq!(dev_nonce, 0x1234);
            pamoja_lorawan_join_request_free(request);

            // This network answers, and the device activates on the reply.
            let mut reply = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_grant_accept(
                    grant,
                    ptr::null(),
                    0,
                    app_key.as_ptr(),
                    app_key.len(),
                    dev_nonce,
                    &mut reply
                ),
                PamojaStatus::Ok
            );
            let accept_bytes =
                std::slice::from_raw_parts(pamoja_buffer_data(reply), pamoja_buffer_len(reply))
                    .to_vec();
            pamoja_buffer_free(reply);

            let mut accept = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_device_accept_join(
                    device,
                    accept_bytes.as_ptr(),
                    accept_bytes.len(),
                    dev_nonce,
                    &mut accept
                ),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_lorawan_join_accept_dev_addr(accept), grant.dev_addr);
            assert_eq!(pamoja_lorawan_join_accept_net_id(accept), grant.net_id);
            assert_eq!(pamoja_lorawan_join_accept_rx_delay(accept), grant.rx_delay);

            // Both sides now hold a session, and each can read what the other secures.
            let mut device_session = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_join_accept_session(accept, &mut device_session),
                PamojaStatus::Ok
            );
            let mut network_session = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_grant_session(
                    grant,
                    ptr::null(),
                    0,
                    app_key.as_ptr(),
                    app_key.len(),
                    dev_nonce,
                    &mut network_session
                ),
                PamojaStatus::Ok
            );

            let payload = b"joined";
            let mut uplink = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_session_encode_uplink(
                    device_session,
                    1,
                    1,
                    payload.as_ptr(),
                    payload.len(),
                    ptr::null(),
                    0,
                    quiet(),
                    &mut uplink
                ),
                PamojaStatus::Ok
            );
            let uplink_bytes =
                std::slice::from_raw_parts(pamoja_buffer_data(uplink), pamoja_buffer_len(uplink))
                    .to_vec();
            pamoja_buffer_free(uplink);

            let mut rx = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_session_decode(
                    network_session,
                    uplink_bytes.as_ptr(),
                    uplink_bytes.len(),
                    1,
                    &mut rx
                ),
                PamojaStatus::Ok,
                "the network reads what the device it just admitted sent"
            );
            let recovered = std::slice::from_raw_parts(
                pamoja_lorawan_rx_payload(rx),
                pamoja_lorawan_rx_payload_len(rx),
            );
            assert_eq!(recovered, payload);

            pamoja_lorawan_rx_free(rx);
            pamoja_lorawan_session_free(network_session);
            pamoja_lorawan_session_free(device_session);
            pamoja_lorawan_join_accept_free(accept);
            pamoja_lorawan_device_free(device);
        }
    }

    #[test]
    fn a_request_signed_with_another_key_is_refused_at_the_boundary() {
        let app_key = [0xABu8; 16];
        let other = [0x00u8; 16];
        // Safety: every pointer below is valid and every handle is released.
        unsafe {
            let mut device = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_device_new(
                    [0x11u8; 8].as_ptr(),
                    8,
                    [0x22u8; 8].as_ptr(),
                    8,
                    other.as_ptr(),
                    other.len(),
                    &mut device
                ),
                PamojaStatus::Ok
            );
            let mut frame = ptr::null_mut();
            pamoja_lorawan_device_join_request(device, 1, &mut frame);
            let on_air =
                std::slice::from_raw_parts(pamoja_buffer_data(frame), pamoja_buffer_len(frame))
                    .to_vec();
            pamoja_buffer_free(frame);
            pamoja_lorawan_device_free(device);

            let mut request = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_join_request_parse(
                    on_air.as_ptr(),
                    on_air.len(),
                    app_key.as_ptr(),
                    app_key.len(),
                    &mut request
                ),
                PamojaStatus::Auth
            );
            assert!(request.is_null());
        }
    }
}
