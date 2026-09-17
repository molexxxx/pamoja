//! The end devices a relay verifies wake-on-radio frames for, TS011-1.0.1 sections 10.4
//! and 10.5.

use super::keys::WorKeys;
use super::limits::TokenBucket;
use super::wor::{Carrier, SealedWor};
use super::TRUSTED_ED_NUMBER;
use crate::LorawanError;

/// An end device a relay trusts: its keys, the WOR frame counter it expects next, and how
/// much of it the relay forwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustedDevice {
    dev_addr: u32,
    keys: WorKeys,
    next_wfcnt: u64,
    bucket: TokenBucket,
}

impl TrustedDevice {
    /// Trusts an end device, as `UpdateUplinkListReq` asks.
    ///
    /// # Arguments
    ///
    /// * `dev_addr` - its address.
    /// * `root_wor_s_key` - the root relay session key the network sent for it, which its
    ///   integrity and encryption keys derive from.
    /// * `next_wfcnt` - the WOR frame counter the network expects from it next.
    /// * `bucket` - how many of its uplinks the relay forwards.
    ///
    /// # Returns
    ///
    /// The trusted device.
    #[must_use]
    pub fn new(
        dev_addr: u32,
        root_wor_s_key: &[u8; 16],
        next_wfcnt: u32,
        bucket: TokenBucket,
    ) -> TrustedDevice {
        TrustedDevice::with_keys(
            dev_addr,
            WorKeys::derive(root_wor_s_key, dev_addr),
            next_wfcnt,
            bucket,
        )
    }

    /// Trusts an end device whose keys are already derived.
    ///
    /// # Arguments
    ///
    /// * `dev_addr` - its address.
    /// * `keys` - its integrity and encryption keys.
    /// * `next_wfcnt` - the WOR frame counter expected from it next.
    /// * `bucket` - how many of its uplinks the relay forwards.
    ///
    /// # Returns
    ///
    /// The trusted device.
    #[must_use]
    pub const fn with_keys(
        dev_addr: u32,
        keys: WorKeys,
        next_wfcnt: u32,
        bucket: TokenBucket,
    ) -> TrustedDevice {
        TrustedDevice {
            dev_addr,
            keys,
            next_wfcnt: next_wfcnt as u64,
            bucket,
        }
    }

    /// Returns the device's address.
    ///
    /// # Returns
    ///
    /// The address.
    #[must_use]
    pub const fn dev_addr(&self) -> u32 {
        self.dev_addr
    }

    /// Returns the device's keys.
    ///
    /// # Returns
    ///
    /// The integrity and encryption keys.
    #[must_use]
    pub const fn keys(&self) -> &WorKeys {
        &self.keys
    }

    /// Returns the WOR frame counter the relay accepts next, at the least.
    ///
    /// # Returns
    ///
    /// The counter, or `None` once a frame with the last counter there is has been
    /// accepted.
    #[must_use]
    pub const fn next_wfcnt(&self) -> Option<u32> {
        if self.next_wfcnt > u32::MAX as u64 {
            None
        } else {
            Some(self.next_wfcnt as u32)
        }
    }

    /// Returns the last WOR frame counter the relay accepted, which `CtrlUplinkListAns`
    /// reports.
    ///
    /// # Returns
    ///
    /// The counter. Before any frame arrives, it is one less than the counter the network
    /// said to expect, wrapping, so the network reads back what it set.
    #[must_use]
    pub const fn last_wfcnt(&self) -> u32 {
        (self.next_wfcnt as u32).wrapping_sub(1)
    }

    /// Returns how many of the device's uplinks the relay forwards.
    ///
    /// # Returns
    ///
    /// The bucket.
    #[must_use]
    pub const fn bucket(&self) -> &TokenBucket {
        &self.bucket
    }

    /// Returns the device's bucket to spend from or reconfigure.
    ///
    /// # Returns
    ///
    /// The bucket.
    pub fn bucket_mut(&mut self) -> &mut TokenBucket {
        &mut self.bucket
    }

    /// Checks and opens a wake-on-radio frame from the device, then moves the counter past
    /// it.
    ///
    /// The frame carries the counter's low sixteen bits. The relay takes the first full
    /// counter at or after the one it expects with those bits, so a frame it has seen, or
    /// an older one, does not verify.
    ///
    /// # Arguments
    ///
    /// * `wor` - the frame.
    /// * `carrier` - the carrier it arrived on.
    ///
    /// # Returns
    ///
    /// The full counter the frame carried and where its uplink follows.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::FcntMismatch`] for a frame addressed elsewhere or past the
    /// last counter, and [`LorawanError::MicMismatch`] when it does not verify, leaving the
    /// counter where it was.
    pub fn open(
        &mut self,
        wor: &SealedWor,
        carrier: Carrier,
    ) -> Result<(u32, Carrier), LorawanError> {
        if wor.dev_addr() != self.dev_addr {
            return Err(LorawanError::FcntMismatch);
        }
        let mut candidate = (self.next_wfcnt & !0xFFFF) | u64::from(wor.wfcnt());
        if candidate < self.next_wfcnt {
            candidate += 0x1_0000;
        }
        let wfcnt = u32::try_from(candidate).map_err(|_| LorawanError::FcntMismatch)?;
        let uplink = wor.open(&self.keys, wfcnt, carrier)?;
        self.next_wfcnt = candidate + 1;
        Ok((wfcnt, uplink))
    }
}

/// The end devices a relay trusts, [`TRUSTED_ED_NUMBER`] of them, each at the index the
/// network gave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustedDevices {
    entries: [Option<TrustedDevice>; TRUSTED_ED_NUMBER],
}

impl TrustedDevices {
    /// An empty list.
    ///
    /// # Returns
    ///
    /// The list.
    #[must_use]
    pub const fn new() -> TrustedDevices {
        TrustedDevices {
            entries: [None; TRUSTED_ED_NUMBER],
        }
    }

    /// Returns the device at an index.
    ///
    /// # Arguments
    ///
    /// * `index` - the entry, 0 to 15.
    ///
    /// # Returns
    ///
    /// The device, or `None` for an empty entry or one out of range.
    #[must_use]
    pub fn get(&self, index: u8) -> Option<&TrustedDevice> {
        self.entries.get(usize::from(index))?.as_ref()
    }

    /// Returns the device at an index to change.
    ///
    /// # Arguments
    ///
    /// * `index` - the entry, 0 to 15.
    ///
    /// # Returns
    ///
    /// The device, or `None` for an empty entry or one out of range.
    pub fn get_mut(&mut self, index: u8) -> Option<&mut TrustedDevice> {
        self.entries.get_mut(usize::from(index))?.as_mut()
    }

    /// Puts a device at an index, replacing whatever was there.
    ///
    /// # Arguments
    ///
    /// * `index` - the entry, 0 to 15.
    /// * `device` - the device.
    ///
    /// # Returns
    ///
    /// `false` for an index out of range.
    pub fn set(&mut self, index: u8, device: TrustedDevice) -> bool {
        match self.entries.get_mut(usize::from(index)) {
            Some(entry) => {
                *entry = Some(device);
                true
            }
            None => false,
        }
    }

    /// Stops trusting the device at an index.
    ///
    /// # Arguments
    ///
    /// * `index` - the entry, 0 to 15.
    ///
    /// # Returns
    ///
    /// The device that was there.
    pub fn remove(&mut self, index: u8) -> Option<TrustedDevice> {
        self.entries.get_mut(usize::from(index))?.take()
    }

    /// Lists the devices.
    ///
    /// # Returns
    ///
    /// Each device with its index, in index order.
    pub fn iter(&self) -> impl Iterator<Item = (u8, &TrustedDevice)> + '_ {
        (0u8..)
            .zip(self.entries.iter())
            .filter_map(|(index, entry)| entry.as_ref().map(|device| (index, device)))
    }

    /// Reports whether any device has an address.
    ///
    /// # Arguments
    ///
    /// * `dev_addr` - the address.
    ///
    /// # Returns
    ///
    /// `true` when a trusted device has it.
    #[must_use]
    pub fn knows(&self, dev_addr: u32) -> bool {
        self.iter().any(|(_, device)| device.dev_addr() == dev_addr)
    }

    /// Checks and opens a wake-on-radio frame with the keys of whichever device it verifies
    /// for.
    ///
    /// # Arguments
    ///
    /// * `wor` - the frame.
    /// * `carrier` - the carrier it arrived on.
    ///
    /// # Returns
    ///
    /// The index of the device it came from, the full counter it carried, and where its
    /// uplink follows; or `None` when no trusted device has the address it names.
    ///
    /// # Errors
    ///
    /// When devices have the address but the frame verifies for none of them, the last of
    /// their errors, as [`TrustedDevice::open`] gives them.
    pub fn open(
        &mut self,
        wor: &SealedWor,
        carrier: Carrier,
    ) -> Option<Result<(u8, u32, Carrier), LorawanError>> {
        let mut outcome = None;
        for (index, entry) in (0u8..).zip(self.entries.iter_mut()) {
            let Some(device) = entry
                .as_mut()
                .filter(|device| device.dev_addr == wor.dev_addr())
            else {
                continue;
            };
            match device.open(wor, carrier) {
                Ok((wfcnt, uplink)) => return Some(Ok((index, wfcnt, uplink))),
                Err(error) => outcome = Some(Err(error)),
            }
        }
        outcome
    }

    /// Adds what every device's bucket earns over some hours.
    ///
    /// # Arguments
    ///
    /// * `hours` - the whole hours since the last reload.
    pub fn reload(&mut self, hours: u64) {
        for device in self.entries.iter_mut().flatten() {
            device.bucket.reload(hours);
        }
    }
}

impl Default for TrustedDevices {
    fn default() -> TrustedDevices {
        TrustedDevices::new()
    }
}
