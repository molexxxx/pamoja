//! The JSON objects the datagrams carry: `rxpk`, `stat`, `txpk`, and `txpk_ack`.
//!
//! The protocol writes a frequency as megahertz with hertz precision, a payload as base64, a
//! LoRa link as a datarate identifier such as `SF7BW125` beside a coding rate such as `4/5`,
//! and every level as a plain number. These types hold each of those the way the rest of the
//! SDK does, in hertz, in bytes, as [`LinkSettings`], and in [`Decibels`], and convert at the
//! boundary.

use std::fmt;

use pamoja_lora::budget::Decibels;
use pamoja_lora::LinkSettings;
use serde_json::{json, Map, Value};

use crate::base64;

use super::ProtocolError;

/// What the CRC of a received packet said.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CrcStatus {
    /// The CRC checked, which is `1`.
    #[default]
    Ok,
    /// The CRC failed, which is `-1`.
    Failed,
    /// The packet carried no CRC, which is `0`.
    Absent,
}

impl CrcStatus {
    /// Returns the number the protocol writes.
    ///
    /// # Returns
    ///
    /// `1`, `-1`, or `0`.
    pub const fn code(self) -> i8 {
        match self {
            CrcStatus::Ok => 1,
            CrcStatus::Failed => -1,
            CrcStatus::Absent => 0,
        }
    }

    /// Names the status a number selects.
    ///
    /// # Arguments
    ///
    /// * `code` - the number the protocol wrote.
    ///
    /// # Returns
    ///
    /// The status, or `None` for any other number.
    pub const fn from_code(code: i64) -> Option<CrcStatus> {
        match code {
            1 => Some(CrcStatus::Ok),
            -1 => Some(CrcStatus::Failed),
            0 => Some(CrcStatus::Absent),
            _ => None,
        }
    }
}

/// How a packet was modulated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modulation {
    /// LoRa, whose datarate identifier and coding rate are the link's own settings.
    Lora(LinkSettings),
    /// FSK, at a bitrate in bits per second.
    Fsk(u32),
}

impl Modulation {
    /// Returns the link settings, for a LoRa packet.
    ///
    /// # Returns
    ///
    /// The settings, or `None` for FSK.
    pub const fn link(self) -> Option<LinkSettings> {
        match self {
            Modulation::Lora(link) => Some(link),
            Modulation::Fsk(_) => None,
        }
    }

    /// Writes the datarate identifier, such as `SF7BW125`.
    ///
    /// # Returns
    ///
    /// The identifier for LoRa, or the bitrate as a number for FSK.
    fn datarate(self) -> Value {
        match self {
            Modulation::Lora(link) => Value::String(format!(
                "SF{}BW{}",
                link.spreading_factor(),
                link.bandwidth_hz() / 1_000
            )),
            Modulation::Fsk(bitrate) => json!(bitrate),
        }
    }
}

/// Reads a datarate identifier such as `SF7BW125`.
///
/// # Arguments
///
/// * `text` - the identifier.
///
/// # Returns
///
/// The spreading factor and bandwidth it names, or `None` when it is not that shape.
fn link_of(text: &str) -> Option<(u8, u32)> {
    let (factor, bandwidth) = text.strip_prefix("SF")?.split_once("BW")?;
    Some((
        factor.parse().ok()?,
        bandwidth.parse::<u32>().ok()?.checked_mul(1_000)?,
    ))
}

/// Reads a coding rate such as `4/5`.
///
/// # Arguments
///
/// * `text` - the coding rate.
///
/// # Returns
///
/// Its denominator, or `None` when the numerator is not 4 or the text is not that shape.
fn coding_rate_of(text: &str) -> Option<u8> {
    let (numerator, denominator) = text.split_once('/')?;
    if numerator != "4" {
        return None;
    }
    denominator.parse().ok()
}

/// A packet the gateway heard, with the metadata the protocol carries beside it.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::udp::Rxpk;
/// use pamoja_lora::LinkSettings;
///
/// let heard = Rxpk::new(868_100_000, LinkSettings::new(7, 125_000), b"hello".to_vec())
///     .with_rssi_dbm(-35)
///     .with_snr_db(5.1);
/// assert!(heard.to_json().contains("\"datr\":\"SF7BW125\""));
/// assert!(heard.to_json().contains("\"freq\":868.1"));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Rxpk {
    /// When the packet arrived, as [`time::compact`](crate::time::compact) writes it.
    pub received_at: Option<String>,
    /// When it arrived on the GPS clock, in milliseconds since 6 January 1980.
    pub gps_millis: Option<u64>,
    /// The concentrator's own timestamp of the end of reception, in microseconds.
    pub timestamp_us: Option<u32>,
    /// The carrier the packet arrived on, in hertz.
    pub frequency_hz: u32,
    /// The concentrator channel it arrived on.
    pub channel: u8,
    /// The radio chain it arrived on.
    pub rf_chain: u8,
    /// What the CRC said.
    pub crc: CrcStatus,
    /// How it was modulated.
    pub modulation: Modulation,
    /// The received signal strength, to the decibel.
    pub rssi_dbm: Decibels,
    /// The signal-to-noise ratio, to a tenth of a decibel, for a LoRa packet.
    pub snr_db: Option<Decibels>,
    /// The packet itself.
    pub payload: Vec<u8>,
}

impl Rxpk {
    /// Describes a LoRa packet that arrived with a good CRC.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the carrier it arrived on.
    /// * `link` - the spreading factor, bandwidth, and coding rate it used.
    /// * `payload` - the packet.
    ///
    /// # Returns
    ///
    /// The description, on channel 0 and radio chain 0 with no levels yet.
    pub fn new(frequency_hz: u32, link: LinkSettings, payload: Vec<u8>) -> Rxpk {
        Rxpk {
            received_at: None,
            gps_millis: None,
            timestamp_us: None,
            frequency_hz,
            channel: 0,
            rf_chain: 0,
            crc: CrcStatus::Ok,
            modulation: Modulation::Lora(link),
            rssi_dbm: Decibels::ZERO,
            snr_db: None,
            payload,
        }
    }

    /// Returns it with the signal strength it was heard at.
    ///
    /// # Arguments
    ///
    /// * `dbm` - the strength in dBm.
    ///
    /// # Returns
    ///
    /// The description.
    pub fn with_rssi_dbm(mut self, dbm: i32) -> Rxpk {
        self.rssi_dbm = Decibels::from_db(dbm);
        self
    }

    /// Returns it with the signal-to-noise ratio it was heard at.
    ///
    /// # Arguments
    ///
    /// * `db` - the ratio in decibels, which the protocol carries to a tenth.
    ///
    /// # Returns
    ///
    /// The description.
    pub fn with_snr_db(mut self, db: f64) -> Rxpk {
        self.snr_db = Some(Decibels::from_hundredths((db * 100.0).round() as i32));
        self
    }

    /// Returns it with the concentrator's timestamp of the reception.
    ///
    /// # Arguments
    ///
    /// * `micros` - the counter value, which wraps every 71 minutes.
    ///
    /// # Returns
    ///
    /// The description.
    pub fn with_timestamp_us(mut self, micros: u32) -> Rxpk {
        self.timestamp_us = Some(micros);
        self
    }

    /// Returns it with the time it arrived.
    ///
    /// # Arguments
    ///
    /// * `micros_since_epoch` - the time in microseconds since 1970-01-01 UTC.
    ///
    /// # Returns
    ///
    /// The description.
    pub fn with_received_at(mut self, micros_since_epoch: u64) -> Rxpk {
        self.received_at = Some(crate::time::compact(micros_since_epoch));
        self
    }

    /// Returns it on another concentrator channel and radio chain.
    ///
    /// # Arguments
    ///
    /// * `channel` - the concentrator channel.
    /// * `rf_chain` - the radio chain.
    ///
    /// # Returns
    ///
    /// The description.
    pub fn on_channel(mut self, channel: u8, rf_chain: u8) -> Rxpk {
        self.channel = channel;
        self.rf_chain = rf_chain;
        self
    }

    /// Writes the object as the protocol carries it.
    ///
    /// # Returns
    ///
    /// The JSON text of one `rxpk` entry.
    pub fn to_json(&self) -> String {
        self.to_value().to_string()
    }

    /// Builds the object the protocol carries.
    fn to_value(&self) -> Value {
        let mut object = Map::new();
        if let Some(time) = &self.received_at {
            object.insert("time".to_owned(), json!(time));
        }
        if let Some(millis) = self.gps_millis {
            object.insert("tmms".to_owned(), json!(millis));
        }
        if let Some(micros) = self.timestamp_us {
            object.insert("tmst".to_owned(), json!(micros));
        }
        object.insert("chan".to_owned(), json!(self.channel));
        object.insert("rfch".to_owned(), json!(self.rf_chain));
        object.insert("freq".to_owned(), megahertz(self.frequency_hz));
        object.insert("stat".to_owned(), json!(self.crc.code()));
        object.insert("modu".to_owned(), json!(name_of(self.modulation)));
        object.insert("datr".to_owned(), self.modulation.datarate());
        if let Modulation::Lora(link) = self.modulation {
            object.insert(
                "codr".to_owned(),
                json!(format!("4/{}", link.coding_rate_denominator())),
            );
        }
        object.insert("rssi".to_owned(), json!(self.rssi_dbm.round_db()));
        if let Some(snr) = self.snr_db {
            object.insert("lsnr".to_owned(), decibels(snr));
        }
        object.insert("size".to_owned(), json!(self.payload.len()));
        object.insert("data".to_owned(), json!(base64::encode(&self.payload)));
        Value::Object(object)
    }

    /// Reads one `rxpk` entry.
    fn from_value(value: &Value) -> Result<Rxpk, ProtocolError> {
        let object = object_of(value, "an rxpk entry")?;
        let modulation = modulation_of(object, "rxpk")?;
        Ok(Rxpk {
            received_at: text(object, "time").map(str::to_owned),
            gps_millis: whole(object, "tmms").map(|value| value as u64),
            timestamp_us: whole(object, "tmst").map(|value| value as u32),
            frequency_hz: hertz(object, "rxpk")?,
            channel: whole(object, "chan").unwrap_or(0) as u8,
            rf_chain: whole(object, "rfch").unwrap_or(0) as u8,
            crc: whole(object, "stat")
                .and_then(CrcStatus::from_code)
                .unwrap_or(CrcStatus::Absent),
            modulation,
            rssi_dbm: Decibels::from_db(whole(object, "rssi").unwrap_or(0) as i32),
            snr_db: number(object, "lsnr")
                .map(|snr| Decibels::from_hundredths((snr * 100.0).round() as i32)),
            payload: payload_of(object, "rxpk")?,
        })
    }
}

/// The gateway's own status report.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::udp::Stat;
///
/// let report = Stat::new().with_counts(2, 2, 2).with_downlinks(2, 2);
/// assert!(report.to_json().contains("\"rxfw\":2"));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stat {
    /// The gateway's clock, as [`time::expanded`](crate::time::expanded) writes it.
    pub time: Option<String>,
    /// Its latitude in degrees, north positive.
    pub latitude_deg: Option<f64>,
    /// Its longitude in degrees, east positive.
    pub longitude_deg: Option<f64>,
    /// Its altitude in meters.
    pub altitude_m: Option<i32>,
    /// How many packets its radio received.
    pub received: u32,
    /// How many of those had a good CRC.
    pub received_ok: u32,
    /// How many it forwarded.
    pub forwarded: u32,
    /// What share of its datagrams were acknowledged, as a percentage.
    pub acknowledged_percent: f64,
    /// How many downlink datagrams it received.
    pub downlinks: u32,
    /// How many packets it transmitted.
    pub transmitted: u32,
}

impl Stat {
    /// An empty report, which is what a gateway with nothing yet to say sends.
    ///
    /// # Returns
    ///
    /// The report.
    pub fn new() -> Stat {
        Stat::default()
    }

    /// Returns it with the gateway's clock.
    ///
    /// # Arguments
    ///
    /// * `seconds_since_epoch` - the time in seconds since 1970-01-01 UTC.
    ///
    /// # Returns
    ///
    /// The report.
    pub fn at(mut self, seconds_since_epoch: u64) -> Stat {
        self.time = Some(crate::time::expanded(seconds_since_epoch));
        self
    }

    /// Returns it with the gateway's position.
    ///
    /// # Arguments
    ///
    /// * `latitude_deg` - degrees north.
    /// * `longitude_deg` - degrees east.
    /// * `altitude_m` - meters above sea level.
    ///
    /// # Returns
    ///
    /// The report.
    pub fn at_position(mut self, latitude_deg: f64, longitude_deg: f64, altitude_m: i32) -> Stat {
        self.latitude_deg = Some(latitude_deg);
        self.longitude_deg = Some(longitude_deg);
        self.altitude_m = Some(altitude_m);
        self
    }

    /// Returns it with what the radio heard.
    ///
    /// # Arguments
    ///
    /// * `received` - packets received.
    /// * `received_ok` - those with a good CRC.
    /// * `forwarded` - those forwarded to the server.
    ///
    /// # Returns
    ///
    /// The report.
    pub fn with_counts(mut self, received: u32, received_ok: u32, forwarded: u32) -> Stat {
        self.received = received;
        self.received_ok = received_ok;
        self.forwarded = forwarded;
        self
    }

    /// Returns it with what the downlink side did.
    ///
    /// # Arguments
    ///
    /// * `downlinks` - datagrams received from the server.
    /// * `transmitted` - packets transmitted.
    ///
    /// # Returns
    ///
    /// The report.
    pub fn with_downlinks(mut self, downlinks: u32, transmitted: u32) -> Stat {
        self.downlinks = downlinks;
        self.transmitted = transmitted;
        self
    }

    /// Returns it with the share of its datagrams the server acknowledged.
    ///
    /// # Arguments
    ///
    /// * `percent` - the percentage.
    ///
    /// # Returns
    ///
    /// The report.
    pub fn with_acknowledged_percent(mut self, percent: f64) -> Stat {
        self.acknowledged_percent = percent;
        self
    }

    /// Writes the object as the protocol carries it.
    ///
    /// # Returns
    ///
    /// The JSON text of the `stat` object.
    pub fn to_json(&self) -> String {
        self.to_value().to_string()
    }

    /// Builds the object the protocol carries.
    fn to_value(&self) -> Value {
        let mut object = Map::new();
        if let Some(time) = &self.time {
            object.insert("time".to_owned(), json!(time));
        }
        if let (Some(latitude), Some(longitude)) = (self.latitude_deg, self.longitude_deg) {
            object.insert("lati".to_owned(), json!(latitude));
            object.insert("long".to_owned(), json!(longitude));
        }
        if let Some(altitude) = self.altitude_m {
            object.insert("alti".to_owned(), json!(altitude));
        }
        object.insert("rxnb".to_owned(), json!(self.received));
        object.insert("rxok".to_owned(), json!(self.received_ok));
        object.insert("rxfw".to_owned(), json!(self.forwarded));
        object.insert("ackr".to_owned(), json!(self.acknowledged_percent));
        object.insert("dwnb".to_owned(), json!(self.downlinks));
        object.insert("txnb".to_owned(), json!(self.transmitted));
        Value::Object(object)
    }

    /// Reads the `stat` object.
    fn from_value(value: &Value) -> Result<Stat, ProtocolError> {
        let object = object_of(value, "a stat object")?;
        Ok(Stat {
            time: text(object, "time").map(str::to_owned),
            latitude_deg: number(object, "lati"),
            longitude_deg: number(object, "long"),
            altitude_m: whole(object, "alti").map(|value| value as i32),
            received: whole(object, "rxnb").unwrap_or(0) as u32,
            received_ok: whole(object, "rxok").unwrap_or(0) as u32,
            forwarded: whole(object, "rxfw").unwrap_or(0) as u32,
            acknowledged_percent: number(object, "ackr").unwrap_or(0.0),
            downlinks: whole(object, "dwnb").unwrap_or(0) as u32,
            transmitted: whole(object, "txnb").unwrap_or(0) as u32,
        })
    }
}

/// What a PUSH_DATA carries: the packets heard, and the gateway's own report.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Uplink {
    /// The packets, which may be none when only a report is being sent.
    pub packets: Vec<Rxpk>,
    /// The report, which a gateway sends every half minute or so.
    pub status: Option<Stat>,
}

impl From<Rxpk> for Uplink {
    fn from(packet: Rxpk) -> Uplink {
        Uplink {
            packets: vec![packet],
            status: None,
        }
    }
}

impl From<Stat> for Uplink {
    fn from(status: Stat) -> Uplink {
        Uplink {
            packets: Vec::new(),
            status: Some(status),
        }
    }
}

impl Uplink {
    /// Writes the payload as the protocol carries it.
    ///
    /// # Returns
    ///
    /// The JSON text, with an `rxpk` array when there are packets and a `stat` object when
    /// there is a report.
    pub fn to_json(&self) -> String {
        let mut object = Map::new();
        if !self.packets.is_empty() {
            object.insert(
                "rxpk".to_owned(),
                Value::Array(self.packets.iter().map(Rxpk::to_value).collect()),
            );
        }
        if let Some(status) = &self.status {
            object.insert("stat".to_owned(), status.to_value());
        }
        Value::Object(object).to_string()
    }

    /// Reads the payload of a PUSH_DATA.
    ///
    /// # Arguments
    ///
    /// * `body` - the bytes after the header and the gateway's identifier.
    ///
    /// # Returns
    ///
    /// The packets and the report it carries.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the body is not a JSON object, carries neither
    /// an `rxpk` array nor a `stat` object, or holds an entry the protocol does not describe.
    pub fn from_json(body: &[u8]) -> Result<Uplink, ProtocolError> {
        let value = parse_json(body, "a PUSH_DATA")?;
        let object = object_of(&value, "a PUSH_DATA payload")?;
        let packets = match object.get("rxpk") {
            None => Vec::new(),
            Some(Value::Array(entries)) => entries
                .iter()
                .map(Rxpk::from_value)
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err(refused("\"rxpk\" is not an array")),
        };
        let status = match object.get("stat") {
            None => None,
            Some(value) => Some(Stat::from_value(value)?),
        };
        if packets.is_empty() && status.is_none() {
            return Err(refused(
                "a PUSH_DATA payload carries neither \"rxpk\" nor \"stat\"",
            ));
        }
        Ok(Uplink { packets, status })
    }
}

/// A packet the server asks the gateway to transmit.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::udp::Txpk;
/// use pamoja_lora::LinkSettings;
///
/// let downlink = Txpk::at(3_512_348_611, 868_500_000, LinkSettings::new(9, 125_000), b"ok".to_vec())
///     .with_power_dbm(27)
///     .with_inverted_polarity(true);
/// assert!(downlink.to_json().contains("\"ipol\":true"));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Txpk {
    /// Whether to transmit at once, which ignores the timestamps.
    pub immediate: bool,
    /// The concentrator timestamp to transmit at, in microseconds.
    pub timestamp_us: Option<u32>,
    /// The GPS time to transmit at, in milliseconds since 6 January 1980.
    pub gps_millis: Option<u64>,
    /// The carrier to transmit on, in hertz.
    pub frequency_hz: u32,
    /// The radio chain to transmit from.
    pub rf_chain: u8,
    /// The power to transmit at, in dBm.
    pub power_dbm: i8,
    /// How to modulate it.
    pub modulation: Modulation,
    /// The FSK frequency deviation in hertz.
    pub frequency_deviation_hz: Option<u32>,
    /// Whether to invert the LoRa polarity, which a LoRaWAN downlink does.
    pub invert_polarity: bool,
    /// How long a preamble to send, in symbols.
    pub preamble_symbols: Option<u16>,
    /// Whether to leave the physical CRC off, which LoRaWAN downlinks do.
    pub without_crc: bool,
    /// The packet itself.
    pub payload: Vec<u8>,
}

impl Txpk {
    /// Describes a LoRa packet to transmit as soon as the gateway can.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the carrier.
    /// * `link` - the spreading factor, bandwidth, and coding rate.
    /// * `payload` - the packet.
    ///
    /// # Returns
    ///
    /// The request, at 14 dBm on radio chain 0 with standard polarity.
    pub fn immediate(frequency_hz: u32, link: LinkSettings, payload: Vec<u8>) -> Txpk {
        Txpk {
            immediate: true,
            timestamp_us: None,
            gps_millis: None,
            frequency_hz,
            rf_chain: 0,
            power_dbm: 14,
            modulation: Modulation::Lora(link),
            frequency_deviation_hz: None,
            invert_polarity: false,
            preamble_symbols: None,
            without_crc: false,
            payload,
        }
    }

    /// Describes a LoRa packet to transmit at a concentrator timestamp, which is how a
    /// LoRaWAN receive window is hit.
    ///
    /// # Arguments
    ///
    /// * `timestamp_us` - the concentrator counter value to transmit at.
    /// * `frequency_hz` - the carrier.
    /// * `link` - the spreading factor, bandwidth, and coding rate.
    /// * `payload` - the packet.
    ///
    /// # Returns
    ///
    /// The request.
    pub fn at(timestamp_us: u32, frequency_hz: u32, link: LinkSettings, payload: Vec<u8>) -> Txpk {
        Txpk {
            immediate: false,
            timestamp_us: Some(timestamp_us),
            ..Txpk::immediate(frequency_hz, link, payload)
        }
    }

    /// Returns it at another power.
    ///
    /// # Arguments
    ///
    /// * `dbm` - the power in dBm.
    ///
    /// # Returns
    ///
    /// The request.
    pub fn with_power_dbm(mut self, dbm: i8) -> Txpk {
        self.power_dbm = dbm;
        self
    }

    /// Returns it with the LoRa polarity inverted or not.
    ///
    /// # Arguments
    ///
    /// * `inverted` - `true` for a LoRaWAN downlink, which a device listens for inverted.
    ///
    /// # Returns
    ///
    /// The request.
    pub fn with_inverted_polarity(mut self, inverted: bool) -> Txpk {
        self.invert_polarity = inverted;
        self
    }

    /// Returns it with the physical CRC left off, as LoRaWAN downlinks are sent.
    ///
    /// # Returns
    ///
    /// The request.
    pub fn without_crc(mut self) -> Txpk {
        self.without_crc = true;
        self
    }

    /// Writes the payload of a PULL_RESP.
    ///
    /// # Returns
    ///
    /// The JSON text, with the request under `txpk`.
    pub fn to_json(&self) -> String {
        json!({ "txpk": self.to_value() }).to_string()
    }

    /// Builds the object the protocol carries.
    fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("imme".to_owned(), json!(self.immediate));
        if let Some(micros) = self.timestamp_us {
            object.insert("tmst".to_owned(), json!(micros));
        }
        if let Some(millis) = self.gps_millis {
            object.insert("tmms".to_owned(), json!(millis));
        }
        object.insert("freq".to_owned(), megahertz(self.frequency_hz));
        object.insert("rfch".to_owned(), json!(self.rf_chain));
        object.insert("powe".to_owned(), json!(self.power_dbm));
        object.insert("modu".to_owned(), json!(name_of(self.modulation)));
        object.insert("datr".to_owned(), self.modulation.datarate());
        match self.modulation {
            Modulation::Lora(link) => {
                object.insert(
                    "codr".to_owned(),
                    json!(format!("4/{}", link.coding_rate_denominator())),
                );
                object.insert("ipol".to_owned(), json!(self.invert_polarity));
            }
            Modulation::Fsk(_) => {
                if let Some(deviation) = self.frequency_deviation_hz {
                    object.insert("fdev".to_owned(), json!(deviation));
                }
            }
        }
        if let Some(symbols) = self.preamble_symbols {
            object.insert("prea".to_owned(), json!(symbols));
        }
        object.insert("size".to_owned(), json!(self.payload.len()));
        object.insert("data".to_owned(), json!(base64::encode(&self.payload)));
        if self.without_crc {
            object.insert("ncrc".to_owned(), json!(true));
        }
        Value::Object(object)
    }

    /// Reads the payload of a PULL_RESP.
    ///
    /// # Arguments
    ///
    /// * `body` - the bytes after the header.
    ///
    /// # Returns
    ///
    /// The request it carries.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the body is not a JSON object with a `txpk`
    /// the protocol describes.
    pub fn from_json(body: &[u8]) -> Result<Txpk, ProtocolError> {
        let value = parse_json(body, "a PULL_RESP")?;
        let object = object_of(&value, "a PULL_RESP payload")?;
        let request = object
            .get("txpk")
            .ok_or_else(|| refused("a PULL_RESP payload carries no \"txpk\""))?;
        let request = object_of(request, "a txpk object")?;
        Ok(Txpk {
            immediate: flag(request, "imme"),
            timestamp_us: whole(request, "tmst").map(|value| value as u32),
            gps_millis: whole(request, "tmms").map(|value| value as u64),
            frequency_hz: hertz(request, "txpk")?,
            rf_chain: whole(request, "rfch").unwrap_or(0) as u8,
            power_dbm: whole(request, "powe").unwrap_or(14) as i8,
            modulation: modulation_of(request, "txpk")?,
            frequency_deviation_hz: whole(request, "fdev").map(|value| value as u32),
            invert_polarity: flag(request, "ipol"),
            preamble_symbols: whole(request, "prea").map(|value| value as u16),
            without_crc: flag(request, "ncrc"),
            payload: payload_of(request, "txpk")?,
        })
    }
}

/// What became of a downlink the server asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TxStatus {
    /// It was scheduled, which the protocol writes as `NONE`.
    #[default]
    None,
    /// It arrived too late to be scheduled.
    TooLate,
    /// Its timestamp is too far ahead.
    TooEarly,
    /// Another packet was already scheduled then.
    CollisionPacket,
    /// A beacon was already scheduled then.
    CollisionBeacon,
    /// The radio chain cannot reach that frequency.
    TxFreq,
    /// The gateway cannot transmit at that power.
    TxPower,
    /// A GPS timestamp was asked for while the GPS is unlocked.
    GpsUnlocked,
}

impl TxStatus {
    /// Returns the value the protocol writes.
    ///
    /// # Returns
    ///
    /// The `error` string.
    pub const fn as_str(self) -> &'static str {
        match self {
            TxStatus::None => "NONE",
            TxStatus::TooLate => "TOO_LATE",
            TxStatus::TooEarly => "TOO_EARLY",
            TxStatus::CollisionPacket => "COLLISION_PACKET",
            TxStatus::CollisionBeacon => "COLLISION_BEACON",
            TxStatus::TxFreq => "TX_FREQ",
            TxStatus::TxPower => "TX_POWER",
            TxStatus::GpsUnlocked => "GPS_UNLOCKED",
        }
    }

    /// Names the status a value selects.
    ///
    /// # Arguments
    ///
    /// * `text` - the `error` string.
    ///
    /// # Returns
    ///
    /// The status, or `None` for a value the protocol does not define.
    pub fn named(text: &str) -> Option<TxStatus> {
        [
            TxStatus::None,
            TxStatus::TooLate,
            TxStatus::TooEarly,
            TxStatus::CollisionPacket,
            TxStatus::CollisionBeacon,
            TxStatus::TxFreq,
            TxStatus::TxPower,
            TxStatus::GpsUnlocked,
        ]
        .into_iter()
        .find(|status| status.as_str() == text)
    }

    /// Reports whether the downlink was scheduled.
    ///
    /// # Returns
    ///
    /// `true` for [`TxStatus::None`], which is the protocol's way of saying nothing failed.
    pub const fn scheduled(self) -> bool {
        matches!(self, TxStatus::None)
    }

    /// Writes the payload of a TX_ACK.
    ///
    /// # Returns
    ///
    /// The JSON text, with the status under `txpk_ack`.
    pub fn to_json(self) -> String {
        json!({ "txpk_ack": { "error": self.as_str() } }).to_string()
    }

    /// Reads the payload of a TX_ACK, which a gateway may leave empty when nothing failed.
    ///
    /// # Arguments
    ///
    /// * `body` - the bytes after the gateway's identifier.
    ///
    /// # Returns
    ///
    /// The status.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::Payload`] when the body is neither empty nor a JSON object
    /// with a `txpk_ack` naming a value the protocol defines.
    pub fn from_json(body: &[u8]) -> Result<TxStatus, ProtocolError> {
        if body.iter().all(u8::is_ascii_whitespace) {
            return Ok(TxStatus::None);
        }
        let value = parse_json(body, "a TX_ACK")?;
        let object = object_of(&value, "a TX_ACK payload")?;
        let acknowledgment = object
            .get("txpk_ack")
            .ok_or_else(|| refused("a TX_ACK payload carries no \"txpk_ack\""))?;
        let acknowledgment = object_of(acknowledgment, "a txpk_ack object")?;
        match text(acknowledgment, "error") {
            None => Ok(TxStatus::None),
            Some(error) => TxStatus::named(error)
                .ok_or_else(|| refused(&format!("{error} is not a TX_ACK error"))),
        }
    }
}

impl fmt::Display for TxStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Names a modulation the way the protocol writes it.
fn name_of(modulation: Modulation) -> &'static str {
    match modulation {
        Modulation::Lora(_) => "LORA",
        Modulation::Fsk(_) => "FSK",
    }
}

/// Writes a frequency in megahertz, which is how the protocol carries one.
fn megahertz(hertz: u32) -> Value {
    json!(f64::from(hertz) / 1_000_000.0)
}

/// Writes a level to the tenth of a decibel the protocol carries.
fn decibels(value: Decibels) -> Value {
    json!(f64::from(value.hundredths()) / 100.0)
}

/// Reads an object's frequency field, in hertz.
fn hertz(object: &Map<String, Value>, what: &str) -> Result<u32, ProtocolError> {
    let megahertz =
        number(object, "freq").ok_or_else(|| refused(&format!("a {what} carries no \"freq\"")))?;
    let hertz = (megahertz * 1_000_000.0).round();
    if !(0.0..=f64::from(u32::MAX)).contains(&hertz) {
        return Err(refused(&format!("{megahertz} MHz is not a carrier")));
    }
    Ok(hertz as u32)
}

/// Reads an object's modulation, from its `modu`, `datr`, and `codr` fields.
fn modulation_of(object: &Map<String, Value>, what: &str) -> Result<Modulation, ProtocolError> {
    match text(object, "modu") {
        Some("FSK") => {
            let bitrate = whole(object, "datr")
                .ok_or_else(|| refused(&format!("an FSK {what} carries no bitrate in \"datr\"")))?;
            Ok(Modulation::Fsk(bitrate as u32))
        }
        Some("LORA") | None => {
            let datarate = text(object, "datr")
                .ok_or_else(|| refused(&format!("a LoRa {what} carries no \"datr\"")))?;
            let (factor, bandwidth) = link_of(datarate)
                .ok_or_else(|| refused(&format!("{datarate} is not a datarate identifier")))?;
            let mut link = LinkSettings::new(factor, bandwidth);
            if let Some(coding) = text(object, "codr") {
                let denominator = coding_rate_of(coding)
                    .ok_or_else(|| refused(&format!("{coding} is not a coding rate")))?;
                link = link.with_coding_rate(denominator);
            }
            Ok(Modulation::Lora(link))
        }
        Some(other) => Err(refused(&format!("{other} is not a modulation"))),
    }
}

/// Reads an object's payload, from its base64 `data` field.
fn payload_of(object: &Map<String, Value>, what: &str) -> Result<Vec<u8>, ProtocolError> {
    let data =
        text(object, "data").ok_or_else(|| refused(&format!("a {what} carries no \"data\"")))?;
    base64::decode(data)
        .map_err(|error| refused(&format!("the {what} payload is not base64: {error}")))
}

/// Parses a datagram's payload as JSON.
fn parse_json(body: &[u8], what: &str) -> Result<Value, ProtocolError> {
    serde_json::from_slice(body)
        .map_err(|error| refused(&format!("the payload of {what} is not JSON: {error}")))
}

/// Borrows a value as an object.
fn object_of<'a>(value: &'a Value, what: &str) -> Result<&'a Map<String, Value>, ProtocolError> {
    value
        .as_object()
        .ok_or_else(|| refused(&format!("{what} is not a JSON object")))
}

/// Reads a string field.
fn text<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key).and_then(Value::as_str)
}

/// Reads a number field.
fn number(object: &Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key).and_then(Value::as_f64)
}

/// Reads a whole-number field, which a gateway may write as a float.
fn whole(object: &Map<String, Value>, key: &str) -> Option<i64> {
    number(object, key).map(|value| value.round() as i64)
}

/// Reads a flag field, which is absent when it is false.
fn flag(object: &Map<String, Value>, key: &str) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// Builds the error a malformed payload reports.
fn refused(why: &str) -> ProtocolError {
    ProtocolError::Payload(why.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first rxpk entry of the protocol's own example, in section 4.
    const EXAMPLE_RXPK: &str = r#"{"rxpk":[{
        "time":"2013-03-31T16:21:17.528002Z",
        "tmst":3512348611,
        "chan":2,
        "rfch":0,
        "freq":866.349812,
        "stat":1,
        "modu":"LORA",
        "datr":"SF7BW125",
        "codr":"4/6",
        "rssi":-35,
        "lsnr":5.1,
        "size":32,
        "data":"VEVTVF9QQUNLRVRfMTIzNA=="
    }]}"#;

    /// The stat example of section 4.
    const EXAMPLE_STAT: &str = r#"{"stat":{
        "time":"2014-01-12 08:59:28 GMT",
        "lati":46.24000,
        "long":3.25230,
        "alti":145,
        "rxnb":2,
        "rxok":2,
        "rxfw":2,
        "ackr":100.0,
        "dwnb":2,
        "txnb":2
    }}"#;

    /// The LoRa txpk example of section 6.
    const EXAMPLE_TXPK: &str = r#"{"txpk":{
        "imme":true,
        "freq":864.123456,
        "rfch":0,
        "powe":14,
        "modu":"LORA",
        "datr":"SF11BW125",
        "codr":"4/6",
        "ipol":false,
        "size":32,
        "data":"H3P3N2i9qc4yt7rK7ldqoeCVJGBybzPY5h1Dd7P7p8v"
    }}"#;

    #[test]
    fn the_protocols_own_rxpk_example_reads() {
        let uplink = Uplink::from_json(EXAMPLE_RXPK.as_bytes()).expect("the example parses");
        let heard = &uplink.packets[0];

        assert_eq!(
            heard.received_at.as_deref(),
            Some("2013-03-31T16:21:17.528002Z")
        );
        assert_eq!(heard.timestamp_us, Some(3_512_348_611));
        assert_eq!(heard.frequency_hz, 866_349_812);
        assert_eq!((heard.channel, heard.rf_chain), (2, 0));
        assert_eq!(heard.crc, CrcStatus::Ok);
        assert_eq!(
            heard.modulation,
            Modulation::Lora(LinkSettings::new(7, 125_000).with_coding_rate(6))
        );
        assert_eq!(heard.rssi_dbm.round_db(), -35);
        assert_eq!(heard.snr_db.map(Decibels::hundredths), Some(510));
        assert_eq!(heard.payload, b"TEST_PACKET_1234");
    }

    #[test]
    fn the_protocols_own_stat_example_reads() {
        let uplink = Uplink::from_json(EXAMPLE_STAT.as_bytes()).expect("the example parses");
        let report = uplink.status.expect("it carries a report");

        assert_eq!(report.time.as_deref(), Some("2014-01-12 08:59:28 GMT"));
        assert_eq!(report.latitude_deg, Some(46.24));
        assert_eq!(report.altitude_m, Some(145));
        assert_eq!(
            (report.received, report.received_ok, report.forwarded),
            (2, 2, 2)
        );
        assert_eq!(report.acknowledged_percent, 100.0);
        assert_eq!((report.downlinks, report.transmitted), (2, 2));
        assert!(uplink.packets.is_empty());
    }

    #[test]
    fn the_protocols_own_txpk_example_reads() {
        let request = Txpk::from_json(EXAMPLE_TXPK.as_bytes()).expect("the example parses");

        assert!(request.immediate && !request.invert_polarity);
        assert_eq!(request.frequency_hz, 864_123_456);
        assert_eq!(request.power_dbm, 14);
        assert_eq!(
            request.modulation,
            Modulation::Lora(LinkSettings::new(11, 125_000).with_coding_rate(6))
        );
        // The example's payload is written without padding, which the protocol allows.
        assert_eq!(request.payload.len(), 32);
    }

    #[test]
    fn an_fsk_packet_carries_its_bitrate_as_a_number() {
        let fsk = r#"{"rxpk":[{"tmst":1,"freq":869.1,"stat":1,"modu":"FSK","datr":50000,
            "rssi":-75,"size":16,"data":"VEVTVF9QQUNLRVRfMTIzNA=="}]}"#;
        let uplink = Uplink::from_json(fsk.as_bytes()).expect("it parses");

        assert_eq!(uplink.packets[0].modulation, Modulation::Fsk(50_000));
        assert!(uplink.to_json().contains("\"datr\":50000"));
    }

    #[test]
    fn what_is_written_is_read_back() {
        let uplink = Uplink {
            packets: vec![Rxpk::new(
                868_100_000,
                LinkSettings::new(12, 125_000).with_coding_rate(8),
                b"\x00\x01\xFE\xFF".to_vec(),
            )
            .with_rssi_dbm(-107)
            .with_snr_db(-7.8)
            .with_timestamp_us(1_234_567)
            .with_received_at(1_364_746_877_528_002)
            .on_channel(3, 1)],
            status: Some(
                Stat::new()
                    .at(1_389_517_168)
                    .at_position(46.24, 3.2523, 145)
                    .with_counts(7, 6, 6)
                    .with_downlinks(2, 1)
                    .with_acknowledged_percent(99.5),
            ),
        };

        let round = Uplink::from_json(uplink.to_json().as_bytes()).expect("what we wrote parses");
        assert_eq!(round, uplink);

        let downlink = Txpk::at(
            3_512_348_611,
            869_525_000,
            LinkSettings::new(9, 125_000),
            b"downlink".to_vec(),
        )
        .with_power_dbm(27)
        .with_inverted_polarity(true)
        .without_crc();
        assert_eq!(
            Txpk::from_json(downlink.to_json().as_bytes()).expect("what we wrote parses"),
            downlink
        );
    }

    #[test]
    fn every_tx_ack_value_is_the_one_the_protocol_lists() {
        for (status, text) in [
            (TxStatus::None, "NONE"),
            (TxStatus::TooLate, "TOO_LATE"),
            (TxStatus::TooEarly, "TOO_EARLY"),
            (TxStatus::CollisionPacket, "COLLISION_PACKET"),
            (TxStatus::CollisionBeacon, "COLLISION_BEACON"),
            (TxStatus::TxFreq, "TX_FREQ"),
            (TxStatus::TxPower, "TX_POWER"),
            (TxStatus::GpsUnlocked, "GPS_UNLOCKED"),
        ] {
            assert_eq!(status.as_str(), text);
            assert_eq!(TxStatus::named(text), Some(status));
            assert_eq!(TxStatus::from_json(status.to_json().as_bytes()), Ok(status));
        }
        assert!(TxStatus::None.scheduled() && !TxStatus::TxPower.scheduled());
        assert_eq!(TxStatus::from_json(b""), Ok(TxStatus::None));
        assert!(TxStatus::named("SOMETHING_ELSE").is_none());
    }

    #[test]
    fn a_payload_the_protocol_does_not_describe_is_refused() {
        for (body, why) in [
            (r#"{"rxpk":{}}"#, "array"),
            (r#"{"rxpk":[{"freq":868.1,"datr":"SF7BW125"}]}"#, "data"),
            (r#"{"rxpk":[{"datr":"SF7BW125","data":"aGk="}]}"#, "freq"),
            (
                r#"{"rxpk":[{"freq":868.1,"datr":"7BW125","data":"aGk="}]}"#,
                "datarate",
            ),
            (
                r#"{"rxpk":[{"freq":868.1,"modu":"GFSK","datr":50000,"data":"aGk="}]}"#,
                "modulation",
            ),
            (r#"{}"#, "neither"),
        ] {
            let refusal = Uplink::from_json(body.as_bytes()).expect_err("it is refused");
            assert!(
                refusal.to_string().contains(why),
                "{body} said {refusal} rather than naming {why}"
            );
        }
    }
}
