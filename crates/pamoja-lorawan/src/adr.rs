//! Keeping a device on the air when the network stops answering.
//!
//! A network that runs adaptive data rate moves a device to the fastest rate and lowest power
//! that still reach it, which costs the device less battery and the network less air time.
//! The risk is the other direction: a device moved up and then left there, in a spot the
//! network can no longer hear, would go on transmitting into nothing forever.
//!
//! So a device counts. Every new uplink raises a counter, any Class A downlink at all resets
//! it, and once the counter passes a limit the device starts asking the network to say
//! something. Past that it gives back what the network had tuned away, a step at a time,
//! until either the network answers or the device is back where it started.
//!
//! The two revisions of the specification step differently, and [`Backoff`] follows the one
//! it is given.
//!
//! - **LoRaWAN 1.0.3**, section 4.3.1.1: after the limit plus a delay the device lowers its
//!   data rate, and again every delay after that. The request bit is not set at the slowest
//!   rate, since nothing could be done about the answer.
//! - **TS001-1.0.4**, section 4.3.1.1 and its table 9: the first step restores the default
//!   transmit power, each step after that lowers the data rate, and the step after reaching
//!   the default data rate re-enables the default channels and sets the repetition count
//!   back to one. The request bit is set throughout, and stops once that last step is done.
//!
//! This is the device half, and it is the half the specification settles. Which rate to move
//! a device to in the first place is a policy the specification deliberately leaves open,
//! and it is not here.
//!
//! The limit and the delay belong to the regional parameters rather than the link layer,
//! which names them without values. RP002-1.0.5 section 3.3 recommends 64 and 32 for every
//! region, and [`Backoff::recommended`] starts with those; [`Backoff::new`] takes others, for
//! a device commissioned with different ones.

use crate::defaults::{ADR_ACK_DELAY, ADR_ACK_LIMIT};
use crate::Version;

/// Where a device stands against its defaults when it sends an uplink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    /// Whether it is already at its default data rate, the slowest it uses, so there is no
    /// lower rate to step to.
    pub default_data_rate: bool,
}

/// What the back-off says to do with one uplink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Step {
    /// Whether to set the ADR acknowledgment request bit, asking the network to answer.
    pub request_ack: bool,
    /// Whether to go back to the default transmit power before sending this uplink.
    ///
    /// Only TS001-1.0.4 takes this step, the first after the limit plus a delay.
    pub restore_power: bool,
    /// Whether to step the data rate down, by the region's back-off table, before sending.
    pub lower_data_rate: bool,
    /// Whether to re-enable the default channels and set the repetition count back to one
    /// before sending.
    ///
    /// Only TS001-1.0.4 takes this step, once the default data rate has been reached. A
    /// dynamic channel plan re-enables its default channels, and a fixed plan all of them.
    pub restore_channels: bool,
}

/// A device's count of how long the network has been silent.
///
/// # Examples
///
/// A 1.0.4 device that stops hearing its network, starting two rates above its default:
///
/// ```
/// use pamoja_lorawan::adr::{Backoff, Standing};
/// use pamoja_lorawan::Version;
///
/// let mut backoff = Backoff::recommended(Version::V1_0_4);
/// let mut data_rate = 2;
/// let mut sent = 0;
/// let mut steps = Vec::new();
///
/// while sent < 200 {
///     let step = backoff.uplink(Standing { default_data_rate: data_rate == 0 });
///     sent += 1;
///     if step.restore_power {
///         steps.push((sent, "power"));
///     }
///     if step.lower_data_rate {
///         data_rate -= 1;
///         steps.push((sent, "rate"));
///     }
///     if step.restore_channels {
///         steps.push((sent, "channels"));
///     }
/// }
///
/// // Power a delay after the limit, then a rate every delay, then the channels.
/// assert_eq!(steps, [(96, "power"), (128, "rate"), (160, "rate"), (192, "channels")]);
/// assert!(!backoff.uplink(Standing { default_data_rate: true }).request_ack);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Backoff {
    version: Version,
    counter: u32,
    limit: u32,
    delay: u32,
    restored: bool,
}

impl Backoff {
    /// A count from zero, with the limit and delay RP002-1.0.5 section 3.3 recommends for
    /// every region.
    ///
    /// # Arguments
    ///
    /// * `version` - the revision whose steps to follow.
    ///
    /// # Returns
    ///
    /// The count, asking after [`ADR_ACK_LIMIT`] unanswered uplinks and stepping every
    /// [`ADR_ACK_DELAY`] after that.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::adr::Backoff;
    /// use pamoja_lorawan::Version;
    ///
    /// assert_eq!(Backoff::recommended(Version::V1_0_4), Backoff::new(Version::V1_0_4, 64, 32));
    /// ```
    #[must_use]
    pub const fn recommended(version: Version) -> Backoff {
        Backoff::new(version, ADR_ACK_LIMIT, ADR_ACK_DELAY)
    }

    /// A count from zero, with a limit and delay a device was commissioned with.
    ///
    /// # Arguments
    ///
    /// * `version` - the revision whose steps to follow.
    /// * `limit` - how many unanswered uplinks before it starts asking.
    /// * `delay` - how many more before its first step, and between each step after that.
    ///   Treated as one if given as zero, since a step every zero uplinks is not a thing a
    ///   device can do.
    ///
    /// # Returns
    ///
    /// The count.
    #[must_use]
    pub const fn new(version: Version, limit: u32, delay: u32) -> Backoff {
        Backoff {
            version,
            counter: 0,
            limit,
            delay: if delay == 0 { 1 } else { delay },
            restored: false,
        }
    }

    /// Counts one new uplink, and says what to do before sending it.
    ///
    /// Call this once per uplink the frame counter moves for. A repeated transmission of the
    /// same uplink is not a new one and does not count, which is the same rule the frame
    /// counter itself follows.
    ///
    /// # Arguments
    ///
    /// * `standing` - where the device stands against its defaults right now.
    ///
    /// # Returns
    ///
    /// Whether to ask the network for an answer, and which step, if any, this uplink takes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::adr::{Backoff, Standing};
    /// use pamoja_lorawan::Version;
    ///
    /// let mut backoff = Backoff::recommended(Version::V1_0_3);
    /// let above = Standing { default_data_rate: false };
    /// for _ in 0..63 {
    ///     assert!(!backoff.uplink(above).request_ack);
    /// }
    ///
    /// // The sixty-fourth unanswered uplink is the one that starts asking.
    /// assert!(backoff.uplink(above).request_ack);
    /// ```
    pub fn uplink(&mut self, standing: Standing) -> Step {
        self.counter = self.counter.saturating_add(1);
        if self.counter < self.limit {
            return Step::default();
        }

        let since = self.counter - self.limit;
        let stepping = since >= self.delay && since.is_multiple_of(self.delay);
        match self.version {
            Version::V1_0_3 => Step {
                request_ack: !standing.default_data_rate,
                lower_data_rate: stepping && !standing.default_data_rate,
                ..Step::default()
            },
            Version::V1_0_4 => {
                if self.restored {
                    return Step::default();
                }
                let mut step = Step {
                    request_ack: true,
                    ..Step::default()
                };
                if stepping {
                    if since == self.delay {
                        step.restore_power = true;
                    } else if !standing.default_data_rate {
                        step.lower_data_rate = true;
                    } else {
                        step.restore_channels = true;
                        self.restored = true;
                    }
                }
                step
            }
        }
    }

    /// Counts a Class A downlink, which proves the network still hears the device.
    ///
    /// Any such downlink resets the count, whatever it carried; it does not need to
    /// acknowledge anything.
    pub fn downlink(&mut self) {
        self.counter = 0;
        self.restored = false;
    }

    /// How many uplinks have gone unanswered.
    ///
    /// # Returns
    ///
    /// The counter.
    #[must_use]
    pub const fn counter(&self) -> u32 {
        self.counter
    }

    /// The revision whose steps this count follows.
    ///
    /// # Returns
    ///
    /// The revision.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABOVE: Standing = Standing {
        default_data_rate: false,
    };
    const AT_DEFAULT: Standing = Standing {
        default_data_rate: true,
    };

    // Sends `count` uplinks from the same standing and hands back the last step.
    fn after(backoff: &mut Backoff, count: u32, standing: Standing) -> Step {
        let mut step = backoff.uplink(standing);
        for _ in 1..count {
            step = backoff.uplink(standing);
        }
        step
    }

    #[test]
    fn nothing_is_asked_for_until_the_limit_is_reached() {
        for version in [Version::V1_0_3, Version::V1_0_4] {
            let mut backoff = Backoff::recommended(version);
            assert!(!after(&mut backoff, ADR_ACK_LIMIT - 1, ABOVE).request_ack);
            assert!(
                backoff.uplink(ABOVE).request_ack,
                "{version:?}: the uplink that reaches the limit is the one that asks"
            );
        }
    }

    #[test]
    fn any_downlink_resets_the_count() {
        for version in [Version::V1_0_3, Version::V1_0_4] {
            let mut backoff = Backoff::recommended(version);
            assert!(after(&mut backoff, ADR_ACK_LIMIT, ABOVE).request_ack);

            backoff.downlink();
            assert_eq!(backoff.counter(), 0);
            assert!(!backoff.uplink(ABOVE).request_ack);
        }
    }

    #[test]
    fn version_1_0_4_follows_table_9() {
        // TS001-1.0.4 table 9, ADR_ACK_LIMIT 64 and ADR_ACK_DELAY 32, starting at DR1 with
        // reduced power: counts 0 to 63 normal; 64 to 95 asking; 96 to 127 default power;
        // 128 DR0, the default; 160 channels and NbTrans restored.
        let mut backoff = Backoff::recommended(Version::V1_0_4);

        let step = after(&mut backoff, 63, ABOVE);
        assert_eq!(step, Step::default(), "the first 63 are normal operation");

        for count in 64..=95 {
            let step = backoff.uplink(ABOVE);
            assert!(step.request_ack, "uplink {count} asks");
            assert!(!step.restore_power && !step.lower_data_rate && !step.restore_channels);
        }

        let step = backoff.uplink(ABOVE);
        assert_eq!(
            step,
            Step {
                request_ack: true,
                restore_power: true,
                ..Step::default()
            },
            "uplink 96 restores the power and leaves the rate"
        );

        let step = after(&mut backoff, 32, ABOVE);
        assert_eq!(
            step,
            Step {
                request_ack: true,
                lower_data_rate: true,
                ..Step::default()
            },
            "uplink 128 steps DR1 down to DR0"
        );

        let step = after(&mut backoff, 32, AT_DEFAULT);
        assert_eq!(
            step,
            Step {
                request_ack: true,
                restore_channels: true,
                ..Step::default()
            },
            "uplink 160, at the default rate, restores the channels and NbTrans"
        );

        assert_eq!(
            after(&mut backoff, 100, AT_DEFAULT),
            Step::default(),
            "and with nothing left to give back, it stops asking"
        );
    }

    #[test]
    fn version_1_0_3_steps_the_rate_a_delay_after_the_limit_and_every_delay_after() {
        // LoRaWAN 1.0.3 section 4.3.1.1.
        let mut backoff = Backoff::recommended(Version::V1_0_3);

        let step = after(&mut backoff, ADR_ACK_LIMIT + ADR_ACK_DELAY - 1, ABOVE);
        assert!(step.request_ack);
        assert!(!step.lower_data_rate);

        let step = backoff.uplink(ABOVE);
        assert!(step.lower_data_rate, "the first step comes at 96");
        assert!(!step.restore_power, "and 1.0.3 has no power step");

        let step = after(&mut backoff, ADR_ACK_DELAY, ABOVE);
        assert!(step.lower_data_rate, "and another at 128");
        assert!(!step.restore_channels, "and no channel step either");
    }

    #[test]
    fn version_1_0_3_does_not_ask_at_the_slowest_rate() {
        // "The ADRACKReq SHALL not be set if the device uses its lowest available data rate."
        let mut backoff = Backoff::recommended(Version::V1_0_3);
        let step = after(&mut backoff, ADR_ACK_LIMIT * 4, AT_DEFAULT);
        assert!(!step.request_ack);
        assert!(!step.lower_data_rate);
    }

    #[test]
    fn a_device_already_at_its_default_rate_still_restores_power_then_channels() {
        let mut backoff = Backoff::recommended(Version::V1_0_4);
        let step = after(&mut backoff, ADR_ACK_LIMIT + ADR_ACK_DELAY, AT_DEFAULT);
        assert!(step.restore_power);
        let step = after(&mut backoff, ADR_ACK_DELAY, AT_DEFAULT);
        assert!(step.restore_channels);
        assert!(!step.lower_data_rate);
    }

    #[test]
    fn a_downlink_after_restoring_starts_the_whole_sequence_again() {
        let mut backoff = Backoff::new(Version::V1_0_4, 2, 1);
        let steps: Vec<Step> = (0..4).map(|_| backoff.uplink(AT_DEFAULT)).collect();
        assert!(steps[2].restore_power);
        assert!(steps[3].restore_channels);
        assert_eq!(backoff.uplink(AT_DEFAULT), Step::default());

        backoff.downlink();
        let steps: Vec<Step> = (0..4).map(|_| backoff.uplink(AT_DEFAULT)).collect();
        assert!(steps[1].request_ack);
        assert!(steps[3].restore_channels, "restoring can happen again");
    }

    #[test]
    fn a_delay_of_zero_is_taken_as_one_rather_than_dividing_by_it() {
        let mut backoff = Backoff::new(Version::V1_0_3, 2, 0);

        assert!(!backoff.uplink(ABOVE).lower_data_rate);
        assert!(backoff.uplink(ABOVE).request_ack);
        assert!(
            backoff.uplink(ABOVE).lower_data_rate,
            "a step every uplink after the limit"
        );
    }
}
