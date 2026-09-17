//! Reading what a ChirpStack network server publishes about the devices it hears.
//!
//! A gateway hands packets to a network server, and the server hands what they carried to the
//! application: decrypted, deduplicated across gateways, and tagged with the device it came
//! from. ChirpStack publishes that on MQTT, one event per uplink, on
//! `application/{application id}/device/{dev eui}/event/up`, as the JSON form of its
//! `integration.UplinkEvent` message. [`UplinkEvent::from_json`] reads it into plain fields,
//! so a program that feeds a dashboard or a store needs to know nothing about the message
//! definitions.
//!
//! Two things about the JSON are worth knowing, both from protobuf's JSON mapping rather than
//! from ChirpStack. Every name is camelCase, so `f_port` arrives as `fPort`. And a field at its
//! default value is left out, so the first uplink after a join has no `fCnt`, and an uplink
//! without ADR has no `adr`; they read here as zero and `false`.
//!
//! The layout follows ChirpStack v4's `api/proto/integration/integration.proto` and the example
//! on its integration events documentation page, and the ChirpStack interop job reads a live
//! server's events with it.
//!
//! # Examples
//!
//! The event ChirpStack's documentation shows for an uplink:
//!
//! ```
//! use pamoja_gateway::chirpstack::UplinkEvent;
//!
//! let json = r#"{
//!     "deduplicationId": "3ac7e3c4-4401-4b8d-9386-a5c902f9202d",
//!     "time": "2022-07-18T09:34:15.775023242+00:00",
//!     "deviceInfo": {
//!         "applicationId": "17c82e96-be03-4f38-aef3-f83d48582d97",
//!         "deviceName": "Test device",
//!         "devEui": "0101010101010101"
//!     },
//!     "devAddr": "00189440",
//!     "dr": 1,
//!     "fPort": 1,
//!     "data": "qg==",
//!     "rxInfo": [{ "gatewayId": "0016c001f153a14c", "rssi": -36, "snr": 10.5 }],
//!     "txInfo": { "frequency": 867100000 }
//! }"#;
//!
//! let event = UplinkEvent::from_json(json)?;
//! assert_eq!(event.dev_eui.to_hex(), "0101010101010101");
//! assert_eq!(event.fport, Some(1));
//! assert_eq!(event.data, [0xAA]);
//! assert_eq!(event.fcnt, 0, "left out because it is zero");
//! assert_eq!(event.best_reception().map(|heard| heard.rssi_dbm), Some(-36));
//! # Ok::<(), pamoja_gateway::chirpstack::EventError>(())
//! ```

use std::fmt;

use serde_json::{Map, Value};

use crate::base64;
use crate::udp::Eui;

/// The topic filter for every uplink event of every application on a server.
pub const UPLINK_TOPIC: &str = "application/+/device/+/event/up";

/// Returns the topic filter for one application's uplink events.
///
/// # Arguments
///
/// * `application_id` - the application's identifier, as ChirpStack shows it.
///
/// # Returns
///
/// The filter, with every device of the application matched.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::chirpstack::uplink_topic;
///
/// assert_eq!(
///     uplink_topic("17c82e96-be03-4f38-aef3-f83d48582d97"),
///     "application/17c82e96-be03-4f38-aef3-f83d48582d97/device/+/event/up"
/// );
/// ```
pub fn uplink_topic(application_id: &str) -> String {
    format!("application/{application_id}/device/+/event/up")
}

/// One gateway's reception of an uplink.
#[derive(Clone, Debug, PartialEq)]
pub struct Reception {
    /// The gateway that heard it.
    pub gateway: Eui,
    /// The signal strength it heard, in dBm.
    pub rssi_dbm: i32,
    /// The signal-to-noise ratio it measured, in dB.
    pub snr_db: f32,
}

/// An uplink as a ChirpStack server published it.
#[derive(Clone, Debug, PartialEq)]
pub struct UplinkEvent {
    /// The identifier ChirpStack gave the uplink once it had deduplicated it across gateways.
    pub deduplication_id: String,
    /// When the server received it, as RFC 3339 text, if the event carried the time.
    pub time: Option<String>,
    /// The application the device belongs to.
    pub application_id: String,
    /// The device's name on the server.
    pub device_name: String,
    /// The device's identifier.
    pub dev_eui: Eui,
    /// The device's address in its current session.
    pub dev_addr: Option<u32>,
    /// Whether the uplink had adaptive data rate on.
    pub adr: bool,
    /// The data rate it arrived at, as the region numbers them.
    pub data_rate: u8,
    /// The uplink frame counter.
    pub fcnt: u32,
    /// The application port, or `None` for an uplink the server reports without one.
    pub fport: Option<u8>,
    /// Whether the device asked for an acknowledgment.
    pub confirmed: bool,
    /// The decrypted application payload.
    pub data: Vec<u8>,
    /// The frequency it arrived on, in hertz.
    pub frequency_hz: Option<u32>,
    /// Every gateway that heard it.
    pub receptions: Vec<Reception>,
}

/// Why a message is not an uplink event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventError {
    /// The message is not JSON, or its top level is not an object.
    Json(String),
    /// A field the event always carries is not there.
    Missing(&'static str),
    /// A field is there but does not hold what it should.
    Invalid(&'static str),
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventError::Json(why) => write!(f, "the event is not a JSON object: {why}"),
            EventError::Missing(field) => write!(f, "the event has no {field}"),
            EventError::Invalid(field) => write!(f, "the event's {field} is not valid"),
        }
    }
}

impl std::error::Error for EventError {}

impl UplinkEvent {
    /// Reads an uplink event.
    ///
    /// # Arguments
    ///
    /// * `text` - the MQTT message's payload.
    ///
    /// # Returns
    ///
    /// The event.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::Json`] for text that is not a JSON object,
    /// [`EventError::Missing`] without `deviceInfo.devEui`, and [`EventError::Invalid`] for a
    /// field of the wrong kind: an identifier that is not hexadecimal, a payload that is not
    /// base64, or a number out of its range.
    pub fn from_json(text: &str) -> Result<UplinkEvent, EventError> {
        let value: Value =
            serde_json::from_str(text).map_err(|error| EventError::Json(error.to_string()))?;
        let event = value
            .as_object()
            .ok_or_else(|| EventError::Json("the top level is not an object".to_owned()))?;
        let empty = Map::new();
        let device = match event.get("deviceInfo") {
            Some(Value::Object(device)) => device,
            Some(_) => return Err(EventError::Invalid("deviceInfo")),
            None => &empty,
        };

        let dev_eui = text_field(device, "devEui")
            .ok_or(EventError::Missing("deviceInfo.devEui"))
            .and_then(|hex| Eui::from_hex(&hex).ok_or(EventError::Invalid("deviceInfo.devEui")))?;
        let dev_addr = match text_field(event, "devAddr") {
            Some(hex) => {
                Some(u32::from_str_radix(&hex, 16).map_err(|_| EventError::Invalid("devAddr"))?)
            }
            None => None,
        };
        let data = match text_field(event, "data") {
            Some(encoded) => base64::decode(&encoded).map_err(|_| EventError::Invalid("data"))?,
            None => Vec::new(),
        };

        let mut receptions = Vec::new();
        if let Some(Value::Array(heard)) = event.get("rxInfo") {
            for entry in heard {
                let Some(entry) = entry.as_object() else {
                    return Err(EventError::Invalid("rxInfo"));
                };
                let gateway = text_field(entry, "gatewayId")
                    .and_then(|hex| Eui::from_hex(&hex))
                    .ok_or(EventError::Invalid("rxInfo.gatewayId"))?;
                receptions.push(Reception {
                    gateway,
                    rssi_dbm: whole(entry, "rssi", "rxInfo.rssi")?.unwrap_or(0),
                    snr_db: entry.get("snr").and_then(Value::as_f64).unwrap_or(0.0) as f32,
                });
            }
        }

        let frequency_hz = match event.get("txInfo") {
            Some(Value::Object(transmitted)) => {
                whole(transmitted, "frequency", "txInfo.frequency")?
            }
            _ => None,
        };

        Ok(UplinkEvent {
            deduplication_id: text_field(event, "deduplicationId").unwrap_or_default(),
            time: text_field(event, "time"),
            application_id: text_field(device, "applicationId").unwrap_or_default(),
            device_name: text_field(device, "deviceName").unwrap_or_default(),
            dev_eui,
            dev_addr,
            adr: event.get("adr").and_then(Value::as_bool).unwrap_or(false),
            data_rate: whole(event, "dr", "dr")?.unwrap_or(0),
            fcnt: whole(event, "fCnt", "fCnt")?.unwrap_or(0),
            fport: whole(event, "fPort", "fPort")?,
            confirmed: event
                .get("confirmed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            data,
            frequency_hz,
            receptions,
        })
    }

    /// Returns the gateway that heard the uplink best.
    ///
    /// # Returns
    ///
    /// The reception with the highest signal-to-noise ratio, or `None` for an event that
    /// named no gateway.
    pub fn best_reception(&self) -> Option<&Reception> {
        self.receptions
            .iter()
            .max_by(|one, other| one.snr_db.total_cmp(&other.snr_db))
    }
}

fn text_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Reads a whole number into a type, where it is present.
fn whole<T: TryFrom<i64>>(
    object: &Map<String, Value>,
    key: &str,
    field: &'static str,
) -> Result<Option<T>, EventError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .and_then(|number| T::try_from(number).ok())
            .map(Some)
            .ok_or(EventError::Invalid(field)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The uplink event on ChirpStack's integration events documentation page, as published.
    const DOCUMENTED: &str = r#"{
	"deduplicationId": "3ac7e3c4-4401-4b8d-9386-a5c902f9202d",
	"time": "2022-07-18T09:34:15.775023242+00:00",
	"deviceInfo": {
		"tenantId": "52f14cd4-c6f1-4fbd-8f87-4025e1d49242",
		"tenantName": "ChirpStack",
		"applicationId": "17c82e96-be03-4f38-aef3-f83d48582d97",
		"applicationName": "Test application",
		"deviceProfileId": "14855bf7-d10d-4aee-b618-ebfcb64dc7ad",
		"deviceProfileName": "Test device-profile",
		"deviceName": "Test device",
		"devEui": "0101010101010101",
		"tags": {
			"key": "value"
		}
	},
	"devAddr": "00189440",
	"dr": 1,
	"fPort": 1,
	"data": "qg==",
	"rxInfo": [{
		"gatewayId": "0016c001f153a14c",
		"uplinkId": 4217106255,
		"rssi": -36,
		"snr": 10.5,
		"context": "E3OWOQ==",
		"metadata": {
			"region_name": "eu868",
			"region_common_name": "EU868"
		}
	}],
	"txInfo": {
		"frequency": 867100000,
		"modulation": {
			"lora": {
				"bandwidth": 125000,
				"spreadingFactor": 11,
				"codeRate": "CR_4_5"
			}
		}
	}
}"#;

    #[test]
    fn the_documented_event_reads_field_for_field() {
        let event = UplinkEvent::from_json(DOCUMENTED).expect("reads");
        assert_eq!(
            event.deduplication_id,
            "3ac7e3c4-4401-4b8d-9386-a5c902f9202d"
        );
        assert_eq!(
            event.time.as_deref(),
            Some("2022-07-18T09:34:15.775023242+00:00")
        );
        assert_eq!(event.application_id, "17c82e96-be03-4f38-aef3-f83d48582d97");
        assert_eq!(event.device_name, "Test device");
        assert_eq!(event.dev_eui, Eui::new([1; 8]));
        assert_eq!(event.dev_addr, Some(0x0018_9440));
        assert_eq!(event.data_rate, 1);
        assert_eq!(event.fport, Some(1));
        assert_eq!(event.data, [0xAA]);
        assert_eq!(event.frequency_hz, Some(867_100_000));
        assert_eq!(
            event.receptions,
            [Reception {
                gateway: Eui::new([0x00, 0x16, 0xC0, 0x01, 0xF1, 0x53, 0xA1, 0x4C]),
                rssi_dbm: -36,
                snr_db: 10.5,
            }]
        );
    }

    #[test]
    fn fields_left_at_their_defaults_read_as_zero_and_false() {
        // Protobuf's JSON mapping leaves them out, which is why the documented event has no
        // fCnt, adr or confirmed.
        let event = UplinkEvent::from_json(DOCUMENTED).expect("reads");
        assert_eq!(event.fcnt, 0);
        assert!(!event.adr);
        assert!(!event.confirmed);

        let later = DOCUMENTED.replacen(
            r#""dr": 1,"#,
            r#""dr": 1, "fCnt": 70000, "adr": true, "confirmed": true,"#,
            1,
        );
        let event = UplinkEvent::from_json(&later).expect("reads");
        assert_eq!(event.fcnt, 70_000);
        assert!(event.adr && event.confirmed);
    }

    #[test]
    fn the_best_reception_is_the_clearest_one() {
        let two = DOCUMENTED.replacen(
            r#""rxInfo": [{"#,
            r#""rxInfo": [{ "gatewayId": "0202020202020202", "rssi": -110, "snr": -4.25 }, {"#,
            1,
        );
        let event = UplinkEvent::from_json(&two).expect("reads");
        assert_eq!(event.receptions.len(), 2);
        assert_eq!(
            event.best_reception().map(|heard| heard.gateway.to_hex()),
            Some("0016c001f153a14c".to_owned())
        );
    }

    #[test]
    fn an_event_without_its_device_or_with_a_bad_field_is_refused() {
        assert_eq!(
            UplinkEvent::from_json(r#"{"fPort": 2}"#),
            Err(EventError::Missing("deviceInfo.devEui"))
        );
        assert_eq!(
            UplinkEvent::from_json(&DOCUMENTED.replacen("qg==", "q", 1)),
            Err(EventError::Invalid("data"))
        );
        assert_eq!(
            UplinkEvent::from_json(&DOCUMENTED.replacen(r#""fPort": 1"#, r#""fPort": 300"#, 1)),
            Err(EventError::Invalid("fPort"))
        );
        assert!(matches!(
            UplinkEvent::from_json("[]"),
            Err(EventError::Json(_))
        ));
    }

    #[test]
    fn the_topic_filters_match_what_chirpstack_publishes_on() {
        assert_eq!(UPLINK_TOPIC, "application/+/device/+/event/up");
        assert!(uplink_topic("app").ends_with("/device/+/event/up"));
    }
}
