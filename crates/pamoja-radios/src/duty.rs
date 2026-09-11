//! A duty-cycle guard for a radio.
//!
//! A regional duty-cycle limit caps the share of time a node may transmit, and
//! [`LinkSettings::min_off_time_us`] turns that limit into the silence a transmission
//! of a given length owes. [`DutyCycle`] keeps the account against a clock the caller
//! supplies, so the same guard works over a microcontroller's hardware timer and a
//! host's monotonic clock.

use pamoja_lora::LinkSettings;

/// The silence a radio owes after its transmissions under a duty-cycle limit.
///
/// Record each transmission with [`transmitted`](DutyCycle::transmitted), then ask
/// [`ready`](DutyCycle::ready) or [`wait_us`](DutyCycle::wait_us) before the next. A
/// limit of zero forbids transmitting, and the guard never becomes ready.
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
/// use pamoja_radios::duty::DutyCycle;
///
/// // SF12 at 125 kHz under a 1% limit.
/// let link = LinkSettings::new(12, 125_000);
/// let mut guard = DutyCycle::new(10);
/// assert!(guard.ready(0));
///
/// // A ten-byte reading sent at time zero holds the channel for 991,232 us, and owes
/// // ninety-nine times that in silence.
/// assert_eq!(guard.transmitted(0, &link, 10), 991_232);
/// assert_eq!(guard.wait_us(0), 99_123_200);
/// assert!(!guard.ready(99_000_000));
/// assert!(guard.ready(99_123_200));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DutyCycle {
    permille: u32,
    earliest_us: u64,
}

impl DutyCycle {
    /// Creates a guard for a duty-cycle limit, ready to transmit at once.
    ///
    /// # Arguments
    ///
    /// * `permille` - the limit in parts per thousand, so `10` is 1%; `0` forbids
    ///   transmitting and `1000` or more imposes no silence.
    ///
    /// # Returns
    ///
    /// The guard.
    pub const fn new(permille: u32) -> Self {
        Self {
            permille,
            earliest_us: if permille == 0 { u64::MAX } else { 0 },
        }
    }

    /// Returns the limit the guard enforces.
    ///
    /// # Returns
    ///
    /// The limit in parts per thousand.
    pub fn permille(&self) -> u32 {
        self.permille
    }

    /// Returns the earliest time the next transmission may start.
    ///
    /// # Returns
    ///
    /// A time in microseconds on the caller's clock, or [`u64::MAX`] when the limit
    /// forbids transmitting.
    pub fn earliest_us(&self) -> u64 {
        self.earliest_us
    }

    /// Returns how long the radio must still stay silent.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the current time in microseconds on the caller's clock.
    ///
    /// # Returns
    ///
    /// The remaining silence in microseconds, zero when a transmission may start.
    pub fn wait_us(&self, now_us: u64) -> u64 {
        self.earliest_us.saturating_sub(now_us)
    }

    /// Reports whether a transmission may start now.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the current time in microseconds on the caller's clock.
    ///
    /// # Returns
    ///
    /// `true` once the silence the last transmission owed has passed.
    pub fn ready(&self, now_us: u64) -> bool {
        self.permille != 0 && now_us >= self.earliest_us
    }

    /// Records a transmission and the silence it owes.
    ///
    /// # Arguments
    ///
    /// * `started_us` - when the transmission started, in microseconds on the caller's
    ///   clock.
    /// * `link` - the settings the frame was sent with.
    /// * `payload_len` - the payload length in bytes.
    ///
    /// # Returns
    ///
    /// The frame's time on air in microseconds.
    pub fn transmitted(&mut self, started_us: u64, link: &LinkSettings, payload_len: usize) -> u64 {
        let airtime = link.airtime_us(payload_len);
        if self.permille == 0 {
            return airtime;
        }
        let off_time = link.min_off_time_us(payload_len, self.permille.min(1000));
        self.earliest_us = started_us
            .saturating_add(airtime)
            .saturating_add(off_time);
        airtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_guard_is_ready_at_once() {
        assert!(DutyCycle::new(10).ready(0));
        assert_eq!(DutyCycle::new(10).wait_us(123), 0);
    }

    #[test]
    fn a_transmission_owes_its_off_time_after_its_airtime() {
        let link = LinkSettings::new(7, 125_000);
        let mut guard = DutyCycle::new(10);
        let airtime = guard.transmitted(5_000, &link, 20);
        assert_eq!(guard.earliest_us(), 5_000 + airtime + airtime * 99);
        assert!(!guard.ready(5_000 + airtime * 100 - 1));
        assert!(guard.ready(5_000 + airtime * 100));
    }

    #[test]
    fn a_zero_limit_never_becomes_ready() {
        let link = LinkSettings::new(7, 125_000);
        let mut guard = DutyCycle::new(0);
        assert!(!guard.ready(0));
        guard.transmitted(0, &link, 10);
        assert!(!guard.ready(u64::MAX));
        assert_eq!(guard.wait_us(0), u64::MAX);
    }

    #[test]
    fn a_limit_of_a_thousand_or_more_owes_no_silence() {
        let link = LinkSettings::new(7, 125_000);
        for permille in [1_000, 1_500] {
            let mut guard = DutyCycle::new(permille);
            let airtime = guard.transmitted(0, &link, 10);
            assert!(guard.ready(airtime));
        }
    }
}
