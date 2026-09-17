//! Sending through a relay: when the wake-on-radio frame goes out, on which channel, with
//! how long a preamble, and what the relay's answer changes, TS011-1.0.1 chapters 3 and 5.

use pamoja_lora::region::{Modulation, RelayChannel};
use pamoja_lora::LinkSettings;

use super::{DeviceError, EndDevice, Window};
use crate::relay::{
    open_wor_ack, wor_join_request, wor_uplink, CadPeriodicity, CadToRx, Carrier, Forward,
    RelayActivation, RelaySync, Synchronization, XtalAccuracy, FORWARD_OVERHEAD, RXR_DELAY_US,
    WOR_ACK_DELAY_US, WOR_ACK_LEN, WOR_ATTEMPTS_WO_ACK, WOR_DATA_DELAY_US, WOR_JOIN_REQUEST_LEN,
    WOR_UPLINK_LEN,
};

/// How many join attempts a device left to itself makes before one goes through a relay,
/// TS011-1.0.1 appendix 5.
const JOINS_PER_RELAYED: u32 = 4;

/// How many uplinks may go unanswered before a device left to itself starts using a relay,
/// TS011-1.0.1 appendix 5.
const QUIET_UPLINKS_TO_RELAY: u16 = 16;

/// The wake-on-radio frame an uplink goes out behind, and where it is heard, TS011-1.0.1
/// section 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WakeUp {
    frame: [u8; WOR_UPLINK_LEN],
    len: usize,
    /// When to start sending it, in the caller's microseconds.
    pub start_us: u64,
    /// Where and how fast.
    pub carrier: Carrier,
    /// The LoRa settings, with the preamble this frame needs: an explicit header and a
    /// payload CRC, sent with inverted IQ, as RP002-1.0.5 table 124 has.
    pub link: LinkSettings,
    /// The power to ask of the radio, the region's default.
    pub output_dbm: i8,
    /// How long it holds the air, in microseconds.
    pub airtime_us: u64,
}

impl WakeUp {
    /// Returns the frame to send.
    ///
    /// # Returns
    ///
    /// Five bytes ahead of a join request, fifteen ahead of an uplink.
    #[must_use]
    pub fn frame(&self) -> &[u8] {
        &self.frame[..self.len]
    }
}

/// When and where a relay's acknowledgment would arrive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AckWindow {
    /// When it starts, in the caller's microseconds.
    pub start_us: u64,
    /// Where and how fast: the channel's acknowledgment frequency, at the WOR frame's data
    /// rate.
    pub carrier: Carrier,
    /// The LoRa settings to listen with, RP002-1.0.5 table 125.
    pub link: LinkSettings,
    /// How long the acknowledgment lasts, in microseconds.
    pub airtime_us: u64,
}

/// The wake-on-radio exchange an uplink under a relay goes out behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelayExchange {
    /// The frame that wakes the relay.
    pub wake_up: WakeUp,
    /// Where the relay's acknowledgment would arrive, or `None` ahead of a join request,
    /// which TS011-1.0.1 section 3.3 has no relay acknowledge.
    pub ack: Option<AckWindow>,
    /// When the uplink itself goes out, in the caller's microseconds. It is the same
    /// whether or not the acknowledgment arrives, as section 3.7.2 asks.
    pub uplink_start_us: u64,
    /// The window a forwarded downlink arrives in, timed from the end of the uplink like
    /// the other two, RP002-1.0.5 table 126.
    pub rxr: Window,
}

/// What an end device does once its wake-on-radio frame went unanswered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorNext {
    /// Send the uplink at the time the exchange named anyway, TS011-1.0.1 section 3.4.
    Uplink,
    /// Wake the relay again first, as the network's `BackOff` asks.
    WakeUp(RelayExchange),
}

/// What a relay's last acknowledgment said about itself, TS011-1.0.1 table 14.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RelayStatus {
    /// How often it scans.
    pub cad_periodicity: CadPeriodicity,
    /// How accurate its crystal is.
    pub xtal_accuracy: XtalAccuracy,
    /// How long it takes to start receiving.
    pub cad_to_rx: CadToRx,
    /// The data rate it forwards at, which bounds what the device may send.
    pub relay_data_rate: u8,
    /// Whether it will forward.
    pub forward: Forward,
}

/// The wake-on-radio frame a device sent and is waiting on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sent {
    join: bool,
    started_us: u64,
    preamble_symbols: u16,
    wor: Carrier,
    ack: Carrier,
    uplink: Carrier,
    wfcnt: u32,
    second_channel: bool,
    symbol_us: u64,
}

/// What an end device keeps to send through a relay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Relayed {
    pub(crate) activation: RelayActivation,
    pub(crate) smart_level: u8,
    pub(crate) back_off: u8,
    pub(crate) second_channel: Option<RelayChannel>,
    pub(crate) enabled: bool,
    pub(crate) automatic: bool,
    pub(crate) wfcnt: u32,
    pub(crate) attempts: u8,
    pub(crate) quiet_uplinks: u16,
    pub(crate) channel: u8,
    pub(crate) fixed_channel: bool,
    pub(crate) heard: Option<RelayStatus>,
    sync: Option<Synchronization>,
    reference_second: bool,
    sent: Option<Sent>,
}

impl Relayed {
    pub(crate) const fn new() -> Relayed {
        Relayed {
            activation: RelayActivation::DeviceControlled,
            smart_level: 0,
            back_off: 0,
            second_channel: None,
            enabled: false,
            automatic: true,
            wfcnt: 0,
            attempts: 0,
            quiet_uplinks: 0,
            channel: 0,
            fixed_channel: false,
            heard: None,
            sync: None,
            reference_second: false,
            sent: None,
        }
    }

    /// What the device knows about when its relay listens.
    ///
    /// A device is out of the initialized state once a relay has answered it at all, on a
    /// wake-on-radio acknowledgment or a downlink in the relay window, which is also when
    /// it stops alternating the region's default channels, TS011-1.0.1 section 3.9.
    pub(crate) const fn sync(&self) -> RelaySync {
        match (
            self.sync.is_some(),
            self.heard.is_some() || self.fixed_channel,
        ) {
            (true, _) => RelaySync::Synchronized,
            (false, true) => RelaySync::Unsynchronized,
            (false, false) => RelaySync::Initialized,
        }
    }

    /// A new join request: the mode a network set lapses with the session, and a device
    /// left to itself tries a relay on one join in four, TS011-1.0.1 section 10.2 and
    /// appendix 5.
    pub(crate) fn joining(&mut self, joins: u32) {
        self.activation = RelayActivation::DeviceControlled;
        if self.automatic {
            self.enabled = joins % JOINS_PER_RELAYED == JOINS_PER_RELAYED - 1;
        }
        self.attempts = 0;
        self.sent = None;
    }

    /// A join accept arrived: the wake-on-radio counter restarts, and a device left to
    /// itself keeps the relay only if the accept came back through one.
    pub(crate) fn joined(&mut self, through_relay: bool) {
        self.wfcnt = 0;
        self.attempts = 0;
        self.quiet_uplinks = 0;
        self.sent = None;
        self.sync = None;
        self.fixed_channel = through_relay;
        if self.automatic {
            self.enabled = through_relay;
        }
    }

    /// A downlink arrived in the relay window, so the relay heard the device and it need no
    /// longer alternate channels.
    pub(crate) fn heard_on_rxr(&mut self) {
        self.fixed_channel = true;
    }

    /// Takes what an `EndDeviceConfReq` sets, TS011-1.0.1 section 10.2.
    pub(crate) fn configure(
        &mut self,
        activation: RelayActivation,
        smart_level: u8,
        back_off: u8,
        second_channel: Option<RelayChannel>,
    ) {
        self.activation = activation;
        self.automatic = activation == RelayActivation::DeviceControlled;
        self.smart_level = smart_level;
        self.back_off = back_off;
        self.second_channel = second_channel;
        match activation {
            RelayActivation::Disabled => {
                self.enabled = false;
                self.sent = None;
            }
            RelayActivation::Enabled => self.enabled = true,
            RelayActivation::Dynamic => self.quiet_uplinks = 0,
            RelayActivation::DeviceControlled => {}
        }
    }

    /// Forgets the timing, and with it the relay's settings once the frames keep going
    /// unanswered, TS011-1.0.1 section 3.9.
    fn lose_synchronization(&mut self) {
        if self.sync.take().is_none() {
            self.heard = None;
        }
    }

    /// Counts an uplink that went unanswered, and says whether a device left to itself
    /// should start using a relay, TS011-1.0.1 appendix 5 and table 41.
    pub(crate) fn uplink_unanswered(&mut self, smart_uplinks: u16) {
        self.quiet_uplinks = self.quiet_uplinks.saturating_add(1);
        let enough = match self.activation {
            RelayActivation::Dynamic => smart_uplinks,
            RelayActivation::DeviceControlled if self.automatic => QUIET_UPLINKS_TO_RELAY,
            _ => return,
        };
        if self.quiet_uplinks >= enough {
            self.enabled = true;
        }
    }

    /// Counts a downlink, which is what a device in dynamic mode is waiting for.
    pub(crate) fn heard_downlink(&mut self) {
        self.quiet_uplinks = 0;
    }
}

impl EndDevice<'_> {
    /// Returns how the device decides to use a relay, TS011-1.0.1 table 40.
    ///
    /// # Returns
    ///
    /// The mode, which the network sets with `EndDeviceConfReq` and which every join puts
    /// back to [`RelayActivation::DeviceControlled`].
    #[must_use]
    pub const fn relay_activation(&self) -> RelayActivation {
        self.relayed.activation
    }

    /// Reports whether the next uplink goes through a relay.
    ///
    /// # Returns
    ///
    /// `true` while relay mode is on.
    #[must_use]
    pub const fn relaying(&self) -> bool {
        self.relayed.enabled
    }

    /// Turns relay mode on or off, and takes the decision from the device itself.
    ///
    /// A device left to itself, which is where every device starts, decides this as
    /// TS011-1.0.1 appendix 5 recommends: it tries a relay on one join in four, keeps it
    /// only if the join accept comes back through one, turns it on again after sixteen
    /// unanswered uplinks, and gives it up after eight frames a relay never answered. From
    /// this call on, that is the caller's to decide, until the network takes it over with
    /// `EndDeviceConfReq` or hands it back with its end-device controlled mode. What a
    /// device knows about a relay it stops hearing is still given up either way.
    ///
    /// # Arguments
    ///
    /// * `on` - whether to send through a relay.
    ///
    /// # Returns
    ///
    /// `false` when the network has taken the decision with `EndDeviceConfReq`, leaving the
    /// mode as it was.
    ///
    /// # Examples
    ///
    /// A reading goes out behind a frame that wakes the relay:
    ///
    /// ```
    /// use pamoja_lora::region::Region;
    /// use pamoja_lorawan::device::{EndDevice, Settings, WorNext};
    /// use pamoja_lorawan::relay::Carrier;
    /// use pamoja_lorawan::Session;
    ///
    /// let session = Session::new(0x2601_1BDA, [0x2B; 16], [0x99; 16]);
    /// let mut device = EndDevice::personalized(Region::Eu868.plan(), session, Settings::new(2, 14))?;
    /// assert!(device.use_relay(true));
    ///
    /// // The wake-on-radio frame goes out on the region's first relay channel, ahead of
    /// // the uplink it names.
    /// let reading = device.send(2, b"21.5", false, 0)?;
    /// let exchange = reading.relay.expect("relay mode is on");
    /// assert_eq!(exchange.wake_up.carrier, Carrier::new(865_100_000, 3));
    /// assert_eq!(exchange.wake_up.start_us, 0);
    ///
    /// // A device that has heard from no relay covers a whole second of scanning, and the
    /// // uplink follows once the acknowledgment would have come and gone.
    /// assert_eq!(exchange.wake_up.link.preamble_symbols(), 259);
    /// let ack = exchange.ack.expect("an uplink is acknowledged");
    /// assert_eq!(exchange.uplink_start_us, ack.start_us + ack.airtime_us + 50_000);
    ///
    /// // With nothing in the acknowledgment window, the uplink goes out anyway.
    /// assert_eq!(device.no_wor_ack(ack.start_us + ack.airtime_us)?, WorNext::Uplink);
    /// # Ok::<(), pamoja_lorawan::device::DeviceError>(())
    /// ```
    pub fn use_relay(&mut self, on: bool) -> bool {
        match self.relayed.activation {
            RelayActivation::Dynamic | RelayActivation::DeviceControlled => {
                self.relayed.enabled = on;
                self.relayed.automatic = false;
                if !on {
                    self.relayed.sent = None;
                }
                true
            }
            _ => false,
        }
    }

    /// Returns what the device knows about when its relay listens, TS011-1.0.1 section 3.9.
    ///
    /// # Returns
    ///
    /// The synchronization state.
    #[must_use]
    pub const fn relay_sync(&self) -> RelaySync {
        self.relayed.sync()
    }

    /// Returns what the relay's last acknowledgment said about itself.
    ///
    /// # Returns
    ///
    /// Its scan period, crystal, switching time, forwarding data rate and whether it
    /// forwards, or `None` before one has arrived.
    #[must_use]
    pub const fn relay_status(&self) -> Option<RelayStatus> {
        self.relayed.heard
    }

    /// Returns the wake-on-radio frame counter, which every frame raises and a join accept
    /// resets, TS011-1.0.1 section 5.3.2.
    ///
    /// # Returns
    ///
    /// The counter the next frame will use.
    #[must_use]
    pub const fn wor_counter(&self) -> u32 {
        self.relayed.wfcnt
    }

    /// Reads the acknowledgment a relay answered the last wake-on-radio frame with.
    ///
    /// The device is now synchronized: it knows when the relay scans, so its next frames
    /// carry only as much preamble as the two crystals could drift, and it holds its
    /// payloads to what the relay forwards.
    ///
    /// # Arguments
    ///
    /// * `frame` - the bytes the radio received in the acknowledgment window.
    ///
    /// # Returns
    ///
    /// What the relay said about itself.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NothingPending`] with no wake-on-radio frame waiting on an
    /// answer, [`DeviceError::NotJoined`] before the device has a session, and
    /// [`DeviceError::Frame`] for an acknowledgment that does not decode or verify.
    pub fn heard_wor_ack(&mut self, frame: &[u8]) -> Result<RelayStatus, DeviceError> {
        let session = self.session.ok_or(DeviceError::NotJoined)?;
        let sent = self.relayed.sent.ok_or(DeviceError::NothingPending)?;
        let state = open_wor_ack(
            frame,
            &session.wor_keys(),
            session.dev_addr(),
            sent.wfcnt,
            sent.ack,
            sent.uplink,
        )?;
        self.relayed.sent = None;
        self.relayed.attempts = 0;
        self.relayed.sync = Some(Synchronization::from_ack(
            sent.started_us,
            sent.preamble_symbols,
            sent.symbol_us,
            &state,
        ));
        self.relayed.reference_second = sent.second_channel;
        self.relayed.fixed_channel = true;
        let heard = RelayStatus {
            cad_periodicity: state.cad_periodicity,
            xtal_accuracy: state.xtal_accuracy,
            cad_to_rx: state.cad_to_rx,
            relay_data_rate: state.relay_data_rate,
            forward: state.forward,
        };
        self.relayed.heard = Some(heard);
        Ok(heard)
    }

    /// Says what to do once the acknowledgment window closed with nothing in it.
    ///
    /// The uplink goes out anyway unless the network's `BackOff` asks for more attempts
    /// first. After [`WOR_ATTEMPTS_WO_ACK`] frames without an answer the device gives up
    /// what it knows: first when the relay scans, then the relay altogether.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the time, in microseconds.
    ///
    /// # Returns
    ///
    /// Whether to send the uplink or wake the relay again.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NothingPending`] with no wake-on-radio frame waiting on an
    /// answer, and whatever building another frame returns.
    pub fn no_wor_ack(&mut self, now_us: u64) -> Result<WorNext, DeviceError> {
        let sent = self.relayed.sent.ok_or(DeviceError::NothingPending)?;
        self.relayed.sent = None;
        self.relayed.attempts = self.relayed.attempts.saturating_add(1);
        if self.relayed.attempts >= WOR_ATTEMPTS_WO_ACK {
            self.relayed.attempts = 0;
            self.relayed.lose_synchronization();
            if self.relayed.automatic
                && self.relayed.activation == RelayActivation::DeviceControlled
            {
                // TS011-1.0.1 appendix 5: a device left to itself stops using a relay that
                // never answers, and counts its unanswered uplinks afresh before trying
                // another one.
                self.relayed.enabled = false;
                self.relayed.quiet_uplinks = 0;
                return Ok(WorNext::Uplink);
            }
        }
        // The next frame alternates the region's default channels while the device has no
        // relay of its own to aim at, TS011-1.0.1 section 3.2.2.
        if !self.relayed.fixed_channel {
            self.relayed.channel = self.relayed.channel.wrapping_add(1);
        }
        if self.relayed.back_off != 0
            && !self.relayed.attempts.is_multiple_of(self.relayed.back_off)
        {
            match self.wor_exchange(sent.uplink, sent.join, now_us)? {
                Some(exchange) => return Ok(WorNext::WakeUp(exchange)),
                None => return Ok(WorNext::Uplink),
            }
        }
        Ok(WorNext::Uplink)
    }

    /// Builds the wake-on-radio exchange an uplink goes out behind, and records what it
    /// costs the device's duty cycle.
    pub(super) fn wor_exchange(
        &mut self,
        uplink: Carrier,
        join: bool,
        now_us: u64,
    ) -> Result<Option<RelayExchange>, DeviceError> {
        if !self.relayed.enabled {
            return Ok(None);
        }
        let Some(channel) = self.wor_channel() else {
            return Ok(None);
        };
        let link = self.wor_link(channel.data_rate)?;
        let symbol_us = link.symbol_time_us();
        let earliest = now_us.max(self.free_at(channel.wor_frequency_hz));
        let second_channel = self.relayed.second_channel.is_some();

        let (start_us, preamble_symbols) = match self.relayed.sync {
            Some(sync) => {
                let other = second_channel != self.relayed.reference_second;
                match sync.next_wor(earliest, self.settings.crystal_ppm, symbol_us, other) {
                    Some(slot) => (slot.start_us, slot.preamble_symbols),
                    None => {
                        self.relayed.sync = None;
                        (earliest, self.unsynchronized_preamble(symbol_us))
                    }
                }
            }
            None => (earliest, self.unsynchronized_preamble(symbol_us)),
        };

        let wor = Carrier::new(channel.wor_frequency_hz, channel.data_rate);
        let ack_carrier = Carrier::new(channel.ack_frequency_hz, channel.data_rate);
        let mut frame = [0u8; WOR_UPLINK_LEN];
        let (len, wfcnt) = if join {
            frame[..WOR_JOIN_REQUEST_LEN].copy_from_slice(&wor_join_request(uplink)?);
            (WOR_JOIN_REQUEST_LEN, 0)
        } else {
            let session = self.session.ok_or(DeviceError::NotJoined)?;
            let wfcnt = self.relayed.wfcnt;
            frame = wor_uplink(&session.wor_keys(), session.dev_addr(), wfcnt, uplink, wor)?;
            self.relayed.wfcnt = wfcnt.saturating_add(1);
            (WOR_UPLINK_LEN, wfcnt)
        };

        let link = link.with_preamble(preamble_symbols);
        let airtime_us = link.airtime_us(len);
        let ended_us = start_us + airtime_us;
        let ack_link = self.wor_link(channel.data_rate)?;
        let ack_airtime_us = ack_link.airtime_us(WOR_ACK_LEN);
        let (ack, uplink_start_us) = if join {
            (None, ended_us + u64::from(WOR_DATA_DELAY_US))
        } else {
            let start_us = ended_us + u64::from(WOR_ACK_DELAY_US);
            (
                Some(AckWindow {
                    start_us,
                    carrier: ack_carrier,
                    link: ack_link,
                    airtime_us: ack_airtime_us,
                }),
                start_us + ack_airtime_us + u64::from(WOR_DATA_DELAY_US),
            )
        };

        let rxr_data_rate = self
            .plan
            .rx1_data_rate(uplink.data_rate, 0)
            .ok_or(DeviceError::DataRate(uplink.data_rate))?;
        let rxr = Window {
            delay_us: RXR_DELAY_US,
            frequency_hz: channel.wor_frequency_hz,
            data_rate: rxr_data_rate,
            link: self.wor_link(rxr_data_rate)?,
        };

        self.record_air(start_us, airtime_us, channel.wor_frequency_hz);
        self.relayed.sent = Some(Sent {
            join,
            started_us: start_us,
            preamble_symbols,
            wor,
            ack: ack_carrier,
            uplink,
            wfcnt,
            second_channel,
            symbol_us,
        });
        Ok(Some(RelayExchange {
            wake_up: WakeUp {
                frame,
                len,
                start_us,
                carrier: wor,
                link,
                output_dbm: self.output_dbm(channel.wor_frequency_hz, 0),
                airtime_us,
            },
            ack,
            uplink_start_us,
            rxr,
        }))
    }

    /// The channel the next wake-on-radio frame goes out on: the second channel a network
    /// set, or the region's default channels in turn, TS011-1.0.1 sections 3.2.2 and 3.2.3.
    fn wor_channel(&self) -> Option<RelayChannel> {
        if let Some(second) = self.relayed.second_channel {
            return Some(second);
        }
        let count = self.plan.relay_channels.len();
        if count == 0 {
            return None;
        }
        self.plan.relay_channel(self.relayed.channel % count as u8)
    }

    /// The preamble of a device that cannot predict the relay's next scan: one scan period
    /// of it, or a whole second before any acknowledgment has arrived, TS011-1.0.1
    /// section 5.2.
    fn unsynchronized_preamble(&self, symbol_us: u64) -> u16 {
        let (period, cad_to_rx) = match self.relayed.heard {
            Some(heard) => (heard.cad_periodicity, heard.cad_to_rx),
            None => (CadPeriodicity::Ms1000, CadToRx::Symbols8),
        };
        crate::relay::unsynchronized_preamble_symbols(period, symbol_us, cad_to_rx)
    }

    /// The LoRa settings of a data rate the downlink table numbers, with the payload CRC
    /// every relay frame carries and a receive window does not, RP002-1.0.5 tables 124, 125
    /// and 127.
    fn wor_link(&self, data_rate: u8) -> Result<LinkSettings, DeviceError> {
        match self
            .plan
            .downlink_data_rate(data_rate)
            .map(|rate| rate.modulation)
        {
            Some(Modulation::LoRa {
                spreading_factor,
                bandwidth_hz,
            }) => Ok(LinkSettings::new(spreading_factor, bandwidth_hz)),
            _ => Err(DeviceError::DataRate(data_rate)),
        }
    }

    /// What a relay's forwarding leaves for the application payload: its own frame at the
    /// data rate the relay forwards at, less the forwarded metadata and the end device's
    /// own header, TS011-1.0.1 section 8.3.
    pub(super) fn relayed_room(&self) -> Option<usize> {
        let heard = self.relayed.heard?;
        let carried = self
            .plan
            .max_payload(heard.relay_data_rate, false)
            .map(|limit| usize::from(limit.application))?;
        Some(carried.saturating_sub(FORWARD_OVERHEAD + 13))
    }
}
