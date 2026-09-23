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
//!
//! Every call on an endpoint takes it by shared pointer, and an endpoint may be
//! used from several threads at once: a publish or a subscribe runs while a next
//! waits on another thread, and two nexts on one endpoint take turns.

use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use pamoja_bus::{BroadcastBus, EventPublisher};
use pamoja_core::EventBus;
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::{read_bytes, runtime, set_last_error, PamojaBuffer, PamojaStatus};

/// An opaque handle to one endpoint on an event bus.
///
/// A handle both publishes and receives, and it receives what it publishes
/// itself. Each subscriber needs its own, taken with
/// [`pamoja_event_bus_subscribe`], because a handle only sees events published
/// after it existed.
pub struct PamojaEventBus {
    publisher: EventPublisher<Vec<u8>>,
    receiver: Mutex<BroadcastBus<Vec<u8>>>,
    missed: AtomicU64,
}

impl PamojaEventBus {
    /// Boxes an endpoint around one subscription.
    fn into_raw(receiver: BroadcastBus<Vec<u8>>) -> *mut PamojaEventBus {
        Box::into_raw(Box::new(PamojaEventBus {
            publisher: receiver.publisher(),
            receiver: Mutex::new(receiver),
            missed: AtomicU64::new(0),
        }))
    }
}

/// An opaque handle that publishes to a bus and has no queue of its own.
///
/// A part that only announces holds one of these rather than an endpoint, so it
/// never fills a buffer it does not read.
pub struct PamojaEventPublisher {
    inner: EventPublisher<Vec<u8>>,
}

/// Creates an event bus.
///
/// # Arguments
///
/// * `capacity` - how many events a slow subscriber may fall behind before it
///   starts missing them, rounded up to the next power of two and at most
///   1048576.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_event_bus_free`].
#[no_mangle]
pub extern "C" fn pamoja_event_bus_new(capacity: usize) -> *mut PamojaEventBus {
    PamojaEventBus::into_raw(BroadcastBus::new(capacity))
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
    match endpoint(bus) {
        Some(bus) => PamojaEventBus::into_raw(bus.publisher.subscribe()),
        None => ptr::null_mut(),
    }
}

/// Takes a publish-only handle on the same bus.
///
/// # Arguments
///
/// * `bus` - an existing endpoint on the bus.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_event_publisher_free`], or
/// null if `bus` is null.
///
/// # Safety
///
/// `bus` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_bus_publisher(
    bus: *const PamojaEventBus,
) -> *mut PamojaEventPublisher {
    match endpoint(bus) {
        Some(bus) => Box::into_raw(Box::new(PamojaEventPublisher {
            inner: bus.publisher.clone(),
        })),
        None => ptr::null_mut(),
    }
}

/// Publishes an event to every subscriber, this endpoint included.
///
/// The call never waits: a subscriber that has fallen behind loses its oldest
/// event rather than holding up the publisher.
///
/// # Arguments
///
/// * `bus` - the endpoint to publish from.
/// * `payload` - the event bytes.
/// * `payload_len` - the length of `payload`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once every subscriber has been handed the event.
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
    let Some(bus) = endpoint(bus) else {
        return PamojaStatus::InvalidArgument;
    };
    match read_bytes(payload, payload_len) {
        Ok(payload) => {
            bus.publisher.publish(payload);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Waits for the next event on this endpoint.
///
/// The endpoint holds a publisher of its own, so the bus stays open while it
/// exists and the call waits until an event arrives. Two calls on one endpoint
/// take turns.
///
/// # Arguments
///
/// * `bus` - the endpoint to receive on.
/// * `out_event` - receives a buffer handle the caller must release with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `bus` must be a live handle and `out_event` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_bus_next(
    bus: *const PamojaEventBus,
    out_event: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let Some(bus) = endpoint(bus) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_event.is_null() {
        set_last_error("out_event must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_event = ptr::null_mut();
    match runtime().block_on(next_on(bus, None)) {
        Some(event) => deliver(event, out_event),
        None => PamojaStatus::Ok,
    }
}

/// Waits for the next event on this endpoint, giving up after a time limit.
///
/// A wait that runs out takes no event, so the next one published is left for
/// the next call.
///
/// # Arguments
///
/// * `bus` - the endpoint to receive on.
/// * `timeout_ms` - how long to wait, in milliseconds, including any wait for
///   another call on this endpoint to finish. A limit too far off to schedule
///   waits as [`pamoja_event_bus_next`] does.
/// * `out_event` - receives a buffer handle the caller must release with
///   [`pamoja_buffer_free`](crate::pamoja_buffer_free), or null when the time
///   ran out.
/// * `out_timed_out` - receives `true` when the time ran out.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] whether an event arrived or the time ran out.
///
/// # Safety
///
/// `bus` must be a live handle, and `out_event` and `out_timed_out` must be
/// writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_bus_next_within(
    bus: *const PamojaEventBus,
    timeout_ms: u64,
    out_event: *mut *mut PamojaBuffer,
    out_timed_out: *mut bool,
) -> PamojaStatus {
    let Some(bus) = endpoint(bus) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_event.is_null() || out_timed_out.is_null() {
        set_last_error("out_event and out_timed_out must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_event = ptr::null_mut();
    *out_timed_out = false;
    let limit = Duration::from_millis(timeout_ms);
    match runtime().block_on(async { next_on(bus, Instant::now().checked_add(limit)).await }) {
        Some(event) => deliver(event, out_event),
        None => {
            *out_timed_out = true;
            PamojaStatus::Ok
        }
    }
}

/// Counts the events this endpoint lost by falling behind.
///
/// # Arguments
///
/// * `bus` - the endpoint to ask.
///
/// # Returns
///
/// How many events were dropped from this endpoint's buffer before it read
/// them, as of its last completed wait, or 0 if `bus` is null.
///
/// # Safety
///
/// `bus` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_bus_missed(bus: *const PamojaEventBus) -> u64 {
    endpoint(bus).map_or(0, |bus| bus.missed.load(Ordering::Relaxed))
}

/// Releases an event bus endpoint.
///
/// Other endpoints on the same bus keep working.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `bus` must be a handle from [`pamoja_event_bus_new`],
/// [`pamoja_event_bus_subscribe`], or [`pamoja_event_publisher_subscribe`] that
/// has not already been freed, or null. After this call it must not be used
/// again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_bus_free(bus: *mut PamojaEventBus) {
    if !bus.is_null() {
        drop(Box::from_raw(bus));
    }
}

/// Creates a bus with no subscribers yet, and a publisher on it.
///
/// # Arguments
///
/// * `capacity` - how many events a slow subscriber may fall behind before it
///   starts missing them, rounded up to the next power of two and at most
///   1048576.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_event_publisher_free`].
#[no_mangle]
pub extern "C" fn pamoja_event_publisher_new(capacity: usize) -> *mut PamojaEventPublisher {
    Box::into_raw(Box::new(PamojaEventPublisher {
        inner: EventPublisher::new(capacity),
    }))
}

/// Hands an event to every current subscriber.
///
/// The call never waits, and it may be made from any thread.
///
/// # Arguments
///
/// * `publisher` - the publisher.
/// * `payload` - the event bytes.
/// * `payload_len` - the length of `payload`.
/// * `out_reached` - receives how many subscribers the event was handed to; an
///   event published while no one is subscribed is dropped, and this is 0.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `publisher` must be a live handle, `payload` must point to at least
/// `payload_len` readable bytes or be null when that length is 0, and
/// `out_reached` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_publisher_publish(
    publisher: *const PamojaEventPublisher,
    payload: *const u8,
    payload_len: usize,
    out_reached: *mut usize,
) -> PamojaStatus {
    if publisher.is_null() || out_reached.is_null() {
        set_last_error("publisher and out_reached must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match read_bytes(payload, payload_len) {
        Ok(payload) => {
            *out_reached = (*publisher).inner.publish(payload);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Subscribes to the bus a publisher publishes on.
///
/// # Arguments
///
/// * `publisher` - the publisher.
///
/// # Returns
///
/// An endpoint that receives events published from now on, which the caller must
/// release with [`pamoja_event_bus_free`], or null if `publisher` is null.
///
/// # Safety
///
/// `publisher` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_publisher_subscribe(
    publisher: *const PamojaEventPublisher,
) -> *mut PamojaEventBus {
    if publisher.is_null() {
        set_last_error("publisher must not be null".to_owned());
        return ptr::null_mut();
    }
    PamojaEventBus::into_raw((*publisher).inner.subscribe())
}

/// Takes another publisher on the same bus, for another part that announces.
///
/// # Arguments
///
/// * `publisher` - an existing publisher on the bus.
///
/// # Returns
///
/// A handle with a lifetime of its own, which the caller must release with
/// [`pamoja_event_publisher_free`], or null if `publisher` is null.
///
/// # Safety
///
/// `publisher` must be a live handle, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_publisher_clone(
    publisher: *const PamojaEventPublisher,
) -> *mut PamojaEventPublisher {
    if publisher.is_null() {
        set_last_error("publisher must not be null".to_owned());
        return ptr::null_mut();
    }
    Box::into_raw(Box::new(PamojaEventPublisher {
        inner: (*publisher).inner.publisher(),
    }))
}

/// Releases a publisher.
///
/// The bus stays open for the endpoints still on it.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `publisher` must be a handle from [`pamoja_event_publisher_new`],
/// [`pamoja_event_publisher_clone`], or [`pamoja_event_bus_publisher`] that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_event_publisher_free(publisher: *mut PamojaEventPublisher) {
    if !publisher.is_null() {
        drop(Box::from_raw(publisher));
    }
}

/// Borrows an endpoint, rejecting a null pointer.
///
/// # Safety
///
/// `bus` must be a live handle, or null.
unsafe fn endpoint<'a>(bus: *const PamojaEventBus) -> Option<&'a PamojaEventBus> {
    if bus.is_null() {
        set_last_error("bus must not be null".to_owned());
        return None;
    }
    Some(&*bus)
}

/// Waits for this endpoint's next event, until `deadline` when one is given.
///
/// Returns `None` when the deadline passed, whether waiting for another call on
/// the endpoint or for an event, and records the endpoint's missed count after
/// every wait that reached its buffer.
async fn next_on(bus: &PamojaEventBus, deadline: Option<Instant>) -> Option<Option<Vec<u8>>> {
    let mut receiver = match deadline {
        Some(deadline) => tokio::time::timeout_at(deadline, bus.receiver.lock())
            .await
            .ok()?,
        None => bus.receiver.lock().await,
    };
    let event = match deadline {
        Some(deadline) => tokio::time::timeout_at(deadline, receiver.next_event()).await,
        None => Ok(receiver.next_event().await),
    };
    bus.missed.store(receiver.missed(), Ordering::Relaxed);
    event.ok().map(|event| event.ok().flatten())
}

/// Writes an event through `out_event`.
///
/// # Safety
///
/// `out_event` must be writable.
unsafe fn deliver(event: Option<Vec<u8>>, out_event: *mut *mut PamojaBuffer) -> PamojaStatus {
    if let Some(event) = event {
        *out_event = PamojaBuffer::into_raw(event);
    }
    PamojaStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};

    unsafe fn take(event: *mut PamojaBuffer) -> Vec<u8> {
        assert!(!event.is_null());
        let bytes = std::slice::from_raw_parts(pamoja_buffer_data(event), pamoja_buffer_len(event))
            .to_vec();
        pamoja_buffer_free(event);
        bytes
    }

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
                assert_eq!(take(event), b"battery.low");
                pamoja_event_bus_free(subscriber);
            }

            pamoja_event_bus_free(bus);
        }
    }

    #[test]
    fn an_endpoint_publishes_while_its_own_next_waits() {
        unsafe {
            let bus = pamoja_event_bus_new(8);
            let address = bus as usize;
            let waiting = std::thread::spawn(move || {
                let mut event = ptr::null_mut();
                let status = pamoja_event_bus_next(address as *const PamojaEventBus, &mut event);
                (status, event as usize)
            });
            std::thread::sleep(Duration::from_millis(50));

            assert_eq!(
                pamoja_event_bus_publish(bus, b"heater.off".as_ptr(), 10),
                PamojaStatus::Ok
            );
            let (status, event) = waiting.join().expect("the waiting thread");
            assert_eq!(status, PamojaStatus::Ok);
            assert_eq!(take(event as *mut PamojaBuffer), b"heater.off");

            pamoja_event_bus_free(bus);
        }
    }

    #[test]
    fn a_timed_next_runs_out_and_leaves_the_next_event() {
        unsafe {
            let bus = pamoja_event_bus_new(8);
            let mut event = ptr::null_mut();
            let mut timed_out = false;
            assert_eq!(
                pamoja_event_bus_next_within(bus, 20, &mut event, &mut timed_out),
                PamojaStatus::Ok
            );
            assert!(timed_out);
            assert!(event.is_null());

            pamoja_event_bus_publish(bus, b"link.up".as_ptr(), 7);
            assert_eq!(
                pamoja_event_bus_next_within(bus, 1_000, &mut event, &mut timed_out),
                PamojaStatus::Ok
            );
            assert!(!timed_out);
            assert_eq!(take(event), b"link.up");

            pamoja_event_bus_publish(bus, b"link.down".as_ptr(), 9);
            assert_eq!(
                pamoja_event_bus_next_within(bus, u64::MAX, &mut event, &mut timed_out),
                PamojaStatus::Ok
            );
            assert!(!timed_out);
            assert_eq!(take(event), b"link.down");

            pamoja_event_bus_free(bus);
        }
    }

    #[test]
    fn a_publisher_counts_its_subscribers_and_an_endpoint_counts_what_it_missed() {
        unsafe {
            let power = pamoja_event_publisher_new(2);
            let mut reached = usize::MAX;
            assert_eq!(
                pamoja_event_publisher_publish(power, b"0".as_ptr(), 1, &mut reached),
                PamojaStatus::Ok
            );
            assert_eq!(reached, 0);

            let logger = pamoja_event_publisher_subscribe(power);
            let sampler = pamoja_event_publisher_clone(power);
            for sample in [b"1", b"2", b"3", b"4", b"5"] {
                pamoja_event_publisher_publish(sampler, sample.as_ptr(), 1, &mut reached);
                assert_eq!(reached, 1);
            }
            pamoja_event_publisher_free(sampler);

            let mut event = ptr::null_mut();
            assert_eq!(pamoja_event_bus_next(logger, &mut event), PamojaStatus::Ok);
            assert_eq!(take(event), b"4");
            assert_eq!(pamoja_event_bus_missed(logger), 3);

            pamoja_event_bus_free(logger);
            pamoja_event_publisher_free(power);
        }
    }

    #[test]
    fn a_null_handle_is_refused_rather_than_dereferenced() {
        unsafe {
            assert!(pamoja_event_bus_subscribe(ptr::null()).is_null());
            assert!(pamoja_event_bus_publisher(ptr::null()).is_null());
            assert!(pamoja_event_publisher_subscribe(ptr::null()).is_null());
            assert!(pamoja_event_publisher_clone(ptr::null()).is_null());
            assert_eq!(
                pamoja_event_bus_publish(ptr::null(), b"x".as_ptr(), 1),
                PamojaStatus::InvalidArgument
            );
            let mut reached = 0;
            assert_eq!(
                pamoja_event_publisher_publish(ptr::null(), b"x".as_ptr(), 1, &mut reached),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(pamoja_event_bus_missed(ptr::null()), 0);
            pamoja_event_bus_free(ptr::null_mut());
            pamoja_event_publisher_free(ptr::null_mut());
        }
    }
}
