//! A transport the host implements: callbacks that cross the C boundary.
//!
//! Everything else in this crate drives a transport pamoja wrote. This module
//! goes the other way: the host supplies the functions that connect, send,
//! subscribe, and receive, and pamoja wraps them in a transport that composes
//! like any other, as a ladder rung, under a fault injector, or driven directly.
//! It is how a link pamoja does not ship, a vendor SDK, a proprietary radio, a
//! cloud client, or a class in a language binding, takes its place on a node.
//!
//! # The contract
//!
//! A host fills a [`PamojaTransportCallbacks`] and hands it, with a `user_data`
//! pointer, to [`pamoja_transport_from_callbacks`]. Every callback receives that
//! pointer back. The callbacks are ordinary blocking functions:
//!
//! - `connect`, `send`, and `subscribe` are called one at a time per transport,
//!   from a thread pamoja owns, never the caller's thread. Each returns
//!   [`PamojaStatus::Ok`] or a status describing the failure; a host that wants
//!   the failure explained sets the text with [`pamoja_last_error_set`] on the
//!   same thread before returning.
//! - `recv` is optional. A link that only sends, an uplink, leaves it null and is
//!   never listened on. When present it is called repeatedly from one background
//!   thread, from the moment `connect` succeeds, each call as soon as the previous
//!   returned. It blocks until a message is available, builds the message with
//!   [`pamoja_message_new`], stores it in `out_message`, and returns
//!   [`PamojaStatus::Ok`]; it stores null and returns [`PamojaStatus::Ok`] once
//!   the link has ended and no further messages will arrive, which stops the
//!   calls. Because it runs on its own thread, it may overlap with the other
//!   three callbacks.
//! - `release` is optional. It is called once, after the transport is dropped and
//!   after every callback still running has returned, so it may free whatever
//!   `user_data` points at.
//!
//! Messages the host delivers are queued inside the transport, so a receive
//! through pamoja is cancel-safe whatever the host's `recv` does.

use std::ffi::{c_char, c_void, CString};
use std::ptr;
use std::sync::Arc;

use pamoja_core::{Error, Message, Receive, Result, Transport};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::transport::{Kind, PamojaMessage, PamojaTransport};
use crate::{read_bytes, read_str, set_last_error, take_last_error, PamojaStatus};

/// The functions a host supplies to stand as a transport.
///
/// `connect`, `send`, and `subscribe` are required. `recv` is null for a link that
/// only sends. `release` is null when `user_data` needs no cleanup. The threading
/// rules are in the module documentation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PamojaTransportCallbacks {
    /// Establishes the link. Called once per `connect` on the transport.
    pub connect: Option<unsafe extern "C" fn(user_data: *mut c_void) -> PamojaStatus>,
    /// Publishes `payload_len` bytes at `payload` to the null-terminated UTF-8
    /// `topic`. Both pointers are valid only for the duration of the call.
    pub send: Option<
        unsafe extern "C" fn(
            user_data: *mut c_void,
            topic: *const c_char,
            payload: *const u8,
            payload_len: usize,
        ) -> PamojaStatus,
    >,
    /// Subscribes to the null-terminated UTF-8 `topic` filter, valid only for the
    /// duration of the call.
    pub subscribe:
        Option<unsafe extern "C" fn(user_data: *mut c_void, topic: *const c_char) -> PamojaStatus>,
    /// Waits for the next message on a subscribed topic and stores it in
    /// `out_message`, or stores null once the link has ended. Null for a link that
    /// only sends.
    pub recv: Option<
        unsafe extern "C" fn(
            user_data: *mut c_void,
            out_message: *mut *mut PamojaMessage,
        ) -> PamojaStatus,
    >,
    /// Frees whatever `user_data` refers to, once nothing will call the host again.
    pub release: Option<unsafe extern "C" fn(user_data: *mut c_void)>,
}

/// The host's side of a transport: its callbacks and the pointer they receive.
///
/// Shared between the transport and every callback in flight, so `release` runs
/// only after the last of them has returned.
struct Host {
    callbacks: PamojaTransportCallbacks,
    user_data: *mut c_void,
}

// SAFETY: the callbacks are documented as callable from any thread, with the
// serialization the module documentation states, and the pointer is only ever
// handed back to them. Nothing here reads through it.
unsafe impl Send for Host {}
unsafe impl Sync for Host {}

impl Drop for Host {
    fn drop(&mut self) {
        if let Some(release) = self.callbacks.release {
            // SAFETY: the host promised `release` accepts the pointer it gave
            // `pamoja_transport_from_callbacks`, and this runs once, after every
            // callback holding the host has returned.
            unsafe { release(self.user_data) }
        }
    }
}

/// Turns the status a callback returned, and any text it set on this thread, into
/// a core result. Runs on the thread that made the call so the text is the one the
/// host set.
fn outcome(status: PamojaStatus) -> Result<()> {
    if status == PamojaStatus::Ok {
        return Ok(());
    }
    let reason =
        take_last_error().unwrap_or_else(|| format!("the host transport reported {status:?}"));
    Err(match status {
        PamojaStatus::Io => Error::Io(reason),
        PamojaStatus::Codec => Error::Codec(reason),
        PamojaStatus::Closed => Error::Closed,
        PamojaStatus::Auth => Error::Auth(reason),
        _ => Error::Transport(reason),
    })
}

/// Runs a host callback on the runtime's blocking pool, so a host that blocks does
/// not stall the tasks driving other transports.
async fn blocking<T: Send + 'static>(call: impl FnOnce() -> T + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(call)
        .await
        .map_err(|error| Error::Transport(format!("the host callback thread failed: {error}")))
}

/// Converts a topic to the C string a callback receives.
fn c_topic(topic: &str) -> Result<CString> {
    CString::new(topic)
        .map_err(|_| Error::Transport("a topic cannot carry an interior null byte".to_owned()))
}

/// A transport whose every operation is a host callback.
///
/// Built by [`pamoja_transport_from_callbacks`] and held inside a
/// [`PamojaTransport`], so it composes wherever a transport does.
pub struct CallbackTransport {
    host: Arc<Host>,
    connected: bool,
    inbox: Option<mpsc::UnboundedReceiver<Message>>,
    pump: Option<JoinHandle<()>>,
}

impl CallbackTransport {
    /// Whether the host supplied a `recv`, so the transport delivers and a ladder
    /// listens on it.
    pub(crate) fn listens(&self) -> bool {
        self.host.callbacks.recv.is_some()
    }

    /// Starts the thread that asks the host for messages until the link ends.
    fn start_pump(&mut self) {
        let Some(recv) = self.host.callbacks.recv else {
            return;
        };
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
        let (sender, receiver) = mpsc::unbounded_channel();
        let host = Arc::clone(&self.host);
        self.inbox = Some(receiver);
        self.pump = Some(tokio::spawn(async move {
            loop {
                let host = Arc::clone(&host);
                let next = blocking(move || {
                    let mut out_message: *mut PamojaMessage = ptr::null_mut();
                    // SAFETY: the host promised `recv` accepts its pointer and a
                    // writable slot, and `out_message` is either null or a message
                    // built with `pamoja_message_new`, which this side now owns.
                    let status = unsafe { recv(host.user_data, &mut out_message) };
                    let delivered = (!out_message.is_null())
                        .then(|| unsafe { Box::from_raw(out_message) }.into_message());
                    outcome(status).map(|()| delivered)
                })
                .await;
                let Ok(Ok(Some(message))) = next else {
                    break;
                };
                if sender.send(message).is_err() {
                    break;
                }
            }
        }));
    }
}

impl Drop for CallbackTransport {
    fn drop(&mut self) {
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
    }
}

impl Transport for CallbackTransport {
    async fn connect(&mut self) -> Result<()> {
        let host = Arc::clone(&self.host);
        blocking(move || {
            let connect = host.callbacks.connect.expect("required at creation");
            // SAFETY: the host promised `connect` accepts its pointer.
            outcome(unsafe { connect(host.user_data) })
        })
        .await??;
        self.connected = true;
        self.start_pump();
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let host = Arc::clone(&self.host);
        let topic = c_topic(topic)?;
        let payload = payload.to_vec();
        blocking(move || {
            let send = host.callbacks.send.expect("required at creation");
            // SAFETY: the host promised `send` accepts its pointer, a C string, and
            // a byte range, all of which outlive the call.
            outcome(unsafe {
                send(
                    host.user_data,
                    topic.as_ptr(),
                    payload.as_ptr(),
                    payload.len(),
                )
            })
        })
        .await?
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let host = Arc::clone(&self.host);
        let topic = c_topic(topic)?;
        blocking(move || {
            let subscribe = host.callbacks.subscribe.expect("required at creation");
            // SAFETY: the host promised `subscribe` accepts its pointer and a C
            // string that outlives the call.
            outcome(unsafe { subscribe(host.user_data, topic.as_ptr()) })
        })
        .await?
    }
}

impl Receive for CallbackTransport {
    /// Awaits the next message the host delivered.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next queued message; `None` once the host reported
    /// the link ended, or at once for a host with no `recv`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if the transport is not connected.
    async fn recv(&mut self) -> Result<Option<Message>> {
        if !self.connected {
            return Err(Error::Closed);
        }
        match self.inbox.as_mut() {
            Some(inbox) => Ok(inbox.recv().await),
            None => Ok(None),
        }
    }
}

/// Wraps host callbacks in a transport.
///
/// # Arguments
///
/// * `callbacks` - the host's functions, copied by this call; the struct need not
///   outlive it.
/// * `user_data` - the pointer every callback receives, owned by the host until
///   `release` is called with it.
///
/// # Returns
///
/// A handle the caller releases with [`pamoja_transport_free`] or hands to a call
/// that consumes it, such as adding it to a ladder. Null if `callbacks` is null or
/// lacks `connect`, `send`, or `subscribe`; the host then keeps `user_data` and
/// `release` is never called.
///
/// # Safety
///
/// `callbacks` must point to a valid struct for the duration of the call, and
/// each function in it must honor the contract in the module documentation for as
/// long as the transport, or any transport composed from it, lives.
///
/// [`pamoja_transport_free`]: crate::transport::pamoja_transport_free
#[no_mangle]
pub unsafe extern "C" fn pamoja_transport_from_callbacks(
    callbacks: *const PamojaTransportCallbacks,
    user_data: *mut c_void,
) -> *mut PamojaTransport {
    if callbacks.is_null() {
        set_last_error("callbacks must not be null".to_owned());
        return ptr::null_mut();
    }
    let callbacks = *callbacks;
    if callbacks.connect.is_none() || callbacks.send.is_none() || callbacks.subscribe.is_none() {
        set_last_error("a host transport needs connect, send, and subscribe callbacks".to_owned());
        return ptr::null_mut();
    }
    let transport = CallbackTransport {
        host: Arc::new(Host {
            callbacks,
            user_data,
        }),
        connected: false,
        inbox: None,
        pump: None,
    };
    PamojaTransport::into_raw(Kind::Host(transport))
}

/// Builds a message for a host's `recv` callback to deliver.
///
/// # Arguments
///
/// * `topic` - the topic the message arrived on, as null-terminated UTF-8.
/// * `payload` - the payload bytes, or null when `payload_len` is 0.
/// * `payload_len` - the length of `payload`.
///
/// # Returns
///
/// A message the receiving side owns once it is stored in `out_message`; release
/// it with [`pamoja_message_free`] only if it is not handed over. Null if `topic`
/// is null or not UTF-8, or `payload` is null with a nonzero length.
///
/// # Safety
///
/// `topic` must be a valid null-terminated string and `payload` must point to at
/// least `payload_len` readable bytes or be null when that length is 0.
///
/// [`pamoja_message_free`]: crate::transport::pamoja_message_free
#[no_mangle]
pub unsafe extern "C" fn pamoja_message_new(
    topic: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> *mut PamojaMessage {
    let Some(topic) = read_str(topic, "topic") else {
        return ptr::null_mut();
    };
    let Ok(payload) = read_bytes(payload, payload_len) else {
        return ptr::null_mut();
    };
    PamojaMessage::into_raw(topic.to_owned(), payload)
}

/// Records the text a host callback wants attached to the status it returns.
///
/// Call it on the thread the callback is running on, before returning the
/// status; pamoja reads it there and carries it to whoever made the call, where
/// [`pamoja_last_error_message`] then reports it.
///
/// # Arguments
///
/// * `message` - the description as null-terminated UTF-8, or null to clear it.
///
/// # Safety
///
/// `message` must be a valid null-terminated string, or null.
///
/// [`pamoja_last_error_message`]: crate::pamoja_last_error_message
#[no_mangle]
pub unsafe extern "C" fn pamoja_last_error_set(message: *const c_char) {
    match read_str(message, "message") {
        Some(message) => set_last_error(message.to_owned()),
        None => drop(take_last_error()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::VecDeque;
    use std::ffi::CStr;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Condvar, Mutex};

    use crate::transport::{
        pamoja_message_free, pamoja_message_payload, pamoja_message_payload_len,
        pamoja_message_topic, pamoja_transport_connect, pamoja_transport_free,
        pamoja_transport_recv, pamoja_transport_send, pamoja_transport_subscribe,
    };
    use crate::{pamoja_last_error_message, PamojaStatus};

    /// A host written in Rust: records what it is asked to do and delivers what the
    /// test feeds it.
    #[derive(Default)]
    struct FakeHost {
        connects: Mutex<usize>,
        sent: Mutex<Vec<(String, Vec<u8>)>>,
        filters: Mutex<Vec<String>>,
        inbox: Mutex<VecDeque<Option<Message>>>,
        ready: Condvar,
        refuse_sends: AtomicBool,
        released: Arc<AtomicBool>,
    }

    impl FakeHost {
        fn deliver(&self, next: Option<Message>) {
            self.inbox.lock().expect("inbox").push_back(next);
            self.ready.notify_all();
        }
    }

    unsafe extern "C" fn connect(user_data: *mut c_void) -> PamojaStatus {
        let host = &*(user_data as *const FakeHost);
        *host.connects.lock().expect("connects") += 1;
        PamojaStatus::Ok
    }

    unsafe extern "C" fn send(
        user_data: *mut c_void,
        topic: *const c_char,
        payload: *const u8,
        payload_len: usize,
    ) -> PamojaStatus {
        let host = &*(user_data as *const FakeHost);
        if host.refuse_sends.load(Ordering::SeqCst) {
            let reason = CString::new("the radio is out of range").expect("static");
            pamoja_last_error_set(reason.as_ptr());
            return PamojaStatus::Transport;
        }
        let topic = CStr::from_ptr(topic).to_str().expect("utf-8").to_owned();
        let payload = std::slice::from_raw_parts(payload, payload_len).to_vec();
        host.sent.lock().expect("sent").push((topic, payload));
        PamojaStatus::Ok
    }

    unsafe extern "C" fn subscribe(user_data: *mut c_void, topic: *const c_char) -> PamojaStatus {
        let host = &*(user_data as *const FakeHost);
        let topic = CStr::from_ptr(topic).to_str().expect("utf-8").to_owned();
        host.filters.lock().expect("filters").push(topic);
        PamojaStatus::Ok
    }

    unsafe extern "C" fn recv(
        user_data: *mut c_void,
        out_message: *mut *mut PamojaMessage,
    ) -> PamojaStatus {
        let host = &*(user_data as *const FakeHost);
        let mut inbox = host.inbox.lock().expect("inbox");
        while inbox.is_empty() {
            inbox = host.ready.wait(inbox).expect("inbox");
        }
        *out_message = match inbox.pop_front().expect("non-empty") {
            Some(message) => {
                let topic = CString::new(message.topic).expect("topic");
                pamoja_message_new(
                    topic.as_ptr(),
                    message.payload.as_ptr(),
                    message.payload.len(),
                )
            }
            None => ptr::null_mut(),
        };
        PamojaStatus::Ok
    }

    unsafe extern "C" fn release(user_data: *mut c_void) {
        let host = Box::from_raw(user_data as *mut FakeHost);
        host.released.store(true, Ordering::SeqCst);
    }

    /// Leaks a fake host to the C side and returns the callbacks that reach it.
    fn fake_host(listens: bool) -> (*mut FakeHost, PamojaTransportCallbacks, Arc<AtomicBool>) {
        let host = Box::new(FakeHost::default());
        let released = Arc::clone(&host.released);
        let host = Box::into_raw(host);
        let callbacks = PamojaTransportCallbacks {
            connect: Some(connect),
            send: Some(send),
            subscribe: Some(subscribe),
            recv: listens.then_some(recv as unsafe extern "C" fn(_, _) -> _),
            release: Some(release),
        };
        (host, callbacks, released)
    }

    fn last_error() -> String {
        unsafe {
            CStr::from_ptr(pamoja_last_error_message())
                .to_str()
                .expect("utf-8")
                .to_owned()
        }
    }

    #[test]
    fn the_host_carries_sends_and_delivers_what_it_receives() {
        unsafe {
            let (host, callbacks, released) = fake_host(true);
            let transport = pamoja_transport_from_callbacks(&callbacks, host.cast());
            assert!(!transport.is_null());
            let topic = CString::new("sensors/1").expect("static");

            assert_eq!(pamoja_transport_connect(transport), PamojaStatus::Ok);
            assert_eq!(
                pamoja_transport_subscribe(transport, topic.as_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_transport_send(transport, topic.as_ptr(), b"21.5".as_ptr(), 4),
                PamojaStatus::Ok
            );
            assert_eq!(*(*host).connects.lock().expect("connects"), 1);
            assert_eq!(
                (*host).filters.lock().expect("filters").as_slice(),
                ["sensors/1"]
            );
            assert_eq!(
                (*host).sent.lock().expect("sent").as_slice(),
                [("sensors/1".to_owned(), b"21.5".to_vec())]
            );

            // What the host delivers comes out of the transport, in order.
            (*host).deliver(Some(Message::new("commands/1", b"open")));
            (*host).deliver(Some(Message::new("commands/1", b"close")));
            for expected in [b"open".as_slice(), b"close".as_slice()] {
                let mut message = ptr::null_mut();
                assert_eq!(
                    pamoja_transport_recv(transport, &mut message),
                    PamojaStatus::Ok
                );
                assert!(!message.is_null());
                assert_eq!(
                    CStr::from_ptr(pamoja_message_topic(message)).to_bytes(),
                    b"commands/1"
                );
                let payload = std::slice::from_raw_parts(
                    pamoja_message_payload(message),
                    pamoja_message_payload_len(message),
                );
                assert_eq!(payload, expected);
                pamoja_message_free(message);
            }

            // The host ends the link: the receive reports it, and once the transport
            // goes the host is released, after its last recv returned.
            (*host).deliver(None);
            let mut message = ptr::null_mut();
            assert_eq!(
                pamoja_transport_recv(transport, &mut message),
                PamojaStatus::Ok
            );
            assert!(message.is_null());
            assert!(!released.load(Ordering::SeqCst));
            pamoja_transport_free(transport);
            for _ in 0..200 {
                if released.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            assert!(
                released.load(Ordering::SeqCst),
                "release runs after the pump ends"
            );
        }
    }

    #[test]
    fn a_host_that_refuses_a_send_explains_itself() {
        unsafe {
            let (host, callbacks, _released) = fake_host(false);
            let transport = pamoja_transport_from_callbacks(&callbacks, host.cast());
            let topic = CString::new("t").expect("static");
            assert_eq!(pamoja_transport_connect(transport), PamojaStatus::Ok);

            (*host).refuse_sends.store(true, Ordering::SeqCst);
            assert_eq!(
                pamoja_transport_send(transport, topic.as_ptr(), b"x".as_ptr(), 1),
                PamojaStatus::Transport
            );
            assert_eq!(last_error(), "transport error: the radio is out of range");

            // A send-only host reports an ended link rather than waiting.
            let mut message = ptr::null_mut();
            assert_eq!(
                pamoja_transport_recv(transport, &mut message),
                PamojaStatus::Ok
            );
            assert!(message.is_null());
            pamoja_transport_free(transport);
        }
    }

    #[test]
    fn operations_before_connect_report_closed() {
        unsafe {
            let (host, callbacks, _released) = fake_host(true);
            let transport = pamoja_transport_from_callbacks(&callbacks, host.cast());
            let topic = CString::new("t").expect("static");
            assert_eq!(
                pamoja_transport_send(transport, topic.as_ptr(), b"x".as_ptr(), 1),
                PamojaStatus::Closed
            );
            let mut message = ptr::null_mut();
            assert_eq!(
                pamoja_transport_recv(transport, &mut message),
                PamojaStatus::Closed
            );
            pamoja_transport_free(transport);
        }
    }

    #[test]
    fn incomplete_callbacks_are_refused_and_the_host_keeps_its_data() {
        unsafe {
            let (host, mut callbacks, released) = fake_host(true);
            callbacks.send = None;
            assert!(pamoja_transport_from_callbacks(&callbacks, host.cast()).is_null());
            assert_eq!(
                last_error(),
                "a host transport needs connect, send, and subscribe callbacks"
            );
            assert!(pamoja_transport_from_callbacks(ptr::null(), host.cast()).is_null());
            assert!(!released.load(Ordering::SeqCst));
            drop(Box::from_raw(host));
        }
    }

    #[test]
    fn a_message_needs_a_topic_and_a_readable_payload() {
        unsafe {
            let topic = CString::new("t").expect("static");
            assert!(pamoja_message_new(ptr::null(), ptr::null(), 0).is_null());
            assert!(pamoja_message_new(topic.as_ptr(), ptr::null(), 3).is_null());
            let empty = pamoja_message_new(topic.as_ptr(), ptr::null(), 0);
            assert!(!empty.is_null());
            assert_eq!(pamoja_message_payload_len(empty), 0);
            pamoja_message_free(empty);
        }
    }
}
