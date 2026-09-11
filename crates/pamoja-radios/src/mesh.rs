//! A pamoja transport over a LoRa radio, carrying topics in pamoja-mesh frames.
//!
//! [`MeshRadio`] turns a radio into a [`Transport`] and a [`Receive`]. A message goes out
//! as a broadcast [`Frame`] whose payload is the topic's length in one byte, the topic, and
//! the payload. Every node that hears it drops copies it has already seen, delivers what
//! its subscriptions match, and relays the frame onward while hops remain, so a message
//! crosses a mesh of radios that each hear only their neighbors. A [`DutyCycle`] holds the
//! radio silent for the off time the region requires after each transmission, its own
//! messages and its relays alike.
//!
//! The radio is driven from the task that awaits the transport. It is read every [`POLL`],
//! with the tokio timer sleeping in between, so neither a frame's airtime nor a quiet
//! channel blocks the runtime. Topic filters follow MQTT, through [`topic_matches`].
//!
//! # Examples
//!
//! Two nodes whose air is a queue each, with the frame carried across by hand:
//!
//! ```
//! use std::collections::VecDeque;
//! use std::convert::Infallible;
//!
//! use pamoja_core::{Receive, Transport};
//! use pamoja_lora::LinkSettings;
//! use pamoja_radios::mesh::{LoraRadio, MeshRadio};
//!
//! #[derive(Default)]
//! struct Air(VecDeque<Vec<u8>>);
//!
//! impl LoraRadio for Air {
//!     type Error = Infallible;
//!
//!     fn link(&self) -> Option<LinkSettings> {
//!         Some(LinkSettings::new(7, 125_000))
//!     }
//!
//!     fn start_transmit(&mut self, frame: &[u8]) -> Result<u64, Infallible> {
//!         self.0.push_back(frame.to_vec());
//!         Ok(LinkSettings::new(7, 125_000).airtime_us(frame.len()))
//!     }
//!
//!     fn finish_transmit(&mut self) -> Result<bool, Infallible> {
//!         Ok(true)
//!     }
//!
//!     fn listen(&mut self) -> Result<(), Infallible> {
//!         Ok(())
//!     }
//!
//!     fn take_frame(&mut self, buffer: &mut [u8]) -> Result<Option<usize>, Infallible> {
//!         Ok(self.0.pop_front().map(|frame| {
//!             buffer[..frame.len()].copy_from_slice(&frame);
//!             frame.len()
//!         }))
//!     }
//! }
//!
//! # let runtime = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
//! # runtime.block_on(async {
//! // A soil sensor on a 1% duty cycle, and a gateway that listens without relaying.
//! let mut sensor = MeshRadio::new(Air::default(), 0x0A, 10);
//! let mut gateway = MeshRadio::new(Air::default(), 0x0B, 10).without_relaying();
//! sensor.connect().await?;
//! gateway.connect().await?;
//! gateway.subscribe("garden/+/moisture").await?;
//!
//! sensor.send_text("garden/bed-1/moisture", "28.5").await?;
//! let frame = sensor.radio_mut().0.pop_front().expect("the sensor transmitted");
//! gateway.radio_mut().0.push_back(frame);
//!
//! let reading = gateway.recv().await?.expect("the gateway heard it");
//! assert_eq!(reading.topic, "garden/bed-1/moisture");
//! assert_eq!(reading.number()?, 28.5);
//! # Ok::<(), pamoja_core::Error>(())
//! # }).unwrap();
//! ```

use std::time::Duration;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;
use pamoja_core::{topic_matches, Error, Message, Receive, Result, Transport};
use pamoja_lora::LinkSettings;
use pamoja_mesh::{DynamicSeenCache, Frame};
use tokio::time::{sleep, Instant};

use crate::duty::DutyCycle;
use crate::sx126x::{RadioError, Reception, Sx126x};

/// The most topic and payload bytes one message carries together: a frame's payload less
/// the byte that holds the topic's length.
pub const MAX_MESSAGE: usize = Frame::MAX_PAYLOAD - 1;

/// How often the transport reads the radio for a received frame or a finished transmission.
pub const POLL: Duration = Duration::from_millis(1);

/// How long past a frame's airtime the transport waits for the radio to report it sent.
pub const TRANSMIT_GRACE: Duration = Duration::from_secs(2);

/// How many recent frames a node remembers, so copies arriving by other paths are dropped.
pub const SEEN_CAPACITY: usize = 64;

/// A radio a [`MeshRadio`] carries frames over.
///
/// Each method returns at once, so the transport can sleep on the runtime's timer between
/// calls rather than block it. [`Sx126x`] implements it; a radio of another family, or a
/// simulated one, implements the same five methods.
pub trait LoraRadio {
    /// What the radio reports when it or the bus under it fails.
    type Error: core::fmt::Debug;

    /// Returns the link settings frames go out with, which the duty cycle's arithmetic
    /// needs.
    ///
    /// # Returns
    ///
    /// The settings, or `None` while the radio is unconfigured.
    fn link(&self) -> Option<LinkSettings>;

    /// Starts sending one frame, without waiting for it to leave.
    ///
    /// # Arguments
    ///
    /// * `frame` - the bytes to put on the air.
    ///
    /// # Returns
    ///
    /// The frame's airtime in microseconds.
    ///
    /// # Errors
    ///
    /// Whatever the radio reports.
    fn start_transmit(&mut self, frame: &[u8]) -> std::result::Result<u64, Self::Error>;

    /// Reports whether the frame [`start_transmit`](LoraRadio::start_transmit) began has
    /// left.
    ///
    /// # Returns
    ///
    /// `true` once it has been sent.
    ///
    /// # Errors
    ///
    /// Whatever the radio reports, including a transmission that timed out.
    fn finish_transmit(&mut self) -> std::result::Result<bool, Self::Error>;

    /// Starts listening, and keeps listening frame after frame.
    ///
    /// # Errors
    ///
    /// Whatever the radio reports.
    fn listen(&mut self) -> std::result::Result<(), Self::Error>;

    /// Takes a frame the radio has received since the last call, dropping one whose CRC
    /// failed.
    ///
    /// # Arguments
    ///
    /// * `buffer` - where the frame goes, 255 bytes long.
    ///
    /// # Returns
    ///
    /// The frame's length, or `None` when no good frame has arrived.
    ///
    /// # Errors
    ///
    /// Whatever the radio reports.
    fn take_frame(&mut self, buffer: &mut [u8]) -> std::result::Result<Option<usize>, Self::Error>;
}

impl<SPI, BUSY, RESET, D> LoraRadio for Sx126x<SPI, BUSY, RESET, D>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    RESET: OutputPin,
    D: DelayNs,
{
    type Error = RadioError<SPI::Error>;

    fn link(&self) -> Option<LinkSettings> {
        self.config().map(|config| config.link)
    }

    fn start_transmit(&mut self, frame: &[u8]) -> std::result::Result<u64, Self::Error> {
        Sx126x::start_transmit(self, frame)
    }

    fn finish_transmit(&mut self) -> std::result::Result<bool, Self::Error> {
        Sx126x::finish_transmit(self)
    }

    fn listen(&mut self) -> std::result::Result<(), Self::Error> {
        Sx126x::listen(self)
    }

    fn take_frame(&mut self, buffer: &mut [u8]) -> std::result::Result<Option<usize>, Self::Error> {
        match Sx126x::take_frame(self, buffer)? {
            Some(Reception::Frame { len, .. }) => Ok(Some(len)),
            _ => Ok(None),
        }
    }
}

/// A node on a LoRa mesh: a radio, the node's address, and the rules it keeps.
///
/// Build it around a configured radio, [`connect`](Transport::connect) to start listening,
/// then send and receive as over any other pamoja transport.
pub struct MeshRadio<R> {
    radio: R,
    node: u32,
    next_id: u16,
    hop_limit: u8,
    relay: bool,
    duty: DutyCycle,
    epoch: Instant,
    filters: Vec<String>,
    seen: DynamicSeenCache,
    pending: Option<Message>,
    connected: bool,
    buffer: [u8; 255],
}

impl<R: LoraRadio> MeshRadio<R> {
    /// Builds a node around a radio that is already configured.
    ///
    /// Frames start with [`Frame::DEFAULT_HOP_LIMIT`] hops, and the node relays what it
    /// hears.
    ///
    /// # Arguments
    ///
    /// * `radio` - the radio, tuned to the channel the mesh uses.
    /// * `node` - this node's address, which every frame it sends carries as its source.
    /// * `duty_cycle_permille` - the region's duty-cycle limit in parts per thousand, such
    ///   as `10` for 1%, or `1000` where the region sets none.
    ///
    /// # Returns
    ///
    /// The node, not yet connected.
    pub fn new(radio: R, node: u32, duty_cycle_permille: u32) -> MeshRadio<R> {
        MeshRadio {
            radio,
            node,
            next_id: 0,
            hop_limit: Frame::DEFAULT_HOP_LIMIT,
            relay: true,
            duty: DutyCycle::new(duty_cycle_permille),
            epoch: Instant::now(),
            filters: Vec::new(),
            seen: DynamicSeenCache::new(SEEN_CAPACITY),
            pending: None,
            connected: false,
            buffer: [0; 255],
        }
    }

    /// Returns the node with another hop limit for the frames it sends.
    ///
    /// # Arguments
    ///
    /// * `hop_limit` - how many relays a frame may take; `0` keeps it to the nodes in range.
    ///
    /// # Returns
    ///
    /// The node.
    pub fn with_hop_limit(mut self, hop_limit: u8) -> MeshRadio<R> {
        self.hop_limit = hop_limit;
        self
    }

    /// Returns the node with relaying off, so it only sends and receives its own traffic.
    ///
    /// # Returns
    ///
    /// The node.
    pub fn without_relaying(mut self) -> MeshRadio<R> {
        self.relay = false;
        self
    }

    /// Returns this node's address.
    ///
    /// # Returns
    ///
    /// The address frames carry as their source.
    pub fn node(&self) -> u32 {
        self.node
    }

    /// Reports whether the node is connected and listening.
    ///
    /// # Returns
    ///
    /// `true` after [`connect`](Transport::connect).
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Returns the duty-cycle guard, to see how long the radio must still stay silent.
    ///
    /// # Returns
    ///
    /// The guard, whose clock is microseconds since the node was built.
    pub fn duty_cycle(&self) -> &DutyCycle {
        &self.duty
    }

    /// Returns the radio.
    ///
    /// # Returns
    ///
    /// A reference to the radio.
    pub fn radio(&self) -> &R {
        &self.radio
    }

    /// Returns the radio, to reach what the transport does not cover.
    ///
    /// # Returns
    ///
    /// A mutable reference to the radio.
    pub fn radio_mut(&mut self) -> &mut R {
        &mut self.radio
    }

    /// Gives back the radio.
    ///
    /// # Returns
    ///
    /// The radio.
    pub fn release(self) -> R {
        self.radio
    }

    fn now_us(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    async fn transmit(&mut self, frame: &Frame) -> Result<()> {
        let now_us = self.now_us();
        let wait_us = self.duty.wait_us(now_us);
        if wait_us > 0 {
            return Err(Error::Transport(format!(
                "the duty cycle keeps the radio silent for another {} ms",
                wait_us.div_ceil(1000)
            )));
        }
        let link = self
            .radio
            .link()
            .ok_or_else(|| Error::Transport("the radio is not configured".to_owned()))?;
        let bytes = frame.as_bytes();
        let airtime_us = self.radio.start_transmit(bytes).map_err(radio_error)?;
        self.duty.transmitted(now_us, &link, bytes.len());

        let airtime = Duration::from_micros(airtime_us);
        sleep(airtime).await;
        let deadline = Instant::now() + TRANSMIT_GRACE;
        while !self.radio.finish_transmit().map_err(radio_error)? {
            if Instant::now() >= deadline {
                return Err(Error::Transport(
                    "the radio never reported the frame sent".to_owned(),
                ));
            }
            sleep(POLL).await;
        }
        self.radio.listen().map_err(radio_error)
    }

    async fn accept(&mut self, len: usize) -> Result<()> {
        let Ok(frame) = Frame::parse(&self.buffer[..len]) else {
            return Ok(());
        };
        if frame.src() == self.node || !self.seen.record(frame.dedup_key()) {
            return Ok(());
        }
        if let Some((topic, payload)) = decode(frame.payload()) {
            if self
                .filters
                .iter()
                .any(|filter| topic_matches(filter, topic))
            {
                self.pending = Some(Message::new(topic, payload));
            }
        }
        if self.relay {
            if let Some(onward) = frame.relayed() {
                if self.duty.ready(self.now_us()) {
                    self.transmit(&onward).await?;
                }
            }
        }
        Ok(())
    }
}

impl<R: LoraRadio + Send> Transport for MeshRadio<R> {
    async fn connect(&mut self) -> Result<()> {
        self.radio.listen().map_err(radio_error)?;
        self.connected = true;
        Ok(())
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        let body = encode(topic, payload)?;
        let frame = Frame::broadcast(self.node, self.next_id, &body)
            .map_err(|error| Error::Transport(format!("mesh frame: {error:?}")))?
            .with_hop_limit(self.hop_limit);
        self.next_id = self.next_id.wrapping_add(1);
        self.seen.record(frame.dedup_key());
        self.transmit(&frame).await
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        if !self.connected {
            return Err(Error::Closed);
        }
        self.filters.push(topic.to_owned());
        Ok(())
    }
}

impl<R: LoraRadio + Send> Receive for MeshRadio<R> {
    /// Awaits the next message whose topic a subscription matches.
    ///
    /// A frame relayed on the way is sent before the message is handed up, and a message
    /// already taken off the air waits in the node if the call is dropped meanwhile, so
    /// nothing received is lost to a cancellation.
    ///
    /// # Returns
    ///
    /// The next message; the air never ends, so never `None`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] before [`connect`](Transport::connect), and
    /// [`Error::Transport`] if the radio fails.
    async fn recv(&mut self) -> Result<Option<Message>> {
        if !self.connected {
            return Err(Error::Closed);
        }
        loop {
            if let Some(message) = self.pending.take() {
                return Ok(Some(message));
            }
            let received = self
                .radio
                .take_frame(&mut self.buffer)
                .map_err(radio_error)?;
            match received {
                Some(len) => self.accept(len).await?,
                None => sleep(POLL).await,
            }
        }
    }
}

fn radio_error<E: core::fmt::Debug>(error: E) -> Error {
    Error::Transport(format!("radio: {error:?}"))
}

fn encode(topic: &str, payload: &[u8]) -> Result<Vec<u8>> {
    let too_long = || {
        Error::Transport(format!(
            "a {} byte topic and a {} byte payload exceed the {MAX_MESSAGE} bytes a mesh frame carries",
            topic.len(),
            payload.len()
        ))
    };
    if topic.len() + payload.len() > MAX_MESSAGE {
        return Err(too_long());
    }
    let topic_len = u8::try_from(topic.len()).map_err(|_| too_long())?;
    let mut body = Vec::with_capacity(1 + topic.len() + payload.len());
    body.push(topic_len);
    body.extend_from_slice(topic.as_bytes());
    body.extend_from_slice(payload);
    Ok(body)
}

fn decode(body: &[u8]) -> Option<(&str, &[u8])> {
    let (&topic_len, rest) = body.split_first()?;
    let topic_len = usize::from(topic_len);
    if rest.len() < topic_len {
        return None;
    }
    let (topic, payload) = rest.split_at(topic_len);
    Some((core::str::from_utf8(topic).ok()?, payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::convert::Infallible;

    fn link() -> LinkSettings {
        LinkSettings::new(7, 125_000)
    }

    #[derive(Default)]
    struct Air {
        sent: Vec<Vec<u8>>,
        heard: VecDeque<Vec<u8>>,
        listens: usize,
    }

    impl LoraRadio for Air {
        type Error = Infallible;

        fn link(&self) -> Option<LinkSettings> {
            Some(link())
        }

        fn start_transmit(&mut self, frame: &[u8]) -> std::result::Result<u64, Infallible> {
            self.sent.push(frame.to_vec());
            Ok(link().airtime_us(frame.len()))
        }

        fn finish_transmit(&mut self) -> std::result::Result<bool, Infallible> {
            Ok(true)
        }

        fn listen(&mut self) -> std::result::Result<(), Infallible> {
            self.listens += 1;
            Ok(())
        }

        fn take_frame(
            &mut self,
            buffer: &mut [u8],
        ) -> std::result::Result<Option<usize>, Infallible> {
            Ok(self.heard.pop_front().map(|frame| {
                buffer[..frame.len()].copy_from_slice(&frame);
                frame.len()
            }))
        }
    }

    fn on_air(src: u32, id: u16, hops: u8, topic: &str, payload: &[u8]) -> Vec<u8> {
        let body = encode(topic, payload).unwrap();
        Frame::broadcast(src, id, &body)
            .unwrap()
            .with_hop_limit(hops)
            .as_bytes()
            .to_vec()
    }

    #[tokio::test(start_paused = true)]
    async fn nothing_goes_out_or_comes_in_before_connect() {
        let mut node = MeshRadio::new(Air::default(), 0x0A, 1000);
        assert!(matches!(node.send("a", b"1").await, Err(Error::Closed)));
        assert!(matches!(node.subscribe("a").await, Err(Error::Closed)));
        assert!(matches!(node.recv().await, Err(Error::Closed)));
        assert!(!node.is_connected());
    }

    #[tokio::test(start_paused = true)]
    async fn a_message_leaves_as_a_broadcast_frame_carrying_its_topic() {
        let mut node = MeshRadio::new(Air::default(), 0x0A, 1000);
        node.connect().await.unwrap();
        node.send("garden/moisture", b"28.5").await.unwrap();

        let air = node.release();
        assert_eq!(air.listens, 2);
        let sent = Frame::parse(&air.sent[0]).unwrap();
        assert_eq!(sent.src(), 0x0A);
        assert!(sent.is_broadcast());
        assert_eq!(sent.id(), 0);
        assert_eq!(sent.hop_limit(), Frame::DEFAULT_HOP_LIMIT);
        let mut body = vec![15];
        body.extend_from_slice(b"garden/moisture28.5");
        assert_eq!(sent.payload(), body.as_slice());
    }

    #[tokio::test(start_paused = true)]
    async fn the_duty_cycle_holds_back_the_next_message_for_its_off_time() {
        let mut node = MeshRadio::new(Air::default(), 0x0A, 10);
        node.connect().await.unwrap();
        node.send("t", b"1").await.unwrap();

        let refused = node.send("t", b"2").await;
        assert!(
            matches!(&refused, Err(Error::Transport(reason)) if reason.contains("duty cycle")),
            "{refused:?}"
        );
        let len = node.radio().sent[0].len();
        sleep(Duration::from_micros(link().min_off_time_us(len, 10))).await;
        node.send("t", b"2").await.unwrap();
        assert_eq!(node.radio().sent.len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn recv_delivers_each_matching_topic_once_and_ignores_its_own_frames() {
        let mut air = Air::default();
        air.heard.extend([
            on_air(0x0B, 7, 0, "kitchen/light", b"on"),
            on_air(0x0B, 8, 0, "garden/moisture", b"28.5"),
            on_air(0x0B, 8, 0, "garden/moisture", b"28.5"),
            on_air(0x0A, 1, 0, "garden/valve", b"open"),
            on_air(0x0C, 1, 0, "garden/valve", b"shut"),
        ]);
        let mut node = MeshRadio::new(air, 0x0A, 1000);
        node.connect().await.unwrap();
        node.subscribe("garden/+").await.unwrap();

        let first = node.recv().await.unwrap().unwrap();
        assert_eq!(first, Message::new("garden/moisture", b"28.5"));
        let second = node.recv().await.unwrap().unwrap();
        assert_eq!(second, Message::new("garden/valve", b"shut"));
        assert!(node.radio().heard.is_empty());
        assert!(node.radio().sent.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn a_frame_with_hops_left_is_relayed_with_one_hop_spent() {
        let heard = on_air(0x0B, 3, 2, "alerts/flood", b"high");
        let mut air = Air::default();
        air.heard.push_back(heard.clone());
        let mut node = MeshRadio::new(air, 0x0A, 1000);
        node.connect().await.unwrap();
        node.subscribe("alerts/#").await.unwrap();

        let alert = node.recv().await.unwrap().unwrap();
        assert_eq!(alert, Message::new("alerts/flood", b"high"));
        let relayed = Frame::parse(&node.radio().sent[0]).unwrap();
        let original = Frame::parse(&heard).unwrap();
        assert_eq!(relayed.src(), 0x0B);
        assert_eq!(relayed.id(), 3);
        assert_eq!(relayed.hop_limit(), 1);
        assert_eq!(relayed.payload(), original.payload());
    }

    #[tokio::test(start_paused = true)]
    async fn a_node_that_does_not_relay_only_listens() {
        let mut air = Air::default();
        air.heard
            .push_back(on_air(0x0B, 3, 2, "alerts/flood", b"high"));
        let mut node = MeshRadio::new(air, 0x0A, 1000).without_relaying();
        node.connect().await.unwrap();
        node.subscribe("#").await.unwrap();

        assert!(node.recv().await.unwrap().is_some());
        assert!(node.radio().sent.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn a_message_too_long_for_one_frame_is_refused() {
        let mut node = MeshRadio::new(Air::default(), 0x0A, 1000);
        node.connect().await.unwrap();
        let most = [0u8; MAX_MESSAGE];
        assert!(matches!(
            node.send("t", &most).await,
            Err(Error::Transport(_))
        ));
        node.send("", &most).await.unwrap();
        assert_eq!(node.radio().sent[0].len(), Frame::MAX_LEN);
    }

    #[test]
    fn a_body_that_claims_a_longer_topic_than_it_holds_is_not_a_message() {
        assert_eq!(decode(&[5, b'a', b'b']), None);
        assert_eq!(decode(&[]), None);
        assert_eq!(decode(&[1, 0xFF, b'x']), None);
        assert_eq!(decode(&[1, b't', b'x']), Some(("t", &b"x"[..])));
    }
}
