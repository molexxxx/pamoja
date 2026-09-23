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

use pamoja_coap::{CoapConfig, CoapPublisher, CoapServer, CoapTransport, Reliability};
use pamoja_core::{Receive, Transport};
use tokio::sync::Mutex;

use crate::transport::{receive_within, status, Kind, PamojaMessage, PamojaTransport};
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
    /// How long to wait for an acknowledgment, in milliseconds, or 0 for the
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

/// Waits a limited time for the next message on an observed path.
///
/// Running out of time loses nothing: a message that arrives afterwards waits for
/// the next receive.
///
/// # Arguments
///
/// * `client` - the endpoint.
/// * `timeout_ms` - how long to wait, in milliseconds.
/// * `out_message` - receives a message handle, or null when the time ran out or
///   the endpoint is closed.
/// * `out_timed_out` - receives whether the time ran out before a message arrived.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with a message, with the time run out, or with null once
/// the endpoint has closed.
///
/// # Safety
///
/// `client` must be a live handle, and `out_message` and `out_timed_out` must be
/// writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_client_recv_within(
    client: *mut PamojaCoapClient,
    timeout_ms: u64,
    out_message: *mut *mut PamojaMessage,
    out_timed_out: *mut bool,
) -> PamojaStatus {
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&client.inner);
    receive_within(timeout_ms, out_message, out_timed_out, || async move {
        inner.lock().await.recv().await
    })
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

/// An opaque handle to a CoAP server.
///
/// Connecting, subscribing, receiving, and disconnecting take the server one call
/// at a time, and a receive holds it while it waits. Sending and counting observers
/// go through the server's publisher instead, so a command goes out while another
/// thread waits for a reading.
pub struct PamojaCoapServer {
    server: Arc<Mutex<CoapServer>>,
    publisher: CoapPublisher,
}

/// Creates a CoAP server that will listen on a local address.
///
/// # Arguments
///
/// * `bind` - the local `host:port`, as null-terminated UTF-8, such as
///   `0.0.0.0:5683`, or port 0 for a free one.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_coap_server_free`], or null if
/// `bind` is null or not UTF-8.
///
/// # Safety
///
/// `bind` must be a valid null-terminated UTF-8 string for the duration of the
/// call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_new(bind: *const c_char) -> *mut PamojaCoapServer {
    let Some(bind) = read_str(bind, "bind") else {
        return ptr::null_mut();
    };
    let server = CoapServer::new(bind);
    let publisher = server.publisher();
    Box::into_raw(Box::new(PamojaCoapServer {
        server: Arc::new(Mutex::new(server)),
        publisher,
    }))
}

/// Binds the server's socket and starts answering requests.
///
/// # Arguments
///
/// * `server` - the server.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once bound, or [`PamojaStatus::Transport`] if the address
/// cannot be bound.
///
/// # Safety
///
/// `server` must be a live handle from [`pamoja_coap_server_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_connect(server: *mut PamojaCoapServer) -> PamojaStatus {
    let Some(server) = server_handle(server) else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&server.server);
    status(runtime().block_on(async move { inner.lock().await.connect().await }))
}

/// Takes the readings sent to the paths a filter matches.
///
/// A PUT or POST to any other path is answered 4.04 Not Found.
///
/// # Arguments
///
/// * `server` - the server.
/// * `filter` - the filter, as null-terminated UTF-8, with `+` for one level and
///   `#` for the rest.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the filter is in place.
///
/// # Safety
///
/// `server` must be a live handle and `filter` a valid null-terminated UTF-8
/// string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_subscribe(
    server: *mut PamojaCoapServer,
    filter: *const c_char,
) -> PamojaStatus {
    let Some(server) = server_handle(server) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(filter) = read_str(filter, "filter") else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&server.server);
    let filter = filter.to_owned();
    status(runtime().block_on(async move { inner.lock().await.subscribe(&filter).await }))
}

/// Sets the state of a resource and notifies every observer of it.
///
/// This goes through the server's publisher, so it does not wait for a receive
/// running on another thread.
///
/// # Arguments
///
/// * `server` - the server.
/// * `path` - the resource's path, as null-terminated UTF-8.
/// * `payload` - its new state.
/// * `payload_len` - the length of `payload`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once each notification has left, or
/// [`PamojaStatus::Closed`] if the server is not connected.
///
/// # Safety
///
/// `server` must be a live handle, `path` a valid null-terminated UTF-8 string, and
/// `payload` must point to at least `payload_len` readable bytes or be null when
/// that length is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_send(
    server: *mut PamojaCoapServer,
    path: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> PamojaStatus {
    let Some(server) = server_handle(server) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(path) = read_str(path, "path") else {
        return PamojaStatus::InvalidArgument;
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    let publisher = server.publisher.clone();
    let path = path.to_owned();
    status(runtime().block_on(async move { publisher.publish(&path, &payload).await }))
}

/// Waits for the next reading sent to a path the server takes.
///
/// # Arguments
///
/// * `server` - the server.
/// * `out_message` - receives a message handle to release with
///   [`pamoja_message_free`](crate::transport::pamoja_message_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with a message, or [`PamojaStatus::Closed`] if the server is
/// not connected.
///
/// # Safety
///
/// `server` must be a live handle and `out_message` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_recv(
    server: *mut PamojaCoapServer,
    out_message: *mut *mut PamojaMessage,
) -> PamojaStatus {
    let Some(server) = server_handle(server) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_message.is_null() {
        set_last_error("out_message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_message = ptr::null_mut();
    let inner = Arc::clone(&server.server);
    match runtime().block_on(async move { inner.lock().await.recv().await }) {
        Ok(Some(message)) => {
            *out_message = PamojaMessage::into_raw(message.topic, message.payload);
            PamojaStatus::Ok
        }
        Ok(None) => PamojaStatus::Ok,
        Err(error) => status(Err(error)),
    }
}

/// Waits a limited time for the next reading sent to a path the server takes.
///
/// Running out of time loses nothing: a reading that arrives afterwards waits for
/// the next receive.
///
/// # Arguments
///
/// * `server` - the server.
/// * `timeout_ms` - how long to wait, in milliseconds.
/// * `out_message` - receives a message handle, or null when the time ran out.
/// * `out_timed_out` - receives whether the time ran out before a reading arrived.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with a message or with the time run out, or
/// [`PamojaStatus::Closed`] if the server is not connected.
///
/// # Safety
///
/// `server` must be a live handle, and `out_message` and `out_timed_out` must be
/// writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_recv_within(
    server: *mut PamojaCoapServer,
    timeout_ms: u64,
    out_message: *mut *mut PamojaMessage,
    out_timed_out: *mut bool,
) -> PamojaStatus {
    let Some(server) = server_handle(server) else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&server.server);
    receive_within(timeout_ms, out_message, out_timed_out, || async move {
        inner.lock().await.recv().await
    })
}

/// Counts the clients observing a resource.
///
/// # Arguments
///
/// * `server` - the server.
/// * `path` - the resource's path, as null-terminated UTF-8.
///
/// # Returns
///
/// How many observers the resource has, or 0 if `server` or `path` is null.
///
/// # Safety
///
/// `server` must be a live handle and `path` a valid null-terminated UTF-8 string,
/// or either may be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_observers(
    server: *mut PamojaCoapServer,
    path: *const c_char,
) -> usize {
    let Some(server) = server_handle(server) else {
        return 0;
    };
    let Some(path) = read_str(path, "path") else {
        return 0;
    };
    server.publisher.observers(path)
}

/// The port the server listens on, which names the one the system chose when it
/// bound port 0.
///
/// # Arguments
///
/// * `server` - the server.
///
/// # Returns
///
/// The port, or 0 while the server is not connected or if `server` is null.
///
/// # Safety
///
/// `server` must be a live handle from [`pamoja_coap_server_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_local_port(server: *mut PamojaCoapServer) -> u16 {
    let Some(server) = server_handle(server) else {
        return 0;
    };
    server
        .publisher
        .local_addr()
        .map_or(0, |address| address.port())
}

/// Reports whether the server holds a bound socket.
///
/// # Arguments
///
/// * `server` - the server.
///
/// # Returns
///
/// `true` while connected, or `false` if `server` is null.
///
/// # Safety
///
/// `server` must be a live handle from [`pamoja_coap_server_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_is_connected(server: *mut PamojaCoapServer) -> bool {
    let Some(server) = server_handle(server) else {
        return false;
    };
    server.publisher.is_connected()
}

/// Closes the server's socket, keeping its resources and filters.
///
/// # Arguments
///
/// * `server` - the server.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once closed.
///
/// # Safety
///
/// `server` must be a live handle from [`pamoja_coap_server_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_disconnect(
    server: *mut PamojaCoapServer,
) -> PamojaStatus {
    let Some(server) = server_handle(server) else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&server.server);
    status(runtime().block_on(async move { inner.lock().await.disconnect().await }))
}

/// Releases a CoAP server handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `server` must be a handle from [`pamoja_coap_server_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_coap_server_free(server: *mut PamojaCoapServer) {
    if !server.is_null() {
        drop(Box::from_raw(server));
    }
}

/// Borrows a server handle, rejecting a null pointer.
///
/// # Safety
///
/// `server` must be a live handle from [`pamoja_coap_server_new`], or null.
unsafe fn server_handle<'a>(server: *mut PamojaCoapServer) -> Option<&'a PamojaCoapServer> {
    if server.is_null() {
        set_last_error("server must not be null".to_owned());
        return None;
    }
    Some(&*server)
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

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;
    use crate::transport::{
        pamoja_message_free, pamoja_message_payload, pamoja_message_payload_len,
    };

    /// Reads a message handle's payload and releases it.
    unsafe fn take(message: *mut PamojaMessage) -> Vec<u8> {
        assert!(!message.is_null());
        let bytes = std::slice::from_raw_parts(
            pamoja_message_payload(message),
            pamoja_message_payload_len(message),
        )
        .to_vec();
        pamoja_message_free(message);
        bytes
    }

    #[test]
    fn a_server_takes_a_reading_and_notifies_an_observer() {
        unsafe {
            let bind = CString::new("127.0.0.1:0").expect("static");
            let server = pamoja_coap_server_new(bind.as_ptr());
            assert_eq!(pamoja_coap_server_connect(server), PamojaStatus::Ok);
            let port = pamoja_coap_server_local_port(server);
            assert_ne!(port, 0);
            let filter = CString::new("sensors/#").expect("static");
            assert_eq!(
                pamoja_coap_server_subscribe(server, filter.as_ptr()),
                PamojaStatus::Ok
            );
            let command = CString::new("commands/valve").expect("static");
            assert_eq!(
                pamoja_coap_server_send(server, command.as_ptr(), b"closed".as_ptr(), 6),
                PamojaStatus::Ok
            );

            let host = CString::new("127.0.0.1").expect("static");
            let config = PamojaCoapConfig {
                host: host.as_ptr(),
                port,
                bind: ptr::null(),
                reliability: PamojaCoapReliability::Confirmable,
                ack_timeout_ms: 200,
                max_retransmits: 1,
            };
            let client = pamoja_coap_client_new(&config);
            assert_eq!(pamoja_coap_client_connect(client), PamojaStatus::Ok);
            assert_eq!(
                pamoja_coap_client_subscribe(client, command.as_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_coap_server_observers(server, command.as_ptr()), 1);

            let mut message = ptr::null_mut();
            let mut timed_out = false;
            pamoja_coap_client_recv_within(client, 2000, &mut message, &mut timed_out);
            assert!(!timed_out);
            assert_eq!(take(message), b"closed");

            let reading = CString::new("sensors/1/temperature").expect("static");
            assert_eq!(
                pamoja_coap_client_send(client, reading.as_ptr(), b"21.5".as_ptr(), 4),
                PamojaStatus::Ok
            );
            pamoja_coap_server_recv_within(server, 2000, &mut message, &mut timed_out);
            assert!(!timed_out);
            assert_eq!(take(message), b"21.5");

            let stray = CString::new("pumps/1").expect("static");
            assert_eq!(
                pamoja_coap_client_send(client, stray.as_ptr(), b"on".as_ptr(), 2),
                PamojaStatus::Transport
            );

            pamoja_coap_client_free(client);
            assert_eq!(pamoja_coap_server_disconnect(server), PamojaStatus::Ok);
            assert!(!pamoja_coap_server_is_connected(server));
            pamoja_coap_server_free(server);
        }
    }
}
