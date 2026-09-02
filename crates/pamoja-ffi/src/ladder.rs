//! The C ABI for the cost-aware transport ladder.
//!
//! These functions wrap [`pamoja_ladder`] for callers that reach the SDK through
//! the flat C boundary. A ladder is the answer to a node that has more than one
//! way to reach the network and no single one that always works: rungs are tried
//! in the order they were added, cheapest first, and a message no rung accepts
//! goes into a buffer rather than being lost.
//!
//! The Rust ladder is generic over its buffer, which cannot cross a C ABI, so
//! this one is built over the store handle from [`crate::sync`]. That handle
//! already covers both an in-memory and a file-backed queue, so nothing is given
//! up: a caller still chooses whether the buffer survives a restart.

use std::ffi::c_char;
use std::ptr;

use pamoja_ladder::{Delivery, TransportLadder};

use crate::sync::{take_store, PamojaStore, StoreKind};
use crate::transport::{take_transport, PamojaTransport};
use crate::{read_bytes, read_str, runtime, set_last_error, PamojaStatus};

/// What became of a message handed to a ladder.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaDelivery {
    /// A rung took the message and it is on its way.
    Sent = 0,
    /// No rung would take it, so it is in the buffer awaiting a flush.
    Buffered = 1,
}

/// An opaque handle to a ladder and the buffer behind it.
///
/// The Rust builder takes the ladder by value to add a rung, which would move
/// the handle out from under a caller holding a pointer to it. Holding the
/// ladder in an option lets it be taken and put back so the handle address stays
/// good for the life of the ladder.
pub struct PamojaLadder {
    inner: Option<TransportLadder<StoreKind>>,
}

/// Creates a ladder with no rungs, buffering into a store.
///
/// # Arguments
///
/// * `store` - the buffer to hold messages no rung would take, consumed by this
///   call.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_ladder_free`], or null if
/// `store` is null.
///
/// # Safety
///
/// `store` must be a live handle from [`crate::sync`] that has not been freed or
/// consumed. After this call it must not be used again, whatever the result.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ladder_new(store: *mut PamojaStore) -> *mut PamojaLadder {
    let Some(store) = take_store(store) else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(PamojaLadder {
        inner: Some(TransportLadder::new(store)),
    }))
}

/// Adds a rung, which is tried after the rungs already added.
///
/// Add the cheapest, most-preferred link first and the costliest fallback last,
/// because a send takes the first rung that accepts it.
///
/// # Arguments
///
/// * `ladder` - the ladder to add to.
/// * `transport` - the transport to add, consumed by this call.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the rung is added.
///
/// # Safety
///
/// `ladder` must be a live handle, and `transport` a live transport handle that
/// has not been freed or consumed. After this call the transport must not be
/// used again, whatever the result.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ladder_rung(
    ladder: *mut PamojaLadder,
    transport: *mut PamojaTransport,
) -> PamojaStatus {
    let Some(handle) = ladder_handle(ladder) else {
        // The transport was promised to this call, so it is released rather than
        // left for a caller who has already been told not to touch it again.
        drop(take_transport(transport));
        return PamojaStatus::InvalidArgument;
    };
    let Some(transport) = take_transport(transport) else {
        return PamojaStatus::InvalidArgument;
    };
    let Some(inner) = handle.inner.take() else {
        set_last_error("this ladder is no longer usable".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    handle.inner = Some(inner.rung(transport));
    PamojaStatus::Ok
}

/// Connects every rung, so a send can be tried against each in turn.
///
/// A rung that will not connect is left in the ladder: it may come back, and a
/// send simply falls through it until it does.
///
/// # Arguments
///
/// * `ladder` - the ladder.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the rungs have been tried.
///
/// # Safety
///
/// `ladder` must be a live handle from [`pamoja_ladder_new`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_ladder_connect(ladder: *mut PamojaLadder) -> PamojaStatus {
    let Some(inner) = ladder_inner(ladder) else {
        return PamojaStatus::InvalidArgument;
    };
    match runtime().block_on(inner.connect()) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => fail(error),
    }
}

/// Sends a payload, falling through the rungs and buffering if none take it.
///
/// # Arguments
///
/// * `ladder` - the ladder.
/// * `topic` - the destination topic, as null-terminated UTF-8.
/// * `payload` - the bytes to send.
/// * `payload_len` - the length of `payload`.
/// * `out_delivery` - receives whether the message went out or was buffered.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `out_delivery` saying which happened.
/// Buffering is a success, not a failure: it is what the ladder exists to do.
///
/// # Safety
///
/// `ladder` must be a live handle, `topic` a valid null-terminated UTF-8 string,
/// `payload` must point to at least `payload_len` readable bytes or be null when
/// that length is 0, and `out_delivery` must be writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ladder_send(
    ladder: *mut PamojaLadder,
    topic: *const c_char,
    payload: *const u8,
    payload_len: usize,
    out_delivery: *mut PamojaDelivery,
) -> PamojaStatus {
    let Some(topic) = read_str(topic, "topic") else {
        return PamojaStatus::InvalidArgument;
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    let Some(inner) = ladder_inner(ladder) else {
        return PamojaStatus::InvalidArgument;
    };
    match runtime().block_on(inner.send(topic, &payload)) {
        Ok(delivery) => {
            if !out_delivery.is_null() {
                *out_delivery = match delivery {
                    Delivery::Sent => PamojaDelivery::Sent,
                    Delivery::Buffered => PamojaDelivery::Buffered,
                };
            }
            PamojaStatus::Ok
        }
        Err(error) => fail(error),
    }
}

/// Replays the buffer over the rungs, oldest message first.
///
/// # Arguments
///
/// * `ladder` - the ladder.
/// * `out_sent` - receives how many buffered messages went out, or may be null.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `ladder` must be a live handle and `out_sent` writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ladder_flush(
    ladder: *mut PamojaLadder,
    out_sent: *mut usize,
) -> PamojaStatus {
    let Some(inner) = ladder_inner(ladder) else {
        return PamojaStatus::InvalidArgument;
    };
    match runtime().block_on(inner.flush()) {
        Ok(sent) => {
            if !out_sent.is_null() {
                *out_sent = sent;
            }
            PamojaStatus::Ok
        }
        Err(error) => fail(error),
    }
}

/// Reports how many messages are waiting in the buffer.
///
/// # Arguments
///
/// * `ladder` - the ladder.
/// * `out_count` - receives the count.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `ladder` must be a live handle and `out_count` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ladder_buffered(
    ladder: *mut PamojaLadder,
    out_count: *mut usize,
) -> PamojaStatus {
    let Some(inner) = ladder_inner(ladder) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_count.is_null() {
        set_last_error("out_count must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match runtime().block_on(inner.buffered()) {
        Ok(count) => {
            *out_count = count;
            PamojaStatus::Ok
        }
        Err(error) => fail(error),
    }
}

/// Releases a ladder handle, and the rungs and buffer it owns.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `ladder` must be a handle from [`pamoja_ladder_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_ladder_free(ladder: *mut PamojaLadder) {
    if !ladder.is_null() {
        drop(Box::from_raw(ladder));
    }
}

/// Borrows a ladder handle, rejecting a null pointer.
///
/// # Safety
///
/// `ladder` must be a live handle from [`pamoja_ladder_new`], or null.
unsafe fn ladder_handle<'a>(ladder: *mut PamojaLadder) -> Option<&'a mut PamojaLadder> {
    if ladder.is_null() {
        set_last_error("ladder must not be null".to_owned());
        return None;
    }
    Some(&mut *ladder)
}

/// Borrows the ladder inside a handle, rejecting a null or spent one.
///
/// # Safety
///
/// `ladder` must be a live handle from [`pamoja_ladder_new`], or null.
unsafe fn ladder_inner<'a>(
    ladder: *mut PamojaLadder,
) -> Option<&'a mut TransportLadder<StoreKind>> {
    let handle = ladder_handle(ladder)?;
    match handle.inner.as_mut() {
        Some(inner) => Some(inner),
        None => {
            set_last_error("this ladder is no longer usable".to_owned());
            None
        }
    }
}

/// Records an error and maps it onto a status.
fn fail(error: pamoja_core::Error) -> PamojaStatus {
    let status = PamojaStatus::from_error(&error);
    set_last_error(error.to_string());
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loopback::{
        pamoja_loopback_broker_free, pamoja_loopback_broker_new, pamoja_loopback_transport_connect,
        pamoja_loopback_transport_free, pamoja_loopback_transport_new,
        pamoja_loopback_transport_recv, pamoja_loopback_transport_subscribe,
        pamoja_transport_loopback,
    };
    use crate::sync::pamoja_store_memory;
    use crate::transport::{
        pamoja_message_free, pamoja_message_payload, pamoja_message_payload_len,
        pamoja_transport_faulty,
    };

    #[test]
    fn a_message_no_rung_takes_is_buffered_rather_than_lost() {
        unsafe {
            let ladder = pamoja_ladder_new(pamoja_store_memory(0));
            let topic = std::ffi::CString::new("sensors/1").expect("static");

            let mut delivery = PamojaDelivery::Sent;
            assert_eq!(
                pamoja_ladder_send(ladder, topic.as_ptr(), b"21.5".as_ptr(), 4, &mut delivery),
                PamojaStatus::Ok,
                "buffering is a success, not a failure"
            );
            assert_eq!(delivery, PamojaDelivery::Buffered);

            let mut waiting = 0;
            assert_eq!(
                pamoja_ladder_buffered(ladder, &mut waiting),
                PamojaStatus::Ok
            );
            assert_eq!(waiting, 1);

            pamoja_ladder_free(ladder);
        }
    }

    #[test]
    fn a_rung_that_refuses_falls_through_to_the_next() {
        unsafe {
            let broker = pamoja_loopback_broker_new();
            let listener = pamoja_loopback_transport_new(broker);
            pamoja_loopback_transport_connect(listener);
            let topic = std::ffi::CString::new("sensors/1").expect("static");
            pamoja_loopback_transport_subscribe(listener, topic.as_ptr());

            // The first rung fails its next send; the second is the same broker.
            let failing = pamoja_transport_faulty(pamoja_transport_loopback(broker), 1);
            let working = pamoja_transport_loopback(broker);

            let ladder = pamoja_ladder_new(pamoja_store_memory(0));
            assert_eq!(pamoja_ladder_rung(ladder, failing), PamojaStatus::Ok);
            assert_eq!(pamoja_ladder_rung(ladder, working), PamojaStatus::Ok);
            assert_eq!(pamoja_ladder_connect(ladder), PamojaStatus::Ok);

            let mut delivery = PamojaDelivery::Buffered;
            assert_eq!(
                pamoja_ladder_send(ladder, topic.as_ptr(), b"21.5".as_ptr(), 4, &mut delivery),
                PamojaStatus::Ok
            );
            assert_eq!(
                delivery,
                PamojaDelivery::Sent,
                "the second rung carried what the first refused"
            );

            let mut message = ptr::null_mut();
            pamoja_loopback_transport_recv(listener, &mut message);
            assert!(!message.is_null());
            let payload = std::slice::from_raw_parts(
                pamoja_message_payload(message),
                pamoja_message_payload_len(message),
            )
            .to_vec();
            assert_eq!(payload, b"21.5");
            pamoja_message_free(message);

            pamoja_ladder_free(ladder);
            pamoja_loopback_transport_free(listener);
            pamoja_loopback_broker_free(broker);
        }
    }

    #[test]
    fn a_flush_replays_what_was_buffered() {
        unsafe {
            let broker = pamoja_loopback_broker_new();
            let listener = pamoja_loopback_transport_new(broker);
            pamoja_loopback_transport_connect(listener);
            let topic = std::ffi::CString::new("sensors/1").expect("static");
            pamoja_loopback_transport_subscribe(listener, topic.as_ptr());

            let ladder = pamoja_ladder_new(pamoja_store_memory(0));

            // Nothing to send over yet, so it buffers.
            pamoja_ladder_send(ladder, topic.as_ptr(), b"one".as_ptr(), 3, ptr::null_mut());
            pamoja_ladder_send(ladder, topic.as_ptr(), b"two".as_ptr(), 3, ptr::null_mut());

            // The link comes back.
            assert_eq!(
                pamoja_ladder_rung(ladder, pamoja_transport_loopback(broker)),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_ladder_connect(ladder), PamojaStatus::Ok);

            let mut sent = 0;
            assert_eq!(pamoja_ladder_flush(ladder, &mut sent), PamojaStatus::Ok);
            assert_eq!(sent, 2, "both buffered messages went out");

            let mut waiting = 1;
            pamoja_ladder_buffered(ladder, &mut waiting);
            assert_eq!(waiting, 0, "and the buffer is empty");

            pamoja_ladder_free(ladder);
            pamoja_loopback_transport_free(listener);
            pamoja_loopback_broker_free(broker);
        }
    }

    #[test]
    fn a_null_handle_is_refused_rather_than_dereferenced() {
        unsafe {
            assert!(pamoja_ladder_new(ptr::null_mut()).is_null());
            assert_eq!(
                pamoja_ladder_connect(ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_ladder_buffered(ptr::null_mut(), ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            pamoja_ladder_free(ptr::null_mut());
        }
    }
}
