//! The Semtech packet forwarder protocol, both sides.
//!
//! A gateway and a network server exchange six kinds of UDP datagram, described by
//! PROTOCOL.TXT in Semtech's `packet_forwarder`. Each begins with the same three bytes, a
//! protocol version of 2 and a random token, then an identifier that says which kind it is:
//!
//! - The gateway sends PUSH_DATA with the packets it heard, and the server answers PUSH_ACK
//!   with the same token.
//! - The gateway sends PULL_DATA to hold a route open through whatever network address
//!   translation sits in front of it, and the server answers PULL_ACK.
//! - The server sends PULL_RESP with a packet to transmit, and the gateway answers TX_ACK
//!   saying whether it accepted it.
//!
//! The protocol has no authentication and no retries, which is why it belongs on a private
//! network or behind a tunnel. [`Packet`] builds and parses every kind, and the objects they
//! carry are in [`Rxpk`], [`Stat`], [`Txpk`], and [`TxStatus`].
//!
//! # Examples
//!
//! A server reads a forwarded packet and answers it.
//!
//! ```
//! use pamoja_gateway::udp::{Eui, Packet, PacketKind, Rxpk, Uplink};
//! use pamoja_lora::LinkSettings;
//!
//! let gateway = Eui::new([0xB8, 0x27, 0xEB, 0xFF, 0xFE, 0x01, 0x02, 0x03]);
//! let datagram = Packet::PushData {
//!     token: 0x0102,
//!     gateway,
//!     uplink: Uplink::from(Rxpk::new(
//!         868_100_000,
//!         LinkSettings::new(7, 125_000),
//!         b"hello".to_vec(),
//!     )),
//! }
//! .to_bytes();
//!
//! let packet = Packet::parse(&datagram)?;
//! assert_eq!(packet.kind(), PacketKind::PushData);
//! assert_eq!(packet.gateway(), Some(gateway));
//! assert_eq!(packet.acknowledgment().map(|ack| ack.to_bytes()), Some(vec![2, 0x01, 0x02, 0x01]));
//! # Ok::<(), pamoja_gateway::udp::ProtocolError>(())
//! ```

mod payload;

use std::fmt;

pub use payload::{CrcStatus, Modulation, Rxpk, Stat, TxStatus, Txpk, Uplink};

/// The protocol version every datagram starts with.
pub const PROTOCOL_VERSION: u8 = 2;

/// The bytes before a datagram's payload: the version, the token, and the identifier.
pub const HEADER_LEN: usize = 4;

/// The length of a gateway's unique identifier.
pub const EUI_LEN: usize = 8;

/// Which kind of datagram, as its fourth byte says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PacketKind {
    /// 0x00, the gateway forwarding what it heard.
    PushData,
    /// 0x01, the server acknowledging a PUSH_DATA.
    PushAck,
    /// 0x02, the gateway holding its route open.
    PullData,
    /// 0x03, the server sending a packet to transmit.
    PullResp,
    /// 0x04, the server acknowledging a PULL_DATA.
    PullAck,
    /// 0x05, the gateway reporting what became of a PULL_RESP.
    TxAck,
}

impl PacketKind {
    /// Returns the identifier byte.
    ///
    /// # Returns
    ///
    /// The value the datagram's fourth byte carries.
    pub const fn identifier(self) -> u8 {
        match self {
            PacketKind::PushData => 0x00,
            PacketKind::PushAck => 0x01,
            PacketKind::PullData => 0x02,
            PacketKind::PullResp => 0x03,
            PacketKind::PullAck => 0x04,
            PacketKind::TxAck => 0x05,
        }
    }

    /// Names the kind an identifier byte selects.
    ///
    /// # Arguments
    ///
    /// * `identifier` - the datagram's fourth byte.
    ///
    /// # Returns
    ///
    /// The kind, or `None` for a byte the protocol does not define.
    pub const fn from_identifier(identifier: u8) -> Option<PacketKind> {
        match identifier {
            0x00 => Some(PacketKind::PushData),
            0x01 => Some(PacketKind::PushAck),
            0x02 => Some(PacketKind::PullData),
            0x03 => Some(PacketKind::PullResp),
            0x04 => Some(PacketKind::PullAck),
            0x05 => Some(PacketKind::TxAck),
            _ => None,
        }
    }

    /// Reports whether this kind carries the gateway's identifier.
    ///
    /// # Returns
    ///
    /// `true` for PUSH_DATA, PULL_DATA, and TX_ACK.
    pub const fn carries_gateway(self) -> bool {
        matches!(
            self,
            PacketKind::PushData | PacketKind::PullData | PacketKind::TxAck
        )
    }
}

/// A gateway's unique identifier, the eight bytes it puts in every datagram it sends.
///
/// It is written from the host's MAC address, usually with `FF FE` in the middle, which is
/// why a Raspberry Pi gateway's identifier starts with the three bytes of its network
/// interface.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::udp::Eui;
///
/// let gateway = Eui::from_hex("b827ebfffe010203").expect("sixteen hex digits");
/// assert_eq!(gateway.to_hex(), "b827ebfffe010203");
/// assert_eq!(gateway.bytes()[0], 0xB8);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Eui([u8; EUI_LEN]);

impl Eui {
    /// Takes the eight bytes as they go on the wire.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the identifier, most significant byte first.
    ///
    /// # Returns
    ///
    /// The identifier.
    pub const fn new(bytes: [u8; EUI_LEN]) -> Eui {
        Eui(bytes)
    }

    /// Returns the bytes.
    ///
    /// # Returns
    ///
    /// The identifier as it goes on the wire.
    pub const fn bytes(&self) -> [u8; EUI_LEN] {
        self.0
    }

    /// Reads an identifier written as sixteen hexadecimal digits.
    ///
    /// # Arguments
    ///
    /// * `text` - the digits, in either case, with no separators.
    ///
    /// # Returns
    ///
    /// The identifier, or `None` when the text is not sixteen hexadecimal digits.
    pub fn from_hex(text: &str) -> Option<Eui> {
        if text.len() != EUI_LEN * 2 {
            return None;
        }
        let mut bytes = [0u8; EUI_LEN];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
        }
        Some(Eui(bytes))
    }

    /// Writes the identifier as sixteen lowercase hexadecimal digits.
    ///
    /// # Returns
    ///
    /// The digits, which is how a network server's console shows a gateway.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

impl fmt::Display for Eui {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Why a datagram is not one this protocol defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    /// A datagram shorter than the four bytes every kind starts with, with its length.
    Short(usize),
    /// A protocol version this crate does not speak.
    Version(u8),
    /// An identifier byte the protocol does not define.
    Identifier(u8),
    /// A datagram too short for the fields its kind carries.
    Truncated {
        /// The kind its identifier named.
        kind: PacketKind,
        /// How many bytes it holds.
        len: usize,
    },
    /// A payload that is not the JSON object the protocol describes, and why.
    Payload(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::Short(len) => {
                write!(f, "a datagram of {len} bytes is shorter than the header")
            }
            ProtocolError::Version(version) => {
                write!(f, "protocol version {version} is not {PROTOCOL_VERSION}")
            }
            ProtocolError::Identifier(identifier) => {
                write!(f, "{identifier:#04x} is not a packet identifier")
            }
            ProtocolError::Truncated { kind, len } => {
                write!(
                    f,
                    "a {kind:?} of {len} bytes is missing fields it must carry"
                )
            }
            ProtocolError::Payload(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// One datagram of the protocol.
///
/// [`to_bytes`](Packet::to_bytes) writes it and [`parse`](Packet::parse) reads one, so the
/// same type serves a gateway and a server.
#[derive(Clone, Debug, PartialEq)]
pub enum Packet {
    /// What the gateway heard, and how its own radio is doing.
    PushData {
        /// The random token the acknowledgment carries back.
        token: u16,
        /// The gateway's identifier.
        gateway: Eui,
        /// The packets and the status report.
        uplink: Uplink,
    },
    /// The server acknowledging a PUSH_DATA, which says only that it arrived.
    PushAck {
        /// The token of the PUSH_DATA being acknowledged.
        token: u16,
    },
    /// The gateway holding a route open for downlinks.
    PullData {
        /// The random token the acknowledgment carries back.
        token: u16,
        /// The gateway's identifier.
        gateway: Eui,
    },
    /// The server acknowledging a PULL_DATA.
    PullAck {
        /// The token of the PULL_DATA being acknowledged.
        token: u16,
    },
    /// The server asking the gateway to transmit a packet.
    PullResp {
        /// The random token the TX_ACK carries back.
        token: u16,
        /// What to transmit, and when.
        transmit: Txpk,
    },
    /// The gateway reporting what became of a PULL_RESP.
    TxAck {
        /// The token of the PULL_RESP being answered.
        token: u16,
        /// The gateway's identifier.
        gateway: Eui,
        /// Whether it was scheduled, or why it was refused.
        status: TxStatus,
    },
}

impl Packet {
    /// Returns which kind of datagram this is.
    ///
    /// # Returns
    ///
    /// The kind.
    pub const fn kind(&self) -> PacketKind {
        match self {
            Packet::PushData { .. } => PacketKind::PushData,
            Packet::PushAck { .. } => PacketKind::PushAck,
            Packet::PullData { .. } => PacketKind::PullData,
            Packet::PullAck { .. } => PacketKind::PullAck,
            Packet::PullResp { .. } => PacketKind::PullResp,
            Packet::TxAck { .. } => PacketKind::TxAck,
        }
    }

    /// Returns the token, which pairs a datagram with its answer.
    ///
    /// # Returns
    ///
    /// The token.
    pub const fn token(&self) -> u16 {
        match self {
            Packet::PushData { token, .. }
            | Packet::PushAck { token }
            | Packet::PullData { token, .. }
            | Packet::PullAck { token }
            | Packet::PullResp { token, .. }
            | Packet::TxAck { token, .. } => *token,
        }
    }

    /// Returns the gateway's identifier, for the kinds that carry it.
    ///
    /// # Returns
    ///
    /// The identifier, or `None` for the datagrams a server sends, which do not name one.
    pub const fn gateway(&self) -> Option<Eui> {
        match self {
            Packet::PushData { gateway, .. }
            | Packet::PullData { gateway, .. }
            | Packet::TxAck { gateway, .. } => Some(*gateway),
            _ => None,
        }
    }

    /// Returns the acknowledgment a server owes this datagram.
    ///
    /// # Returns
    ///
    /// The PUSH_ACK or PULL_ACK to send back, or `None` for a datagram that is itself an
    /// answer. A PULL_RESP is answered with a TX_ACK, which names the gateway and what
    /// became of the transmission, so the gateway builds that one itself.
    pub fn acknowledgment(&self) -> Option<Packet> {
        match self {
            Packet::PushData { token, .. } => Some(Packet::PushAck { token: *token }),
            Packet::PullData { token, .. } => Some(Packet::PullAck { token: *token }),
            _ => None,
        }
    }

    /// Writes the datagram.
    ///
    /// # Returns
    ///
    /// The bytes to send.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + EUI_LEN);
        out.push(PROTOCOL_VERSION);
        out.extend_from_slice(&self.token().to_be_bytes());
        out.push(self.kind().identifier());
        if let Some(gateway) = self.gateway() {
            out.extend_from_slice(&gateway.bytes());
        }
        match self {
            Packet::PushData { uplink, .. } => out.extend_from_slice(uplink.to_json().as_bytes()),
            Packet::PullResp { transmit, .. } => {
                out.extend_from_slice(transmit.to_json().as_bytes());
            }
            Packet::TxAck { status, .. } => out.extend_from_slice(status.to_json().as_bytes()),
            _ => {}
        }
        out
    }

    /// Reads a datagram.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the datagram as it arrived.
    ///
    /// # Returns
    ///
    /// The packet.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError`] for a datagram shorter than its header, another protocol
    /// version, an identifier the protocol does not define, a datagram missing the fields its
    /// kind carries, or a payload that is not the JSON object described for it.
    pub fn parse(bytes: &[u8]) -> Result<Packet, ProtocolError> {
        if bytes.len() < HEADER_LEN {
            return Err(ProtocolError::Short(bytes.len()));
        }
        if bytes[0] != PROTOCOL_VERSION {
            return Err(ProtocolError::Version(bytes[0]));
        }
        let token = u16::from_be_bytes([bytes[1], bytes[2]]);
        let kind =
            PacketKind::from_identifier(bytes[3]).ok_or(ProtocolError::Identifier(bytes[3]))?;

        let body = &bytes[HEADER_LEN..];
        let (gateway, body) = if kind.carries_gateway() {
            if body.len() < EUI_LEN {
                return Err(ProtocolError::Truncated {
                    kind,
                    len: bytes.len(),
                });
            }
            let mut identifier = [0u8; EUI_LEN];
            identifier.copy_from_slice(&body[..EUI_LEN]);
            (Eui(identifier), &body[EUI_LEN..])
        } else {
            (Eui::default(), body)
        };

        Ok(match kind {
            PacketKind::PushData => Packet::PushData {
                token,
                gateway,
                uplink: Uplink::from_json(body)?,
            },
            PacketKind::PushAck => Packet::PushAck { token },
            PacketKind::PullData => Packet::PullData { token, gateway },
            PacketKind::PullAck => Packet::PullAck { token },
            PacketKind::PullResp => Packet::PullResp {
                token,
                transmit: Txpk::from_json(body)?,
            },
            PacketKind::TxAck => Packet::TxAck {
                token,
                gateway,
                status: TxStatus::from_json(body)?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pamoja_lora::LinkSettings;

    fn gateway() -> Eui {
        Eui::new([0xB8, 0x27, 0xEB, 0xFF, 0xFE, 0x01, 0x02, 0x03])
    }

    fn heard() -> Rxpk {
        Rxpk::new(
            868_100_000,
            LinkSettings::new(7, 125_000),
            b"hello".to_vec(),
        )
    }

    #[test]
    fn every_identifier_is_the_one_the_protocol_gives() {
        for (kind, identifier) in [
            (PacketKind::PushData, 0x00),
            (PacketKind::PushAck, 0x01),
            (PacketKind::PullData, 0x02),
            (PacketKind::PullResp, 0x03),
            (PacketKind::PullAck, 0x04),
            (PacketKind::TxAck, 0x05),
        ] {
            assert_eq!(kind.identifier(), identifier);
            assert_eq!(PacketKind::from_identifier(identifier), Some(kind));
        }
        assert_eq!(PacketKind::from_identifier(0x06), None);
    }

    #[test]
    fn a_push_data_carries_its_header_then_its_json() {
        let datagram = Packet::PushData {
            token: 0x1234,
            gateway: gateway(),
            uplink: Uplink::from(heard()),
        }
        .to_bytes();

        assert_eq!(datagram[0], PROTOCOL_VERSION);
        assert_eq!(&datagram[1..3], [0x12, 0x34]);
        assert_eq!(datagram[3], 0x00);
        assert_eq!(&datagram[4..12], gateway().bytes());
        assert!(datagram[12..].starts_with(b"{\"rxpk\":["));
        assert_eq!(Packet::parse(&datagram).expect("it parses").token(), 0x1234);
    }

    #[test]
    fn the_acknowledgments_are_four_bytes_with_the_same_token() {
        let push = Packet::PushData {
            token: 0x0102,
            gateway: gateway(),
            uplink: Uplink::from(heard()),
        };
        let pull = Packet::PullData {
            token: 0x0304,
            gateway: gateway(),
        };

        assert_eq!(
            push.acknowledgment()
                .expect("a push is answered")
                .to_bytes(),
            [2, 0x01, 0x02, 0x01]
        );
        assert_eq!(
            pull.acknowledgment()
                .expect("a pull is answered")
                .to_bytes(),
            [2, 0x03, 0x04, 0x04]
        );
        assert_eq!(Packet::PushAck { token: 1 }.acknowledgment(), None);
    }

    #[test]
    fn a_pull_data_is_twelve_bytes_and_a_tx_ack_carries_its_status() {
        let pull = Packet::PullData {
            token: 0xABCD,
            gateway: gateway(),
        };
        let bytes = pull.to_bytes();
        assert_eq!(bytes.len(), 12);
        assert_eq!(Packet::parse(&bytes), Ok(pull));

        let ack = Packet::TxAck {
            token: 0xABCD,
            gateway: gateway(),
            status: TxStatus::CollisionPacket,
        };
        let bytes = ack.to_bytes();
        assert!(bytes[12..].starts_with(b"{\"txpk_ack\":"));
        assert_eq!(Packet::parse(&bytes), Ok(ack));
    }

    #[test]
    fn a_tx_ack_without_a_payload_reports_no_error() {
        let bytes = [vec![2, 0x00, 0x01, 0x05], gateway().bytes().to_vec()].concat();
        assert_eq!(
            Packet::parse(&bytes),
            Ok(Packet::TxAck {
                token: 1,
                gateway: gateway(),
                status: TxStatus::None,
            })
        );
    }

    #[test]
    fn a_datagram_that_is_not_this_protocol_is_refused() {
        assert_eq!(Packet::parse(&[2, 0, 1]), Err(ProtocolError::Short(3)));
        assert_eq!(Packet::parse(&[1, 0, 1, 0]), Err(ProtocolError::Version(1)));
        assert_eq!(
            Packet::parse(&[2, 0, 1, 0x09]),
            Err(ProtocolError::Identifier(0x09))
        );
        assert_eq!(
            Packet::parse(&[2, 0, 1, 0x02, 0xB8]),
            Err(ProtocolError::Truncated {
                kind: PacketKind::PullData,
                len: 5,
            })
        );
        assert!(matches!(
            Packet::parse(
                &[
                    vec![2, 0, 1, 0x00],
                    gateway().bytes().to_vec(),
                    b"{}".to_vec()
                ]
                .concat()
            ),
            Err(ProtocolError::Payload(_))
        ));
    }

    #[test]
    fn an_eui_reads_and_writes_as_hex() {
        assert_eq!(Eui::from_hex("b827ebfffe010203"), Some(gateway()));
        assert_eq!(Eui::from_hex("B827EBFFFE010203"), Some(gateway()));
        assert_eq!(gateway().to_string(), "b827ebfffe010203");
        assert_eq!(Eui::from_hex("b827ebfffe0102"), None);
        assert_eq!(Eui::from_hex("b827ebfffe01020g"), None);
    }
}
