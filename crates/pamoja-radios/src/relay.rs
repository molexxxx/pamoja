//! A LoRaWAN relay with a radio: it scans, answers, forwards, and sends the network's
//! answers on, TS011-1.0.1.
//!
//! [`pamoja_lorawan::relay::Relay`] decides everything a relay does and owns no radio;
//! [`RelayNode`] gives it one and a clock, as [`Node`](crate::lorawan::Node) does for an end
//! device. One turn of the loop is [`scan`](RelayNode::scan): it sleeps until the next scan
//! slot, listens for a preamble over a few symbols, and only if one is there does it
//! receive the wake-on-radio frame, answer it, take the uplink behind it, forward that to
//! the network, and send whatever comes back in the end device's relay window.
//!
//! ```no_run
//! # use pamoja_lora::region::Region;
//! # use pamoja_lorawan::device::{EndDevice, Settings};
//! # use pamoja_lorawan::relay::{CadToRx, Relay, RelayConfig, RelaySettings, XtalAccuracy};
//! # use pamoja_lorawan::Session;
//! # use pamoja_radios::relay::{RelayNode, Scanned};
//! # fn run<R, C>(radio: R, clock: C) -> Result<(), Box<dyn std::error::Error>>
//! # where
//! #     R: pamoja_radios::lorawan::Transceiver,
//! #     R::Error: 'static,
//! #     C: pamoja_radios::lorawan::Clock + embedded_hal::delay::DelayNs,
//! # {
//! let plan = Region::Eu868.plan();
//! let session = Session::new(0x2601_0001, [0x11; 16], [0x22; 16]);
//! let device = EndDevice::personalized(plan, session, Settings::new(2, 14))?;
//! let mut relay = Relay::new(device, RelaySettings::new(XtalAccuracy::Ppm20, CadToRx::Symbols4));
//! relay.start(RelayConfig::region_default(plan).expect("EU868 has relay channels"))?;
//!
//! let mut node = RelayNode::new(relay, radio, clock);
//! loop {
//!     match node.scan()? {
//!         Scanned::Quiet => continue,
//!         Scanned::Forwarded { dev_addr, .. } => println!("forwarded for {dev_addr:08X?}"),
//!         other => println!("{other:?}"),
//!     }
//! }
//! # }
//! ```

use core::fmt;

use embedded_hal::delay::DelayNs;
use pamoja_lora::LinkSettings;
use pamoja_lorawan::device::{Delivery, ReceiveWindow};
use pamoja_lorawan::relay::{
    Acknowledgment, Listen, Relay, RelayError, RelayHeard, RxrDownlink, Wake,
};
use pamoja_lorawan::MAX_FRAME;

use crate::lorawan::{lorawan_sync_word, window_parameters, Clock, Reception, Transceiver, Tuning};

/// How many symbols a scan listens over by default.
///
/// Semtech's own relay listens over two to eight, by spreading factor and bandwidth; two is
/// what it uses on the 125 kHz channels of the slower spreading factors, which is where the
/// wake-on-radio channels of most regions are.
pub const DETECTION_SYMBOLS: u8 = 2;

/// What one scan came to.
// A delivery holds its payload inline, since the crate runs without an allocator.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scanned {
    /// Nothing was on the air.
    Quiet,
    /// Something was, but no wake-on-radio frame came of it.
    Nothing,
    /// A device the relay does not trust woke it, and the network will hear of it with the
    /// relay's next uplink.
    Notified {
        /// The address the frame named.
        dev_addr: u32,
    },
    /// A wake-on-radio frame was answered or listened to, and nothing followed.
    Woken {
        /// The device, or `None` for a join request, which carries no address.
        dev_addr: Option<u32>,
    },
    /// An uplink was forwarded to the network.
    Forwarded {
        /// The device, or `None` for a join request.
        dev_addr: Option<u32>,
        /// What the network sent back for the relay itself, if anything.
        delivery: Option<Delivery>,
        /// Whether a downlink went on to the end device in its relay window.
        answered: bool,
    },
}

/// What stopped a relay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayNodeError<E> {
    /// The radio failed.
    Radio(E),
    /// The relay refused.
    Relay(RelayError),
}

impl<E: fmt::Debug> fmt::Display for RelayNodeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelayNodeError::Radio(error) => write!(f, "the radio failed: {error:?}"),
            RelayNodeError::Relay(error) => write!(f, "{error}"),
        }
    }
}

impl<E: fmt::Debug> core::error::Error for RelayNodeError<E> {}

impl<E> From<RelayError> for RelayNodeError<E> {
    fn from(error: RelayError) -> RelayNodeError<E> {
        RelayNodeError::Relay(error)
    }
}

/// A frame a relay received, with what its radio heard of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Received {
    len: usize,
    snr_db: i8,
    rssi_dbm: i16,
}

/// A LoRaWAN relay: the relay, a radio, and the time.
///
/// See the [module documentation](self).
pub struct RelayNode<'p, R, C> {
    relay: Relay<'p>,
    radio: R,
    clock: C,
    band_hz: Option<(u32, u32)>,
    min_rx_symbols: u8,
    max_rx_error_us: u32,
    detection_symbols: u8,
    buffer: [u8; MAX_FRAME],
}

impl<'p, R, C> RelayNode<'p, R, C>
where
    R: Transceiver,
    C: Clock + DelayNs,
{
    /// A relay with a radio.
    ///
    /// # Arguments
    ///
    /// * `relay` - the relay, started or waiting for its network to start it.
    /// * `radio` - the radio, initialized.
    /// * `clock` - the time, and a way to wait.
    ///
    /// # Returns
    ///
    /// The relay node.
    pub fn new(relay: Relay<'p>, radio: R, clock: C) -> RelayNode<'p, R, C> {
        RelayNode {
            relay,
            radio,
            clock,
            band_hz: None,
            min_rx_symbols: crate::lorawan::MIN_RX_SYMBOLS,
            max_rx_error_us: crate::lorawan::MAX_RX_ERROR_US,
            detection_symbols: DETECTION_SYMBOLS,
            buffer: [0; MAX_FRAME],
        }
    }

    /// Sets the band an SX126x calibrates for, as [`Node::with_band`](crate::lorawan::Node::with_band) does.
    ///
    /// # Arguments
    ///
    /// * `low_hz` - the lower edge.
    /// * `high_hz` - the upper edge.
    ///
    /// # Returns
    ///
    /// The relay node, for chaining.
    #[must_use]
    pub fn with_band(mut self, low_hz: u32, high_hz: u32) -> RelayNode<'p, R, C> {
        self.band_hz = Some((low_hz, high_hz));
        self
    }

    /// Sets how long a receive window is and how early it opens, as
    /// [`Node::with_window_margin`](crate::lorawan::Node::with_window_margin) does.
    ///
    /// # Arguments
    ///
    /// * `min_rx_symbols` - the fewest symbols a window lasts.
    /// * `max_rx_error_us` - how far the relay's clock may be off over a window.
    ///
    /// # Returns
    ///
    /// The relay node, for chaining.
    #[must_use]
    pub fn with_window_margin(
        mut self,
        min_rx_symbols: u8,
        max_rx_error_us: u32,
    ) -> RelayNode<'p, R, C> {
        self.min_rx_symbols = min_rx_symbols;
        self.max_rx_error_us = max_rx_error_us;
        self
    }

    /// Sets how many symbols each scan listens over.
    ///
    /// # Arguments
    ///
    /// * `symbols` - the count, which an SX126x rounds down to 1, 2, 4, 8 or 16.
    ///
    /// # Returns
    ///
    /// The relay node, for chaining.
    #[must_use]
    pub fn with_detection(mut self, symbols: u8) -> RelayNode<'p, R, C> {
        self.detection_symbols = symbols;
        self
    }

    /// Returns the relay.
    ///
    /// # Returns
    ///
    /// The relay.
    pub const fn relay(&self) -> &Relay<'p> {
        &self.relay
    }

    /// Returns the relay, to configure or to send its own uplinks.
    ///
    /// # Returns
    ///
    /// The relay.
    pub fn relay_mut(&mut self) -> &mut Relay<'p> {
        &mut self.relay
    }

    /// Gives the relay, the radio and the clock back.
    ///
    /// # Returns
    ///
    /// The three of them.
    pub fn release(self) -> (Relay<'p>, R, C) {
        (self.relay, self.radio, self.clock)
    }

    /// Runs one scan, and sees through whatever it hears.
    ///
    /// The relay sleeps until its next scan slot, listens for a preamble over
    /// [`with_detection`](RelayNode::with_detection) symbols, and stops there if the
    /// channel is quiet. Otherwise it receives the wake-on-radio frame, sends the
    /// acknowledgment the relay builds, listens for the uplink behind it, forwards that to
    /// the network in its own uplink, and sends whatever the network answers with in the
    /// end device's relay window.
    ///
    /// # Returns
    ///
    /// What the scan came to.
    ///
    /// # Errors
    ///
    /// Returns [`RelayNodeError::Relay`] with [`RelayError::Stopped`] while the relay is
    /// not running, and [`RelayNodeError::Radio`] when the radio fails. A frame that does
    /// not decode, a device the relay does not know, and a forwarding limit are outcomes
    /// rather than errors.
    pub fn scan(&mut self) -> Result<Scanned, RelayNodeError<R::Error>> {
        let now = self.clock.now_us();
        let scan = self.relay.next_scan(now).ok_or(RelayError::Stopped)?;
        self.tune(scan.carrier.frequency_hz, scan.link, 0, true)?;
        self.wait_until(scan.start_us);
        if !self
            .radio
            .detect(self.detection_symbols)
            .map_err(RelayNodeError::Radio)?
        {
            self.radio.sleep().map_err(RelayNodeError::Radio)?;
            return Ok(Scanned::Quiet);
        }

        // Whatever is on the air may have begun a whole preamble ago, so the relay waits
        // out the longest one an end device sends on this channel and the frame behind it.
        let symbol_us = scan.link.symbol_time_us();
        let timeout_us = u64::from(scan.preamble_symbols)
            .saturating_mul(symbol_us)
            .saturating_add(scan.link.airtime_us(MAX_FRAME));
        let Some(heard) = self.receive(timeout_us)? else {
            self.radio.sleep().map_err(RelayNodeError::Radio)?;
            return Ok(Scanned::Nothing);
        };
        let ended_us = self.clock.now_us();

        let mut frame = [0u8; MAX_FRAME];
        frame[..heard.len].copy_from_slice(&self.buffer[..heard.len]);
        match self.relay.heard_wor(
            &scan,
            &frame[..heard.len],
            heard.rssi_dbm,
            heard.snr_db,
            ended_us,
        ) {
            Ok(Wake::JoinRequest { listen }) => self.follow(None, None, listen),
            Ok(Wake::Uplink {
                dev_addr,
                acknowledgment,
                listen,
                ..
            }) => match listen {
                Some(listen) => self.follow(Some(dev_addr), acknowledgment, listen),
                None => {
                    if let Some(acknowledgment) = acknowledgment {
                        self.acknowledge(&acknowledgment)?;
                    }
                    self.radio.sleep().map_err(RelayNodeError::Radio)?;
                    Ok(Scanned::Woken {
                        dev_addr: Some(dev_addr),
                    })
                }
            },
            Ok(Wake::Notified { dev_addr }) => {
                self.radio.sleep().map_err(RelayNodeError::Radio)?;
                Ok(Scanned::Notified { dev_addr })
            }
            Err(RelayError::Stopped | RelayError::Busy) => Err(RelayError::Stopped.into()),
            Err(_) => {
                self.radio.sleep().map_err(RelayNodeError::Radio)?;
                Ok(Scanned::Nothing)
            }
        }
    }

    /// Acknowledges a wake-on-radio frame, listens for the uplink it announced, and
    /// forwards what arrives.
    fn follow(
        &mut self,
        dev_addr: Option<u32>,
        acknowledgment: Option<Acknowledgment>,
        listen: Listen,
    ) -> Result<Scanned, RelayNodeError<R::Error>> {
        if let Some(acknowledgment) = acknowledgment {
            self.acknowledge(&acknowledgment)?;
        }

        self.tune(listen.carrier.frequency_hz, listen.link, 0, false)?;
        let (opens_us, length_us) = self.window(&listen.link, listen.start_us);
        self.wait_until(opens_us);
        let timeout_us = length_us.saturating_add(listen.link.airtime_us(listen.max_len));
        let Some(heard) = self.receive(timeout_us)? else {
            self.relay.uplink_missed();
            self.radio.sleep().map_err(RelayNodeError::Radio)?;
            return Ok(Scanned::Woken { dev_addr });
        };
        let ended_us = self.clock.now_us();

        let mut frame = [0u8; MAX_FRAME];
        frame[..heard.len].copy_from_slice(&self.buffer[..heard.len]);
        match self
            .relay
            .heard_uplink(&frame[..heard.len], heard.rssi_dbm, heard.snr_db, ended_us)
        {
            Ok(due_us) => self.forward(dev_addr, due_us),
            Err(_) => {
                self.radio.sleep().map_err(RelayNodeError::Radio)?;
                Ok(Scanned::Woken { dev_addr })
            }
        }
    }

    /// Sends the acknowledgment at the time the relay set for it.
    fn acknowledge(
        &mut self,
        acknowledgment: &Acknowledgment,
    ) -> Result<(), RelayNodeError<R::Error>> {
        self.tune(
            acknowledgment.carrier.frequency_hz,
            acknowledgment.link,
            acknowledgment.output_dbm,
            true,
        )?;
        self.wait_until(acknowledgment.start_us);
        self.radio
            .transmit(&acknowledgment.frame)
            .map_err(RelayNodeError::Radio)
    }

    /// Forwards the uplink the relay is holding, and sees the answer through to the end
    /// device's relay window.
    fn forward(
        &mut self,
        dev_addr: Option<u32>,
        due_us: u64,
    ) -> Result<Scanned, RelayNodeError<R::Error>> {
        self.wait_until(due_us);
        let now = self.clock.now_us();
        let transmission = self.relay.forward(now)?;
        self.tune(
            transmission.frequency_hz,
            transmission.link,
            transmission.output_dbm,
            false,
        )?;
        self.radio
            .transmit(transmission.frame.as_bytes())
            .map_err(RelayNodeError::Radio)?;
        let ended_us = self.clock.now_us();

        let mut delivery = None;
        let mut downlink = None;
        for (which, window) in [
            (ReceiveWindow::Rx1, transmission.rx1),
            (ReceiveWindow::Rx2, transmission.rx2),
        ] {
            self.tune(window.frequency_hz, window.link, 0, true)?;
            let (opens_us, length_us) = self.window(
                &window.link,
                ended_us.saturating_add(u64::from(window.delay_us)),
            );
            self.wait_until(opens_us);
            let Some(heard) = self.receive(length_us)? else {
                continue;
            };
            let mut frame = [0u8; MAX_FRAME];
            frame[..heard.len].copy_from_slice(&self.buffer[..heard.len]);
            match self
                .relay
                .heard_in(which, &frame[..heard.len], heard.snr_db)
            {
                Ok(RelayHeard::Downlink {
                    delivery: heard,
                    downlink: forwarded,
                }) => {
                    delivery = Some(heard);
                    downlink = Some(forwarded);
                }
                Ok(RelayHeard::Undeliverable {
                    delivery: heard, ..
                }) => delivery = Some(heard),
                Ok(RelayHeard::Device(pamoja_lorawan::device::Heard::Data(heard))) => {
                    delivery = Some(heard);
                }
                _ => continue,
            }
            break;
        }
        if delivery.is_none() {
            // Both windows closed with nothing, which settles the relay's own uplink.
            let closed = self.clock.now_us();
            let _ = self.relay.nothing_heard(closed);
        }

        let answered = match downlink {
            Some(downlink) => {
                self.send_on(&downlink)?;
                true
            }
            None => false,
        };
        self.radio.sleep().map_err(RelayNodeError::Radio)?;
        Ok(Scanned::Forwarded {
            dev_addr,
            delivery,
            answered,
        })
    }

    /// Sends a downlink on in the end device's relay window.
    fn send_on(&mut self, downlink: &RxrDownlink) -> Result<(), RelayNodeError<R::Error>> {
        self.tune(
            downlink.carrier.frequency_hz,
            downlink.link,
            downlink.output_dbm,
            true,
        )?;
        self.wait_until(downlink.start_us);
        self.radio
            .transmit(downlink.frame())
            .map_err(RelayNodeError::Radio)
    }

    /// Reads one frame, if one arrives inside the timeout.
    fn receive(&mut self, timeout_us: u64) -> Result<Option<Received>, RelayNodeError<R::Error>> {
        match self
            .radio
            .receive(&mut self.buffer, timeout_us)
            .map_err(RelayNodeError::Radio)?
        {
            Reception::Frame {
                len,
                snr_db,
                rssi_dbm,
            } => Ok(Some(Received {
                len: len.min(MAX_FRAME),
                snr_db,
                rssi_dbm,
            })),
            Reception::Nothing => Ok(None),
        }
    }

    /// When a window opens and how long it lasts, given the relay's own clock error.
    fn window(&self, link: &LinkSettings, at_us: u64) -> (u64, u64) {
        let (symbols, offset_us) = window_parameters(
            link.symbol_time_us(),
            self.min_rx_symbols,
            self.max_rx_error_us,
        );
        let opens_us = (at_us as i64 + offset_us).max(0) as u64;
        (opens_us, u64::from(symbols) * link.symbol_time_us())
    }

    /// Tunes the radio, with the inverted polarity every relay frame carries.
    fn tune(
        &mut self,
        frequency_hz: u32,
        link: LinkSettings,
        output_dbm: i8,
        inverted: bool,
    ) -> Result<(), RelayNodeError<R::Error>> {
        let band_hz = self
            .band_hz
            .unwrap_or_else(|| self.relay.device().frequency_span());
        self.radio
            .tune(Tuning {
                frequency_hz,
                link,
                output_dbm,
                sync_word: lorawan_sync_word(&link),
                invert_iq_transmit: inverted,
                invert_iq_receive: inverted,
                band_hz,
            })
            .map_err(RelayNodeError::Radio)
    }

    /// Waits until a time, if it is still ahead.
    fn wait_until(&mut self, until_us: u64) {
        let now = self.clock.now_us();
        if until_us > now {
            crate::lorawan::wait(&mut self.clock, until_us - now);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::convert::Infallible;
    use std::rc::Rc;
    use std::vec::Vec;

    use pamoja_lora::region::Region;
    use pamoja_lorawan::device::{EndDevice, Settings};
    use pamoja_lorawan::relay::{
        root_wor_s_key, CadToRx, ForwardedUplink, RelayConfig, RelaySettings, XtalAccuracy,
        LA_FPORT_RELAY,
    };
    use pamoja_lorawan::{Downlink, Session};

    use super::*;

    const DEV_ADDR: u32 = 0x2601_2E43;
    const NWK_S_KEY: [u8; 16] = [0x5A; 16];
    const APP_S_KEY: [u8; 16] = [0xA5; 16];
    const RELAY_ADDR: u32 = 0x2601_0001;

    /// What the relay's radio was asked to do.
    #[derive(Clone, Debug, PartialEq)]
    enum Call {
        Detect(u64),
        Transmit(u64, u32, Vec<u8>),
        Receive(u64, u32),
        Sleep,
    }

    /// A radio that hands the relay the frames a test lines up, and records what it sends.
    struct Air {
        now: Rc<Cell<u64>>,
        calls: Rc<RefCell<Vec<Call>>>,
        frequency_hz: u32,
        detects: bool,
        /// The frames waiting, each on a frequency or on whichever one is tuned.
        incoming: Vec<(Option<u32>, Vec<u8>)>,
    }

    impl Transceiver for Air {
        type Error = Infallible;

        fn tune(&mut self, tuning: Tuning) -> Result<(), Infallible> {
            self.frequency_hz = tuning.frequency_hz;
            Ok(())
        }

        fn transmit(&mut self, frame: &[u8]) -> Result<(), Infallible> {
            self.calls.borrow_mut().push(Call::Transmit(
                self.now.get(),
                self.frequency_hz,
                frame.to_vec(),
            ));
            Ok(())
        }

        fn receive(&mut self, buffer: &mut [u8], timeout_us: u64) -> Result<Reception, Infallible> {
            self.calls
                .borrow_mut()
                .push(Call::Receive(self.now.get(), self.frequency_hz));
            let waiting = self
                .incoming
                .first()
                .is_some_and(|(hz, _)| hz.is_none_or(|hz| hz == self.frequency_hz));
            if !waiting {
                self.now.set(self.now.get() + timeout_us);
                return Ok(Reception::Nothing);
            }
            let (_, frame) = self.incoming.remove(0);
            buffer[..frame.len()].copy_from_slice(&frame);
            Ok(Reception::Frame {
                len: frame.len(),
                snr_db: 6,
                rssi_dbm: -88,
            })
        }

        fn detect(&mut self, symbols: u8) -> Result<bool, Infallible> {
            let _ = symbols;
            self.calls.borrow_mut().push(Call::Detect(self.now.get()));
            Ok(self.detects)
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

    fn sensor() -> Session {
        Session::new(DEV_ADDR, NWK_S_KEY, APP_S_KEY)
    }

    fn relay_session() -> Session {
        Session::new(RELAY_ADDR, [0x11; 16], [0x22; 16])
    }

    /// A relay trusting the sensor, with a radio a test hands frames to.
    fn node(
        detects: bool,
        incoming: Vec<(Option<u32>, Vec<u8>)>,
    ) -> (RelayNode<'static, Air, Ticks>, Rc<RefCell<Vec<Call>>>) {
        let plan = Region::Eu868.plan();
        let device = EndDevice::personalized(
            plan,
            relay_session(),
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
        relay.trust(0, DEV_ADDR, &root_wor_s_key(&NWK_S_KEY), 0, 63, 0);

        let now = Rc::new(Cell::new(0));
        let calls = Rc::new(RefCell::new(Vec::new()));
        let air = Air {
            now: Rc::clone(&now),
            calls: Rc::clone(&calls),
            frequency_hz: 0,
            detects,
            incoming,
        };
        (RelayNode::new(relay, air, Ticks(now)), calls)
    }

    /// The frames a device under a relay puts on the air for one reading, the frequency its
    /// uplink went out on, and the answer a network sends it.
    fn reading() -> (Vec<u8>, Vec<u8>, u32, Vec<u8>) {
        let plan = Region::Eu868.plan();
        let mut device =
            EndDevice::personalized(plan, sensor(), Settings::new(2, 14)).expect("a device");
        assert!(device.use_relay(true));
        let transmission = device.send(2, b"21.5", false, 0).expect("an uplink");
        let exchange = transmission.relay.expect("relay mode is on");
        let answer = sensor()
            .encode_downlink(&Downlink::new(0, 2, b"ok"))
            .expect("a downlink");
        (
            exchange.wake_up.frame().to_vec(),
            transmission.frame.as_bytes().to_vec(),
            transmission.frequency_hz,
            answer.as_bytes().to_vec(),
        )
    }

    #[test]
    fn a_quiet_channel_costs_one_detection_and_nothing_else() {
        let (mut node, calls) = node(false, Vec::new());
        assert_eq!(node.scan(), Ok(Scanned::Quiet));
        let calls = calls.borrow();
        assert!(matches!(calls[0], Call::Detect(_)));
        assert_eq!(calls[1], Call::Sleep);
        assert_eq!(calls.len(), 2, "nothing was received: {calls:?}");
    }

    #[test]
    fn a_relay_answers_a_frame_forwards_the_uplink_behind_it_and_sends_the_answer_on() {
        let (wake_up, uplink, uplink_hz, answer) = reading();
        let forwarded_back = relay_session()
            .encode_downlink(&Downlink::new(0, LA_FPORT_RELAY, &answer))
            .expect("a downlink");
        let (mut node, calls) = node(
            true,
            vec![
                (Some(865_100_000), wake_up),
                (Some(uplink_hz), uplink.clone()),
                (None, forwarded_back.as_bytes().to_vec()),
            ],
        );

        let scanned = node.scan().expect("a scan");
        let Scanned::Forwarded {
            dev_addr,
            answered,
            delivery,
        } = scanned
        else {
            panic!("the uplink was forwarded: {scanned:?}");
        };
        assert_eq!(dev_addr, Some(DEV_ADDR));
        assert!(answered, "the answer went on to the sensor");
        assert_eq!(delivery.expect("a downlink").port(), Some(LA_FPORT_RELAY));

        let calls = calls.borrow();
        let sent: Vec<_> = calls
            .iter()
            .filter_map(|call| match call {
                Call::Transmit(at, hz, frame) => Some((*at, *hz, frame.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(sent.len(), 3, "an acknowledgment, a forward, a downlink");

        // The acknowledgment goes out on the answering frequency of the WOR channel.
        assert_eq!(sent[0].1, 865_300_000);
        assert_eq!(sent[0].2.len(), 7);

        // The forward carries the sensor's own frame to the network on port 226.
        let uplink_at = sent[1].0;
        let forward = relay_session()
            .decode(&sent[1].2, 0)
            .expect("the relay's uplink decodes");
        assert_eq!(forward.fport(), Some(LA_FPORT_RELAY));
        let inside = ForwardedUplink::parse(forward.payload()).expect("a forwarded uplink");
        assert_eq!(inside.phy_payload, uplink);
        assert_eq!(inside.metadata.rssi_dbm, -88, "what the radio heard");
        assert_eq!(inside.metadata.snr_db, 6);

        // The answer goes back on the wake-on-radio frequency, after the forward.
        assert_eq!(sent[2].1, 865_100_000);
        assert_eq!(sent[2].2, answer);
        assert!(sent[2].0 >= uplink_at, "the relay window comes last");
    }

    #[test]
    fn a_frame_from_a_device_the_relay_does_not_know_notifies_the_network() {
        let (wake_up, _, _, _) = reading();
        let (mut node, _) = node(true, vec![(Some(865_100_000), wake_up)]);
        node.relay_mut().trusted_mut().remove(0);
        assert_eq!(node.scan(), Ok(Scanned::Notified { dev_addr: DEV_ADDR }));
    }

    #[test]
    fn a_relay_that_hears_a_preamble_but_no_frame_goes_back_to_scanning() {
        let (mut node, calls) = node(true, Vec::new());
        assert_eq!(node.scan(), Ok(Scanned::Nothing));
        assert!(calls
            .borrow()
            .iter()
            .any(|call| matches!(call, Call::Receive(_, 865_100_000))));
    }

    #[test]
    fn a_stopped_relay_scans_nothing() {
        let (mut node, _) = node(false, Vec::new());
        node.relay_mut().stop();
        assert_eq!(node.scan(), Err(RelayNodeError::Relay(RelayError::Stopped)));
    }

    #[test]
    fn a_wake_on_radio_frame_with_no_uplink_behind_it_leaves_the_relay_listening_again() {
        let (wake_up, _, _, _) = reading();
        let (mut node, _) = node(true, vec![(Some(865_100_000), wake_up)]);
        assert_eq!(
            node.scan(),
            Ok(Scanned::Woken {
                dev_addr: Some(DEV_ADDR)
            })
        );
        assert_eq!(node.relay().forward_due(), None, "nothing waits to go out");
    }
}
