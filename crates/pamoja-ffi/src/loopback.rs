//! The C ABI for the in-process loopback broker.
//!
//! These functions wrap [`pamoja_loopback`] so a caller can exercise the
//! publish-and-subscribe path with no broker, no network, and no hardware. That
//! matters most for the bindings: someone writing against the SDK from Python or
//! C# can drive a whole message flow in a unit test rather than standing up
//! infrastructure to find out whether their topics line up.
//!
//! A broker is shared by cloning, so every transport built from one sees the
//! same traffic. Pair it with
//! [`pamoja_transport_faulty`](crate::transport::pamoja_transport_faulty) to
//! check what a caller does when a link starts refusing sends.

use std::ffi::c_char;
use std::ptr;
use std::sync::Arc;

use pamoja_core::Transport;
use pamoja_loopback::{LoopbackBroker, LoopbackTransport};
use tokio::sync::Mutex;

use crate::transport::{status, Kind, PamojaMessage, PamojaTransport};
use crate::{read_bytes, read_str, runtime, set_last_error, PamojaStatus};

/// An opaque handle to an in-process broker.
///
/// Every transport built from one broker shares its traffic, so a message one
/// publishes reaches the others that subscribed to the topic.
pub struct PamojaLoopbackBroker {
    inner: LoopbackBroker,
}

/// An opaque handle to one in-process link to a broker.
pub struct PamojaLoopbackTransport {
    inner: Arc<Mutex<LoopbackTransport>>,
}

/// Creates an in-process broker with no traffic.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_loopback_broker_free`].
#[no_mangle]
pub extern "C" fn pamoja_loopback_broker_new() -> *mut PamojaLoopbackBroker {
    Box::into_raw(Box::new(PamojaLoopbackBroker {
        inner: LoopbackBroker::new(),
    }))
}

/// Releases a broker handle.
///
/// Transports already built from the broker keep working, because each holds
/// its own share of it.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `broker` must be a handle from [`pamoja_loopback_broker_new`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_loopback_broker_free(broker: *mut PamojaLoopbackBroker) {
    if !broker.is_null() {
        drop(Box::from_raw(broker));
    }
}

/// Creates a link to a broker.
///
/// # Arguments
///
/// * `broker` - the broker to join. It is shared, not consumed, so the same
///   broker can back as many links as the caller needs.
///
/// # Returns
///
/// A handle the caller must release with
/// [`pamoja_loopback_transport_free`], or null if `broker` is null.
///
/// # Safety
///
/// `broker` must be a live handle from [`pamoja_loopback_broker_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_loopback_transport_new(
    broker: *const PamojaLoopbackBroker,
) -> *mut PamojaLoopbackTransport {
    let Some(broker) = broker_handle(broker) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaLoopbackTransport {
        inner: Arc::new(Mutex::new(LoopbackTransport::new(broker.inner.clone()))),
    }))
}

/// Marks a link connected so it will carry traffic.
///
/// # Arguments
///
/// * `transport` - the link.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once connected.
///
/// # Safety
///
/// `transport` must be a live handle from [`pamoja_loopback_transport_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_loopback_transport_connect(
    transport: *mut PamojaLoopbackTransport,
) -> PamojaStatus {
    let Some(transport) = transport_handle(transport) else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&transport.inner);
    status(runtime().block_on(async move { inner.lock().await.connect().await }))
}

/// Publishes a payload to a topic on the broker.
///
/// # Arguments
///
/// * `transport` - the link.
/// * `topic` - the destination topic, as null-terminated UTF-8.
/// * `payload` - the bytes to publish.
/// * `payload_len` - the length of `payload`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once every subscriber has been handed the message.
///
/// # Safety
///
/// `transport` must be a live handle, `topic` a valid null-terminated UTF-8
/// string, and `payload` must point to at least `payload_len` readable bytes or
/// be null when that length is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_loopback_transport_send(
    transport: *mut PamojaLoopbackTransport,
    topic: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> PamojaStatus {
    let Some(transport) = transport_handle(transport) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(topic) = read_str(topic, "topic") else {
        return PamojaStatus::InvalidArgument;
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    let inner = Arc::clone(&transport.inner);
    let topic = topic.to_owned();
    status(runtime().block_on(async move { inner.lock().await.send(&topic, &payload).await }))
}

/// Subscribes a link to a topic.
///
/// # Arguments
///
/// * `transport` - the link.
/// * `topic` - the topic to subscribe to, as null-terminated UTF-8.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once subscribed.
///
/// # Safety
///
/// `transport` must be a live handle and `topic` a valid null-terminated UTF-8
/// string.
#[no_mangle]
pub unsafe extern "C" fn pamoja_loopback_transport_subscribe(
    transport: *mut PamojaLoopbackTransport,
    topic: *const c_char,
) -> PamojaStatus {
    let Some(transport) = transport_handle(transport) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(topic) = read_str(topic, "topic") else {
        return PamojaStatus::InvalidArgument;
    };
    let inner = Arc::clone(&transport.inner);
    let topic = topic.to_owned();
    status(runtime().block_on(async move { inner.lock().await.subscribe(&topic).await }))
}

/// Waits for the next message on a subscribed topic.
///
/// # Arguments
///
/// * `transport` - the link.
/// * `out_message` - receives a message handle, or null when the link is closed.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. A null `out_message` with an `Ok` status
/// means the link closed rather than that anything failed.
///
/// # Safety
///
/// `transport` must be a live handle and `out_message` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_loopback_transport_recv(
    transport: *mut PamojaLoopbackTransport,
    out_message: *mut *mut PamojaMessage,
) -> PamojaStatus {
    let Some(transport) = transport_handle(transport) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_message.is_null() {
        set_last_error("out_message must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_message = ptr::null_mut();

    let inner = Arc::clone(&transport.inner);
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

/// Reports whether a link is connected.
///
/// # Arguments
///
/// * `transport` - the link.
///
/// # Returns
///
/// `true` when connected, or `false` if `transport` is null.
///
/// # Safety
///
/// `transport` must be a live handle from [`pamoja_loopback_transport_new`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_loopback_transport_is_connected(
    transport: *mut PamojaLoopbackTransport,
) -> bool {
    let Some(transport) = transport_handle(transport) else {
        return false;
    };
    let inner = Arc::clone(&transport.inner);
    runtime().block_on(async move { inner.lock().await.is_connected() })
}

/// Marks a link disconnected, so sends over it fail.
///
/// # Arguments
///
/// * `transport` - the link.
///
/// # Safety
///
/// `transport` must be a live handle from [`pamoja_loopback_transport_new`], or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_loopback_transport_disconnect(
    transport: *mut PamojaLoopbackTransport,
) {
    let Some(transport) = transport_handle(transport) else {
        return;
    };
    let inner = Arc::clone(&transport.inner);
    runtime().block_on(async move { inner.lock().await.disconnect() });
}

/// Releases a link handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `transport` must be a handle from [`pamoja_loopback_transport_new`] that has
/// not already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_loopback_transport_free(
    transport: *mut PamojaLoopbackTransport,
) {
    if !transport.is_null() {
        drop(Box::from_raw(transport));
    }
}

/// Creates a loopback transport for composing into a ladder or a wrapper.
///
/// # Arguments
///
/// * `broker` - the broker to join, shared rather than consumed.
///
/// # Returns
///
/// A handle the caller must release with
/// [`pamoja_transport_free`](crate::transport::pamoja_transport_free) or hand to
/// a call that consumes it, or null if `broker` is null.
///
/// # Safety
///
/// `broker` must be a live handle from [`pamoja_loopback_broker_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_loopback(
    broker: *const PamojaLoopbackBroker,
) -> *mut PamojaTransport {
    let Some(broker) = broker_handle(broker) else {
        return ptr::null_mut();
    };
    PamojaTransport::into_raw(Kind::Loopback(LoopbackTransport::new(broker.inner.clone())))
}

/// Borrows a broker handle, rejecting a null pointer.
///
/// # Safety
///
/// `broker` must be a live handle from [`pamoja_loopback_broker_new`], or null.
unsafe fn broker_handle<'a>(
    broker: *const PamojaLoopbackBroker,
) -> Option<&'a PamojaLoopbackBroker> {
    if broker.is_null() {
        set_last_error("broker must not be null".to_owned());
        return None;
    }
    Some(&*broker)
}

/// Borrows a link handle, rejecting a null pointer.
///
/// # Safety
///
/// `transport` must be a live handle from [`pamoja_loopback_transport_new`], or
/// null.
unsafe fn transport_handle<'a>(
    transport: *mut PamojaLoopbackTransport,
) -> Option<&'a PamojaLoopbackTransport> {
    if transport.is_null() {
        set_last_error("transport must not be null".to_owned());
        return None;
    }
    Some(&*transport)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{pamoja_message_payload, pamoja_message_payload_len, pamoja_message_free};

    /// Reads a message handle out and releases it.
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
    fn a_published_message_reaches_a_subscriber() {
        unsafe {
            let broker = pamoja_loopback_broker_new();
            let publisher = pamoja_loopback_transport_new(broker);
            let subscriber = pamoja_loopback_transport_new(broker);

            assert_eq!(pamoja_loopback_transport_connect(publisher), PamojaStatus::Ok);
            assert_eq!(
                pamoja_loopback_transport_connect(subscriber),
                PamojaStatus::Ok
            );
            assert!(pamoja_loopback_transport_is_connected(publisher));

            let topic = std::ffi::CString::new("sensors/1").expect("static");
            assert_eq!(
                pamoja_loopback_transport_subscribe(subscriber, topic.as_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_loopback_transport_send(publisher, topic.as_ptr(), b"21.5".as_ptr(), 4),
                PamojaStatus::Ok
            );

            let mut message = ptr::null_mut();
            assert_eq!(
                pamoja_loopback_transport_recv(subscriber, &mut message),
                PamojaStatus::Ok
            );
            assert_eq!(take(message), b"21.5");

            pamoja_loopback_transport_free(subscriber);
            pamoja_loopback_transport_free(publisher);
            pamoja_loopback_broker_free(broker);
        }
    }

    #[test]
    fn a_disconnected_link_refuses_to_send() {
        unsafe {
            let broker = pamoja_loopback_broker_new();
            let transport = pamoja_loopback_transport_new(broker);
            pamoja_loopback_transport_connect(transport);
            pamoja_loopback_transport_disconnect(transport);
            assert!(!pamoja_loopback_transport_is_connected(transport));

            let topic = std::ffi::CString::new("sensors/1").expect("static");
            assert_ne!(
                pamoja_loopback_transport_send(transport, topic.as_ptr(), b"x".as_ptr(), 1),
                PamojaStatus::Ok
            );

            pamoja_loopback_transport_free(transport);
            pamoja_loopback_broker_free(broker);
        }
    }

    #[test]
    fn a_null_handle_is_refused_rather_than_dereferenced() {
        unsafe {
            assert!(pamoja_loopback_transport_new(ptr::null()).is_null());
            assert!(!pamoja_loopback_transport_is_connected(ptr::null_mut()));
            assert_eq!(
                pamoja_loopback_transport_connect(ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            pamoja_loopback_transport_free(ptr::null_mut());
            pamoja_loopback_broker_free(ptr::null_mut());
        }
    }
}
