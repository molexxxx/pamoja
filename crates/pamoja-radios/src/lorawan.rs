//! A LoRaWAN Class A node: a radio driven by a `pamoja-lorawan` end device.
//!
//! [`EndDevice`] decides what a node sends and when it listens, and owns no radio. [`Node`]
//! is the other half: it puts each transmission on the air, opens the two receive windows on
//! time, hands what it hears back to the device, and sends repeats when the device asks for
//! them. It blocks while it does, which is the shape of a Class A exchange: a node sends,
//! listens twice, and is done until it next has something to say.
//!
//! A receive window opens around the downlink's preamble rather than exactly at its delay.
//! The node cannot know the end of its own transmission to the microsecond, and a gateway
//! starts the downlink at the delay by its own clock, so each window is sized to catch the
//! preamble with a margin for both. The sizing is the one Semtech's LoRaMac-node reference
//! implementation computes: at least `min_rx_symbols` of the eight-symbol preamble must
//! land inside the window, with `max_rx_error_us` of timing error either way, and the window
//! is centered on the fourth preamble symbol.
//!
//! The radio is tuned the way RP002-1.0.5 table 112 describes a LoRaWAN device: sync word
//! 0x34 from SF12 to SF7 and 0x12 at SF6 and SF5, standard IQ going up, inverted IQ coming
//! down.
//!
//! # Examples
//!
//! A node joining through a radio the example plays the part of, with a network holding the
//! same root key:
//!
//! ```
//! use core::convert::Infallible;
//!
//! use pamoja_lora::region::Region;
//! use pamoja_lorawan::device::{EndDevice, Settings};
//! use pamoja_lorawan::{Device, JoinGrant, JoinRequest};
//! use pamoja_radios::lorawan::{Clock, Node, Reception, Transceiver, Tuning};
//!
//! // What the node was provisioned with, and what the network grants it: the first nonce
//! // it uses for this device, The Things Network's identifier, and an address.
//! const DEV_EUI: [u8; 8] = [0x70, 0xB3, 0xD5, 0x7E, 0xD0, 0x00, 0x12, 0x34];
//! const JOIN_EUI: [u8; 8] = [0x70, 0xB3, 0xD5, 0x7E, 0xD0, 0x00, 0x00, 0x00];
//! const APP_KEY: [u8; 16] = [7; 16];
//! const APP_NONCE: u32 = 1;
//! const NET_ID: u32 = 0x00_00_13;
//! const DEV_ADDR: u32 = 0x2601_2E43;
//!
//! /// A radio whose network answers every join request.
//! struct Air {
//!     accept: Option<Vec<u8>>,
//! }
//!
//! impl Transceiver for Air {
//!     type Error = Infallible;
//!
//!     fn tune(&mut self, _: Tuning) -> Result<(), Infallible> {
//!         Ok(())
//!     }
//!
//!     fn transmit(&mut self, frame: &[u8]) -> Result<(), Infallible> {
//!         let request = JoinRequest::parse(frame, &APP_KEY).expect("a join request");
//!         let grant = JoinGrant::new(APP_NONCE, NET_ID, DEV_ADDR);
//!         self.accept = Some(grant.accept(&APP_KEY, request.dev_nonce()).as_bytes().to_vec());
//!         Ok(())
//!     }
//!
//!     fn receive(&mut self, buffer: &mut [u8], _: u64) -> Result<Reception, Infallible> {
//!         Ok(match self.accept.take() {
//!             Some(frame) => {
//!                 buffer[..frame.len()].copy_from_slice(&frame);
//!                 Reception::Frame { len: frame.len(), snr_db: 9, rssi_dbm: -95 }
//!             }
//!             None => Reception::Nothing,
//!         })
//!     }
//!
//!     fn sleep(&mut self) -> Result<(), Infallible> {
//!         Ok(())
//!     }
//! }
//!
//! /// A clock that a delay moves forward, so the example runs instantly.
//! struct Instant(u64);
//!
//! impl Clock for Instant {
//!     fn now_us(&mut self) -> u64 {
//!         self.0 += 1;
//!         self.0
//!     }
//! }
//!
//! impl embedded_hal::delay::DelayNs for Instant {
//!     fn delay_ns(&mut self, ns: u32) {
//!         self.0 += u64::from(ns / 1000);
//!     }
//! }
//!
//! let device = EndDevice::new(
//!     Region::Eu868.plan(),
//!     Device::new(DEV_EUI, JOIN_EUI, APP_KEY),
//!     Settings::new(2, 20),
//! )?;
//! let mut node = Node::new(device, Air { accept: None }, Instant(0));
//!
//! assert!(node.join(1)?);
//! assert_eq!(node.device().dev_addr(), Some(DEV_ADDR));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use core::fmt;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;
use pamoja_lora::LinkSettings;
use pamoja_lorawan::device::{
    AckWindow, Delivery, DeviceError, EndDevice, Heard, Next, ReceiveWindow, RelayExchange,
    Transmission, Window, WorNext,
};

use crate::radio::{self, Radio, RadioConfig, SyncWord};
use crate::sx126x::{self, Sx126x};
use crate::sx127x::{self, Sx127x};

/// LoRaMac-node's default for the fewest preamble symbols a window must catch.
pub const MIN_RX_SYMBOLS: u8 = 6;

/// LoRaMac-node's default for the timing error a window allows either way, in microseconds.
pub const MAX_RX_ERROR_US: u32 = 10_000;

/// The longest LoRa payload, which is the most a receive buffer needs.
const MAX_FRAME: usize = 255;

/// A source of monotonic time in microseconds.
///
/// A closure returning the time serves, so an `esp-hal` program passes
/// `|| Instant::now().duration_since_epoch().as_micros()`.
pub trait Clock {
    /// Returns the time.
    ///
    /// # Returns
    ///
    /// Microseconds from any fixed start, never going backward.
    fn now_us(&mut self) -> u64;
}

impl<F: FnMut() -> u64> Clock for F {
    fn now_us(&mut self) -> u64 {
        self()
    }
}

/// A clock and a delay held together, for a board that provides them separately.
///
/// # Examples
///
/// ```
/// use embedded_hal::delay::DelayNs;
/// use pamoja_radios::lorawan::{Clock, Timer};
///
/// struct Busy;
///
/// impl DelayNs for Busy {
///     fn delay_ns(&mut self, _: u32) {}
/// }
///
/// let mut timer = Timer::new(|| 42, Busy);
/// assert_eq!(timer.now_us(), 42);
/// timer.delay_ms(1);
/// ```
pub struct Timer<C, D> {
    clock: C,
    delay: D,
}

impl<C, D> Timer<C, D> {
    /// Pairs a clock with a delay.
    ///
    /// # Arguments
    ///
    /// * `clock` - the time, such as a closure over a board's timer.
    /// * `delay` - a blocking delay.
    ///
    /// # Returns
    ///
    /// The pair.
    pub const fn new(clock: C, delay: D) -> Timer<C, D> {
        Timer { clock, delay }
    }
}

impl<C: Clock, D> Clock for Timer<C, D> {
    fn now_us(&mut self) -> u64 {
        self.clock.now_us()
    }
}

impl<C, D: DelayNs> DelayNs for Timer<C, D> {
    fn delay_ns(&mut self, ns: u32) {
        self.delay.delay_ns(ns);
    }
}

/// How to tune a radio for one transmission or one receive window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tuning {
    /// The carrier, in hertz.
    pub frequency_hz: u32,
    /// The LoRa settings.
    pub link: LinkSettings,
    /// The power to transmit at, conducted, in dBm.
    pub output_dbm: i8,
    /// The sync word.
    pub sync_word: SyncWord,
    /// Whether to send with inverted IQ.
    pub invert_iq_transmit: bool,
    /// Whether to listen for inverted IQ.
    pub invert_iq_receive: bool,
    /// The band an SX126x calibrates its image rejection for, lower and upper edge in hertz.
    pub band_hz: (u32, u32),
}

/// How a receive window ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reception {
    /// A frame whose CRC checked, at the start of the buffer.
    Frame {
        /// Its length.
        len: usize,
        /// Its signal-to-noise ratio, rounded to whole decibels.
        snr_db: i8,
        /// Its signal strength, rounded to whole decibels over a milliwatt. A relay
        /// forwards it with the uplink; nothing else needs it.
        rssi_dbm: i16,
    },
    /// No frame, or one that failed its checks.
    Nothing,
}

/// The four things a node asks of a radio.
///
/// The SX126x and SX127x drivers and the [`Radio`] that holds either implement it. A test,
/// or a radio driven some other way, implements the same four methods.
pub trait Transceiver {
    /// What the radio reports when it or its bus fails.
    type Error: fmt::Debug;

    /// Tunes the radio.
    ///
    /// # Arguments
    ///
    /// * `tuning` - the carrier, link, power, sync word and IQ polarity.
    ///
    /// # Errors
    ///
    /// Whatever the radio reports.
    fn tune(&mut self, tuning: Tuning) -> Result<(), Self::Error>;

    /// Sends a frame and waits until it has left.
    ///
    /// # Arguments
    ///
    /// * `frame` - the bytes.
    ///
    /// # Errors
    ///
    /// Whatever the radio reports.
    fn transmit(&mut self, frame: &[u8]) -> Result<(), Self::Error>;

    /// Listens for one frame.
    ///
    /// # Arguments
    ///
    /// * `buffer` - where a frame goes.
    /// * `timeout_us` - how long to wait for a frame to start.
    ///
    /// # Returns
    ///
    /// The frame, or that none arrived.
    ///
    /// # Errors
    ///
    /// Whatever the radio reports.
    fn receive(&mut self, buffer: &mut [u8], timeout_us: u64) -> Result<Reception, Self::Error>;

    /// Listens for a preamble over a few symbols, and reports whether one is there.
    ///
    /// This is what a relay scans its channels with, once a scan period, so it can sleep
    /// between them and only listen when something is on the air. The default says there
    /// may be: a radio that cannot detect activity by itself listens every time, which
    /// costs it power but hears everything.
    ///
    /// # Arguments
    ///
    /// * `symbols` - how many symbols to listen over.
    ///
    /// # Returns
    ///
    /// `true` when there may be a frame starting.
    ///
    /// # Errors
    ///
    /// Whatever the radio reports.
    fn detect(&mut self, symbols: u8) -> Result<bool, Self::Error> {
        let _ = symbols;
        Ok(true)
    }

    /// Puts the radio to sleep until the next exchange.
    ///
    /// # Errors
    ///
    /// Whatever the radio reports.
    fn sleep(&mut self) -> Result<(), Self::Error>;
}

/// What came of an uplink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    /// The downlink that answered it, if one did.
    pub delivery: Option<Delivery>,
    /// Whether the network acknowledged a confirmed uplink.
    pub acknowledged: bool,
    /// Whether the application payload went out, or waited behind answers the device owed.
    pub carried_payload: bool,
    /// How many times the frame went out.
    pub transmissions: u8,
}

/// What stopped an exchange.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeError<E> {
    /// The radio failed.
    Radio(E),
    /// The device refused; [`DeviceError::Wait`] says when the air is free.
    Device(DeviceError),
}

impl<E: fmt::Debug> fmt::Display for NodeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeError::Radio(error) => write!(f, "the radio failed: {error:?}"),
            NodeError::Device(error) => write!(f, "{error}"),
        }
    }
}

impl<E: fmt::Debug> core::error::Error for NodeError<E> {}

impl<E> From<DeviceError> for NodeError<E> {
    fn from(error: DeviceError) -> NodeError<E> {
        NodeError::Device(error)
    }
}

/// A LoRaWAN Class A node: an end device, a radio, and the time.
///
/// See the [module documentation](self).
pub struct Node<'p, R, C> {
    device: EndDevice<'p>,
    radio: R,
    clock: C,
    band_hz: Option<(u32, u32)>,
    min_rx_symbols: u8,
    max_rx_error_us: u32,
    buffer: [u8; MAX_FRAME],
}

impl<'p, R, C> Node<'p, R, C>
where
    R: Transceiver,
    C: Clock + DelayNs,
{
    /// A node.
    ///
    /// # Arguments
    ///
    /// * `device` - the end device, joined or not.
    /// * `radio` - the radio, initialized.
    /// * `clock` - the time, and a way to wait, in one: a board's timer usually provides both.
    ///
    /// # Returns
    ///
    /// The node, with LoRaMac-node's default window margins and an SX126x calibrated for the
    /// frequencies the device uses.
    pub fn new(device: EndDevice<'p>, radio: R, clock: C) -> Node<'p, R, C> {
        Node {
            device,
            radio,
            clock,
            band_hz: None,
            min_rx_symbols: MIN_RX_SYMBOLS,
            max_rx_error_us: MAX_RX_ERROR_US,
            buffer: [0; MAX_FRAME],
        }
    }

    /// Sets the band an SX126x calibrates for, in place of the span of frequencies the device
    /// uses, which is what it calibrates for otherwise. An SX127x ignores it.
    ///
    /// # Arguments
    ///
    /// * `low_hz` - the lower edge.
    /// * `high_hz` - the upper edge.
    ///
    /// # Returns
    ///
    /// The node.
    pub fn with_band(mut self, low_hz: u32, high_hz: u32) -> Node<'p, R, C> {
        self.band_hz = Some((low_hz, high_hz));
        self
    }

    /// Sets how much preamble a window must catch and how much timing error it allows.
    ///
    /// A board with a poor clock, or a slow bus between the radio's interrupt and the
    /// program noticing it, widens `max_rx_error_us`; a wider window costs a little more
    /// listening.
    ///
    /// # Arguments
    ///
    /// * `min_rx_symbols` - the fewest preamble symbols to catch.
    /// * `max_rx_error_us` - the timing error either way, in microseconds.
    ///
    /// # Returns
    ///
    /// The node.
    pub fn with_window_margin(
        mut self,
        min_rx_symbols: u8,
        max_rx_error_us: u32,
    ) -> Node<'p, R, C> {
        self.min_rx_symbols = min_rx_symbols;
        self.max_rx_error_us = max_rx_error_us;
        self
    }

    /// Returns the end device.
    ///
    /// # Returns
    ///
    /// The device, for its address, counters and settings.
    pub fn device(&self) -> &EndDevice<'p> {
        &self.device
    }

    /// Returns the end device to change, such as its battery level or a link check request.
    ///
    /// # Returns
    ///
    /// The device.
    pub fn device_mut(&mut self) -> &mut EndDevice<'p> {
        &mut self.device
    }

    /// Returns the radio.
    ///
    /// # Returns
    ///
    /// The radio, for anything the node does not do with it.
    pub fn radio_mut(&mut self) -> &mut R {
        &mut self.radio
    }

    /// Gives back the device, the radio and the clock.
    ///
    /// # Returns
    ///
    /// The three parts.
    pub fn release(self) -> (EndDevice<'p>, R, C) {
        (self.device, self.radio, self.clock)
    }

    /// Waits until a time, such as the one a [`DeviceError::Wait`] names.
    ///
    /// # Arguments
    ///
    /// * `until_us` - the time to wait until, by the node's clock.
    pub fn wait_until(&mut self, until_us: u64) {
        let now = self.clock.now_us();
        if until_us > now {
            wait(&mut self.clock, until_us - now);
        }
    }

    /// Makes one join attempt: sends a join request and listens in both join windows.
    ///
    /// # Arguments
    ///
    /// * `dev_nonce` - a nonce this device has never used; see [`EndDevice::join`].
    ///
    /// # Returns
    ///
    /// `true` once the network accepted, `false` if no accept arrived.
    ///
    /// # Errors
    ///
    /// Returns [`NodeError::Device`] with [`DeviceError::Wait`] when the join back-off or a
    /// duty cycle holds the device, and [`NodeError::Radio`] when the radio fails.
    pub fn join(&mut self, dev_nonce: u16) -> Result<bool, NodeError<R::Error>> {
        let now = self.clock.now_us();
        let request = self.device.join(dev_nonce, now)?;
        match self.exchange(&request)? {
            Some(Heard::Joined { .. }) => Ok(true),
            _ => {
                let closed = self.clock.now_us();
                self.device.nothing_heard(closed)?;
                Ok(false)
            }
        }
    }

    /// Sends an uplink and sees it through: both windows, and every repeat the device asks
    /// for.
    ///
    /// # Arguments
    ///
    /// * `port` - the application port.
    /// * `payload` - the payload.
    /// * `confirmed` - whether to ask for an acknowledgment.
    ///
    /// # Returns
    ///
    /// What came back, and whether the payload went out.
    ///
    /// # Errors
    ///
    /// Returns [`NodeError::Device`] when the device cannot send now, [`DeviceError::Wait`]
    /// among them, and [`NodeError::Radio`] when the radio fails.
    pub fn send(
        &mut self,
        port: u8,
        payload: &[u8],
        confirmed: bool,
    ) -> Result<Report, NodeError<R::Error>> {
        let now = self.clock.now_us();
        let first = self.device.send(port, payload, confirmed, now)?;
        self.see_through(first)
    }

    /// Sends an uplink with no payload, carrying whatever the device owes the network.
    ///
    /// # Returns
    ///
    /// What came back.
    ///
    /// # Errors
    ///
    /// As [`send`](Node::send).
    pub fn send_empty(&mut self) -> Result<Report, NodeError<R::Error>> {
        let now = self.clock.now_us();
        let first = self.device.send_empty(now)?;
        self.see_through(first)
    }

    fn see_through(&mut self, first: Transmission) -> Result<Report, NodeError<R::Error>> {
        let mut report = Report {
            delivery: None,
            acknowledged: false,
            carried_payload: first.carries_payload,
            transmissions: 0,
        };
        let mut transmission = first;
        loop {
            report.transmissions += 1;
            if let Some(Heard::Data(delivery)) = self.exchange(&transmission)? {
                report.acknowledged = delivery.acknowledged();
                report.delivery = Some(delivery);
                return Ok(report);
            }
            let closed = self.clock.now_us();
            match self.device.nothing_heard(closed)? {
                Next::Repeat { not_before_us } => loop {
                    self.wait_until(not_before_us);
                    let now = self.clock.now_us();
                    match self.device.repeat(now) {
                        Ok(next) => {
                            transmission = next;
                            break;
                        }
                        Err(DeviceError::Wait { until_us }) => self.wait_until(until_us),
                        Err(error) => return Err(error.into()),
                    }
                },
                _ => return Ok(report),
            }
        }
    }

    /// Puts one transmission on the air and listens in its windows.
    ///
    /// Under a relay the frame goes out behind the wake-on-radio frame that names it, and a
    /// third window carries what the relay forwards back.
    fn exchange(
        &mut self,
        transmission: &Transmission,
    ) -> Result<Option<Heard>, NodeError<R::Error>> {
        let relay = match transmission.relay {
            Some(exchange) => Some(self.wake_relay(exchange)?),
            None => None,
        };
        self.wait_until(relay.map_or(0, |exchange: RelayExchange| exchange.uplink_start_us));
        self.radio
            .tune(self.tuning(
                transmission.frequency_hz,
                transmission.link,
                transmission.output_dbm,
                false,
            ))
            .map_err(NodeError::Radio)?;
        self.radio
            .transmit(transmission.frame.as_bytes())
            .map_err(NodeError::Radio)?;
        let ended_us = self.clock.now_us();

        let mut heard = None;
        let windows = [
            Some((ReceiveWindow::Rx1, transmission.rx1)),
            Some((ReceiveWindow::Rx2, transmission.rx2)),
            relay.map(|exchange| (ReceiveWindow::Rxr, exchange.rxr)),
        ];
        for (which, window) in windows.into_iter().flatten() {
            if let Some(found) = self.listen(ended_us, which, &window, transmission.output_dbm)? {
                heard = Some(found);
                break;
            }
        }
        self.radio.sleep().map_err(NodeError::Radio)?;
        Ok(heard)
    }

    /// Wakes the relay ahead of an uplink, TS011-1.0.1 sections 3.2 and 3.7.
    ///
    /// The frame goes out at the time the device chose, and the acknowledgment window opens
    /// after it. Without an answer the device says whether to wake the relay again or let
    /// the uplink go anyway.
    ///
    /// # Returns
    ///
    /// The exchange the uplink is timed from, which is the last one attempted.
    fn wake_relay(
        &mut self,
        mut exchange: RelayExchange,
    ) -> Result<RelayExchange, NodeError<R::Error>> {
        loop {
            let wake_up = exchange.wake_up;
            self.wait_until(wake_up.start_us);
            self.radio
                .tune(Tuning {
                    frequency_hz: wake_up.carrier.frequency_hz,
                    link: wake_up.link,
                    output_dbm: wake_up.output_dbm,
                    sync_word: lorawan_sync_word(&wake_up.link),
                    // RP002-1.0.5 table 124: a wake-on-radio frame goes out with the
                    // inverted polarity of a downlink, so an uplink preamble never wakes a
                    // relay.
                    invert_iq_transmit: true,
                    invert_iq_receive: true,
                    band_hz: self.band_hz.unwrap_or_else(|| self.device.frequency_span()),
                })
                .map_err(NodeError::Radio)?;
            self.radio
                .transmit(wake_up.frame())
                .map_err(NodeError::Radio)?;

            let Some(ack) = exchange.ack else {
                return Ok(exchange);
            };
            if self.listen_for_ack(&ack)? {
                return Ok(exchange);
            }
            let now = self.clock.now_us();
            match self.device.no_wor_ack(now)? {
                WorNext::Uplink => return Ok(exchange),
                WorNext::WakeUp(again) => exchange = again,
            }
        }
    }

    /// Opens the window a relay's acknowledgment would arrive in, and reads it.
    ///
    /// # Returns
    ///
    /// `true` when one arrived and verified.
    fn listen_for_ack(&mut self, ack: &AckWindow) -> Result<bool, NodeError<R::Error>> {
        let (symbols, offset_us) = window_parameters(
            ack.link.symbol_time_us(),
            self.min_rx_symbols,
            self.max_rx_error_us,
        );
        let length_us = u64::from(symbols) * ack.link.symbol_time_us();
        let opens_us = (ack.start_us as i64 + offset_us).max(0) as u64;
        self.radio
            .tune(Tuning {
                frequency_hz: ack.carrier.frequency_hz,
                link: ack.link,
                output_dbm: 0,
                sync_word: lorawan_sync_word(&ack.link),
                invert_iq_transmit: true,
                invert_iq_receive: true,
                band_hz: self.band_hz.unwrap_or_else(|| self.device.frequency_span()),
            })
            .map_err(NodeError::Radio)?;
        self.wait_until(opens_us);

        let timeout_us = length_us.saturating_add(ack.airtime_us);
        match self
            .radio
            .receive(&mut self.buffer, timeout_us)
            .map_err(NodeError::Radio)?
        {
            Reception::Frame { len, .. } => {
                let len = len.min(MAX_FRAME);
                Ok(self.device.heard_wor_ack(&self.buffer[..len]).is_ok())
            }
            Reception::Nothing => Ok(false),
        }
    }

    /// Opens one window, and reads a frame for this device if one arrives in it.
    fn listen(
        &mut self,
        ended_us: u64,
        which: ReceiveWindow,
        window: &Window,
        output_dbm: i8,
    ) -> Result<Option<Heard>, NodeError<R::Error>> {
        let (symbols, offset_us) = window_parameters(
            window.link.symbol_time_us(),
            self.min_rx_symbols,
            self.max_rx_error_us,
        );
        let symbol_us = window.link.symbol_time_us();
        let length_us = u64::from(symbols) * symbol_us;
        let opens_us = (ended_us as i64 + i64::from(window.delay_us) + offset_us).max(0) as u64;

        self.radio
            .tune(self.tuning(window.frequency_hz, window.link, output_dbm, true))
            .map_err(NodeError::Radio)?;
        let now = self.clock.now_us();
        if now > opens_us + length_us / 2 {
            return Ok(None);
        }
        if opens_us > now {
            wait(&mut self.clock, opens_us - now);
        }

        match self
            .radio
            .receive(&mut self.buffer, length_us)
            .map_err(NodeError::Radio)?
        {
            Reception::Frame { len, snr_db, .. } => {
                let len = len.min(MAX_FRAME);
                Ok(self
                    .device
                    .heard_in(which, &self.buffer[..len], snr_db)
                    .ok())
            }
            Reception::Nothing => Ok(None),
        }
    }

    fn tuning(
        &self,
        frequency_hz: u32,
        link: LinkSettings,
        output_dbm: i8,
        downlink: bool,
    ) -> Tuning {
        Tuning {
            frequency_hz,
            link,
            output_dbm,
            sync_word: lorawan_sync_word(&link),
            invert_iq_transmit: false,
            invert_iq_receive: downlink,
            band_hz: self.band_hz.unwrap_or_else(|| self.device.frequency_span()),
        }
    }
}

/// Waits a number of microseconds, in steps a `u32` of them can hold.
pub(crate) fn wait(delay: &mut impl DelayNs, mut us: u64) {
    while us > 0 {
        let step = us.min(u64::from(u32::MAX));
        delay.delay_us(step as u32);
        us -= step;
    }
}

/// The sync word a LoRaWAN link uses, from RP002-1.0.5 table 112: 0x34 from SF12 to SF7, and
/// 0x12 at SF6 and SF5.
///
/// # Arguments
///
/// * `link` - the link.
///
/// # Returns
///
/// The sync word.
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
/// use pamoja_radios::lorawan::lorawan_sync_word;
/// use pamoja_radios::radio::SyncWord;
///
/// assert_eq!(lorawan_sync_word(&LinkSettings::new(7, 125_000)), SyncWord::Public);
/// assert_eq!(lorawan_sync_word(&LinkSettings::new(6, 125_000)), SyncWord::Private);
/// ```
pub fn lorawan_sync_word(link: &LinkSettings) -> SyncWord {
    if link.spreading_factor() >= 7 {
        SyncWord::Public
    } else {
        SyncWord::Private
    }
}

/// How long a receive window listens, and how far from its nominal opening it starts.
///
/// This is LoRaMac-node's `RegionCommonComputeRxWindowParameters`: the window holds at least
/// `min_rx_symbols` symbols, and enough for `(2 * min_rx_symbols - 8)` of them plus the timing
/// error twice over; it opens so its middle falls on the fourth of the eight preamble
/// symbols.
///
/// # Arguments
///
/// * `symbol_us` - one symbol, in microseconds.
/// * `min_rx_symbols` - the fewest preamble symbols to catch.
/// * `max_rx_error_us` - the timing error either way.
///
/// # Returns
///
/// The window length in symbols, and the offset from the nominal opening in microseconds,
/// negative to open early.
///
/// # Examples
///
/// ```
/// use pamoja_radios::lorawan::window_parameters;
///
/// // SF7 at 125 kHz: 1024 us symbols, so 24 of them, opening a little over 8 ms early.
/// assert_eq!(window_parameters(1_024, 6, 10_000), (24, -8_192));
/// // SF12: the six-symbol floor wins, and the window opens after its nominal time.
/// assert_eq!(window_parameters(32_768, 6, 10_000), (6, 32_768));
/// ```
pub fn window_parameters(symbol_us: u64, min_rx_symbols: u8, max_rx_error_us: u32) -> (u32, i64) {
    let symbol_us = symbol_us.max(1);
    let span =
        (2 * i64::from(min_rx_symbols) - 8) * symbol_us as i64 + 2 * i64::from(max_rx_error_us);
    let needed = (span.max(0) as u64).div_ceil(symbol_us);
    let symbols = needed.max(u64::from(min_rx_symbols)) as u32;
    let half = (u64::from(symbols) * symbol_us).div_ceil(2);
    let offset = 4 * symbol_us as i64 - half as i64;
    (symbols, offset)
}

impl<SPI, BUSY, RESET, D> Transceiver for Radio<SPI, BUSY, RESET, D>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    RESET: OutputPin,
    D: DelayNs,
{
    type Error = radio::RadioError<SPI::Error>;

    fn tune(&mut self, tuning: Tuning) -> Result<(), Self::Error> {
        let (low, high) = tuning.band_hz;
        self.configure(
            RadioConfig::new(tuning.frequency_hz, tuning.link, tuning.output_dbm)
                .with_band(low, high)
                .with_sync_word(tuning.sync_word)
                .with_inverted_iq(tuning.invert_iq_transmit, tuning.invert_iq_receive),
        )
    }

    fn transmit(&mut self, frame: &[u8]) -> Result<(), Self::Error> {
        Radio::transmit(self, frame).map(|_| ())
    }

    fn receive(&mut self, buffer: &mut [u8], timeout_us: u64) -> Result<Reception, Self::Error> {
        Ok(match Radio::receive(self, buffer, timeout_us)? {
            radio::Reception::Frame { len, levels } => Reception::Frame {
                len,
                snr_db: levels.snr_db.round_db().clamp(-128, 127) as i8,
                rssi_dbm: levels.rssi_dbm.round_db().clamp(-32_768, 32_767) as i16,
            },
            _ => Reception::Nothing,
        })
    }

    fn detect(&mut self, symbols: u8) -> Result<bool, Self::Error> {
        Radio::detect(self, symbols)
    }

    fn sleep(&mut self) -> Result<(), Self::Error> {
        Radio::sleep(self)
    }
}

impl<SPI, BUSY, RESET, D> Transceiver for Sx126x<SPI, BUSY, RESET, D>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    RESET: OutputPin,
    D: DelayNs,
{
    type Error = sx126x::RadioError<SPI::Error>;

    fn tune(&mut self, tuning: Tuning) -> Result<(), Self::Error> {
        let (low, high) = tuning.band_hz;
        let power = self.tx_power(tuning.output_dbm);
        self.configure(
            sx126x::RadioConfig::new(tuning.frequency_hz, tuning.link, power)
                .with_band(low, high)
                .with_sync_word(tuning.sync_word.sx126x())
                .with_inverted_iq(tuning.invert_iq_transmit, tuning.invert_iq_receive),
        )
    }

    fn transmit(&mut self, frame: &[u8]) -> Result<(), Self::Error> {
        Sx126x::transmit(self, frame).map(|_| ())
    }

    fn receive(&mut self, buffer: &mut [u8], timeout_us: u64) -> Result<Reception, Self::Error> {
        Ok(match Sx126x::receive(self, buffer, timeout_us)? {
            sx126x::Reception::Frame { len, status } => Reception::Frame {
                len,
                snr_db: status.snr_db.round_db().clamp(-128, 127) as i8,
                rssi_dbm: status.rssi_dbm.round_db().clamp(-32_768, 32_767) as i16,
            },
            _ => Reception::Nothing,
        })
    }

    fn detect(&mut self, symbols: u8) -> Result<bool, Self::Error> {
        Sx126x::detect(self, symbols)
    }

    fn sleep(&mut self) -> Result<(), Self::Error> {
        Sx126x::sleep(self, true)
    }
}

impl<SPI, RESET, D> Transceiver for Sx127x<SPI, RESET, D>
where
    SPI: SpiDevice,
    RESET: OutputPin,
    D: DelayNs,
{
    type Error = sx127x::RadioError<SPI::Error>;

    fn tune(&mut self, tuning: Tuning) -> Result<(), Self::Error> {
        let power = self.tx_power(tuning.output_dbm);
        self.configure(
            sx127x::RadioConfig::new(tuning.frequency_hz, tuning.link, power)
                .with_sync_word(tuning.sync_word.sx127x())
                .with_inverted_iq(tuning.invert_iq_transmit, tuning.invert_iq_receive),
        )
    }

    fn transmit(&mut self, frame: &[u8]) -> Result<(), Self::Error> {
        Sx127x::transmit(self, frame).map(|_| ())
    }

    fn receive(&mut self, buffer: &mut [u8], timeout_us: u64) -> Result<Reception, Self::Error> {
        Ok(match Sx127x::receive(self, buffer, timeout_us)? {
            sx127x::Reception::Frame { len, status } => Reception::Frame {
                len,
                snr_db: status.snr_db.round_db().clamp(-128, 127) as i8,
                rssi_dbm: status.rssi_dbm.round_db().clamp(-32_768, 32_767) as i16,
            },
            _ => Reception::Nothing,
        })
    }

    fn detect(&mut self, _symbols: u8) -> Result<bool, Self::Error> {
        Sx127x::detect(self)
    }

    fn sleep(&mut self) -> Result<(), Self::Error> {
        Sx127x::sleep(self)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::convert::Infallible;
    use std::rc::Rc;
    use std::vec::Vec;

    use pamoja_lora::region::Region;
    use pamoja_lorawan::device::Settings;
    use pamoja_lorawan::mac::{encode_all, MacCommand};
    use pamoja_lorawan::{Device, Downlink, JoinGrant, JoinRequest, Session};

    use super::*;

    const APP_KEY: [u8; 16] = [0x2B; 16];
    const DEV_ADDR: u32 = 0x2601_2E43;

    #[derive(Clone, Debug, PartialEq)]
    enum Call {
        Tune(Tuning),
        Transmit(u64, Vec<u8>),
        Receive(u64, u64),
        Sleep,
    }

    /// Which window a network answers in.
    #[derive(Clone, Copy, PartialEq)]
    enum Answer {
        Rx1,
        Rx2,
        Silent,
    }

    /// A radio over a network that answers joins and data, timestamped by a shared clock.
    struct Air {
        now: Rc<Cell<u64>>,
        calls: Rc<RefCell<Vec<Call>>>,
        answer: Answer,
        foreign_first: bool,
        session: Option<Session>,
        fcnt_down: u32,
        pending: Option<Vec<u8>>,
        receives: u32,
        commands: Vec<MacCommand>,
        payload: Vec<u8>,
    }

    impl Transceiver for Air {
        type Error = Infallible;

        fn tune(&mut self, tuning: Tuning) -> Result<(), Infallible> {
            self.calls.borrow_mut().push(Call::Tune(tuning));
            Ok(())
        }

        fn transmit(&mut self, frame: &[u8]) -> Result<(), Infallible> {
            self.now.set(self.now.get() + 50_000);
            self.calls
                .borrow_mut()
                .push(Call::Transmit(self.now.get(), frame.to_vec()));
            self.receives = 0;
            self.pending = if let Ok(request) = JoinRequest::parse(frame, &APP_KEY) {
                let grant = JoinGrant::new(0x01, 0x13, DEV_ADDR);
                self.session = Some(grant.session(&APP_KEY, request.dev_nonce()));
                Some(
                    grant
                        .accept(&APP_KEY, request.dev_nonce())
                        .as_bytes()
                        .to_vec(),
                )
            } else if let Some(session) = self.session {
                let mut fopts = [0u8; 15];
                let len = encode_all(&self.commands, &mut fopts).expect("fits");
                let downlink = if self.payload.is_empty() {
                    Downlink::empty(self.fcnt_down)
                } else {
                    Downlink::new(self.fcnt_down, 3, &self.payload)
                };
                let downlink = session
                    .encode_downlink(&downlink.with_fopts(&fopts[..len]))
                    .expect("encodes");
                self.fcnt_down += 1;
                self.commands.clear();
                Some(downlink.as_bytes().to_vec())
            } else {
                None
            };
            Ok(())
        }

        fn receive(&mut self, buffer: &mut [u8], timeout_us: u64) -> Result<Reception, Infallible> {
            self.calls
                .borrow_mut()
                .push(Call::Receive(self.now.get(), timeout_us));
            self.receives += 1;
            let window = if self.receives == 1 {
                Answer::Rx1
            } else {
                Answer::Rx2
            };
            if self.foreign_first && self.receives == 1 {
                let stranger = Session::new(DEV_ADDR + 1, [1; 16], [2; 16])
                    .encode_downlink(&Downlink::empty(0))
                    .expect("encodes");
                let bytes = stranger.as_bytes();
                buffer[..bytes.len()].copy_from_slice(bytes);
                return Ok(Reception::Frame {
                    len: bytes.len(),
                    snr_db: 3,
                    rssi_dbm: -100,
                });
            }
            if window != self.answer {
                self.now.set(self.now.get() + timeout_us);
                return Ok(Reception::Nothing);
            }
            match self.pending.take() {
                Some(frame) => {
                    buffer[..frame.len()].copy_from_slice(&frame);
                    Ok(Reception::Frame {
                        len: frame.len(),
                        snr_db: 8,
                        rssi_dbm: -95,
                    })
                }
                None => Ok(Reception::Nothing),
            }
        }

        fn sleep(&mut self) -> Result<(), Infallible> {
            self.calls.borrow_mut().push(Call::Sleep);
            Ok(())
        }
    }

    struct Ticks(Rc<Cell<u64>>);

    impl Clock for Ticks {
        fn now_us(&mut self) -> u64 {
            self.0.get()
        }
    }

    impl DelayNs for Ticks {
        fn delay_ns(&mut self, ns: u32) {
            self.0.set(self.0.get() + u64::from(ns).div_ceil(1000));
        }

        fn delay_us(&mut self, us: u32) {
            self.0.set(self.0.get() + u64::from(us));
        }
    }

    type Fixture = (
        Node<'static, Air, Ticks>,
        Rc<RefCell<Vec<Call>>>,
        Rc<Cell<u64>>,
    );

    fn node(answer: Answer) -> Fixture {
        let now = Rc::new(Cell::new(1_000_000));
        let calls = Rc::new(RefCell::new(Vec::new()));
        let air = Air {
            now: now.clone(),
            calls: calls.clone(),
            answer,
            foreign_first: false,
            session: None,
            fcnt_down: 0,
            pending: None,
            receives: 0,
            commands: Vec::new(),
            payload: Vec::new(),
        };
        let device = EndDevice::new(
            Region::Eu868.plan(),
            Device::new([0x11; 8], [0x22; 8], APP_KEY),
            Settings::new(2, 20).without_regional_duty_cycle(),
        )
        .expect("dynamic");
        (Node::new(device, air, Ticks(now.clone())), calls, now)
    }

    fn transmitted_at(calls: &[Call]) -> u64 {
        calls
            .iter()
            .find_map(|call| match call {
                Call::Transmit(at, _) => Some(*at),
                _ => None,
            })
            .expect("a transmission")
    }

    fn receives(calls: &[Call]) -> Vec<(u64, u64)> {
        calls
            .iter()
            .filter_map(|call| match call {
                Call::Receive(at, timeout) => Some((*at, *timeout)),
                _ => None,
            })
            .collect()
    }

    fn tunings(calls: &[Call]) -> Vec<Tuning> {
        calls
            .iter()
            .filter_map(|call| match call {
                Call::Tune(tuning) => Some(*tuning),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_join_accept_in_the_first_window_is_caught_where_the_window_opens() {
        let (mut node, calls, _) = node(Answer::Rx1);
        assert_eq!(node.join(1), Ok(true));
        let calls = calls.borrow();

        let tuned = tunings(&calls);
        assert!(!tuned[0].invert_iq_receive && !tuned[0].invert_iq_transmit);
        assert_eq!(tuned[0].sync_word, SyncWord::Public);
        assert_eq!(tuned[0].link.spreading_factor(), 7);
        assert!(
            tuned[1].invert_iq_receive,
            "a downlink is heard with inverted IQ"
        );
        assert_eq!(tuned[1].frequency_hz, tuned[0].frequency_hz);
        assert!(!tuned[1].link.crc());
        assert_eq!(
            tuned[0].band_hz,
            (868_100_000, 869_525_000),
            "calibrated for the European channels and the second window"
        );

        // SF7: a 24-symbol window opening 8192 us before JOIN_ACCEPT_DELAY1.
        let ended = transmitted_at(&calls);
        assert_eq!(receives(&calls), [(ended + 5_000_000 - 8_192, 24 * 1_024)]);
        assert_eq!(calls.last(), Some(&Call::Sleep));
    }

    #[test]
    fn nothing_in_the_first_window_opens_the_second() {
        let (mut node, calls, _) = node(Answer::Rx2);
        assert_eq!(node.join(1), Ok(true));
        let calls = calls.borrow();
        let ended = transmitted_at(&calls);

        // RX2 is SF12 at 869.525 MHz: six symbols, opening 32768 us after its delay.
        let tuned = tunings(&calls);
        assert_eq!(tuned[2].frequency_hz, 869_525_000);
        assert_eq!(tuned[2].link.spreading_factor(), 12);
        let windows = receives(&calls);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[1], (ended + 6_000_000 + 32_768, 6 * 32_768));
    }

    #[test]
    fn no_answer_in_either_window_leaves_the_device_to_try_again() {
        let (mut node, calls, _) = node(Answer::Silent);
        assert_eq!(node.join(1), Ok(false));
        assert!(!node.device().is_joined());
        assert_eq!(receives(&calls.borrow()).len(), 2);
    }

    #[test]
    fn a_frame_for_another_device_in_the_first_window_still_leaves_the_second() {
        let (mut node, calls, _) = node(Answer::Rx2);
        node.radio_mut().foreign_first = true;
        assert_eq!(node.join(1), Ok(true));
        assert_eq!(receives(&calls.borrow()).len(), 2);
    }

    #[test]
    fn a_frame_longer_than_the_window_it_arrived_in_carries_is_not_taken() {
        // TS001-1.0.4 section 4.1. The second window listens at DR0 here, where RP002-1.0.5
        // table 16 holds a MACPayload to 59 bytes; a 52-byte payload makes one of 60.
        let (mut node, _, now) = node(Answer::Rx1);
        assert_eq!(node.join(1), Ok(true));
        now.set(now.get() + 600_000_000);
        node.radio_mut().answer = Answer::Rx2;
        node.radio_mut().payload = vec![0x5A; 52];
        let long = node.send(2, b"21.5", false).expect("sends");
        assert!(long.delivery.is_none());

        now.set(now.get() + 600_000_000);
        node.radio_mut().payload = vec![0x5A; 51];
        let full = node.send(2, b"21.6", false).expect("sends");
        assert_eq!(
            full.delivery.map(|delivery| delivery.payload().len()),
            Some(51)
        );
    }

    #[test]
    fn a_send_is_seen_through_its_repeats() {
        let (mut node, calls, now) = node(Answer::Rx1);
        assert_eq!(node.join(1), Ok(true));
        now.set(now.get() + 600_000_000);

        // The network answers the first uplink by asking for two transmissions of each.
        node.radio_mut().commands = vec![MacCommand::LinkAdrReq {
            data_rate: 5,
            tx_power: 0,
            channel_mask: 0b111,
            mask_control: 0,
            transmissions: 2,
        }];
        let first = node.send(2, b"21.5", false).expect("sends");
        assert_eq!(first.transmissions, 1);
        assert!(first.delivery.is_some());

        // Then it goes quiet, so the next uplink goes out twice, the same frame both times.
        node.radio_mut().answer = Answer::Silent;
        now.set(now.get() + 600_000_000);
        calls.borrow_mut().clear();
        let second = node.send(2, b"21.6", false).expect("sends");
        assert_eq!(second.transmissions, 2);
        assert!(second.delivery.is_none());
        let frames: Vec<Vec<u8>> = calls
            .borrow()
            .iter()
            .filter_map(|call| match call {
                Call::Transmit(_, frame) => Some(frame.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], frames[1]);
    }

    #[test]
    fn a_window_is_sized_as_loramac_node_sizes_it() {
        // SF9 at 125 kHz: 4096 us symbols, and (4 * 4096 + 20000) / 4096 rounds up to 9.
        assert_eq!(window_parameters(4_096, 6, 10_000), (9, -2_048));
        // A perfect clock still keeps the six-symbol floor.
        assert_eq!(window_parameters(1_024, 6, 0), (6, 1_024));
    }

    #[test]
    fn a_timer_pairs_a_clock_with_a_delay() {
        let ticks = Rc::new(Cell::new(5));
        let mut timer = Timer::new(|| 9, Ticks(ticks.clone()));
        assert_eq!(timer.now_us(), 9);
        timer.delay_us(10);
        assert_eq!(ticks.get(), 15);
    }

    /// A radio for a node under a relay: it answers the wake-on-radio frame with the
    /// acknowledgment a real relay builds, and carries the forwarded downlink in the relay
    /// window.
    struct Relayed {
        now: Rc<Cell<u64>>,
        calls: Rc<RefCell<Vec<Call>>>,
        relay: pamoja_lorawan::relay::Relay<'static>,
        scan: Option<pamoja_lorawan::relay::Scan>,
        frequency_hz: u32,
        answer: Option<Vec<u8>>,
        acknowledge: bool,
    }

    impl Transceiver for Relayed {
        type Error = Infallible;

        fn tune(&mut self, tuning: Tuning) -> Result<(), Infallible> {
            self.frequency_hz = tuning.frequency_hz;
            self.calls.borrow_mut().push(Call::Tune(tuning));
            Ok(())
        }

        fn transmit(&mut self, frame: &[u8]) -> Result<(), Infallible> {
            self.calls
                .borrow_mut()
                .push(Call::Transmit(self.now.get(), frame.to_vec()));
            // A wake-on-radio frame goes to the relay, which answers it.
            if self.frequency_hz == 865_100_000 && self.acknowledge {
                let scan = self
                    .relay
                    .next_scan(self.now.get().saturating_sub(1_000_000))
                    .expect("the relay is running");
                let heard = self.relay.heard_wor(&scan, frame, -90, 4, self.now.get());
                self.scan = Some(scan);
                if let Ok(pamoja_lorawan::relay::Wake::Uplink {
                    acknowledgment: Some(acknowledgment),
                    ..
                }) = heard
                {
                    self.answer = Some(acknowledgment.frame.to_vec());
                }
            }
            Ok(())
        }

        fn receive(&mut self, buffer: &mut [u8], timeout_us: u64) -> Result<Reception, Infallible> {
            self.calls
                .borrow_mut()
                .push(Call::Receive(self.now.get(), timeout_us));
            match self.answer.take() {
                Some(frame) => {
                    buffer[..frame.len()].copy_from_slice(&frame);
                    Ok(Reception::Frame {
                        len: frame.len(),
                        snr_db: 7,
                        rssi_dbm: -90,
                    })
                }
                None => {
                    self.now.set(self.now.get() + timeout_us);
                    Ok(Reception::Nothing)
                }
            }
        }

        fn sleep(&mut self) -> Result<(), Infallible> {
            self.calls.borrow_mut().push(Call::Sleep);
            Ok(())
        }
    }

    /// A relay that trusts the device these tests run.
    fn relay_for(session: Session) -> pamoja_lorawan::relay::Relay<'static> {
        use pamoja_lorawan::relay::{CadToRx, Relay, RelayConfig, RelaySettings, XtalAccuracy};
        let plan = Region::Eu868.plan();
        let device = EndDevice::personalized(
            plan,
            Session::new(0x2601_0001, [0x11; 16], [0x22; 16]),
            Settings::new(2, 14)
                .with_seed(5)
                .with_tuning_range(863_000_000, 870_000_000),
        )
        .expect("a device");
        let mut relay = Relay::new(
            device,
            RelaySettings::new(XtalAccuracy::Ppm20, CadToRx::Symbols4),
        );
        relay
            .start(RelayConfig::region_default(plan).expect("relay channels"))
            .expect("a configuration it can run");
        relay.trust(0, session.dev_addr(), &session.root_wor_s_key(), 0, 63, 0);
        relay
    }

    fn relayed_node(acknowledge: bool) -> (Node<'static, Relayed, Ticks>, Rc<RefCell<Vec<Call>>>) {
        let session = Session::new(DEV_ADDR, [0x5A; 16], [0xA5; 16]);
        let mut device =
            EndDevice::personalized(Region::Eu868.plan(), session, Settings::new(2, 14))
                .expect("a device");
        assert!(device.use_relay(true));

        let now = Rc::new(Cell::new(0));
        let calls = Rc::new(RefCell::new(Vec::new()));
        let radio = Relayed {
            now: Rc::clone(&now),
            calls: Rc::clone(&calls),
            relay: relay_for(session),
            scan: None,
            frequency_hz: 0,
            answer: None,
            acknowledge,
        };
        (Node::new(device, radio, Ticks(now)), calls)
    }

    #[test]
    fn a_node_under_a_relay_wakes_it_first_and_listens_in_three_windows() {
        let (mut node, calls) = relayed_node(true);
        node.send(2, b"21.5", false).expect("an uplink");

        let calls = calls.borrow();
        let sent: Vec<_> = calls
            .iter()
            .filter_map(|call| match call {
                Call::Transmit(at, frame) => Some((*at, frame.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(sent.len(), 2, "the wake-on-radio frame, then the uplink");
        assert_eq!(sent[0].1.len(), 15, "a wake-on-radio uplink");
        assert!(
            sent[1].0 > sent[0].0,
            "the uplink follows the frame that woke the relay",
        );

        let tuned: Vec<_> = calls
            .iter()
            .filter_map(|call| match call {
                Call::Tune(tuning) => Some((tuning.frequency_hz, tuning.invert_iq_transmit)),
                _ => None,
            })
            .collect();
        assert_eq!(
            tuned[0],
            (865_100_000, true),
            "a wake-on-radio frame goes out inverted, RP002-1.0.5 table 124",
        );
        assert_eq!(tuned[1].0, 865_300_000, "then the acknowledgment window");
        assert!(!tuned[2].1, "the uplink itself goes out as any uplink does");

        // The acknowledgment was read, so the device knows when the relay scans and its
        // next frame is short.
        assert_eq!(
            node.device().relay_sync(),
            pamoja_lorawan::relay::RelaySync::Synchronized
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| matches!(call, Call::Receive(..)))
                .count(),
            4,
            "the acknowledgment window and the three after the uplink",
        );
    }

    #[test]
    fn a_node_whose_relay_never_answers_still_sends_its_uplink() {
        let (mut node, calls) = relayed_node(false);
        node.send(2, b"21.5", false).expect("an uplink");

        let calls = calls.borrow();
        let sent = calls
            .iter()
            .filter(|call| matches!(call, Call::Transmit(..)))
            .count();
        assert_eq!(sent, 2, "the frame that went unanswered, and the uplink");
        assert_eq!(
            node.device().relay_sync(),
            pamoja_lorawan::relay::RelaySync::Initialized,
            "nothing was learned about the relay",
        );
    }
}
