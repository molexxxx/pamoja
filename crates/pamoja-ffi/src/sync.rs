//! The C ABI for store-and-forward buffers.
//!
//! These functions wrap [`pamoja_sync`] for callers that reach the SDK through
//! the flat C boundary: the queue a node writes into while it has nowhere to
//! send, and the drain that empties it once a link comes back.
//!
//! Two stores cross, behind one handle. An in-memory queue is the right choice
//! for a test or a process that will not outlive its buffer; a file-backed one
//! survives a reboot, which is what a node in a place with no reliable power
//! actually needs. The kind is chosen when the store is created and nothing
//! afterwards has to care which it is.

use std::ffi::c_char;
use std::ptr;

use pamoja_core::{Result, Store};
use pamoja_sync::{drain_to, FileStore, MemoryStore};

use crate::transport::PamojaTransport;
use crate::{read_bytes, read_str, runtime, set_last_error, PamojaBuffer, PamojaStatus};

/// One buffer, whichever kind it was created as.
pub(crate) enum StoreKind {
    /// A queue held in memory, lost when the process ends.
    Memory(MemoryStore),
    /// A queue on disk, which survives a restart.
    File(FileStore),
}

impl Store for StoreKind {
    async fn append(&mut self, record: &[u8]) -> Result<()> {
        match self {
            StoreKind::Memory(store) => store.append(record).await,
            StoreKind::File(store) => store.append(record).await,
        }
    }

    async fn peek(&self) -> Result<Option<Vec<u8>>> {
        match self {
            StoreKind::Memory(store) => store.peek().await,
            StoreKind::File(store) => store.peek().await,
        }
    }

    async fn pop(&mut self) -> Result<Option<Vec<u8>>> {
        match self {
            StoreKind::Memory(store) => store.pop().await,
            StoreKind::File(store) => store.pop().await,
        }
    }

    async fn len(&self) -> Result<usize> {
        match self {
            StoreKind::Memory(store) => store.len().await,
            StoreKind::File(store) => store.len().await,
        }
    }
}

/// An opaque handle to a store-and-forward buffer.
pub struct PamojaStore {
    pub(crate) kind: StoreKind,
}

/// Creates a buffer held in memory.
///
/// # Arguments
///
/// * `capacity` - the most records to hold, or 0 for no bound. A full store
///   refuses the next append rather than dropping anything, so a record is
///   never lost without the caller being told.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_store_free`] or hand to a call
/// that consumes it.
#[no_mangle]
pub extern "C" fn pamoja_store_memory(capacity: usize) -> *mut PamojaStore {
    let store = if capacity == 0 {
        MemoryStore::new()
    } else {
        MemoryStore::with_capacity(capacity)
    };
    Box::into_raw(Box::new(PamojaStore {
        kind: StoreKind::Memory(store),
    }))
}

/// Opens a buffer backed by a directory, so it survives a restart.
///
/// # Arguments
///
/// * `dir` - the directory to hold records in, as null-terminated UTF-8. It is
///   created if it does not exist.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_store_free`] or hand to a call
/// that consumes it, or null if the directory cannot be opened.
///
/// # Safety
///
/// `dir` must be a valid null-terminated UTF-8 string for the duration of the
/// call.
#[no_mangle]
pub unsafe extern "C" fn pamoja_store_file(dir: *const c_char) -> *mut PamojaStore {
    let Some(dir) = read_str(dir, "dir") else {
        return ptr::null_mut();
    };
    match FileStore::open(dir) {
        Ok(store) => Box::into_raw(Box::new(PamojaStore {
            kind: StoreKind::File(store),
        })),
        Err(error) => {
            set_last_error(error.to_string());
            ptr::null_mut()
        }
    }
}

/// Adds a record to the end of a buffer.
///
/// # Arguments
///
/// * `store` - the buffer.
/// * `record` - the bytes to hold.
/// * `record_len` - the length of `record`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] once the record is held.
///
/// # Safety
///
/// `store` must be a live handle, and `record` must point to at least
/// `record_len` readable bytes or be null when that length is 0.
#[no_mangle]
pub unsafe extern "C" fn pamoja_store_append(
    store: *mut PamojaStore,
    record: *const u8,
    record_len: usize,
) -> PamojaStatus {
    let Some(store) = store_handle(store) else {
        return PamojaStatus::InvalidArgument;
    };
    let record = match read_bytes(record, record_len) {
        Ok(record) => record,
        Err(status) => return status,
    };
    match runtime().block_on(store.kind.append(&record)) {
        Ok(()) => PamojaStatus::Ok,
        Err(error) => fail(error),
    }
}

/// Reads the oldest record without removing it.
///
/// # Arguments
///
/// * `store` - the buffer.
/// * `out_record` - receives a buffer handle, or null when the store is empty.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. A null `out_record` with an `Ok` status
/// means the buffer is empty.
///
/// # Safety
///
/// `store` must be a live handle and `out_record` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_store_peek(
    store: *mut PamojaStore,
    out_record: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let Some(store) = store_handle(store) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_record.is_null() {
        set_last_error("out_record must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_record = ptr::null_mut();
    match runtime().block_on(store.kind.peek()) {
        Ok(Some(record)) => {
            *out_record = PamojaBuffer::into_raw(record);
            PamojaStatus::Ok
        }
        Ok(None) => PamojaStatus::Ok,
        Err(error) => fail(error),
    }
}

/// Removes and returns the oldest record.
///
/// # Arguments
///
/// * `store` - the buffer.
/// * `out_record` - receives a buffer handle, or null when the store is empty.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. A null `out_record` with an `Ok` status
/// means the buffer is empty.
///
/// # Safety
///
/// `store` must be a live handle and `out_record` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_store_pop(
    store: *mut PamojaStore,
    out_record: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let Some(store) = store_handle(store) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_record.is_null() {
        set_last_error("out_record must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_record = ptr::null_mut();
    match runtime().block_on(store.kind.pop()) {
        Ok(Some(record)) => {
            *out_record = PamojaBuffer::into_raw(record);
            PamojaStatus::Ok
        }
        Ok(None) => PamojaStatus::Ok,
        Err(error) => fail(error),
    }
}

/// Reports how many records a buffer holds.
///
/// # Arguments
///
/// * `store` - the buffer.
/// * `out_len` - receives the count.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Safety
///
/// `store` must be a live handle and `out_len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_store_len(
    store: *mut PamojaStore,
    out_len: *mut usize,
) -> PamojaStatus {
    let Some(store) = store_handle(store) else {
        return PamojaStatus::InvalidArgument;
    };
    if out_len.is_null() {
        set_last_error("out_len must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match runtime().block_on(store.kind.len()) {
        Ok(len) => {
            *out_len = len;
            PamojaStatus::Ok
        }
        Err(error) => fail(error),
    }
}

/// Sends every held record over a transport, oldest first.
///
/// A record is removed only once the transport has taken it, so a link that
/// fails part-way leaves the rest of the queue intact for the next attempt.
///
/// # Arguments
///
/// * `store` - the buffer to drain.
/// * `transport` - the transport to send over, borrowed rather than consumed.
/// * `topic` - the topic to send to, as null-terminated UTF-8.
/// * `out_sent` - receives how many records went out, or may be null.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] if the whole buffer drained, or a transport error with
/// `out_sent` holding how many got through before it stopped.
///
/// # Safety
///
/// `store` and `transport` must be live handles, `topic` a valid
/// null-terminated UTF-8 string, and `out_sent` writable or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_store_drain_to(
    store: *mut PamojaStore,
    transport: *mut PamojaTransport,
    topic: *const c_char,
    out_sent: *mut usize,
) -> PamojaStatus {
    let Some(store) = store_handle(store) else {
        return PamojaStatus::InvalidArgument;
    };
    if transport.is_null() {
        set_last_error("transport must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(topic) = read_str(topic, "topic") else {
        return PamojaStatus::InvalidArgument;
    };

    let transport = &mut (*transport).kind;
    match runtime().block_on(drain_to(&mut store.kind, transport, topic)) {
        Ok(sent) => {
            if !out_sent.is_null() {
                *out_sent = sent;
            }
            PamojaStatus::Ok
        }
        Err(error) => fail(error),
    }
}

/// Releases a store handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `store` must be a handle from a call that produced one and that has not
/// already been freed or consumed, or null. After this call it must not be used
/// again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_store_free(store: *mut PamojaStore) {
    if !store.is_null() {
        drop(Box::from_raw(store));
    }
}

/// Borrows a store handle, rejecting a null pointer.
///
/// # Safety
///
/// `store` must be a live handle from a call that produced one, or null.
unsafe fn store_handle<'a>(store: *mut PamojaStore) -> Option<&'a mut PamojaStore> {
    if store.is_null() {
        set_last_error("store must not be null".to_owned());
        return None;
    }
    Some(&mut *store)
}

/// Takes ownership of a store handle, leaving the caller nothing to free.
///
/// # Safety
///
/// `store` must be a live handle that has not been freed or consumed, or null.
/// After this call the caller must not use it again.
pub(crate) unsafe fn take_store(store: *mut PamojaStore) -> Option<StoreKind> {
    if store.is_null() {
        set_last_error("store must not be null".to_owned());
        return None;
    }
    Some(Box::from_raw(store).kind)
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

    /// Reads a buffer handle out and releases it.
    unsafe fn take(buffer: *mut PamojaBuffer) -> Vec<u8> {
        assert!(!buffer.is_null());
        let bytes =
            std::slice::from_raw_parts(pamoja_buffer_data(buffer), pamoja_buffer_len(buffer))
                .to_vec();
        pamoja_buffer_free(buffer);
        bytes
    }

    #[test]
    fn a_buffer_returns_records_oldest_first() {
        unsafe {
            let store = pamoja_store_memory(0);
            assert_eq!(
                pamoja_store_append(store, b"one".as_ptr(), 3),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_store_append(store, b"two".as_ptr(), 3),
                PamojaStatus::Ok
            );

            let mut len = 0;
            assert_eq!(pamoja_store_len(store, &mut len), PamojaStatus::Ok);
            assert_eq!(len, 2);

            let mut record = ptr::null_mut();
            assert_eq!(pamoja_store_peek(store, &mut record), PamojaStatus::Ok);
            assert_eq!(take(record), b"one", "peek leaves the record in place");

            assert_eq!(pamoja_store_pop(store, &mut record), PamojaStatus::Ok);
            assert_eq!(take(record), b"one");
            assert_eq!(pamoja_store_pop(store, &mut record), PamojaStatus::Ok);
            assert_eq!(take(record), b"two");

            assert_eq!(pamoja_store_pop(store, &mut record), PamojaStatus::Ok);
            assert!(record.is_null(), "an empty store yields nothing");

            pamoja_store_free(store);
        }
    }

    #[test]
    fn a_full_buffer_refuses_rather_than_losing_a_record() {
        unsafe {
            let store = pamoja_store_memory(2);
            assert_eq!(
                pamoja_store_append(store, b"one".as_ptr(), 3),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_store_append(store, b"two".as_ptr(), 3),
                PamojaStatus::Ok
            );
            assert_ne!(
                pamoja_store_append(store, b"raw".as_ptr(), 3),
                PamojaStatus::Ok,
                "a full store tells the caller rather than dropping something"
            );

            let mut len = 0;
            pamoja_store_len(store, &mut len);
            assert_eq!(len, 2, "and what it already held is untouched");

            let mut record = ptr::null_mut();
            pamoja_store_pop(store, &mut record);
            assert_eq!(take(record), b"one");

            pamoja_store_free(store);
        }
    }

    #[test]
    fn a_null_handle_is_refused_rather_than_dereferenced() {
        unsafe {
            assert_eq!(
                pamoja_store_append(ptr::null_mut(), b"x".as_ptr(), 1),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_store_len(ptr::null_mut(), ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            pamoja_store_free(ptr::null_mut());
        }
    }
}
