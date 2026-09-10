//! Durable local storage backing the offline-first synchronization layer.

use alloc::vec::Vec;
use core::future::Future;

use crate::error::Result;

/// A durable, first-in first-out queue for store-and-forward buffering.
///
/// Records are appended while a device is offline and drained in order when a
/// link becomes available, letting applications tolerate intermittent
/// connectivity without losing data.
///
/// The returned futures are `Send`, for the same reason a [`Transport`]'s are: a
/// transport ladder buffers into a store and is itself a transport, so a task on a
/// multi-threaded runtime can drive it. An implementation written as `async fn`
/// satisfies this as long as everything it holds across an await is `Send`.
///
/// [`Transport`]: crate::Transport
pub trait Store {
    /// Appends a record to the back of the queue.
    ///
    /// # Arguments
    ///
    /// * `record` - the raw bytes to persist.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the record is durably stored.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the record cannot be written to
    /// durable storage.
    fn append(&mut self, record: &[u8]) -> impl Future<Output = Result<()>> + Send;

    /// Appends text to the back of the queue: a reading or a line written out.
    ///
    /// This is [`append`](Self::append) with the text's UTF-8 bytes, so a queue of
    /// readings needs no encoding step; [`peek_text`](Self::peek_text) and
    /// [`pop_text`](Self::pop_text) read them back.
    ///
    /// # Arguments
    ///
    /// * `text` - the text to persist.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the record is durably stored.
    ///
    /// # Errors
    ///
    /// Whatever [`append`](Self::append) returns.
    fn append_text(&mut self, text: &str) -> impl Future<Output = Result<()>> + Send {
        self.append(text.as_bytes())
    }

    /// Returns the oldest record without removing it.
    ///
    /// This lets a forwarder send a record before committing to its removal, so a
    /// failed send can leave the record buffered in order rather than dropping it.
    ///
    /// # Returns
    ///
    /// `Some(record)` containing the oldest buffered bytes, or `None` if the queue
    /// is empty.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the queue cannot be read.
    fn peek(&self) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send;

    /// Returns the oldest record as text, without removing it.
    ///
    /// It borrows the store across an await, so it asks for `Sync`, which a store
    /// whose own futures are `Send` already is.
    ///
    /// # Returns
    ///
    /// `Some(text)` for the oldest record, or `None` if the queue is empty.
    ///
    /// # Errors
    ///
    /// Whatever [`peek`](Self::peek) returns, or
    /// [`Error::Codec`](crate::Error::Codec) if the record is not UTF-8 text.
    fn peek_text(&self) -> impl Future<Output = Result<Option<String>>> + Send
    where
        Self: Sync,
    {
        async { as_text(self.peek().await?) }
    }

    /// Removes and returns the oldest record in the queue.
    ///
    /// # Returns
    ///
    /// `Some(record)` containing the oldest buffered bytes, or `None` if the queue
    /// is empty.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the queue cannot be read.
    fn pop(&mut self) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send;

    /// Removes and returns the oldest record as text.
    ///
    /// # Returns
    ///
    /// `Some(text)` for the oldest record, or `None` if the queue is empty.
    ///
    /// # Errors
    ///
    /// Whatever [`pop`](Self::pop) returns, or
    /// [`Error::Codec`](crate::Error::Codec) if the record is not UTF-8 text.
    fn pop_text(&mut self) -> impl Future<Output = Result<Option<String>>> + Send
    where
        Self: Send,
    {
        async { as_text(self.pop().await?) }
    }

    /// Returns the number of records currently buffered.
    ///
    /// # Returns
    ///
    /// The count of records waiting to be drained.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the queue length cannot be
    /// determined.
    fn len(&self) -> impl Future<Output = Result<usize>> + Send;

    /// Returns whether the queue currently holds no records.
    ///
    /// The default implementation reports whether [`len`](Self::len) is zero. It
    /// borrows the store across an await, so it asks for `Sync`, which a store whose
    /// own futures are `Send` already is.
    ///
    /// # Returns
    ///
    /// `true` if the queue is empty, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the queue length cannot be
    /// determined.
    fn is_empty(&self) -> impl Future<Output = Result<bool>> + Send
    where
        Self: Sync,
    {
        async { Ok(self.len().await? == 0) }
    }
}

// A buffered record read as text, for the `_text` helpers above.
fn as_text(record: Option<Vec<u8>>) -> Result<Option<String>> {
    match record {
        Some(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| crate::Error::Codec("the record is not UTF-8 text".into())),
        None => Ok(None),
    }
}
