//! LoRaWAN gateway protocols for the pamoja SDK.
//!
//! A LoRa gateway is a radio with an uplink: it hears packets from every node in range and
//! hands them to a network server, which hands back the packets to transmit. The protocol
//! between the two is not LoRaWAN, which is what the packets themselves carry; it is a
//! separate, deliberately plain exchange of UDP datagrams, and this crate speaks it from
//! both sides.
//!
//! - [`udp`] - the Semtech packet forwarder protocol: the PUSH_DATA and PULL_DATA datagrams a
//!   gateway sends, the PUSH_ACK, PULL_ACK and PULL_RESP a server answers with, the TX_ACK
//!   that reports what became of a downlink, and the `rxpk`, `stat`, `txpk` and `txpk_ack`
//!   objects they carry.
//! - [`base64`] - the payload encoding of RFC 4648, padded on the way out and read either
//!   way on the way in, because gateways in the field send both.
//! - [`time`] - the two timestamp formats the protocol prescribes, to the microsecond for a
//!   reception and to the second for a gateway's own clock.
//! - `network`, with the `network` feature - the network side of one site: admitting a join,
//!   reading an uplink, and working out where and when to answer it.
//! - `bridge`, with the `bridge` feature - carrying messages between the radio the nodes are
//!   on and the link that leaves the site, under a prefix that names the site.
//!
//! Every datagram is data: the crate builds and parses them, and leaves the socket, the
//! keepalive, and the scheduling to the program that owns them.
//!
//! # Examples
//!
//! A gateway forwards one packet it heard, and the server acknowledges it.
//!
//! ```
//! use pamoja_gateway::udp::{Eui, Packet, Rxpk, Uplink};
//! use pamoja_lora::LinkSettings;
//!
//! let gateway = Eui::new([0xB8, 0x27, 0xEB, 0xFF, 0xFE, 0x01, 0x02, 0x03]);
//! let heard = Rxpk::new(868_100_000, LinkSettings::new(7, 125_000), b"hello".to_vec())
//!     .with_rssi_dbm(-35)
//!     .with_snr_db(5.1);
//! let push = Packet::PushData {
//!     token: 0x1234,
//!     gateway,
//!     uplink: Uplink::from(heard),
//! };
//!
//! let datagram = push.to_bytes();
//! assert_eq!(&datagram[..4], &[2, 0x12, 0x34, 0x00]);
//!
//! // The server reads it and answers with the same token.
//! let received = Packet::parse(&datagram).expect("the datagram is well formed");
//! let ack = received.acknowledgment().expect("a PUSH_DATA is acknowledged");
//! assert_eq!(ack.to_bytes(), [2, 0x12, 0x34, 0x01]);
//! ```

pub mod base64;
#[cfg(feature = "bridge")]
pub mod bridge;
#[cfg(feature = "network")]
pub mod network;
pub mod time;
pub mod udp;
