//! A joined device's state, saved to outlast a loss of power and resumed afterward.
//!
//! A node that sleeps between readings loses its RAM, and joining again after every wake
//! costs airtime and a join nonce, and resets its frame counters. [`EndDevice::save`] turns
//! everything the device has settled with its network into bytes a program keeps in flash or
//! retained memory, and [`EndDevice::resume`] puts them back: the session and its keys, both
//! frame counters, every setting the network's MAC commands changed, the channels, the
//! answers still owed, and how long each duty cycle wait still has to run.
//!
//! The bytes hold the session keys, so keep them wherever the keys would be safe. A CRC-32
//! catches storage that corrupted them, and a fingerprint of the channel plan refuses a
//! state saved in another region.

use pamoja_lora::region::ChannelPlan;

use super::air::{Air, Sequence, MAX_SUB_BANDS};
use super::answers::{Answers, MAX_ANSWER, MAX_ANSWERS};
use super::channels::{Channels, MASK_GROUPS};
use super::{Channel, DeviceError, EndDevice, MAX_CHANNELS};
use crate::adr::Backoff;
use crate::Session;

/// How many bytes a saved state takes.
pub const SAVED_LEN: usize =
    HEADER_LEN + SESSION_LEN + SETTINGS_LEN + AIR_LEN + CHANNELS_LEN + ANSWERS_LEN + CHECKSUM_LEN;

const MAGIC: [u8; 4] = *b"PJLW";
const FORMAT: u8 = 1;

const HEADER_LEN: usize = 4 + 1 + 4 + 1 + 2;
const SESSION_LEN: usize = 4 + 16 + 16 + 4 + 4;
const SETTINGS_LEN: usize = 7 + 4 + 4 + 4 + 4;
const AIR_LEN: usize = 8 + 8 * MAX_SUB_BANDS;
const CHANNEL_LEN: usize = 4 + 4 + 1 + 1;
const CHANNELS_LEN: usize = 1 + 2 * MASK_GROUPS + 2 * MASK_GROUPS + CHANNEL_LEN * MAX_CHANNELS;
const ANSWERS_LEN: usize = 1 + MAX_ANSWERS * (1 + MAX_ANSWER);
const CHECKSUM_LEN: usize = 4;

const FCNT_DOWN: u8 = 1 << 0;
const JOINED_ON: u8 = 1 << 1;
const UPLINK_DWELL: u8 = 1 << 2;
const DOWNLINK_DWELL: u8 = 1 << 3;
const ACK_OWED: u8 = 1 << 4;
const LINK_CHECK: u8 = 1 << 5;
const DEVICE_TIME: u8 = 1 << 6;
const BACKOFF_RESTORED: u8 = 1 << 7;

const STICKY: u8 = 1 << 7;

/// Why a saved state could not be resumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StateError {
    /// The bytes are not the [`SAVED_LEN`] a saved state takes.
    Length,
    /// The bytes do not begin as a saved state does, their checksum does not match, or what
    /// they hold is out of range.
    Corrupt,
    /// The state was saved in a format this version does not read.
    Format(u8),
    /// The state was saved on a different channel plan.
    Plan,
}

impl core::fmt::Display for StateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StateError::Length => write!(f, "a saved state is {SAVED_LEN} bytes"),
            StateError::Corrupt => f.write_str("the saved state is corrupt"),
            StateError::Format(format) => write!(f, "saved state format {format} is not read here"),
            StateError::Plan => f.write_str("the state was saved on another channel plan"),
        }
    }
}

impl core::error::Error for StateError {}

/// A device's state as bytes to keep across a loss of power.
///
/// It holds the session keys; its `Debug` form leaves them, and everything else, out.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Saved {
    bytes: [u8; SAVED_LEN],
}

impl Saved {
    /// Returns the bytes to store.
    ///
    /// # Returns
    ///
    /// The [`SAVED_LEN`] bytes.
    pub const fn as_bytes(&self) -> &[u8; SAVED_LEN] {
        &self.bytes
    }

    /// Reads bytes a device saved.
    ///
    /// # Arguments
    ///
    /// * `bytes` - what was stored.
    ///
    /// # Returns
    ///
    /// The saved state, checked for length, format and checksum.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Length`], [`StateError::Corrupt`] or [`StateError::Format`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Saved, StateError> {
        let bytes: [u8; SAVED_LEN] = bytes.try_into().map_err(|_| StateError::Length)?;
        if bytes[..4] != MAGIC {
            return Err(StateError::Corrupt);
        }
        let body = SAVED_LEN - CHECKSUM_LEN;
        let stored = u32::from_le_bytes([
            bytes[body],
            bytes[body + 1],
            bytes[body + 2],
            bytes[body + 3],
        ]);
        if crc32(&[&bytes[..body]]) != stored {
            return Err(StateError::Corrupt);
        }
        if bytes[4] != FORMAT {
            return Err(StateError::Format(bytes[4]));
        }
        Ok(Saved { bytes })
    }
}

impl core::fmt::Debug for Saved {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Saved { .. }")
    }
}

impl<'p> EndDevice<'p> {
    /// Saves what the device has settled with its network, to resume after losing power.
    ///
    /// Save between exchanges, once the last uplink is done, and again after each one: a
    /// state resumed after an uplink it does not know about would send the next frame on a
    /// counter the network has already seen.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the time, in microseconds, which the duty cycle waits are counted from.
    ///
    /// # Returns
    ///
    /// The state.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NotJoined`] before the device has a session, and
    /// [`DeviceError::Busy`] while a transmission waits on its windows or its repeats.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lora::region::Region;
    /// use pamoja_lorawan::device::{EndDevice, Next, Saved, Settings};
    /// use pamoja_lorawan::Session;
    ///
    /// let settings = Settings::new(2, 14);
    /// let session = Session::new(0x2601_2E43, [0x5A; 16], [0xA5; 16]);
    /// let mut device = EndDevice::personalized(Region::Eu868.plan(), session, settings)?;
    /// device.send(2, b"21.5", false, 0)?;
    /// assert_eq!(device.nothing_heard(3_000_000)?, Next::Done);
    ///
    /// // Before sleeping, keep the bytes somewhere that survives it.
    /// let kept = device.save(3_000_000)?.as_bytes().to_vec();
    ///
    /// // On waking, build the device as before and carry on from them.
    /// let mut woken = EndDevice::personalized(Region::Eu868.plan(), session, settings)?;
    /// woken.resume(&Saved::from_bytes(&kept).expect("intact"), 0)?;
    /// assert_eq!(woken.fcnt_up(), 1, "the next frame does not reuse a counter");
    /// # Ok::<(), pamoja_lorawan::device::DeviceError>(())
    /// ```
    pub fn save(&self, now_us: u64) -> Result<Saved, DeviceError> {
        let session = self.session.ok_or(DeviceError::NotJoined)?;
        if self.pending.is_some() {
            return Err(DeviceError::Busy);
        }

        let mut bytes = [0u8; SAVED_LEN];
        let mut out = Writer {
            bytes: &mut bytes,
            at: 0,
        };
        let (link_check, device_time) = self.answers.requests();
        let flags = flag(self.fcnt_down.is_some(), FCNT_DOWN)
            | flag(self.joined_on.is_some(), JOINED_ON)
            | flag(self.uplink_dwell, UPLINK_DWELL)
            | flag(self.downlink_dwell, DOWNLINK_DWELL)
            | flag(self.ack_owed, ACK_OWED)
            | flag(link_check, LINK_CHECK)
            | flag(device_time, DEVICE_TIME)
            | flag(self.backoff.has_restored(), BACKOFF_RESTORED);
        out.put(&MAGIC);
        out.put(&[FORMAT]);
        out.put(&fingerprint(self.plan).to_le_bytes());
        out.put(&[flags]);
        out.put(&self.joined_on.unwrap_or(0).to_le_bytes());

        let (nwk_skey, app_skey) = session.keys();
        out.put(&session.dev_addr().to_le_bytes());
        out.put(&nwk_skey);
        out.put(&app_skey);
        out.put(&self.fcnt_up.to_le_bytes());
        out.put(&self.fcnt_down.unwrap_or(0).to_le_bytes());

        out.put(&[
            self.data_rate,
            self.tx_power,
            self.nb_trans,
            self.rx1_dr_offset,
            self.rx2_data_rate,
            self.max_eirp_dbm as u8,
            self.max_duty_cycle,
        ]);
        out.put(&self.rx2_frequency_hz.to_le_bytes());
        out.put(&self.rx1_delay_us.to_le_bytes());
        out.put(&self.backoff.counter().to_le_bytes());
        out.put(&self.sequence.state().to_le_bytes());

        let (aggregated, sub_bands) = self.air.remaining(now_us);
        out.put(&aggregated.to_le_bytes());
        for left in sub_bands {
            out.put(&left.to_le_bytes());
        }

        out.put(&[self.channels.default_count() as u8]);
        for group in self.channels.mask() {
            out.put(&group.to_le_bytes());
        }
        let mut defined = [0u16; MASK_GROUPS];
        for (index, slot) in self.channels.slots().iter().enumerate() {
            if slot.is_some() {
                defined[index / 16] |= 1 << (index % 16);
            }
        }
        for group in defined {
            out.put(&group.to_le_bytes());
        }
        for slot in self.channels.slots() {
            let channel = slot.unwrap_or(Channel::new(0, 0, 0));
            out.put(&channel.uplink_hz.to_le_bytes());
            out.put(&channel.downlink_hz.to_le_bytes());
            out.put(&[channel.min_data_rate, channel.max_data_rate]);
        }

        let mut answers = [0u8; ANSWERS_LEN];
        let mut at = 1;
        for (answer, sticky) in self.answers.owed() {
            answers[0] += 1;
            answers[at] = answer.len() as u8 | flag(sticky, STICKY);
            answers[at + 1..at + 1 + answer.len()].copy_from_slice(answer);
            at += 1 + answer.len();
        }
        out.put(&answers);

        let body = out.at;
        let checksum = crc32(&[&out.bytes[..body]]);
        out.put(&checksum.to_le_bytes());
        Ok(Saved { bytes })
    }

    /// Resumes a state [`save`](EndDevice::save) produced.
    ///
    /// Build the device as it was first built, with the same plan, identifiers and settings,
    /// and resume it before anything else. On CN470-510 any of the four RP002-1.0.5 plans
    /// will do, since the state names the join channel that chose its plan. The device's
    /// duty cycle waits run again from `now_us` for as long as they still had at the save,
    /// which never lets it transmit sooner than it could have without the loss of power.
    ///
    /// # Arguments
    ///
    /// * `saved` - the stored state.
    /// * `now_us` - the time, in microseconds, on the clock the device now runs by.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::State`] with [`StateError::Plan`] for a state saved on another
    /// plan and [`StateError::Corrupt`] for one whose contents are out of range, and
    /// [`DeviceError::Busy`] while a transmission waits on its windows or its repeats. The
    /// device is left as it was on any error.
    pub fn resume(&mut self, saved: &Saved, now_us: u64) -> Result<(), DeviceError> {
        if self.pending.is_some() {
            return Err(DeviceError::Busy);
        }
        let mut input = Reader {
            bytes: &saved.bytes,
            at: 5,
        };
        let corrupt = DeviceError::State(StateError::Corrupt);

        let stored_fingerprint = input.u32();
        let flags = input.u8();
        let joined_channel = input.u16();
        let joined_on = (flags & JOINED_ON != 0).then_some(joined_channel);
        let plan = match joined_on {
            Some(channel) => {
                self.plan
                    .join_plan(channel)
                    .ok_or(DeviceError::State(StateError::Plan))?
                    .0
                    .plan
            }
            None => self.plan,
        };
        if fingerprint(plan) != stored_fingerprint {
            return Err(DeviceError::State(StateError::Plan));
        }

        let dev_addr = input.u32();
        let nwk_skey = input.array::<16>();
        let app_skey = input.array::<16>();
        let fcnt_up = input.u32();
        let fcnt_down = input.u32();

        let [data_rate, tx_power, nb_trans, rx1_dr_offset, rx2_data_rate, max_eirp, max_duty_cycle] =
            input.array::<7>();
        let rx2_frequency_hz = input.u32();
        let rx1_delay_us = input.u32();
        let backoff_counter = input.u32();
        let sequence = input.u32();

        let aggregated = input.u64();
        let mut sub_bands = [0u64; MAX_SUB_BANDS];
        for left in &mut sub_bands {
            *left = input.u64();
        }

        let defaults = usize::from(input.u8());
        let mut enabled = [0u16; MASK_GROUPS];
        for group in &mut enabled {
            *group = input.u16();
        }
        let mut defined = [0u16; MASK_GROUPS];
        for group in &mut defined {
            *group = input.u16();
        }
        let mut slots = [None; MAX_CHANNELS];
        for (index, slot) in slots.iter_mut().enumerate() {
            let uplink_hz = input.u32();
            let downlink_hz = input.u32();
            let [min_data_rate, max_data_rate] = input.array::<2>();
            if defined[index / 16] & (1 << (index % 16)) != 0 {
                *slot = Some(Channel {
                    uplink_hz,
                    downlink_hz,
                    min_data_rate,
                    max_data_rate,
                });
            }
        }
        let stray = enabled
            .iter()
            .zip(defined)
            .any(|(enabled, defined)| enabled & !defined != 0);
        if defaults > MAX_CHANNELS || stray || nb_trans == 0 || nb_trans > 15 {
            return Err(corrupt);
        }

        let mut answers = Answers::new();
        let count = input.u8();
        for _ in 0..count {
            let header = input.u8();
            let len = usize::from(header & !STICKY);
            if len == 0 || len > MAX_ANSWER {
                return Err(corrupt);
            }
            let bytes = input.take(len);
            if !answers.push_encoded(bytes, header & STICKY != 0) {
                return Err(corrupt);
            }
        }
        if flags & LINK_CHECK != 0 {
            answers.request_link_check();
        }
        if flags & DEVICE_TIME != 0 {
            answers.request_device_time();
        }

        self.plan = plan;
        self.session = Some(Session::new(dev_addr, nwk_skey, app_skey));
        self.joined_on = joined_on;
        self.fcnt_up = fcnt_up;
        self.fcnt_down = (flags & FCNT_DOWN != 0).then_some(fcnt_down);
        self.data_rate = data_rate;
        self.tx_power = tx_power;
        self.nb_trans = nb_trans;
        self.rx1_dr_offset = rx1_dr_offset;
        self.rx2_frequency_hz = rx2_frequency_hz;
        self.rx2_data_rate = rx2_data_rate;
        self.rx1_delay_us = rx1_delay_us;
        self.max_eirp_dbm = max_eirp as i8;
        self.uplink_dwell = flags & UPLINK_DWELL != 0;
        self.downlink_dwell = flags & DOWNLINK_DWELL != 0;
        self.max_duty_cycle = max_duty_cycle;
        self.channels = Channels::from_parts(slots, enabled, defaults);
        self.answers = answers;
        self.backoff = Backoff::recommended(self.settings.version)
            .resumed(backoff_counter, flags & BACKOFF_RESTORED != 0);
        self.air = Air::resumed(now_us, aggregated, sub_bands);
        self.sequence = Sequence::from_state(sequence);
        self.ack_owed = flags & ACK_OWED != 0;
        self.joins = 0;
        self.join_used = [0; MASK_GROUPS];
        self.join_slot = 0;
        self.quiet_until_us = 0;
        Ok(())
    }
}

const fn flag(on: bool, bit: u8) -> u8 {
    if on {
        bit
    } else {
        0
    }
}

/// A fingerprint of what a saved state depends on in a plan: its name, its channels, and its
/// second receive window.
fn fingerprint(plan: &ChannelPlan) -> u32 {
    let mut crc = Crc32::new();
    crc.update(plan.name.as_bytes());
    for blocks in [plan.default_channels, plan.downlink_channels] {
        crc.update(&(blocks.len() as u32).to_le_bytes());
        for block in blocks {
            crc.update(&block.start_hz.to_le_bytes());
            crc.update(&block.step_hz.to_le_bytes());
            crc.update(&block.count.to_le_bytes());
            crc.update(&[block.min_data_rate, block.max_data_rate]);
        }
    }
    crc.update(&plan.rx2_frequency_hz.to_le_bytes());
    crc.update(&[plan.rx2_data_rate]);
    crc.finish()
}

/// The CRC-32 of ISO-HDLC, the one Ethernet and zlib use.
fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = Crc32::new();
    for part in parts {
        crc.update(part);
    }
    crc.finish()
}

struct Crc32(u32);

impl Crc32 {
    const fn new() -> Crc32 {
        Crc32(0xFFFF_FFFF)
    }

    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u32::from(byte);
            for _ in 0..8 {
                let mask = (self.0 & 1).wrapping_neg();
                self.0 = (self.0 >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
    }

    const fn finish(&self) -> u32 {
        !self.0
    }
}

struct Writer<'a> {
    bytes: &'a mut [u8; SAVED_LEN],
    at: usize,
}

impl Writer<'_> {
    fn put(&mut self, bytes: &[u8]) {
        self.bytes[self.at..self.at + bytes.len()].copy_from_slice(bytes);
        self.at += bytes.len();
    }
}

struct Reader<'a> {
    bytes: &'a [u8; SAVED_LEN],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> &'a [u8] {
        let end = (self.at + len).min(SAVED_LEN);
        let taken = &self.bytes[self.at.min(end)..end];
        self.at = end;
        taken
    }

    fn array<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        let taken = self.take(N);
        out[..taken.len()].copy_from_slice(taken);
        out
    }

    fn u8(&mut self) -> u8 {
        self.array::<1>()[0]
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.array())
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.array())
    }

    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.array())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checksum_is_the_iso_hdlc_crc_32() {
        // The catalogue check value of CRC-32/ISO-HDLC.
        assert_eq!(crc32(&[b"123456789"]), 0xCBF4_3926);
        assert_eq!(crc32(&[b"1234", b"56789"]), 0xCBF4_3926);
        assert_eq!(crc32(&[]), 0);
    }

    #[test]
    fn the_layout_adds_up() {
        assert_eq!(SAVED_LEN, 1_589);
    }

    #[test]
    fn a_state_in_a_later_format_is_named_rather_than_misread() {
        use pamoja_lora::region::Region;

        let session = Session::new(0x2601_2E43, [1; 16], [2; 16]);
        let device = EndDevice::personalized(
            Region::Eu868.plan(),
            session,
            super::super::Settings::new(2, 14),
        )
        .expect("the plan fits");
        let mut bytes = *device
            .save(0)
            .expect("a personalized device has a session")
            .as_bytes();
        assert_eq!(
            Saved::from_bytes(&bytes).map(|saved| saved.bytes),
            Ok(bytes)
        );

        bytes[4] = 2;
        let body = SAVED_LEN - CHECKSUM_LEN;
        let checksum = crc32(&[&bytes[..body]]);
        bytes[body..].copy_from_slice(&checksum.to_le_bytes());
        assert_eq!(Saved::from_bytes(&bytes), Err(StateError::Format(2)));
    }
}
