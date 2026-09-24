//! Generated Node bindings for the LoRa Basics Station protocol.
//!
//! These mirror the `pamoja_gateway::station` Rust API: the messages a station and its network
//! server exchange over a websocket, and the frame split a station does before it reports what
//! it heard. Nothing here opens a socket, so a program brings its own websocket, sends what
//! {@link stationEncode} writes, and reads whatever arrives with {@link stationParse}.
//!
//! Frequencies are in hertz and payloads are buffers rather than hexadecimal text, so nothing
//! has to be formatted by hand. A station clock and a downlink identifier are bigints: the
//! session byte in bits 48 to 55 of an xtime puts it past what a JavaScript number holds
//! exactly.

use napi::bindgen_prelude::{BigInt, Buffer};
use napi_derive::napi;
use pamoja_gateway::station::{eui_of, id6, Broadcast, Discovery, Levels, Message, Router, Xtime};
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
    pub xtime: BigInt,
    /// The GPS time, when the station has one.
    pub gpstime: Option<i64>,
    /// The received signal strength, in dBm.
    pub rssi: f64,
    /// The signal-to-noise ratio, in dB.
    pub snr: f64,
}

/// A receive window, or the ping slot a class B frame goes out in.
#[napi(object, js_name = "GatewayStationWindow")]
pub struct GatewayStationWindow {
    /// The data rate, as the network's table numbers it.
    pub data_rate: u8,
    /// The frequency in hertz.
    pub frequency_hz: u32,
}

/// One number of a configuration's data-rate table.
#[napi(object, js_name = "GatewayStationDataRate")]
pub struct GatewayStationDataRate {
    /// The spreading factor, 0 for FSK.
    pub spreading_factor: u8,
    /// The bandwidth in hertz.
    pub bandwidth_hz: u32,
    /// Whether the rate is used only for downlinks.
    pub downlink_only: bool,
}

/// A range of join identifiers whose join requests a station forwards, both ends included.
#[napi(object, js_name = "GatewayStationJoinRange")]
pub struct GatewayStationJoinRange {
    /// The first identifier, as sixteen hexadecimal digits.
    pub first: String,
    /// The last identifier, as sixteen hexadecimal digits.
    pub last: String,
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
    /// The kind as the protocol writes it, such as `jreq`; the word a kind this build does not
    /// model is written with.
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
    /// The networks whose data frames a configuration forwards, absent to forward every
    /// network's. A station matches each against the top seven bits of a device address, so an
    /// empty list forwards no data frame at all.
    pub net_id: Option<Vec<u32>>,
    /// The join identifier ranges a configuration forwards; empty or absent forwards every
    /// join.
    pub join_eui_ranges: Option<Vec<GatewayStationJoinRange>>,
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
    /// A configuration's data rates, indexed by data-rate number, with `null` for a number the
    /// table leaves undefined.
    pub data_rates: Option<Vec<Option<GatewayStationDataRate>>>,
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
    /// The data rate it arrived at.
    pub data_rate: Option<u8>,
    /// The frequency in hertz.
    pub frequency_hz: Option<u32>,
    /// How it was heard, for the kinds a station sends up.
    pub levels: Option<GatewayStationLevels>,
    /// Which class of downlink this is: 0 for A, 1 for B, 2 for C.
    pub class: Option<u8>,
    /// The identifier a downlink and its transmission report share.
    pub diid: Option<BigInt>,
    /// The frame to transmit, for a downlink.
    pub pdu: Option<Buffer>,
    /// The delay before the first receive window, in seconds.
    pub rx_delay: Option<u8>,
    /// The first receive window a downlink names.
    pub rx1: Option<GatewayStationWindow>,
    /// The second receive window a downlink names.
    pub rx2: Option<GatewayStationWindow>,
    /// The ping slot a class B downlink goes out in.
    pub ping_slot: Option<GatewayStationWindow>,
    /// How urgent a downlink is.
    pub priority: Option<u8>,
    /// The station clock in microseconds: the uplink a downlink answers, the moment a reported
    /// frame went out, or the one a time sync carries.
    pub xtime: Option<BigInt>,
    /// The radio, for a downlink or a report.
    pub rctx: Option<i64>,
    /// When a frame went out, in seconds, or the station time a time sync carries, in
    /// microseconds.
    pub txtime: Option<f64>,
    /// The GPS time in microseconds since the GPS epoch.
    pub gpstime: Option<i64>,
    /// What to transmit to a group, for a schedule.
    pub schedule: Option<Vec<GatewayStationBroadcast>>,
}

/// A station clock value taken apart.
#[napi(object, js_name = "GatewayStationXtime")]
pub struct GatewayStationXtime {
    /// The radio unit the time was read on, 0 to 127.
    pub unit: u8,
    /// The run of the station the time belongs to.
    pub session: u8,
    /// The microseconds the run had counted, below 2^48.
    pub micros: i64,
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
    let heard = Message::heard(&frame, data_rate, frequency_hz, levels_of(&levels)?)
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

/// Reads the request a station sent to find its network server, as the server does, and
/// returns the station asking as sixteen hexadecimal digits.
#[napi(js_name = "stationDiscoveryParse")]
pub fn station_discovery_parse(text: String) -> napi::Result<String> {
    let asked = Discovery::from_json(text.as_bytes())
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(asked.router.to_hex())
}

/// Writes the answer that sends a station to the websocket its session runs on.
#[napi(js_name = "stationRouterAccepted")]
pub fn station_router_accepted(router: String, muxs: String, uri: String) -> napi::Result<String> {
    Ok(Router::accepted(identifier(&router)?, identifier(&muxs)?, uri).to_json())
}

/// Writes the answer that refuses a station, saying why.
#[napi(js_name = "stationRouterRefused")]
pub fn station_router_refused(router: String, error: String) -> napi::Result<String> {
    Ok(Router::refused(identifier(&router)?, error).to_json())
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

/// Builds a station clock value from the radio it was read on, the run of the station, and
/// the microseconds that run had counted.
#[napi(js_name = "stationXtime")]
pub fn station_xtime(unit: f64, session: f64, micros: f64) -> napi::Result<BigInt> {
    let whole = |value: f64, below: f64| {
        (value.fract() == 0.0 && (0.0..below).contains(&value)).then_some(value as u64)
    };
    let clock = whole(unit, 128.0)
        .zip(whole(session, 256.0))
        .zip(whole(micros, (1u64 << 48) as f64))
        .and_then(|((unit, session), micros)| Xtime::new(unit as u8, session as u8, micros));
    clock.map(|clock| BigInt::from(clock.value())).ok_or_else(|| {
        napi::Error::from_reason(format!(
            "a station clock takes a unit up to 127, a session up to 255 and a whole number of microseconds below 2^48, not {unit}, {session} and {micros}"
        ))
    })
}

/// Takes a station clock value apart into its radio unit, run, and microseconds.
#[napi(js_name = "stationXtimeParts")]
pub fn station_xtime_parts(xtime: BigInt) -> napi::Result<GatewayStationXtime> {
    let clock = Xtime::of(exact(&xtime, "xtime")?);
    Ok(GatewayStationXtime {
        unit: clock.unit,
        session: clock.session,
        micros: clock.micros as i64,
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

/// Reads a 64-bit value JavaScript passed as a bigint, refusing one it does not fit.
fn exact(value: &BigInt, name: &str) -> napi::Result<i64> {
    match value.get_i64() {
        (read, true) => Ok(read),
        (_, false) => Err(napi::Error::from_reason(format!(
            "{name} does not fit a signed 64-bit integer"
        ))),
    }
}

/// Reads a join identifier range.
fn range_of(range: &GatewayStationJoinRange) -> napi::Result<(u64, u64)> {
    let bound = |text: &str| identifier(text).map(|eui| u64::from_be_bytes(eui.bytes()));
    Ok((bound(&range.first)?, bound(&range.last)?))
}

/// Writes a join identifier range.
fn range_to_js((first, last): (u64, u64)) -> GatewayStationJoinRange {
    GatewayStationJoinRange {
        first: Eui::new(first.to_be_bytes()).to_hex(),
        last: Eui::new(last.to_be_bytes()).to_hex(),
    }
}

/// Reads a window.
fn window_of(window: &GatewayStationWindow) -> (u8, u32) {
    (window.data_rate, window.frequency_hz)
}

/// Writes a window.
fn window_to_js(window: Option<(u8, u32)>) -> Option<GatewayStationWindow> {
    window.map(|(data_rate, frequency_hz)| GatewayStationWindow {
        data_rate,
        frequency_hz,
    })
}

/// Reads how a packet was heard.
fn levels_of(levels: &GatewayStationLevels) -> napi::Result<Levels> {
    Ok(Levels {
        rctx: levels.rctx,
        xtime: exact(&levels.xtime, "xtime")?,
        gpstime: levels.gpstime,
        rssi: levels.rssi,
        snr: levels.snr,
    })
}

/// Writes how a packet was heard.
fn levels_to_js(levels: Levels) -> GatewayStationLevels {
    GatewayStationLevels {
        rctx: levels.rctx,
        xtime: BigInt::from(levels.xtime),
        gpstime: levels.gpstime,
        rssi: levels.rssi,
        snr: levels.snr,
    }
}

/// An empty message of the given kind, which each arm then fills in.
fn blank(kind: GatewayStationKind, msgtype: &str) -> GatewayStationMessage {
    GatewayStationMessage {
        kind,
        msgtype: Some(msgtype.to_owned()),
        station: None,
        firmware: None,
        package: None,
        model: None,
        protocol: None,
        features: None,
        net_id: None,
        join_eui_ranges: None,
        region: None,
        max_eirp: None,
        hwspec: None,
        freq_min: None,
        freq_max: None,
        data_rates: None,
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
        rx1: None,
        rx2: None,
        ping_slot: None,
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
    let msgtype = message.msgtype();
    match message {
        Message::Version {
            station,
            firmware,
            package,
            model,
            protocol,
            features,
        } => {
            let mut held = blank(GatewayStationKind::Version, msgtype);
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
            join_eui,
            region,
            max_eirp,
            hwspec,
            freq_range,
            data_rates,
        } => {
            let mut held = blank(GatewayStationKind::RouterConfig, msgtype);
            held.net_id = net_id.clone();
            held.join_eui_ranges = Some(join_eui.iter().copied().map(range_to_js).collect());
            held.region = Some(region.clone());
            held.max_eirp = Some(*max_eirp);
            held.hwspec = Some(hwspec.clone());
            held.freq_min = Some(freq_range.0);
            held.freq_max = Some(freq_range.1);
            held.data_rates = Some(
                data_rates
                    .iter()
                    .map(|entry| {
                        entry.map(|(spreading_factor, bandwidth_hz, downlink_only)| {
                            GatewayStationDataRate {
                                spreading_factor,
                                bandwidth_hz,
                                downlink_only,
                            }
                        })
                    })
                    .collect(),
            );
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
            let mut held = blank(GatewayStationKind::JoinRequest, msgtype);
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
            let mut held = blank(GatewayStationKind::Uplink, msgtype);
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
            let mut held = blank(GatewayStationKind::Proprietary, msgtype);
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
            rx1,
            rx2,
            ping_slot,
            priority,
            xtime,
            rctx,
            gpstime,
        } => {
            let mut held = blank(GatewayStationKind::Downlink, msgtype);
            held.dev_eui = Some(dev_eui.to_hex());
            held.class = Some(*class);
            held.diid = Some(BigInt::from(*diid));
            held.pdu = Some(Buffer::from(pdu.clone()));
            held.rx_delay = *rx_delay;
            held.rx1 = window_to_js(*rx1);
            held.rx2 = window_to_js(*rx2);
            held.ping_slot = window_to_js(*ping_slot);
            held.priority = Some(*priority);
            held.xtime = xtime.map(BigInt::from);
            held.rctx = *rctx;
            held.gpstime = *gpstime;
            held
        }
        Message::Schedule { frames } => {
            let mut held = blank(GatewayStationKind::Schedule, msgtype);
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
            let mut held = blank(GatewayStationKind::Transmitted, msgtype);
            held.diid = Some(BigInt::from(*diid));
            held.dev_eui = Some(dev_eui.to_hex());
            held.rctx = Some(*rctx);
            held.xtime = Some(BigInt::from(*xtime));
            held.txtime = Some(*txtime);
            held.gpstime = *gpstime;
            held
        }
        Message::TimeSync {
            txtime,
            xtime,
            gpstime,
        } => {
            let mut held = blank(GatewayStationKind::TimeSync, msgtype);
            held.txtime = txtime.map(|value| value as f64);
            held.xtime = xtime.map(BigInt::from);
            held.gpstime = *gpstime;
            held
        }
        Message::Other { .. } => blank(GatewayStationKind::Other, msgtype),
    }
}

/// Reads a message JavaScript describes.
fn message_of(message: GatewayStationMessage) -> napi::Result<Message> {
    let levels = match &message.levels {
        Some(levels) => levels_of(levels)?,
        None => Levels::default(),
    };
    let xtime = message
        .xtime
        .as_ref()
        .map(|value| exact(value, "xtime"))
        .transpose()?;
    let diid = message
        .diid
        .as_ref()
        .map(|value| exact(value, "diid"))
        .transpose()?
        .unwrap_or_default();
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
            net_id: message.net_id,
            join_eui: message
                .join_eui_ranges
                .unwrap_or_default()
                .iter()
                .map(range_of)
                .collect::<napi::Result<_>>()?,
            region: message.region.unwrap_or_default(),
            max_eirp: message.max_eirp.unwrap_or_default(),
            hwspec: message.hwspec.unwrap_or_default(),
            freq_range: (
                message.freq_min.unwrap_or_default(),
                message.freq_max.unwrap_or_default(),
            ),
            data_rates: message
                .data_rates
                .unwrap_or_default()
                .into_iter()
                .map(|entry| {
                    entry.map(|rate| (rate.spreading_factor, rate.bandwidth_hz, rate.downlink_only))
                })
                .collect(),
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
            diid,
            pdu: message.pdu.map(|bytes| bytes.to_vec()).unwrap_or_default(),
            rx_delay: message.rx_delay,
            rx1: message.rx1.as_ref().map(window_of),
            rx2: message.rx2.as_ref().map(window_of),
            ping_slot: message.ping_slot.as_ref().map(window_of),
            priority: message.priority.unwrap_or_default(),
            xtime,
            rctx: message.rctx,
            gpstime: message.gpstime,
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
            diid,
            dev_eui: identifier(message.dev_eui.as_deref().unwrap_or_default())?,
            rctx: message.rctx.unwrap_or_default(),
            xtime: xtime.unwrap_or_default(),
            txtime: message.txtime.unwrap_or_default(),
            gpstime: message.gpstime,
        },
        GatewayStationKind::TimeSync => Message::TimeSync {
            txtime: message.txtime.map(|value| value.round() as i64),
            xtime,
            gpstime: message.gpstime,
        },
        GatewayStationKind::Other => Message::Other {
            msgtype: message
                .msgtype
                .filter(|word| !word.is_empty())
                .ok_or_else(|| {
                    napi::Error::from_reason(
                        "a message of a kind this build does not model needs its msgtype",
                    )
                })?,
        },
    })
}
