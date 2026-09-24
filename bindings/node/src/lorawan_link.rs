//! Generated Node bindings for what keeps a LoRaWAN link running.
//!
//! The specification revision a device follows, the timings and counts the regional
//! parameters recommend, the back-off that hands back what adaptive data rate tuned away
//! once the network falls silent, and the channel list a join accept can carry.

use crate::checked::{self, OptionalWhole};
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_lorawan::adr::{Backoff, Standing};
use pamoja_lorawan::{defaults, CfList, CfListKind, Version, CFLIST_LEN};

/// How long after an uplink the first receive window opens, in microseconds.
#[napi]
pub const LORAWAN_RECEIVE_DELAY1_US: u32 = defaults::RECEIVE_DELAY1_US;

/// How long after an uplink the second receive window opens, in microseconds.
#[napi]
pub const LORAWAN_RECEIVE_DELAY2_US: u32 = defaults::RECEIVE_DELAY2_US;

/// How long after a join request the first join accept window opens, in microseconds.
#[napi]
pub const LORAWAN_JOIN_ACCEPT_DELAY1_US: u32 = defaults::JOIN_ACCEPT_DELAY1_US;

/// How long after a join request the second join accept window opens, in microseconds.
#[napi]
pub const LORAWAN_JOIN_ACCEPT_DELAY2_US: u32 = defaults::JOIN_ACCEPT_DELAY2_US;

/// How far a receive window may open either side of its time, in microseconds.
#[napi]
pub const LORAWAN_RECEIVE_WINDOW_TOLERANCE_US: u32 = defaults::RECEIVE_WINDOW_TOLERANCE_US;

/// The largest gap a frame counter may jump across and still be accepted.
#[napi]
pub const LORAWAN_MAX_FCNT_GAP: u32 = defaults::MAX_FCNT_GAP;

/// How many unanswered uplinks before a device asks the network to answer.
#[napi]
pub const LORAWAN_ADR_ACK_LIMIT: u32 = defaults::ADR_ACK_LIMIT;

/// How many more before a device starts giving back what adaptive data rate took.
#[napi]
pub const LORAWAN_ADR_ACK_DELAY: u32 = defaults::ADR_ACK_DELAY;

/// The shortest wait before a confirmed uplink is sent again, in microseconds.
#[napi]
pub const LORAWAN_RETRANSMIT_TIMEOUT_MIN_US: u32 = defaults::RETRANSMIT_TIMEOUT_MIN_US;

/// The longest wait before a confirmed uplink is sent again, in microseconds.
#[napi]
pub const LORAWAN_RETRANSMIT_TIMEOUT_MAX_US: u32 = defaults::RETRANSMIT_TIMEOUT_MAX_US;

/// A revision of the LoRaWAN link layer.
#[napi(string_enum)]
pub enum LorawanVersion {
    /// LoRaWAN 1.0.3.
    V1_0_3,
    /// TS001-1.0.4, the LoRaWAN 1.0.4 link layer.
    V1_0_4,
}

impl LorawanVersion {
    /// The Rust revision this names.
    pub(crate) fn core(self) -> Version {
        match self {
            Self::V1_0_3 => Version::V1_0_3,
            Self::V1_0_4 => Version::V1_0_4,
        }
    }
}

/// What a back-off says to do with one uplink.
#[napi(object)]
pub struct LorawanBackoffStep {
    /// Set the ADRACKReq bit, asking the network to answer.
    pub request_ack: bool,
    /// Go back to the default transmit power before sending. Only TS001-1.0.4 takes
    /// this step.
    pub restore_power: bool,
    /// Step the data rate down by the region's back-off table before sending.
    pub lower_data_rate: bool,
    /// Re-enable the default channels and set the repetition count back to one before
    /// sending. Only TS001-1.0.4 takes this step.
    pub restore_channels: bool,
}

/// A device's count of how long the network has been silent.
///
/// A network running adaptive data rate moves a device to the fastest rate and lowest
/// power that still reach it. Once uplinks go unanswered, this says when to ask the
/// network to answer and which of those settings to give back, a step at a time, as
/// LoRaWAN 1.0.3 or TS001-1.0.4 describes.
#[napi]
pub struct LorawanBackoff {
    inner: Backoff,
}

#[napi]
impl LorawanBackoff {
    /// Starts a count from zero.
    ///
    /// `limit` and `delay` default to the 64 and 32 RP002-1.0.5 recommends for every
    /// region. A delay of zero is taken as one.
    #[napi(constructor)]
    pub fn new(
        version: LorawanVersion,
        limit: Option<checked::u32>,
        delay: Option<checked::u32>,
    ) -> Self {
        Self {
            inner: Backoff::new(
                version.core(),
                limit.get().unwrap_or(defaults::ADR_ACK_LIMIT),
                delay.get().unwrap_or(defaults::ADR_ACK_DELAY),
            ),
        }
    }

    /// Counts one new uplink, and says what to do before sending it.
    ///
    /// Call it once per uplink the frame counter moves for, passing whether the device
    /// is already at its default data rate. A repeat of the same uplink does not count.
    #[napi]
    pub fn uplink(&mut self, at_default_data_rate: bool) -> LorawanBackoffStep {
        let step = self.inner.uplink(Standing {
            default_data_rate: at_default_data_rate,
        });
        LorawanBackoffStep {
            request_ack: step.request_ack,
            restore_power: step.restore_power,
            lower_data_rate: step.lower_data_rate,
            restore_channels: step.restore_channels,
        }
    }

    /// Counts a Class A downlink, which proves the network still hears the device and
    /// resets the count.
    #[napi]
    pub fn downlink(&mut self) {
        self.inner.downlink();
    }

    /// How many uplinks have gone unanswered.
    #[napi(getter)]
    pub fn counter(&self) -> u32 {
        self.inner.counter()
    }

    /// The revision whose steps this count follows.
    #[napi(getter)]
    pub fn version(&self) -> LorawanVersion {
        match self.inner.version() {
            Version::V1_0_3 => LorawanVersion::V1_0_3,
            Version::V1_0_4 => LorawanVersion::V1_0_4,
        }
    }
}

/// Which form a channel list takes, from its last byte.
#[napi(string_enum)]
pub enum LorawanCfListKind {
    /// Type 0: a list of frequencies.
    Frequencies,
    /// Type 1: groups of channel mask bits.
    ChannelMasks,
    /// A type the regional parameters reserve, which a device ignores.
    Reserved,
}

/// The optional channel list at the end of a join accept.
///
/// It keeps the sixteen bytes as they arrived and reads either form out of them, so
/// nothing a network sent is lost to a type this build does not recognize.
#[napi]
pub struct LorawanCfList {
    inner: CfList,
}

impl LorawanCfList {
    /// Wraps a Rust channel list.
    pub(crate) fn from_core(inner: CfList) -> Self {
        Self { inner }
    }
}

#[napi]
impl LorawanCfList {
    /// Builds a type 0 list from five frequencies in hertz, with `0` for a slot left
    /// unused.
    ///
    /// Throws if there are not five, or a frequency is not a whole number of hundreds
    /// of hertz from 100 MHz to just under 1.678 GHz.
    #[napi(factory)]
    pub fn from_frequencies(frequencies_hz: Vec<checked::u32>) -> napi::Result<Self> {
        let frequencies_hz = checked::all(frequencies_hz);
        let slots = <[u32; 5]>::try_from(frequencies_hz.as_slice()).map_err(|_| {
            napi::Error::from_reason(format!(
                "a channel list takes 5 frequencies, not {}",
                frequencies_hz.len()
            ))
        })?;
        CfList::frequencies(slots)
            .map(Self::from_core)
            .map_err(|_| {
                napi::Error::from_reason(
                    "a channel list frequency is a whole number of hundreds of hertz from 100 MHz to just under 1.678 GHz"
                        .to_owned(),
                )
            })
    }

    /// Builds a type 1 list from six mask groups, where bit n of group g enables channel
    /// g * 16 + n.
    #[napi(factory)]
    pub fn from_channel_masks(masks: Vec<checked::u32>) -> napi::Result<Self> {
        let masks = checked::all(masks);
        if masks.len() != 6 {
            return Err(napi::Error::from_reason(format!(
                "a channel list takes 6 mask groups, not {}",
                masks.len()
            )));
        }
        let mut groups = [0u16; 6];
        for (slot, mask) in groups.iter_mut().zip(&masks) {
            *slot = u16::try_from(*mask).map_err(|_| {
                napi::Error::from_reason(format!("{mask} does not fit a sixteen-bit mask"))
            })?;
        }
        Ok(Self::from_core(CfList::channel_masks(groups)))
    }

    /// Keeps a channel list exactly as it arrived, whatever its type byte says.
    #[napi(factory)]
    pub fn from_bytes(bytes: Buffer) -> napi::Result<Self> {
        let bytes = <[u8; CFLIST_LEN]>::try_from(bytes.as_ref()).map_err(|_| {
            napi::Error::from_reason(format!(
                "a channel list is exactly {CFLIST_LEN} bytes, not {}",
                bytes.len()
            ))
        })?;
        Ok(Self::from_core(CfList::from_bytes(bytes)))
    }

    /// The sixteen bytes, as a join accept carries them.
    #[napi(getter)]
    pub fn bytes(&self) -> Buffer {
        self.inner.to_bytes().to_vec().into()
    }

    /// Which form the list takes.
    #[napi(getter)]
    pub fn kind(&self) -> LorawanCfListKind {
        match self.inner.kind() {
            CfListKind::Frequencies => LorawanCfListKind::Frequencies,
            CfListKind::ChannelMasks => LorawanCfListKind::ChannelMasks,
            CfListKind::Reserved(_) => LorawanCfListKind::Reserved,
        }
    }

    /// The CFListType byte the list ends with.
    #[napi(getter)]
    pub fn type_byte(&self) -> u8 {
        self.inner.kind().to_byte()
    }

    /// The five frequencies of a type 0 list in hertz, `0` for an unused slot, or
    /// `null` for a list of any other type.
    #[napi]
    pub fn frequencies_hz(&self) -> Option<Vec<u32>> {
        self.inner.frequencies_hz().map(|slots| slots.to_vec())
    }

    /// The six mask groups of a type 1 list, or `null` for a list of any other type.
    #[napi]
    pub fn channel_mask_groups(&self) -> Option<Vec<u32>> {
        self.inner
            .channel_mask_groups()
            .map(|groups| groups.iter().map(|group| u32::from(*group)).collect())
    }

    /// Whether a type 1 list enables a channel, or `null` for a list of any other type
    /// or a channel past the 96 the groups cover.
    #[napi]
    pub fn enables(&self, channel: checked::u32) -> Option<bool> {
        self.inner.enables(u8::try_from(channel.get()).ok()?)
    }

    /// The channels a type 1 list enables, lowest first, which is empty for a list of
    /// any other type.
    #[napi]
    pub fn enabled_channels(&self) -> Vec<u32> {
        self.inner.enabled_channels().map(u32::from).collect()
    }
}
