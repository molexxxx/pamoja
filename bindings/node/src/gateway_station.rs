//! Generated Node bindings for the LoRa Basics Station protocol.
//!
//! These mirror the `pamoja_gateway::station` Rust API: the messages a station and its network
//! server exchange over a websocket, and the frame split a station does before it reports what
//! it heard. Nothing here opens a socket, so a program brings its own websocket, sends what
//! {@link stationEncode} writes, and reads whatever arrives with {@link stationParse}.
//!
//! Frequencies are in hertz and payloads are buffers rather than hexadecimal text, so nothing
//! has to be formatted by hand.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_gateway::station::{eui_of, id6, Broadcast, Discovery, Levels, Message, Router};
use pamoja_gateway::udp::Eui;

/// Which kind of message this is.
#[napi(string_enum, js_name = "GatewayStationKind")]
pub enum GatewayStationKind {
    /// What the station reports about itself when a session opens.
    Version,
    /// How the server tells the station to configure its radios.
    RouterConfig,
    /// A join request the station heard.
    JoinRequest,
    /// A data frame the station heard.
    Uplink,
    /// A frame of a kind this protocol does not describe, carried whole.
    Proprietary,
    /// A frame the server asks the station to transmit.
    Downlink,
    /// Frames the server asks the station to transmit to a group.
    Schedule,
    /// What became of a frame the station was asked to transmit.
    Transmitted,
    /// The clock the two keep between them.
    TimeSync,
    /// A kind this build does not model, readable only as its text.
    Other,
}

/// How a station heard a packet, as it reports it.
#[napi(object, js_name = "GatewayStationLevels")]
pub struct GatewayStationLevels {
    /// The radio the packet arrived on, which an answer goes back out on.
    pub rctx: i64,
    /// The station clock, in microseconds.
    pub xtime: i64,
    /// The GPS time, when the station has one.
    pub gpstime: Option<i64>,
    /// The received signal strength, in dBm.
    pub rssi: f64,
    /// The signal-to-noise ratio, in dB.
    pub snr: f64,
}

/// One frame of a schedule, transmitted to a group rather than a device.
#[napi(object, js_name = "GatewayStationBroadcast")]
pub struct GatewayStationBroadcast {
    /// The frame to transmit.
    pub pdu: Buffer,
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

/// A message either side of a session sends.
///
/// Every message names its kind, and carries the fields that kind uses.
#[napi(object, js_name = "GatewayStationMessage")]
pub struct GatewayStationMessage {
    /// Which kind of message.
    pub kind: GatewayStationKind,
    /// The kind as the protocol writes it, such as `jreq`, for a kind this build does not
    /// model.
    pub msgtype: Option<String>,
    /// The station software, for a version.
    pub station: Option<String>,
    /// Its firmware, for a version.
    pub firmware: Option<String>,
    /// The package it came from, for a version.
    pub package: Option<String>,
    /// The hardware model, for a version.
    pub model: Option<String>,
    /// The protocol version it speaks, for a version.
    pub protocol: Option<u32>,
    /// What it can do, for a version.
    pub features: Option<String>,
    /// The networks whose frames are carried, for a configuration.
    pub net_id: Option<Vec<u32>>,
    /// The region name, for a configuration.
    pub region: Option<String>,
    /// The highest radiated power the region allows, in dBm, for a configuration.
    pub max_eirp: Option<f64>,
    /// The concentrator the configuration is written for.
    pub hwspec: Option<String>,
    /// The lowest frequency the station may use, in hertz, for a configuration.
    pub freq_min: Option<u32>,
    /// The highest frequency the station may use, in hertz, for a configuration.
    pub freq_max: Option<u32>,
    /// The MAC header byte, for a join request or a data frame.
    pub mhdr: Option<u8>,
    /// The application being joined, as sixteen hexadecimal digits.
    pub join_eui: Option<String>,
    /// The device, as sixteen hexadecimal digits.
    pub dev_eui: Option<String>,
    /// The nonce a join request used.
    pub dev_nonce: Option<u16>,
    /// The address a data frame came from.
    pub dev_addr: Option<i32>,
    /// The frame control byte.
    pub fctrl: Option<u8>,
    /// The frame counter, as the sixteen bits on the air.
    pub fcnt: Option<u16>,
    /// The frame options, for a data frame.
    pub fopts: Option<Buffer>,
    /// The port, absent for a frame carrying only options.
    pub fport: Option<u8>,
    /// The payload, still encrypted, or the whole frame for a proprietary one.
    pub payload: Option<Buffer>,
    /// The message integrity code.
    pub mic: Option<i32>,
    /// The data rate it arrived at, or is to be sent at.
    pub data_rate: Option<u8>,
    /// The frequency in hertz.
    pub frequency_hz: Option<u32>,
    /// How it was heard, for the kinds a station sends up.
    pub levels: Option<GatewayStationLevels>,
    /// Which class of downlink this is.
    pub class: Option<u8>,
    /// The identifier a transmission report carries back.
    pub diid: Option<i64>,
    /// The frame to transmit, for a downlink.
    pub pdu: Option<Buffer>,
    /// The delay before the first receive window, in seconds.
    pub rx_delay: Option<u8>,
    /// How urgent a downlink is.
    pub priority: Option<u8>,
    /// The station clock, for a downlink or a report.
    pub xtime: Option<i64>,
    /// The radio, for a downlink or a report.
    pub rctx: Option<i64>,
    /// When a frame went out, in seconds.
    pub txtime: Option<f64>,
    /// The GPS time, when the station has one.
    pub gpstime: Option<i64>,
    /// What to transmit to a group, for a schedule.
    pub schedule: Option<Vec<GatewayStationBroadcast>>,
}

/// The answer a discovery endpoint gives.
#[napi(object, js_name = "GatewayStationRouter")]
pub struct GatewayStationRouter {
    /// The station, as the server read it.
    pub router: Option<String>,
    /// The server endpoint carrying the session.
    pub muxs: Option<String>,
    /// The websocket to open, absolute.
    pub uri: Option<String>,
    /// Why the station was refused, when it was.
    pub error: Option<String>,
}

/// Reads a frame the radio heard into the message that reports it.
#[napi(js_name = "stationHeard")]
pub fn station_heard(
    frame: Buffer,
    data_rate: u8,
    frequency_hz: u32,
    levels: GatewayStationLevels,
) -> napi::Result<GatewayStationMessage> {
    let heard = Message::heard(&frame, data_rate, frequency_hz, levels_of(levels))
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(message_to_js(&heard))
}

/// Writes a message as the websocket carries it.
#[napi(js_name = "stationEncode")]
pub fn station_encode(message: GatewayStationMessage) -> napi::Result<String> {
    Ok(message_of(message)?.to_json())
}

/// Reads a message that arrived over the websocket.
#[napi(js_name = "stationParse")]
pub fn station_parse(text: String) -> napi::Result<GatewayStationMessage> {
    let message = Message::from_json(text.as_bytes())
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(message_to_js(&message))
}

/// Writes the request a station sends to find its network server.
#[napi(js_name = "stationDiscovery")]
pub fn station_discovery(router: String) -> napi::Result<String> {
    Ok(Discovery::new(identifier(&router)?).to_json())
}

/// Reads the answer a discovery endpoint gives.
#[napi(js_name = "stationRouterParse")]
pub fn station_router_parse(text: String) -> napi::Result<GatewayStationRouter> {
    let answer = Router::from_json(text.as_bytes())
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(GatewayStationRouter {
        router: answer.router.map(|eui| eui.to_hex()),
        muxs: answer.muxs.map(|eui| eui.to_hex()),
        uri: answer.uri,
        error: answer.error,
    })
}

/// Writes an identifier in the ID6 form the protocol prefers.
#[napi(js_name = "stationId6")]
pub fn station_id6(eui: String) -> napi::Result<String> {
    Ok(id6(identifier(&eui)?))
}

/// Reads an identifier written in any form the protocol accepts.
#[napi(js_name = "stationEuiOf")]
pub fn station_eui_of(text: String) -> napi::Result<String> {
    eui_of(&text)
        .map(|eui| eui.to_hex())
        .ok_or_else(|| napi::Error::from_reason(format!("{text} is not an identifier")))
}

/// Reads an identifier written as sixteen hexadecimal digits.
fn identifier(text: &str) -> napi::Result<Eui> {
    Eui::from_hex(text).ok_or_else(|| {
        napi::Error::from_reason(format!("{text} is not sixteen hexadecimal digits"))
    })
}

/// Reads how a packet was heard.
fn levels_of(levels: GatewayStationLevels) -> Levels {
    Levels {
        rctx: levels.rctx,
        xtime: levels.xtime,
        gpstime: levels.gpstime,
        rssi: levels.rssi,
        snr: levels.snr,
    }
}

/// Writes how a packet was heard.
fn levels_to_js(levels: Levels) -> GatewayStationLevels {
    GatewayStationLevels {
        rctx: levels.rctx,
        xtime: levels.xtime,
        gpstime: levels.gpstime,
        rssi: levels.rssi,
        snr: levels.snr,
    }
}

/// An empty message of the given kind, which each arm then fills in.
fn blank(kind: GatewayStationKind) -> GatewayStationMessage {
    GatewayStationMessage {
        kind,
        msgtype: None,
        station: None,
        firmware: None,
        package: None,
        model: None,
        protocol: None,
        features: None,
        net_id: None,
        region: None,
        max_eirp: None,
        hwspec: None,
        freq_min: None,
        freq_max: None,
        mhdr: None,
        join_eui: None,
        dev_eui: None,
        dev_nonce: None,
        dev_addr: None,
        fctrl: None,
        fcnt: None,
        fopts: None,
        fport: None,
        payload: None,
        mic: None,
        data_rate: None,
        frequency_hz: None,
        levels: None,
        class: None,
        diid: None,
        pdu: None,
        rx_delay: None,
        priority: None,
        xtime: None,
        rctx: None,
        txtime: None,
        gpstime: None,
        schedule: None,
    }
}

/// Writes a message JavaScript reads.
fn message_to_js(message: &Message) -> GatewayStationMessage {
    match message {
        Message::Version {
            station,
            firmware,
            package,
            model,
            protocol,
            features,
        } => {
            let mut held = blank(GatewayStationKind::Version);
            held.station = Some(station.clone());
            held.firmware = Some(firmware.clone());
            held.package = Some(package.clone());
            held.model = Some(model.clone());
            held.protocol = Some(*protocol);
            held.features = Some(features.clone());
            held
        }
        Message::RouterConfig {
            net_id,
            region,
            max_eirp,
            hwspec,
            freq_range,
            ..
        } => {
            let mut held = blank(GatewayStationKind::RouterConfig);
            held.net_id = Some(net_id.clone());
            held.region = Some(region.clone());
            held.max_eirp = Some(*max_eirp);
            held.hwspec = Some(hwspec.clone());
            held.freq_min = Some(freq_range.0);
            held.freq_max = Some(freq_range.1);
            held
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
            let mut held = blank(GatewayStationKind::JoinRequest);
            held.mhdr = Some(*mhdr);
            held.join_eui = Some(join_eui.to_hex());
            held.dev_eui = Some(dev_eui.to_hex());
            held.dev_nonce = Some(*dev_nonce);
            held.mic = Some(*mic);
            held.data_rate = Some(*data_rate);
            held.frequency_hz = Some(*frequency_hz);
            held.levels = Some(levels_to_js(*levels));
            held
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
            let mut held = blank(GatewayStationKind::Uplink);
            held.mhdr = Some(*mhdr);
            held.dev_addr = Some(*dev_addr);
            held.fctrl = Some(*fctrl);
            held.fcnt = Some(*fcnt);
            held.fopts = Some(Buffer::from(fopts.clone()));
            held.fport = *fport;
            held.payload = Some(Buffer::from(payload.clone()));
            held.mic = Some(*mic);
            held.data_rate = Some(*data_rate);
            held.frequency_hz = Some(*frequency_hz);
            held.levels = Some(levels_to_js(*levels));
            held
        }
        Message::Proprietary {
            payload,
            data_rate,
            frequency_hz,
            levels,
        } => {
            let mut held = blank(GatewayStationKind::Proprietary);
            held.payload = Some(Buffer::from(payload.clone()));
            held.data_rate = Some(*data_rate);
            held.frequency_hz = Some(*frequency_hz);
            held.levels = Some(levels_to_js(*levels));
            held
        }
        Message::Downlink {
            dev_eui,
            class,
            diid,
            pdu,
            rx_delay,
            priority,
            xtime,
            rctx,
            ..
        } => {
            let mut held = blank(GatewayStationKind::Downlink);
            held.dev_eui = Some(dev_eui.to_hex());
            held.class = Some(*class);
            held.diid = Some(*diid);
            held.pdu = Some(Buffer::from(pdu.clone()));
            held.rx_delay = *rx_delay;
            held.priority = Some(*priority);
            held.xtime = *xtime;
            held.rctx = *rctx;
            held
        }
        Message::Schedule { frames } => {
            let mut held = blank(GatewayStationKind::Schedule);
            held.schedule = Some(
                frames
                    .iter()
                    .map(|frame| GatewayStationBroadcast {
                        pdu: Buffer::from(frame.pdu.clone()),
                        data_rate: frame.data_rate,
                        frequency_hz: frame.frequency_hz,
                        priority: frame.priority,
                        gpstime: frame.gpstime,
                        rctx: frame.rctx,
                    })
                    .collect(),
            );
            held
        }
        Message::Transmitted {
            diid,
            dev_eui,
            rctx,
            xtime,
            txtime,
            gpstime,
        } => {
            let mut held = blank(GatewayStationKind::Transmitted);
            held.diid = Some(*diid);
            held.dev_eui = Some(dev_eui.to_hex());
            held.rctx = Some(*rctx);
            held.xtime = Some(*xtime);
            held.txtime = Some(*txtime);
            held.gpstime = *gpstime;
            held
        }
        Message::TimeSync {
            txtime,
            xtime,
            gpstime,
        } => {
            let mut held = blank(GatewayStationKind::TimeSync);
            held.txtime = txtime.map(|value| value as f64);
            held.xtime = *xtime;
            held.gpstime = *gpstime;
            held
        }
        Message::Other { msgtype } => {
            let mut held = blank(GatewayStationKind::Other);
            held.msgtype = Some(msgtype.clone());
            held
        }
    }
}

/// Reads a message JavaScript describes.
fn message_of(message: GatewayStationMessage) -> napi::Result<Message> {
    let levels = message.levels.map_or_else(Levels::default, levels_of);
    Ok(match message.kind {
        GatewayStationKind::Version => Message::Version {
            station: message.station.unwrap_or_default(),
            firmware: message.firmware.unwrap_or_default(),
            package: message.package.unwrap_or_default(),
            model: message.model.unwrap_or_default(),
            protocol: message
                .protocol
                .unwrap_or(pamoja_gateway::station::PROTOCOL_VERSION),
            features: message.features.unwrap_or_default(),
        },
        GatewayStationKind::RouterConfig => Message::RouterConfig {
            net_id: message.net_id.unwrap_or_default(),
            join_eui: Vec::new(),
            region: message.region.unwrap_or_default(),
            max_eirp: message.max_eirp.unwrap_or_default(),
            hwspec: message.hwspec.unwrap_or_default(),
            freq_range: (
                message.freq_min.unwrap_or_default(),
                message.freq_max.unwrap_or_default(),
            ),
            data_rates: Vec::new(),
        },
        GatewayStationKind::JoinRequest => Message::JoinRequest {
            mhdr: message.mhdr.unwrap_or_default(),
            join_eui: identifier(message.join_eui.as_deref().unwrap_or_default())?,
            dev_eui: identifier(message.dev_eui.as_deref().unwrap_or_default())?,
            dev_nonce: message.dev_nonce.unwrap_or_default(),
            mic: message.mic.unwrap_or_default(),
            data_rate: message.data_rate.unwrap_or_default(),
            frequency_hz: message.frequency_hz.unwrap_or_default(),
            levels,
        },
        GatewayStationKind::Uplink => Message::Uplink {
            mhdr: message.mhdr.unwrap_or_default(),
            dev_addr: message.dev_addr.unwrap_or_default(),
            fctrl: message.fctrl.unwrap_or_default(),
            fcnt: message.fcnt.unwrap_or_default(),
            fopts: message
                .fopts
                .map(|bytes| bytes.to_vec())
                .unwrap_or_default(),
            fport: message.fport,
            payload: message
                .payload
                .map(|bytes| bytes.to_vec())
                .unwrap_or_default(),
            mic: message.mic.unwrap_or_default(),
            data_rate: message.data_rate.unwrap_or_default(),
            frequency_hz: message.frequency_hz.unwrap_or_default(),
            levels,
        },
        GatewayStationKind::Proprietary => Message::Proprietary {
            payload: message
                .payload
                .map(|bytes| bytes.to_vec())
                .unwrap_or_default(),
            data_rate: message.data_rate.unwrap_or_default(),
            frequency_hz: message.frequency_hz.unwrap_or_default(),
            levels,
        },
        GatewayStationKind::Downlink => Message::Downlink {
            dev_eui: identifier(message.dev_eui.as_deref().unwrap_or_default())?,
            class: message.class.unwrap_or_default(),
            diid: message.diid.unwrap_or_default(),
            pdu: message.pdu.map(|bytes| bytes.to_vec()).unwrap_or_default(),
            rx_delay: message.rx_delay,
            rx1: None,
            rx2: None,
            priority: message.priority.unwrap_or_default(),
            xtime: message.xtime,
            rctx: message.rctx,
        },
        GatewayStationKind::Schedule => Message::Schedule {
            frames: message
                .schedule
                .unwrap_or_default()
                .into_iter()
                .map(|frame| Broadcast {
                    pdu: frame.pdu.to_vec(),
                    data_rate: frame.data_rate,
                    frequency_hz: frame.frequency_hz,
                    priority: frame.priority,
                    gpstime: frame.gpstime,
                    rctx: frame.rctx,
                })
                .collect(),
        },
        GatewayStationKind::Transmitted => Message::Transmitted {
            diid: message.diid.unwrap_or_default(),
            dev_eui: identifier(message.dev_eui.as_deref().unwrap_or_default())?,
            rctx: message.rctx.unwrap_or_default(),
            xtime: message.xtime.unwrap_or_default(),
            txtime: message.txtime.unwrap_or_default(),
            gpstime: message.gpstime,
        },
        GatewayStationKind::TimeSync => Message::TimeSync {
            txtime: message.txtime.map(|value| value as i64),
            xtime: message.xtime,
            gpstime: message.gpstime,
        },
        GatewayStationKind::Other => Message::Other {
            msgtype: message.msgtype.unwrap_or_default(),
        },
    })
}
