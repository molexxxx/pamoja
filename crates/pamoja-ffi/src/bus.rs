//! The C ABI for the in-process event bus.
//!
//! These functions wrap [`pamoja_bus`] for callers that reach the SDK through
//! the flat C boundary: one publisher, many subscribers, inside a single
//! process. It is how the parts of a gateway talk to each other without knowing
//! about each other, so a sampler can announce a reading and whatever cares
//! about readings picks it up.
//!
//! The Rust bus carries any cloneable event; a C ABI has no such parameter, so
//! this one carries bytes. That is the shape every binding already exchanges,
//! and a caller who wants structure encodes it with
//! [`crate::codec`] on the way in.

use std::ptr;

use pamoja_bus::BroadcastBus;
use pamoja_core::EventBus;

use crate::{read_bytes, runtime, set_last_error, PamojaBuffer, PamojaStatus};

/// An opaque handle to one endpoint on an event bus.
///
/// A handle both publishes and receives. Each subscriber needs its own, taken
/// with [`pamoja_event_bus_subscribe`], because a handle only sees events
/// published after it existed.
pub struct PamojaEventBus {
    inner: BroadcastBus<Vec<u8>>,
}

/// Creates an event bus.
///
/// # Arguments
///
/// * `capacity` - how many events a slow subscriber may fall behind before it
///   starts missing them.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_event_bus_free`].
#[no_mangle]
pub extern "C" fn pamoja_event_bus_new(capacity: usize) -> *mut PamojaEventBus {
    Box::into_raw(Box::new(PamojaEventBus {
        inner: BroadcastBus::new(capacity),
    }))
}

/// Takes another endpoint on the same bus.
///
/// The new endpoint sees events published from now on, not those already sent,
/// so subscribe before publishing anything the subscriber needs to see.
///
/// # Arguments
///
/// * `bus` - an existing endpoint on the bus to join.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_event_bus_free`], or null if
/// `bus` is null.
///
/// # Safety
///
/// `bus` must be a live handle from [`pamoja_event_bus_new`] or this function,
/// or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_bus_subscribe(
    bus: *const PamojaEventBus,
) -> *mut PamojaEventBus {
    if bus.is_null() {
        set_last_error("bus must not be null".to_owned());
        return ptr::null_mut();
    }
    Box::into_raw(Box::new(PamojaEventBus {
        inner: (*bus).inner.subscribe(),
    }))
}

/// Publishes an event to every subscriber.
///
/// # Arguments
///
/// * `bus` - the endpoint to publish from.
/// * `payload` - the event bytes.
/// * `payload_len` - the length of `payload`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once every subscriber has been handed the event, or
/// [`PamojaStatus::Closed`] if the bus has shut down.
///
/// # Safety
///
/// `bus` must be a live handle, and `payload` must point to at least
/// `payload_len` readable bytes or be null when that length is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_bus_publish(
    bus: *const PamojaEventBus,
    payload: *const u8,
    payload_len: usize,
) -> PamojaStatus {
    if bus.is_null() {
        set_last_error("bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    match runtime().block_on((*bus).inner.publish(payload)) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => fail(error),
    }
}

/// Waits for the next event on this endpoint.
///
/// # Arguments
///
/// * `bus` - the endpoint to receive on.
/// * `out_event` - receives a buffer handle, or null when the bus has closed.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. A null `out_event` with an `Ok` status means
/// the bus closed rather than that anything failed.
///
/// # Safety
///
/// `bus` must be a live handle and `out_event` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_bus_next(
    bus: *mut PamojaEventBus,
    out_event: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    if bus.is_null() {
        set_last_error("bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if out_event.is_null() {
        set_last_error("out_event must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_event = ptr::null_mut();
    match runtime().block_on((*bus).inner.next_event()) {
        Ok(Some(event)) => {
            *out_event = PamojaBuffer::into_raw(event);
            PamojaStatus::Ok
        }
        Ok(None) => PamojaStatus::Ok,
        Err(error) => fail(error),
    }
}

/// Releases an event bus endpoint.
///
/// Other endpoints on the same bus keep working.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `bus` must be a handle from [`pamoja_event_bus_new`] or
/// [`pamoja_event_bus_subscribe`] that has not already been freed, or null.
/// After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_bus_free(bus: *mut PamojaEventBus) {
    if !bus.is_null() {
        drop(Box::from_raw(bus));
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
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    #[test]
    fn every_subscriber_sees_a_published_event() {
        unsafe {
            let bus = pamoja_event_bus_new(8);
            let first = pamoja_event_bus_subscribe(bus);
            let second = pamoja_event_bus_subscribe(bus);

            assert_eq!(
                pamoja_event_bus_publish(bus, b"battery.low".as_ptr(), 11),
                PamojaStatus::Ok
            );

            for subscriber in [first, second] {
                let mut event = ptr::null_mut();
                assert_eq!(
                    pamoja_event_bus_next(subscriber, &mut event),
                    PamojaStatus::Ok
                );
                assert!(!event.is_null());
                let bytes =
                    std::slice::from_raw_parts(pamoja_buffer_data(event), pamoja_buffer_len(event))
                        .to_vec();
                assert_eq!(bytes, b"battery.low");
                pamoja_buffer_free(event);
                pamoja_event_bus_free(subscriber);
            }

            pamoja_event_bus_free(bus);
        }
    }

    #[test]
    fn a_null_handle_is_refused_rather_than_dereferenced() {
        unsafe {
            assert!(pamoja_event_bus_subscribe(ptr::null()).is_null());
            assert_eq!(
                pamoja_event_bus_publish(ptr::null(), b"x".as_ptr(), 1),
                PamojaStatus::InvalidArgument
            );
            pamoja_event_bus_free(ptr::null_mut());
        }
    }
}
