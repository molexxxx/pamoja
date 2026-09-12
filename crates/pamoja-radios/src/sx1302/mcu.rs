//! Starting the two microcontrollers inside a concentrator.
//!
//! Giving a microcontroller its firmware is only half of bringing one up. The gain control
//! comes out of reset waiting to be configured, and it is configured through four mailbox
//! registers and a status register, one small group of settings at a time: the host writes
//! the values, writes a code saying which group it just wrote, waits for the status to reach
//! the next number, and reads every value back to see that it took. The arbiter is simpler
//! but works the same way, through its own debug registers.
//!
//! A concentrator whose microcontrollers have been loaded but not started reports a healthy
//! version register, passes every firmware check, and hears nothing useful, because its gain
//! control has never been told what gains to use.
//!
//! Nothing here reads a register. The order and the values are data, so the whole handshake
//! can be checked against the reference with no hardware present.

use super::tx::FrontEnd;

/// The gain control firmware a concentrator with SX1250 front ends expects.
pub const AGC_VERSION_SX1250: u8 = 10;

/// The one a concentrator with the earlier front ends expects.
pub const AGC_VERSION_SX125X: u8 = 6;

/// The arbiter firmware a concentrator expects.
pub const ARB_VERSION: u8 = 2;

/// The gain value that leaves the gain control to decide for itself.
pub const GAIN_AUTOMATIC: u8 = 0xff;

/// How long the amplifier is started before a packet, in hundreds of microseconds.
pub const PA_START_DELAY: u8 = 8;

/// The status a microcontroller reports once its firmware is running.
pub const STATUS_RUNNING: u8 = 0x01;

/// The status the arbiter returns to once it has been told to resume.
pub const ARB_STATUS_RESUMED: u8 = 0x00;

/// How many peaks a second detection has to clear to be believed.
pub const ARB_DETECT_THRESHOLD: u8 = 3;

/// The gains a gain control is configured with.
///
/// Every value comes from the reference, which keeps one set for each kind of front end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gains {
    /// The least analog gain it may use.
    pub analog_min: u8,
    /// The most.
    pub analog_max: u8,
    /// The level it stops reducing analog gain below.
    pub analog_low: u8,
    /// The level it starts reducing above.
    pub analog_high: u8,
    /// The least the decimator may attenuate.
    pub decimator_min: u8,
    /// The most.
    pub decimator_max: u8,
    /// The level the decimator stops attenuating below.
    pub decimator_low: u8,
    /// The first level it attenuates above.
    pub decimator_high: u8,
    /// The second, which it attenuates harder above.
    pub decimator_higher: u8,
    /// The least a channel may be attenuated.
    pub channel_min: u8,
    /// The most.
    pub channel_max: u8,
    /// The level a channel stops being attenuated below.
    pub channel_low: u8,
    /// The level it starts being attenuated above.
    pub channel_high: u8,
    /// Which amplifier an SX1250 uses.
    pub device: u8,
    /// How far its high power amplifier is driven.
    pub high_power_max: u8,
    /// The duty cycle it drives at.
    pub duty_cycle: u8,
}

/// The gains an SX1250 front end is configured with.
pub const SX1250_GAINS: Gains = Gains {
    analog_min: 1,
    analog_max: 13,
    analog_low: 3,
    analog_high: 12,
    decimator_min: 4,
    decimator_max: 15,
    decimator_low: 40,
    decimator_high: 80,
    decimator_higher: 90,
    channel_min: 4,
    channel_max: 14,
    channel_low: 52,
    channel_high: 132,
    device: 0,
    high_power_max: 7,
    duty_cycle: 4,
};

/// The gains an SX1255 or SX1257 front end is configured with.
pub const SX125X_GAINS: Gains = Gains {
    analog_min: 0,
    analog_max: 9,
    analog_low: 16,
    analog_high: 35,
    decimator_min: 7,
    decimator_max: 11,
    decimator_low: 45,
    decimator_high: 100,
    decimator_higher: 115,
    channel_min: 4,
    channel_max: 14,
    channel_low: 52,
    channel_high: 132,
    device: 0,
    high_power_max: 0,
    duty_cycle: 0,
};

/// A step in the exchange that starts a microcontroller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Wait until the status reads this.
    Await(u8),
    /// Read the firmware version and refuse anything but this.
    Version(u8),
    /// Put a value in a mailbox.
    Write(u8, u8),
    /// Tell the microcontroller which group of settings was just written.
    Notify(u8),
    /// Read a mailbox back and refuse anything but this.
    Verify(u8, u8),
}

/// What a gain control is configured with and how it is told.
///
/// The exchange is strictly ordered: every group is written, announced, acknowledged and read
/// back before the next begins, and the status the microcontroller reports rises by one at
/// each acknowledgement. Skipping a read-back would let a setting that never took pass for
/// one that did.
///
/// # Arguments
///
/// * `front_end` - which front end the board carries, which decides both the gain table and
///   the firmware version expected.
/// * `listen_before_talk` - whether the SX1261 beside the concentrator is checking channels
///   before the gateway talks on them.
///
/// # Returns
///
/// The steps, in the order the microcontroller takes them.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::mcu::{start_gain_control, Step};
/// use pamoja_radios::sx1302::tx::FrontEnd;
///
/// let mut order = start_gain_control(FrontEnd::Sx1250, false);
///
/// // Nothing is written until the firmware says it is running and names its version.
/// assert_eq!(order.next(), Some(Step::Await(0x01)));
/// assert_eq!(order.next(), Some(Step::Version(10)));
///
/// // The exchange ends by telling the microcontroller there is nothing more to come.
/// assert_eq!(
///     start_gain_control(FrontEnd::Sx1250, false).last(),
///     Some(Step::Notify(0x0f))
/// );
/// ```
pub fn start_gain_control(
    front_end: FrontEnd,
    listen_before_talk: bool,
) -> impl Iterator<Item = Step> {
    let gains = match front_end {
        FrontEnd::Sx1250 => SX1250_GAINS,
        FrontEnd::Sx125x => SX125X_GAINS,
    };
    let version = match front_end {
        FrontEnd::Sx1250 => AGC_VERSION_SX1250,
        FrontEnd::Sx125x => AGC_VERSION_SX125X,
    };

    // The earlier front ends take a duplex setting in the third mailbox that the SX1250 has
    // no use for, and a radio is configured twice, once for each chain.
    let duplex = matches!(front_end, FrontEnd::Sx125x).then_some(Step::Write(2, 0));
    let radio = move |code: u8, acknowledged: u8| {
        [
            Some(Step::Write(0, GAIN_AUTOMATIC)),
            Some(Step::Write(1, GAIN_AUTOMATIC)),
            duplex,
            Some(Step::Notify(code)),
            Some(Step::Await(acknowledged)),
            Some(Step::Verify(0, GAIN_AUTOMATIC)),
            Some(Step::Verify(1, GAIN_AUTOMATIC)),
        ]
        .into_iter()
        .flatten()
    };

    let amplifier = matches!(front_end, FrontEnd::Sx1250).then(|| {
        [
            Step::Write(0, gains.device),
            Step::Write(1, gains.high_power_max),
            Step::Write(2, gains.duty_cycle),
            Step::Notify(0x09),
            Step::Await(0x0a),
            Step::Verify(0, gains.device),
            Step::Verify(1, gains.high_power_max),
            Step::Verify(2, gains.duty_cycle),
        ]
    });

    [Step::Await(STATUS_RUNNING), Step::Version(version)]
        .into_iter()
        .chain(radio(0x80, 0x02))
        .chain(radio(0x20, 0x03))
        .chain(pair(0x03, 0x04, gains.analog_min, gains.analog_max))
        .chain(pair(0x04, 0x05, gains.analog_low, gains.analog_high))
        .chain(pair(0x05, 0x06, gains.decimator_min, gains.decimator_max))
        .chain([
            Step::Write(0, gains.decimator_low),
            Step::Write(1, gains.decimator_high),
            Step::Write(2, gains.decimator_higher),
            Step::Notify(0x06),
            Step::Await(0x07),
            Step::Verify(0, gains.decimator_low),
            Step::Verify(1, gains.decimator_high),
            Step::Verify(2, gains.decimator_higher),
        ])
        .chain(pair(0x07, 0x08, gains.channel_min, gains.channel_max))
        .chain(pair(0x08, 0x09, gains.channel_low, gains.channel_high))
        .chain(amplifier.into_iter().flatten())
        .chain([
            Step::Write(0, PA_START_DELAY),
            Step::Notify(0x0a),
            Step::Await(0x0b),
            Step::Verify(0, PA_START_DELAY),
            Step::Write(0, u8::from(listen_before_talk)),
            Step::Notify(0x0b),
            Step::Await(0x0f),
            Step::Verify(0, u8::from(listen_before_talk)),
            Step::Notify(0x0f),
        ])
}

/// One group of two settings, written, announced, acknowledged and read back.
fn pair(code: u8, acknowledged: u8, first: u8, second: u8) -> impl Iterator<Item = Step> {
    [
        Step::Write(0, first),
        Step::Write(1, second),
        Step::Notify(code),
        Step::Await(acknowledged),
        Step::Verify(0, first),
        Step::Verify(1, second),
    ]
    .into_iter()
}

/// What the arbiter is told before it is let go.
///
/// The arbiter shares the receivers between the channels. It comes up halted, reports its
/// version, takes a few settings in its own registers, and is then told to resume.
///
/// # Arguments
///
/// * `dual_demodulation` - which spreading factors are demodulated twice over, one bit each
///   counting from SF5. Demodulating twice buys a finer timestamp at the cost of capacity,
///   so a gateway that does not need fine timestamps passes zero.
///
/// # Returns
///
/// The steps, in the order the arbiter takes them.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::mcu::{start_arbiter, Step};
///
/// let mut order = start_arbiter(0x00);
/// assert_eq!(order.next(), Some(Step::Await(0x01)));
/// assert_eq!(order.next(), Some(Step::Version(2)));
///
/// // It ends waiting for the arbiter to report that it is running again.
/// assert_eq!(start_arbiter(0x00).last(), Some(Step::Await(0x00)));
/// ```
pub fn start_arbiter(dual_demodulation: u8) -> impl Iterator<Item = Step> {
    [
        Step::Await(STATUS_RUNNING),
        Step::Version(ARB_VERSION),
        // Which spreading factor the arbiter keeps detection statistics for.
        Step::Write(0, 7),
        Step::Write(3, dual_demodulation),
        Step::Write(2, ARB_DETECT_THRESHOLD),
        Step::Write(1, 1),
        Step::Await(ARB_STATUS_RESUMED),
    ]
    .into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_group_is_acknowledged_and_read_back_before_the_next() {
        // A write that is never verified is a setting that can quietly not take, so each
        // group has to be announced, acknowledged, and then read back, in that order.
        let steps: Vec<Step> = start_gain_control(FrontEnd::Sx1250, false).collect();

        for (at, step) in steps.iter().enumerate() {
            let Step::Notify(code) = step else {
                continue;
            };

            // The last one only says there is nothing more to come.
            if *code == 0x0f {
                assert_eq!(at, steps.len() - 1, "the finish code is not the last step");
                continue;
            }

            let rest = &steps[at + 1..];
            let awaited = rest
                .iter()
                .position(|step| matches!(step, Step::Await(_)))
                .expect("a group that is announced is acknowledged");
            let verified = rest
                .iter()
                .position(|step| matches!(step, Step::Verify(_, _)))
                .expect("a group that is acknowledged is read back");
            assert!(
                awaited < verified,
                "group {code:#04x} is read back before it is acknowledged"
            );

            // And nothing else is written until this group has been read back.
            assert!(
                !rest[..verified]
                    .iter()
                    .any(|step| matches!(step, Step::Write(_, _))),
                "group {code:#04x} is followed by another write before its read back"
            );
        }
    }

    #[test]
    fn the_status_the_exchange_waits_for_only_ever_rises() {
        let steps: Vec<Step> = start_gain_control(FrontEnd::Sx1250, true).collect();
        let mut last = 0;
        for step in steps {
            if let Step::Await(status) = step {
                assert!(status > last, "{status} came after {last}");
                last = status;
            }
        }
        assert_eq!(last, 0x0f);
    }

    #[test]
    fn an_earlier_front_end_is_told_its_duplex_mode_and_takes_no_amplifier_settings() {
        let older: Vec<Step> = start_gain_control(FrontEnd::Sx125x, false).collect();
        let newer: Vec<Step> = start_gain_control(FrontEnd::Sx1250, false).collect();

        // The older front end takes a third mailbox for each radio, twice over.
        assert_eq!(
            older
                .iter()
                .filter(|step| **step == Step::Write(2, 0))
                .count(),
            2
        );

        // And the newer one takes a group of amplifier settings the older has no use for.
        assert!(newer.contains(&Step::Notify(0x09)));
        assert!(!older.contains(&Step::Notify(0x09)));
    }

    #[test]
    fn each_front_end_expects_its_own_firmware() {
        assert!(start_gain_control(FrontEnd::Sx1250, false).any(|step| step == Step::Version(10)));
        assert!(start_gain_control(FrontEnd::Sx125x, false).any(|step| step == Step::Version(6)));
    }

    #[test]
    fn the_gains_are_the_values_the_reference_keeps() {
        // Spot-checked against the reference table rather than restated from the struct.
        assert_eq!(SX1250_GAINS.analog_max, 13);
        assert_eq!(SX1250_GAINS.decimator_higher, 90);
        assert_eq!(SX1250_GAINS.channel_high, 132);
        assert_eq!(SX125X_GAINS.analog_max, 9);
        assert_eq!(SX125X_GAINS.decimator_higher, 115);

        // The channel settings are the only group both front ends share.
        assert_eq!(SX1250_GAINS.channel_min, SX125X_GAINS.channel_min);
        assert_eq!(SX1250_GAINS.channel_high, SX125X_GAINS.channel_high);
    }

    #[test]
    fn listen_before_talk_is_carried_to_the_gain_control() {
        // The flag has a group of its own, so it is found by that group rather than by the
        // value it carries: a one in the first mailbox is also the analog gain minimum.
        for enabled in [false, true] {
            let steps: Vec<Step> = start_gain_control(FrontEnd::Sx1250, enabled).collect();
            let announced = steps
                .iter()
                .position(|step| *step == Step::Notify(0x0b))
                .expect("the flag is announced as its own group");
            let read_back = steps[announced..]
                .iter()
                .find_map(|step| match step {
                    Step::Verify(0, value) => Some(*value),
                    _ => None,
                })
                .expect("and it is read back");

            assert_eq!(read_back, u8::from(enabled));
        }
    }

    #[test]
    fn the_arbiter_is_resumed_last() {
        let steps: Vec<Step> = start_arbiter(0xff).collect();
        assert_eq!(steps.last(), Some(&Step::Await(ARB_STATUS_RESUMED)));

        // The resume is the write before that wait, and the mask reaches it beforehand.
        assert!(steps.contains(&Step::Write(3, 0xff)));
        assert!(steps.contains(&Step::Write(1, 1)));
    }
}
