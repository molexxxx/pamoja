//! The C ABI for the MQTT transport.
//!
//! These functions wrap [`pamoja_mqtt`] for callers that reach the SDK through
//! the flat C boundary. Because that boundary has no async support, the crate
//! owns a single multi-threaded Tokio runtime and each call blocks on it until
//! the underlying async operation completes; a host that wants concurrency runs
//! these calls on its own threads. The shared transport sits behind an async
//! mutex, mirroring the Node and Python bindings so behavior matches across
//! languages.

use std::ffi::{c_char, CString};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use pamoja_core::{Error, Transport};
use pamoja_mqtt::{Inbox, MqttConfig, MqttTransport, PublishOptions, QualityOfService, Tls, Will};

use crate::{read_bytes, read_str, runtime, set_last_error, PamojaStatus};

/// MQTT delivery guarantee, mirroring the protocol's quality-of-service levels.
// The shared `Once` suffix is the protocol's own vocabulary; renaming would
// distort the C ABI, so the variant-name lint is allowed here.
#[allow(clippy::enum_variant_names)]
#[repr(C)]
#[derive(Clone, Copy)]
pub enum PamojaQos {
    /// Fire and forget; the broker does not acknowledge delivery.
    AtMostOnce = 0,
    /// Delivered at least once and acknowledged.
    AtLeastOnce = 1,
    /// Delivered exactly once via a four-step handshake.
    ExactlyOnce = 2,
}

impl From<PamojaQos> for QualityOfService {
    fn from(value: PamojaQos) -> Self {
        match value {
            PamojaQos::AtMostOnce => QualityOfService::AtMostOnce,
            PamojaQos::AtLeastOnce => QualityOfService::AtLeastOnce,
            PamojaQos::ExactlyOnce => QualityOfService::ExactlyOnce,
        }
    }
}

/// A message the broker publishes on the client's behalf if its connection ends without a
/// disconnect: the network dropped, the power failed, or the keep-alive ran out.
///
/// `topic` is a borrowed null-terminated UTF-8 string with no wildcard, and `payload`
/// points at `payload_len` bytes, or is null when `payload_len` is 0.
#[repr(C)]
pub struct PamojaMqttWill {
    /// The topic the broker publishes it to.
    pub topic: *const c_char,
    /// What it publishes.
    pub payload: *const u8,
    /// How many bytes `payload` holds.
    pub payload_len: usize,
    /// The quality of service it is published at.
    pub qos: PamojaQos,
    /// Whether the broker retains it for clients that subscribe later.
    pub retain: bool,
}

/// How a connection is secured with TLS, conventionally on port 8883.
///
/// Each PEM is a borrowed run of bytes with its length. A `ca_pem_len` of 0 trusts the
/// system's certificate authorities; a client certificate and its key are both given or
/// both left at 0.
#[repr(C)]
pub struct PamojaMqttTls {
    /// The certificate authorities to trust, as PEM.
    pub ca_pem: *const u8,
    /// How many bytes `ca_pem` holds, or 0 to trust the system's authorities.
    pub ca_pem_len: usize,
    /// A client certificate to present, as PEM.
    pub certificate_pem: *const u8,
    /// How many bytes `certificate_pem` holds, or 0 for none.
    pub certificate_pem_len: usize,
    /// The client certificate's private key, as PEM.
    pub key_pem: *const u8,
    /// How many bytes `key_pem` holds, or 0 for none.
    pub key_pem_len: usize,
}

/// Connection settings for an MQTT client.
///
/// `client_id` and `host` are borrowed null-terminated UTF-8 strings. A
/// `keep_alive_secs`, `capacity`, or `max_packet_size` of `0` selects the core
/// default. `username`, `password`, `will`, and `tls` are each null when unused.
#[repr(C)]
pub struct PamojaMqttConfig {
    /// The MQTT client identifier presented to the broker.
    pub client_id: *const c_char,
    /// The broker hostname or IP address.
    pub host: *const c_char,
    /// The broker TCP port, conventionally 1883 for plaintext MQTT.
    pub port: u16,
    /// Keep-alive interval in seconds, or 0 for the default of 30.
    pub keep_alive_secs: u32,
    /// Bound on outstanding client requests, or 0 for the default of 64.
    pub capacity: u32,
    /// Default quality of service for publishes and subscriptions.
    pub qos: PamojaQos,
    /// The largest packet the connection sends or accepts, in bytes, or 0 for the
    /// default of 10,240.
    pub max_packet_size: u32,
    /// The name to sign in to the broker with, or null.
    pub username: *const c_char,
    /// The password to sign in with, which needs a username, or null.
    pub password: *const c_char,
    /// A message the broker publishes if the connection ends without a goodbye, or null.
    pub will: *const PamojaMqttWill,
    /// TLS settings, or null for plain TCP.
    pub tls: *const PamojaMqttTls,
}

/// An opaque handle to an MQTT client transport.
pub struct PamojaMqttClient {
    inner: Arc<Mutex<MqttTransport>>,
    inbox: Inbox,
}

/// An opaque handle to a message received from a subscribed topic.
pub struct PamojaMqttMessage {
    topic: CString,
    payload: Vec<u8>,
}

/// Creates a disconnected MQTT client from the given settings.
///
/// # Returns
///
/// A heap-allocated client handle the caller owns and must release with
/// [`pamoja_mqtt_client_free`], or null on failure with the reason available from
/// [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `config` must point to a valid [`PamojaMqttConfig`] whose `client_id` and
/// `host` are valid null-terminated UTF-8 strings for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_new(
    config: *const PamojaMqttConfig,
) -> *mut PamojaMqttClient {
    if config.is_null() {
        set_last_error("config must not be null".to_owned());
        return ptr::null_mut();
    }
    let Some(settings) = mqtt_settings(config) else {
        return ptr::null_mut();
    };

    let transport = MqttTransport::new(settings);
    let client = PamojaMqttClient {
        inbox: transport.inbox(),
        inner: Arc::new(Mutex::new(transport)),
    };
    Box::into_raw(Box::new(client))
}

/// Reads the broker settings a config describes.
///
/// Shared with the composable transport handle, so a client and a ladder rung
/// read the same fields the same way.
///
/// # Safety
///
/// `config` must point to a valid [`PamojaMqttConfig`] whose `client_id` and
/// `host` are valid null-terminated UTF-8 strings for the duration of the call,
/// or be null.
pub(crate) unsafe fn mqtt_settings(config: *const PamojaMqttConfig) -> Option<MqttConfig> {
    if config.is_null() {
        set_last_error("config must not be null".to_owned());
        return None;
    }
    let config = &*config;
    let client_id = read_str(config.client_id, "client_id")?;
    let host = read_str(config.host, "host")?;

    let mut settings = MqttConfig::new(client_id, host, config.port);
    if config.keep_alive_secs != 0 {
        settings = settings.keep_alive(Duration::from_secs(u64::from(config.keep_alive_secs)));
    }
    if config.capacity != 0 {
        settings = settings.capacity(config.capacity as usize);
    }
    if config.max_packet_size != 0 {
        settings = settings.max_packet_size(config.max_packet_size as usize);
    }
    match (config.username.is_null(), config.password.is_null()) {
        (false, _) => {
            let username = read_str(config.username, "username")?;
            let password = if config.password.is_null() {
                ""
            } else {
                read_str(config.password, "password")?
            };
            settings = settings.credentials(username, password);
        }
        (true, false) => {
            set_last_error("a password needs a username: MQTT sends no password alone".to_owned());
            return None;
        }
        (true, true) => {}
    }
    if let Some(will) = config.will.as_ref() {
        let topic = read_str(will.topic, "will topic")?;
        let payload = read_bytes(will.payload, will.payload_len).ok()?;
        let mut built = Will::new(topic, payload).qos(will.qos.into());
        if will.retain {
            built = built.retained();
        }
        settings = settings.last_will(built);
    }
    if let Some(tls) = config.tls.as_ref() {
        let mut built = if tls.ca_pem_len == 0 {
            Tls::system_roots()
        } else {
            Tls::with_ca_pem(read_bytes(tls.ca_pem, tls.ca_pem_len).ok()?)
        };
        match (tls.certificate_pem_len, tls.key_pem_len) {
            (0, 0) => {}
            (0, _) | (_, 0) => {
                set_last_error("a client certificate and its key come together".to_owned());
                return None;
            }
            (certificate_len, key_len) => {
                built = built.client_certificate(
                    read_bytes(tls.certificate_pem, certificate_len).ok()?,
                    read_bytes(tls.key_pem, key_len).ok()?,
                );
            }
        }
        settings = settings.tls(built);
    }
    Some(settings.qos(config.qos.into()))
}

/// Connects to the broker and starts the background event loop.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once connected, or an error status whose message is
/// available from [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `client` must be a non-null handle returned by [`pamoja_mqtt_client_new`] and
/// not yet freed.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_connect(client: *mut PamojaMqttClient) -> PamojaStatus {
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&client.inner);
    run(async move { inner.lock().await.connect().await })
}

/// Publishes a payload to a topic.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the payload is handed to the transport, or an error
/// status.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_mqtt_client_new`]; `topic` must be
/// a valid null-terminated UTF-8 string; and `payload` must point to at least
/// `payload_len` bytes, or be null when `payload_len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_publish(
    client: *mut PamojaMqttClient,
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
    let topic = topic.to_owned();
    let inner = Arc::clone(&client.inner);
    run(async move { inner.lock().await.send(&topic, &payload).await })
}

/// Publishes a payload to a topic with options of its own, optionally waiting for the
/// broker to acknowledge it.
///
/// # Arguments
///
/// * `client` - the client.
/// * `topic` - the destination topic, with no wildcard.
/// * `payload` - the message.
/// * `payload_len` - how many bytes `payload` holds.
/// * `qos` - the quality of service for this message.
/// * `retain` - whether the broker keeps it for clients that subscribe later; an empty
///   retained message clears the one the broker holds.
/// * `confirmed` - whether to return only once the broker acknowledges the message: its
///   `PUBACK` at QoS 1, its `PUBCOMP` at QoS 2, and once the connection has taken it at
///   QoS 0, where MQTT acknowledges nothing.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the message is queued, or acknowledged when `confirmed` is
/// set, or an error status. A confirmed publish whose connection ends first fails with
/// [`PamojaStatus::Transport`], when the message may or may not have arrived.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_mqtt_client_new`]; `topic` must be a valid
/// null-terminated UTF-8 string; and `payload` must point to at least `payload_len` bytes,
/// or be null when `payload_len` is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_publish_with(
    client: *mut PamojaMqttClient,
    topic: *const c_char,
    payload: *const u8,
    payload_len: usize,
    qos: PamojaQos,
    retain: bool,
    confirmed: bool,
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
    let mut options = PublishOptions::new().qos(qos.into());
    if retain {
        options = options.retained();
    }
    let topic = topic.to_owned();
    let inner = Arc::clone(&client.inner);
    run(async move {
        let delivery = inner
            .lock()
            .await
            .publish(&topic, &payload, options)
            .await?;
        if confirmed {
            delivery.confirmed().await
        } else {
            Ok(())
        }
    })
}

/// Subscribes to a topic filter.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the subscription is registered, or an error status.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_mqtt_client_new`] and `topic` a
/// valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_subscribe(
    client: *mut PamojaMqttClient,
    topic: *const c_char,
) -> PamojaStatus {
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(topic) = read_str(topic, "topic") else {
        return PamojaStatus::InvalidArgument;
    };
    let topic = topic.to_owned();
    let inner = Arc::clone(&client.inner);
    run(async move { inner.lock().await.subscribe(&topic).await })
}

/// Awaits the next message from any subscribed topic.
///
/// On success `*out_message` is set to a new message handle the caller owns and
/// must release with [`pamoja_mqtt_message_free`], or to null once the connection
/// has ended and no further messages will arrive.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success (including end of stream), or an error status:
/// [`PamojaStatus::Transport`] once, with the reason as the last error message,
/// when the connection ended on its own.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_mqtt_client_new`] and
/// `out_message` must point to a writable `*mut PamojaMqttMessage`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_recv(
    client: *mut PamojaMqttClient,
    out_message: *mut *mut PamojaMqttMessage,
) -> PamojaStatus {
    if out_message.is_null() {
        set_last_error("out_message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_message = ptr::null_mut();
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let inbox = client.inbox.clone();

    match catch_unwind(AssertUnwindSafe(|| {
        runtime().block_on(async move { inbox.recv().await })
    })) {
        Ok(Ok(Some(message))) => {
            let boxed = Box::new(PamojaMqttMessage {
                topic: CString::new(message.topic)
                    .unwrap_or_else(|_| CString::new("").expect("static")),
                payload: message.payload,
            });
            *out_message = Box::into_raw(boxed);
            PamojaStatus::Ok
        }
        Ok(Ok(None)) => PamojaStatus::Ok,
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            PamojaStatus::from_error(&error)
        }
        Err(_) => {
            set_last_error("panic at the FFI boundary".to_owned());
            PamojaStatus::Panic
        }
    }
}

/// Waits a limited time for the next message from any subscribed topic.
///
/// Running out of time loses nothing: a message that arrives afterwards waits for
/// the next receive. This is the call to use rather than abandoning a receive that
/// is still waiting, which would take that message instead.
///
/// # Arguments
///
/// * `client` - the client.
/// * `timeout_ms` - how long to wait, in milliseconds.
/// * `out_message` - receives a message handle the caller releases with
///   [`pamoja_mqtt_message_free`], or null when the time ran out or the
///   connection has ended.
/// * `out_timed_out` - receives whether the time ran out before a message arrived.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with a message, with the time run out, or with null once
/// the connection has ended, or an error status.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_mqtt_client_new`], and
/// `out_message` and `out_timed_out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_recv_within(
    client: *mut PamojaMqttClient,
    timeout_ms: u64,
    out_message: *mut *mut PamojaMqttMessage,
    out_timed_out: *mut bool,
) -> PamojaStatus {
    if out_message.is_null() || out_timed_out.is_null() {
        set_last_error("out_message and out_timed_out must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_message = ptr::null_mut();
    *out_timed_out = false;
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let inbox = client.inbox.clone();
    let limit = Duration::from_millis(timeout_ms);

    match catch_unwind(AssertUnwindSafe(|| {
        runtime().block_on(async move { tokio::time::timeout(limit, inbox.recv()).await })
    })) {
        Ok(Err(_)) => {
            *out_timed_out = true;
            PamojaStatus::Ok
        }
        Ok(Ok(Ok(Some(message)))) => {
            *out_message = Box::into_raw(Box::new(PamojaMqttMessage {
                topic: CString::new(message.topic)
                    .unwrap_or_else(|_| CString::new("").expect("static")),
                payload: message.payload,
            }));
            PamojaStatus::Ok
        }
        Ok(Ok(Ok(None))) => PamojaStatus::Ok,
        Ok(Ok(Err(error))) => {
            set_last_error(error.to_string());
            PamojaStatus::from_error(&error)
        }
        Err(_) => {
            set_last_error("panic at the FFI boundary".to_owned());
            PamojaStatus::Panic
        }
    }
}

/// Reports whether the client currently holds an active connection.
///
/// # Returns
///
/// `true` while connected. Returns `false` for a null handle or if the check
/// panics.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_mqtt_client_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_is_connected(client: *mut PamojaMqttClient) -> bool {
    let Some(client) = client_handle(client) else {
        return false;
    };
    let inner = Arc::clone(&client.inner);
    catch_unwind(AssertUnwindSafe(|| {
        runtime().block_on(async move { inner.lock().await.is_connected() })
    }))
    .unwrap_or(false)
}

/// Closes the connection and stops the background event loop.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the client has disconnected.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_mqtt_client_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_disconnect(
    client: *mut PamojaMqttClient,
) -> PamojaStatus {
    let Some(client) = client_handle(client) else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&client.inner);
    run(async move { inner.lock().await.disconnect().await })
}

/// Releases an MQTT client handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `client` must be a handle from [`pamoja_mqtt_client_new`] that has not already
/// been freed, or null. After this call the handle must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_client_free(client: *mut PamojaMqttClient) {
    if !client.is_null() {
        drop(Box::from_raw(client));
    }
}

/// Returns the topic a message was published to.
///
/// # Returns
///
/// A pointer to a null-terminated UTF-8 string valid until the message is freed,
/// or null if `message` is null.
///
/// # Safety
///
/// `message` must be a live handle from [`pamoja_mqtt_client_recv`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_message_topic(
    message: *const PamojaMqttMessage,
) -> *const c_char {
    if message.is_null() {
        return ptr::null();
    }
    (*message).topic.as_ptr()
}

/// Returns a pointer to a message's payload bytes.
///
/// Use [`pamoja_mqtt_message_payload_len`] for the length. The pointer is valid
/// until the message is freed.
///
/// # Returns
///
/// A pointer to the payload bytes, or null if `message` is null.
///
/// # Safety
///
/// `message` must be a live handle from [`pamoja_mqtt_client_recv`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_message_payload(
    message: *const PamojaMqttMessage,
) -> *const u8 {
    if message.is_null() {
        return ptr::null();
    }
    (*message).payload.as_ptr()
}

/// Returns the length in bytes of a message's payload.
///
/// # Returns
///
/// The payload length, or 0 if `message` is null.
///
/// # Safety
///
/// `message` must be a live handle from [`pamoja_mqtt_client_recv`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_message_payload_len(
    message: *const PamojaMqttMessage,
) -> usize {
    if message.is_null() {
        return 0;
    }
    (*message).payload.len()
}

/// Releases a message handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `message` must be a handle from [`pamoja_mqtt_client_recv`] that has not
/// already been freed, or null. After this call the handle must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mqtt_message_free(message: *mut PamojaMqttMessage) {
    if !message.is_null() {
        drop(Box::from_raw(message));
    }
}

/// Runs a unit-returning async operation to completion on the shared runtime.
///
/// Panics are caught so they never unwind across the C boundary; a caught panic
/// is reported as [`PamojaStatus::Panic`].
fn run<F>(future: F) -> PamojaStatus
where
    F: Future<Output = Result<(), Error>>,
{
    match catch_unwind(AssertUnwindSafe(|| runtime().block_on(future))) {
        Ok(Ok(())) => PamojaStatus::Ok,
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            PamojaStatus::from_error(&error)
        }
        Err(_) => {
            set_last_error("panic at the FFI boundary".to_owned());
            PamojaStatus::Panic
        }
    }
}

/// Borrows a client handle, recording an error and returning `None` if it is null.
///
/// # Safety
///
/// `client` must be a live handle from [`pamoja_mqtt_client_new`], or null.
unsafe fn client_handle<'a>(client: *mut PamojaMqttClient) -> Option<&'a PamojaMqttClient> {
    if client.is_null() {
        set_last_error("client must not be null".to_owned());
        None
    } else {
        Some(&*client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qos_maps_to_core_levels() {
        assert_eq!(
            QualityOfService::from(PamojaQos::AtMostOnce),
            QualityOfService::AtMostOnce
        );
        assert_eq!(
            QualityOfService::from(PamojaQos::AtLeastOnce),
            QualityOfService::AtLeastOnce
        );
        assert_eq!(
            QualityOfService::from(PamojaQos::ExactlyOnce),
            QualityOfService::ExactlyOnce
        );
    }

    #[test]
    fn read_bytes_treats_zero_length_as_empty() {
        // Safety: a null pointer is allowed when the length is zero.
        let bytes = unsafe { read_bytes(ptr::null(), 0) }.expect("empty payload");
        assert!(bytes.is_empty());
    }

    #[test]
    fn a_password_without_a_username_is_refused() {
        let client_id = CString::new("lonely").expect("no null byte");
        let host = CString::new("localhost").expect("no null byte");
        let password = CString::new("hunter2").expect("no null byte");
        let config = PamojaMqttConfig {
            client_id: client_id.as_ptr(),
            host: host.as_ptr(),
            port: 1883,
            keep_alive_secs: 0,
            capacity: 0,
            qos: PamojaQos::AtLeastOnce,
            max_packet_size: 0,
            username: ptr::null(),
            password: password.as_ptr(),
            will: ptr::null(),
            tls: ptr::null(),
        };
        // Safety: the config and its borrowed strings are valid for the call.
        let client = unsafe { pamoja_mqtt_client_new(&config) };
        assert!(client.is_null());
    }

    #[test]
    fn new_with_null_config_returns_null() {
        // Safety: passing null is explicitly handled by the constructor.
        let client = unsafe { pamoja_mqtt_client_new(ptr::null()) };
        assert!(client.is_null());
    }

    #[test]
    fn calls_on_a_null_client_are_rejected() {
        // Safety: every entry point tolerates a null handle without dereferencing it.
        let status =
            unsafe { pamoja_mqtt_client_publish(ptr::null_mut(), ptr::null(), ptr::null(), 0) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
        // Freeing null is a documented no-op.
        unsafe { pamoja_mqtt_client_free(ptr::null_mut()) };
    }

    #[test]
    fn a_null_topic_is_rejected_before_any_network_use() {
        let client_id = CString::new("audit").expect("no null byte");
        let host = CString::new("localhost").expect("no null byte");
        let config = PamojaMqttConfig {
            client_id: client_id.as_ptr(),
            host: host.as_ptr(),
            port: 1883,
            keep_alive_secs: 0,
            capacity: 0,
            qos: PamojaQos::AtMostOnce,
            max_packet_size: 0,
            username: ptr::null(),
            password: ptr::null(),
            will: ptr::null(),
            tls: ptr::null(),
        };
        // Safety: the config and its borrowed strings are valid for the call.
        let client = unsafe { pamoja_mqtt_client_new(&config) };
        assert!(!client.is_null());
        // A null topic is caught before any connection is attempted.
        let status = unsafe { pamoja_mqtt_client_publish(client, ptr::null(), ptr::null(), 0) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
        // Safety: the handle came from client_new and has not been freed.
        unsafe { pamoja_mqtt_client_free(client) };
    }
}
