//! The network side of a single site: what a server does with what a gateway forwarded.
//!
//! A gateway hands over packets without reading them, because it holds no keys. Deciding
//! what a packet is, admitting the device that sent it, decrypting what it carries, and
//! working out when and where to answer is the network server's work, and this module does
//! that much of it for one site: a [`Network`] holds the devices it admits, the sessions it
//! has granted, and the counters it has seen, and turns an [`Rxpk`] into an [`Event`].
//!
//! It runs no sockets and keeps no clock. An uplink goes in as the gateway reported it, and
//! a downlink comes back as a [`Txpk`] ready for a `PULL_RESP`, timed in the concentrator's
//! own microseconds, so the caller owns every decision about the wire.
//!
//! The windows come from the published parameters rather than from habit. RP002-1.0.5
//! section 3.3 gives the delays that are recommended for every region, TS001-1.0.4 gives the
//! layout of the two bytes a join accept carries, and the channel a first window answers on
//! is regional: by default the plan decides it, and [`Rx1Channels`] can override that.
//!
//! # Examples
//!
//! A device joins, sends a reading, and is answered in its first receive window.
//!
//! ```
//! use pamoja_gateway::network::{Event, Network, Registration};
//! use pamoja_gateway::udp::Rxpk;
//! use pamoja_lora::region::Region;
//! use pamoja_lora::LinkSettings;
//! use pamoja_lorawan::Device;
//!
//! let mut network = Network::new(Region::Eu868.plan(), 0x00_00_2A);
//! let device = Device::new([1; 8], [2; 8], [3; 16]);
//! network.register(Registration::new([1; 8], [2; 8], [3; 16]));
//!
//! // The gateway forwards the join request it heard.
//! let link = LinkSettings::new(7, 125_000);
//! let request = device.join_request(0x1234);
//! let heard = Rxpk::new(868_100_000, link, request.as_bytes().to_vec()).with_timestamp_us(1_000);
//! let joined = network.uplink(&heard).expect("the request verifies");
//! assert!(matches!(joined, Event::Joined { .. }));
//! ```

use pamoja_lora::region::{ChannelBlock, ChannelPlan, OwnedChannelPlan};
use pamoja_lora::LinkSettings;
use pamoja_lorawan::mac::{encode_all, MacCommand, MacCommands, FOPTS_MAX, MAX_COMMAND};
use pamoja_lorawan::relay::{ForwardedUplink, UplinkMetadata, LA_FPORT_RELAY};
use pamoja_lorawan::{
    Direction, Downlink, FrameHeader, JoinGrant, JoinRequest, LorawanError, MessageType, Session,
};

use crate::udp::{Rxpk, Txpk};

// The delays a window opens after and the counter gap a session follows are the ones every
// region shares, kept with the rest of the link layer so a device and this network read the
// same values.
pub use pamoja_lorawan::defaults::{
    JOIN_ACCEPT_DELAY1_US, JOIN_ACCEPT_DELAY2_US, MAX_FCNT_GAP, RECEIVE_DELAY1_US,
    RECEIVE_DELAY2_US,
};

/// The channel the first receive window answers on, which the region decides.
///
/// Most regions answer on the frequency the uplink arrived on (RP002-1.0.5 section 3.4.7 for
/// EU863-870, and the same wording for EU433, AS923, KR920-923, IN865 and RU864-870). The
/// fixed plans instead answer on numbered downlink channels, choosing one by the uplink
/// channel number: "RX1 Channel Number = Transmit Channel Number modulo NbChannel" (sections
/// 3.5.7 and 3.8.7, and section 3.9.7 for CN470-510). The channel plan carries which, so
/// [`Rx1Channels::Plan`] is the default, and the other two override it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rx1Channels {
    /// The window answers where the network's channel plan says: on the uplink's frequency in
    /// a dynamic plan, and on the downlink channel a fixed plan numbers for the uplink
    /// channel. A join on a CN470-510 common join channel is answered on the frequency
    /// RP002-1.0.5 table 49 gives it.
    Plan,
    /// The window answers on the frequency the uplink arrived on.
    SameAsUplink,
    /// The window answers on a run of downlink channels, indexed by the uplink channel
    /// number modulo how many the run holds.
    Downstream(ChannelBlock),
}

impl Rx1Channels {
    /// Returns the downstream channels US902-928 answers on.
    ///
    /// Eight channels of 500 kHz starting at 923.3 MHz and stepping 600 kHz to 927.5 MHz,
    /// carrying downlink data rates DR8 to DR13, from RP002-1.0.5 section 3.5.2.
    ///
    /// # Returns
    ///
    /// The channels.
    pub const fn us915() -> Rx1Channels {
        Rx1Channels::Downstream(ChannelBlock::new(923_300_000, 600_000, 8, 8, 13))
    }

    /// Returns the downstream channels AU915-928 answers on, which are the same run.
    ///
    /// # Returns
    ///
    /// The channels.
    pub const fn au915() -> Rx1Channels {
        Rx1Channels::Downstream(ChannelBlock::new(923_300_000, 600_000, 8, 8, 13))
    }
}

/// When and where a network answers, and at what rate.
///
/// The defaults are the values recommended for every region in RP002-1.0.5 section 3.3: one
/// second to the first receive window, five to the first join window, and no offset between
/// the uplink data rate and the downlink one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Windows {
    receive_delay_us: u32,
    join_delay_us: u32,
    rx1_data_rate_offset: u8,
    rx1_channels: Rx1Channels,
}

impl Windows {
    /// Returns the recommended windows: one second, five seconds, no offset, answering where
    /// the channel plan says.
    ///
    /// # Returns
    ///
    /// The windows.
    pub const fn new() -> Windows {
        Windows {
            receive_delay_us: RECEIVE_DELAY1_US,
            join_delay_us: JOIN_ACCEPT_DELAY1_US,
            rx1_data_rate_offset: 0,
            rx1_channels: Rx1Channels::Plan,
        }
    }

    /// Sets the delay before the first receive window, in microseconds.
    ///
    /// # Arguments
    ///
    /// * `micros` - the delay.
    ///
    /// # Returns
    ///
    /// The updated windows, for chaining.
    pub const fn with_receive_delay_us(mut self, micros: u32) -> Windows {
        self.receive_delay_us = micros;
        self
    }

    /// Sets the delay before the window a join accept is sent in, in microseconds.
    ///
    /// # Arguments
    ///
    /// * `micros` - the delay.
    ///
    /// # Returns
    ///
    /// The updated windows, for chaining.
    pub const fn with_join_delay_us(mut self, micros: u32) -> Windows {
        self.join_delay_us = micros;
        self
    }

    /// Sets the offset between the uplink data rate and the one the first window answers at.
    ///
    /// # Arguments
    ///
    /// * `offset` - the RX1DROffset the plan allows.
    ///
    /// # Returns
    ///
    /// The updated windows, for chaining.
    pub const fn with_rx1_data_rate_offset(mut self, offset: u8) -> Windows {
        self.rx1_data_rate_offset = offset;
        self
    }

    /// Sets the channels the first window answers on.
    ///
    /// # Arguments
    ///
    /// * `channels` - the regional rule.
    ///
    /// # Returns
    ///
    /// The updated windows, for chaining.
    pub const fn with_rx1_channels(mut self, channels: Rx1Channels) -> Windows {
        self.rx1_channels = channels;
        self
    }

    /// Returns the delay before the first receive window, in microseconds.
    ///
    /// # Returns
    ///
    /// The delay.
    pub const fn receive_delay_us(&self) -> u32 {
        self.receive_delay_us
    }

    /// Returns the delay before the join accept window, in microseconds.
    ///
    /// # Returns
    ///
    /// The delay.
    pub const fn join_delay_us(&self) -> u32 {
        self.join_delay_us
    }

    /// Returns the offset the first window answers at.
    ///
    /// # Returns
    ///
    /// The offset.
    pub const fn rx1_data_rate_offset(&self) -> u8 {
        self.rx1_data_rate_offset
    }

    /// Returns the channels the first window answers on.
    ///
    /// # Returns
    ///
    /// The regional rule.
    pub const fn rx1_channels(&self) -> Rx1Channels {
        self.rx1_channels
    }

    /// Returns the `RXDelay` byte a join accept carries.
    ///
    /// TS001-1.0.4 table 44 puts the delay in the low four bits, in seconds, and states that
    /// a zero there means one second, so a sub-second delay cannot be expressed and rounds
    /// down to the same one second the device already assumes.
    ///
    /// # Returns
    ///
    /// The byte.
    pub const fn rx_delay_byte(&self) -> u8 {
        let seconds = self.receive_delay_us / 1_000_000;
        if seconds > 15 {
            return 15;
        }
        seconds as u8
    }

    /// Returns the `DLSettings` byte a join accept carries.
    ///
    /// TS001-1.0.4 table 55 lays it out as a reserved top bit, the RX1DROffset in bits 6 to
    /// 4, and the second window's data rate in the low four bits.
    ///
    /// # Arguments
    ///
    /// * `rx2_data_rate` - the data rate the second window listens at, from the plan.
    ///
    /// # Returns
    ///
    /// The byte.
    pub const fn dl_settings_byte(&self, rx2_data_rate: u8) -> u8 {
        ((self.rx1_data_rate_offset & 0x07) << 4) | (rx2_data_rate & 0x0F)
    }
}

impl Default for Windows {
    fn default() -> Windows {
        Windows::new()
    }
}

/// A device the network admits, and the key it was provisioned with.
#[derive(Clone, Copy)]
pub struct Registration {
    dev_eui: [u8; 8],
    app_eui: [u8; 8],
    app_key: [u8; 16],
}

impl Registration {
    /// Registers a device by its identifiers and its root key.
    ///
    /// # Arguments
    ///
    /// * `dev_eui` - the device's identifier.
    /// * `app_eui` - the application identifier it joins under.
    /// * `app_key` - the root key it was provisioned with.
    ///
    /// # Returns
    ///
    /// The registration.
    pub const fn new(dev_eui: [u8; 8], app_eui: [u8; 8], app_key: [u8; 16]) -> Registration {
        Registration {
            dev_eui,
            app_eui,
            app_key,
        }
    }

    /// Returns the device's identifier.
    ///
    /// # Returns
    ///
    /// The DevEUI.
    pub const fn dev_eui(&self) -> [u8; 8] {
        self.dev_eui
    }

    /// Returns the application identifier the device joins under.
    ///
    /// # Returns
    ///
    /// The AppEUI.
    pub const fn app_eui(&self) -> [u8; 8] {
        self.app_eui
    }
}

// The root key never reaches a log, a panic message, or a test failure through this type.
impl core::fmt::Debug for Registration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Registration")
            .field("dev_eui", &self.dev_eui)
            .field("app_eui", &self.app_eui)
            .finish_non_exhaustive()
    }
}

/// What a relay added to an uplink it forwarded, TS011-1.0.1 section 9.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relayed {
    /// The relay's own address.
    pub relay: u32,
    /// What it heard of the uplink.
    pub metadata: UplinkMetadata,
    /// The frequency the uplink arrived on, in hertz.
    pub frequency_hz: u32,
}

/// A device a relay heard and could not verify, TS011-1.0.1 section 10.7.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Notice {
    /// The relay that heard it.
    pub relay: u32,
    /// The address the wake-on-radio frame named.
    pub dev_addr: u32,
    /// The frame's signal strength in dBm.
    pub rssi_dbm: i16,
    /// Its signal-to-noise ratio in dB.
    pub snr_db: i8,
}

/// Where and when a downlink answers an uplink, in the concentrator's own terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    /// The concentrator timestamp to transmit at, in microseconds.
    pub timestamp_us: u32,
    /// The frequency to transmit on, in hertz.
    pub frequency_hz: u32,
    /// The settings to transmit with.
    pub link: LinkSettings,
}

/// What a forwarded packet turned out to be.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A device joined, and the accept is ready to transmit.
    Joined {
        /// The device that joined.
        dev_eui: [u8; 8],
        /// The address it was granted.
        dev_addr: u32,
        /// The accept, timed for the join window.
        accept: Txpk,
    },
    /// A session frame arrived, decrypted.
    Data {
        /// The address it came from.
        dev_addr: u32,
        /// The counter it carried, reconstructed to its full width.
        fcnt: u32,
        /// The port it was sent on, absent for a frame carrying only MAC options.
        fport: Option<u8>,
        /// What the device sent.
        payload: Vec<u8>,
        /// Whether the device asked to be acknowledged.
        confirmed: bool,
        /// Where an answer would go.
        slot: Slot,
        /// The relay that forwarded it, and what it heard, for an uplink that came that
        /// way. [`answer`](Network::answer) sends the reply back through the same relay.
        relay: Option<Relayed>,
    },
    /// A data frame for an address this network has not granted, which is another
    /// network's traffic and is not an error.
    Foreign {
        /// The address the frame carried.
        dev_addr: u32,
    },
}

/// What can go wrong admitting or reading a forwarded packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkError {
    /// A relay forwarded something that is not an uplink it could have heard.
    Relayed(LorawanError),
    /// The packet was not LoRa, so it carries no LoRaWAN frame here.
    NotLora,
    /// The frame did not parse, verify, or decrypt.
    Frame(LorawanError),
    /// A join request that no registered key verifies.
    UnknownDevice,
    /// A frame at a counter already seen, which is a replay.
    Replayed {
        /// The address it claimed.
        dev_addr: u32,
        /// The counter it carried.
        fcnt: u32,
    },
    /// A frame further ahead of the counter last seen than [`MAX_FCNT_GAP`] allows.
    CounterGap {
        /// The address it claimed.
        dev_addr: u32,
        /// The counter last accepted.
        seen: u32,
        /// The counter it carried.
        carried: u32,
    },
    /// The plan names no data rate for the settings the packet arrived at, so the rate the
    /// first window answers at cannot be worked out.
    UnknownDataRate,
    /// No first receive window exists for the packet: its frequency is not a channel the
    /// plan defines, or the plan names no downlink rate at that offset.
    NoWindow,
    /// A downlink for an address this network holds no session for.
    NoSession {
        /// The address asked for.
        dev_addr: u32,
    },
}

impl core::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NetworkError::NotLora => f.write_str("the packet was not LoRa"),
            NetworkError::Frame(error) => write!(f, "the frame was refused: {error}"),
            NetworkError::Relayed(error) => {
                write!(f, "the frame a relay forwarded was refused: {error}")
            }
            NetworkError::UnknownDevice => {
                f.write_str("no registered key verifies the join request")
            }
            NetworkError::Replayed { dev_addr, fcnt } => {
                write!(f, "frame {fcnt} from {dev_addr:#010x} was already seen")
            }
            NetworkError::CounterGap {
                dev_addr,
                seen,
                carried,
            } => write!(
                f,
                "frame {carried} from {dev_addr:#010x} runs too far ahead of {seen}"
            ),
            NetworkError::UnknownDataRate => {
                f.write_str("the plan names no data rate for the settings heard")
            }
            NetworkError::NoWindow => f.write_str("the plan defines no first receive window"),
            NetworkError::NoSession { dev_addr } => {
                write!(f, "no session for {dev_addr:#010x}")
            }
        }
    }
}

impl core::error::Error for NetworkError {}

// A run of common join channels and where a join on each is answered, kept apart from the
// plan because an owned copy of a plan does not carry the plans a join selects between.
struct JoinAccepts {
    channels: ChannelBlock,
    start_hz: u32,
    step_hz: u32,
}

// A device that has joined: the session that was granted, and the counters seen since.
struct Admitted {
    dev_eui: [u8; 8],
    dev_addr: u32,
    session: Session,
    fcnt_up: Option<u32>,
    fcnt_down: u32,
    via_relay: Option<u32>,
}

/// The network side of one site.
///
/// It admits the devices it is told about, grants sessions, follows frame counters, and
/// decides where an answer goes. Everything it needs about the band comes from the
/// [`ChannelPlan`] it is built with, so the same type serves any region.
pub struct Network {
    plan: OwnedChannelPlan,
    join_accepts: Vec<JoinAccepts>,
    windows: Windows,
    net_id: u32,
    registrations: Vec<Registration>,
    admitted: Vec<Admitted>,
    next_dev_addr: u32,
    next_app_nonce: u32,
    notices: Vec<Notice>,
}

impl Network {
    /// Builds a network on a channel plan.
    ///
    /// The plan is copied into the network, so one holds its band for as long as it runs
    /// rather than borrowing a table that lives somewhere else.
    ///
    /// # Arguments
    ///
    /// * `plan` - the band this site operates in.
    /// * `net_id` - the network identifier granted addresses carry; only its low 24 bits
    ///   travel.
    ///
    /// # Returns
    ///
    /// The network, with the recommended windows and no devices registered.
    pub fn new(plan: &ChannelPlan<'_>, net_id: u32) -> Network {
        Network {
            plan: OwnedChannelPlan::from_plan(plan),
            join_accepts: plan
                .join_plans
                .iter()
                .map(|run| JoinAccepts {
                    channels: run.channels,
                    start_hz: run.accept_start_hz,
                    step_hz: run.accept_step_hz,
                })
                .collect(),
            windows: Windows::new(),
            net_id,
            registrations: Vec::new(),
            admitted: Vec::new(),
            next_dev_addr: 1,
            next_app_nonce: 1,
            notices: Vec::new(),
        }
    }

    /// Sets the windows this network answers in.
    ///
    /// # Arguments
    ///
    /// * `windows` - the delays, offset, and channels.
    ///
    /// # Returns
    ///
    /// The updated network, for chaining.
    pub fn with_windows(mut self, windows: Windows) -> Network {
        self.windows = windows;
        self
    }

    /// Sets the first address this network grants; later joins take the ones after it.
    ///
    /// # Arguments
    ///
    /// * `dev_addr` - the address to grant next.
    ///
    /// # Returns
    ///
    /// The updated network, for chaining.
    pub fn with_first_dev_addr(mut self, dev_addr: u32) -> Network {
        self.next_dev_addr = dev_addr;
        self
    }

    /// Admits a device, so a join request signed with its key is accepted.
    ///
    /// # Arguments
    ///
    /// * `registration` - the device and its root key.
    pub fn register(&mut self, registration: Registration) {
        self.registrations.push(registration);
    }

    /// Returns the windows this network answers in.
    ///
    /// # Returns
    ///
    /// The windows.
    pub const fn windows(&self) -> Windows {
        self.windows
    }

    /// Returns the session granted to an address, once a device has joined.
    ///
    /// # Arguments
    ///
    /// * `dev_addr` - the address.
    ///
    /// # Returns
    ///
    /// The session, or `None` if no device holds that address here.
    pub fn session(&self, dev_addr: u32) -> Option<Session> {
        self.admitted
            .iter()
            .find(|held| held.dev_addr == dev_addr)
            .map(|held| held.session)
    }

    /// Reads a packet the gateway forwarded.
    ///
    /// A join request is verified against every registered key, granted an address and a
    /// session, and answered with an accept timed for the join window. A data frame is
    /// routed by its address, checked against the counter last seen, and decrypted. A frame
    /// for an address this network has not granted is reported rather than refused, because
    /// a gateway hears every network in range.
    ///
    /// # Arguments
    ///
    /// * `heard` - the packet as the gateway reported it.
    ///
    /// # Returns
    ///
    /// What the packet turned out to be.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::NotLora`] for an FSK packet, [`NetworkError::Frame`] if the
    /// frame does not parse or its MIC does not verify, [`NetworkError::UnknownDevice`] if
    /// no registered key verifies a join request, [`NetworkError::Replayed`] or
    /// [`NetworkError::CounterGap`] if the counter is not one this network will follow, and
    /// [`NetworkError::UnknownDataRate`] or [`NetworkError::NoWindow`] if the plan does not
    /// describe where to answer.
    pub fn uplink(&mut self, heard: &Rxpk) -> Result<Event, NetworkError> {
        let link = heard.modulation.link().ok_or(NetworkError::NotLora)?;
        let header = FrameHeader::parse(&heard.payload).map_err(NetworkError::Frame)?;

        match header.message_type() {
            MessageType::JoinRequest => self.admit(heard, link),
            MessageType::UnconfirmedUp | MessageType::ConfirmedUp => {
                self.receive(heard, link, &header)
            }
            other => Err(NetworkError::Frame(LorawanError::UnsupportedMType(
                downlink_mtype(other),
            ))),
        }
    }

    /// Builds a downlink for a device, encrypted with its session.
    ///
    /// # Arguments
    ///
    /// * `dev_addr` - the device to answer.
    /// * `slot` - where and when to transmit, from the event that reported the uplink.
    /// * `fport` - the port to answer on.
    /// * `payload` - what to send.
    ///
    /// # Returns
    ///
    /// The packet to put in a `PULL_RESP`, with the inverted polarity a device listens for.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::NoSession`] if no device holds that address here, or
    /// [`NetworkError::Frame`] if the payload does not fit a single frame.
    pub fn answer(
        &mut self,
        dev_addr: u32,
        slot: Slot,
        fport: u8,
        payload: &[u8],
    ) -> Result<Txpk, NetworkError> {
        let held = self
            .admitted
            .iter_mut()
            .find(|held| held.dev_addr == dev_addr)
            .ok_or(NetworkError::NoSession { dev_addr })?;

        let frame = held
            .session
            .encode_downlink(&Downlink::new(held.fcnt_down, fport, payload))
            .map_err(NetworkError::Frame)?;
        held.fcnt_down = held.fcnt_down.wrapping_add(1);
        let relay = held.via_relay;

        let frame = match relay {
            Some(relay) => self.wrap_for_relay(relay, frame.as_bytes())?,
            None => frame.as_bytes().to_vec(),
        };
        Ok(transmit(slot, frame))
    }

    /// Builds a downlink carrying MAC commands, which is how a network configures a relay,
    /// TS011-1.0.1 chapter 10.
    ///
    /// The commands ride in the frame options where they fit, and in a frame of their own on
    /// port 0 where they do not.
    ///
    /// # Arguments
    ///
    /// * `dev_addr` - the device to send to, a relay for the relay commands.
    /// * `slot` - where and when to transmit, from the event that reported its uplink.
    /// * `commands` - the commands, in the order they should be read.
    ///
    /// # Returns
    ///
    /// The packet to put in a `PULL_RESP`.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::NoSession`] if no device holds that address here, and
    /// [`NetworkError::Frame`] if the commands do not fit one frame.
    pub fn command(
        &mut self,
        dev_addr: u32,
        slot: Slot,
        commands: &[MacCommand],
    ) -> Result<Txpk, NetworkError> {
        let mut bytes = vec![0u8; commands.len() * MAX_COMMAND];
        let len = encode_all(commands, &mut bytes).map_err(NetworkError::Frame)?;
        let held = self
            .admitted
            .iter_mut()
            .find(|held| held.dev_addr == dev_addr)
            .ok_or(NetworkError::NoSession { dev_addr })?;

        let downlink = if len <= FOPTS_MAX {
            Downlink::empty(held.fcnt_down).with_fopts(&bytes[..len])
        } else {
            Downlink::new(held.fcnt_down, 0, &bytes[..len])
        };
        let frame = held
            .session
            .encode_downlink(&downlink)
            .map_err(NetworkError::Frame)?;
        held.fcnt_down = held.fcnt_down.wrapping_add(1);
        Ok(transmit(slot, frame.as_bytes().to_vec()))
    }

    /// Builds the command that tells a relay to trust a device, with the key that lets it
    /// verify the device's wake-on-radio frames, TS011-1.0.1 section 10.4.
    ///
    /// # Arguments
    ///
    /// * `dev_addr` - the end device to trust.
    /// * `index` - which of the relay's sixteen entries to put it in.
    /// * `reload_rate` - how many of its uplinks the relay forwards an hour, 63 for no
    ///   limit.
    /// * `bucket_size` - the coded bucket size multiplier, table 55.
    ///
    /// # Returns
    ///
    /// The `UpdateUplinkListReq` to send to the relay with [`command`](Network::command).
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::NoSession`] if no device holds that address here.
    pub fn trust_command(
        &self,
        dev_addr: u32,
        index: u8,
        reload_rate: u8,
        bucket_size: u8,
    ) -> Result<MacCommand, NetworkError> {
        let held = self
            .admitted
            .iter()
            .find(|held| held.dev_addr == dev_addr)
            .ok_or(NetworkError::NoSession { dev_addr })?;
        Ok(MacCommand::UpdateUplinkListReq {
            index,
            reload_rate,
            bucket_size,
            dev_addr,
            wfcnt: 0,
            root_wor_s_key: held.session.root_wor_s_key(),
        })
    }

    /// Takes the devices relays have reported hearing and could not verify, TS011-1.0.1
    /// section 10.7.
    ///
    /// A relay carries these in the MAC commands of its own uplinks, alongside whatever
    /// else that uplink was for, so they wait here until they are read.
    ///
    /// # Returns
    ///
    /// Every notice since the last call, in the order they arrived.
    pub fn notices(&mut self) -> Vec<Notice> {
        core::mem::take(&mut self.notices)
    }

    /// Counts the notices waiting to be read, without taking them.
    ///
    /// # Returns
    ///
    /// How many [`Notice`]s the next call to [`Network::notices`] would return.
    #[must_use]
    pub fn notice_count(&self) -> usize {
        self.notices.len()
    }

    // Verifies a join request against every registered key, grants a session, and answers.
    fn admit(&mut self, heard: &Rxpk, link: LinkSettings) -> Result<Event, NetworkError> {
        self.admit_frame(
            &heard.payload,
            self.slot(heard, link, self.windows.join_delay_us)?,
            Some(heard.frequency_hz),
            None,
        )
    }

    // The same, for a frame a relay forwarded, which is answered through that relay in its
    // own window rather than in the device's.
    fn admit_frame(
        &mut self,
        payload: &[u8],
        mut slot: Slot,
        accept_hz: Option<u32>,
        relay: Option<u32>,
    ) -> Result<Event, NetworkError> {
        let (registration, request) = self
            .registrations
            .iter()
            .find_map(|registration| {
                let request = JoinRequest::parse(payload, &registration.app_key).ok()?;
                (request.dev_eui() == registration.dev_eui).then_some((*registration, request))
            })
            .ok_or(NetworkError::UnknownDevice)?;

        let dev_addr = self.next_dev_addr;
        let app_nonce = self.next_app_nonce;
        let grant = JoinGrant::new(app_nonce, self.net_id, dev_addr)
            .with_dl_settings(
                self.windows
                    .dl_settings_byte(self.plan.with_plan(|plan| plan.rx2_data_rate)),
            )
            .with_rx_delay(self.windows.rx_delay_byte());
        let accept = grant.accept(&registration.app_key, request.dev_nonce());
        let session = grant.session(&registration.app_key, request.dev_nonce());

        self.next_dev_addr = self.next_dev_addr.wrapping_add(1);
        self.next_app_nonce = self.next_app_nonce.wrapping_add(1);
        self.admitted
            .retain(|held| held.dev_eui != registration.dev_eui);
        self.admitted.push(Admitted {
            dev_eui: registration.dev_eui,
            dev_addr,
            session,
            fcnt_up: None,
            fcnt_down: 0,
            via_relay: relay,
        });

        if let Some(uplink_hz) = accept_hz {
            if self.windows.rx1_channels == Rx1Channels::Plan {
                if let Some(hz) = self.join_accept_hz(uplink_hz) {
                    slot.frequency_hz = hz;
                }
            }
        }
        let accept = match relay {
            Some(relay) => self.wrap_for_relay(relay, accept.as_bytes())?,
            None => accept.as_bytes().to_vec(),
        };
        Ok(Event::Joined {
            dev_eui: registration.dev_eui,
            dev_addr,
            accept: transmit(slot, accept),
        })
    }

    // Routes a data frame to its session, follows its counter, and decrypts it.
    fn receive(
        &mut self,
        heard: &Rxpk,
        link: LinkSettings,
        header: &FrameHeader,
    ) -> Result<Event, NetworkError> {
        let dev_addr = header
            .dev_addr()
            .ok_or(NetworkError::Frame(LorawanError::MalformedFrame))?;
        let carried = header
            .fcnt()
            .ok_or(NetworkError::Frame(LorawanError::MalformedFrame))?;
        let slot = self.slot(heard, link, self.windows.receive_delay_us)?;

        let Some(held) = self
            .admitted
            .iter_mut()
            .find(|held| held.dev_addr == dev_addr)
        else {
            return Ok(Event::Foreign { dev_addr });
        };

        let fcnt = match held.fcnt_up {
            None => u32::from(carried),
            Some(seen) => {
                let mut candidate = (seen & 0xFFFF_0000) | u32::from(carried);
                // The counter already accepted, sent again.
                if candidate == seen {
                    return Err(NetworkError::Replayed {
                        dev_addr,
                        fcnt: candidate,
                    });
                }
                // Only the low sixteen bits travel, so a counter below the one last seen is
                // the device having wrapped rather than having gone backwards.
                if candidate < seen {
                    candidate = candidate.wrapping_add(0x0001_0000);
                }
                if candidate - seen > MAX_FCNT_GAP {
                    return Err(NetworkError::CounterGap {
                        dev_addr,
                        seen,
                        carried: candidate,
                    });
                }
                candidate
            }
        };

        let data = held
            .session
            .decode(&heard.payload, fcnt)
            .map_err(NetworkError::Frame)?;
        held.fcnt_up = Some(fcnt);
        held.via_relay = None;

        // A relay tells the network about devices it could not verify in the MAC commands
        // of its own uplinks, TS011-1.0.1 section 10.7.
        let commands = if data.fport() == Some(0) {
            data.payload()
        } else {
            data.fopts()
        };
        for command in MacCommands::new(Direction::Uplink, commands).flatten() {
            if let MacCommand::NotifyNewEndDeviceReq {
                dev_addr: heard_addr,
                rssi_dbm,
                snr_db,
            } = command
            {
                self.notices.push(Notice {
                    relay: dev_addr,
                    dev_addr: heard_addr,
                    rssi_dbm,
                    snr_db,
                });
            }
        }

        if data.fport() == Some(LA_FPORT_RELAY) {
            return self.forwarded(dev_addr, data.payload(), slot);
        }

        Ok(Event::Data {
            dev_addr,
            fcnt,
            fport: data.fport(),
            payload: data.payload().to_vec(),
            confirmed: data.confirmed(),
            slot,
            relay: None,
        })
    }

    // Reads an uplink a relay forwarded, and treats it as if the device itself had sent it,
    // TS011-1.0.1 section 9.1.
    fn forwarded(&mut self, relay: u32, payload: &[u8], slot: Slot) -> Result<Event, NetworkError> {
        let forwarded = ForwardedUplink::parse(payload).map_err(NetworkError::Relayed)?;
        let relayed = Relayed {
            relay,
            metadata: forwarded.metadata,
            frequency_hz: forwarded.frequency_hz,
        };
        let header = FrameHeader::parse(forwarded.phy_payload).map_err(NetworkError::Relayed)?;
        match header.message_type() {
            MessageType::JoinRequest => {
                self.admit_frame(forwarded.phy_payload, slot, None, Some(relay))
            }
            MessageType::UnconfirmedUp | MessageType::ConfirmedUp => {
                let dev_addr = header
                    .dev_addr()
                    .ok_or(NetworkError::Relayed(LorawanError::MalformedFrame))?;
                let carried = header
                    .fcnt()
                    .ok_or(NetworkError::Relayed(LorawanError::MalformedFrame))?;
                let Some(held) = self
                    .admitted
                    .iter_mut()
                    .find(|held| held.dev_addr == dev_addr)
                else {
                    return Ok(Event::Foreign { dev_addr });
                };
                let fcnt = match held.fcnt_up {
                    None => u32::from(carried),
                    Some(seen) => {
                        let mut candidate = (seen & 0xFFFF_0000) | u32::from(carried);
                        if candidate == seen {
                            return Err(NetworkError::Replayed {
                                dev_addr,
                                fcnt: candidate,
                            });
                        }
                        if candidate < seen {
                            candidate = candidate.wrapping_add(0x0001_0000);
                        }
                        if candidate - seen > MAX_FCNT_GAP {
                            return Err(NetworkError::CounterGap {
                                dev_addr,
                                seen,
                                carried: candidate,
                            });
                        }
                        candidate
                    }
                };
                let data = held
                    .session
                    .decode(forwarded.phy_payload, fcnt)
                    .map_err(NetworkError::Relayed)?;
                held.fcnt_up = Some(fcnt);
                held.via_relay = Some(relay);
                Ok(Event::Data {
                    dev_addr,
                    fcnt,
                    fport: data.fport(),
                    payload: data.payload().to_vec(),
                    confirmed: data.confirmed(),
                    slot,
                    relay: Some(relayed),
                })
            }
            other => Err(NetworkError::Relayed(LorawanError::UnsupportedMType(
                downlink_mtype(other),
            ))),
        }
    }

    // Wraps a frame for an end device in the downlink its relay forwards it in, section 9.2.
    fn wrap_for_relay(&mut self, relay: u32, frame: &[u8]) -> Result<Vec<u8>, NetworkError> {
        let held = self
            .admitted
            .iter_mut()
            .find(|held| held.dev_addr == relay)
            .ok_or(NetworkError::NoSession { dev_addr: relay })?;
        let wrapped = held
            .session
            .encode_downlink(&Downlink::new(held.fcnt_down, LA_FPORT_RELAY, frame))
            .map_err(NetworkError::Frame)?;
        held.fcnt_down = held.fcnt_down.wrapping_add(1);
        Ok(wrapped.as_bytes().to_vec())
    }

    // Works out where and when the first receive window opens for a packet.
    fn slot(&self, heard: &Rxpk, link: LinkSettings, delay_us: u32) -> Result<Slot, NetworkError> {
        let uplink_rate = self
            .data_rate_of(link)
            .ok_or(NetworkError::UnknownDataRate)?;
        let settings = self
            .plan
            .with_plan(|plan| {
                plan.rx1_data_rate(uplink_rate, self.windows.rx1_data_rate_offset)
                    .and_then(|downlink_rate| plan.downlink_data_rate(downlink_rate))
                    .and_then(|rate| rate.link_settings())
            })
            .ok_or(NetworkError::NoWindow)?;

        Ok(Slot {
            timestamp_us: heard.timestamp_us.unwrap_or(0).wrapping_add(delay_us),
            frequency_hz: self.rx1_frequency_hz(heard.frequency_hz)?,
            link: settings,
        })
    }

    // Applies the region's rule for which channel the first window answers on.
    fn rx1_frequency_hz(&self, uplink_hz: u32) -> Result<u32, NetworkError> {
        match self.windows.rx1_channels {
            Rx1Channels::Plan => {
                if self.plan.with_plan(|plan| plan.downlink_channel_count()) == 0 {
                    return Ok(uplink_hz);
                }
                let channel = self.channel_of(uplink_hz).ok_or(NetworkError::NoWindow)?;
                self.plan
                    .with_plan(|plan| plan.rx1_frequency_hz(channel, uplink_hz))
                    .ok_or(NetworkError::NoWindow)
            }
            Rx1Channels::SameAsUplink => Ok(uplink_hz),
            Rx1Channels::Downstream(block) => {
                let channel = self.channel_of(uplink_hz).ok_or(NetworkError::NoWindow)?;
                block
                    .frequency_hz(channel % block.count)
                    .ok_or(NetworkError::NoWindow)
            }
        }
    }

    // Finds where a join on a common join channel is answered, for a plan that has them.
    fn join_accept_hz(&self, uplink_hz: u32) -> Option<u32> {
        self.join_accepts.iter().find_map(|run| {
            (0..run.channels.count)
                .find(|offset| run.channels.frequency_hz(*offset) == Some(uplink_hz))
                .map(|offset| run.start_hz + run.step_hz * u32::from(offset))
        })
    }

    // Finds which channel of the plan a frequency is, counting through the default blocks
    // in the order the channel numbering follows.
    fn channel_of(&self, frequency_hz: u32) -> Option<u16> {
        self.plan.with_plan(|plan| {
            let mut first = 0u16;
            for block in plan.default_channels {
                for index in 0..block.count {
                    if block.frequency_hz(index) == Some(frequency_hz) {
                        return Some(first + index);
                    }
                }
                first += block.count;
            }
            None
        })
    }

    // Finds the data rate number whose settings a packet arrived with. Only the spreading
    // factor and the bandwidth name a rate; the coding rate and the frame options a radio
    // reports alongside them do not.
    fn data_rate_of(&self, link: LinkSettings) -> Option<u8> {
        self.plan.with_plan(|plan| {
            plan.uplink_data_rates
                .iter()
                .enumerate()
                .find_map(|(number, rate)| {
                    let settings = rate.as_ref()?.link_settings()?;
                    (settings.spreading_factor() == link.spreading_factor()
                        && settings.bandwidth_hz() == link.bandwidth_hz())
                    .then_some(number as u8)
                })
        })
    }
}

// The packet a downlink goes out as: at the window, with the polarity a device listens for.
fn transmit(slot: Slot, payload: Vec<u8>) -> Txpk {
    Txpk::at(slot.timestamp_us, slot.frequency_hz, slot.link, payload).with_inverted_polarity(true)
}

// The MHDR byte a message type carries, for reporting one that is not an uplink.
const fn downlink_mtype(message_type: MessageType) -> u8 {
    match message_type {
        MessageType::JoinRequest => 0x00,
        MessageType::JoinAccept => 0x20,
        MessageType::UnconfirmedUp => 0x40,
        MessageType::UnconfirmedDown => 0x60,
        MessageType::ConfirmedUp => 0x80,
        MessageType::ConfirmedDown => 0xA0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pamoja_lora::region::{Cn470Plan, Region};
    use pamoja_lorawan::{Device, Uplink};

    const DEV_EUI: [u8; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
    const APP_EUI: [u8; 8] = [0x70, 0xB3, 0xD5, 0x7E, 0xD0, 0x00, 0x00, 0x01];
    const APP_KEY: [u8; 16] = [
        0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6, 0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F,
        0x3C,
    ];

    fn site() -> Network {
        let mut network =
            Network::new(Region::Eu868.plan(), 0x00_00_2A).with_first_dev_addr(0x2601_0001);
        network.register(Registration::new(DEV_EUI, APP_EUI, APP_KEY));
        network
    }

    fn heard(frame: Vec<u8>, timestamp_us: u32) -> Rxpk {
        Rxpk::new(868_100_000, LinkSettings::new(7, 125_000), frame).with_timestamp_us(timestamp_us)
    }

    #[test]
    fn the_recommended_delays_are_the_published_ones() {
        // RP002-1.0.5 section 3.3: RECEIVE_DELAY1 1s, RECEIVE_DELAY2 2s, JOIN_ACCEPT_DELAY1
        // 5s, JOIN_ACCEPT_DELAY2 6s.
        assert_eq!(RECEIVE_DELAY1_US, 1_000_000);
        assert_eq!(RECEIVE_DELAY2_US, 2_000_000);
        assert_eq!(JOIN_ACCEPT_DELAY1_US, 5_000_000);
        assert_eq!(JOIN_ACCEPT_DELAY2_US, 6_000_000);
        assert_eq!(MAX_FCNT_GAP, 16_384);
    }

    #[test]
    fn the_join_accept_bytes_follow_the_specification() {
        // TS001-1.0.4 table 55: RFU, then RX1DROffset in bits 6:4, then RX2DataRate in 3:0.
        let windows = Windows::new().with_rx1_data_rate_offset(5);
        assert_eq!(windows.dl_settings_byte(0), 0x50);
        assert_eq!(windows.dl_settings_byte(0x0F), 0x5F);
        assert_eq!(Windows::new().dl_settings_byte(8), 0x08);

        // TS001-1.0.4 table 44: the delay in seconds in the low four bits, and a zero there
        // means one second.
        assert_eq!(Windows::new().rx_delay_byte(), 1);
        assert_eq!(
            Windows::new()
                .with_receive_delay_us(5_000_000)
                .rx_delay_byte(),
            5
        );
    }

    #[test]
    fn a_registration_never_prints_its_key() {
        let printed = format!("{:?}", Registration::new(DEV_EUI, APP_EUI, APP_KEY));
        assert!(printed.contains("dev_eui"));
        assert!(!printed.contains("2b") && !printed.contains("43"));
    }

    #[test]
    fn a_device_joins_and_is_answered_in_the_join_window() {
        let mut network = site();
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let request = device.join_request(0x0102);

        let event = network
            .uplink(&heard(request.as_bytes().to_vec(), 1_000_000))
            .expect("the request verifies");

        let Event::Joined {
            dev_eui,
            dev_addr,
            accept,
        } = event
        else {
            panic!("a join request is admitted");
        };
        assert_eq!(dev_eui, DEV_EUI);
        assert_eq!(dev_addr, 0x2601_0001);
        // Five seconds after the uplink, on the uplink frequency, with inverted polarity.
        assert_eq!(accept.timestamp_us, Some(6_000_000));
        assert_eq!(accept.frequency_hz, 868_100_000);
        assert!(accept.invert_polarity);

        // The device reads the accept it was sent and derives the same session.
        let granted = device
            .accept_join(&accept.payload, 0x0102)
            .expect("the accept verifies");
        assert_eq!(granted.dev_addr(), 0x2601_0001);
    }

    #[test]
    fn an_uplink_is_decrypted_and_answered_one_second_later() {
        let mut network = site();
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let request = device.join_request(0x0102);
        let Event::Joined { accept, .. } = network
            .uplink(&heard(request.as_bytes().to_vec(), 1_000_000))
            .expect("the request verifies")
        else {
            panic!("a join request is admitted");
        };
        let session = device
            .accept_join(&accept.payload, 0x0102)
            .expect("the accept verifies")
            .session();

        let frame = session
            .encode_uplink(&pamoja_lorawan::Uplink::new(0, 2, b"21.5"))
            .expect("it fits one frame");
        let event = network
            .uplink(&heard(frame.as_bytes().to_vec(), 9_000_000))
            .expect("the frame verifies");

        let Event::Data {
            dev_addr,
            fcnt,
            fport,
            payload,
            confirmed,
            slot,
            ..
        } = event
        else {
            panic!("a data frame is read");
        };
        assert_eq!(dev_addr, 0x2601_0001);
        assert_eq!(fcnt, 0);
        assert_eq!(fport, Some(2));
        assert_eq!(payload, b"21.5");
        assert!(!confirmed);
        assert_eq!(slot.timestamp_us, 10_000_000);
        assert_eq!(slot.frequency_hz, 868_100_000);
        assert_eq!(slot.link.spreading_factor(), 7);

        // The answer goes out in that window, and the device reads it.
        let downlink = network
            .answer(dev_addr, slot, 2, b"ok")
            .expect("the session is held");
        assert_eq!(downlink.timestamp_us, Some(10_000_000));
        assert!(downlink.invert_polarity);
        let read = session.decode(&downlink.payload, 0).expect("it verifies");
        assert_eq!(read.payload(), b"ok");
    }

    #[test]
    fn a_replayed_frame_is_refused() {
        let mut network = site();
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let request = device.join_request(0x0102);
        let Event::Joined { accept, .. } = network
            .uplink(&heard(request.as_bytes().to_vec(), 1_000_000))
            .expect("the request verifies")
        else {
            panic!("a join request is admitted");
        };
        let session = device
            .accept_join(&accept.payload, 0x0102)
            .expect("the accept verifies")
            .session();

        let first = session
            .encode_uplink(&pamoja_lorawan::Uplink::new(1, 2, b"one"))
            .expect("it fits one frame");
        network
            .uplink(&heard(first.as_bytes().to_vec(), 2_000_000))
            .expect("the first frame is read");

        let error = network
            .uplink(&heard(first.as_bytes().to_vec(), 3_000_000))
            .expect_err("the same counter twice is a replay");
        assert!(matches!(
            error,
            NetworkError::Replayed {
                dev_addr: 0x2601_0001,
                ..
            }
        ));
    }

    #[test]
    fn a_counter_that_wraps_carries_on_where_it_left_off() {
        let mut network = site();
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let request = device.join_request(0x0102);
        let Event::Joined { accept, .. } = network
            .uplink(&heard(request.as_bytes().to_vec(), 1_000_000))
            .expect("the request verifies")
        else {
            panic!("a join request is admitted");
        };
        let session = device
            .accept_join(&accept.payload, 0x0102)
            .expect("the accept verifies")
            .session();

        // The last counter the low sixteen bits can hold, then the one after it.
        let last = session
            .encode_uplink(&pamoja_lorawan::Uplink::new(0xFFFF, 2, b"last"))
            .expect("it fits one frame");
        network
            .uplink(&heard(last.as_bytes().to_vec(), 2_000_000))
            .expect("the frame is read");

        let wrapped = session
            .encode_uplink(&pamoja_lorawan::Uplink::new(0x0001_0000, 2, b"next"))
            .expect("it fits one frame");
        let event = network
            .uplink(&heard(wrapped.as_bytes().to_vec(), 3_000_000))
            .expect("the wrapped frame is read");

        let Event::Data { fcnt, payload, .. } = event else {
            panic!("a data frame is read");
        };
        assert_eq!(fcnt, 0x0001_0000);
        assert_eq!(payload, b"next");
    }

    #[test]
    fn a_frame_for_another_network_is_reported_not_refused() {
        let mut network = site();
        let stranger = Session::new(0x1234_5678, [9; 16], [8; 16]);
        let frame = stranger
            .encode_uplink(&pamoja_lorawan::Uplink::new(0, 1, b"hello"))
            .expect("it fits one frame");

        let event = network
            .uplink(&heard(frame.as_bytes().to_vec(), 1_000_000))
            .expect("a frame from elsewhere is not an error");
        assert_eq!(
            event,
            Event::Foreign {
                dev_addr: 0x1234_5678
            }
        );
    }

    #[test]
    fn the_first_window_follows_the_region() {
        // RP002-1.0.5 section 3.5.7: RX1 Channel Number = Transmit Channel Number modulo
        // NbChannel, over the eight downlink channels of section 3.5.2.
        let block = match Rx1Channels::us915() {
            Rx1Channels::Downstream(block) => block,
            other => panic!("US902-928 answers on its own channels, not {other:?}"),
        };
        assert_eq!(block.frequency_hz(0), Some(923_300_000));
        assert_eq!(block.frequency_hz(7), Some(927_500_000));
        assert_eq!(block.count, 8);
    }

    /// Admits the test device on a plan and returns the accept it was sent.
    fn joined_on(network: &mut Network, frequency_hz: u32, link: LinkSettings) -> Txpk {
        network.register(Registration::new(DEV_EUI, APP_EUI, APP_KEY));
        let request = Device::new(DEV_EUI, APP_EUI, APP_KEY).join_request(0x0102);
        let heard =
            Rxpk::new(frequency_hz, link, request.as_bytes().to_vec()).with_timestamp_us(1_000_000);
        match network.uplink(&heard).expect("the request verifies") {
            Event::Joined { accept, .. } => accept,
            other => panic!("a join request is admitted, not {other:?}"),
        }
    }

    #[test]
    fn a_fixed_plan_is_answered_on_its_downlink_channels_at_its_downlink_rates() {
        // RP002-1.0.5 sections 3.5.2 and 3.5.7, and tables 20 and 26: a join on channel 0 at
        // DR0 is answered on 923.3 MHz at DR10, SF10 on 500 kHz, and an uplink on channel 65
        // at DR4 on 923.9 MHz at DR13, SF7 on 500 kHz.
        let mut network = Network::new(Region::Us915.plan(), 0x00_00_2A);
        let accept = joined_on(&mut network, 902_300_000, LinkSettings::new(10, 125_000));
        assert_eq!(accept.frequency_hz, 923_300_000);
        let crate::udp::Modulation::Lora(link) = accept.modulation else {
            panic!("a LoRa accept");
        };
        assert_eq!(
            (link.spreading_factor(), link.bandwidth_hz()),
            (10, 500_000)
        );

        let session = Device::new(DEV_EUI, APP_EUI, APP_KEY)
            .accept_join(&accept.payload, 0x0102)
            .expect("the accept verifies")
            .session();
        let frame = session
            .encode_uplink(&pamoja_lorawan::Uplink::new(0, 2, b"21.5"))
            .expect("it fits one frame");
        let heard = Rxpk::new(
            904_600_000,
            LinkSettings::new(8, 500_000),
            frame.as_bytes().to_vec(),
        )
        .with_timestamp_us(9_000_000);
        let Event::Data { slot, .. } = network.uplink(&heard).expect("the frame verifies") else {
            panic!("a data frame is read");
        };
        assert_eq!(slot.frequency_hz, 923_900_000);
        assert_eq!(
            (slot.link.spreading_factor(), slot.link.bandwidth_hz()),
            (7, 500_000)
        );
    }

    #[test]
    fn a_chinese_join_is_answered_where_its_common_join_channel_says() {
        // RP002-1.0.5 table 49: common join channel 11, 472.3 MHz, is answered on 492.5 MHz,
        // and channel 3, 475.7 MHz, on 489.3 MHz.
        let mut network = Network::new(Cn470Plan::Antenna26MhzA.plan(), 0x00_00_2A);
        let accept = joined_on(&mut network, 472_300_000, LinkSettings::new(9, 125_000));
        assert_eq!(accept.frequency_hz, 492_500_000);

        let mut network = Network::new(Region::Cn470.plan(), 0x00_00_2A);
        let accept = joined_on(&mut network, 475_700_000, LinkSettings::new(9, 125_000));
        assert_eq!(accept.frequency_hz, 489_300_000);
    }

    #[test]
    fn a_window_override_still_wins_over_the_plan() {
        let mut network = Network::new(Region::Us915.plan(), 0x00_00_2A)
            .with_windows(Windows::new().with_rx1_channels(Rx1Channels::SameAsUplink));
        let accept = joined_on(&mut network, 902_300_000, LinkSettings::new(10, 125_000));
        assert_eq!(accept.frequency_hz, 902_300_000);
    }

    #[test]
    fn an_fsk_packet_carries_no_frame() {
        let mut network = site();
        let mut packet = heard(vec![0x40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0);
        packet.modulation = crate::udp::Modulation::Fsk(50_000);
        assert_eq!(network.uplink(&packet), Err(NetworkError::NotLora));
    }

    const RELAY_EUI: [u8; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x88];
    const RELAY_KEY: [u8; 16] = [0x5A; 16];

    /// A network with a relay and an end device registered, the relay joined and holding a
    /// session, and its address.
    fn site_with_relay() -> (Network, Session, u32) {
        let mut network = site();
        network.register(Registration::new(RELAY_EUI, APP_EUI, RELAY_KEY));
        let relay = Device::new(RELAY_EUI, APP_EUI, RELAY_KEY);
        let request = relay.join_request(0x0101);
        let Event::Joined { dev_addr, .. } = network
            .uplink(&heard(request.as_bytes().to_vec(), 1_000))
            .expect("the relay joins")
        else {
            panic!("the relay joined");
        };
        let session = network.session(dev_addr).expect("a session");
        (network, session, dev_addr)
    }

    /// The uplink a relay sends to forward a frame it heard.
    fn forwarded(session: &Session, fcnt: u32, phy_payload: &[u8]) -> Vec<u8> {
        let uplink = ForwardedUplink {
            metadata: UplinkMetadata {
                wor_channel: pamoja_lorawan::relay::WorChannel::Default,
                rssi_dbm: -92,
                snr_db: 5,
                data_rate: 5,
            },
            frequency_hz: 868_300_000,
            phy_payload,
        };
        let mut payload = vec![0u8; phy_payload.len() + 16];
        let len = uplink.encode(&mut payload).expect("it encodes");
        session
            .encode_uplink(&Uplink::new(fcnt, LA_FPORT_RELAY, &payload[..len]))
            .expect("the relay's uplink encodes")
            .as_bytes()
            .to_vec()
    }

    #[test]
    fn a_join_a_relay_forwards_is_admitted_and_its_accept_goes_back_the_same_way() {
        let (mut network, relay_session, relay_addr) = site_with_relay();
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let request = device.join_request(0x1234);

        let event = network
            .uplink(&heard(
                forwarded(&relay_session, 0, request.as_bytes()),
                5_000,
            ))
            .expect("the forwarded request verifies");
        let Event::Joined {
            dev_eui,
            dev_addr,
            accept,
        } = event
        else {
            panic!("the device joined: {event:?}");
        };
        assert_eq!(dev_eui, DEV_EUI);
        assert_ne!(dev_addr, relay_addr);

        // The accept is addressed to the relay, on the port a relay forwards on, and
        // carries the device's own accept for it to send on.
        let to_relay = relay_session
            .decode(&accept.payload, 0)
            .expect("the relay's downlink decodes");
        assert_eq!(to_relay.fport(), Some(LA_FPORT_RELAY));
        let session = device
            .accept_join(to_relay.payload(), 0x1234)
            .expect("the device's own accept")
            .session();
        assert_eq!(session.dev_addr(), dev_addr);
    }

    #[test]
    fn an_uplink_a_relay_forwards_carries_what_it_heard_and_is_answered_through_it() {
        let (mut network, relay_session, _) = site_with_relay();
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let request = device.join_request(0x1234);
        let Event::Joined { dev_addr, .. } = network
            .uplink(&heard(
                forwarded(&relay_session, 0, request.as_bytes()),
                5_000,
            ))
            .expect("the device joins")
        else {
            panic!("the device joined");
        };
        let session = network.session(dev_addr).expect("the granted session");

        let reading = session
            .encode_uplink(&Uplink::new(0, 2, b"21.5"))
            .expect("the reading encodes");
        let event = network
            .uplink(&heard(
                forwarded(&relay_session, 1, reading.as_bytes()),
                9_000,
            ))
            .expect("the forwarded reading verifies");
        let Event::Data {
            dev_addr: from,
            payload,
            relay,
            slot,
            ..
        } = event
        else {
            panic!("a reading came through: {event:?}");
        };
        assert_eq!(from, dev_addr);
        assert_eq!(payload, b"21.5");
        let relayed = relay.expect("it came through a relay");
        assert_eq!(relayed.metadata.rssi_dbm, -92);
        assert_eq!(relayed.metadata.snr_db, 5);
        assert_eq!(relayed.frequency_hz, 868_300_000);

        // The answer goes to the relay, which sends it on.
        let answer = network
            .answer(dev_addr, slot, 2, b"ok")
            .expect("an answer for the device");
        // The accept took the relay's first downlink, so this is its second.
        let to_relay = relay_session
            .decode(&answer.payload, 1)
            .expect("the relay's downlink decodes");
        assert_eq!(to_relay.fport(), Some(LA_FPORT_RELAY));
        let inside = session
            .decode(to_relay.payload(), 0)
            .expect("the device's own downlink");
        assert_eq!(inside.payload(), b"ok");
    }

    #[test]
    fn a_relay_reports_the_devices_it_cannot_verify_and_is_told_to_trust_them() {
        let (mut network, relay_session, relay_addr) = site_with_relay();
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let request = device.join_request(0x1234);
        let Event::Joined { dev_addr, .. } = network
            .uplink(&heard(
                forwarded(&relay_session, 0, request.as_bytes()),
                5_000,
            ))
            .expect("the device joins")
        else {
            panic!("the device joined");
        };

        // The relay carries a notice in the frame options of its own uplink.
        let mut fopts = [0u8; MAX_COMMAND];
        let len = encode_all(
            &[MacCommand::NotifyNewEndDeviceReq {
                dev_addr: 0x2601_9999,
                rssi_dbm: -100,
                snr_db: 3,
            }],
            &mut fopts,
        )
        .expect("the notice encodes");
        let uplink = relay_session
            .encode_uplink(&Uplink::empty(1).with_fopts(&fopts[..len]))
            .expect("the relay's uplink encodes");
        let event = network
            .uplink(&heard(uplink.as_bytes().to_vec(), 11_000))
            .expect("the uplink verifies");
        let slot = match event {
            Event::Data { slot, .. } => slot,
            other => panic!("the relay sent an uplink: {other:?}"),
        };
        assert_eq!(
            network.notices(),
            vec![Notice {
                relay: relay_addr,
                dev_addr: 0x2601_9999,
                rssi_dbm: -100,
                snr_db: 3,
            }]
        );
        assert!(network.notices().is_empty(), "each notice is read once");

        // The network tells the relay to trust the device, with the key that lets it check
        // the device's wake-on-radio frames.
        let command = network
            .trust_command(dev_addr, 0, 63, 0)
            .expect("the device has a session");
        let MacCommand::UpdateUplinkListReq {
            root_wor_s_key,
            dev_addr: trusted,
            ..
        } = command
        else {
            panic!("an UpdateUplinkListReq: {command:?}");
        };
        assert_eq!(trusted, dev_addr);
        assert_eq!(
            root_wor_s_key,
            network
                .session(dev_addr)
                .expect("a session")
                .root_wor_s_key(),
        );

        let txpk = network
            .command(relay_addr, slot, &[command])
            .expect("a downlink for the relay");
        let to_relay = relay_session
            .decode(&txpk.payload, 1)
            .expect("the relay's downlink decodes");
        let carried: Vec<_> = MacCommands::new(Direction::Downlink, to_relay.payload())
            .flatten()
            .collect();
        assert_eq!(carried, vec![command], "a long command takes port 0");
        assert_eq!(to_relay.fport(), Some(0));
    }

    #[test]
    fn a_relay_that_forwards_nonsense_is_refused_without_taking_the_network_down() {
        let (mut network, relay_session, _) = site_with_relay();
        let uplink = relay_session
            .encode_uplink(&Uplink::new(0, LA_FPORT_RELAY, &[0x01, 0x02]))
            .expect("the relay's uplink encodes");
        assert!(matches!(
            network.uplink(&heard(uplink.as_bytes().to_vec(), 3_000)),
            Err(NetworkError::Relayed(_))
        ));

        let stranger = Session::new(0x2601_5555, [0x11; 16], [0x22; 16]);
        let frame = stranger
            .encode_uplink(&Uplink::new(0, 1, b"hi"))
            .expect("it encodes");
        assert_eq!(
            network.uplink(&heard(
                forwarded(&relay_session, 1, frame.as_bytes()),
                4_000
            )),
            Ok(Event::Foreign {
                dev_addr: 0x2601_5555
            }),
            "a device this network never granted",
        );
    }
}
