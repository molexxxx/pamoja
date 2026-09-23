//! A CoAP server: the end that nodes report their readings to and take their
//! commands from.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use coap_lite::{CoapOption, MessageClass, MessageType, Packet, RequestType, ResponseType};
use pamoja_core::{topic_matches, Error, Message, Receive, Result, Transport};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{observe_value, path_from_packet, random};

/// How long the server remembers the answer to a confirmable request, so that a
/// retransmission of it is answered again rather than acted on twice. It is
/// EXCHANGE_LIFETIME, RFC 7252 section 4.8.2.
const EXCHANGE_LIFETIME: Duration = Duration::from_secs(247);

/// The largest datagram the server reads; a larger one is cut short and fails to
/// decode.
const DATAGRAM: usize = 1500;

/// A CoAP server that implements the core [`Transport`] and [`Receive`] traits.
///
/// It is the far end of a [`CoapTransport`](crate::CoapTransport): a gateway that
/// nodes send their readings to and observe their commands on. A PUT or POST to a
/// path matching one of its [`subscribe`](Transport::subscribe) filters is answered
/// 2.04 Changed and delivered to [`recv`](Receive::recv), and one to any other path
/// is answered 4.04 Not Found. [`send`](Transport::send) sets the state of a
/// resource, which a GET reads and every observer of the path is notified of, as
/// RFC 7641 describes.
///
/// # Examples
///
/// ```no_run
/// use pamoja_coap::CoapServer;
/// use pamoja_core::{Receive, Transport};
///
/// # async fn run() -> pamoja_core::Result<()> {
/// let mut gateway = CoapServer::new("0.0.0.0:5683");
/// gateway.connect().await?;
/// gateway.subscribe("sensors/#").await?;
/// gateway.send_text("commands/valve", "closed").await?;
///
/// while let Some(reading) = gateway.recv().await? {
///     println!("{}: {}", reading.topic, reading.text()?);
/// }
/// # Ok(())
/// # }
/// ```
pub struct CoapServer {
    bind: String,
    shared: Arc<Shared>,
    incoming: Option<mpsc::UnboundedReceiver<Message>>,
    pump: Option<JoinHandle<()>>,
}

/// A handle that sets the state of a server's resources from anywhere, while
/// another task waits on the server's [`recv`](Receive::recv).
///
/// A gateway that takes readings in one loop and sends commands from another holds
/// one of these for the commands, since a receive holds the server itself.
#[derive(Clone)]
pub struct CoapPublisher {
    shared: Arc<Shared>,
}

/// What a server and its publishers share.
struct Shared {
    /// The bound socket, while the server is connected.
    socket: Mutex<Option<Arc<UdpSocket>>>,
    /// The resources, the observers, and the rest of the server's state.
    state: Mutex<State>,
}

/// What the server holds between datagrams.
struct State {
    /// The filters a PUT or POST path has to match to be taken.
    filters: Vec<String>,
    /// The state of each resource, by path.
    resources: HashMap<String, Vec<u8>>,
    /// The observers of each resource, by path.
    observers: HashMap<String, Vec<Observer>>,
    /// The answer sent to each recent confirmable request, by sender and message id.
    answered: HashMap<(SocketAddr, u16), (Instant, Vec<u8>)>,
    /// The next message id for a message the server starts.
    next_id: u16,
    /// The number of the next notification, of which RFC 7641 section 4.4 sends the
    /// low 24 bits.
    sequence: u32,
}

/// One client observing one resource.
struct Observer {
    /// Where its notifications go.
    peer: SocketAddr,
    /// The token of its registration, which every notification carries back.
    token: Vec<u8>,
    /// The message id of the last notification sent to it, which a Reset names.
    last_id: u16,
}

impl CoapServer {
    /// Creates a server that will listen on a local address, without binding it.
    ///
    /// # Arguments
    ///
    /// * `bind` - the local `host:port`, such as `"0.0.0.0:5683"` for the plaintext
    ///   CoAP port on every interface, or port `0` for a free one.
    ///
    /// # Returns
    ///
    /// A server ready for [`connect`](Transport::connect).
    pub fn new(bind: impl Into<String>) -> Self {
        Self {
            bind: bind.into(),
            shared: Arc::new(Shared {
                socket: Mutex::new(None),
                state: Mutex::new(State {
                    filters: Vec::new(),
                    resources: HashMap::new(),
                    observers: HashMap::new(),
                    answered: HashMap::new(),
                    next_id: random() as u16,
                    sequence: 0,
                }),
            }),
            incoming: None,
            pump: None,
        }
    }

    /// Takes a handle that publishes resource states while a receive waits.
    ///
    /// # Returns
    ///
    /// A publisher sharing this server's resources and socket.
    pub fn publisher(&self) -> CoapPublisher {
        CoapPublisher {
            shared: Arc::clone(&self.shared),
        }
    }

    /// Reports whether the server holds a bound socket.
    ///
    /// # Returns
    ///
    /// `true` from a successful [`connect`](Transport::connect) until
    /// [`disconnect`](CoapServer::disconnect).
    pub fn is_connected(&self) -> bool {
        self.shared.socket().is_some()
    }

    /// The address the server listens on, which names the port the system chose
    /// when it bound port `0`.
    ///
    /// # Returns
    ///
    /// The local address, or `None` before [`connect`](Transport::connect).
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.shared.socket()?.local_addr().ok()
    }

    /// Counts the clients observing a resource.
    ///
    /// # Arguments
    ///
    /// * `path` - the resource's path.
    ///
    /// # Returns
    ///
    /// How many observers the resource has.
    pub fn observers(&self, path: &str) -> usize {
        self.shared.observers(path)
    }

    /// Closes the socket and stops the background task.
    ///
    /// The resources and filters are kept for the next
    /// [`connect`](Transport::connect), and the observers are dropped, since their
    /// registrations were made to the socket that closed.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the socket is released.
    ///
    /// # Errors
    ///
    /// This call is best-effort and always returns `Ok(())`.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
        *self.shared.socket.lock().expect("socket lock") = None;
        self.incoming = None;
        self.shared.state().observers.clear();
        Ok(())
    }
}

impl CoapPublisher {
    /// Sets the state of a resource and notifies every observer of it.
    ///
    /// # Arguments
    ///
    /// * `path` - the resource's path.
    /// * `payload` - its new state, which a GET now reads.
    ///
    /// # Returns
    ///
    /// `Ok(())` once each observer's notification has left. An observer that has
    /// gone does not stop the others' notifications.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if the server is not connected, or
    /// [`Error::Transport`] if the socket failed for a reason other than an observer
    /// that has gone.
    pub async fn publish(&self, path: &str, payload: &[u8]) -> Result<()> {
        let socket = self.shared.socket().ok_or(Error::Closed)?;
        let out = {
            let mut state = self.shared.state();
            let path = path.trim_matches('/').to_owned();
            state.resources.insert(path.clone(), payload.to_vec());
            state.notify(&path, payload)
        };
        let mut failure = None;
        for (to, bytes) in out {
            if let Err(error) = socket.send_to(&bytes, to).await {
                if !crate::refused(&error) {
                    failure.get_or_insert(Error::Transport(error.to_string()));
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }

    /// Counts the clients observing a resource.
    ///
    /// # Arguments
    ///
    /// * `path` - the resource's path.
    ///
    /// # Returns
    ///
    /// How many observers the resource has.
    pub fn observers(&self, path: &str) -> usize {
        self.shared.observers(path)
    }

    /// Reports whether the server holds a bound socket.
    ///
    /// # Returns
    ///
    /// `true` while the server is connected.
    pub fn is_connected(&self) -> bool {
        self.shared.socket().is_some()
    }

    /// The address the server listens on.
    ///
    /// # Returns
    ///
    /// The local address, or `None` while the server is not connected.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.shared.socket()?.local_addr().ok()
    }
}

impl Shared {
    /// The bound socket, while the server is connected.
    fn socket(&self) -> Option<Arc<UdpSocket>> {
        self.socket.lock().expect("socket lock").clone()
    }

    /// The server's state, locked.
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().expect("server lock")
    }

    /// Counts the observers of a resource.
    fn observers(&self, path: &str) -> usize {
        self.state()
            .observers
            .get(path.trim_matches('/'))
            .map_or(0, Vec::len)
    }
}

impl Transport for CoapServer {
    /// Binds the server's socket and starts answering requests.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the socket is bound.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if the address cannot be bound, such as a port
    /// another program holds.
    async fn connect(&mut self) -> Result<()> {
        self.disconnect().await?;
        let socket = Arc::new(
            UdpSocket::bind(&self.bind)
                .await
                .map_err(|err| Error::Transport(format!("could not bind {}: {err}", self.bind)))?,
        );
        let (tx, rx) = mpsc::unbounded_channel();
        let shared = Arc::clone(&self.shared);
        let pump_socket = Arc::clone(&socket);
        let pump = tokio::spawn(async move {
            let mut buf = vec![0u8; DATAGRAM];
            loop {
                let (len, peer) = match pump_socket.recv_from(&mut buf).await {
                    Ok(received) => received,
                    Err(error) if crate::refused(&error) => continue,
                    Err(_) => break,
                };
                let Ok(packet) = Packet::from_bytes(&buf[..len]) else {
                    continue;
                };
                let out = handle(&packet, peer, &shared.state, &tx);
                for (to, bytes) in out {
                    let _ = pump_socket.send_to(&bytes, to).await;
                }
            }
        });
        *self.shared.socket.lock().expect("socket lock") = Some(socket);
        self.incoming = Some(rx);
        self.pump = Some(pump);
        Ok(())
    }

    /// Sets the state of a resource and notifies every observer of it, as
    /// [`CoapPublisher::publish`] does.
    ///
    /// # Arguments
    ///
    /// * `topic` - the resource's path.
    /// * `payload` - its new state, which a GET now reads.
    ///
    /// # Returns
    ///
    /// `Ok(())` once each observer's notification has left.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if the server is not connected.
    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        self.publisher().publish(topic, payload).await
    }

    /// Takes the readings sent to the paths a filter matches.
    ///
    /// # Arguments
    ///
    /// * `topic` - the filter, with `+` for one level and `#` for the rest, as
    ///   [`topic_matches`] reads them.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the filter is in place. A server takes readings for its filters
    /// whether or not it is connected.
    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        self.shared
            .state()
            .filters
            .push(topic.trim_matches('/').to_owned());
        Ok(())
    }
}

impl Receive for CoapServer {
    /// Awaits the next reading sent to a path the server takes.
    ///
    /// # Returns
    ///
    /// `Some(message)` for the next reading, carrying the path it was sent to.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Closed`] if the server is not connected.
    async fn recv(&mut self) -> Result<Option<Message>> {
        let incoming = self.incoming.as_mut().ok_or(Error::Closed)?;
        Ok(incoming.recv().await)
    }
}

impl State {
    /// Takes the next message id for a message the server starts.
    fn message_id(&mut self) -> u16 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Takes the value of the next notification's Observe option.
    fn next_sequence(&mut self) -> u32 {
        self.sequence = (self.sequence + 1) & 0x00FF_FFFF;
        self.sequence
    }

    /// Builds a notification of a resource's new state for each of its observers.
    fn notify(&mut self, path: &str, payload: &[u8]) -> Vec<(SocketAddr, Vec<u8>)> {
        let sequence = self.next_sequence();
        let count = self.observers.get(path).map_or(0, Vec::len);
        let ids: Vec<u16> = (0..count).map(|_| self.message_id()).collect();
        let Some(observers) = self.observers.get_mut(path) else {
            return Vec::new();
        };
        observers
            .iter_mut()
            .zip(ids)
            .filter_map(|(observer, id)| {
                observer.last_id = id;
                let mut packet = response(ResponseType::Content, payload);
                packet.header.set_type(MessageType::NonConfirmable);
                packet.header.message_id = id;
                packet.set_token(observer.token.clone());
                packet.add_option(CoapOption::Observe, observe_bytes(sequence));
                Some((observer.peer, packet.to_bytes().ok()?))
            })
            .collect()
    }
}

/// Answers one datagram, returning what to send and where.
fn handle(
    packet: &Packet,
    peer: SocketAddr,
    state: &Mutex<State>,
    tx: &mpsc::UnboundedSender<Message>,
) -> Vec<(SocketAddr, Vec<u8>)> {
    let mut state = state.lock().expect("server lock");
    let id = packet.header.message_id;
    match (packet.header.get_type(), packet.header.code) {
        (MessageType::Reset, _) => {
            for observers in state.observers.values_mut() {
                observers.retain(|observer| !(observer.peer == peer && observer.last_id == id));
            }
            Vec::new()
        }
        (MessageType::Confirmable, MessageClass::Empty) => {
            vec![(peer, empty(MessageType::Reset, id))]
        }
        (
            kind @ (MessageType::Confirmable | MessageType::NonConfirmable),
            MessageClass::Request(method),
        ) => {
            let confirmable = kind == MessageType::Confirmable;
            if confirmable {
                let now = Instant::now();
                state
                    .answered
                    .retain(|_, (at, _)| now.duration_since(*at) < EXCHANGE_LIFETIME);
                if let Some((_, bytes)) = state.answered.get(&(peer, id)) {
                    return vec![(peer, bytes.clone())];
                }
            }
            let (mut answer, mut out) = request(&mut state, packet, method, peer, tx);
            answer.set_token(packet.get_token().to_vec());
            if confirmable {
                answer.header.set_type(MessageType::Acknowledgement);
                answer.header.message_id = id;
            } else {
                answer.header.set_type(MessageType::NonConfirmable);
                answer.header.message_id = state.message_id();
            }
            if let Ok(bytes) = answer.to_bytes() {
                if confirmable {
                    state
                        .answered
                        .insert((peer, id), (Instant::now(), bytes.clone()));
                }
                out.insert(0, (peer, bytes));
            }
            out
        }
        (MessageType::Confirmable, _) => vec![(peer, empty(MessageType::Reset, id))],
        _ => Vec::new(),
    }
}

/// Carries out a request, returning the response and any notifications it causes.
fn request(
    state: &mut State,
    packet: &Packet,
    method: RequestType,
    peer: SocketAddr,
    tx: &mpsc::UnboundedSender<Message>,
) -> (Packet, Vec<(SocketAddr, Vec<u8>)>) {
    let path = path_from_packet(packet);
    match method {
        RequestType::Get => {
            let Some(current) = state.resources.get(&path).cloned() else {
                return (response(ResponseType::NotFound, &[]), Vec::new());
            };
            let token = packet.get_token().to_vec();
            let mut answer = response(ResponseType::Content, &current);
            match observe_value(packet) {
                Some(0) => {
                    let observers = state.observers.entry(path).or_default();
                    observers
                        .retain(|observer| !(observer.peer == peer && observer.token == token));
                    observers.push(Observer {
                        peer,
                        token,
                        last_id: 0,
                    });
                    answer.add_option(CoapOption::Observe, observe_bytes(state.sequence));
                }
                Some(1) => {
                    if let Some(observers) = state.observers.get_mut(&path) {
                        observers
                            .retain(|observer| !(observer.peer == peer && observer.token == token));
                    }
                }
                _ => {}
            }
            (answer, Vec::new())
        }
        RequestType::Put | RequestType::Post => {
            if !state
                .filters
                .iter()
                .any(|filter| topic_matches(filter, &path))
            {
                return (response(ResponseType::NotFound, &[]), Vec::new());
            }
            let _ = tx.send(Message::new(path.clone(), packet.payload.clone()));
            state.resources.insert(path.clone(), packet.payload.clone());
            let notifications = state.notify(&path, &packet.payload);
            (response(ResponseType::Changed, &[]), notifications)
        }
        _ => (response(ResponseType::MethodNotAllowed, &[]), Vec::new()),
    }
}

/// Builds a response with a code and a payload, its type, id, and token still to set.
fn response(code: ResponseType, payload: &[u8]) -> Packet {
    let mut packet = Packet::new();
    packet.header.set_version(1);
    packet.header.code = MessageClass::Response(code);
    packet.payload = payload.to_vec();
    packet
}

/// Encodes an empty message of a type, answering a message id.
fn empty(kind: MessageType, message_id: u16) -> Vec<u8> {
    let mut packet = Packet::new();
    packet.header.set_version(1);
    packet.header.set_type(kind);
    packet.header.code = MessageClass::Empty;
    packet.header.message_id = message_id;
    packet.to_bytes().unwrap_or_default()
}

/// Encodes an Observe value as the shortest big-endian integer that holds it, with
/// zero as no bytes at all (RFC 7252 section 3.2).
fn observe_bytes(value: u32) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    bytes[first..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_observe_value_takes_the_fewest_bytes_that_hold_it() {
        assert_eq!(observe_bytes(0), Vec::<u8>::new());
        assert_eq!(observe_bytes(1), vec![1]);
        assert_eq!(observe_bytes(0x0100), vec![1, 0]);
        assert_eq!(observe_bytes(0x00FF_FFFF), vec![0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn the_notification_number_wraps_at_24_bits() {
        let mut state = State {
            filters: Vec::new(),
            resources: HashMap::new(),
            observers: HashMap::new(),
            answered: HashMap::new(),
            next_id: 0,
            sequence: 0x00FF_FFFF,
        };
        assert_eq!(state.next_sequence(), 0);
        assert_eq!(state.next_sequence(), 1);
    }
}
