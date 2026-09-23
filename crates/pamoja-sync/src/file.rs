//! A crash-safe on-disk store-and-forward queue.

use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use pamoja_core::{Error, Result, Store};

/// The filename extension for a stored record.
const RECORD_EXTENSION: &str = "rec";

/// The extension of a record still being written.
const PARTIAL_EXTENSION: &str = "rec.tmp";

/// A durable first-in first-out queue backed by one file per record.
///
/// Each [`append`](Store::append) writes a record to its own sequence-numbered
/// file, flushes it to disk, and atomically renames it into place, so a power
/// loss mid-write leaves the queue consistent: a partially written record is
/// never visible, and [`open`](FileStore::open) clears what one left behind. On
/// Linux and other Unix systems the directory is synced after the rename as well,
/// so an append that returned is on disk. On Windows the directory entry is left
/// to the file system, so an append that returned just before a power cut can be
/// missing afterwards, though never half written. [`pop`](Store::pop) reads and
/// deletes the oldest record.
///
/// Delivery is at-least-once: if the process stops between reading a record and
/// deleting it, the next [`open`](FileStore::open) returns that record again, so
/// consumers must tolerate the occasional redelivery.
///
/// # Examples
///
/// ```no_run
/// use pamoja_core::Store;
/// use pamoja_sync::FileStore;
///
/// # async fn run() -> pamoja_core::Result<()> {
/// let mut store = FileStore::open("/var/lib/pamoja/outbox")?;
/// store.append(b"reading").await?;
/// if let Some(record) = store.pop().await? {
///     // forward `record` over a transport, then it is gone from the queue
///     let _ = record;
/// }
/// # Ok(())
/// # }
/// ```
pub struct FileStore {
    dir: PathBuf,
    pending: VecDeque<u64>,
    next: u64,
    capacity: Option<usize>,
}

impl FileStore {
    /// Opens a store rooted at `dir`, creating the directory if needed.
    ///
    /// Records left in the directory by a previous run are adopted in sequence
    /// order, so the queue resumes where it left off, and a record a power cut
    /// interrupted mid-write is deleted.
    ///
    /// # Arguments
    ///
    /// * `dir` - the directory that holds the queue's record files.
    ///
    /// # Returns
    ///
    /// A store with no limit on how many records it holds, ready to append and
    /// drain records.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](pamoja_core::Error::Io) if the directory cannot be
    /// created or scanned, or an interrupted record cannot be deleted.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir).map_err(io)?;

        let mut sequences = Vec::new();
        for entry in fs::read_dir(&dir).map_err(io)? {
            let path = entry.map_err(io)?.path();
            let name = path.file_name().and_then(|name| name.to_str());
            if name.is_some_and(|name| name.ends_with(PARTIAL_EXTENSION)) {
                fs::remove_file(&path).map_err(io)?;
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some(RECORD_EXTENSION) {
                continue;
            }
            if let Some(sequence) = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.parse::<u64>().ok())
            {
                sequences.push(sequence);
            }
        }
        sequences.sort_unstable();
        let next = sequences.last().map_or(0, |last| last + 1);

        Ok(Self {
            dir,
            pending: sequences.into(),
            next,
            capacity: None,
        })
    }

    /// Opens a store rooted at `dir` that holds at most `capacity` records.
    ///
    /// An append that would take the store past `capacity` is refused rather
    /// than written, which on a device writing to an SD card is what keeps a long
    /// outage from filling the card. A store reopened with more records than
    /// `capacity` keeps them all and refuses appends until it is drained below it.
    ///
    /// # Arguments
    ///
    /// * `dir` - the directory that holds the queue's record files.
    /// * `capacity` - the most records the store holds.
    ///
    /// # Returns
    ///
    /// A bounded store, ready to append and drain records.
    ///
    /// # Errors
    ///
    /// As for [`open`](FileStore::open).
    pub fn open_with_capacity(dir: impl AsRef<Path>, capacity: usize) -> Result<Self> {
        let mut store = Self::open(dir)?;
        store.capacity = Some(capacity);
        Ok(store)
    }

    /// Returns the path of the record file for a sequence number.
    fn record_path(&self, sequence: u64) -> PathBuf {
        self.dir.join(format!("{sequence:020}.{RECORD_EXTENSION}"))
    }

    /// Makes the directory's latest rename durable, where the platform allows it.
    fn sync_dir(&self) -> Result<()> {
        #[cfg(unix)]
        fs::File::open(&self.dir)
            .and_then(|dir| dir.sync_all())
            .map_err(io)?;
        Ok(())
    }
}

impl Store for FileStore {
    async fn append(&mut self, record: &[u8]) -> Result<()> {
        if self
            .capacity
            .is_some_and(|capacity| self.pending.len() >= capacity)
        {
            return Err(Error::Io("store is at capacity".to_owned()));
        }
        let sequence = self.next;
        let final_path = self.record_path(sequence);
        let temp_path = final_path.with_extension(PARTIAL_EXTENSION);

        let mut file = fs::File::create(&temp_path).map_err(io)?;
        file.write_all(record).map_err(io)?;
        file.sync_all().map_err(io)?;
        drop(file);
        fs::rename(&temp_path, &final_path).map_err(io)?;
        self.sync_dir()?;

        self.next += 1;
        self.pending.push_back(sequence);
        Ok(())
    }

    async fn peek(&self) -> Result<Option<Vec<u8>>> {
        let Some(&sequence) = self.pending.front() else {
            return Ok(None);
        };
        let record = fs::read(self.record_path(sequence)).map_err(io)?;
        Ok(Some(record))
    }

    async fn pop(&mut self) -> Result<Option<Vec<u8>>> {
        let Some(sequence) = self.pending.pop_front() else {
            return Ok(None);
        };
        let path = self.record_path(sequence);
        let record = fs::read(&path).map_err(io)?;
        fs::remove_file(&path).map_err(io)?;
        Ok(Some(record))
    }

    async fn len(&self) -> Result<usize> {
        Ok(self.pending.len())
    }
}

/// Maps a filesystem error onto the shared I/O error.
fn io(error: std::io::Error) -> Error {
    Error::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn drains_in_first_in_first_out_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = FileStore::open(dir.path()).expect("open");

        store.append(b"a").await.expect("append");
        store.append(b"b").await.expect("append");
        assert_eq!(store.len().await.expect("len"), 2);

        assert_eq!(store.pop().await.expect("pop"), Some(b"a".to_vec()));
        assert_eq!(store.pop().await.expect("pop"), Some(b"b".to_vec()));
        assert_eq!(store.pop().await.expect("pop"), None);
    }

    #[tokio::test]
    async fn records_survive_reopening() {
        let dir = tempfile::tempdir().expect("tempdir");
        {
            let mut store = FileStore::open(dir.path()).expect("open");
            store.append(b"durable").await.expect("append");
            store.append(b"records").await.expect("append");
        }

        let mut reopened = FileStore::open(dir.path()).expect("reopen");
        assert_eq!(reopened.len().await.expect("len"), 2);
        assert_eq!(
            reopened.pop().await.expect("pop"),
            Some(b"durable".to_vec())
        );
        assert_eq!(
            reopened.pop().await.expect("pop"),
            Some(b"records".to_vec())
        );
    }

    #[tokio::test]
    async fn peek_reads_the_oldest_without_removing_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = FileStore::open(dir.path()).expect("open");
        store.append(b"oldest").await.expect("append");
        store.append(b"newer").await.expect("append");

        assert_eq!(store.peek().await.expect("peek"), Some(b"oldest".to_vec()));
        assert_eq!(store.len().await.expect("len"), 2);
        assert_eq!(store.pop().await.expect("pop"), Some(b"oldest".to_vec()));
    }

    #[tokio::test]
    async fn popping_removes_the_record_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = FileStore::open(dir.path()).expect("open");
        store.append(b"once").await.expect("append");
        let _ = store.pop().await.expect("pop");

        let reopened = FileStore::open(dir.path()).expect("reopen");
        assert!(reopened.is_empty().await.expect("is_empty"));
    }

    #[tokio::test]
    async fn a_record_interrupted_mid_write_is_never_seen_and_is_cleared() {
        let dir = tempfile::tempdir().expect("tempdir");
        {
            let mut store = FileStore::open(dir.path()).expect("open");
            store.append(b"whole").await.expect("append");
        }
        let partial = dir.path().join(format!("{:020}.{PARTIAL_EXTENSION}", 1));
        fs::write(&partial, b"hal").expect("a half-written record");

        let mut reopened = FileStore::open(dir.path()).expect("reopen");
        assert_eq!(reopened.len().await.expect("len"), 1);
        assert!(!partial.exists(), "the interrupted write was cleared");
        assert_eq!(reopened.pop().await.expect("pop"), Some(b"whole".to_vec()));
        reopened.append(b"next").await.expect("append");
        assert_eq!(reopened.pop().await.expect("pop"), Some(b"next".to_vec()));
    }

    #[tokio::test]
    async fn a_bounded_store_refuses_the_append_that_would_overflow_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = FileStore::open_with_capacity(dir.path(), 2).expect("open");
        store.append(b"1").await.expect("room");
        store.append(b"2").await.expect("room");

        match store.append(b"3").await {
            Err(Error::Io(reason)) => assert_eq!(reason, "store is at capacity"),
            other => panic!("a full store took a record: {other:?}"),
        }
        assert_eq!(store.len().await.expect("len"), 2);

        let mut smaller = FileStore::open_with_capacity(dir.path(), 1).expect("reopen");
        assert_eq!(
            smaller.len().await.expect("len"),
            2,
            "a smaller bound keeps what is there"
        );
        assert!(smaller.append(b"3").await.is_err());
        smaller.pop().await.expect("pop");
        smaller.pop().await.expect("pop");
        smaller.append(b"3").await.expect("room once drained");
    }
}
