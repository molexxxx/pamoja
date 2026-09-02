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

use std::sync::{Arc, Mutex as SyncMutex};

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_core::Store as CoreStore;
use pamoja_sync::{FileStore, MemoryStore};
use tokio::sync::Mutex;

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

/// A buffer shared between the class JavaScript holds and the ladder it is given
/// to.
///
/// A JavaScript method borrows `self`, so the buffer has to be reachable from an
/// owned handle that outlives the call. A ladder takes its buffer by value, so
/// the same shared handle is what it receives.
#[derive(Clone)]
pub(crate) struct SharedStore(Arc<Mutex<StoreKind>>);

impl SharedStore {
    /// Wraps a buffer in a handle the ladder and the class can both hold.
    fn new(kind: StoreKind) -> Self {
        Self(Arc::new(Mutex::new(kind)))
    }
}

impl CoreStore for SharedStore {
    async fn append(&mut self, record: &[u8]) -> pamoja_core::Result<()> {
        self.0.lock().await.append(record).await
    }

    async fn peek(&self) -> pamoja_core::Result<Option<Vec<u8>>> {
        self.0.lock().await.peek().await
    }

    async fn pop(&mut self) -> pamoja_core::Result<Option<Vec<u8>>> {
        self.0.lock().await.pop().await
    }

    async fn len(&self) -> pamoja_core::Result<usize> {
        self.0.lock().await.len().await
    }
}

/// A store-and-forward buffer.
///
/// Handing a store to a ladder consumes it, because the ladder owns it from then
/// on. A consumed store is emptied rather than left aliasing what now belongs to
/// the ladder, so using one twice throws.
#[napi]
pub struct Store {
    inner: SyncMutex<Option<SharedStore>>,
}

impl Store {
    /// Takes the buffer, leaving this handle spent.
    ///
    /// The buffer is behind a lock so a shared reference can empty it, which is
    /// what a JavaScript method is handed.
    pub(crate) fn take(&self) -> napi::Result<SharedStore> {
        self.locked()?.take().ok_or_else(given_away)
    }

    /// Borrows the buffer, refusing one that has been given away.
    fn borrow(&self) -> napi::Result<SharedStore> {
        self.locked()?.clone().ok_or_else(given_away)
    }

    /// Locks the slot the buffer sits in.
    fn locked(&self) -> napi::Result<std::sync::MutexGuard<'_, Option<SharedStore>>> {
        self.inner
            .lock()
            .map_err(|_| napi::Error::from_reason("this store is poisoned"))
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
            inner: SyncMutex::new(Some(SharedStore::new(StoreKind::Memory(store)))),
        }
    }

    /// Opens a buffer backed by a directory, so it survives a restart.
    ///
    /// @param dir - the directory to hold records in; it is created if missing.
    #[napi(factory)]
    pub fn file(dir: String) -> napi::Result<Self> {
        FileStore::open(dir)
            .map(|store| Self {
                inner: SyncMutex::new(Some(SharedStore::new(StoreKind::File(store)))),
            })
            .map_err(to_napi)
    }

    /// Adds a record to the end of the buffer.
    #[napi]
    pub async fn append(&self, record: Buffer) -> napi::Result<()> {
        let record = record.to_vec();
        self.borrow()?.append(&record).await.map_err(to_napi)
    }

    /// Reads the oldest record without removing it, or `null` when empty.
    #[napi]
    pub async fn peek(&self) -> napi::Result<Option<Buffer>> {
        self.borrow()?
            .peek()
            .await
            .map(|record| record.map(Buffer::from))
            .map_err(to_napi)
    }

    /// Removes and returns the oldest record, or `null` when empty.
    #[napi]
    pub async fn pop(&self) -> napi::Result<Option<Buffer>> {
        self.borrow()?
            .pop()
            .await
            .map(|record| record.map(Buffer::from))
            .map_err(to_napi)
    }

    /// Whether this store is still holdable, or has been given to a ladder.
    #[napi(getter)]
    pub fn is_available(&self) -> bool {
        self.inner.lock().is_ok_and(|slot| slot.is_some())
    }

    /// How many records the buffer holds.
    #[napi]
    pub async fn len(&self) -> napi::Result<u32> {
        self.borrow()?
            .len()
            .await
            .map(|len| len as u32)
            .map_err(to_napi)
    }
}

/// The error a store that has already been given to a ladder reports.
fn given_away() -> napi::Error {
    napi::Error::from_reason("this store was already given to a ladder")
}

/// Maps a core error onto the one JavaScript sees.
fn to_napi(error: pamoja_core::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
