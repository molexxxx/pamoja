//! The curated C ABI surface for the pamoja SDK.
//!
//! This crate exposes a small, hand-written `extern "C"` API over
//! [`pamoja_core`] and the capability crates so that languages without a native
//! Rust bridge - C, C++, and C#/.NET through P/Invoke - can drive the SDK. It is
//! deliberately the project's single auditable `unsafe` boundary: every raw
//! pointer is dereferenced here and nowhere else.
//!
//! The committed header `include/pamoja.h` is generated from this source by
//! `cbindgen` (see `build.rs`) and is drift-checked in CI, so the C contract can
//! never fall behind the Rust surface.
//!
//! # Conventions
//!
//! - Fallible calls return a [`PamojaStatus`] code. On any non-`Ok` result a
//!   human-readable message is stored for the calling thread and can be read with
//!   [`pamoja_last_error_message`].
//! - Handles are opaque, heap-allocated, and owned by the caller, who must release
//!   each with its matching `*_free` function.
//! - All strings crossing the boundary are UTF-8. Inputs are borrowed for the
//!   duration of the call; returned pointers document their own lifetime.

// This crate is the FFI boundary, so raw-pointer work is its entire purpose; the
// workspace `unsafe_code = "warn"` lint is therefore allowed here. Safety is kept
// reviewable by confining every `unsafe` block to this crate.
#![allow(unsafe_code)]

use std::cell::RefCell;
use std::ffi::{c_char, CString};
use std::ptr;
use std::sync::OnceLock;

use pamoja_core::Error;

// The capability modules are public so every item the C ABI exports, including
// the buffer-size constants a caller sizes an array from, stays reachable from
// the crate root. A constant referenced only from the generated header reads as
// dead code otherwise.
#[cfg(feature = "actuators")]
pub mod actuators;
#[cfg(feature = "audit")]
pub mod audit;
#[cfg(feature = "bus")]
pub mod bus;
#[cfg(feature = "can")]
pub mod can;
#[cfg(feature = "coap")]
pub mod coap;
#[cfg(feature = "codec")]
pub mod codec;
#[cfg(feature = "gpio")]
pub mod gpio;
#[cfg(feature = "kit")]
pub mod kit;
#[cfg(feature = "ladder")]
pub mod ladder;
#[cfg(feature = "loopback")]
pub mod loopback;
#[cfg(feature = "lora")]
pub mod lora;
#[cfg(feature = "lora")]
pub mod lora_region;
#[cfg(feature = "lorawan")]
pub mod lorawan;
#[cfg(feature = "mavlink")]
pub mod mavlink;
#[cfg(feature = "mavlink")]
pub mod mavlink_schema;
#[cfg(feature = "mesh")]
pub mod mesh;
#[cfg(feature = "modbus")]
pub mod modbus;
#[cfg(feature = "mqtt")]
pub mod mqtt;
#[cfg(feature = "power")]
pub mod power;
#[cfg(feature = "profile")]
pub mod profile;
#[cfg(feature = "ros2")]
pub mod ros2;
#[cfg(feature = "routing")]
pub mod routing;
#[cfg(feature = "security")]
pub mod security;
#[cfg(feature = "sensors")]
pub mod sensors;
#[cfg(feature = "serial")]
pub mod serial;
#[cfg(feature = "session")]
pub mod session;
#[cfg(feature = "sim")]
pub mod sim;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "telemetry")]
pub mod telemetry;
#[cfg(feature = "runtime")]
pub mod transport;
#[cfg(feature = "update")]
pub mod update;
#[cfg(feature = "zenoh")]
pub mod zenoh;

/// The result of a fallible pamoja call.
///
/// A return of [`PamojaStatus::Ok`] means success; any other value indicates a
/// failure whose description is available from [`pamoja_last_error_message`] on
/// the same thread.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaStatus {
    /// The call succeeded.
    Ok = 0,
    /// A transport-level failure while connecting, sending, or receiving.
    Transport = 1,
    /// A device or peripheral input/output operation failed.
    Io = 2,
    /// A payload could not be encoded or decoded.
    Codec = 3,
    /// The operation targeted a resource that is closed or disconnected.
    Closed = 4,
    /// The requested capability is not compiled into this build.
    Unsupported = 5,
    /// An argument was null or otherwise invalid, for example non-UTF-8 text.
    InvalidArgument = 6,
    /// A failure that does not map onto a more specific status.
    Other = 7,
    /// A Rust panic was caught at the boundary; the call had no effect.
    Panic = 8,
    /// A security check failed, such as an invalid identity or a bad signature.
    Auth = 9,
}

impl PamojaStatus {
    /// Maps a core [`Error`] onto the matching status code.
    ///
    /// # Arguments
    ///
    /// * `error` - the error returned by a core or capability call.
    ///
    /// # Returns
    ///
    /// The [`PamojaStatus`] that classifies `error`.
    pub(crate) fn from_error(error: &Error) -> Self {
        match error {
            Error::Transport(_) => Self::Transport,
            Error::Io(_) => Self::Io,
            Error::Codec(_) => Self::Codec,
            Error::Closed => Self::Closed,
            Error::Auth(_) => Self::Auth,
            Error::Unsupported(_) => Self::Unsupported,
            _ => Self::Other,
        }
    }
}

thread_local! {
    /// The most recent error message produced on this thread.
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Records `message` as the calling thread's most recent error.
///
/// # Arguments
///
/// * `message` - the human-readable description to expose through
///   [`pamoja_last_error_message`]. Any interior null byte is replaced with a
///   generic message so the value always stores cleanly as a C string.
pub(crate) fn set_last_error(message: String) {
    let value =
        CString::new(message).unwrap_or_else(|_| CString::new("pamoja error").expect("static"));
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(value));
}

/// Returns the calling thread's most recent error message, or null if none.
///
/// # Returns
///
/// A pointer to a null-terminated UTF-8 string owned by the library, valid until
/// the next failing call on the same thread, or null if no error has been
/// recorded. The caller must not free it and should copy it before making another
/// pamoja call on this thread.
#[no_mangle]
pub extern "C" fn pamoja_last_error_message() -> *const c_char {
    LAST_ERROR.with(|slot| match &*slot.borrow() {
        Some(value) => value.as_ptr(),
        None => ptr::null(),
    })
}

/// Copies a borrowed byte buffer, treating a zero length as an empty payload.
///
/// # Safety
///
/// When `len` is non-zero, `ptr` must point to at least `len` readable bytes.
#[cfg(any(
    feature = "audit",
    feature = "can",
    feature = "codec",
    feature = "lora",
    feature = "lorawan",
    feature = "mavlink",
    feature = "mesh",
    feature = "modbus",
    feature = "mqtt",
    feature = "ros2",
    feature = "runtime",
    feature = "security",
    feature = "sensors",
    feature = "serial",
    feature = "session",
    feature = "update"
))]
pub(crate) unsafe fn read_bytes(ptr: *const u8, len: usize) -> Result<Vec<u8>, PamojaStatus> {
    if len == 0 {
        Ok(Vec::new())
    } else if ptr.is_null() {
        set_last_error("payload must not be null when its length is non-zero".to_owned());
        Err(PamojaStatus::InvalidArgument)
    } else {
        Ok(std::slice::from_raw_parts(ptr, len).to_vec())
    }
}

/// An opaque handle to a byte buffer owned by the caller.
///
/// Calls that produce a variable-length result hand back one of these rather than
/// writing into a caller buffer, so the caller never has to guess a size. Read it
/// with [`pamoja_buffer_data`] and [`pamoja_buffer_len`], then release it with
/// [`pamoja_buffer_free`].
#[cfg(any(
    feature = "audit",
    feature = "bus",
    feature = "codec",
    feature = "lorawan",
    feature = "modbus",
    feature = "ros2",
    feature = "serial",
    feature = "sync",
    feature = "update"
))]
pub struct PamojaBuffer {
    bytes: Vec<u8>,
}

#[cfg(any(
    feature = "audit",
    feature = "bus",
    feature = "codec",
    feature = "lorawan",
    feature = "modbus",
    feature = "ros2",
    feature = "serial",
    feature = "sync",
    feature = "update"
))]
impl PamojaBuffer {
    /// Wraps owned bytes in a heap-allocated handle for the caller to own.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the buffer contents to hand across the boundary.
    ///
    /// # Returns
    ///
    /// A raw handle the caller must release with [`pamoja_buffer_free`].
    pub(crate) fn into_raw(bytes: Vec<u8>) -> *mut Self {
        Box::into_raw(Box::new(Self { bytes }))
    }
}

/// Returns a pointer to a buffer's bytes.
///
/// Use [`pamoja_buffer_len`] for the length. The pointer is valid until the
/// buffer is freed.
///
/// # Returns
///
/// A pointer to the bytes, or null if `buffer` is null.
///
/// # Safety
///
/// `buffer` must be a live handle from a pamoja call that produced one, or null.
#[cfg(any(
    feature = "audit",
    feature = "bus",
    feature = "codec",
    feature = "lorawan",
    feature = "modbus",
    feature = "ros2",
    feature = "serial",
    feature = "sync",
    feature = "update"
))]
#[no_mangle]
pub unsafe extern "C" fn pamoja_buffer_data(buffer: *const PamojaBuffer) -> *const u8 {
    if buffer.is_null() {
        return ptr::null();
    }
    (*buffer).bytes.as_ptr()
}

/// Returns the length in bytes of a buffer.
///
/// # Returns
///
/// The length, or 0 if `buffer` is null.
///
/// # Safety
///
/// `buffer` must be a live handle from a pamoja call that produced one, or null.
#[cfg(any(
    feature = "audit",
    feature = "bus",
    feature = "codec",
    feature = "lorawan",
    feature = "modbus",
    feature = "ros2",
    feature = "serial",
    feature = "sync",
    feature = "update"
))]
#[no_mangle]
pub unsafe extern "C" fn pamoja_buffer_len(buffer: *const PamojaBuffer) -> usize {
    if buffer.is_null() {
        return 0;
    }
    (*buffer).bytes.len()
}

/// Releases a buffer handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `buffer` must be a handle from a pamoja call that produced one and that has
/// not already been freed, or null. After this call it must not be used again.
#[cfg(any(
    feature = "audit",
    feature = "bus",
    feature = "codec",
    feature = "lorawan",
    feature = "modbus",
    feature = "ros2",
    feature = "serial",
    feature = "sync",
    feature = "update"
))]
#[no_mangle]
pub unsafe extern "C" fn pamoja_buffer_free(buffer: *mut PamojaBuffer) {
    if !buffer.is_null() {
        drop(Box::from_raw(buffer));
    }
}

/// An owned, null-terminated UTF-8 string produced by the library.
///
/// Some calls build a string rather than borrowing one that already lives inside
/// a handle: a canonical key expression, a DDS topic name, a profile serialized
/// to JSON. Those return this, and the caller releases it with
/// [`pamoja_string_free`].
#[cfg(any(
    feature = "lora",
    feature = "mavlink",
    feature = "profile",
    feature = "ros2",
    feature = "zenoh"
))]
pub struct PamojaString {
    text: CString,
}

#[cfg(any(
    feature = "lora",
    feature = "mavlink",
    feature = "profile",
    feature = "ros2",
    feature = "zenoh"
))]
impl PamojaString {
    /// Wraps an owned string in a heap-allocated handle for the caller to own.
    ///
    /// # Arguments
    ///
    /// * `text` - the string to hand across the boundary.
    ///
    /// # Returns
    ///
    /// A raw handle the caller must release with [`pamoja_string_free`], or null
    /// if `text` contains an interior null byte.
    pub(crate) fn into_raw(text: String) -> *mut Self {
        match CString::new(text) {
            Ok(text) => Box::into_raw(Box::new(Self { text })),
            Err(_) => {
                set_last_error("the string contains an interior null byte".to_owned());
                ptr::null_mut()
            }
        }
    }
}

/// Returns a pointer to a string's bytes.
///
/// The pointer is valid until the string is freed.
///
/// # Returns
///
/// A null-terminated UTF-8 string, or null if `string` is null.
///
/// # Safety
///
/// `string` must be a live handle from a call that produced one, or null. After
/// [`pamoja_string_free`] it must not be used again.
#[cfg(any(
    feature = "lora",
    feature = "mavlink",
    feature = "profile",
    feature = "ros2",
    feature = "zenoh"
))]
#[no_mangle]
pub unsafe extern "C" fn pamoja_string_data(string: *const PamojaString) -> *const c_char {
    if string.is_null() {
        return ptr::null();
    }
    (*string).text.as_ptr()
}

/// Returns the length in bytes of a string, excluding its null terminator.
///
/// # Returns
///
/// The byte length, or 0 if `string` is null.
///
/// # Safety
///
/// `string` must be a live handle from a call that produced one, or null.
#[cfg(any(
    feature = "lora",
    feature = "mavlink",
    feature = "profile",
    feature = "ros2",
    feature = "zenoh"
))]
#[no_mangle]
pub unsafe extern "C" fn pamoja_string_len(string: *const PamojaString) -> usize {
    if string.is_null() {
        return 0;
    }
    (*string).text.as_bytes().len()
}

/// Releases a string handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `string` must be a handle from a call that produced one and that has not
/// already been freed, or null. After this call it must not be used again.
#[cfg(any(
    feature = "lora",
    feature = "mavlink",
    feature = "profile",
    feature = "ros2",
    feature = "zenoh"
))]
#[no_mangle]
pub unsafe extern "C" fn pamoja_string_free(string: *mut PamojaString) {
    if !string.is_null() {
        drop(Box::from_raw(string));
    }
}

/// Borrows a C string argument as `&str`, recording an error on null or non-UTF-8.
///
/// # Safety
///
/// `ptr` must be a valid null-terminated string for the duration of the call, or
/// null.
#[cfg(any(
    feature = "coap",
    feature = "ladder",
    feature = "lora",
    feature = "mavlink",
    feature = "mqtt",
    feature = "profile",
    feature = "ros2",
    feature = "runtime",
    feature = "sync",
    feature = "zenoh"
))]
pub(crate) unsafe fn read_str<'a>(ptr: *const c_char, name: &str) -> Option<&'a str> {
    if ptr.is_null() {
        set_last_error(format!("{name} must not be null"));
        return None;
    }
    match std::ffi::CStr::from_ptr(ptr).to_str() {
        Ok(value) => Some(value),
        Err(_) => {
            set_last_error(format!("{name} must be valid UTF-8"));
            None
        }
    }
}

/// The process-wide runtime that drives every blocking async call.
#[cfg(feature = "runtime")]
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Returns the shared Tokio runtime, building it on first use.
///
/// A multi-threaded runtime is required because several transports spawn a
/// background event loop that has to keep running after a `block_on` returns.
/// Every async capability shares this one executor, so a process that uses two
/// of them does not carry two runtimes.
#[cfg(feature = "runtime")]
pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build the pamoja tokio runtime")
    })
}

/// The version of the native pamoja library.
static VERSION: OnceLock<CString> = OnceLock::new();

/// Returns the version string of the native pamoja library.
///
/// # Returns
///
/// A pointer to a static null-terminated UTF-8 string owned by the library. The
/// caller must not free it; it is valid for the lifetime of the process.
#[no_mangle]
pub extern "C" fn pamoja_version() -> *const c_char {
    VERSION
        .get_or_init(|| CString::new(env!("CARGO_PKG_VERSION")).expect("version has no null byte"))
        .as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_maps_each_error_variant() {
        assert!(matches!(
            PamojaStatus::from_error(&Error::Transport("x".into())),
            PamojaStatus::Transport
        ));
        assert!(matches!(
            PamojaStatus::from_error(&Error::Io("x".into())),
            PamojaStatus::Io
        ));
        assert!(matches!(
            PamojaStatus::from_error(&Error::Codec("x".into())),
            PamojaStatus::Codec
        ));
        assert!(matches!(
            PamojaStatus::from_error(&Error::Closed),
            PamojaStatus::Closed
        ));
        assert!(matches!(
            PamojaStatus::from_error(&Error::Unsupported("mqtt")),
            PamojaStatus::Unsupported
        ));
    }

    #[test]
    fn version_is_a_non_empty_c_string() {
        let ptr = pamoja_version();
        assert!(!ptr.is_null());
        // Safety: `pamoja_version` returns a valid static C string.
        let version = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(version.to_str().expect("utf-8"), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn last_error_round_trips_on_this_thread() {
        set_last_error("transport error: boom".to_owned());
        let ptr = pamoja_last_error_message();
        assert!(!ptr.is_null());
        // Safety: a message was just recorded on this thread.
        let message = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(message.to_str().expect("utf-8"), "transport error: boom");
    }
}
