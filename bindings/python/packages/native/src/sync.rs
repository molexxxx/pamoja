//! Generated Python bindings for store-and-forward buffers.
//!
//! These mirror the `pamoja-sync` Rust API: the queue a node writes into while
//! it has nowhere to send, and the drain that empties it once a link comes back.
//!
//! Two stores cross, behind one class. An in-memory queue suits a test or a
//! process that will not outlive its buffer; a file-backed one survives a
//! reboot, which is what a node somewhere without reliable power actually needs.

use std::sync::{Arc, Mutex as SyncMutex};

use pamoja_core::Store as CoreStore;
use pamoja_sync::{FileStore, MemoryStore};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use tokio::sync::Mutex;

use crate::PamojaError;

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

/// A buffer shared between the class Python holds and the ladder it is given to.
///
/// A Python method borrows `self`, so the buffer has to be reachable from an
/// owned handle that outlives the call. A ladder takes its buffer by value, so
/// the same shared handle is what it receives.
#[derive(Clone)]
pub(crate) struct SharedStore(Arc<Mutex<StoreKind>>);

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
/// the ladder, so using one twice raises.
#[gen_stub_pyclass]
#[pyclass]
pub struct Store {
    inner: SyncMutex<Option<SharedStore>>,
}

impl Store {
    /// Takes the buffer, leaving this handle spent.
    ///
    /// The buffer is behind a lock so a shared reference can empty it, which is
    /// what Python hands a method, and so the class is `Sync` as a `pyclass` has
    /// to be.
    pub(crate) fn take(&self) -> PyResult<SharedStore> {
        self.inner
            .lock()
            .map_err(|_| PamojaError::new_err("this store is poisoned"))?
            .take()
            .ok_or_else(given_away)
    }

    /// Borrows the buffer, refusing one that has been given away.
    fn borrow(&self) -> PyResult<SharedStore> {
        self.inner
            .lock()
            .map_err(|_| PamojaError::new_err("this store is poisoned"))?
            .clone()
            .ok_or_else(given_away)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Store {
    /// Creates a buffer held in memory.
    ///
    /// A full store refuses the next append rather than dropping anything, so a
    /// record is never lost without the caller being told.
    #[staticmethod]
    #[pyo3(signature = (capacity = 0))]
    fn memory(capacity: usize) -> Self {
        let store = if capacity == 0 {
            MemoryStore::new()
        } else {
            MemoryStore::with_capacity(capacity)
        };
        Self {
            inner: SyncMutex::new(Some(SharedStore(Arc::new(Mutex::new(StoreKind::Memory(
                store,
            )))))),
        }
    }

    /// Opens a buffer backed by a directory, so it survives a restart.
    #[staticmethod]
    fn file(dir: String) -> PyResult<Self> {
        FileStore::open(dir)
            .map(|store| Self {
                inner: SyncMutex::new(Some(SharedStore(Arc::new(Mutex::new(StoreKind::File(
                    store,
                )))))),
            })
            .map_err(to_pyerr)
    }

    /// Adds a record to the end of the buffer.
    fn append<'py>(&self, py: Python<'py>, record: Vec<u8>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.borrow()?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut store = inner;
            store.append(&record).await.map_err(to_pyerr)
        })
    }

    /// Reads the oldest record without removing it, or `None` when empty.
    fn peek<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.borrow()?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.peek().await.map_err(to_pyerr)
        })
    }

    /// Removes and returns the oldest record, or `None` when empty.
    fn pop<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.borrow()?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut store = inner;
            store.pop().await.map_err(to_pyerr)
        })
    }

    /// How many records the buffer holds.
    fn len<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.borrow()?;
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            inner.len().await.map_err(to_pyerr)
        })
    }

    /// Whether this store is still holdable, or has been given to a ladder.
    #[getter]
    fn is_available(&self) -> bool {
        self.inner
            .lock()
            .map(|held| held.is_some())
            .unwrap_or(false)
    }
}

/// Maps a core error onto the one Python sees.
fn to_pyerr(error: pamoja_core::Error) -> PyErr {
    PamojaError::new_err(error.to_string())
}

/// The error a store already handed to a ladder reports.
fn given_away() -> PyErr {
    PamojaError::new_err("this store was already given to a ladder")
}
