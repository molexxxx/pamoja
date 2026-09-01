//! Where images live, and what the device believes about each one.
//!
//! A device that can be updated safely needs somewhere to put the new image that
//! is not where the running one lives, so a failed update leaves something to go
//! back to. That is all a slot is.
//!
//! Storage itself is the integrator's: internal flash, an external chip, a file on
//! an SD card. [`SlotStore`] is the seam, so the update rules can be exercised in
//! full with no hardware, the way `MemoryLink` does for the MAVLink link layer.

use alloc::vec;
use alloc::vec::Vec;

use crate::error::{Refusal, Result};
use crate::manifest::DIGEST_LEN;

/// What the device believes about a slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SlotState {
    /// Nothing here, or nothing worth keeping.
    #[default]
    Empty,
    /// Holds a verified image that has not been booted yet.
    Staged,
    /// Was booted but has not yet reported itself healthy.
    Pending,
    /// Booted and confirmed healthy. This is what the device falls back to.
    Confirmed,
    /// Was booted and never confirmed, so it is not to be tried again.
    Failed,
}

/// What a slot holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotRecord {
    /// The slot's state.
    pub state: SlotState,
    /// The sequence number of the image in the slot.
    pub sequence: u64,
    /// The image's length in bytes.
    pub size: u32,
    /// The image's digest.
    pub digest: [u8; DIGEST_LEN],
}

/// Somewhere to keep images, and the device's belief about each one.
///
/// An implementation must keep the record durable across a reboot. If it does not,
/// a device that loses power mid-update cannot tell what it was doing, which is
/// exactly the situation slots exist to survive.
pub trait SlotStore {
    /// Returns how many slots this device has.
    fn slot_count(&self) -> u8;

    /// Returns how many bytes a slot can hold.
    ///
    /// # Arguments
    ///
    /// * `slot` - the slot to size.
    ///
    /// # Returns
    ///
    /// The slot's capacity in bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::NoSuchSlot`] if the slot does not exist.
    fn capacity(&self, slot: u8) -> Result<u32>;

    /// Reads what the device believes about a slot.
    ///
    /// # Arguments
    ///
    /// * `slot` - the slot to read.
    ///
    /// # Returns
    ///
    /// The slot's record.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::NoSuchSlot`] if the slot does not exist.
    fn record(&self, slot: u8) -> Result<SlotRecord>;

    /// Writes what the device believes about a slot, durably.
    ///
    /// # Arguments
    ///
    /// * `slot` - the slot to describe.
    /// * `record` - the new record.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the record will survive a reboot.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::NoSuchSlot`] if the slot does not exist.
    fn set_record(&mut self, slot: u8, record: SlotRecord) -> Result<()>;

    /// Clears a slot's contents and marks it empty.
    ///
    /// # Arguments
    ///
    /// * `slot` - the slot to clear.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the slot holds nothing.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::NoSuchSlot`] if the slot does not exist.
    fn erase(&mut self, slot: u8) -> Result<()>;

    /// Writes image bytes at an offset within a slot.
    ///
    /// # Arguments
    ///
    /// * `slot` - the slot to write into.
    /// * `offset` - where in the slot the bytes belong.
    /// * `bytes` - the bytes to write.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the bytes are stored.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::NoSuchSlot`] if the slot does not exist, or
    /// [`Refusal::SlotTooSmall`] if the write would run past the slot's end.
    fn write(&mut self, slot: u8, offset: u32, bytes: &[u8]) -> Result<()>;

    /// Reads image bytes from an offset within a slot.
    ///
    /// # Arguments
    ///
    /// * `slot` - the slot to read from.
    /// * `offset` - where in the slot to start.
    /// * `buf` - the destination.
    ///
    /// # Returns
    ///
    /// How many bytes were read, which is short only at the slot's end.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::NoSuchSlot`] if the slot does not exist.
    fn read(&self, slot: u8, offset: u32, buf: &mut [u8]) -> Result<usize>;
}

/// A [`SlotStore`] held in memory, for tests and for running the flow with no
/// hardware.
///
/// It forgets everything when dropped, which is the one thing a real store must
/// not do, so it stands in for storage without pretending to be it.
#[derive(Clone, Debug)]
pub struct MemoryStore {
    slots: Vec<Vec<u8>>,
    records: Vec<SlotRecord>,
}

impl MemoryStore {
    /// Creates a store with `count` slots of `capacity` bytes each.
    ///
    /// # Arguments
    ///
    /// * `count` - how many slots the device has.
    /// * `capacity` - how many bytes each slot holds.
    ///
    /// # Returns
    ///
    /// An empty store.
    pub fn new(count: u8, capacity: u32) -> Self {
        Self {
            slots: vec![vec![0u8; capacity as usize]; count as usize],
            records: vec![SlotRecord::default(); count as usize],
        }
    }

    /// Returns the index of a slot, refusing one this device does not have.
    fn index(&self, slot: u8) -> Result<usize> {
        if usize::from(slot) >= self.slots.len() {
            return Err(Refusal::NoSuchSlot);
        }
        Ok(usize::from(slot))
    }
}

impl SlotStore for MemoryStore {
    fn slot_count(&self) -> u8 {
        self.slots.len() as u8
    }

    fn capacity(&self, slot: u8) -> Result<u32> {
        Ok(self.slots[self.index(slot)?].len() as u32)
    }

    fn record(&self, slot: u8) -> Result<SlotRecord> {
        Ok(self.records[self.index(slot)?])
    }

    fn set_record(&mut self, slot: u8, record: SlotRecord) -> Result<()> {
        let at = self.index(slot)?;
        self.records[at] = record;
        Ok(())
    }

    fn erase(&mut self, slot: u8) -> Result<()> {
        let at = self.index(slot)?;
        self.slots[at].fill(0);
        self.records[at] = SlotRecord::default();
        Ok(())
    }

    fn write(&mut self, slot: u8, offset: u32, bytes: &[u8]) -> Result<()> {
        let at = self.index(slot)?;
        let start = offset as usize;
        let end = start
            .checked_add(bytes.len())
            .ok_or(Refusal::SlotTooSmall)?;
        if end > self.slots[at].len() {
            return Err(Refusal::SlotTooSmall);
        }
        self.slots[at][start..end].copy_from_slice(bytes);
        Ok(())
    }

    fn read(&self, slot: u8, offset: u32, buf: &mut [u8]) -> Result<usize> {
        let at = self.index(slot)?;
        let start = (offset as usize).min(self.slots[at].len());
        let len = buf.len().min(self.slots[at].len() - start);
        buf[..len].copy_from_slice(&self.slots[at][start..start + len]);
        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_store_has_empty_slots() {
        let store = MemoryStore::new(2, 64);
        assert_eq!(store.slot_count(), 2);
        assert_eq!(store.capacity(0).expect("capacity"), 64);
        assert_eq!(store.record(0).expect("record").state, SlotState::Empty);
    }

    #[test]
    fn writes_read_back() {
        let mut store = MemoryStore::new(1, 16);
        store.write(0, 4, b"abcd").expect("write");
        let mut buf = [0u8; 4];
        assert_eq!(store.read(0, 4, &mut buf).expect("read"), 4);
        assert_eq!(&buf, b"abcd");
    }

    #[test]
    fn a_write_past_the_end_is_refused() {
        let mut store = MemoryStore::new(1, 8);
        assert_eq!(
            store.write(0, 6, b"abcd"),
            Err(Refusal::SlotTooSmall),
            "a slot must never be written past its capacity"
        );
    }

    #[test]
    fn a_slot_the_device_does_not_have_is_refused() {
        let mut store = MemoryStore::new(1, 8);
        assert_eq!(store.capacity(3), Err(Refusal::NoSuchSlot));
        assert_eq!(store.record(3), Err(Refusal::NoSuchSlot));
        assert_eq!(store.write(3, 0, b"x"), Err(Refusal::NoSuchSlot));
    }

    #[test]
    fn erasing_clears_both_the_bytes_and_the_record() {
        let mut store = MemoryStore::new(1, 8);
        store.write(0, 0, b"abcd").expect("write");
        store
            .set_record(
                0,
                SlotRecord {
                    state: SlotState::Confirmed,
                    sequence: 4,
                    size: 4,
                    digest: [7; DIGEST_LEN],
                },
            )
            .expect("record");

        store.erase(0).expect("erase");
        assert_eq!(store.record(0).expect("record"), SlotRecord::default());
        let mut buf = [0xffu8; 4];
        store.read(0, 0, &mut buf).expect("read");
        assert_eq!(&buf, &[0, 0, 0, 0]);
    }

    #[test]
    fn reading_past_the_end_returns_what_there_is() {
        let store = MemoryStore::new(1, 4);
        let mut buf = [0u8; 8];
        assert_eq!(store.read(0, 2, &mut buf).expect("read"), 2);
        assert_eq!(store.read(0, 99, &mut buf).expect("read"), 0);
    }
}
