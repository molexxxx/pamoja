//! Generated Python bindings for what keeps a LoRaWAN link running.
//!
//! The specification revision a device follows, the timings and counts the regional
//! parameters recommend, the back-off that hands back what adaptive data rate tuned away
//! once the network falls silent, and the channel list a join accept can carry.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_lorawan::adr::{Backoff, Standing};
use pamoja_lorawan::{defaults, CfList, CfListKind, Version, CFLIST_LEN};

use crate::PamojaError;

/// The timings and counts RP002-1.0.5 section 3.3 recommends, as the module constants they
/// are published under.
pub const DEFAULTS: [(&str, u32); 10] = [
    ("LORAWAN_RECEIVE_DELAY1_US", defaults::RECEIVE_DELAY1_US),
    ("LORAWAN_RECEIVE_DELAY2_US", defaults::RECEIVE_DELAY2_US),
    (
        "LORAWAN_JOIN_ACCEPT_DELAY1_US",
        defaults::JOIN_ACCEPT_DELAY1_US,
    ),
    (
        "LORAWAN_JOIN_ACCEPT_DELAY2_US",
        defaults::JOIN_ACCEPT_DELAY2_US,
    ),
    (
        "LORAWAN_RECEIVE_WINDOW_TOLERANCE_US",
        defaults::RECEIVE_WINDOW_TOLERANCE_US,
    ),
    ("LORAWAN_MAX_FCNT_GAP", defaults::MAX_FCNT_GAP),
    ("LORAWAN_ADR_ACK_LIMIT", defaults::ADR_ACK_LIMIT),
    ("LORAWAN_ADR_ACK_DELAY", defaults::ADR_ACK_DELAY),
    (
        "LORAWAN_RETRANSMIT_TIMEOUT_MIN_US",
        defaults::RETRANSMIT_TIMEOUT_MIN_US,
    ),
    (
        "LORAWAN_RETRANSMIT_TIMEOUT_MAX_US",
        defaults::RETRANSMIT_TIMEOUT_MAX_US,
    ),
];

/// Reads a revision name, `1.0.3` or `1.0.4`.
pub(crate) fn version(name: &str) -> PyResult<Version> {
    match name {
        "1.0.3" => Ok(Version::V1_0_3),
        "1.0.4" => Ok(Version::V1_0_4),
        other => Err(PamojaError::new_err(format!(
            "{other} is not a LoRaWAN version; expected 1.0.3 or 1.0.4"
        ))),
    }
}

/// What a back-off says to do with one uplink.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanBackoffStep {
    /// Set the ADRACKReq bit, asking the network to answer.
    #[pyo3(get)]
    request_ack: bool,
    /// Go back to the default transmit power before sending. Only TS001-1.0.4 takes this
    /// step.
    #[pyo3(get)]
    restore_power: bool,
    /// Step the data rate down by the region's back-off table before sending.
    #[pyo3(get)]
    lower_data_rate: bool,
    /// Re-enable the default channels and set the repetition count back to one before
    /// sending. Only TS001-1.0.4 takes this step.
    #[pyo3(get)]
    restore_channels: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanBackoffStep {
    fn __repr__(&self) -> String {
        let flag = |on: bool| if on { "True" } else { "False" };
        format!(
            "LorawanBackoffStep(request_ack={}, restore_power={}, lower_data_rate={}, restore_channels={})",
            flag(self.request_ack),
            flag(self.restore_power),
            flag(self.lower_data_rate),
            flag(self.restore_channels)
        )
    }
}

/// A device's count of how long the network has been silent.
///
/// A network running adaptive data rate moves a device to the fastest rate and lowest power
/// that still reach it. Once uplinks go unanswered, this says when to ask the network to
/// answer and which of those settings to give back, a step at a time, as LoRaWAN 1.0.3 or
/// TS001-1.0.4 describes.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanBackoff {
    inner: Backoff,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanBackoff {
    /// Starts a count from zero.
    ///
    /// `version` is `1.0.3` or `1.0.4`. `limit` and `delay` are the 64 and 32 RP002-1.0.5
    /// recommends for every region unless given; a delay of zero is taken as one.
    #[new]
    #[pyo3(signature = (version = "1.0.4", limit = defaults::ADR_ACK_LIMIT, delay = defaults::ADR_ACK_DELAY))]
    fn new(version: &str, limit: u32, delay: u32) -> PyResult<Self> {
        Ok(Self {
            inner: Backoff::new(self::version(version)?, limit, delay),
        })
    }

    /// Counts one new uplink, and says what to do before sending it.
    ///
    /// Call it once per uplink the frame counter moves for, passing whether the device is
    /// already at its default data rate. A repeat of the same uplink does not count.
    fn uplink(&mut self, at_default_data_rate: bool) -> LorawanBackoffStep {
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
    fn downlink(&mut self) {
        self.inner.downlink();
    }

    /// How many uplinks have gone unanswered.
    #[getter]
    fn counter(&self) -> u32 {
        self.inner.counter()
    }

    /// The revision whose steps this count follows, `1.0.3` or `1.0.4`.
    #[getter]
    fn version(&self) -> &'static str {
        match self.inner.version() {
            Version::V1_0_3 => "1.0.3",
            Version::V1_0_4 => "1.0.4",
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanBackoff(version={:?}, counter={})",
            self.version(),
            self.inner.counter()
        )
    }
}

/// The optional channel list at the end of a join accept.
///
/// A network that wants a device on more channels than its region's defaults says so in
/// sixteen bytes: five frequencies for a dynamic plan such as EU868, or six groups of
/// channel mask bits for a fixed plan such as US915. The list keeps the bytes as they
/// arrived and reads either form out of them.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct LorawanCfList {
    inner: CfList,
}

impl LorawanCfList {
    /// Wraps a Rust channel list.
    pub(crate) fn from_core(inner: CfList) -> Self {
        Self { inner }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanCfList {
    /// Builds a type 0 list from five frequencies in hertz, with `0` for a slot left
    /// unused.
    ///
    /// Raises `PamojaError` if there are not five, or a frequency is not a whole number of
    /// hundreds of hertz from 100 MHz to just under 1.678 GHz.
    #[staticmethod]
    fn from_frequencies(frequencies_hz: Vec<u32>) -> PyResult<Self> {
        let slots = <[u32; 5]>::try_from(frequencies_hz.as_slice()).map_err(|_| {
            PamojaError::new_err(format!(
                "a channel list takes 5 frequencies, not {}",
                frequencies_hz.len()
            ))
        })?;
        CfList::frequencies(slots).map(Self::from_core).map_err(|_| {
            PamojaError::new_err(
                "a channel list frequency is a whole number of hundreds of hertz from 100 MHz to just under 1.678 GHz",
            )
        })
    }

    /// Builds a type 1 list from six mask groups, where bit n of group g enables channel
    /// g * 16 + n.
    #[staticmethod]
    fn from_channel_masks(masks: Vec<u16>) -> PyResult<Self> {
        let groups = <[u16; 6]>::try_from(masks.as_slice()).map_err(|_| {
            PamojaError::new_err(format!(
                "a channel list takes 6 mask groups, not {}",
                masks.len()
            ))
        })?;
        Ok(Self::from_core(CfList::channel_masks(groups)))
    }

    /// Keeps a channel list exactly as it arrived, whatever its type byte says.
    #[staticmethod]
    fn from_bytes(data: Vec<u8>) -> PyResult<Self> {
        let bytes = <[u8; CFLIST_LEN]>::try_from(data.as_slice()).map_err(|_| {
            PamojaError::new_err(format!(
                "a channel list is exactly {CFLIST_LEN} bytes, not {}",
                data.len()
            ))
        })?;
        Ok(Self::from_core(CfList::from_bytes(bytes)))
    }

    /// The sixteen bytes, as a join accept carries them.
    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.to_bytes())
    }

    /// Which form the list takes: `frequencies`, `channel_masks` or `reserved`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner.kind() {
            CfListKind::Frequencies => "frequencies",
            CfListKind::ChannelMasks => "channel_masks",
            CfListKind::Reserved(_) => "reserved",
        }
    }

    /// The CFListType byte the list ends with.
    #[getter]
    fn type_byte(&self) -> u8 {
        self.inner.kind().to_byte()
    }

    /// The five frequencies of a type 0 list in hertz, `0` for an unused slot, or `None`
    /// for a list of any other type.
    fn frequencies_hz(&self) -> Option<Vec<u32>> {
        self.inner.frequencies_hz().map(|slots| slots.to_vec())
    }

    /// The six mask groups of a type 1 list, or `None` for a list of any other type.
    fn channel_mask_groups(&self) -> Option<Vec<u16>> {
        self.inner
            .channel_mask_groups()
            .map(|groups| groups.to_vec())
    }

    /// Whether a type 1 list enables a channel, or `None` for a list of any other type or
    /// a channel past the 96 the groups cover.
    fn enables(&self, channel: u32) -> Option<bool> {
        self.inner.enables(u8::try_from(channel).ok()?)
    }

    /// The channels a type 1 list enables, lowest first, which is empty for a list of any
    /// other type.
    fn enabled_channels(&self) -> Vec<u32> {
        self.inner.enabled_channels().map(u32::from).collect()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanCfList.from_bytes(bytes.fromhex({:?}))",
            hex(&self.inner.to_bytes())
        )
    }
}

/// Renders bytes as lowercase hexadecimal.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
