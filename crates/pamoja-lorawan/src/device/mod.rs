//! A LoRaWAN Class A end device, without a radio.
//!
//! [`EndDevice`] is what a node runs to take part in a network: it joins, chooses a channel
//! and data rate for each uplink, says when and where to listen for the answer, reads what
//! comes back, and does what the network's MAC commands ask. It owns no radio and no clock.
//! Every step takes the time in microseconds and hands back what to put on the air, so the
//! same logic drives an SX1276 on a microcontroller, an SX1262 on a Linux board, or a test
//! with no hardware at all.
//!
//! One exchange runs like this:
//!
//! 1. [`join`](EndDevice::join) or [`send`](EndDevice::send) returns a [`Transmission`]:
//!    the frame, the carrier, the link, the power, and the two receive windows, each timed
//!    from the end of the transmission.
//! 2. The radio sends it, then listens in the first window and, if nothing for this device
//!    arrived, in the second.
//! 3. A frame heard in either window goes to [`heard`](EndDevice::heard). If neither window
//!    held one, [`nothing_heard`](EndDevice::nothing_heard) says whether to send the same
//!    frame again, with [`repeat`](EndDevice::repeat), or move on.
//!
//! What it follows, and from where:
//!
//! - **Receive windows**: TS001-1.0.4 section 3.3 and RP002-1.0.5 section 3.3, one second and
//!   two after an uplink and five and six after a join request unless the network moves them.
//! - **Channels and data rates**: the region's plan from `pamoja-lora`, a join accept's
//!   channel list, and `LinkADRReq`, `NewChannelReq` and `DlChannelReq`. A fixed plan answers
//!   each channel on the downlink channel it names, and reads `ChMaskCntl` by its own table.
//! - **Joining**: a random join channel with the data rate stepping down across attempts, or
//!   on US902-928 and AU915-928 the passes of RP002-1.0.5 section 3.5.2, eight 125 kHz
//!   channels from successive groups and then a 500 kHz one. On CN470-510 the common join
//!   channel that answered puts the device on its plan, section 3.9.2.
//! - **MAC commands**: every command of TS001-1.0.4 section 5 a device receives, answered in
//!   order, with the four that change how it listens repeated until a downlink arrives.
//! - **Repetition**: NbTrans transmissions of each uplink, stopped by any Class A downlink,
//!   and a retransmission timeout before retrying a confirmed uplink.
//! - **Staying reachable**: the adaptive data rate back-off of [`adr`](crate::adr).
//! - **Sharing the air**: the region's sub-band duty cycles, the network's `DutyCycleReq`,
//!   and the join back-off of TS001-1.0.4 section 7.
//! - **Sleeping**: [`save`](EndDevice::save) and [`resume`](EndDevice::resume) carry a joined
//!   device across a loss of power, so a node that sleeps between readings keeps its session,
//!   its counters and everything its network set.
//!
//! This covers every published plan: the dynamic EU868, EU433, AS923, KR920, IN865 and RU864,
//! and the fixed US915, AU915 and CN470, the last in all four of its RP002-1.0.5 plans and the
//! 96-channel plan before them. KR920, CN470, and AS923 where ARIB STD-T108 applies also
//! require listening before talking, which is the radio's to do.
//!
//! # Examples
//!
//! A device joins a European network the test plays the part of:
//!
//! ```
//! use pamoja_lora::region::Region;
//! use pamoja_lorawan::device::{EndDevice, Heard, Settings};
//! use pamoja_lorawan::{Device, JoinGrant};
//!
//! const APP_KEY: [u8; 16] = [0x2B; 16];
//! let credentials = Device::new([0x11; 8], [0x22; 8], APP_KEY);
//! let mut device = EndDevice::new(Region::Eu868.plan(), credentials, Settings::new(2, 14))?;
//!
//! // The request goes out on a European default channel, and the answer is due five and
//! // six seconds after it.
//! let request = device.join(1, 0)?;
//! assert!([868_100_000, 868_300_000, 868_500_000].contains(&request.frequency_hz));
//! assert_eq!(request.rx1.delay_us, 5_000_000);
//! assert_eq!(request.rx2.frequency_hz, 869_525_000);
//!
//! // The network's accept arrives in the first window.
//! let accept = JoinGrant::new(0x01, 0x13, 0x2601_2E43).accept(&APP_KEY, 1);
//! let heard = device.heard(accept.as_bytes(), 7)?;
//! assert_eq!(heard, Heard::Joined { dev_addr: 0x2601_2E43 });
//!
//! // And the first reading goes out, answered a second after it ends.
//! let reading = device.send(2, b"21.5", false, 7_000_000)?;
//! assert_eq!(reading.rx1.delay_us, 1_000_000);
//! assert!(reading.carries_payload);
//! # Ok::<(), pamoja_lorawan::device::DeviceError>(())
//! ```

mod air;
mod answers;
mod channels;
mod commands;
mod state;

pub use channels::{Channel, MAX_CHANNELS};
pub use state::{Saved, StateError, SAVED_LEN};

use pamoja_lora::region::{ChannelBlock, ChannelPlan, JoinSequence, Modulation, PowerReference};
use pamoja_lora::LinkSettings;

use crate::adr::{Backoff, Standing, Step};
use crate::defaults::{
    JOIN_ACCEPT_DELAY1_US, JOIN_ACCEPT_DELAY2_US, MAX_FCNT_GAP, RECEIVE_DELAY1_US,
    RETRANSMIT_TIMEOUT_MAX_US, RETRANSMIT_TIMEOUT_MIN_US,
};
use crate::mac::FOPTS_MAX;
use crate::{
    Device, FrameHeader, LorawanError, MessageType, PhyPayload, Session, Uplink, Version,
    MAX_PAYLOAD,
};
use air::{Air, Sequence, MAX_SUB_BANDS};
use answers::Answers;
use channels::{Channels, MASK_GROUPS};

/// How long a transmission may hold a channel under a dwell time limit, from RP002-1.0.5
/// section 3.3 and TS001-1.0.4 table 49.
const DWELL_LIMIT_US: u64 = 400_000;

/// What a device's radio can do, and how it takes part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    version: Version,
    adr: bool,
    min_output_dbm: i8,
    max_output_dbm: i8,
    antenna_gain_db: i8,
    lowest_hz: u32,
    highest_hz: u32,
    regional_duty_cycle: bool,
    behind_repeater: bool,
    seed: u32,
}

impl Settings {
    /// Settings for a radio with an output power range.
    ///
    /// The rest start as a typical node: TS001-1.0.4, adaptive data rate on, an antenna
    /// with no gain over its cable, a radio that tunes 137 to 1020 MHz as an SX1276 does,
    /// the region's duty cycle kept, and no repeater in the path.
    ///
    /// # Arguments
    ///
    /// * `min_output_dbm` - the lowest power the radio puts out, conducted.
    /// * `max_output_dbm` - the highest, conducted.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn new(min_output_dbm: i8, max_output_dbm: i8) -> Settings {
        Settings {
            version: Version::V1_0_4,
            adr: true,
            min_output_dbm,
            max_output_dbm,
            antenna_gain_db: 0,
            lowest_hz: 137_000_000,
            highest_hz: 1_020_000_000,
            regional_duty_cycle: true,
            behind_repeater: false,
            seed: 0,
        }
    }

    /// Follows another revision of the link layer.
    ///
    /// # Arguments
    ///
    /// * `version` - the revision the network was told this device follows.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn with_version(mut self, version: Version) -> Settings {
        self.version = version;
        self
    }

    /// Turns adaptive data rate on or off.
    ///
    /// With it on, the device sets the ADR bit, takes the data rate and power the network
    /// chooses, and backs off when the network goes quiet. With it off, the device keeps its
    /// own data rate and TS001-1.0.4 section 5.2 lets it accept a `LinkADRReq` one field at
    /// a time.
    ///
    /// # Arguments
    ///
    /// * `adr` - whether the network manages the data rate.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn with_adr(mut self, adr: bool) -> Settings {
        self.adr = adr;
        self
    }

    /// Accounts for the antenna and its feed.
    ///
    /// Most regions limit radiated power, so a device with a 3 dBi antenna puts out 3 dB less
    /// than one with a 0 dBi antenna to radiate the same. US902-928 limits conducted power
    /// instead, and only the gain above the 6 dBi its limit allows for comes off.
    ///
    /// # Arguments
    ///
    /// * `gain_db` - the antenna gain less the cable and connector losses, in dB.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn with_antenna_gain(mut self, gain_db: i8) -> Settings {
        self.antenna_gain_db = gain_db;
        self
    }

    /// Sets the frequencies the radio and its front end can use.
    ///
    /// A network that asks for a channel or a receive window outside them is told so.
    ///
    /// # Arguments
    ///
    /// * `lowest_hz` - the lowest usable frequency.
    /// * `highest_hz` - the highest.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn with_tuning_range(mut self, lowest_hz: u32, highest_hz: u32) -> Settings {
        self.lowest_hz = lowest_hz;
        self.highest_hz = highest_hz;
        self
    }

    /// Stops holding the device to the region's sub-band duty cycles.
    ///
    /// The regional tables report a limit rather than enforce one, because the operator is
    /// the one who knows whether a regulation applies where the device stands. The network's
    /// `DutyCycleReq` and the join back-off still hold.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn without_regional_duty_cycle(mut self) -> Settings {
        self.regional_duty_cycle = false;
        self
    }

    /// Sizes payloads for a path through a relay, which costs a few bytes of each frame.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn behind_repeater(mut self) -> Settings {
        self.behind_repeater = true;
        self
    }

    /// Seeds the device's random choices of channel and retry delay.
    ///
    /// The device identifier is mixed in, so devices given the same seed still differ.
    ///
    /// # Arguments
    ///
    /// * `seed` - any value, ideally from a hardware random source.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn with_seed(mut self, seed: u32) -> Settings {
        self.seed = seed;
        self
    }

    /// Reports whether the radio can tune a frequency.
    const fn usable(&self, hz: u32) -> bool {
        hz >= 100_000_000 && hz >= self.lowest_hz && hz <= self.highest_hz
    }
}

/// How a device reports its battery when a network asks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Battery {
    /// Running from an external supply.
    External,
    /// A level from 1, empty, to 254, full. Values outside that range are clamped into it.
    Level(u8),
    /// The device cannot measure it.
    #[default]
    Unknown,
}

impl Battery {
    /// The byte `DevStatusAns` carries, from TS001-1.0.4 table 32.
    const fn code(self) -> u8 {
        match self {
            Battery::External => 0,
            Battery::Level(level) => {
                if level == 0 {
                    1
                } else if level == 255 {
                    254
                } else {
                    level
                }
            }
            Battery::Unknown => 255,
        }
    }
}

/// A frame to put on the air, and where to listen afterward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transmission {
    /// The frame.
    pub frame: PhyPayload,
    /// The carrier, in hertz.
    pub frequency_hz: u32,
    /// The data rate, as the region numbers them.
    pub data_rate: u8,
    /// The LoRa settings that data rate stands for: an eight-symbol preamble, an explicit
    /// header and a payload CRC, sent with standard IQ.
    pub link: LinkSettings,
    /// The power to ask of the radio, conducted, with the antenna gain taken off as the
    /// region's power limit requires.
    pub output_dbm: i8,
    /// How long the frame holds the air, in microseconds.
    pub airtime_us: u64,
    /// The first receive window.
    pub rx1: Window,
    /// The second, which opens only if nothing for this device arrived in the first.
    pub rx2: Window,
    /// Whether the application payload went out in this frame.
    ///
    /// It is held back when the answers a device owes the network do not leave room for
    /// it, since TS001-1.0.4 table 15 sends answers first. Send it again afterward.
    pub carries_payload: bool,
}

/// When and where to listen for a downlink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    /// How long after the end of the transmission the window opens, in microseconds.
    pub delay_us: u32,
    /// The carrier, in hertz.
    pub frequency_hz: u32,
    /// The downlink data rate, as the region numbers them.
    pub data_rate: u8,
    /// The LoRa settings to listen with: no payload CRC, and inverted IQ, as RP002-1.0.5
    /// table 112 has for a downlink.
    pub link: LinkSettings,
}

/// What a frame heard in a receive window turned out to be.
// The delivery holds its payload inline, since the crate runs without an allocator.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heard {
    /// A join accept: the device is on the network.
    Joined {
        /// The address the network gave it.
        dev_addr: u32,
    },
    /// A data frame for this device.
    Data(Delivery),
}

/// A downlink, read and acted on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delivery {
    port: Option<u8>,
    payload: [u8; MAX_PAYLOAD],
    len: usize,
    acknowledged: bool,
    confirmed: bool,
    more_pending: bool,
    link_check: Option<LinkCheck>,
    device_time: Option<DeviceTime>,
}

impl Delivery {
    /// Returns the application port the payload arrived on.
    ///
    /// # Returns
    ///
    /// The port, or `None` for a frame that carried only MAC commands or nothing.
    pub fn port(&self) -> Option<u8> {
        self.port
    }

    /// Returns the application payload, decrypted.
    ///
    /// # Returns
    ///
    /// The bytes, empty when the frame carried none for the application.
    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.len]
    }

    /// Reports whether the network acknowledged the confirmed uplink this answered.
    ///
    /// # Returns
    ///
    /// `true` for an acknowledgment of a confirmed uplink.
    pub fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    /// Reports whether the network asked for this downlink to be acknowledged.
    ///
    /// The device sets the acknowledgment on its next uplink by itself. A device with
    /// nothing to send soon can send an empty frame with
    /// [`send_empty`](EndDevice::send_empty).
    ///
    /// # Returns
    ///
    /// `true` for a confirmed downlink.
    pub fn confirmed(&self) -> bool {
        self.confirmed
    }

    /// Reports whether the network has more waiting.
    ///
    /// TS001-1.0.4 section 4.3.1.4 lets the device send an uplink as soon as it can to
    /// collect it.
    ///
    /// # Returns
    ///
    /// The FPending bit.
    pub fn more_pending(&self) -> bool {
        self.more_pending
    }

    /// Returns the answer to a link check the device asked for.
    ///
    /// # Returns
    ///
    /// The margin and gateway count, when the downlink carried them.
    pub fn link_check(&self) -> Option<LinkCheck> {
        self.link_check
    }

    /// Returns the answer to a time request the device asked for.
    ///
    /// # Returns
    ///
    /// The network's time, when the downlink carried it.
    pub fn device_time(&self) -> Option<DeviceTime> {
        self.device_time
    }
}

/// How well the network heard a link check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LinkCheck {
    /// How far above the demodulation floor the best gateway heard it, in dB.
    pub margin_db: u8,
    /// How many gateways heard it.
    pub gateways: u8,
}

/// The time a network gave a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeviceTime {
    /// Whole seconds since the GPS epoch, 1980-01-06 00:00:00 UTC, at the end of the uplink
    /// that asked.
    pub gps_seconds: u32,
    /// The fraction of a second, in 256ths.
    pub fraction: u8,
}

/// What to do once both receive windows have closed with nothing for the device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Next {
    /// Send the same frame again with [`repeat`](EndDevice::repeat), no sooner than this.
    Repeat {
        /// The earliest time to send it, in the caller's microseconds.
        not_before_us: u64,
    },
    /// The uplink is finished.
    Done,
    /// A confirmed uplink went out every time it may without an acknowledgment.
    Unacknowledged,
    /// The join got no answer; join again with a new nonce, no sooner than this.
    JoinAgain {
        /// The earliest time to try, in the caller's microseconds.
        not_before_us: u64,
    },
}

/// Why a device could not do what it was asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceError {
    /// The plan defines more channels than a device keeps.
    TooManyChannels {
        /// The most a device keeps, [`MAX_CHANNELS`].
        max: usize,
    },
    /// A device activated by personalization has nothing to join with.
    NoCredentials,
    /// The device has not joined.
    NotJoined,
    /// A transmission is still waiting on its receive windows.
    Busy,
    /// There is no transmission waiting on its windows or due to repeat.
    NothingPending,
    /// The air is not free until this time, in the caller's microseconds.
    Wait {
        /// The earliest time to try again.
        until_us: u64,
    },
    /// No enabled channel carries the data rate, and restoring the defaults did not help.
    NoChannel,
    /// The data rate is not a LoRa one this device can send or hear.
    DataRate(u8),
    /// The payload does not fit a frame at the current data rate.
    PayloadTooLong {
        /// The most the frame can carry, in bytes.
        max: usize,
    },
    /// The uplink frame counter has reached its end, and the device has to join again.
    CounterExhausted,
    /// The frame did not decode.
    Frame(LorawanError),
    /// The frame is addressed to another device.
    Foreign,
    /// The frame repeats or precedes the last downlink the device accepted.
    Replayed,
    /// The frame is further ahead of the last downlink than LoRaWAN 1.0.3 lets a device
    /// follow.
    CounterGap,
    /// A join accept carries settings the region does not allow, which RP002-1.0.5 has a
    /// device ignore.
    Refused,
    /// A saved state could not be resumed.
    State(StateError),
}

impl core::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DeviceError::TooManyChannels { max } => {
                write!(
                    f,
                    "the plan defines more than the {max} channels a device keeps"
                )
            }
            DeviceError::NoCredentials => f.write_str("the device has no keys to join with"),
            DeviceError::NotJoined => f.write_str("the device has not joined a network"),
            DeviceError::Busy => f.write_str("a transmission is still waiting on its windows"),
            DeviceError::NothingPending => f.write_str("nothing is waiting on its windows"),
            DeviceError::Wait { until_us } => write!(f, "the air is not free until {until_us}"),
            DeviceError::NoChannel => f.write_str("no enabled channel carries the data rate"),
            DeviceError::DataRate(rate) => write!(f, "DR{rate} is not a LoRa data rate here"),
            DeviceError::PayloadTooLong { max } => {
                write!(f, "the payload does not fit; {max} bytes do")
            }
            DeviceError::CounterExhausted => f.write_str("the uplink frame counter is spent"),
            DeviceError::Frame(error) => write!(f, "the frame did not decode: {error}"),
            DeviceError::Foreign => f.write_str("the frame is for another device"),
            DeviceError::Replayed => f.write_str("the frame repeats an earlier downlink"),
            DeviceError::CounterGap => f.write_str("the frame counter jumped too far ahead"),
            DeviceError::Refused => f.write_str("the join accept carries settings not allowed"),
            DeviceError::State(error) => write!(f, "the saved state was not resumed: {error}"),
        }
    }
}

impl core::error::Error for DeviceError {}

impl From<LorawanError> for DeviceError {
    fn from(error: LorawanError) -> DeviceError {
        DeviceError::Frame(error)
    }
}

/// A transmission that has gone out and is waiting on its windows, or is due to repeat.
// The frame is kept inline to repeat it byte for byte, with no allocator to box it.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Join {
        dev_nonce: u16,
        data_rate: u8,
        join_channel: u16,
    },
    Uplink {
        frame: PhyPayload,
        data_rate: u8,
        confirmed: bool,
        carries_payload: bool,
        transmissions_left: u8,
        windows_closed: bool,
        not_before_us: u64,
    },
}

/// A LoRaWAN Class A end device.
///
/// See the [module documentation](self) for the exchange it runs.
pub struct EndDevice<'p> {
    plan: &'p ChannelPlan<'p>,
    credentials: Option<Device>,
    settings: Settings,
    sequence: Sequence,
    battery: Battery,
    session: Option<Session>,
    joined_on: Option<u16>,
    fcnt_up: u32,
    fcnt_down: Option<u32>,
    data_rate: u8,
    tx_power: u8,
    nb_trans: u8,
    rx1_dr_offset: u8,
    rx2_frequency_hz: u32,
    rx2_data_rate: u8,
    rx1_delay_us: u32,
    max_eirp_dbm: i8,
    uplink_dwell: bool,
    downlink_dwell: bool,
    max_duty_cycle: u8,
    channels: Channels,
    answers: Answers,
    backoff: Backoff,
    air: Air,
    pending: Option<Pending>,
    ack_owed: bool,
    joins: u32,
    join_used: [u16; MASK_GROUPS],
    join_slot: u32,
    quiet_until_us: u64,
}

/// A join channel chosen for one attempt, and how the attempt goes out on it.
#[derive(Clone, Copy, Debug)]
struct JoinChoice {
    number: u16,
    channel: Channel,
    data_rate: u8,
    link: LinkSettings,
    airtime_us: u64,
    next_slot: u32,
    restart_cycle: bool,
}

impl<'p> EndDevice<'p> {
    /// A device that joins over the air.
    ///
    /// # Arguments
    ///
    /// * `plan` - the region's channel plan.
    /// * `credentials` - the device's identifiers and root key.
    /// * `settings` - what its radio can do.
    ///
    /// # Returns
    ///
    /// The device, not yet joined.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::TooManyChannels`] for a plan with more default or join channels
    /// than [`MAX_CHANNELS`].
    pub fn new(
        plan: &'p ChannelPlan<'p>,
        credentials: Device,
        settings: Settings,
    ) -> Result<EndDevice<'p>, DeviceError> {
        let sequence = Sequence::new(settings.seed, &credentials.dev_eui());
        EndDevice::build(plan, Some(credentials), sequence, settings)
    }

    /// A device activated by personalization, with its session provisioned.
    ///
    /// Such a device never resets its frame counters, TS001-1.0.4 section 4.3.1.5, so a
    /// device that loses power carries them over with
    /// [`with_frame_counters`](EndDevice::with_frame_counters).
    ///
    /// # Arguments
    ///
    /// * `plan` - the region's channel plan.
    /// * `session` - the address and session keys it was provisioned with.
    /// * `settings` - what its radio can do.
    ///
    /// # Returns
    ///
    /// The device, ready to send at its slowest data rate.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::TooManyChannels`] for a plan with more default or join channels
    /// than [`MAX_CHANNELS`].
    pub fn personalized(
        plan: &'p ChannelPlan<'p>,
        session: Session,
        settings: Settings,
    ) -> Result<EndDevice<'p>, DeviceError> {
        let mut eui = [0u8; 8];
        eui[..4].copy_from_slice(&session.dev_addr().to_le_bytes());
        let sequence = Sequence::new(settings.seed, &eui);
        let mut device = EndDevice::build(plan, None, sequence, settings)?;
        device.session = Some(session);
        device.data_rate = device.lowest_data_rate();
        Ok(device)
    }

    fn build(
        plan: &'p ChannelPlan<'p>,
        credentials: Option<Device>,
        sequence: Sequence,
        settings: Settings,
    ) -> Result<EndDevice<'p>, DeviceError> {
        let join_channels = if plan.join_plans.is_empty() {
            plan.join_channels
                .iter()
                .map(|block| usize::from(block.count))
                .sum::<usize>()
        } else {
            plan.join_plans
                .iter()
                .map(|run| usize::from(run.channels.count))
                .sum::<usize>()
        };
        if usize::from(plan.default_channel_count()) > MAX_CHANNELS || join_channels > MAX_CHANNELS
        {
            return Err(DeviceError::TooManyChannels { max: MAX_CHANNELS });
        }
        let mut device = EndDevice {
            plan,
            credentials,
            settings,
            sequence,
            battery: Battery::Unknown,
            session: None,
            joined_on: None,
            fcnt_up: 0,
            fcnt_down: None,
            data_rate: 0,
            tx_power: 0,
            nb_trans: 1,
            rx1_dr_offset: 0,
            rx2_frequency_hz: plan.rx2_frequency_hz,
            rx2_data_rate: plan.rx2_data_rate,
            rx1_delay_us: RECEIVE_DELAY1_US,
            max_eirp_dbm: plan.default_max_eirp_dbm,
            uplink_dwell: false,
            downlink_dwell: false,
            max_duty_cycle: 0,
            channels: Channels::defaults(plan),
            answers: Answers::new(),
            backoff: Backoff::recommended(settings.version),
            air: Air::new(),
            pending: None,
            ack_owed: false,
            joins: 0,
            join_used: [0; MASK_GROUPS],
            join_slot: 0,
            quiet_until_us: 0,
        };
        device.reset_mac();
        Ok(device)
    }

    /// Carries frame counters over from before a restart.
    ///
    /// # Arguments
    ///
    /// * `up` - the next uplink counter to use.
    /// * `down` - the last downlink counter accepted, if any was.
    ///
    /// # Returns
    ///
    /// The device.
    pub fn with_frame_counters(mut self, up: u32, down: Option<u32>) -> EndDevice<'p> {
        self.fcnt_up = up;
        self.fcnt_down = down;
        self
    }

    /// Sets what the device reports its battery as, when a network asks.
    ///
    /// # Arguments
    ///
    /// * `battery` - the level.
    pub fn set_battery(&mut self, battery: Battery) {
        self.battery = battery;
    }

    /// Asks the network, with the next uplink, how well it hears the device.
    ///
    /// The answer arrives in a [`Delivery`]'s [`link_check`](Delivery::link_check).
    pub fn request_link_check(&mut self) {
        self.answers.request_link_check();
    }

    /// Asks the network, with the next uplink, for the time.
    ///
    /// The answer arrives in a [`Delivery`]'s [`device_time`](Delivery::device_time).
    pub fn request_device_time(&mut self) {
        self.answers.request_device_time();
    }

    /// Reports whether the device is on a network.
    ///
    /// # Returns
    ///
    /// `true` once joined, or from the start for a personalized device.
    pub fn is_joined(&self) -> bool {
        self.session.is_some()
    }

    /// Returns the address the device is on the network by.
    ///
    /// # Returns
    ///
    /// The address, or `None` before joining.
    pub fn dev_addr(&self) -> Option<u32> {
        self.session.map(|session| session.dev_addr())
    }

    /// Returns the data rate the next uplink goes out at, before any back-off step.
    ///
    /// # Returns
    ///
    /// The data rate.
    pub fn data_rate(&self) -> u8 {
        self.data_rate
    }

    /// Returns the next uplink frame counter.
    ///
    /// # Returns
    ///
    /// The counter a device stores to carry over a restart.
    pub fn fcnt_up(&self) -> u32 {
        self.fcnt_up
    }

    /// Returns the last downlink frame counter accepted.
    ///
    /// # Returns
    ///
    /// The counter, or `None` before any downlink.
    pub fn fcnt_down(&self) -> Option<u32> {
        self.fcnt_down
    }

    /// Returns how many times each uplink goes out, as the network last set it.
    ///
    /// # Returns
    ///
    /// NbTrans, from 1 to 15.
    pub fn transmissions(&self) -> u8 {
        self.nb_trans
    }

    /// Returns the channels the device may send on.
    ///
    /// # Returns
    ///
    /// Each enabled channel with its index.
    pub fn channels(&self) -> impl Iterator<Item = (usize, Channel)> + '_ {
        self.channels.enabled()
    }

    /// Returns the lowest and highest frequency the device transmits or listens on.
    ///
    /// A radio that calibrates for a band, as an SX126x does, calibrates for this one, so
    /// moving between the device's channels and windows needs no calibration in between.
    ///
    /// # Returns
    ///
    /// The lower and upper frequency in hertz, across every enabled channel's uplink and
    /// downlink, the join channels and where their accepts arrive, and the second receive
    /// window.
    pub fn frequency_span(&self) -> (u32, u32) {
        let mut low = self.rx2_frequency_hz.min(self.plan.rx2_frequency_hz);
        let mut high = self.rx2_frequency_hz.max(self.plan.rx2_frequency_hz);
        let mut take = |hz: u32| {
            low = low.min(hz);
            high = high.max(hz);
        };
        for (_, channel) in self.channels.enabled() {
            take(channel.uplink_hz);
            take(channel.downlink_hz);
        }
        let mut number = 0u16;
        let mut index = 0;
        while let Some(block) = self.join_block(index) {
            for offset in 0..block.count {
                if let Some(hz) = block.frequency_hz(offset) {
                    take(hz);
                    take(self.join_downlink_hz(number, hz));
                }
                number = number.saturating_add(1);
            }
            index += 1;
        }
        (low, high)
    }

    /// Returns the second receive window's frequency and data rate.
    ///
    /// # Returns
    ///
    /// The frequency in hertz and the data rate.
    pub fn rx2(&self) -> (u32, u8) {
        (self.rx2_frequency_hz, self.rx2_data_rate)
    }

    /// Returns the delay from the end of an uplink to the first receive window.
    ///
    /// # Returns
    ///
    /// The delay in microseconds.
    pub fn receive_delay_us(&self) -> u32 {
        self.rx1_delay_us
    }

    /// Builds a join request.
    ///
    /// Each attempt picks a join channel at random and a data rate from the fastest the join
    /// channels carry down to the slowest, one step per attempt and round again, which is
    /// how TS001-1.0.4 section 6.2.5 asks a device to cover every channel and rate. On a plan
    /// that joins in octet passes, US902-928 and AU915-928, each attempt instead takes the
    /// next slot of the pass at the rate its channel carries. The air it takes is held to the
    /// region's duty cycle and to the join back-off of section 7.
    ///
    /// The accept is due on the uplink's frequency in a dynamic plan, on the downlink channel
    /// a fixed plan answers the join channel on, and on CN470-510 on the frequency table 49
    /// gives the common join channel, in both windows, as Semtech's LoRaMac-node listens.
    ///
    /// # Arguments
    ///
    /// * `dev_nonce` - a nonce this device has never used with its join identifier. TS001-1.0.4
    ///   counts up from zero and keeps the count across restarts; LoRaWAN 1.0.3 draws one at
    ///   random.
    /// * `now_us` - the time, in microseconds.
    ///
    /// # Returns
    ///
    /// What to transmit, with the join accept windows.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NoCredentials`] for a personalized device,
    /// [`DeviceError::Busy`] while a transmission waits on its windows, and
    /// [`DeviceError::Wait`] when the air is not free.
    pub fn join(&mut self, dev_nonce: u16, now_us: u64) -> Result<Transmission, DeviceError> {
        let credentials = self
            .credentials
            .as_ref()
            .ok_or(DeviceError::NoCredentials)?;
        if self.pending.is_some() {
            return Err(DeviceError::Busy);
        }
        if now_us < self.quiet_until_us {
            return Err(DeviceError::Wait {
                until_us: self.quiet_until_us,
            });
        }

        let frame = credentials.join_request(dev_nonce);
        let length = frame.as_bytes().len();
        let choice = match self.plan.join_sequence {
            JoinSequence::Random => self.random_join_channel(length, now_us)?,
            JoinSequence::OctetPasses => self.octet_pass_channel(length, now_us)?,
        };
        let rx2_hz = match self.plan.join_plan(choice.number) {
            Some(_) => choice.channel.downlink_hz,
            None => self.plan.rx2_frequency_hz,
        };
        let transmission = self.transmission(
            frame,
            choice.channel,
            choice.data_rate,
            choice.link,
            choice.airtime_us,
            self.tx_power,
            Some(rx2_hz),
            true,
        )?;

        self.air.joined_air(choice.airtime_us);
        self.record_air(now_us, choice.airtime_us, choice.channel.uplink_hz);
        self.joins = self.joins.wrapping_add(1);
        self.join_slot = choice.next_slot;
        if choice.restart_cycle {
            self.join_used = [0; MASK_GROUPS];
        }
        let number = usize::from(choice.number);
        if number < MAX_CHANNELS {
            self.join_used[number / 16] |= 1 << (number % 16);
        }
        self.pending = Some(Pending::Join {
            dev_nonce,
            data_rate: choice.data_rate,
            join_channel: choice.number,
        });
        Ok(transmission)
    }

    /// A join channel at random carrying this attempt's data rate.
    fn random_join_channel(
        &mut self,
        length: usize,
        now_us: u64,
    ) -> Result<JoinChoice, DeviceError> {
        let data_rate = self.join_data_rate()?;
        let link = self.uplink_link(data_rate)?;
        let airtime_us = link.airtime_us(length);
        if let Err(until_us) = self.air.join_allowed(now_us, airtime_us) {
            return Err(DeviceError::Wait { until_us });
        }

        let mut candidates = [None; MAX_CHANNELS];
        let mut count = 0;
        let mut number = 0u16;
        let mut index = 0;
        while let Some(block) = self.join_block(index) {
            for offset in 0..block.count {
                if let Some(hz) = block.frequency_hz(offset) {
                    if block.min_data_rate <= data_rate
                        && data_rate <= block.max_data_rate
                        && self.settings.usable(hz)
                        && count < MAX_CHANNELS
                    {
                        candidates[count] = Some((number, self.join_channel(number, hz, &block)));
                        count += 1;
                    }
                }
                number = number.saturating_add(1);
            }
            index += 1;
        }
        let (number, channel) = self.pick(&candidates[..count], now_us)?;
        Ok(JoinChoice {
            number,
            channel,
            data_rate,
            link,
            airtime_us,
            next_slot: self.join_slot,
            restart_cycle: false,
        })
    }

    /// The next slot of an octet pass over the join channels, RP002-1.0.5 section 3.5.2.
    ///
    /// The first join block is split into groups of eight, visited in order, and every block
    /// after it forms one more slot at the end of the pass. A slot takes a channel no attempt
    /// of the current cycle has used, at random within a group and the lowest in the last
    /// slot, as the section's example has the 500 kHz channels go 64, 65 and on. The cycle
    /// starts over once every channel has gone out, and a slot with no channel the radio can
    /// use is passed over.
    fn octet_pass_channel(
        &mut self,
        length: usize,
        now_us: u64,
    ) -> Result<JoinChoice, DeviceError> {
        let blocks = self.plan.join_channels;
        let narrow = blocks.first().map_or(0, |block| block.count);
        let groups = u32::from(narrow.div_ceil(8));
        let total = blocks.iter().map(|block| block.count).sum::<u16>();
        let pass = groups + u32::from(total > narrow);
        if pass == 0 {
            return Err(DeviceError::NoChannel);
        }

        let mut every = [(0u16, Channel::new(0, 0, 0)); MAX_CHANNELS];
        let mut count = 0;
        let mut number = 0u16;
        for block in blocks {
            for offset in 0..block.count {
                if let Some(hz) = block.frequency_hz(offset) {
                    if self.settings.usable(hz)
                        && self.uplink_link(block.min_data_rate).is_ok()
                        && self.application_room(block.min_data_rate).is_ok()
                        && count < MAX_CHANNELS
                    {
                        every[count] = (number, self.join_channel(number, hz, block));
                        count += 1;
                    }
                }
                number = number.saturating_add(1);
            }
        }
        let every = &every[..count];
        let used = |number: u16| {
            let number = usize::from(number);
            self.join_used[number / 16] & (1 << (number % 16)) != 0
        };
        let fresh_cycle = every.iter().all(|(number, _)| used(*number));

        for step in 0..pass {
            let slot = (self.join_slot + step) % pass;
            let in_slot = |number: u16| {
                if slot < groups {
                    u32::from(number) / 8 == slot && number < narrow
                } else {
                    number >= narrow
                }
            };
            let mut candidates = [None; 8];
            let mut found = 0;
            for &(number, channel) in every {
                if in_slot(number) && (fresh_cycle || !used(number)) && found < candidates.len() {
                    candidates[found] = Some((number, channel));
                    found += 1;
                    if slot >= groups {
                        break;
                    }
                }
            }
            let Some((_, first)) = candidates[0] else {
                continue;
            };
            let data_rate = first.min_data_rate;
            let link = self.uplink_link(data_rate)?;
            let airtime_us = link.airtime_us(length);
            if let Err(until_us) = self.air.join_allowed(now_us, airtime_us) {
                return Err(DeviceError::Wait { until_us });
            }
            let (number, channel) = self.pick(&candidates[..found], now_us)?;
            return Ok(JoinChoice {
                number,
                channel,
                data_rate,
                link,
                airtime_us,
                next_slot: (slot + 1) % pass,
                restart_cycle: fresh_cycle,
            });
        }
        Err(DeviceError::NoChannel)
    }

    /// One block of the channels a join request may go out on, numbered through in order: the
    /// join plans' channels where the plan picks a plan by the join channel, and otherwise its
    /// join channels.
    fn join_block(&self, index: usize) -> Option<ChannelBlock> {
        if self.plan.join_plans.is_empty() {
            self.plan.join_channels.get(index).copied()
        } else {
            self.plan.join_plans.get(index).map(|run| run.channels)
        }
    }

    /// A join channel with the downlink its accept arrives on.
    fn join_channel(&self, number: u16, hz: u32, block: &ChannelBlock) -> Channel {
        let mut channel = Channel::new(hz, block.min_data_rate, block.max_data_rate);
        channel.downlink_hz = self.join_downlink_hz(number, hz);
        channel
    }

    /// Where the accept for a join on a numbered join channel arrives.
    fn join_downlink_hz(&self, number: u16, hz: u32) -> u32 {
        match self.plan.join_plan(number) {
            Some((run, offset)) => run.accept_hz(offset).unwrap_or(hz),
            None => self.plan.rx1_frequency_hz(number, hz).unwrap_or(hz),
        }
    }

    /// Builds an uplink carrying a payload.
    ///
    /// The frame takes every answer the device owes the network. If they leave no room for
    /// the payload, the answers go alone and [`Transmission::carries_payload`] says the
    /// payload waits.
    ///
    /// # Arguments
    ///
    /// * `port` - the application port, 1 to 223, or 224 for the certification test port.
    /// * `payload` - the application payload.
    /// * `confirmed` - whether to ask the network to acknowledge it.
    /// * `now_us` - the time, in microseconds.
    ///
    /// # Returns
    ///
    /// What to transmit, with its receive windows.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NotJoined`] before joining, [`DeviceError::Busy`] while a
    /// transmission waits on its windows or its repeats, [`DeviceError::Wait`] when the air
    /// is not free, [`DeviceError::PayloadTooLong`] for a payload the data rate cannot carry,
    /// and [`DeviceError::Frame`] for port 0, which belongs to MAC commands.
    pub fn send(
        &mut self,
        port: u8,
        payload: &[u8],
        confirmed: bool,
        now_us: u64,
    ) -> Result<Transmission, DeviceError> {
        if port == 0 {
            return Err(DeviceError::Frame(LorawanError::MalformedFrame));
        }
        self.uplink(Some((port, payload)), confirmed, now_us)
    }

    /// Builds an uplink with no payload.
    ///
    /// It carries the answers the device owes, an acknowledgment of a confirmed downlink,
    /// or an ADR acknowledgment request, whichever are due.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the time, in microseconds.
    ///
    /// # Returns
    ///
    /// What to transmit, with its receive windows.
    ///
    /// # Errors
    ///
    /// As [`send`](EndDevice::send).
    pub fn send_empty(&mut self, now_us: u64) -> Result<Transmission, DeviceError> {
        self.uplink(None, false, now_us)
    }

    /// Reads a frame heard in one of the receive windows of the last transmission.
    ///
    /// A frame that is not for this device, or does not verify, leaves the transmission
    /// waiting, so the second window still opens.
    ///
    /// # Arguments
    ///
    /// * `frame` - the bytes the radio received.
    /// * `snr_db` - the frame's signal-to-noise ratio, which a `DevStatusAns` reports.
    ///
    /// # Returns
    ///
    /// The join, or the downlink read and acted on.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NothingPending`] with no transmission waiting,
    /// [`DeviceError::Foreign`] for another device's frame, [`DeviceError::Replayed`] and
    /// [`DeviceError::CounterGap`] for a counter the device will not accept,
    /// [`DeviceError::Refused`] for a join accept with settings the region forbids, and
    /// [`DeviceError::Frame`] for a frame that does not decode.
    pub fn heard(&mut self, frame: &[u8], snr_db: i8) -> Result<Heard, DeviceError> {
        match self.pending {
            None => Err(DeviceError::NothingPending),
            Some(Pending::Join {
                dev_nonce,
                data_rate,
                join_channel,
            }) => self.heard_join(frame, dev_nonce, data_rate, join_channel),
            Some(Pending::Uplink {
                windows_closed: true,
                ..
            }) => Err(DeviceError::NothingPending),
            Some(Pending::Uplink { confirmed, .. }) => self.heard_data(frame, snr_db, confirmed),
        }
    }

    /// Says what comes next once both receive windows closed with nothing for the device.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the time the second window closed, in microseconds.
    ///
    /// # Returns
    ///
    /// Whether to repeat the frame, join again, or move on.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NothingPending`] with no transmission waiting on its windows.
    pub fn nothing_heard(&mut self, now_us: u64) -> Result<Next, DeviceError> {
        match self.pending {
            None
            | Some(Pending::Uplink {
                windows_closed: true,
                ..
            }) => Err(DeviceError::NothingPending),
            Some(Pending::Join { .. }) => {
                self.pending = None;
                let wait = self
                    .sequence
                    .between(RETRANSMIT_TIMEOUT_MIN_US, RETRANSMIT_TIMEOUT_MAX_US);
                let not_before_us = now_us.saturating_add(u64::from(wait));
                self.quiet_until_us = not_before_us;
                Ok(Next::JoinAgain { not_before_us })
            }
            Some(Pending::Uplink {
                frame,
                data_rate,
                confirmed,
                carries_payload,
                transmissions_left,
                ..
            }) => {
                let not_before_us = if confirmed {
                    let wait = self
                        .sequence
                        .between(RETRANSMIT_TIMEOUT_MIN_US, RETRANSMIT_TIMEOUT_MAX_US);
                    now_us.saturating_add(u64::from(wait))
                } else {
                    now_us
                };
                if transmissions_left > 0 {
                    self.pending = Some(Pending::Uplink {
                        frame,
                        data_rate,
                        confirmed,
                        carries_payload,
                        transmissions_left,
                        windows_closed: true,
                        not_before_us,
                    });
                    return Ok(Next::Repeat { not_before_us });
                }
                self.pending = None;
                if confirmed {
                    self.quiet_until_us = not_before_us;
                    Ok(Next::Unacknowledged)
                } else {
                    Ok(Next::Done)
                }
            }
        }
    }

    /// Sends the last uplink again, the same frame on a channel chosen afresh.
    ///
    /// TS001-1.0.4 section 4.3.1.3 has a device hop between repetitions as usual and keep
    /// the frame counter, so the frame is byte for byte the one that went out before.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the time, in microseconds.
    ///
    /// # Returns
    ///
    /// What to transmit, with its receive windows.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NothingPending`] with nothing due to repeat, and
    /// [`DeviceError::Wait`] before the retransmission timeout or while the air is not free.
    pub fn repeat(&mut self, now_us: u64) -> Result<Transmission, DeviceError> {
        let Some(Pending::Uplink {
            frame,
            data_rate,
            confirmed,
            carries_payload,
            transmissions_left,
            windows_closed: true,
            not_before_us,
        }) = self.pending
        else {
            return Err(DeviceError::NothingPending);
        };
        if now_us < not_before_us {
            return Err(DeviceError::Wait {
                until_us: not_before_us,
            });
        }
        let link = self.uplink_link(data_rate)?;
        let airtime_us = link.airtime_us(frame.as_bytes().len());
        let channels = self.channels;
        let channel = self.pick_enabled(&channels, data_rate, now_us)?;
        let transmission = self.transmission(
            frame,
            channel,
            data_rate,
            link,
            airtime_us,
            self.tx_power,
            None,
            carries_payload,
        )?;
        self.record_air(now_us, airtime_us, channel.uplink_hz);
        self.pending = Some(Pending::Uplink {
            frame,
            data_rate,
            confirmed,
            carries_payload,
            transmissions_left: transmissions_left - 1,
            windows_closed: false,
            not_before_us: 0,
        });
        Ok(transmission)
    }

    fn uplink(
        &mut self,
        data: Option<(u8, &[u8])>,
        confirmed: bool,
        now_us: u64,
    ) -> Result<Transmission, DeviceError> {
        let session = self.session.ok_or(DeviceError::NotJoined)?;
        if self.pending.is_some() {
            return Err(DeviceError::Busy);
        }
        if now_us < self.quiet_until_us {
            return Err(DeviceError::Wait {
                until_us: self.quiet_until_us,
            });
        }
        if self.fcnt_up == u32::MAX {
            return Err(DeviceError::CounterExhausted);
        }

        // Everything the back-off changes is worked out on copies and kept only if the
        // frame goes out, so a refusal leaves the count where it was.
        let mut data_rate = self.data_rate;
        let mut tx_power = self.tx_power;
        let mut nb_trans = self.nb_trans;
        let mut channels = self.channels;
        let mut backoff = self.backoff;
        let lowest = self.lowest_data_rate();

        let step = if self.settings.adr {
            backoff.uplink(Standing {
                default_data_rate: data_rate <= lowest,
            })
        } else {
            Step::default()
        };
        if step.restore_power {
            tx_power = 0;
        }
        if step.lower_data_rate {
            data_rate = self.lower(data_rate, lowest);
        }
        if step.restore_channels {
            channels.enable_defaults();
            nb_trans = 1;
        }
        // TS001-1.0.4 section 4.3.1.1: a combination that leaves no usable channel sends
        // the device straight back to its default channels at full power.
        if !channels.carries(&channels.mask(), data_rate) {
            channels.enable_defaults();
            tx_power = 0;
            if !channels.carries(&channels.mask(), data_rate) {
                data_rate = lowest;
            }
        }

        let room = self.application_room(data_rate)?;
        if let Some((_, payload)) = data {
            if payload.len() > room {
                return Err(DeviceError::PayloadTooLong { max: room });
            }
        }

        let owed = self.answers.len();
        let payload_len = data.map_or(0, |(_, payload)| payload.len());
        let mut fopts = [0u8; FOPTS_MAX];
        let mut mac = [0u8; MAX_PAYLOAD];
        let (port, body, fopts_len, carries_payload, link_check, device_time) =
            if owed <= FOPTS_MAX && owed + payload_len <= room {
                let (len, link_check, device_time) = self.answers.write(&mut fopts);
                (
                    data.map(|(port, _)| port),
                    data.map_or(&[][..], |(_, payload)| payload),
                    len,
                    data.is_some(),
                    link_check,
                    device_time,
                )
            } else if owed <= FOPTS_MAX {
                let (len, link_check, device_time) = self.answers.write(&mut fopts);
                (None, &[][..], len, false, link_check, device_time)
            } else {
                let (len, link_check, device_time) = self.answers.write(&mut mac[..room]);
                (Some(0), &mac[..len], 0, false, link_check, device_time)
            };

        let mut uplink = match port {
            Some(port) => Uplink::new(self.fcnt_up, port, body),
            None => Uplink::empty(self.fcnt_up),
        }
        .with_fopts(&fopts[..fopts_len]);
        let confirmed = confirmed && carries_payload;
        if confirmed {
            uplink = uplink.confirmed();
        }
        if self.settings.adr {
            uplink = uplink.with_adr();
        }
        if step.request_ack {
            uplink = uplink.with_adr_ack_req();
        }
        if self.ack_owed {
            uplink = uplink.with_ack();
        }
        let frame = session.encode_uplink(&uplink)?;

        let link = self.uplink_link(data_rate)?;
        let airtime_us = link.airtime_us(frame.as_bytes().len());
        if self.uplink_dwell && airtime_us > DWELL_LIMIT_US {
            return Err(DeviceError::PayloadTooLong { max: room });
        }
        let channel = self.pick_enabled(&channels, data_rate, now_us)?;
        let transmission = self.transmission(
            frame,
            channel,
            data_rate,
            link,
            airtime_us,
            tx_power,
            None,
            carries_payload,
        )?;

        self.data_rate = data_rate;
        self.tx_power = tx_power;
        self.nb_trans = nb_trans;
        self.channels = channels;
        self.backoff = backoff;
        self.fcnt_up += 1;
        self.answers.sent(link_check, device_time);
        self.ack_owed = false;
        self.record_air(now_us, airtime_us, channel.uplink_hz);
        self.pending = Some(Pending::Uplink {
            frame,
            data_rate,
            confirmed,
            carries_payload,
            transmissions_left: nb_trans.saturating_sub(1),
            windows_closed: false,
            not_before_us: 0,
        });
        Ok(transmission)
    }

    fn heard_join(
        &mut self,
        frame: &[u8],
        dev_nonce: u16,
        data_rate: u8,
        join_channel: u16,
    ) -> Result<Heard, DeviceError> {
        let header = FrameHeader::parse(frame)?;
        if header.message_type() != MessageType::JoinAccept {
            return Err(DeviceError::Frame(LorawanError::UnsupportedMType(
                frame[0] & 0xE0,
            )));
        }
        let credentials = self
            .credentials
            .as_ref()
            .ok_or(DeviceError::NoCredentials)?;
        let accept = credentials.accept_join(frame, dev_nonce)?;

        // RP002-1.0.5 section 3.9.2: the common join channel decides a CN470-510 plan.
        let joined_on = self.plan.join_plan(join_channel);
        let plan = joined_on.map_or(self.plan, |(run, _)| run.plan);

        // RP002-1.0.5 section 3.4.7 and its counterparts: a reserved RX1DROffset in a join
        // accept has the accept ignored.
        let rx2_is_lora = matches!(
            plan.downlink_data_rate(accept.rx2_data_rate())
                .map(|rate| rate.modulation),
            Some(Modulation::LoRa { .. })
        );
        if accept.rx1_dr_offset() > plan.max_rx1_data_rate_offset || !rx2_is_lora {
            return Err(DeviceError::Refused);
        }

        // TS001-1.0.4 section 6.2.6: back to the default channels and MAC settings, then the
        // accept's own settings, then the channel list as if its commands had arrived, with
        // no answers.
        self.plan = plan;
        self.reset_mac();
        if let Some(hz) = joined_on.and_then(|(run, offset)| run.rx2_hz(offset)) {
            self.rx2_frequency_hz = hz;
        }
        self.rx1_dr_offset = accept.rx1_dr_offset();
        self.rx2_data_rate = accept.rx2_data_rate();
        self.rx1_delay_us = accept.receive_delay_us();
        if let Some(list) = accept.cflist() {
            let settings = self.settings;
            self.channels
                .apply_cflist(self.plan, list, |hz| settings.usable(hz));
        }
        let session = accept.session();
        self.session = Some(session);
        self.joined_on = joined_on.map(|_| join_channel);
        self.fcnt_up = 0;
        self.fcnt_down = None;
        self.data_rate = if self.channels.carries(&self.channels.mask(), data_rate) {
            data_rate
        } else {
            self.lowest_data_rate()
        };
        self.air.join_done();
        self.join_used = [0; MASK_GROUPS];
        self.join_slot = 0;
        self.pending = None;
        self.quiet_until_us = 0;
        Ok(Heard::Joined {
            dev_addr: session.dev_addr(),
        })
    }

    fn heard_data(
        &mut self,
        frame: &[u8],
        snr_db: i8,
        confirmed_uplink: bool,
    ) -> Result<Heard, DeviceError> {
        let session = self.session.ok_or(DeviceError::NotJoined)?;
        let header = FrameHeader::parse(frame)?;
        if !matches!(
            header.message_type(),
            MessageType::UnconfirmedDown | MessageType::ConfirmedDown
        ) {
            return Err(DeviceError::Frame(LorawanError::UnsupportedMType(
                frame[0] & 0xE0,
            )));
        }
        if header.dev_addr() != Some(session.dev_addr()) {
            return Err(DeviceError::Foreign);
        }
        let fcnt = self.downlink_counter(header.fcnt().unwrap_or(0))?;
        let rx = session.decode(frame, fcnt)?;

        self.fcnt_down = Some(fcnt);
        self.backoff.downlink();
        self.answers.heard_downlink();
        self.ack_owed = rx.confirmed();
        self.pending = None;
        self.quiet_until_us = 0;

        let mut delivery = Delivery {
            port: None,
            payload: [0; MAX_PAYLOAD],
            len: 0,
            acknowledged: confirmed_uplink && rx.ack(),
            confirmed: rx.confirmed(),
            more_pending: rx.fpending(),
            link_check: None,
            device_time: None,
        };
        match rx.fport() {
            Some(0) => self.process_commands(rx.payload(), snr_db, &mut delivery),
            Some(port) => {
                self.process_commands(rx.fopts(), snr_db, &mut delivery);
                delivery.port = Some(port);
                delivery.len = rx.payload().len();
                delivery.payload[..delivery.len].copy_from_slice(rx.payload());
            }
            None => self.process_commands(rx.fopts(), snr_db, &mut delivery),
        }
        Ok(Heard::Data(delivery))
    }

    /// Works the full downlink counter out from the 16 bits a frame carries.
    ///
    /// TS001-1.0.4 section 4.3.1.5 has the counter only go up and a retransmission of the
    /// last downlink ignored. LoRaWAN 1.0.3 also refuses a jump of [`MAX_FCNT_GAP`] or more.
    fn downlink_counter(&self, low: u16) -> Result<u32, DeviceError> {
        let Some(last) = self.fcnt_down else {
            return Ok(u32::from(low));
        };
        if last as u16 == low {
            return Err(DeviceError::Replayed);
        }
        let mut candidate = (last & !0xFFFF) | u32::from(low);
        if candidate < last {
            candidate = candidate
                .checked_add(0x1_0000)
                .ok_or(DeviceError::CounterExhausted)?;
        }
        if self.settings.version == Version::V1_0_3 && candidate - last >= MAX_FCNT_GAP {
            return Err(DeviceError::CounterGap);
        }
        Ok(candidate)
    }

    /// Puts every MAC setting back to the region's default, as a join accept does.
    fn reset_mac(&mut self) {
        self.channels = Channels::defaults(self.plan);
        self.tx_power = 0;
        self.nb_trans = 1;
        self.rx1_dr_offset = 0;
        self.rx2_frequency_hz = self.plan.rx2_frequency_hz;
        self.rx2_data_rate = self.plan.rx2_data_rate;
        self.rx1_delay_us = RECEIVE_DELAY1_US;
        self.max_eirp_dbm = self.plan.default_max_eirp_dbm;
        // RP002-1.0.5 has AS923 and AU915 devices assume the 400 ms limit until
        // `TXParamSetupReq` says otherwise, and no region assume a downlink limit.
        self.uplink_dwell = self.plan.tx_param_setup;
        self.downlink_dwell = false;
        self.max_duty_cycle = 0;
        self.answers = Answers::new();
        self.backoff = Backoff::recommended(self.settings.version);
        self.ack_owed = false;
    }

    /// The default data rate: the slowest LoRa rate a default channel carries, and under a
    /// dwell limit the slowest that carries a payload inside it.
    fn lowest_data_rate(&self) -> u8 {
        (0..16u8)
            .find(|rate| {
                self.channels.carries(&self.channels.mask(), *rate)
                    && self.uplink_link(*rate).is_ok()
                    && self.application_room(*rate).is_ok()
            })
            .unwrap_or(0)
    }

    /// The next data rate down by the region's back-off table, never below `lowest`.
    fn lower(&self, data_rate: u8, lowest: u8) -> u8 {
        let mut next = data_rate;
        while let Some(lower) = self.plan.next_backoff_data_rate(next) {
            next = lower;
            if next <= lowest {
                return lowest;
            }
            if self.uplink_link(next).is_ok() && self.application_room(next).is_ok() {
                return next;
            }
        }
        data_rate
    }

    /// The data rate this join attempt goes out at.
    fn join_data_rate(&self) -> Result<u8, DeviceError> {
        let mut rates = [0u8; 16];
        let mut count = 0;
        for rate in (0..16u8).rev() {
            let carried = (0..)
                .map_while(|index| self.join_block(index))
                .any(|block| block.min_data_rate <= rate && rate <= block.max_data_rate);
            if carried && self.uplink_link(rate).is_ok() && self.application_room(rate).is_ok() {
                rates[count] = rate;
                count += 1;
            }
        }
        if count == 0 {
            return Err(DeviceError::NoChannel);
        }
        Ok(rates[self.joins as usize % count])
    }

    /// The most application payload a data rate carries with no frame options, from the
    /// region's payload tables.
    fn application_room(&self, data_rate: u8) -> Result<usize, DeviceError> {
        let limit = if self.uplink_dwell && self.plan.has_dwell_time_limit {
            self.plan.max_payload_dwell_limited(data_rate)
        } else {
            self.plan
                .max_payload(data_rate, self.settings.behind_repeater)
        };
        limit
            .map(|limit| usize::from(limit.application).min(MAX_PAYLOAD))
            .ok_or(DeviceError::DataRate(data_rate))
    }

    /// The LoRa settings of an uplink at a data rate.
    fn uplink_link(&self, data_rate: u8) -> Result<LinkSettings, DeviceError> {
        match self
            .plan
            .uplink_data_rate(data_rate)
            .map(|rate| rate.modulation)
        {
            Some(Modulation::LoRa {
                spreading_factor,
                bandwidth_hz,
            }) => Ok(LinkSettings::new(spreading_factor, bandwidth_hz)),
            _ => Err(DeviceError::DataRate(data_rate)),
        }
    }

    /// The LoRa settings a receive window listens with.
    fn downlink_link(&self, data_rate: u8) -> Result<LinkSettings, DeviceError> {
        match self
            .plan
            .downlink_data_rate(data_rate)
            .map(|rate| rate.modulation)
        {
            Some(Modulation::LoRa {
                spreading_factor,
                bandwidth_hz,
            }) => Ok(LinkSettings::new(spreading_factor, bandwidth_hz).without_crc()),
            _ => Err(DeviceError::DataRate(data_rate)),
        }
    }

    /// Picks an enabled channel carrying a data rate whose air is free.
    fn pick_enabled(
        &mut self,
        channels: &Channels,
        data_rate: u8,
        now_us: u64,
    ) -> Result<Channel, DeviceError> {
        let mut candidates = [None; MAX_CHANNELS];
        let mut count = 0;
        for (index, channel) in channels.enabled() {
            if channel.carries(data_rate) && self.settings.usable(channel.uplink_hz) {
                candidates[count] = Some((index as u16, channel));
                count += 1;
            }
        }
        self.pick(&candidates[..count], now_us)
            .map(|(_, channel)| channel)
    }

    /// Picks one of `candidates`, numbered channels, at random among those whose air is free
    /// now.
    fn pick(
        &mut self,
        candidates: &[Option<(u16, Channel)>],
        now_us: u64,
    ) -> Result<(u16, Channel), DeviceError> {
        let mut free = 0u32;
        let mut earliest = u64::MAX;
        for (_, channel) in candidates.iter().flatten() {
            let at = self.free_at(channel.uplink_hz);
            if at <= now_us {
                free += 1;
            } else {
                earliest = earliest.min(at);
            }
        }
        if free == 0 {
            return Err(if earliest == u64::MAX {
                DeviceError::NoChannel
            } else {
                DeviceError::Wait { until_us: earliest }
            });
        }
        let mut choice = self.sequence.below(free);
        for (number, channel) in candidates.iter().flatten() {
            if self.free_at(channel.uplink_hz) <= now_us {
                if choice == 0 {
                    return Ok((*number, *channel));
                }
                choice -= 1;
            }
        }
        Err(DeviceError::NoChannel)
    }

    /// When a frequency's air is next free.
    fn free_at(&self, hz: u32) -> u64 {
        let sub_band = if self.settings.regional_duty_cycle {
            self.air
                .sub_band_free_at(self.sub_band(hz).map(|(index, _)| index))
        } else {
            0
        };
        sub_band.max(self.air.aggregated_free_at())
    }

    /// The sub-band a frequency falls in, with its duty cycle.
    fn sub_band(&self, hz: u32) -> Option<(usize, u32)> {
        self.plan
            .sub_bands
            .iter()
            .enumerate()
            .take(MAX_SUB_BANDS)
            .find(|(_, band)| band.contains(hz))
            .map(|(index, band)| (index, band.duty_cycle_permille))
    }

    fn record_air(&mut self, started_us: u64, airtime_us: u64, hz: u32) {
        let sub_band = if self.settings.regional_duty_cycle {
            self.sub_band(hz)
        } else {
            None
        };
        self.air
            .transmitted(started_us, airtime_us, sub_band, self.max_duty_cycle);
    }

    /// The conducted power for a power index on a frequency.
    fn output_dbm(&self, hz: u32, tx_power: u8) -> i8 {
        let ceiling = self.plan.max_eirp_dbm(hz).min(self.max_eirp_dbm);
        let power = self
            .plan
            .tx_power_dbm(tx_power, self.max_eirp_dbm)
            .unwrap_or(ceiling)
            .min(ceiling);
        self.radio_dbm(power).clamp(
            i16::from(self.settings.min_output_dbm),
            i16::from(self.settings.max_output_dbm),
        ) as i8
    }

    /// The power to ask of the radio for a power the plan's index names: a radiated power
    /// less the antenna's gain, or a conducted one less whatever gain exceeds the plan's
    /// allowance.
    fn radio_dbm(&self, power: i8) -> i16 {
        let gain = i16::from(self.settings.antenna_gain_db);
        match self.plan.power_reference {
            PowerReference::Eirp => i16::from(power) - gain,
            PowerReference::Conducted { gain_allowance_db } => {
                i16::from(power) - (gain - i16::from(gain_allowance_db)).max(0)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn transmission(
        &self,
        frame: PhyPayload,
        channel: Channel,
        data_rate: u8,
        link: LinkSettings,
        airtime_us: u64,
        tx_power: u8,
        join_rx2_hz: Option<u32>,
        carries_payload: bool,
    ) -> Result<Transmission, DeviceError> {
        let (delay1, delay2, offset, (rx2_hz, rx2_rate)) = match join_rx2_hz {
            Some(hz) => (
                JOIN_ACCEPT_DELAY1_US,
                JOIN_ACCEPT_DELAY2_US,
                0,
                (hz, self.plan.rx2_data_rate),
            ),
            None => (
                self.rx1_delay_us,
                self.rx1_delay_us.saturating_add(1_000_000),
                self.rx1_dr_offset,
                (self.rx2_frequency_hz, self.rx2_data_rate),
            ),
        };
        let rx1_rate = if self.downlink_dwell {
            self.plan.rx1_data_rate_dwell_limited(data_rate, offset)
        } else {
            None
        }
        .or_else(|| self.plan.rx1_data_rate(data_rate, offset))
        .ok_or(DeviceError::DataRate(data_rate))?;

        Ok(Transmission {
            frame,
            frequency_hz: channel.uplink_hz,
            data_rate,
            link,
            output_dbm: self.output_dbm(channel.uplink_hz, tx_power),
            airtime_us,
            rx1: Window {
                delay_us: delay1,
                frequency_hz: channel.downlink_hz,
                data_rate: rx1_rate,
                link: self.downlink_link(rx1_rate)?,
            },
            rx2: Window {
                delay_us: delay2,
                frequency_hz: rx2_hz,
                data_rate: rx2_rate,
                link: self.downlink_link(rx2_rate)?,
            },
            carries_payload,
        })
    }
}

#[cfg(test)]
mod tests;
