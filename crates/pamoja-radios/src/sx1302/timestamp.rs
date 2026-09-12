//! The counter a concentrator stamps packets against.
//!
//! Every packet the chip hears carries the value of a counter running at 32 MHz, and every
//! packet the chip sends at a chosen moment is given a value of that same counter. It is what
//! a receive window is measured against, so a gateway that answers an uplink is really doing
//! arithmetic on this counter and nothing else.
//!
//! Two things make it harder than reading a number. The counter is 27 bits wide, so it rolls
//! over about every two minutes, and the chip does not say how many times it has. A reader
//! therefore keeps a little state: the last value it saw and how many times it has seen the
//! count go backward. A packet is widened against that same state, and the rule is not the
//! same one, because a packet whose count is above the last reading was heard before the most
//! recent rollover rather than after it.
//!
//! Nothing here reads a register. The bytes come from one burst and this turns them into a
//! counter, so the arithmetic can be checked without hardware.

use super::register::TIMESTAMP_BASE;

/// Where both counters are read from, in one burst.
pub const COUNTERS: u16 = TIMESTAMP_BASE + 1;

/// How many bytes that burst carries: four for each counter.
pub const COUNTERS_LEN: usize = 8;

/// How many ticks of the counter make a microsecond.
pub const TICKS_PER_US: u32 = 32;

/// How many bits the counter actually holds.
pub const COUNTER_BITS: u32 = 27;

/// How many rollovers are counted before the count itself repeats.
pub const WRAPS: u8 = 32;

/// One counter, and what is known about how often it has rolled over.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tracked {
    /// The last value read, in microseconds.
    pub reference: u32,
    /// How many times it has been seen to roll over, kept to five bits.
    pub wraps: u8,
}

impl Tracked {
    /// Takes a new reading, noticing a rollover.
    ///
    /// # Arguments
    ///
    /// * `now` - the value just read, in microseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::timestamp::Tracked;
    ///
    /// let mut counter = Tracked::default();
    /// counter.advance(1_000);
    /// assert_eq!(counter.wraps, 0);
    ///
    /// // A reading below the last one can only mean it went round.
    /// counter.advance(5);
    /// assert_eq!(counter.wraps, 1);
    /// ```
    pub const fn advance(&mut self, now: u32) {
        if now < self.reference {
            self.wraps = (self.wraps + 1) % WRAPS;
        }
        self.reference = now;
    }

    /// The last reading, widened past the rollover.
    ///
    /// # Returns
    ///
    /// The value in microseconds, counting every rollover so far.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::timestamp::Tracked;
    ///
    /// let mut counter = Tracked::default();
    /// counter.advance(1_000);
    /// counter.advance(5);
    ///
    /// // One rollover, so the reading sits a whole counter above where it reads.
    /// assert_eq!(counter.widened(), (1 << 27) | 5);
    /// ```
    #[must_use]
    pub const fn widened(&self) -> u32 {
        ((self.wraps as u32) << COUNTER_BITS) | self.reference
    }
}

/// Both counters a concentrator keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counter {
    /// The one that free runs, which packets are stamped against.
    pub free: Tracked,
    /// The one a pulse per second line latches, which disciplines the other.
    pub pulse: Tracked,
}

impl Counter {
    /// A counter that has read nothing yet.
    ///
    /// # Returns
    ///
    /// The counter, at zero and having seen no rollover.
    #[must_use]
    pub const fn new() -> Counter {
        Counter {
            free: Tracked {
                reference: 0,
                wraps: 0,
            },
            pulse: Tracked {
                reference: 0,
                wraps: 0,
            },
        }
    }

    /// Takes a reading from the eight bytes the chip answers with.
    ///
    /// The pulse counter comes first and the free running one after it, each most significant
    /// byte first, and both are scaled from the 32 MHz they count at.
    ///
    /// # Arguments
    ///
    /// * `bytes` - what the burst read answered with.
    ///
    /// # Returns
    ///
    /// The free running counter and then the pulse one, both in microseconds and both
    /// widened past any rollover seen so far.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::timestamp::Counter;
    ///
    /// let mut counter = Counter::new();
    ///
    /// // The free running counter reads 64 ticks, which is two microseconds.
    /// let (free, pulse) = counter.read(&[0, 0, 0, 32, 0, 0, 0, 64]);
    /// assert_eq!((free, pulse), (2, 1));
    /// ```
    pub const fn read(&mut self, bytes: &[u8; COUNTERS_LEN]) -> (u32, u32) {
        let pulse = microseconds([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let free = microseconds([bytes[4], bytes[5], bytes[6], bytes[7]]);
        self.pulse.advance(pulse);
        self.free.advance(free);
        (self.free.widened(), self.pulse.widened())
    }

    /// Widens the counter value a packet was stamped with.
    ///
    /// A packet reading above the last counter reading was heard before the most recent
    /// rollover, not after it, so it belongs to the wrap count before this one.
    ///
    /// # Arguments
    ///
    /// * `packet` - what the packet carried, in microseconds.
    ///
    /// # Returns
    ///
    /// The value widened past the rollovers seen so far.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::timestamp::Counter;
    ///
    /// let mut counter = Counter::new();
    /// counter.read(&[0, 0, 0, 0, 0, 0, 0, 32]);
    ///
    /// // A packet heard just before the counter reading sits in the same stretch.
    /// assert_eq!(counter.widened_packet(1), 1);
    /// ```
    #[must_use]
    pub const fn widened_packet(&self, packet: u32) -> u32 {
        let wraps = if self.free.reference >= packet {
            self.free.wraps
        } else {
            self.free.wraps.wrapping_sub(1)
        } % WRAPS;
        ((wraps as u32) << COUNTER_BITS) | packet
    }
}

/// Whether a second reading of the counters disagrees with the first.
///
/// A read of eight bytes can catch the counter mid-step, so the reference reads them twice
/// and takes a third reading when either high byte has moved. It is a different guard from
/// the one on the receive byte count, where the larger of two readings is kept instead.
///
/// # Arguments
///
/// * `first` - the first reading.
/// * `second` - the second.
///
/// # Returns
///
/// Whether either counter stepped over a high byte between them.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::timestamp::disagrees;
///
/// // The low bytes moving is ordinary; the counter is running.
/// assert!(!disagrees(&[1, 0, 0, 0, 2, 0, 0, 0], &[1, 0, 0, 9, 2, 0, 0, 9]));
///
/// // A high byte moving means the reading straddles a step.
/// assert!(disagrees(&[1, 0, 0, 0, 2, 0, 0, 0], &[1, 0, 0, 0, 3, 0, 0, 0]));
/// ```
#[must_use]
pub const fn disagrees(first: &[u8; COUNTERS_LEN], second: &[u8; COUNTERS_LEN]) -> bool {
    first[0] != second[0] || first[4] != second[4]
}

/// Turns the four bytes of a counter into microseconds.
const fn microseconds(bytes: [u8; 4]) -> u32 {
    u32::from_be_bytes(bytes) / TICKS_PER_US
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counter_is_scaled_from_the_rate_it_runs_at() {
        // 32 MHz, so a microsecond is 32 ticks and nothing else.
        let mut counter = Counter::new();
        let (free, pulse) = counter.read(&[0, 0, 0x01, 0x00, 0, 0, 0x02, 0x00]);
        assert_eq!((free, pulse), (16, 8));
    }

    #[test]
    fn a_reading_that_goes_backward_is_a_rollover() {
        let mut counter = Counter::new();
        counter.read(&[0, 0, 0, 0, 0x07, 0xff, 0xff, 0xe0]);
        let before = counter.free.wraps;

        // The next reading is lower, which a counter running forward cannot do.
        counter.read(&[0, 0, 0, 0, 0, 0, 0, 0x20]);
        assert_eq!(counter.free.wraps, before + 1);
        assert_eq!(counter.free.widened(), (1 << COUNTER_BITS) | 1);
    }

    #[test]
    fn a_packet_heard_before_a_rollover_keeps_the_earlier_count() {
        let mut counter = Counter::new();

        // The counter has gone round once and now reads low.
        counter.read(&[0, 0, 0, 0, 0x07, 0xff, 0xff, 0xe0]);
        counter.read(&[0, 0, 0, 0, 0, 0, 0, 0x20]);
        assert_eq!(counter.free.wraps, 1);

        // A packet stamped below the reading belongs to this stretch.
        assert_eq!(counter.widened_packet(1), (1 << COUNTER_BITS) | 1);

        // One stamped above it was heard before the rollover, so it belongs to the last.
        assert_eq!(counter.widened_packet(0x07ff_fff0), 0x07ff_fff0);
    }

    #[test]
    fn the_wrap_count_comes_back_round_rather_than_running_off() {
        let mut counter = Counter::new();
        for _ in 0..WRAPS {
            counter.read(&[0, 0, 0, 0, 0, 0, 0x20, 0x00]);
            counter.read(&[0, 0, 0, 0, 0, 0, 0, 0x20]);
        }
        assert!(counter.free.wraps < WRAPS, "{}", counter.free.wraps);

        // And a packet read against a fresh counter takes the count below zero, which has to
        // come back round rather than underflow.
        let fresh = Counter::new();
        assert_eq!(fresh.widened_packet(5), (31 << COUNTER_BITS) | 5);
    }

    #[test]
    fn only_a_high_byte_moving_forces_a_third_reading() {
        let first = [0x00, 0x11, 0x22, 0x33, 0x00, 0x44, 0x55, 0x66];
        assert!(!disagrees(
            &first,
            &[0x00, 0x11, 0x22, 0x99, 0x00, 0x44, 0x55, 0x99]
        ));
        assert!(disagrees(
            &first,
            &[0x01, 0x11, 0x22, 0x33, 0x00, 0x44, 0x55, 0x66]
        ));
        assert!(disagrees(
            &first,
            &[0x00, 0x11, 0x22, 0x33, 0x01, 0x44, 0x55, 0x66]
        ));
    }
}
