//! The LoRa Basics Station protocol, both sides.
//!
//! The packet forwarder protocol in [`udp`](crate::udp) is a gateway shouting datagrams at a
//! server that may or may not be listening. Basics Station replaces it with a conversation: the
//! gateway asks a discovery endpoint where its network server is, opens a websocket there, says
//! what it is, and is told how to configure its radios. After that the two exchange JSON
//! messages, each naming its kind in a `msgtype` field.
//!
//! This module builds and reads those messages from either side, the way `udp` does. It opens
//! no sockets: a station reaches its server through whatever the caller provides, and the
//! `station-client` feature adds a live one.
//!
//! - [`Discovery`] and [`Router`] are the request and answer on `/router-info`, which hands
//!   back the websocket address to connect to, or says why it will not.
//! - [`Message`] is every message the websocket carries afterwards: what the station reports
//!   about itself, the configuration it is given, the join requests and frames it heard, the
//!   downlinks it is asked to transmit and what became of them, and the clock the two keep
//!   between them.
//!
//! Identifiers cross as [`Eui`], and are written in the ID6 form the protocol prefers,
//! which folds a run of zero groups the way an IPv6 address does.
//!
//! # Examples
//!
//! A station asks where its network server is, and reads the answer.
//!
//! ```
//! use pamoja_gateway::station::{Discovery, Router};
//! use pamoja_gateway::udp::Eui;
//!
//! let asking = Discovery::new(Eui::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]));
//! assert_eq!(asking.to_json(), r#"{"router":"102:304:506:708"}"#);
//!
//! // The server answers with the websocket to open.
//! let answer = Router::from_json(
//!     br#"{"router":"1:203:405:607:8","muxs":"::1","uri":"ws://lns.example.invalid:3001/router"}"#,
//! )
//! .expect("the answer is well formed");
//! assert_eq!(answer.uri.as_deref(), Some("ws://lns.example.invalid:3001/router"));
//! assert!(answer.error.is_none());
//! ```

use serde_json::{json, Map, Value};

use crate::udp::{Eui, ProtocolError};

/// The path a station appends to its configured address to find its network server.
pub const DISCOVERY_PATH: &str = "/router-info";

/// The protocol version a station reports, which is 2 for the protocol described here.
pub const PROTOCOL_VERSION: u32 = 2;

/// The shortest frame that can carry a header and an integrity code.
const MINIMUM_FRAME: usize = 5;

/// How many bytes the message integrity code takes, at the end of every frame.
const MIC_LEN: usize = 4;

/// What a join request carries between its header and its integrity code.
const JOIN_REQUEST_LEN: usize = 18;

/// The address, control byte, and counter every data frame starts with.
const FRAME_HEADER_LEN: usize = 7;

/// A join request, in the top three bits of the header byte.
const JOIN_REQUEST: u8 = 0;

/// An uplink that asks for no acknowledgment.
const UNCONFIRMED_UP: u8 = 2;

/// An uplink that asks to be acknowledged.
const CONFIRMED_UP: u8 = 4;

/// A frame this protocol does not describe, which a station passes along whole.
const PROPRIETARY: u8 = 7;

/// Writes an identifier in the ID6 form the protocol prefers.
///
/// The eight bytes are read as four groups of sixteen bits and written like an IPv6 address:
/// leading zeros in a group are dropped, and the longest run of zero groups becomes `::`.
///
/// # Arguments
///
/// * `eui` - the identifier to write.
///
/// # Returns
///
/// The identifier, such as `1:203:405:607:8` or `::1`.
pub fn id6(eui: Eui) -> String {
    let bytes = eui.bytes();
    let groups: [u16; 4] = [
        u16::from_be_bytes([bytes[0], bytes[1]]),
        u16::from_be_bytes([bytes[2], bytes[3]]),
        u16::from_be_bytes([bytes[4], bytes[5]]),
        u16::from_be_bytes([bytes[6], bytes[7]]),
    ];

    // The longest run of zero groups is elided, and a tie keeps the first run.
    let mut best_at = 0;
    let mut best_len = 0;
    let mut at = 0;
    while at < groups.len() {
        if groups[at] != 0 {
            at += 1;
            continue;
        }
        let start = at;
        while at < groups.len() && groups[at] == 0 {
            at += 1;
        }
        if at - start > best_len {
            best_at = start;
            best_len = at - start;
        }
    }

    if best_len == groups.len() {
        return "::".to_string();
    }
    if best_len < 2 {
        let written: Vec<String> = groups.iter().map(|group| format!("{group:x}")).collect();
        return written.join(":");
    }

    let head: Vec<String> = groups[..best_at]
        .iter()
        .map(|group| format!("{group:x}"))
        .collect();
    let tail: Vec<String> = groups[best_at + best_len..]
        .iter()
        .map(|group| format!("{group:x}"))
        .collect();
    format!("{}::{}", head.join(":"), tail.join(":"))
}

/// Reads an identifier written in ID6, the dashed EUI form, or as a decimal number.
///
/// # Arguments
///
/// * `text` - the identifier as the protocol writes it.
///
/// # Returns
///
/// The identifier, or `None` when the text is none of those three forms.
pub fn eui_of(text: &str) -> Option<Eui> {
    // The dashed form, which is eight pairs of hexadecimal digits.
    if text.contains('-') {
        let mut bytes = [0u8; 8];
        let mut seen = 0;
        for part in text.split('-') {
            if seen == bytes.len() {
                return None;
            }
            bytes[seen] = u8::from_str_radix(part, 16).ok()?;
            seen += 1;
        }
        return (seen == bytes.len()).then(|| Eui::new(bytes));
    }

    // A plain number, which the protocol also accepts.
    if !text.contains(':') {
        return text
            .parse::<u64>()
            .ok()
            .map(|value| Eui::new(value.to_be_bytes()));
    }

    // The ID6 form, four groups with at most one elision.
    let (head, tail) = match text.split_once("::") {
        Some((head, tail)) => (head, tail),
        None => (text, ""),
    };
    let elided = text.contains("::");

    let mut groups: Vec<u16> = Vec::new();
    for part in head.split(':').filter(|part| !part.is_empty()) {
        groups.push(u16::from_str_radix(part, 16).ok()?);
    }
    let mut rest: Vec<u16> = Vec::new();
    for part in tail.split(':').filter(|part| !part.is_empty()) {
        rest.push(u16::from_str_radix(part, 16).ok()?);
    }

    if elided {
        let missing = 4usize.checked_sub(groups.len() + rest.len())?;
        groups.extend(core::iter::repeat_n(0u16, missing));
    } else if groups.len() + rest.len() != 4 {
        return None;
    }
    groups.extend(rest);
    if groups.len() != 4 {
        return None;
    }

    let mut bytes = [0u8; 8];
    for (index, group) in groups.iter().enumerate() {
        let pair = group.to_be_bytes();
        bytes[index * 2] = pair[0];
        bytes[index * 2 + 1] = pair[1];
    }
    Some(Eui::new(bytes))
}

/// What a station asks the discovery endpoint, naming itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Discovery {
    /// The station asking.
    pub router: Eui,
}

impl Discovery {
    /// Builds the discovery request for a station.
    ///
    /// # Arguments
    ///
    /// * `router` - the station's own identifier.
    ///
    /// # Returns
    ///
    /// The request.
    pub const fn new(router: Eui) -> Discovery {
        Discovery { router }
    }

    /// Writes the request as the endpoint expects it.
    ///
    /// # Returns
    ///
    /// The JSON text to send on [`DISCOVERY_PATH`].
    pub fn to_json(&self) -> String {
        json!({ "router": id6(self.router) }).to_string()
    }

    /// Reads a discovery request, as a server does.
    ///
    /// # Arguments
    ///
    /// * `body` - the request body.
    ///
    /// # Returns
    ///
    /// The request.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the body is not this request.
    pub fn from_json(body: &[u8]) -> Result<Discovery, ProtocolError> {
        let value: Value = serde_json::from_slice(body)
            .map_err(|error| ProtocolError::Payload(error.to_string()))?;
        let router = value
            .get("router")
            .and_then(Value::as_str)
            .and_then(eui_of)
            .ok_or_else(|| ProtocolError::Payload("router is not an identifier".to_string()))?;
        Ok(Discovery { router })
    }
}

/// What the discovery endpoint answers: where to connect, or why not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Router {
    /// The station, as the server read it.
    pub router: Option<Eui>,
    /// The server endpoint that will carry the session.
    pub muxs: Option<Eui>,
    /// The websocket to open, absolute.
    pub uri: Option<String>,
    /// Why the station was refused, when it was.
    pub error: Option<String>,
}

impl Router {
    /// Answers a station with the websocket to open.
    ///
    /// # Arguments
    ///
    /// * `router` - the station being answered.
    /// * `muxs` - the server endpoint carrying the session.
    /// * `uri` - the websocket address.
    ///
    /// # Returns
    ///
    /// The answer.
    pub fn accepted(router: Eui, muxs: Eui, uri: impl Into<String>) -> Router {
        Router {
            router: Some(router),
            muxs: Some(muxs),
            uri: Some(uri.into()),
            error: None,
        }
    }

    /// Refuses a station, saying why.
    ///
    /// # Arguments
    ///
    /// * `router` - the station being refused.
    /// * `error` - what is wrong, in words the operator can act on.
    ///
    /// # Returns
    ///
    /// The answer.
    pub fn refused(router: Eui, error: impl Into<String>) -> Router {
        Router {
            router: Some(router),
            muxs: None,
            uri: None,
            error: Some(error.into()),
        }
    }

    /// Writes the answer.
    ///
    /// # Returns
    ///
    /// The JSON body.
    pub fn to_json(&self) -> String {
        let mut object = serde_json::Map::new();
        if let Some(router) = self.router {
            object.insert("router".to_string(), Value::String(id6(router)));
        }
        if let Some(muxs) = self.muxs {
            object.insert("muxs".to_string(), Value::String(id6(muxs)));
        }
        if let Some(uri) = &self.uri {
            object.insert("uri".to_string(), Value::String(uri.clone()));
        }
        if let Some(error) = &self.error {
            object.insert("error".to_string(), Value::String(error.clone()));
        }
        Value::Object(object).to_string()
    }

    /// Reads the answer, as a station does.
    ///
    /// # Arguments
    ///
    /// * `body` - the answer body.
    ///
    /// # Returns
    ///
    /// The answer, whose `error` says whether the station was refused.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the body is not JSON.
    pub fn from_json(body: &[u8]) -> Result<Router, ProtocolError> {
        let value: Value = serde_json::from_slice(body)
            .map_err(|error| ProtocolError::Payload(error.to_string()))?;
        Ok(Router {
            router: value.get("router").and_then(Value::as_str).and_then(eui_of),
            muxs: value.get("muxs").and_then(Value::as_str).and_then(eui_of),
            uri: value.get("uri").and_then(Value::as_str).map(str::to_string),
            error: value
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }
}

/// The levels a packet was heard at, which every uplink carries.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Levels {
    /// The radio context the station uses to answer this packet.
    pub rctx: i64,
    /// The station's own clock, in microseconds.
    pub xtime: i64,
    /// The GPS time, when the station has one.
    pub gpstime: Option<i64>,
    /// The received signal strength, in dBm.
    pub rssi: f64,
    /// The signal-to-noise ratio, in dB.
    pub snr: f64,
}

/// One frame of a [`Message::Schedule`], transmitted to a group rather than a device.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Broadcast {
    /// The frame to transmit.
    pub pdu: Vec<u8>,
    /// The data rate to transmit at.
    pub data_rate: u8,
    /// The frequency to transmit on, in hertz.
    pub frequency_hz: u32,
    /// How urgent it is.
    pub priority: u8,
    /// The GPS time to transmit at.
    pub gpstime: Option<i64>,
    /// The radio to transmit on.
    pub rctx: Option<i64>,
}

impl Broadcast {
    /// Writes the frame as a schedule carries it.
    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("pdu".to_owned(), Value::String(hex(&self.pdu)));
        object.insert("DR".to_owned(), json!(self.data_rate));
        object.insert("Freq".to_owned(), json!(self.frequency_hz));
        object.insert("priority".to_owned(), json!(self.priority));
        if let Some(gpstime) = self.gpstime {
            object.insert("gpstime".to_owned(), json!(gpstime));
        }
        if let Some(rctx) = self.rctx {
            object.insert("rctx".to_owned(), json!(rctx));
        }
        Value::Object(object)
    }

    /// Reads one frame of a schedule.
    fn from_value(value: &Value) -> Result<Broadcast, ProtocolError> {
        let object = value
            .as_object()
            .ok_or_else(|| refused("a scheduled frame is not a JSON object"))?;
        Ok(Broadcast {
            pdu: bytes_of(object, "pdu")?,
            data_rate: whole(object, "DR").unwrap_or(0) as u8,
            frequency_hz: whole(object, "Freq").unwrap_or(0) as u32,
            priority: whole(object, "priority").unwrap_or(0) as u8,
            gpstime: whole(object, "gpstime"),
            rctx: whole(object, "rctx"),
        })
    }
}

/// Every message the websocket carries once the session is open.
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    /// What the station is, sent first thing after the websocket opens.
    Version {
        /// The station software.
        station: String,
        /// Its firmware.
        firmware: String,
        /// The package it came from.
        package: String,
        /// The hardware model.
        model: String,
        /// The protocol version it speaks.
        protocol: u32,
        /// What it can do, such as `gps` or `prod`.
        features: String,
    },
    /// How the server tells the station to configure its radios.
    RouterConfig {
        /// The network identifiers whose frames are carried.
        net_id: Vec<u32>,
        /// The join identifier ranges that are admitted, as inclusive pairs.
        join_eui: Vec<(u64, u64)>,
        /// The region name, such as `EU863`.
        region: String,
        /// The highest radiated power the region allows, in dBm.
        max_eirp: f64,
        /// The concentrator the configuration is written for, such as `sx1301/1`.
        hwspec: String,
        /// The lowest and highest frequency the station may use, in hertz.
        freq_range: (u32, u32),
        /// The data rates, each a spreading factor, a bandwidth, and whether it is
        /// downlink only.
        data_rates: Vec<(u8, u32, bool)>,
    },
    /// A join request the station heard.
    JoinRequest {
        /// The MAC header byte.
        mhdr: u8,
        /// The application the device is joining.
        join_eui: Eui,
        /// The device asking.
        dev_eui: Eui,
        /// The nonce it used, which a server must not see twice.
        dev_nonce: u16,
        /// The message integrity code.
        mic: i32,
        /// The data rate it arrived at.
        data_rate: u8,
        /// The frequency it arrived on, in hertz.
        frequency_hz: u32,
        /// How it was heard.
        levels: Levels,
    },
    /// A data frame the station heard.
    Uplink {
        /// The MAC header byte.
        mhdr: u8,
        /// The device address it came from.
        dev_addr: i32,
        /// The frame control byte.
        fctrl: u8,
        /// The frame counter, as the sixteen bits on the air.
        fcnt: u16,
        /// The frame options, as bytes.
        fopts: Vec<u8>,
        /// The port, or `None` for a frame carrying only options.
        fport: Option<u8>,
        /// The encrypted payload.
        payload: Vec<u8>,
        /// The message integrity code.
        mic: i32,
        /// The data rate it arrived at.
        data_rate: u8,
        /// The frequency it arrived on, in hertz.
        frequency_hz: u32,
        /// How it was heard.
        levels: Levels,
    },
    /// A frame this protocol does not describe, which the station heard and passes on whole.
    Proprietary {
        /// The frame as it arrived, header and integrity code included.
        payload: Vec<u8>,
        /// The data rate it arrived at.
        data_rate: u8,
        /// The frequency it arrived on, in hertz.
        frequency_hz: u32,
        /// How it was heard.
        levels: Levels,
    },
    /// A frame the server asks the station to transmit.
    Downlink {
        /// The device it is for.
        dev_eui: Eui,
        /// Which class of downlink this is.
        class: u8,
        /// The identifier the transmission report carries back.
        diid: i64,
        /// The frame to transmit.
        pdu: Vec<u8>,
        /// The delay before the first receive window, in seconds.
        rx_delay: Option<u8>,
        /// The first window, as a data rate and a frequency in hertz.
        rx1: Option<(u8, u32)>,
        /// The second window, as a data rate and a frequency in hertz.
        rx2: Option<(u8, u32)>,
        /// How urgent it is.
        priority: u8,
        /// The station clock the first window is counted from.
        xtime: Option<i64>,
        /// The radio context the uplink was heard on.
        rctx: Option<i64>,
    },
    /// Frames the server asks the station to transmit to a group at a given time.
    ///
    /// A station reports nothing back about these, unlike a [`Message::Downlink`].
    Schedule {
        /// What to transmit, in the order given.
        frames: Vec<Broadcast>,
    },
    /// What became of a frame the station was asked to transmit.
    Transmitted {
        /// The identifier from the downlink.
        diid: i64,
        /// The device it was for.
        dev_eui: Eui,
        /// The radio context it went out on.
        rctx: i64,
        /// The station clock it went out at.
        xtime: i64,
        /// The time it went out, in seconds.
        txtime: f64,
        /// The GPS time, when the station has one.
        gpstime: Option<i64>,
    },
    /// The clock the two keep between them.
    TimeSync {
        /// The station clock.
        txtime: Option<i64>,
        /// The station clock, when the server is transferring GPS time against it.
        xtime: Option<i64>,
        /// The GPS time.
        gpstime: Option<i64>,
    },
    /// A message this crate does not read, kept so a caller can see what arrived.
    Other {
        /// What the message called itself.
        msgtype: String,
    },
}

impl Message {
    /// Returns the word this message calls itself on the wire.
    ///
    /// # Returns
    ///
    /// The `msgtype` value, such as `updf`.
    pub fn msgtype(&self) -> &str {
        match self {
            Message::Version { .. } => "version",
            Message::RouterConfig { .. } => "router_config",
            Message::JoinRequest { .. } => "jreq",
            Message::Uplink { .. } => "updf",
            Message::Proprietary { .. } => "propdf",
            Message::Downlink { .. } => "dnmsg",
            Message::Schedule { .. } => "dnsched",
            Message::Transmitted { .. } => "dntxed",
            Message::TimeSync { .. } => "timesync",
            Message::Other { msgtype } => msgtype,
        }
    }

    /// Reads a frame the radio heard into the message that reports it.
    ///
    /// A station holds no key, so nothing is verified here: the frame is split into the
    /// fields the protocol names and the server judges them. A join request becomes
    /// [`Message::JoinRequest`] and a data frame [`Message::Uplink`].
    ///
    /// # Arguments
    ///
    /// * `frame` - the bytes as they arrived, header through message integrity code.
    /// * `data_rate` - the data rate it arrived at.
    /// * `frequency_hz` - the frequency it arrived on, in hertz.
    /// * `levels` - how it was heard.
    ///
    /// # Returns
    ///
    /// The message to send up.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the frame is shorter than the fields its
    /// header names, or carries a kind a station does not send up.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_gateway::station::{Levels, Message};
    ///
    /// // An unconfirmed frame going up, carrying one byte on port two.
    /// let frame = [
    ///     0x40, 0x01, 0x00, 0x01, 0x26, 0x00, 0x07, 0x00, 0x02, 0x41, 0x11, 0x22, 0x33, 0x44,
    /// ];
    /// let heard = Message::heard(&frame, 5, 868_100_000, Levels::default())
    ///     .expect("a station sends this one up");
    ///
    /// match heard {
    ///     Message::Uplink { fcnt, fport, payload, .. } => {
    ///         assert_eq!(fcnt, 7);
    ///         assert_eq!(fport, Some(2));
    ///         assert_eq!(payload, vec![0x41]);
    ///     }
    ///     other => panic!("that is a data frame, not {}", other.msgtype()),
    /// }
    /// ```
    pub fn heard(
        frame: &[u8],
        data_rate: u8,
        frequency_hz: u32,
        levels: Levels,
    ) -> Result<Message, ProtocolError> {
        // Every frame is one header byte, what it carries, and four bytes of integrity code.
        if frame.len() < MINIMUM_FRAME {
            return Err(refused(&format!(
                "a frame of {} bytes is shorter than a header and an integrity code",
                frame.len()
            )));
        }
        let mhdr = frame[0];
        let body = &frame[1..frame.len() - MIC_LEN];
        let mic = i32::from_le_bytes([
            frame[frame.len() - 4],
            frame[frame.len() - 3],
            frame[frame.len() - 2],
            frame[frame.len() - 1],
        ]);

        // The top three bits of the header byte say what kind of frame this is.
        match mhdr >> 5 {
            JOIN_REQUEST => {
                if body.len() != JOIN_REQUEST_LEN {
                    return Err(refused(&format!(
                        "a join request carries {JOIN_REQUEST_LEN} bytes, not {}",
                        body.len()
                    )));
                }
                Ok(Message::JoinRequest {
                    mhdr,
                    join_eui: least_first(&body[0..8]),
                    dev_eui: least_first(&body[8..16]),
                    dev_nonce: u16::from_le_bytes([body[16], body[17]]),
                    mic,
                    data_rate,
                    frequency_hz,
                    levels,
                })
            }
            UNCONFIRMED_UP | CONFIRMED_UP => {
                if body.len() < FRAME_HEADER_LEN {
                    return Err(refused(&format!(
                        "a data frame carries {FRAME_HEADER_LEN} bytes of header, not {}",
                        body.len()
                    )));
                }
                let fctrl = body[4];
                // The low four bits of the frame control byte count the options after it.
                let options = usize::from(fctrl & 0x0f);
                let carried = FRAME_HEADER_LEN + options;
                if body.len() < carried {
                    return Err(refused(&format!(
                        "a frame naming {options} bytes of options carries {} in all",
                        body.len()
                    )));
                }
                let rest = &body[carried..];
                Ok(Message::Uplink {
                    mhdr,
                    dev_addr: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
                    fctrl,
                    fcnt: u16::from_le_bytes([body[5], body[6]]),
                    fopts: body[FRAME_HEADER_LEN..carried].to_vec(),
                    // A frame with nothing after its options carries no port at all.
                    fport: rest.first().copied(),
                    payload: rest.get(1..).unwrap_or_default().to_vec(),
                    mic,
                    data_rate,
                    frequency_hz,
                    levels,
                })
            }
            PROPRIETARY => Ok(Message::Proprietary {
                // The protocol carries a proprietary frame whole, header and all.
                payload: frame.to_vec(),
                data_rate,
                frequency_hz,
                levels,
            }),
            kind => Err(refused(&format!(
                "a station does not send a frame of kind {kind} up"
            ))),
        }
    }

    /// Writes the message as the websocket carries it.
    ///
    /// # Returns
    ///
    /// The JSON text to send.
    pub fn to_json(&self) -> String {
        let mut object = Map::new();
        object.insert(
            "msgtype".to_owned(),
            Value::String(self.msgtype().to_owned()),
        );

        match self {
            Message::Version {
                station,
                firmware,
                package,
                model,
                protocol,
                features,
            } => {
                object.insert("station".to_owned(), Value::String(station.clone()));
                object.insert("firmware".to_owned(), Value::String(firmware.clone()));
                object.insert("package".to_owned(), Value::String(package.clone()));
                object.insert("model".to_owned(), Value::String(model.clone()));
                object.insert("protocol".to_owned(), json!(protocol));
                object.insert("features".to_owned(), Value::String(features.clone()));
            }
            Message::RouterConfig {
                net_id,
                join_eui,
                region,
                max_eirp,
                hwspec,
                freq_range,
                data_rates,
            } => {
                object.insert("NetID".to_owned(), json!(net_id));
                object.insert(
                    "JoinEui".to_owned(),
                    Value::Array(
                        join_eui
                            .iter()
                            .map(|(begin, end)| json!([begin, end]))
                            .collect(),
                    ),
                );
                object.insert("region".to_owned(), Value::String(region.clone()));
                object.insert("max_eirp".to_owned(), json!(max_eirp));
                object.insert("hwspec".to_owned(), Value::String(hwspec.clone()));
                object.insert("freq_range".to_owned(), json!([freq_range.0, freq_range.1]));
                object.insert(
                    "DRs".to_owned(),
                    Value::Array(
                        data_rates
                            .iter()
                            .map(|(sf, bw, down)| json!([sf, bw / 1_000, u8::from(*down)]))
                            .collect(),
                    ),
                );
            }
            Message::JoinRequest {
                mhdr,
                join_eui,
                dev_eui,
                dev_nonce,
                mic,
                data_rate,
                frequency_hz,
                levels,
            } => {
                object.insert("MHdr".to_owned(), json!(mhdr));
                object.insert("JoinEui".to_owned(), Value::String(id6(*join_eui)));
                object.insert("DevEui".to_owned(), Value::String(id6(*dev_eui)));
                object.insert("DevNonce".to_owned(), json!(dev_nonce));
                object.insert("MIC".to_owned(), json!(mic));
                object.insert("DR".to_owned(), json!(data_rate));
                object.insert("Freq".to_owned(), json!(frequency_hz));
                object.insert("upinfo".to_owned(), levels.to_value());
            }
            Message::Uplink {
                mhdr,
                dev_addr,
                fctrl,
                fcnt,
                fopts,
                fport,
                payload,
                mic,
                data_rate,
                frequency_hz,
                levels,
            } => {
                object.insert("MHdr".to_owned(), json!(mhdr));
                object.insert("DevAddr".to_owned(), json!(dev_addr));
                object.insert("FCtrl".to_owned(), json!(fctrl));
                object.insert("FCnt".to_owned(), json!(fcnt));
                object.insert("FOpts".to_owned(), Value::String(hex(fopts)));
                object.insert("FPort".to_owned(), json!(fport.map_or(-1, i16::from)));
                object.insert("FRMPayload".to_owned(), Value::String(hex(payload)));
                object.insert("MIC".to_owned(), json!(mic));
                object.insert("DR".to_owned(), json!(data_rate));
                object.insert("Freq".to_owned(), json!(frequency_hz));
                object.insert("upinfo".to_owned(), levels.to_value());
            }
            Message::Proprietary {
                payload,
                data_rate,
                frequency_hz,
                levels,
            } => {
                object.insert("FRMPayload".to_owned(), Value::String(hex(payload)));
                object.insert("DR".to_owned(), json!(data_rate));
                object.insert("Freq".to_owned(), json!(frequency_hz));
                object.insert("upinfo".to_owned(), levels.to_value());
            }
            Message::Downlink {
                dev_eui,
                class,
                diid,
                pdu,
                rx_delay,
                rx1,
                rx2,
                priority,
                xtime,
                rctx,
            } => {
                object.insert("DevEui".to_owned(), Value::String(id6(*dev_eui)));
                object.insert("dC".to_owned(), json!(class));
                object.insert("diid".to_owned(), json!(diid));
                object.insert("pdu".to_owned(), Value::String(hex(pdu)));
                if let Some(delay) = rx_delay {
                    object.insert("RxDelay".to_owned(), json!(delay));
                }
                if let Some((data_rate, frequency_hz)) = rx1 {
                    object.insert("RX1DR".to_owned(), json!(data_rate));
                    object.insert("RX1Freq".to_owned(), json!(frequency_hz));
                }
                if let Some((data_rate, frequency_hz)) = rx2 {
                    object.insert("RX2DR".to_owned(), json!(data_rate));
                    object.insert("RX2Freq".to_owned(), json!(frequency_hz));
                }
                object.insert("priority".to_owned(), json!(priority));
                if let Some(xtime) = xtime {
                    object.insert("xtime".to_owned(), json!(xtime));
                }
                if let Some(rctx) = rctx {
                    object.insert("rctx".to_owned(), json!(rctx));
                }
            }
            Message::Schedule { frames } => {
                object.insert(
                    "schedule".to_owned(),
                    Value::Array(frames.iter().map(Broadcast::to_value).collect()),
                );
            }
            Message::Transmitted {
                diid,
                dev_eui,
                rctx,
                xtime,
                txtime,
                gpstime,
            } => {
                object.insert("diid".to_owned(), json!(diid));
                object.insert("DevEui".to_owned(), Value::String(id6(*dev_eui)));
                object.insert("rctx".to_owned(), json!(rctx));
                object.insert("xtime".to_owned(), json!(xtime));
                object.insert("txtime".to_owned(), json!(txtime));
                if let Some(gpstime) = gpstime {
                    object.insert("gpstime".to_owned(), json!(gpstime));
                }
            }
            Message::TimeSync {
                txtime,
                xtime,
                gpstime,
            } => {
                if let Some(txtime) = txtime {
                    object.insert("txtime".to_owned(), json!(txtime));
                }
                if let Some(xtime) = xtime {
                    object.insert("xtime".to_owned(), json!(xtime));
                }
                if let Some(gpstime) = gpstime {
                    object.insert("gpstime".to_owned(), json!(gpstime));
                }
            }
            Message::Other { .. } => {}
        }

        Value::Object(object).to_string()
    }

    /// Reads a message off the websocket.
    ///
    /// A kind this crate does not build is returned as [`Message::Other`] rather than
    /// refused, because a station and a server may each speak more than the other does.
    ///
    /// # Arguments
    ///
    /// * `body` - the message text.
    ///
    /// # Returns
    ///
    /// What arrived.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the body is not a JSON object, carries no
    /// message kind, or is missing an identifier the kind it names requires.
    pub fn from_json(body: &[u8]) -> Result<Message, ProtocolError> {
        let value: Value = serde_json::from_slice(body)
            .map_err(|error| refused(&format!("the message is not JSON: {error}")))?;
        let object = value
            .as_object()
            .ok_or_else(|| refused("the message is not a JSON object"))?;
        let msgtype =
            text(object, "msgtype").ok_or_else(|| refused("the message names no kind"))?;

        Ok(match msgtype.as_str() {
            "version" => Message::Version {
                station: text(object, "station").unwrap_or_default(),
                firmware: text(object, "firmware").unwrap_or_default(),
                package: text(object, "package").unwrap_or_default(),
                model: text(object, "model").unwrap_or_default(),
                protocol: whole(object, "protocol").unwrap_or(0) as u32,
                features: text(object, "features").unwrap_or_default(),
            },
            "router_config" => Message::RouterConfig {
                net_id: numbers(object, "NetID")
                    .into_iter()
                    .map(|value| value as u32)
                    .collect(),
                join_eui: pairs(object, "JoinEui"),
                region: text(object, "region").unwrap_or_default(),
                max_eirp: number(object, "max_eirp").unwrap_or(0.0),
                hwspec: text(object, "hwspec").unwrap_or_default(),
                freq_range: {
                    let range = numbers(object, "freq_range");
                    (
                        range.first().copied().unwrap_or(0) as u32,
                        range.get(1).copied().unwrap_or(0) as u32,
                    )
                },
                data_rates: data_rates(object),
            },
            "jreq" => Message::JoinRequest {
                // Implementations differ on the spelling of this one field, so both are read.
                mhdr: whole(object, "MHdr")
                    .or_else(|| whole(object, "Mhdr"))
                    .unwrap_or(0) as u8,
                join_eui: identifier(object, "JoinEui")?,
                dev_eui: identifier(object, "DevEui")?,
                dev_nonce: whole(object, "DevNonce").unwrap_or(0) as u16,
                mic: whole(object, "MIC").unwrap_or(0) as i32,
                data_rate: whole(object, "DR").unwrap_or(0) as u8,
                frequency_hz: whole(object, "Freq").unwrap_or(0) as u32,
                levels: Levels::from_value(object.get("upinfo")),
            },
            "updf" => {
                let port = whole(object, "FPort").unwrap_or(-1);
                Message::Uplink {
                    mhdr: whole(object, "MHdr")
                        .or_else(|| whole(object, "Mhdr"))
                        .unwrap_or(0) as u8,
                    dev_addr: whole(object, "DevAddr").unwrap_or(0) as i32,
                    fctrl: whole(object, "FCtrl").unwrap_or(0) as u8,
                    fcnt: whole(object, "FCnt").unwrap_or(0) as u16,
                    fopts: bytes_of(object, "FOpts")?,
                    fport: (port >= 0).then_some(port as u8),
                    payload: bytes_of(object, "FRMPayload")?,
                    mic: whole(object, "MIC").unwrap_or(0) as i32,
                    data_rate: whole(object, "DR").unwrap_or(0) as u8,
                    frequency_hz: whole(object, "Freq").unwrap_or(0) as u32,
                    levels: Levels::from_value(object.get("upinfo")),
                }
            }
            "propdf" => Message::Proprietary {
                payload: bytes_of(object, "FRMPayload")?,
                data_rate: whole(object, "DR").unwrap_or(0) as u8,
                frequency_hz: whole(object, "Freq").unwrap_or(0) as u32,
                levels: Levels::from_value(object.get("upinfo")),
            },
            "dnmsg" => Message::Downlink {
                dev_eui: identifier(object, "DevEui")?,
                class: whole(object, "dC").unwrap_or(0) as u8,
                diid: whole(object, "diid").unwrap_or(0),
                pdu: bytes_of(object, "pdu")?,
                rx_delay: whole(object, "RxDelay").map(|value| value as u8),
                rx1: window(object, "RX1DR", "RX1Freq"),
                rx2: window(object, "RX2DR", "RX2Freq"),
                priority: whole(object, "priority").unwrap_or(0) as u8,
                xtime: whole(object, "xtime"),
                rctx: whole(object, "rctx"),
            },
            "dnsched" => {
                let mut frames = Vec::new();
                if let Some(scheduled) = object.get("schedule").and_then(Value::as_array) {
                    for frame in scheduled {
                        frames.push(Broadcast::from_value(frame)?);
                    }
                }
                Message::Schedule { frames }
            }
            "dntxed" => Message::Transmitted {
                diid: whole(object, "diid").unwrap_or(0),
                dev_eui: identifier(object, "DevEui")?,
                rctx: whole(object, "rctx").unwrap_or(0),
                xtime: whole(object, "xtime").unwrap_or(0),
                txtime: number(object, "txtime").unwrap_or(0.0),
                gpstime: whole(object, "gpstime"),
            },
            "timesync" => Message::TimeSync {
                txtime: whole(object, "txtime"),
                xtime: whole(object, "xtime"),
                gpstime: whole(object, "gpstime"),
            },
            other => Message::Other {
                msgtype: other.to_owned(),
            },
        })
    }
}

impl Levels {
    /// Writes the levels as an uplink carries them.
    fn to_value(self) -> Value {
        let mut object = Map::new();
        object.insert("rctx".to_owned(), json!(self.rctx));
        object.insert("xtime".to_owned(), json!(self.xtime));
        if let Some(gpstime) = self.gpstime {
            object.insert("gpstime".to_owned(), json!(gpstime));
        }
        object.insert("rssi".to_owned(), json!(self.rssi));
        object.insert("snr".to_owned(), json!(self.snr));
        Value::Object(object)
    }

    /// Reads the levels beside an uplink, which a station always sends.
    fn from_value(value: Option<&Value>) -> Levels {
        let Some(object) = value.and_then(Value::as_object) else {
            return Levels::default();
        };
        Levels {
            rctx: whole(object, "rctx").unwrap_or(0),
            xtime: whole(object, "xtime").unwrap_or(0),
            gpstime: whole(object, "gpstime"),
            rssi: number(object, "rssi").unwrap_or(0.0),
            snr: number(object, "snr").unwrap_or(0.0),
        }
    }
}

/// Writes bytes as the lowercase hexadecimal the protocol carries them in.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Reads a hexadecimal field, which may be absent or empty.
fn bytes_of(object: &Map<String, Value>, key: &str) -> Result<Vec<u8>, ProtocolError> {
    let Some(written) = text(object, key) else {
        return Ok(Vec::new());
    };
    if written.len() % 2 != 0 {
        return Err(refused(&format!("{key} is not whole bytes")));
    }
    (0..written.len())
        .step_by(2)
        .map(|at| {
            u8::from_str_radix(&written[at..at + 2], 16)
                .map_err(|_| refused(&format!("{key} is not hexadecimal")))
        })
        .collect()
}

/// Reads an identifier field in any form the protocol writes one.
fn identifier(object: &Map<String, Value>, key: &str) -> Result<Eui, ProtocolError> {
    text(object, key)
        .as_deref()
        .and_then(eui_of)
        .ok_or_else(|| refused(&format!("{key} is not an identifier")))
}

/// Reads a receive window, which is a data rate and a frequency together or not at all.
fn window(object: &Map<String, Value>, rate: &str, frequency: &str) -> Option<(u8, u32)> {
    let rate = whole(object, rate)?;
    let frequency = whole(object, frequency)?;
    Some((rate as u8, frequency as u32))
}

/// Reads the data rates, each a spreading factor, a bandwidth in hertz, and whether it is
/// downlink only. The protocol writes the bandwidth in kilohertz.
fn data_rates(object: &Map<String, Value>) -> Vec<(u8, u32, bool)> {
    let Some(Value::Array(entries)) = object.get("DRs") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let triple = entry.as_array()?;
            let spreading = triple.first()?.as_f64()? as u8;
            let bandwidth = triple.get(1)?.as_f64()? as u32;
            let down = triple.get(2).and_then(Value::as_f64).unwrap_or(0.0) != 0.0;
            Some((spreading, bandwidth * 1_000, down))
        })
        .collect()
}

/// Reads an array of numbers.
fn numbers(object: &Map<String, Value>, key: &str) -> Vec<i64> {
    let Some(Value::Array(entries)) = object.get(key) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| entry.as_f64().map(|value| value.round() as i64))
        .collect()
}

/// Reads an array of inclusive pairs, as the admitted join identifiers are written.
fn pairs(object: &Map<String, Value>, key: &str) -> Vec<(u64, u64)> {
    let Some(Value::Array(entries)) = object.get(key) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let begin = pair.first()?.as_f64()? as u64;
            let end = pair.get(1)?.as_f64()? as u64;
            Some((begin, end))
        })
        .collect()
}

/// Reads a text field.
fn text(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Reads a number field.
fn number(object: &Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key).and_then(Value::as_f64)
}

/// Reads a whole number field.
fn whole(object: &Map<String, Value>, key: &str) -> Option<i64> {
    number(object, key).map(|value| value.round() as i64)
}

/// Reads an identifier that arrived least significant byte first.
fn least_first(bytes: &[u8]) -> Eui {
    let mut eui = [0u8; 8];
    for (index, byte) in bytes.iter().rev().enumerate() {
        eui[index] = *byte;
    }
    Eui::new(eui)
}

/// Refuses a message, saying what is wrong with it.
fn refused(why: &str) -> ProtocolError {
    ProtocolError::Payload(why.to_owned())
}

/// A live station, which reaches its network server over a websocket.
///
/// The protocol above is data; this is the part that opens a socket. It asks the discovery
/// endpoint where the server is, opens the websocket it names, says what it is, and reads the
/// configuration it is given. After that the two exchange [`Message`] values.
///
/// Available with the `station-client` feature, which is off by default so the message layer
/// costs nothing.
#[cfg(feature = "station-client")]
pub struct Station {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    config: Message,
}

#[cfg(feature = "station-client")]
impl core::fmt::Debug for Station {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Station")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "station-client")]
impl Station {
    /// Finds the network server for a station and opens its websocket.
    ///
    /// Discovery is a short-lived websocket of its own: the station asks on
    /// [`DISCOVERY_PATH`], is told where its session runs, and opens that instead.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - the address a station is configured with, as `ws://host:port`.
    /// * `router` - the station asking.
    ///
    /// # Returns
    ///
    /// The open station, whose [`config`](Station::config) holds what the server answered
    /// with.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the endpoint refuses the station, answers
    /// something that is not this protocol, or cannot be reached.
    pub async fn connect(endpoint: &str, router: Eui) -> Result<Station, ProtocolError> {
        let answer = discover(endpoint, router).await?;
        if let Some(error) = answer.error {
            return Err(refused(&format!(
                "the server refused this station: {error}"
            )));
        }
        let uri = answer
            .uri
            .ok_or_else(|| refused("the server named no websocket to open"))?;

        let (mut socket, _) = tokio_tungstenite::connect_async(uri.as_str())
            .await
            .map_err(|error| refused(&format!("the websocket did not open: {error}")))?;

        // The station speaks first, and the server answers with how to configure the radios.
        let version = Message::Version {
            station: "pamoja".to_owned(),
            firmware: env!("CARGO_PKG_VERSION").to_owned(),
            package: "pamoja-gateway".to_owned(),
            model: "pamoja".to_owned(),
            protocol: PROTOCOL_VERSION,
            features: String::new(),
        };
        send(&mut socket, &version).await?;
        let config = receive(&mut socket).await?;

        Ok(Station { socket, config })
    }

    /// Returns what the server said when the session opened.
    ///
    /// # Returns
    ///
    /// The `router_config` message, which names the region, the channel plan and the limits.
    pub const fn config(&self) -> &Message {
        &self.config
    }

    /// Sends a message to the network server.
    ///
    /// # Arguments
    ///
    /// * `message` - what to send.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the websocket has taken it.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the websocket fails.
    pub async fn send(&mut self, message: &Message) -> Result<(), ProtocolError> {
        send(&mut self.socket, message).await
    }

    /// Waits for the next message from the network server.
    ///
    /// # Returns
    ///
    /// What arrived.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the websocket closes or carries something that
    /// is not this protocol.
    pub async fn recv(&mut self) -> Result<Message, ProtocolError> {
        receive(&mut self.socket).await
    }
}

/// Asks the discovery endpoint where the network server for a station is.
#[cfg(feature = "station-client")]
async fn discover(endpoint: &str, router: Eui) -> Result<Router, ProtocolError> {
    use futures_util::{SinkExt, StreamExt};

    let (mut socket, _) = tokio_tungstenite::connect_async(asking(endpoint))
        .await
        .map_err(|error| refused(&format!("the discovery endpoint is not reachable: {error}")))?;

    socket
        .send(tokio_tungstenite::tungstenite::Message::text(
            Discovery::new(router).to_json(),
        ))
        .await
        .map_err(|error| refused(&format!("the discovery request failed: {error}")))?;

    let answer = socket
        .next()
        .await
        .ok_or_else(|| refused("the discovery endpoint answered nothing"))?
        .map_err(|error| refused(&format!("the discovery answer failed: {error}")))?;
    let answer = Router::from_json(
        answer
            .to_text()
            .map_err(|_| refused("the discovery answer is not text"))?
            .as_bytes(),
    )?;

    // The session runs on a socket of its own, so this one is finished with.
    let _ = socket.close(None).await;
    Ok(answer)
}

/// Builds the discovery address from the endpoint a station is configured with.
#[cfg(feature = "station-client")]
fn asking(endpoint: &str) -> String {
    let endpoint = endpoint.trim_end_matches("/");
    // A station is configured with a websocket address; the plain forms are read as one.
    let endpoint = match endpoint.split_once("://") {
        Some(("http", rest)) => format!("ws://{rest}"),
        Some(("https", rest)) => format!("wss://{rest}"),
        Some(_) => endpoint.to_owned(),
        None => format!("ws://{endpoint}"),
    };
    format!("{endpoint}{DISCOVERY_PATH}")
}

/// Writes one message onto the websocket.
#[cfg(feature = "station-client")]
async fn send<S>(socket: &mut S, message: &Message) -> Result<(), ProtocolError>
where
    S: futures_util::Sink<tokio_tungstenite::tungstenite::Message> + Unpin,
    S::Error: core::fmt::Display,
{
    use futures_util::SinkExt;

    socket
        .send(tokio_tungstenite::tungstenite::Message::text(
            message.to_json(),
        ))
        .await
        .map_err(|error| refused(&format!("the websocket would not take a message: {error}")))
}

/// Reads the next message the websocket carries, skipping the frames that are not one.
#[cfg(feature = "station-client")]
async fn receive<S>(socket: &mut S) -> Result<Message, ProtocolError>
where
    S: futures_util::Stream<
            Item = Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        > + Unpin,
{
    use futures_util::StreamExt;

    loop {
        let frame = socket
            .next()
            .await
            .ok_or_else(|| refused("the websocket closed"))?
            .map_err(|error| refused(&format!("the websocket failed: {error}")))?;

        // Keepalives and the close frame are the websocket talking, not the protocol.
        match frame.to_text() {
            Ok(text) if !text.is_empty() => return Message::from_json(text.as_bytes()),
            _ => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The three examples the protocol glossary prints, which are the only concrete
    // identifiers it publishes.
    #[test]
    fn the_published_identifier_examples_read_and_write() {
        let zero = Eui::new([0; 8]);
        assert_eq!(id6(zero), "::");
        assert_eq!(eui_of("::0"), Some(zero));

        let one_high = Eui::new([0x00, 0x01, 0, 0, 0, 0, 0, 0]);
        assert_eq!(eui_of("1::"), Some(one_high));
        assert_eq!(id6(one_high), "1::");

        let low_pair = Eui::new([0, 0, 0, 0, 0, 0x0a, 0, 0x0b]);
        assert_eq!(eui_of("::a:b"), Some(low_pair));
        assert_eq!(id6(low_pair), "::a:b");
    }

    #[test]
    fn an_identifier_reads_in_every_form_the_protocol_accepts() {
        let eui = Eui::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(eui_of("102:304:506:708"), Some(eui));
        assert_eq!(eui_of("01-02-03-04-05-06-07-08"), Some(eui));
        assert_eq!(
            eui_of(&u64::from_be_bytes(eui.bytes()).to_string()),
            Some(eui)
        );
        assert_eq!(eui_of("not an identifier"), None);
        assert_eq!(eui_of("1:2:3"), None);
    }

    #[test]
    fn discovery_asks_and_is_answered() {
        let station = Eui::from_hex("b827ebfffe010203").expect("sixteen digits");
        let asking = Discovery::new(station);
        let read = Discovery::from_json(asking.to_json().as_bytes()).expect("it round trips");
        assert_eq!(read.router, station);

        let muxs = Eui::new([0, 0, 0, 0, 0, 0, 0, 1]);
        let answer = Router::accepted(station, muxs, "ws://lns.example.invalid:3001/router");
        let read = Router::from_json(answer.to_json().as_bytes()).expect("it round trips");
        assert_eq!(read.router, Some(station));
        assert_eq!(read.muxs, Some(muxs));
        assert_eq!(
            read.uri.as_deref(),
            Some("ws://lns.example.invalid:3001/router")
        );
        assert!(read.error.is_none());
    }

    #[test]
    fn a_refused_station_is_told_why() {
        let station = Eui::from_hex("b827ebfffe010203").expect("sixteen digits");
        let refusal = Router::refused(station, "this gateway is not registered");
        let read = Router::from_json(refusal.to_json().as_bytes()).expect("it round trips");
        assert_eq!(
            read.error.as_deref(),
            Some("this gateway is not registered")
        );
        assert!(read.uri.is_none());
    }

    // The protocol publishes field definitions rather than populated examples, so these
    // hold the documented field names and check that what is written reads back.
    #[test]
    fn every_message_kind_survives_a_round_trip() {
        let device = Eui::from_hex("1111111111111111").expect("sixteen digits");
        let application = Eui::from_hex("2222222222222222").expect("sixteen digits");
        let levels = Levels {
            rctx: 1,
            xtime: 3_512_348_611,
            gpstime: Some(1_364_746_877),
            rssi: -35.0,
            snr: 5.1,
        };

        let kinds = [
            Message::Version {
                station: "pamoja".to_owned(),
                firmware: "0.1.18".to_owned(),
                package: "pamoja-gateway".to_owned(),
                model: "linux".to_owned(),
                protocol: PROTOCOL_VERSION,
                features: "gps".to_owned(),
            },
            Message::RouterConfig {
                net_id: vec![0],
                join_eui: vec![(0, u64::MAX)],
                region: "EU863".to_owned(),
                max_eirp: 16.0,
                hwspec: "sx1301/1".to_owned(),
                freq_range: (863_000_000, 870_000_000),
                data_rates: vec![(12, 125_000, false), (7, 250_000, false), (0, 0, true)],
            },
            Message::JoinRequest {
                mhdr: 0x00,
                join_eui: application,
                dev_eui: device,
                dev_nonce: 0x0102,
                mic: -12345,
                data_rate: 5,
                frequency_hz: 868_100_000,
                levels,
            },
            Message::Uplink {
                mhdr: 0x40,
                dev_addr: 0x2601_0001,
                fctrl: 0,
                fcnt: 7,
                fopts: vec![0x03, 0x07],
                fport: Some(2),
                payload: b"21.5".to_vec(),
                mic: 987,
                data_rate: 5,
                frequency_hz: 868_100_000,
                levels,
            },
            Message::Downlink {
                dev_eui: device,
                class: 0,
                diid: 42,
                pdu: vec![0x60, 0x01, 0x02],
                rx_delay: Some(1),
                rx1: Some((5, 868_100_000)),
                rx2: Some((0, 869_525_000)),
                priority: 10,
                xtime: Some(3_513_348_611),
                rctx: Some(1),
            },
            Message::Transmitted {
                diid: 42,
                dev_eui: device,
                rctx: 1,
                xtime: 3_513_348_611,
                txtime: 1.5,
                gpstime: Some(1_364_746_878),
            },
            Message::TimeSync {
                txtime: Some(1_000),
                xtime: None,
                gpstime: Some(1_364_746_879),
            },
            Message::Proprietary {
                payload: vec![0xe0, 0x01, 0x02, 0x03],
                data_rate: 5,
                frequency_hz: 868_100_000,
                levels,
            },
            Message::Schedule {
                frames: vec![Broadcast {
                    pdu: vec![0x60, 0x01, 0x02],
                    data_rate: 3,
                    frequency_hz: 869_525_000,
                    priority: 1,
                    gpstime: Some(1_364_746_880),
                    rctx: Some(0),
                }],
            },
        ];

        for sent in kinds {
            let written = sent.to_json();
            let read = Message::from_json(written.as_bytes()).expect("it is well formed");
            assert_eq!(
                read,
                sent,
                "{} did not survive the round trip",
                sent.msgtype()
            );
            assert!(
                written.contains(&format!("\"msgtype\":\"{}\"", sent.msgtype())),
                "a message names its kind"
            );
        }
    }

    #[test]
    fn a_frame_carrying_only_options_has_no_port() {
        let quiet = Message::Uplink {
            mhdr: 0x40,
            dev_addr: 1,
            fctrl: 0,
            fcnt: 0,
            fopts: vec![0x02],
            fport: None,
            payload: Vec::new(),
            mic: 0,
            data_rate: 5,
            frequency_hz: 868_100_000,
            levels: Levels::default(),
        };
        let written = quiet.to_json();
        // The protocol writes a missing port as -1 rather than leaving the field out.
        assert!(written.contains("\"FPort\":-1"), "{written}");
        let read = Message::from_json(written.as_bytes()).expect("it is well formed");
        assert_eq!(read, quiet);
    }

    #[test]
    fn a_kind_this_crate_does_not_build_is_reported_not_refused() {
        let read = Message::from_json(br#"{"msgtype":"rmtsh","user":"root"}"#)
            .expect("an unknown kind is not an error");
        assert_eq!(
            read,
            Message::Other {
                msgtype: "rmtsh".to_owned()
            }
        );
    }

    #[test]
    fn a_message_that_is_not_one_is_refused() {
        assert!(Message::from_json(b"not json").is_err());
        assert!(Message::from_json(br#"{"station":"pamoja"}"#).is_err());
        assert!(Message::from_json(br#"{"msgtype":"jreq","DevEui":"nonsense"}"#).is_err());
        assert!(Message::from_json(br#"{"msgtype":"dnmsg","DevEui":"::1","pdu":"abc"}"#).is_err());
    }

    // A station splits frames it did not build, so these are built by the device code a
    // real node runs and split back by the station side.
    #[cfg(feature = "network")]
    #[test]
    fn a_join_request_a_device_built_reads_back_field_for_field() {
        use pamoja_lorawan::Device;

        let dev_eui = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let join_eui = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let request = Device::new(dev_eui, join_eui, [0x2b; 16]).join_request(0x0102);

        let heard = Message::heard(request.as_bytes(), 5, 868_100_000, Levels::default())
            .expect("a station sends a join request up");
        match heard {
            Message::JoinRequest {
                mhdr,
                join_eui: application,
                dev_eui: node,
                dev_nonce,
                ..
            } => {
                assert_eq!(mhdr, 0x00);
                assert_eq!(application, Eui::new(join_eui));
                assert_eq!(node, Eui::new(dev_eui));
                assert_eq!(dev_nonce, 0x0102);
            }
            other => panic!("that is a join request, not {}", other.msgtype()),
        }
    }

    #[cfg(feature = "network")]
    #[test]
    fn an_encrypted_frame_reads_back_with_its_address_port_and_counter() {
        use pamoja_lorawan::{Session, Uplink as LorawanUplink};

        let session = Session::new(0x2601_0001, [0x11; 16], [0x22; 16]);
        let frame = session
            .encode_uplink(&LorawanUplink::new(7, 2, b"21.5"))
            .expect("it fits one frame");

        let heard = Message::heard(frame.as_bytes(), 5, 868_100_000, Levels::default())
            .expect("a station sends a data frame up");
        match heard {
            Message::Uplink {
                dev_addr,
                fcnt,
                fport,
                fopts,
                payload,
                ..
            } => {
                assert_eq!(dev_addr, 0x2601_0001);
                assert_eq!(fcnt, 7);
                assert_eq!(fport, Some(2));
                assert!(fopts.is_empty());
                // The payload stays encrypted: a station holds no session key.
                assert_eq!(payload.len(), 4);
                assert_ne!(payload, b"21.5".to_vec());
            }
            other => panic!("that is a data frame, not {}", other.msgtype()),
        }
    }

    #[test]
    fn a_frame_of_options_alone_carries_no_port() {
        // An address, a control byte naming two bytes of options, a counter, the options,
        // and the integrity code. Nothing follows the options, so there is no port.
        let frame = [
            0x40, 0x01, 0x00, 0x00, 0x00, 0x02, 0x05, 0x00, 0x03, 0x07, 0x11, 0x22, 0x33, 0x44,
        ];
        let heard = Message::heard(&frame, 5, 868_100_000, Levels::default())
            .expect("a station sends it up");
        match heard {
            Message::Uplink {
                dev_addr,
                fcnt,
                fopts,
                fport,
                payload,
                ..
            } => {
                assert_eq!(dev_addr, 1);
                assert_eq!(fcnt, 5);
                assert_eq!(fopts, vec![0x03, 0x07]);
                assert_eq!(fport, None);
                assert!(payload.is_empty());
            }
            other => panic!("that is a data frame, not {}", other.msgtype()),
        }
    }

    #[test]
    fn a_proprietary_frame_is_passed_along_whole() {
        let frame = [0xe0, 0x01, 0x02, 0x03, 0x04, 0x05];
        let heard = Message::heard(&frame, 5, 868_100_000, Levels::default())
            .expect("a station sends it up");
        match heard {
            Message::Proprietary { payload, .. } => assert_eq!(payload, frame.to_vec()),
            other => panic!("that one is proprietary, not {}", other.msgtype()),
        }
    }

    #[test]
    fn a_frame_a_station_does_not_send_up_is_refused() {
        let levels = Levels::default();
        // Shorter than a header and an integrity code.
        assert!(Message::heard(&[0x40, 0x00], 5, 868_100_000, levels).is_err());
        // A downlink, which a station transmits rather than reports.
        let downlink = [0x60, 1, 0, 0, 0, 0, 0, 0, 0, 0x11, 0x22, 0x33, 0x44];
        assert!(Message::heard(&downlink, 5, 868_100_000, levels).is_err());
        // A frame naming more bytes of options than it carries.
        let overrun = [0x40, 1, 0, 0, 0, 0x0f, 0, 0, 0x11, 0x22, 0x33, 0x44];
        assert!(Message::heard(&overrun, 5, 868_100_000, levels).is_err());
    }

    #[test]
    fn what_is_not_this_protocol_is_refused() {
        assert!(Discovery::from_json(b"not json").is_err());
        assert!(Discovery::from_json(br#"{"router":"not an identifier"}"#).is_err());
    }
}

#[cfg(all(test, feature = "station-client"))]
mod live {
    use super::*;

    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

    /// The station used throughout.
    fn router() -> Eui {
        Eui::from_hex("0102030405060708").expect("sixteen digits")
    }

    /// The server endpoint that carries the session.
    fn muxs() -> Eui {
        Eui::from_hex("0000000000000000").expect("sixteen digits")
    }

    /// Serves one discovery request from a fresh port and answers it with `answer`.
    ///
    /// Returns the endpoint to ask and a handle that yields the path the station asked on
    /// and what it asked.
    // The error a handshake callback returns is a type the websocket crate sizes.
    #[allow(clippy::result_large_err)]
    async fn endpoint(answer: String) -> (String, tokio::task::JoinHandle<(String, String)>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let address = listener.local_addr().expect("a bound port");

        let serving = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("the station asks");

            let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
            let noted = std::sync::Arc::clone(&seen);
            let mut socket = tokio_tungstenite::accept_hdr_async(
                stream,
                move |request: &Request, response: Response| {
                    noted
                        .lock()
                        .expect("the path is recorded once")
                        .push_str(request.uri().path());
                    Ok(response)
                },
            )
            .await
            .expect("the websocket opens");
            let path = seen.lock().expect("the path was recorded").clone();

            let asked = socket
                .next()
                .await
                .expect("the station asks")
                .expect("it is a frame");
            let asked = asked.to_text().expect("it is text").to_owned();

            socket
                .send(tokio_tungstenite::tungstenite::Message::text(answer))
                .await
                .expect("the answer is sent");
            let _ = socket.close(None).await;

            (path, asked)
        });

        (format!("ws://{address}"), serving)
    }

    #[tokio::test]
    async fn a_station_asks_where_its_network_server_is() {
        let uri = "ws://127.0.0.1:6038/router";
        let (where_to_ask, serving) =
            endpoint(Router::accepted(router(), muxs(), uri).to_json()).await;

        let answer = discover(&where_to_ask, router())
            .await
            .expect("the endpoint answers");
        assert_eq!(answer.uri.as_deref(), Some(uri));

        let (path, asked) = serving.await.expect("the endpoint served");
        assert_eq!(path, DISCOVERY_PATH);
        // The station names itself in the form the protocol writes.
        assert_eq!(asked, r#"{"router":"102:304:506:708"}"#);
    }

    #[tokio::test]
    async fn a_station_the_server_will_not_have_is_told_why() {
        let (where_to_ask, serving) =
            endpoint(Router::refused(router(), "unknown gateway").to_json()).await;

        let refusal = Station::connect(&where_to_ask, router())
            .await
            .expect_err("a refused station does not connect");
        assert!(
            format!("{refusal}").contains("unknown gateway"),
            "{refusal}"
        );

        serving.await.expect("the endpoint served");
    }

    #[tokio::test]
    async fn a_station_says_what_it_is_and_is_given_a_configuration() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let address = listener.local_addr().expect("a bound port");

        let configuration = Message::RouterConfig {
            net_id: vec![0],
            join_eui: vec![(0, u64::MAX)],
            region: "EU863".to_owned(),
            max_eirp: 16.0,
            hwspec: "sx1301/1".to_owned(),
            freq_range: (863_000_000, 870_000_000),
            data_rates: vec![(12, 125_000, false), (7, 250_000, false)],
        };
        let sent_down = Message::Downlink {
            dev_eui: Eui::from_hex("1111111111111111").expect("sixteen digits"),
            class: 0,
            diid: 7,
            pdu: vec![0x60, 0x01],
            rx_delay: Some(1),
            rx1: Some((5, 868_100_000)),
            rx2: None,
            priority: 0,
            xtime: Some(3_513_348_611),
            rctx: Some(1),
        };

        let answering = tokio::spawn({
            let configuration = configuration.clone();
            let sent_down = sent_down.clone();
            async move {
                let (stream, _) = listener.accept().await.expect("the station connects");
                let mut socket = tokio_tungstenite::accept_async(stream)
                    .await
                    .expect("the websocket opens");

                // A station speaks first, and what it says is how it identifies itself.
                let opening = socket
                    .next()
                    .await
                    .expect("the station speaks")
                    .expect("it is a frame");
                let opening = Message::from_json(opening.to_text().expect("it is text").as_bytes())
                    .expect("it is this protocol");

                socket
                    .send(tokio_tungstenite::tungstenite::Message::text(
                        configuration.to_json(),
                    ))
                    .await
                    .expect("the configuration is sent");

                let carried = socket
                    .next()
                    .await
                    .expect("the station reports")
                    .expect("it is a frame");
                let carried = Message::from_json(carried.to_text().expect("it is text").as_bytes())
                    .expect("it is this protocol");

                socket
                    .send(tokio_tungstenite::tungstenite::Message::text(
                        sent_down.to_json(),
                    ))
                    .await
                    .expect("the downlink is sent");

                (opening, carried)
            }
        });

        let (where_to_ask, serving) = endpoint(
            Router::accepted(router(), muxs(), format!("ws://{address}/router")).to_json(),
        )
        .await;

        let mut station = Station::connect(&where_to_ask, router())
            .await
            .expect("the station connects");
        assert_eq!(station.config(), &configuration);

        let reported = Message::Uplink {
            mhdr: 0x40,
            dev_addr: 0x2601_0001,
            fctrl: 0,
            fcnt: 1,
            fopts: Vec::new(),
            fport: Some(2),
            payload: b"21.5".to_vec(),
            mic: 987,
            data_rate: 5,
            frequency_hz: 868_100_000,
            levels: Levels::default(),
        };
        station.send(&reported).await.expect("the uplink is sent");
        assert_eq!(station.recv().await.expect("the server answers"), sent_down);

        serving.await.expect("the endpoint served");
        let (opening, carried) = answering.await.expect("the server served");
        assert_eq!(carried, reported);
        match opening {
            Message::Version {
                station, protocol, ..
            } => {
                assert_eq!(station, "pamoja");
                assert_eq!(protocol, PROTOCOL_VERSION);
            }
            other => panic!("a station opens with its version, not {}", other.msgtype()),
        }
    }
}
