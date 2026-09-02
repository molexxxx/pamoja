//! Generated Node bindings for store-and-forward buffers.
//!
//! These mirror the `pamoja-sync` Rust API: the queue a node writes into while
//! it has nowhere to send, and the drain that empties it once a link comes back.
//!
//! Two stores cross, behind one class. An in-memory queue suits a test or a
//! process that will not outlive its buffer; a file-backed one survives a
//! reboot, which is what a node somewhere without reliable power actually needs.
//! The kind is chosen when the store is created and nothing afterwards has to
//! care which it is.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_core::Store as CoreStore;
use pamoja_sync::{FileStore, MemoryStore};

/// One buffer, whichever kind it was created as.
pub(crate) enum StoreKind {
    /// A queue held in memory, lost when the process ends.
    Memory(MemoryStore),
    /// A queue on disk, which survives a restart.
    File(FileStore),
}

impl CoreStore for StoreKind {
    async fn append(&mut self, record: &[u8]) -> pamoja_core::Result<()> {
        match self {
            StoreKind::Memory(store) => store.append(record).await,
            StoreKind::File(store) => store.append(record).await,
        }
    }

    async fn peek(&self) -> pamoja_core::Result<Option<Vec<u8>>> {
        match self {
            StoreKind::Memory(store) => store.peek().await,
            StoreKind::File(store) => store.peek().await,
        }
    }

    async fn pop(&mut self) -> pamoja_core::Result<Option<Vec<u8>>> {
        match self {
            StoreKind::Memory(store) => store.pop().await,
            StoreKind::File(store) => store.pop().await,
        }
    }

    async fn len(&self) -> pamoja_core::Result<usize> {
        match self {
            StoreKind::Memory(store) => store.len().await,
            StoreKind::File(store) => store.len().await,
        }
    }
}

/// A store-and-forward buffer.
#[napi]
pub struct Store {
    inner: Option<StoreKind>,
}

impl Store {
    /// Borrows the buffer this store holds.
    fn borrow(&mut self) -> napi::Result<&mut StoreKind> {
        self.inner
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("this store is no longer usable"))
    }
}

#[napi]
impl Store {
    /// Creates a buffer held in memory.
    ///
    /// @param capacity - the most records to hold, or omitted for no bound. A
    ///   full store refuses the next append rather than dropping anything, so a
    ///   record is never lost without the caller being told.
    #[napi(factory)]
    pub fn memory(capacity: Option<u32>) -> Self {
        let store = match capacity {
            Some(capacity) if capacity != 0 => MemoryStore::with_capacity(capacity as usize),
            _ => MemoryStore::new(),
        };
        Self {
            inner: Some(StoreKind::Memory(store)),
        }
    }

    /// Opens a buffer backed by a directory, so it survives a restart.
    ///
    /// @param dir - the directory to hold records in; it is created if missing.
    #[napi(factory)]
    pub fn file(dir: String) -> napi::Result<Self> {
        FileStore::open(dir)
            .map(|store| Self {
                inner: Some(StoreKind::File(store)),
            })
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Adds a record to the end of the buffer.
    #[napi]
    pub async unsafe fn append(&mut self, record: Buffer) -> napi::Result<()> {
        let record = record.to_vec();
        self.borrow()?
            .append(&record)
            .await
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Reads the oldest record without removing it, or `null` when empty.
    #[napi]
    pub async unsafe fn peek(&mut self) -> napi::Result<Option<Buffer>> {
        self.borrow()?
            .peek()
            .await
            .map(|record| record.map(Buffer::from))
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Removes and returns the oldest record, or `null` when empty.
    #[napi]
    pub async unsafe fn pop(&mut self) -> napi::Result<Option<Buffer>> {
        self.borrow()?
            .pop()
            .await
            .map(|record| record.map(Buffer::from))
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// How many records the buffer holds.
    #[napi]
    pub async unsafe fn len(&mut self) -> napi::Result<u32> {
        self.borrow()?
            .len()
            .await
            .map(|len| len as u32)
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

}
