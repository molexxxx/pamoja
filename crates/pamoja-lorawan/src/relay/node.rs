//! A relay: an end device that also listens for others and forwards what it hears,
//! TS011-1.0.1 chapters 3, 7 and 8.

use pamoja_lora::region::{Modulation, RelayChannel};
use pamoja_lora::LinkSettings;

use super::ack::{wor_ack, CadPeriodicity, Forward, StateSync};
use super::filter::{FilterAction, JoinFilter};
use super::forward::{ForwardedUplink, UplinkMetadata, WorChannel, FORWARD_OVERHEAD};
use super::limits::{ForwardLimits, RELOAD_PERIOD_US};
use super::state::{config_ok, wor_link, RelayConfig, RelaySettings, RelayState};
use super::timing::{t_offset_ms, unsynchronized_preamble_symbols};
use super::trusted::TrustedDevices;
use super::wor::{Carrier, Wor};
use super::{
    LA_FPORT_RELAY, RELAY_FWD_DELAY_US, RXR_DELAY_US, WOR_ACK_DELAY_US, WOR_ACK_LEN,
    WOR_DATA_DELAY_US,
};
use crate::device::{Delivery, DeviceError, EndDevice, Heard, Next, ReceiveWindow, Transmission};
use crate::mac::{MacCommand, CID_NOTIFY_NEW_END_DEVICE};
use crate::{LorawanError, MAX_FRAME};

/// The length of a join request, TS001-1.0.4 section 6.2.4.
const JOIN_REQUEST_LEN: usize = 23;

/// The shortest data frame: its header, address, control, counter and integrity code.
const MIN_DATA_FRAME: usize = 12;

/// A scan for wake-on-radio frames, due next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scan {
    /// When to start detecting, in microseconds.
    pub start_us: u64,
    /// Which channel.
    pub channel: WorChannel,
    /// Its WOR frequency and data rate.
    pub carrier: Carrier,
    /// The LoRa settings of a WOR frame: an explicit header and a payload CRC, heard with
    /// inverted IQ, as RP002-1.0.5 table 124 has.
    pub link: LinkSettings,
    /// The longest preamble an end device sends on the channel, in symbols: a second of
    /// it on the default channel, which a device that has never heard from a relay assumes
    /// it scans, and a scan period of it on the second.
    pub preamble_symbols: u16,
}

/// What a wake-on-radio frame led a relay to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake {
    /// A join request follows; listen for it.
    JoinRequest {
        /// Where and when.
        listen: Listen,
    },
    /// A trusted end device's uplink follows.
    Uplink {
        /// The device.
        dev_addr: u32,
        /// The WOR frame counter it used.
        wfcnt: u32,
        /// Whether the relay forwards the uplink, which the acknowledgment reports.
        forward: Forward,
        /// The acknowledgment to send, or `None` when the relay's duty cycle leaves no room
        /// for one.
        acknowledgment: Option<Acknowledgment>,
        /// Where and when to listen for the uplink, or `None` when a forwarding limit
        /// keeps the relay from forwarding it.
        listen: Option<Listen>,
    },
    /// A device the relay does not trust woke it, and a `NotifyNewEndDeviceReq` for it
    /// goes out with the relay's next uplink.
    Notified {
        /// The address the frame named.
        dev_addr: u32,
    },
}

/// A WOR ACK to send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Acknowledgment {
    /// The frame.
    pub frame: [u8; WOR_ACK_LEN],
    /// When to start sending it, in microseconds.
    pub start_us: u64,
    /// The channel's acknowledgment frequency, at the WOR frame's data rate.
    pub carrier: Carrier,
    /// The LoRa settings: an eight-symbol preamble, an explicit header and a payload CRC,
    /// sent with inverted IQ, RP002-1.0.5 table 125.
    pub link: LinkSettings,
    /// The power to ask of the radio, the region's default.
    pub output_dbm: i8,
    /// How long it holds the air, in microseconds.
    pub airtime_us: u64,
}

/// When and where to listen for the uplink a wake-on-radio frame announced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listen {
    /// When the uplink starts, in microseconds.
    pub start_us: u64,
    /// Where and how fast.
    pub carrier: Carrier,
    /// The LoRa settings of an uplink, heard with standard IQ.
    pub link: LinkSettings,
    /// The longest frame the relay forwards, in bytes; stop receiving anything longer, as
    /// TS011-1.0.1 section 3.4 asks.
    pub max_len: usize,
}

/// A downlink for an end device, to send in its RXR window, TS011-1.0.1 chapter 7.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RxrDownlink {
    frame: [u8; MAX_FRAME],
    len: usize,
    /// When to start sending it: [`RXR_DELAY_US`] after the end device's uplink ended.
    pub start_us: u64,
    /// The frequency of the WOR frame, at the uplink's first window data rate with no
    /// offset, RP002-1.0.5 table 126.
    pub carrier: Carrier,
    /// The LoRa settings: an eight-symbol preamble, an explicit header and a payload CRC,
    /// sent with inverted IQ, RP002-1.0.5 table 127.
    pub link: LinkSettings,
    /// The power to ask of the radio, the region's default.
    pub output_dbm: i8,
    /// How long it holds the air, in microseconds.
    pub airtime_us: u64,
}

impl RxrDownlink {
    /// Returns the frame to send.
    ///
    /// # Returns
    ///
    /// The end device's PHYPayload, as the network sent it.
    #[must_use]
    pub fn frame(&self) -> &[u8] {
        &self.frame[..self.len]
    }
}

/// What a frame a relay's own device heard turned out to be.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayHeard {
    /// A join accept, or a downlink for the relay itself.
    Device(Heard),
    /// A downlink the network sent for an end device, to send on in its RXR window.
    Downlink {
        /// The downlink as the relay's device read it.
        delivery: Delivery,
        /// What to send the end device.
        downlink: RxrDownlink,
    },
    /// A downlink for an end device the relay cannot send on.
    Undeliverable {
        /// The downlink as the relay's device read it.
        delivery: Delivery,
        /// Why.
        reason: RelayError,
    },
}

/// Why a relay did not do what a frame or a call asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayError {
    /// The relay is not scanning.
    Stopped,
    /// A forwarded uplink is still waiting to go out.
    Busy,
    /// No wake-on-radio frame announced an uplink to listen for.
    NotListening,
    /// No forwarded uplink is waiting to go out.
    NothingHeld,
    /// No forwarded uplink is waiting on the network's answer.
    NothingAwaited,
    /// The frame did not decode or verify.
    Frame(LorawanError),
    /// A frame names a carrier the relay cannot hear or send on.
    Carrier(Carrier),
    /// A forwarding limit is reached.
    Limited,
    /// The join request filter drops the request.
    Filtered,
    /// The uplink is from another device than the WOR frame named.
    Foreign,
    /// The relay configuration uses a channel the relay cannot scan, or scans too often
    /// for its data rates.
    Configuration,
    /// The relay's own device refused.
    Device(DeviceError),
}

impl core::fmt::Display for RelayError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RelayError::Stopped => f.write_str("the relay is not scanning"),
            RelayError::Busy => f.write_str("a forwarded uplink is still waiting to go out"),
            RelayError::NotListening => f.write_str("no WOR frame announced this uplink"),
            RelayError::NothingHeld => f.write_str("no forwarded uplink is waiting to go out"),
            RelayError::NothingAwaited => f.write_str("no forwarded uplink awaits an answer"),
            RelayError::Frame(error) => write!(f, "the frame did not decode: {error}"),
            RelayError::Carrier(carrier) => write!(
                f,
                "the relay cannot use {} Hz at DR{}",
                carrier.frequency_hz, carrier.data_rate
            ),
            RelayError::Limited => f.write_str("a forwarding limit is reached"),
            RelayError::Filtered => f.write_str("the join request filter drops it"),
            RelayError::Foreign => f.write_str("the uplink is not from the device that woke it"),
            RelayError::Configuration => f.write_str("the relay cannot run that configuration"),
            RelayError::Device(error) => write!(f, "the relay's device refused: {error}"),
        }
    }
}

impl core::error::Error for RelayError {}

impl From<DeviceError> for RelayError {
    fn from(error: DeviceError) -> RelayError {
        RelayError::Device(error)
    }
}

/// Which uplink a wake-on-radio frame announced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Announced {
    JoinRequest,
    Uplink { index: u8, dev_addr: u32 },
}

/// A wake-on-radio exchange waiting on its uplink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Exchange {
    announced: Announced,
    channel: WorChannel,
    wor_frequency_hz: u32,
    uplink: Carrier,
    max_len: usize,
}

/// Where a forwarded uplink's answer goes: the end device's RXR window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Route {
    uplink_end_us: u64,
    wor_frequency_hz: u32,
    uplink_data_rate: u8,
}

/// A forwarded uplink waiting to go out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Held {
    payload: [u8; FORWARD_OVERHEAD + MAX_FRAME],
    len: usize,
    due_us: u64,
    route: Route,
}

/// A LoRaWAN relay: an end device that scans for wake-on-radio frames, acknowledges and
/// forwards the uplinks behind them, and sends the network's answers on in each end
/// device's RXR window.
///
/// Like [`EndDevice`], it owns no radio and no clock. One forwarded uplink runs like this:
///
/// 1. [`next_scan`](Relay::next_scan) says when and where to detect a preamble.
/// 2. A WOR frame received there goes to [`heard_wor`](Relay::heard_wor), which says
///    whether to acknowledge it and when to listen for the uplink.
/// 3. That uplink goes to [`heard_uplink`](Relay::heard_uplink), and
///    [`forward`](Relay::forward) sends it to the network in the relay's own uplink on port
///    [`LA_FPORT_RELAY`].
/// 4. What the relay's receive windows hear goes to [`heard_in`](Relay::heard_in), which
///    reads the relay commands among its MAC commands and turns a downlink for the end
///    device into an [`RxrDownlink`].
///
/// # Examples
///
/// A relay forwards a sensor's reading and sends the network's answer back:
///
/// ```
/// use pamoja_lora::region::Region;
/// use pamoja_lorawan::device::{EndDevice, ReceiveWindow, Settings};
/// use pamoja_lorawan::mac::MacCommand;
/// use pamoja_lorawan::relay::{
///     root_wor_s_key, wor_uplink, CadToRx, Carrier, ForwardedUplink, Relay, RelayConfig,
///     RelayHeard, RelaySettings, Wake, WorKeys, XtalAccuracy, LA_FPORT_RELAY,
/// };
/// use pamoja_lorawan::{Downlink, Session, Uplink};
///
/// let plan = Region::Eu868.plan();
/// let relay_session = Session::new(0x2601_0001, [0x11; 16], [0x22; 16]);
/// let device = EndDevice::personalized(plan, relay_session, Settings::new(2, 14))?;
/// let settings = RelaySettings::new(XtalAccuracy::Ppm10, CadToRx::Symbols2);
/// let mut relay = Relay::new(device, settings);
/// relay.start(RelayConfig::region_default(plan).expect("EU868 has relay channels"))?;
///
/// // The network trusts a sensor to it, as an UpdateUplinkListReq would.
/// let sensor = Session::new(0x2601_1BDA, [0x2B; 16], [0x99; 16]);
/// let root = root_wor_s_key(&[0x2B; 16]);
/// relay.trust(0, sensor.dev_addr(), &root, 0, 63, 0);
///
/// // The first scan starts now, on 865.1 MHz.
/// let scan = relay.next_scan(0).expect("running");
/// assert_eq!(scan.carrier, Carrier::new(865_100_000, 3));
///
/// // The sensor's WOR frame says its uplink follows on 868.1 MHz at DR5.
/// let uplink = Carrier::new(868_100_000, 5);
/// let wor = wor_uplink(&WorKeys::derive(&root, sensor.dev_addr()), sensor.dev_addr(), 0, uplink, scan.carrier)?;
/// let Wake::Uplink { acknowledgment, listen, .. } = relay.heard_wor(&scan, &wor, -90, 4, 900_000)?
/// else {
///     panic!("a trusted device's uplink follows");
/// };
/// assert_eq!(acknowledgment.expect("room to acknowledge").carrier, Carrier::new(865_300_000, 3));
/// let listen = listen.expect("forwarding is available");
///
/// // The uplink arrives, and goes out in the relay's own uplink on port 226.
/// let frame = sensor.encode_uplink(&Uplink::new(0, 1, b"21.5"))?;
/// let due = relay.heard_uplink(frame.as_bytes(), -88, 6, listen.start_us + 60_000)?;
/// let transmission = relay.forward(due)?;
///
/// // The network reads it back, and answers the sensor through the relay.
/// let sent = relay_session.decode(transmission.frame.as_bytes(), 0)?;
/// assert_eq!(sent.fport(), Some(LA_FPORT_RELAY));
/// assert_eq!(ForwardedUplink::parse(sent.payload())?.phy_payload, frame.as_bytes());
/// let answer = sensor.encode_downlink(&Downlink::new(0, 1, b"ok"))?;
/// let reply = relay_session.encode_downlink(&Downlink::new(0, LA_FPORT_RELAY, answer.as_bytes()))?;
///
/// let RelayHeard::Downlink { downlink, .. } = relay.heard_in(ReceiveWindow::Rx1, reply.as_bytes(), 7)?
/// else {
///     panic!("a downlink for the sensor");
/// };
/// assert_eq!(downlink.frame(), answer.as_bytes());
/// assert_eq!(downlink.carrier.frequency_hz, 865_100_000, "on the WOR frequency");
/// assert_eq!(downlink.start_us, listen.start_us + 60_000 + 18_000_000);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Relay<'p> {
    device: EndDevice<'p>,
    state: RelayState,
    exchange: Option<Exchange>,
    held: Option<Held>,
    awaiting: Option<Route>,
}

impl<'p> Relay<'p> {
    /// Makes a relay of an end device.
    ///
    /// It does not scan until [`start`](Relay::start) or its network's `RelayConfReq` gives
    /// it channels, and it forwards every join request and trusts no one until its network
    /// says otherwise.
    ///
    /// # Arguments
    ///
    /// * `device` - the relay's own device, joined or to join.
    /// * `settings` - what its radio can do.
    ///
    /// # Returns
    ///
    /// The relay.
    #[must_use]
    pub const fn new(device: EndDevice<'p>, settings: RelaySettings) -> Relay<'p> {
        Relay {
            device,
            state: RelayState::new(settings),
            exchange: None,
            held: None,
            awaiting: None,
        }
    }

    /// Returns the relay's own device.
    ///
    /// # Returns
    ///
    /// The device.
    #[must_use]
    pub const fn device(&self) -> &EndDevice<'p> {
        &self.device
    }

    /// Returns the relay's own device, to join, send its own uplinks, and hear what comes
    /// back for them.
    ///
    /// Read downlinks through [`heard_in`](Relay::heard_in) instead, so the relay commands
    /// among their MAC commands reach the relay.
    ///
    /// # Returns
    ///
    /// The device.
    pub fn device_mut(&mut self) -> &mut EndDevice<'p> {
        &mut self.device
    }

    /// Gives the relay's device back.
    ///
    /// # Returns
    ///
    /// The device.
    #[must_use]
    pub fn into_device(self) -> EndDevice<'p> {
        self.device
    }

    /// Returns what the relay's radio can do.
    ///
    /// # Returns
    ///
    /// The settings.
    #[must_use]
    pub const fn settings(&self) -> RelaySettings {
        self.state.settings
    }

    /// Returns the channels the relay scans.
    ///
    /// # Returns
    ///
    /// The configuration, or `None` while the relay is stopped.
    #[must_use]
    pub const fn config(&self) -> Option<RelayConfig> {
        self.state.config
    }

    /// Returns one of the region's wake-on-radio channels, TS011-1.0.1 section 3.2.2.
    ///
    /// # Arguments
    ///
    /// * `index` - which, 0 or 1.
    ///
    /// # Returns
    ///
    /// The channel, or `None` where the region defines none.
    #[must_use]
    pub fn region_channel(&self, index: u8) -> Option<RelayChannel> {
        self.device.plan().relay_channel(index)
    }

    /// Starts scanning, or changes what a running relay scans from its next scan on.
    ///
    /// # Arguments
    ///
    /// * `config` - the channels and how often.
    ///
    /// # Errors
    ///
    /// Returns [`RelayError::Configuration`] for a channel that is not LoRa or not in the
    /// radio's range, or scans so frequent that detecting a preamble and starting to receive
    /// on one channel does not fit before the next.
    pub fn start(&mut self, config: RelayConfig) -> Result<(), RelayError> {
        if !config_ok(&self.device, &config, self.state.settings.cad_to_rx) {
            return Err(RelayError::Configuration);
        }
        self.state.run(config);
        Ok(())
    }

    /// Stops scanning. A forwarded uplink already waiting still goes out.
    pub fn stop(&mut self) {
        self.state.stop();
        self.exchange = None;
    }

    /// Returns the join request filter.
    ///
    /// # Returns
    ///
    /// The filter.
    #[must_use]
    pub const fn join_filter(&self) -> &JoinFilter {
        &self.state.filter
    }

    /// Returns the join request filter to change.
    ///
    /// # Returns
    ///
    /// The filter.
    pub fn join_filter_mut(&mut self) -> &mut JoinFilter {
        &mut self.state.filter
    }

    /// Returns the end devices the relay trusts.
    ///
    /// # Returns
    ///
    /// The list.
    #[must_use]
    pub const fn trusted(&self) -> &TrustedDevices {
        &self.state.trusted
    }

    /// Returns the end devices the relay trusts, to change.
    ///
    /// # Returns
    ///
    /// The list.
    pub fn trusted_mut(&mut self) -> &mut TrustedDevices {
        &mut self.state.trusted
    }

    /// Trusts an end device, as an `UpdateUplinkListReq` with the same fields does.
    ///
    /// # Arguments
    ///
    /// * `index` - the entry, 0 to 15.
    /// * `dev_addr` - the device's address.
    /// * `root_wor_s_key` - its root relay session key.
    /// * `next_wfcnt` - the WOR frame counter to expect from it next.
    /// * `reload_rate` - its uplinks forwarded an hour, 63 for no limit.
    /// * `bucket_size` - the coded bucket size multiplier, TS011-1.0.1 table 55.
    ///
    /// # Returns
    ///
    /// `false` for an index past 15.
    pub fn trust(
        &mut self,
        index: u8,
        dev_addr: u32,
        root_wor_s_key: &[u8; 16],
        next_wfcnt: u32,
        reload_rate: u8,
        bucket_size: u8,
    ) -> bool {
        index < 16
            && self
                .state
                .command(
                    &MacCommand::UpdateUplinkListReq {
                        index,
                        reload_rate,
                        bucket_size,
                        dev_addr,
                        wfcnt: next_wfcnt,
                        root_wor_s_key: *root_wor_s_key,
                    },
                    &self.device,
                )
                .is_some()
    }

    /// Returns the relay-wide forwarding limits.
    ///
    /// # Returns
    ///
    /// The limits.
    #[must_use]
    pub const fn limits(&self) -> &ForwardLimits {
        &self.state.limits
    }

    /// Returns the relay-wide forwarding limits, to change.
    ///
    /// # Returns
    ///
    /// The limits.
    pub fn limits_mut(&mut self) -> &mut ForwardLimits {
        &mut self.state.limits
    }

    /// Says when and where to scan next.
    ///
    /// The first call after the relay starts begins its scan at `now_us`, with every
    /// forwarding limit at one hour's reload, as TS011-1.0.1 section 8.8 has. Scans fall on
    /// whole periods from then, and a second channel's half a period between, so a relay
    /// that was busy picks the schedule up at the next of them, section 3.2.1.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the time, in microseconds.
    ///
    /// # Returns
    ///
    /// The scan, or `None` while the relay is stopped.
    pub fn next_scan(&mut self, now_us: u64) -> Option<Scan> {
        let config = self.state.config?;
        let anchor = match self.state.anchor_us {
            Some(anchor) => anchor,
            None => {
                self.state.anchor_us = Some(now_us);
                self.state.reloads = 0;
                self.state.limits.restart();
                now_us
            }
        };
        let period = config.cad_periodicity.period_us();
        let default_at = next_multiple(anchor, period, now_us);
        let (start_us, channel) = match config.second_channel {
            Some(_) => {
                let second_at = next_multiple(anchor + period / 2, period, now_us);
                if second_at < default_at {
                    (second_at, WorChannel::Second)
                } else {
                    (default_at, WorChannel::Default)
                }
            }
            None => (default_at, WorChannel::Default),
        };
        let relay_channel = config.channel(channel)?;
        let link = wor_link(self.device.plan(), relay_channel.data_rate)?;
        let longest = match channel {
            WorChannel::Default => CadPeriodicity::Ms1000,
            WorChannel::Second => config.cad_periodicity,
        };
        Some(Scan {
            start_us,
            channel,
            carrier: Carrier::new(relay_channel.wor_frequency_hz, relay_channel.data_rate),
            link,
            preamble_symbols: unsynchronized_preamble_symbols(
                longest,
                link.symbol_time_us(),
                self.state.settings.cad_to_rx,
            ),
        })
    }

    /// Reads a wake-on-radio frame a scan heard, TS011-1.0.1 sections 3.2.4 and 3.3.
    ///
    /// A join request's frame has the relay listen for the request, if the join request
    /// limit allows. A trusted device's frame is verified against the counter the relay
    /// expects from it, and answered with a WOR ACK saying whether the relay forwards the
    /// uplink, which it listens for only if it does. A frame from a device the relay does
    /// not trust queues a notification of it for the network, once while one is waiting,
    /// if the notification limit allows.
    ///
    /// # Arguments
    ///
    /// * `scan` - the scan that heard it.
    /// * `frame` - the frame.
    /// * `rssi_dbm` - its signal strength.
    /// * `snr_db` - its signal-to-noise ratio.
    /// * `ended_us` - when it finished arriving, in microseconds.
    ///
    /// # Returns
    ///
    /// What to do next.
    ///
    /// # Errors
    ///
    /// Returns [`RelayError::Stopped`] while stopped, [`RelayError::Busy`] while a forwarded
    /// uplink waits to go out, [`RelayError::Frame`] for a frame that does not decode or, from
    /// a trusted device's address, does not verify, [`RelayError::Carrier`] for an uplink
    /// on a carrier the relay cannot hear, and [`RelayError::Limited`] when a limit keeps it
    /// from listening or notifying.
    pub fn heard_wor(
        &mut self,
        scan: &Scan,
        frame: &[u8],
        rssi_dbm: i16,
        snr_db: i8,
        ended_us: u64,
    ) -> Result<Wake, RelayError> {
        let config = self.state.config.ok_or(RelayError::Stopped)?;
        if self.held.is_some() {
            return Err(RelayError::Busy);
        }
        self.exchange = None;
        self.state.reload(ended_us);
        let channel = config
            .channel(scan.channel)
            .ok_or(RelayError::Carrier(scan.carrier))?;

        match Wor::parse(frame).map_err(RelayError::Frame)? {
            Wor::JoinRequest { uplink } => {
                let link = self.uplink_link(uplink)?;
                let limits = &self.state.limits;
                if !(limits.join_request.has_token() && limits.overall.has_token()) {
                    return Err(RelayError::Limited);
                }
                if self.forwardable_len(uplink)? < JOIN_REQUEST_LEN {
                    return Err(RelayError::Frame(LorawanError::PayloadTooLong));
                }
                let listen = Listen {
                    start_us: ended_us + u64::from(WOR_DATA_DELAY_US),
                    carrier: uplink,
                    link,
                    max_len: JOIN_REQUEST_LEN,
                };
                self.exchange = Some(Exchange {
                    announced: Announced::JoinRequest,
                    channel: scan.channel,
                    wor_frequency_hz: scan.carrier.frequency_hz,
                    uplink,
                    max_len: JOIN_REQUEST_LEN,
                });
                Ok(Wake::JoinRequest { listen })
            }
            Wor::Uplink(sealed) => {
                let dev_addr = sealed.dev_addr();
                let (index, wfcnt, uplink) = match self.state.trusted.open(&sealed, scan.carrier) {
                    None => return self.notify(dev_addr, rssi_dbm, snr_db),
                    Some(opened) => opened.map_err(RelayError::Frame)?,
                };
                let link = self.uplink_link(uplink)?;
                let max_len = self.forwardable_len(uplink)?;
                let mut forward = self.forward_status(index, ended_us);
                if forward == Forward::Available && max_len < MIN_DATA_FRAME {
                    forward = Forward::Disabled;
                }

                let ack_carrier = Carrier::new(channel.ack_frequency_hz, channel.data_rate);
                let ack_link = scan.link.with_preamble(8);
                let ack_start = ended_us + u64::from(WOR_ACK_DELAY_US);
                let ack_airtime = ack_link.airtime_us(WOR_ACK_LEN);
                let preamble_end =
                    ended_us.saturating_sub(scan.link.with_preamble(0).airtime_us(frame.len()));
                let t_offset = t_offset_ms(scan.start_us, preamble_end).unwrap_or(
                    if preamble_end < scan.start_us {
                        0
                    } else {
                        StateSync::MAX_T_OFFSET_MS
                    },
                );
                let state = StateSync {
                    cad_to_rx: self.state.settings.cad_to_rx,
                    forward,
                    relay_data_rate: self.device.data_rate(),
                    xtal_accuracy: self.state.settings.xtal_accuracy,
                    cad_periodicity: config.cad_periodicity,
                    t_offset_ms: t_offset,
                };
                let keys = *self
                    .state
                    .trusted
                    .get(index)
                    .ok_or(RelayError::NotListening)?
                    .keys();
                let acknowledgment = if self.device.free_at(ack_carrier.frequency_hz) <= ack_start {
                    let frame = wor_ack(&keys, dev_addr, wfcnt, ack_carrier, uplink, state)
                        .map_err(RelayError::Frame)?;
                    self.device
                        .record_air(ack_start, ack_airtime, ack_carrier.frequency_hz);
                    Some(Acknowledgment {
                        frame,
                        start_us: ack_start,
                        carrier: ack_carrier,
                        link: ack_link,
                        output_dbm: self.device.output_dbm(ack_carrier.frequency_hz, 0),
                        airtime_us: ack_airtime,
                    })
                } else {
                    None
                };

                let listen = (forward == Forward::Available).then(|| Listen {
                    start_us: ack_start + ack_airtime + u64::from(WOR_DATA_DELAY_US),
                    carrier: uplink,
                    link,
                    max_len,
                });
                if listen.is_some() {
                    self.exchange = Some(Exchange {
                        announced: Announced::Uplink { index, dev_addr },
                        channel: scan.channel,
                        wor_frequency_hz: scan.carrier.frequency_hz,
                        uplink,
                        max_len,
                    });
                }
                Ok(Wake::Uplink {
                    dev_addr,
                    wfcnt,
                    forward,
                    acknowledgment,
                    listen,
                })
            }
        }
    }

    /// Reads the uplink a wake-on-radio frame announced, and holds it to forward,
    /// TS011-1.0.1 sections 3.5, 8.1 and 8.6.
    ///
    /// A join request must be one, and pass the join request filter. A data uplink must
    /// come from the device whose WOR frame announced it. Either spends a token from every
    /// limit it counts against.
    ///
    /// # Arguments
    ///
    /// * `frame` - the uplink.
    /// * `rssi_dbm` - its signal strength, which the forwarded metadata carries.
    /// * `snr_db` - its signal-to-noise ratio, likewise.
    /// * `ended_us` - when it finished arriving, in microseconds.
    ///
    /// # Returns
    ///
    /// When to [`forward`](Relay::forward) it: [`RELAY_FWD_DELAY_US`] after it ended.
    ///
    /// # Errors
    ///
    /// Returns [`RelayError::NotListening`] with no uplink announced, [`RelayError::Frame`]
    /// for a frame longer than the relay forwards or not of the kind announced,
    /// [`RelayError::Foreign`] for another device's uplink, [`RelayError::Filtered`] for a
    /// join request the filter drops, and [`RelayError::Limited`] when a limit is reached.
    pub fn heard_uplink(
        &mut self,
        frame: &[u8],
        rssi_dbm: i16,
        snr_db: i8,
        ended_us: u64,
    ) -> Result<u64, RelayError> {
        let exchange = self.exchange.take().ok_or(RelayError::NotListening)?;
        if frame.len() > exchange.max_len {
            return Err(RelayError::Frame(LorawanError::PayloadTooLong));
        }
        self.state.reload(ended_us);
        let limits = &mut self.state.limits;
        match exchange.announced {
            Announced::JoinRequest => {
                if frame.len() != JOIN_REQUEST_LEN || frame[0] & 0xE3 != 0x00 {
                    return Err(RelayError::Frame(LorawanError::MalformedFrame));
                }
                let mut join_eui = [0u8; 8];
                let mut dev_eui = [0u8; 8];
                for (at, byte) in frame[1..9].iter().rev().enumerate() {
                    join_eui[at] = *byte;
                }
                for (at, byte) in frame[9..17].iter().rev().enumerate() {
                    dev_eui[at] = *byte;
                }
                if self.state.filter.decide(&join_eui, &dev_eui) == FilterAction::Filter {
                    return Err(RelayError::Filtered);
                }
                if !(limits.join_request.has_token() && limits.overall.has_token()) {
                    return Err(RelayError::Limited);
                }
                limits.join_request.take();
                limits.overall.take();
            }
            Announced::Uplink { index, dev_addr } => {
                if frame.len() < MIN_DATA_FRAME {
                    return Err(RelayError::Frame(LorawanError::FrameTooShort));
                }
                if !matches!(frame[0] & 0xE3, 0x40 | 0x80) {
                    return Err(RelayError::Frame(LorawanError::MalformedFrame));
                }
                if u32::from_le_bytes([frame[1], frame[2], frame[3], frame[4]]) != dev_addr {
                    return Err(RelayError::Foreign);
                }
                let device = self
                    .state
                    .trusted
                    .get_mut(index)
                    .filter(|device| device.dev_addr() == dev_addr)
                    .ok_or(RelayError::NotListening)?;
                if !(device.bucket().has_token()
                    && limits.global_uplink.has_token()
                    && limits.overall.has_token())
                {
                    return Err(RelayError::Limited);
                }
                device.bucket_mut().take();
                limits.global_uplink.take();
                limits.overall.take();
            }
        }

        let forwarded = ForwardedUplink {
            metadata: UplinkMetadata {
                wor_channel: exchange.channel,
                rssi_dbm,
                snr_db,
                data_rate: exchange.uplink.data_rate,
            },
            frequency_hz: exchange.uplink.frequency_hz,
            phy_payload: frame,
        };
        let mut payload = [0u8; FORWARD_OVERHEAD + MAX_FRAME];
        let len = forwarded.encode(&mut payload).map_err(RelayError::Frame)?;
        let due_us = ended_us + u64::from(RELAY_FWD_DELAY_US);
        self.held = Some(Held {
            payload,
            len,
            due_us,
            route: Route {
                uplink_end_us: ended_us,
                wor_frequency_hz: exchange.wor_frequency_hz,
                uplink_data_rate: exchange.uplink.data_rate,
            },
        });
        Ok(due_us)
    }

    /// Clears the uplink a wake-on-radio frame announced, once listening for it heard
    /// nothing.
    pub fn uplink_missed(&mut self) {
        self.exchange = None;
    }

    /// Returns when the forwarded uplink waiting to go out is due.
    ///
    /// # Returns
    ///
    /// The time, or `None` with nothing waiting.
    #[must_use]
    pub fn forward_due(&self) -> Option<u64> {
        self.held.map(|held| held.due_us)
    }

    /// Sends the uplink [`heard_uplink`](Relay::heard_uplink) holds, in the relay's own
    /// uplink on port [`LA_FPORT_RELAY`], TS011-1.0.1 section 9.1.
    ///
    /// It stays held when the relay's device cannot send yet, and when the answers the
    /// device owes crowd it out of this frame, which
    /// [`Transmission::carries_payload`] reports; call again once the device is free.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the time, in microseconds.
    ///
    /// # Returns
    ///
    /// What the relay's device transmits, with its receive windows.
    ///
    /// # Errors
    ///
    /// Returns [`RelayError::NothingHeld`] with nothing held, and [`RelayError::Device`] with
    /// [`DeviceError::Wait`] before it is due or whatever else keeps the device from
    /// sending.
    pub fn forward(&mut self, now_us: u64) -> Result<Transmission, RelayError> {
        let held = self.held.ok_or(RelayError::NothingHeld)?;
        if now_us < held.due_us {
            return Err(RelayError::Device(DeviceError::Wait {
                until_us: held.due_us,
            }));
        }
        let transmission =
            self.device
                .send(LA_FPORT_RELAY, &held.payload[..held.len], false, now_us)?;
        if transmission.carries_payload {
            self.held = None;
            self.awaiting = Some(held.route);
        }
        Ok(transmission)
    }

    /// Reads a frame heard in a given receive window of the relay's last transmission.
    ///
    /// Relay commands among its MAC commands reconfigure the relay, answered in order with
    /// the rest. A downlink on port [`LA_FPORT_RELAY`] answering a forwarded uplink becomes
    /// an [`RxrDownlink`] for the end device, TS011-1.0.1 section 9.2.
    ///
    /// # Arguments
    ///
    /// * `window` - the window the radio heard the frame in.
    /// * `frame` - the bytes the radio received.
    /// * `snr_db` - the frame's signal-to-noise ratio.
    ///
    /// # Returns
    ///
    /// What the frame was.
    ///
    /// # Errors
    ///
    /// Returns [`RelayError::Device`] for whatever the relay's device refuses, as
    /// [`EndDevice::heard_in`] does.
    pub fn heard_in(
        &mut self,
        window: ReceiveWindow,
        frame: &[u8],
        snr_db: i8,
    ) -> Result<RelayHeard, RelayError> {
        self.heard_frame(Some(window), frame, snr_db)
    }

    /// Reads a frame heard in one of the receive windows of the relay's last transmission,
    /// without saying which, as [`heard_in`](Relay::heard_in) otherwise does.
    ///
    /// # Arguments
    ///
    /// * `frame` - the bytes the radio received.
    /// * `snr_db` - the frame's signal-to-noise ratio.
    ///
    /// # Returns
    ///
    /// What the frame was.
    ///
    /// # Errors
    ///
    /// As [`heard_in`](Relay::heard_in).
    pub fn heard(&mut self, frame: &[u8], snr_db: i8) -> Result<RelayHeard, RelayError> {
        self.heard_frame(None, frame, snr_db)
    }

    /// Says what comes next once the relay's receive windows closed with nothing for it, as
    /// [`EndDevice::nothing_heard`] does. A forwarded uplink not repeated no longer waits on
    /// an answer.
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
    /// Returns [`RelayError::Device`] with [`DeviceError::NothingPending`] with no
    /// transmission waiting on its windows.
    pub fn nothing_heard(&mut self, now_us: u64) -> Result<Next, RelayError> {
        let next = self.device.nothing_heard(now_us)?;
        if !matches!(next, Next::Repeat { .. }) {
            self.awaiting = None;
        }
        Ok(next)
    }

    fn heard_frame(
        &mut self,
        window: Option<ReceiveWindow>,
        frame: &[u8],
        snr_db: i8,
    ) -> Result<RelayHeard, RelayError> {
        let heard = self
            .device
            .heard_frame(window, frame, snr_db, Some(&mut self.state))?;
        if self.state.config.is_none() {
            self.exchange = None;
        }
        let Heard::Data(delivery) = heard else {
            return Ok(RelayHeard::Device(heard));
        };
        if delivery.port() != Some(LA_FPORT_RELAY) {
            self.awaiting = None;
            return Ok(RelayHeard::Device(heard));
        }
        let Some(route) = self.awaiting.take() else {
            return Ok(RelayHeard::Undeliverable {
                delivery,
                reason: RelayError::NothingAwaited,
            });
        };
        Ok(match self.rxr(route, delivery.payload()) {
            Ok(downlink) => RelayHeard::Downlink { delivery, downlink },
            Err(reason) => RelayHeard::Undeliverable { delivery, reason },
        })
    }

    /// Builds the RXR downlink for a forwarded uplink's answer.
    fn rxr(&mut self, route: Route, payload: &[u8]) -> Result<RxrDownlink, RelayError> {
        let plan = self.device.plan();
        let data_rate =
            plan.rx1_data_rate(route.uplink_data_rate, 0)
                .ok_or(RelayError::Carrier(Carrier::new(
                    route.wor_frequency_hz,
                    route.uplink_data_rate,
                )))?;
        let carrier = Carrier::new(route.wor_frequency_hz, data_rate);
        let link = match plan
            .downlink_data_rate(data_rate)
            .map(|rate| rate.modulation)
        {
            Some(Modulation::LoRa {
                spreading_factor,
                bandwidth_hz,
            }) => LinkSettings::new(spreading_factor, bandwidth_hz),
            _ => return Err(RelayError::Carrier(carrier)),
        };
        if payload.len() < MIN_DATA_FRAME {
            return Err(RelayError::Frame(LorawanError::FrameTooShort));
        }
        let limit = plan
            .downlink_max_payload(data_rate, true)
            .map_or(MAX_FRAME, |limit| usize::from(limit.mac_payload) + 5);
        if payload.len() > limit.min(MAX_FRAME) {
            return Err(RelayError::Frame(LorawanError::PayloadTooLong));
        }
        let start_us = route.uplink_end_us + u64::from(RXR_DELAY_US);
        let free_at = self.device.free_at(carrier.frequency_hz);
        if free_at > start_us {
            return Err(RelayError::Device(DeviceError::Wait { until_us: free_at }));
        }
        let airtime_us = link.airtime_us(payload.len());
        self.device
            .record_air(start_us, airtime_us, carrier.frequency_hz);
        let mut frame = [0u8; MAX_FRAME];
        frame[..payload.len()].copy_from_slice(payload);
        Ok(RxrDownlink {
            frame,
            len: payload.len(),
            start_us,
            carrier,
            link,
            output_dbm: self.device.output_dbm(carrier.frequency_hz, 0),
            airtime_us,
        })
    }

    /// The LoRa settings of the uplink a WOR frame announced, if the relay can hear it.
    fn uplink_link(&self, carrier: Carrier) -> Result<LinkSettings, RelayError> {
        match self
            .device
            .plan()
            .uplink_data_rate(carrier.data_rate)
            .map(|rate| rate.modulation)
        {
            Some(Modulation::LoRa {
                spreading_factor,
                bandwidth_hz,
            }) if self.device.tunes(carrier.frequency_hz) => {
                Ok(LinkSettings::new(spreading_factor, bandwidth_hz))
            }
            _ => Err(RelayError::Carrier(carrier)),
        }
    }

    /// The longest end device frame the relay forwards at a data rate: what the end device
    /// may send behind a repeater, and what fits the relay's own uplink after the forwarded
    /// metadata, TS011-1.0.1 section 8.3.
    fn forwardable_len(&self, uplink: Carrier) -> Result<usize, RelayError> {
        let plan = self.device.plan();
        let device = plan
            .max_payload(uplink.data_rate, true)
            .map(|limit| usize::from(limit.mac_payload) + 5)
            .ok_or(RelayError::Carrier(uplink))?;
        let room = self.device.application_room(self.device.data_rate())?;
        Ok(device
            .min(room.saturating_sub(FORWARD_OVERHEAD))
            .min(MAX_FRAME))
    }

    /// What a WOR ACK tells a trusted device about forwarding its uplink.
    fn forward_status(&self, index: u8, now_us: u64) -> Forward {
        let Some(device) = self.state.trusted.get(index) else {
            return Forward::Disabled;
        };
        let limits = &self.state.limits;
        let buckets = [device.bucket(), &limits.global_uplink, &limits.overall];
        if buckets.iter().all(|bucket| bucket.has_token()) {
            return Forward::Available;
        }
        if buckets.iter().any(|bucket| bucket.disabled()) {
            return Forward::Disabled;
        }
        let into_hour = self
            .state
            .anchor_us
            .map_or(0, |anchor| now_us.saturating_sub(anchor) % RELOAD_PERIOD_US);
        if RELOAD_PERIOD_US - into_hour <= RELOAD_PERIOD_US / 2 {
            Forward::RetryIn30Minutes
        } else {
            Forward::RetryIn60Minutes
        }
    }

    /// Queues a notification of a device the relay does not trust, once while one for it
    /// is still to go out, TS011-1.0.1 section 8.7.
    fn notify(&mut self, dev_addr: u32, rssi_dbm: i16, snr_db: i8) -> Result<Wake, RelayError> {
        let addr = dev_addr.to_le_bytes();
        let key = [
            CID_NOTIFY_NEW_END_DEVICE,
            addr[0],
            addr[1],
            addr[2],
            addr[3],
        ];
        if self.device.starts_command(&key) {
            return Ok(Wake::Notified { dev_addr });
        }
        let limits = &mut self.state.limits;
        if !(limits.notify.has_token() && limits.overall.has_token()) {
            return Err(RelayError::Limited);
        }
        let queued = self
            .device
            .start_command(MacCommand::NotifyNewEndDeviceReq {
                dev_addr,
                rssi_dbm,
                snr_db,
            });
        if !queued {
            return Err(RelayError::Limited);
        }
        let limits = &mut self.state.limits;
        limits.notify.take();
        limits.overall.take();
        Ok(Wake::Notified { dev_addr })
    }
}

/// The first `base + k * period` at or after `now`.
fn next_multiple(base: u64, period: u64, now: u64) -> u64 {
    if now <= base {
        base
    } else {
        base + (now - base).div_ceil(period) * period
    }
}
