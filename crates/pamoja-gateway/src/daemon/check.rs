//! Downlinks held back until just before their window, on channels a gateway checks first.
//!
//! A carrier check answers for the moment it is made, and the concentrator reads that answer
//! as the packet leaves. So a checked packet cannot be programmed the moment the network
//! sends it, a second or more ahead, the way an unchecked one is. Semtech's packet forwarder
//! queues it and hands it to the concentrator only a few tens of milliseconds before it is due;
//! `lgw_send` then points the SX1261 at the channel, runs the scan, arms the chain, and waits
//! for the gain control to say whether the packet went out. A [`Queue`] holds the packets in
//! between, and [`when`] decides which of the three a newly arrived one gets.

/// How close to its window a checked downlink is handed to the concentrator, in microseconds.
///
/// A queued downlink is looked at once every poll, ten milliseconds apart, and has to come out
/// with at least [`CHECKED_TOO_LATE_US`] still to go, so this is that margin, one poll, and
/// some room.
pub const CHECK_LEAD_US: u32 = 80_000;

/// How close to its window a checked downlink can arrive and still be sent, in microseconds.
///
/// The margin an unchecked downlink needs to be programmed, 42.5 ms, and the 20 ms the check
/// takes before the chain can be: the receiver pointed at the channel in eight commands and the
/// check started in a ninth, each after the millisecond the SX1261 is given, then a scan of up
/// to 5 ms.
pub const CHECKED_TOO_LATE_US: u32 = 42_500 + 20_000;

/// What to do with a checked downlink when it arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum When {
    /// Its window is too close to check the channel and still make it.
    TooLate,
    /// Check and send it now.
    Now,
    /// Hold it until [`CHECK_LEAD_US`] before its window.
    Hold,
}

/// Decides what a checked downlink gets.
///
/// # Arguments
///
/// * `ahead_us` - how far ahead of now its window is, in the concentrator's microseconds.
///
/// # Returns
///
/// Whether it is too late, due now, or to be held.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::daemon::check::{when, When};
///
/// assert_eq!(when(1_000_000), When::Hold, "a receive window a second out");
/// assert_eq!(when(70_000), When::Now);
/// assert_eq!(when(60_000), When::TooLate);
/// ```
pub const fn when(ahead_us: u32) -> When {
    if ahead_us <= CHECKED_TOO_LATE_US {
        When::TooLate
    } else if ahead_us <= CHECK_LEAD_US {
        When::Now
    } else {
        When::Hold
    }
}

/// A downlink taken out of the queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Due<T> {
    /// The concentrator count its window opens at.
    pub at: u32,
    /// What was held.
    pub item: T,
    /// Whether the window had already passed when the queue was asked, which a gateway that
    /// stalled for longer than the lead reports rather than transmits.
    pub missed: bool,
}

/// Checked downlinks waiting for their moment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Queue<T> {
    held: Vec<(u32, T)>,
}

impl<T> Queue<T> {
    /// An empty queue.
    ///
    /// # Returns
    ///
    /// The queue.
    pub const fn new() -> Queue<T> {
        Queue { held: Vec::new() }
    }

    /// Holds a downlink until its window is near.
    ///
    /// # Arguments
    ///
    /// * `at` - the concentrator count its window opens at.
    /// * `item` - what to hand back when it is due.
    pub fn hold(&mut self, at: u32, item: T) {
        self.held.push((at, item));
    }

    /// How many downlinks are held.
    ///
    /// # Returns
    ///
    /// The count.
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether nothing is held.
    ///
    /// # Returns
    ///
    /// `true` for an empty queue.
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// Takes every downlink whose window is within [`CHECK_LEAD_US`] of now, or has passed.
    ///
    /// The count wraps, so a window is taken as passed when it reads as more than half the
    /// counter ahead, which is what parts a moment not yet reached from one long past.
    ///
    /// # Arguments
    ///
    /// * `now` - the concentrator's count now.
    ///
    /// # Returns
    ///
    /// The downlinks due, earliest window first.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_gateway::daemon::check::Queue;
    ///
    /// let mut queue = Queue::new();
    /// queue.hold(2_000_000, "second");
    /// queue.hold(1_000_000, "first");
    /// queue.hold(9_000_000, "later");
    ///
    /// let due = queue.due(1_930_000);
    /// assert_eq!(due.iter().map(|due| due.item).collect::<Vec<_>>(), ["first", "second"]);
    /// assert!(due[0].missed, "its window opened 930 ms ago");
    /// assert!(!due[1].missed);
    /// assert_eq!(queue.len(), 1);
    /// ```
    pub fn due(&mut self, now: u32) -> Vec<Due<T>> {
        let mut due = Vec::new();
        let mut index = 0;
        while index < self.held.len() {
            let ahead = self.held[index].0.wrapping_sub(now);
            let missed = ahead > u32::MAX / 2;
            if missed || ahead <= CHECK_LEAD_US {
                let (at, item) = self.held.swap_remove(index);
                due.push(Due { at, item, missed });
            } else {
                index += 1;
            }
        }
        due.sort_by_key(|due| due.at.wrapping_sub(now).wrapping_add(u32::MAX / 2));
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_outcomes_meet_at_their_margins() {
        assert_eq!(when(CHECKED_TOO_LATE_US), When::TooLate);
        assert_eq!(when(CHECKED_TOO_LATE_US + 1), When::Now);
        assert_eq!(when(CHECK_LEAD_US), When::Now);
        assert_eq!(when(CHECK_LEAD_US + 1), When::Hold);
    }

    #[test]
    fn a_window_across_the_counter_wrap_is_not_taken_early_or_lost() {
        let mut queue = Queue::new();
        // Due 10 ms after the count wraps, asked 100 ms and then 30 ms before it.
        queue.hold(10_000, 'a');
        assert!(queue.due(u32::MAX - 89_999).is_empty());
        let due = queue.due(u32::MAX - 19_999);
        assert_eq!(due.len(), 1);
        assert!(!due[0].missed);
        assert!(queue.is_empty());
    }

    #[test]
    fn the_earliest_window_comes_out_first_whatever_order_it_was_held_in() {
        let mut queue = Queue::new();
        for at in [5_040_000, 5_010_000, 5_030_000, 5_020_000] {
            queue.hold(at, at);
        }
        let order: Vec<u32> = queue.due(5_000_000).into_iter().map(|due| due.at).collect();
        assert_eq!(order, [5_010_000, 5_020_000, 5_030_000, 5_040_000]);
    }
}
