//! Keeping a device on the air when the network stops answering.
//!
//! A network that runs adaptive data rate moves a device to the fastest rate that still
//! reaches it, which costs the device less battery and the network less air time. The risk
//! is the other direction: a device moved up and then left there, in a spot the network can
//! no longer hear, would go on transmitting into nothing forever.
//!
//! So a device counts. Every new uplink raises a counter, any downlink at all resets it, and
//! once the counter passes a limit the device starts asking the network to say something.
//! Past that it steps its own rate back down, again and again, until either the network
//! answers or the device is at the slowest rate it has.
//!
//! This is the device half, and it is the half the specification actually settles: section
//! 4.3.1.1 of LoRaWAN 1.0.3 says when the request bit goes out and when a rate steps down.
//! Which rate to move a device to in the first place is a policy the specification
//! deliberately leaves open, and it is not here.
//!
//! The two counts, the limit and the delay, are regional. The core specification names them
//! and does not give them values, so [`Backoff::new`] takes them rather than guessing: they
//! come from the regional parameters document for the band a device runs in.

/// How a device should send the next uplink, and what the back-off did to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Whether to set the acknowledgment request bit, asking the network to say something.
    ///
    /// It is never set at the slowest rate a device has, because nothing could be done
    /// about the answer.
    pub request_ack: bool,
    /// The data rate to send at.
    pub data_rate: u8,
    /// Whether this uplink is the one that stepped the rate down.
    pub lowered: bool,
}

/// A device's count of how long the network has been silent.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::adr::Backoff;
///
/// // Starting at the fastest rate a European device uses, with the slowest being 0.
/// let mut backoff = Backoff::new(5, 0, 64, 32);
///
/// // Nothing is asked for while the network is still answering.
/// for _ in 0..70 {
///     backoff.uplink();
///     backoff.downlink();
/// }
/// assert!(!backoff.uplink().request_ack, "one uplink since the last answer");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Backoff {
    counter: u32,
    limit: u32,
    delay: u32,
    data_rate: u8,
    lowest: u8,
}

impl Backoff {
    /// A device at a rate, counting from zero.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the rate it is sending at now.
    /// * `lowest` - the slowest rate it has, which it will not step below.
    /// * `limit` - how many unanswered uplinks before it starts asking. Regional.
    /// * `delay` - how many more before it steps its rate down, and between each step
    ///   after that. Regional, and treated as one if given as zero, since a step every
    ///   zero uplinks is not a thing a device can do.
    ///
    /// # Returns
    ///
    /// The count.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::adr::Backoff;
    ///
    /// let backoff = Backoff::new(5, 0, 64, 32);
    /// assert_eq!(backoff.data_rate(), 5);
    /// assert_eq!(backoff.counter(), 0);
    /// ```
    #[must_use]
    pub const fn new(data_rate: u8, lowest: u8, limit: u32, delay: u32) -> Backoff {
        Backoff {
            counter: 0,
            limit,
            delay: if delay == 0 { 1 } else { delay },
            data_rate,
            lowest,
        }
    }

    /// Counts one new uplink, and says how to send it.
    ///
    /// Call this once per uplink the frame counter moves for. A repeated transmission of
    /// the same uplink is not a new one and does not count, which is the same rule the
    /// frame counter itself follows.
    ///
    /// # Returns
    ///
    /// Whether to ask the network for an answer, the rate to send at, and whether this
    /// uplink is the one that stepped that rate down.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::adr::Backoff;
    ///
    /// let mut backoff = Backoff::new(5, 0, 64, 32);
    /// for _ in 0..63 {
    ///     assert!(!backoff.uplink().request_ack);
    /// }
    ///
    /// // The sixty-fourth unanswered uplink is the one that starts asking.
    /// assert!(backoff.uplink().request_ack);
    /// ```
    pub fn uplink(&mut self) -> Step {
        self.counter = self.counter.saturating_add(1);

        // The first step comes a delay after the limit, and another every delay after that,
        // for as long as the network stays silent.
        let lowered = if self.counter >= self.limit.saturating_add(self.delay) {
            let since = self.counter - self.limit;
            if since.is_multiple_of(self.delay) && self.data_rate > self.lowest {
                self.data_rate -= 1;
                true
            } else {
                false
            }
        } else {
            false
        };

        Step {
            request_ack: self.counter >= self.limit && self.data_rate > self.lowest,
            data_rate: self.data_rate,
            lowered,
        }
    }

    /// Notes that the network said something.
    ///
    /// Any downlink counts, whatever it carries and whether or not it acknowledges
    /// anything: it arriving at all is the proof that the uplinks are still being heard.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::adr::Backoff;
    ///
    /// let mut backoff = Backoff::new(5, 0, 64, 32);
    /// for _ in 0..64 {
    ///     backoff.uplink();
    /// }
    /// assert!(backoff.uplink().request_ack);
    ///
    /// backoff.downlink();
    /// assert_eq!(backoff.counter(), 0);
    /// assert!(!backoff.uplink().request_ack, "and it stops asking");
    /// ```
    pub fn downlink(&mut self) {
        self.counter = 0;
    }

    /// How many uplinks have gone unanswered.
    ///
    /// # Returns
    ///
    /// The count since the last downlink.
    #[must_use]
    pub const fn counter(&self) -> u32 {
        self.counter
    }

    /// The rate the device is sending at.
    ///
    /// # Returns
    ///
    /// The rate, which only ever falls while the network is silent.
    #[must_use]
    pub const fn data_rate(&self) -> u8 {
        self.data_rate
    }

    /// The slowest rate this device will step down to.
    ///
    /// # Returns
    ///
    /// The floor.
    #[must_use]
    pub const fn lowest(&self) -> u8 {
        self.lowest
    }

    /// Puts the device on a rate the network chose.
    ///
    /// # Arguments
    ///
    /// * `data_rate` - the rate the network asked for.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::adr::Backoff;
    ///
    /// let mut backoff = Backoff::new(0, 0, 64, 32);
    /// backoff.set_data_rate(5);
    /// assert_eq!(backoff.data_rate(), 5);
    /// ```
    pub fn set_data_rate(&mut self, data_rate: u8) {
        self.data_rate = data_rate;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: u32 = 64;
    const DELAY: u32 = 32;

    // Sends `count` uplinks and hands back the last step.
    fn after(backoff: &mut Backoff, count: u32) -> Step {
        let mut step = backoff.uplink();
        for _ in 1..count {
            step = backoff.uplink();
        }
        step
    }

    #[test]
    fn nothing_is_asked_for_until_the_limit_is_reached() {
        let mut backoff = Backoff::new(5, 0, LIMIT, DELAY);

        assert!(!after(&mut backoff, LIMIT - 1).request_ack);
        assert!(
            backoff.uplink().request_ack,
            "the uplink that reaches the limit is the one that asks"
        );
    }

    #[test]
    fn any_downlink_at_all_resets_the_count() {
        let mut backoff = Backoff::new(5, 0, LIMIT, DELAY);
        assert!(after(&mut backoff, LIMIT).request_ack);

        backoff.downlink();
        assert_eq!(backoff.counter(), 0);
        assert!(!backoff.uplink().request_ack);
        assert_eq!(
            backoff.data_rate(),
            5,
            "and the rate it was moved to stands"
        );
    }

    #[test]
    fn the_rate_steps_down_a_delay_after_the_limit_and_every_delay_after_that() {
        let mut backoff = Backoff::new(5, 0, LIMIT, DELAY);

        // Asking, but not yet stepping.
        let step = after(&mut backoff, LIMIT + DELAY - 1);
        assert!(step.request_ack);
        assert!(!step.lowered);
        assert_eq!(step.data_rate, 5);

        let step = backoff.uplink();
        assert!(step.lowered, "the first step comes a delay after the limit");
        assert_eq!(step.data_rate, 4);

        // And another every delay, for as long as the network stays quiet.
        let step = after(&mut backoff, DELAY);
        assert!(step.lowered);
        assert_eq!(step.data_rate, 3);

        let step = after(&mut backoff, DELAY);
        assert!(step.lowered);
        assert_eq!(step.data_rate, 2);
    }

    #[test]
    fn a_device_never_steps_below_the_slowest_rate_it_has() {
        let mut backoff = Backoff::new(2, 1, LIMIT, DELAY);

        let step = after(&mut backoff, LIMIT + DELAY);
        assert_eq!(step.data_rate, 1, "it stepped down to its floor");

        // Every further step is refused, however long the network stays silent.
        for _ in 0..DELAY * 4 {
            let step = backoff.uplink();
            assert!(!step.lowered);
            assert_eq!(step.data_rate, 1);
        }
    }

    #[test]
    fn a_device_at_its_slowest_rate_does_not_ask() {
        // There would be nothing to do about the answer, so the bit is not set.
        let mut backoff = Backoff::new(0, 0, LIMIT, DELAY);

        let step = after(&mut backoff, LIMIT * 4);
        assert!(!step.request_ack);
        assert_eq!(step.data_rate, 0);
    }

    #[test]
    fn asking_stops_once_the_last_step_reaches_the_floor() {
        let mut backoff = Backoff::new(1, 0, LIMIT, DELAY);

        assert!(
            after(&mut backoff, LIMIT).request_ack,
            "asking on the way down"
        );

        let step = after(&mut backoff, DELAY);
        assert!(step.lowered);
        assert_eq!(step.data_rate, 0);
        assert!(
            !step.request_ack,
            "and the uplink that lands on the floor stops asking"
        );
    }

    #[test]
    fn a_delay_of_zero_is_taken_as_one_rather_than_dividing_by_it() {
        let mut backoff = Backoff::new(5, 0, 2, 0);

        assert!(!backoff.uplink().lowered);
        assert!(backoff.uplink().request_ack);
        assert!(
            backoff.uplink().lowered,
            "a step every uplink after the limit"
        );
    }

    #[test]
    fn a_rate_the_network_chose_is_taken_and_counted_from() {
        let mut backoff = Backoff::new(0, 0, LIMIT, DELAY);
        backoff.set_data_rate(5);
        backoff.downlink();

        assert_eq!(backoff.data_rate(), 5);
        assert!(
            after(&mut backoff, LIMIT).request_ack,
            "and it can ask again"
        );
    }
}
