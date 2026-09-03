//! The C ABI for CoAP.
//!
//! These functions wrap [`pamoja_coap`] for callers that reach the SDK through
//! the flat C boundary. CoAP is the transport for links where MQTT is more than
//! the budget allows: it runs over UDP, its headers are a handful of bytes, and
//! a node can fire a reading and forget it rather than holding a session open.
//!
//! A client holds a socket and a background loop, so it crosses as an opaque
//! handle and every call blocks on the shared runtime. To use CoAP as one rung
//! of a ladder rather than driving it directly, build a
//! [`PamojaTransport`] with
//! [`pamoja_transport_coap`] instead.

use std::ffi::c_char;
use std::ptr;
use std::sync::Arc;
use std::time::Duration;

use pamoja_coap::{CoapConfig, CoapTransport, Reliability};
use pamoja_core::Transport;
use tokio::sync::Mutex;

use crate::transport::{status, Kind, PamojaMessage, PamojaTransport};
use crate::{read_bytes, read_str, runtime, set_last_error, PamojaStatus};

/// Whether a CoAP request is acknowledged and retried.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaCoapReliability {
    /// Fire and forget: the request is sent once and not acknowledged.
    NonConfirmable = 0,
    /// The request is acknowledged, and retransmitted until an ACK arrives.
    Confirmable = 1,
}

/// The settings a CoAP endpoint is built from.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PamojaCoapConfig {
    /// The peer hostname or IP address, as null-terminated UTF-8.
    pub host: *const c_char,
    /// The peer UDP port, conventionally 5683 for plaintext CoAP.
    pub port: u16,
    /// The local address to bind, or null for the default.
    pub bind: *const c_char,
    /// Whether requests are acknowledged and retried.
    pub reliability: PamojaCoapReliability,
    /// How long to wait for an acknowledgement, in milliseconds, or 0 for the
    /// default.
    pub ack_timeout_ms: u32,
    /// How many times to retransmit an unacknowledged request, or 0 for the
    /// default.
    pub max_retransmits: u32,
}

/// An opaque handle to a CoAP endpoint.
pub struct PamojaCoapClient {
    inner: Arc<Mutex<CoapTransport>>,
}

/// Creates a disconnected CoAP endpoint from the given settings.
///
/// # Arguments
///
/// * `config` - the endpoint settings.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_coap_client_free`], or null on
/// failure with the reason available from
/// [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `config` must point to a valid [`PamojaCoapConfig`] whose strings are valid
/// null-terminated UTF-8 for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_client_new(
    config: *const PamojaCoapConfig,
) -> *mut PamojaCoapClient {
    let Some(settings) = coap_settings(config) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaCoapClient {
        inner: Arc::new(Mutex::new(CoapTransport::new(settings))),
    }))
}

/// Binds the local socket so the endpoint can carry traffic.
///
/// # Arguments
///
/// * `client` - the endpoint.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once bound.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_coap_client_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_client_connect(client: *mut PamojaCoapClient) -> PamojaStatus {
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&client.inner);
    status(runtime().block_on(async move { inner.lock().await.connect().await }))
}

/// Sends a payload to a resource path.
///
/// # Arguments
///
/// * `client` - the endpoint.
/// * `topic` - the resource path, as null-terminated UTF-8.
/// * `payload` - the bytes to send.
/// * `payload_len` - the length of `payload`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the request has gone out.
///
/// # Safety
///
/// `client` must be a live handle, `topic` a valid null-terminated UTF-8
/// string, and `payload` must point to at least `payload_len` readable bytes or
/// be null when that length is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_client_send(
    client: *mut PamojaCoapClient,
    topic: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> PamojaStatus {
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(topic) = read_str(topic, "topic") else {
        return PamojaStatus::InvalidArgument;
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    let inner = Arc::clone(&client.inner);
    let topic = topic.to_owned();
    status(runtime().block_on(async move { inner.lock().await.send(&topic, &payload).await }))
}

/// Observes a resource path, so messages published to it arrive at
/// [`pamoja_coap_client_recv`].
///
/// # Arguments
///
/// * `client` - the endpoint.
/// * `topic` - the resource path, as null-terminated UTF-8.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once observing.
///
/// # Safety
///
/// `client` must be a live handle and `topic` a valid null-terminated UTF-8
/// string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_client_subscribe(
    client: *mut PamojaCoapClient,
    topic: *const c_char,
) -> PamojaStatus {
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(topic) = read_str(topic, "topic") else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&client.inner);
    let topic = topic.to_owned();
    status(runtime().block_on(async move { inner.lock().await.subscribe(&topic).await }))
}

/// Waits for the next message on an observed path.
///
/// # Arguments
///
/// * `client` - the endpoint.
/// * `out_message` - receives a message handle, or null when the endpoint is
///   closed.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. A null `out_message` with an `Ok` status
/// means the endpoint closed rather than that anything failed.
///
/// # Safety
///
/// `client` must be a live handle and `out_message` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_client_recv(
    client: *mut PamojaCoapClient,
    out_message: *mut *mut PamojaMessage,
) -> PamojaStatus {
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_message.is_null() {
        set_last_error("out_message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_message = ptr::null_mut();

    let inner = Arc::clone(&client.inner);
    match runtime().block_on(async move { inner.lock().await.recv().await }) {
        Ok(Some(message)) => {
            *out_message = PamojaMessage::into_raw(message.topic, message.payload);
            PamojaStatus::Ok
        }
        Ok(None) => PamojaStatus::Ok,
        Err(error) => {
            let code = PamojaStatus::from_error(&error);
            set_last_error(error.to_string());
            code
        }
    }
}

/// Reports whether the endpoint is bound.
///
/// # Arguments
///
/// * `client` - the endpoint.
///
/// # Returns
///
/// `true` when bound, or `false` if `client` is null.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_coap_client_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_client_is_connected(client: *mut PamojaCoapClient) -> bool {
    let Some(client) = client_handle(client) else {
        return false;
    };
    let inner = Arc::clone(&client.inner);
    runtime().block_on(async move { inner.lock().await.is_connected() })
}

/// Releases the socket the endpoint holds.
///
/// # Arguments
///
/// * `client` - the endpoint.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once closed.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_coap_client_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_client_disconnect(
    client: *mut PamojaCoapClient,
) -> PamojaStatus {
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&client.inner);
    status(runtime().block_on(async move { inner.lock().await.disconnect().await }))
}

/// Releases a CoAP endpoint handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `client` must be a handle from [`pamoja_coap_client_new`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_client_free(client: *mut PamojaCoapClient) {
    if !client.is_null() {
        drop(Box::from_raw(client));
    }
}

/// Creates a CoAP transport for composing into a ladder or a wrapper.
///
/// # Arguments
///
/// * `config` - the endpoint settings.
///
/// # Returns
///
/// A handle the caller must release with
/// [`pamoja_transport_free`](crate::transport::pamoja_transport_free) or hand to
/// a call that consumes it, or null on failure.
///
/// # Safety
///
/// `config` must point to a valid [`PamojaCoapConfig`] whose strings are valid
/// null-terminated UTF-8 for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_coap(
    config: *const PamojaCoapConfig,
) -> *mut PamojaTransport {
    let Some(settings) = coap_settings(config) else {
        return ptr::null_mut();
    };
    PamojaTransport::into_raw(Kind::Coap(CoapTransport::new(settings)))
}

/// Reads the endpoint settings a config describes.
///
/// # Safety
///
/// `config` must point to a valid [`PamojaCoapConfig`] whose strings are valid
/// null-terminated UTF-8 for the duration of the call, or be null.
unsafe fn coap_settings(config: *const PamojaCoapConfig) -> Option<CoapConfig> {
    if config.is_null() {
        set_last_error("config must not be null".to_owned());
        return None;
    }
    let config = &*config;
    let host = read_str(config.host, "host")?;

    let mut settings = CoapConfig::new(host, config.port);
    if !config.bind.is_null() {
        settings = settings.bind(read_str(config.bind, "bind")?);
    }
    settings = settings.reliability(match config.reliability {
        PamojaCoapReliability::NonConfirmable => Reliability::NonConfirmable,
        PamojaCoapReliability::Confirmable => Reliability::Confirmable,
    });
    if config.ack_timeout_ms != 0 {
        settings = settings.ack_timeout(Duration::from_millis(u64::from(config.ack_timeout_ms)));
    }
    if config.max_retransmits != 0 {
        settings = settings.max_retransmits(config.max_retransmits);
    }
    Some(settings)
}

/// Borrows a client handle, rejecting a null pointer.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_coap_client_new`], or null.
unsafe fn client_handle<'a>(client: *mut PamojaCoapClient) -> Option<&'a PamojaCoapClient> {
    if client.is_null() {
        set_last_error("client must not be null".to_owned());
        return None;
    }
    Some(&*client)
}
