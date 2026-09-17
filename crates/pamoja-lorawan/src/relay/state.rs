//! What a network configures on a relay, and the relay commands that configure it,
//! TS011-1.0.1 chapter 10.

use pamoja_lora::region::{ChannelPlan, Modulation, RelayChannel};
use pamoja_lora::LinkSettings;

use super::ack::{CadPeriodicity, CadToRx, XtalAccuracy};
use super::filter::JoinFilter;
use super::forward::WorChannel;
use super::limits::{CounterReset, ForwardLimits, TokenBucket, UNLIMITED_DEVICE_RATE};
use super::trusted::{TrustedDevice, TrustedDevices};
use crate::device::EndDevice;
use crate::mac::{relay_second_channel, MacCommand};

/// The symbols a scan takes to detect a preamble, besides the relay's time to switch to
/// receiving.
const DETECTION_SYMBOLS: u64 = 2;

/// What a relay's radio can do, which every WOR ACK it sends reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RelaySettings {
    /// How accurate its crystal is.
    pub xtal_accuracy: XtalAccuracy,
    /// How many symbols it takes from detecting a preamble to receiving.
    pub cad_to_rx: CadToRx,
}

impl RelaySettings {
    /// Describes a relay's radio.
    ///
    /// # Arguments
    ///
    /// * `xtal_accuracy` - how accurate its crystal is.
    /// * `cad_to_rx` - how many symbols it takes from detecting a preamble to receiving.
    ///
    /// # Returns
    ///
    /// The settings.
    #[must_use]
    pub const fn new(xtal_accuracy: XtalAccuracy, cad_to_rx: CadToRx) -> RelaySettings {
        RelaySettings {
            xtal_accuracy,
            cad_to_rx,
        }
    }
}

/// Which channels a relay scans and how often, as `RelayConfReq` sets them, TS011-1.0.1
/// section 10.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RelayConfig {
    /// How often the relay scans each channel.
    pub cad_periodicity: CadPeriodicity,
    /// The default channel, one of the region's.
    pub default_channel: RelayChannel,
    /// A second channel, scanned half a period after the default one.
    pub second_channel: Option<RelayChannel>,
}

impl RelayConfig {
    /// Scans one channel.
    ///
    /// # Arguments
    ///
    /// * `cad_periodicity` - how often.
    /// * `default_channel` - the channel.
    ///
    /// # Returns
    ///
    /// The configuration.
    #[must_use]
    pub const fn new(
        cad_periodicity: CadPeriodicity,
        default_channel: RelayChannel,
    ) -> RelayConfig {
        RelayConfig {
            cad_periodicity,
            default_channel,
            second_channel: None,
        }
    }

    /// The configuration a relay runs before its network sets one: the region's first
    /// default channel, scanned once a second.
    ///
    /// # Arguments
    ///
    /// * `plan` - the region's plan.
    ///
    /// # Returns
    ///
    /// The configuration, or `None` for a region with no relay channels.
    #[must_use]
    pub fn region_default(plan: &ChannelPlan) -> Option<RelayConfig> {
        Some(RelayConfig::new(
            CadPeriodicity::Ms1000,
            plan.relay_channel(0)?,
        ))
    }

    /// Adds a second channel.
    ///
    /// # Arguments
    ///
    /// * `channel` - the channel.
    ///
    /// # Returns
    ///
    /// The configuration, for chaining.
    #[must_use]
    pub const fn with_second_channel(mut self, channel: RelayChannel) -> RelayConfig {
        self.second_channel = Some(channel);
        self
    }

    /// Returns one of the channels.
    ///
    /// # Arguments
    ///
    /// * `channel` - which.
    ///
    /// # Returns
    ///
    /// The channel, or `None` for a second channel the configuration does not have.
    #[must_use]
    pub const fn channel(&self, channel: WorChannel) -> Option<RelayChannel> {
        match channel {
            WorChannel::Default => Some(self.default_channel),
            WorChannel::Second => self.second_channel,
        }
    }
}

/// Everything a relay keeps besides its own device: its radio, its scan, and the tables
/// its network sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RelayState {
    pub(crate) settings: RelaySettings,
    pub(crate) config: Option<RelayConfig>,
    pub(crate) anchor_us: Option<u64>,
    pub(crate) reloads: u64,
    pub(crate) filter: JoinFilter,
    pub(crate) trusted: TrustedDevices,
    pub(crate) limits: ForwardLimits,
}

impl RelayState {
    pub(crate) const fn new(settings: RelaySettings) -> RelayState {
        RelayState {
            settings,
            config: None,
            anchor_us: None,
            reloads: 0,
            filter: JoinFilter::new(),
            trusted: TrustedDevices::new(),
            limits: ForwardLimits::new(),
        }
    }

    /// Runs a configuration: a stopped relay starts its scan afresh, and a running one
    /// keeps its scan times.
    pub(crate) fn run(&mut self, config: RelayConfig) {
        if self.config.is_none() {
            self.anchor_us = None;
        }
        self.config = Some(config);
    }

    /// Stops the scan.
    pub(crate) fn stop(&mut self) {
        self.config = None;
        self.anchor_us = None;
    }

    /// Adds what the buckets earned in the hours since the scan started.
    pub(crate) fn reload(&mut self, now_us: u64) {
        let Some(anchor) = self.anchor_us else {
            return;
        };
        let hours = now_us.saturating_sub(anchor) / super::limits::RELOAD_PERIOD_US;
        if hours > self.reloads {
            let earned = hours - self.reloads;
            self.limits.reload(earned);
            self.trusted.reload(earned);
            self.reloads = hours;
        }
    }

    /// Acts on a relay command and says what to answer, TS011-1.0.1 sections 10.1 to 10.6.
    pub(crate) fn command(
        &mut self,
        command: &MacCommand,
        device: &EndDevice,
    ) -> Option<MacCommand> {
        match *command {
            MacCommand::RelayConfReq {
                enabled,
                cad_periodicity,
                default_channel_index,
                second_channel_index,
                second_channel_data_rate,
                second_channel_ack_offset,
                second_channel_frequency_hz,
            } => {
                if !enabled {
                    self.stop();
                    return Some(MacCommand::RelayConfAns {
                        cad_periodicity_ack: true,
                        default_channel_index_ack: true,
                        second_channel_index_ack: true,
                        second_channel_data_rate_ack: true,
                        second_channel_ack_offset_ack: true,
                        second_channel_frequency_ack: true,
                    });
                }
                let plan = device.plan();
                let periodicity = CadPeriodicity::from_code(cad_periodicity);
                let default_channel = plan
                    .relay_channel(default_channel_index)
                    .filter(|channel| channel_ok(device, channel));
                let second_channel_index_ack = second_channel_index <= 1;
                let (data_rate_ack, ack_offset_ack, frequency_ack) = if second_channel_index == 1 {
                    let data_rate_ack = wor_link(plan, second_channel_data_rate).is_some();
                    let frequency_ack = device.tunes(second_channel_frequency_hz);
                    let ack_offset_ack = relay_second_channel(
                        1,
                        second_channel_data_rate,
                        second_channel_ack_offset,
                        second_channel_frequency_hz,
                    )
                    .is_some_and(|channel| device.tunes(channel.ack_frequency_hz));
                    (data_rate_ack, ack_offset_ack, frequency_ack)
                } else {
                    (true, true, true)
                };
                let second_channel = relay_second_channel(
                    second_channel_index,
                    second_channel_data_rate,
                    second_channel_ack_offset,
                    second_channel_frequency_hz,
                );
                let scans_fit = match (periodicity, default_channel) {
                    (Some(periodicity), Some(default_channel)) => {
                        let mut config = RelayConfig::new(periodicity, default_channel);
                        config.second_channel = second_channel;
                        scans_fit(plan, &config, self.settings.cad_to_rx)
                    }
                    _ => true,
                };
                let answer = MacCommand::RelayConfAns {
                    cad_periodicity_ack: periodicity.is_some() && scans_fit,
                    default_channel_index_ack: default_channel.is_some(),
                    second_channel_index_ack,
                    second_channel_data_rate_ack: data_rate_ack,
                    second_channel_ack_offset_ack: ack_offset_ack,
                    second_channel_frequency_ack: frequency_ack,
                };
                if let (Some(periodicity), Some(default_channel), true) = (
                    periodicity,
                    default_channel,
                    scans_fit
                        && second_channel_index_ack
                        && data_rate_ack
                        && ack_offset_ack
                        && frequency_ack,
                ) {
                    self.run(RelayConfig {
                        cad_periodicity: periodicity,
                        default_channel,
                        second_channel,
                    });
                }
                Some(answer)
            }
            MacCommand::FilterListReq {
                index,
                action,
                eui_len,
                ref eui,
            } => Some(self.filter.apply(index, action, eui_len, eui)),
            MacCommand::UpdateUplinkListReq {
                index,
                reload_rate,
                bucket_size,
                dev_addr,
                wfcnt,
                ref root_wor_s_key,
            } => {
                let bucket = TokenBucket::coded(reload_rate, bucket_size, UNLIMITED_DEVICE_RATE);
                self.trusted.set(
                    index,
                    TrustedDevice::new(dev_addr, root_wor_s_key, wfcnt, bucket),
                );
                Some(MacCommand::UpdateUplinkListAns)
            }
            MacCommand::CtrlUplinkListReq { index, action } => {
                let wfcnt = self.trusted.get(index).map(TrustedDevice::last_wfcnt);
                if wfcnt.is_some() && action == 1 {
                    self.trusted.remove(index);
                }
                Some(MacCommand::CtrlUplinkListAns {
                    index_ack: wfcnt.is_some(),
                    wfcnt: wfcnt.unwrap_or(0),
                })
            }
            MacCommand::ConfigureFwdLimitReq {
                reset_limit_counters,
                join_request_reload_rate,
                notify_reload_rate,
                global_uplink_reload_rate,
                overall_reload_rate,
                join_request_bucket_size,
                notify_bucket_size,
                global_uplink_bucket_size,
                overall_bucket_size,
            } => {
                self.limits.configure(
                    CounterReset::from_code(reset_limit_counters),
                    [
                        join_request_reload_rate,
                        notify_reload_rate,
                        global_uplink_reload_rate,
                        overall_reload_rate,
                    ],
                    [
                        join_request_bucket_size,
                        notify_bucket_size,
                        global_uplink_bucket_size,
                        overall_bucket_size,
                    ],
                );
                Some(MacCommand::ConfigureFwdLimitAns)
            }
            _ => None,
        }
    }
}

/// The LoRa settings of a relay channel's data rate, numbered in the downlink table.
pub(crate) fn wor_link(plan: &ChannelPlan, data_rate: u8) -> Option<LinkSettings> {
    match plan.downlink_data_rate(data_rate)?.modulation {
        Modulation::LoRa {
            spreading_factor,
            bandwidth_hz,
        } => Some(LinkSettings::new(spreading_factor, bandwidth_hz)),
        _ => None,
    }
}

/// Whether a relay channel is LoRa and its frequencies are ones the radio tunes.
pub(crate) fn channel_ok(device: &EndDevice, channel: &RelayChannel) -> bool {
    wor_link(device.plan(), channel.data_rate).is_some()
        && device.tunes(channel.wor_frequency_hz)
        && device.tunes(channel.ack_frequency_hz)
}

/// Whether a configuration is one a relay can run: LoRa channels it tunes, and scans far
/// enough apart that each can detect a preamble and switch to receiving before the next.
pub(crate) fn config_ok(device: &EndDevice, config: &RelayConfig, cad_to_rx: CadToRx) -> bool {
    channel_ok(device, &config.default_channel)
        && config
            .second_channel
            .is_none_or(|channel| channel_ok(device, &channel))
        && scans_fit(device.plan(), config, cad_to_rx)
}

/// Whether every channel's scan, detection and switch to receiving fit in the time to the
/// next scan: a whole period, or half of one with a second channel. A channel whose data
/// rate is not LoRa is another check's to refuse.
fn scans_fit(plan: &ChannelPlan, config: &RelayConfig, cad_to_rx: CadToRx) -> bool {
    let period = config.cad_periodicity.period_us();
    let interval = if config.second_channel.is_some() {
        period / 2
    } else {
        period
    };
    [Some(config.default_channel), config.second_channel]
        .into_iter()
        .flatten()
        .all(|channel| {
            wor_link(plan, channel.data_rate).is_none_or(|link| {
                (DETECTION_SYMBOLS + u64::from(cad_to_rx.symbols())) * link.symbol_time_us()
                    <= interval
            })
        })
}
