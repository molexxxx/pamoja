//! Generated Node bindings for the uplink events a ChirpStack network server publishes.
//!
//! ChirpStack publishes every uplink it deduplicates to an MQTT topic as the JSON form of its
//! `UplinkEvent` message. These read one into the device, the counter, the payload and every
//! gateway that heard it, so an application that subscribes needs no protobuf or base64 of its
//! own.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_gateway::chirpstack::{uplink_topic, UplinkEvent, UPLINK_TOPIC};

/// The MQTT topic every application's uplink events are published on, with wildcards in place
/// of the application and the device.
#[napi]
pub const CHIRPSTACK_UPLINK_TOPIC: &str = UPLINK_TOPIC;

/// One gateway that heard an uplink.
#[napi(object)]
pub struct ChirpstackReception {
    /// The gateway's EUI, as lowercase hex.
    pub gateway: String,
    /// The received signal strength, in dBm.
    pub rssi_dbm: i32,
    /// The signal-to-noise ratio, in dB.
    pub snr_db: f64,
}

/// An uplink as ChirpStack reports it.
#[napi(object)]
pub struct ChirpstackUplinkEvent {
    /// The identifier ChirpStack gave the uplink once it deduplicated the gateways' copies.
    pub deduplication_id: String,
    /// When the uplink was received, as ChirpStack wrote it, or `null`.
    pub time: Option<String>,
    /// The application the device belongs to.
    pub application_id: String,
    /// The name the device was given in ChirpStack.
    pub device_name: String,
    /// The device EUI, as lowercase hex.
    pub dev_eui: String,
    /// The device's address, or `null` when the event names none.
    pub dev_addr: Option<u32>,
    /// Whether the device had adaptive data rate on.
    pub adr: bool,
    /// The data rate, as the region numbers them.
    pub data_rate: u8,
    /// The uplink frame counter.
    pub fcnt: u32,
    /// The application port, or `null` for a frame that carried none.
    pub fport: Option<u8>,
    /// Whether the uplink was confirmed.
    pub confirmed: bool,
    /// The application payload, decoded from base64.
    pub data: Buffer,
    /// The carrier it was heard on, in hertz, or `null`.
    pub frequency_hz: Option<u32>,
    /// Every gateway that heard it.
    pub receptions: Vec<ChirpstackReception>,
    /// The position in `receptions` of the gateway that heard it with the highest
    /// signal-to-noise ratio, or `null` when none did.
    pub best_reception: Option<u32>,
}

/// Reads an uplink event from the JSON ChirpStack published.
///
/// Fields protobuf's JSON mapping leaves out when they hold their default read as that
/// default: a frame counter of zero, ADR off, unconfirmed. Throws for text that is not a JSON
/// object, an event with no device EUI, or a field that does not read as what it should.
#[napi(js_name = "chirpstackUplinkFromJson")]
pub fn chirpstack_uplink_from_json(text: String) -> napi::Result<ChirpstackUplinkEvent> {
    let event = UplinkEvent::from_json(&text)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    let best_reception = event.best_reception().and_then(|best| {
        event
            .receptions
            .iter()
            .position(|reception| std::ptr::eq(reception, best))
            .map(|index| index as u32)
    });
    Ok(ChirpstackUplinkEvent {
        receptions: event
            .receptions
            .iter()
            .map(|reception| ChirpstackReception {
                gateway: reception.gateway.to_hex(),
                rssi_dbm: reception.rssi_dbm,
                snr_db: f64::from(reception.snr_db),
            })
            .collect(),
        best_reception,
        deduplication_id: event.deduplication_id,
        time: event.time,
        application_id: event.application_id,
        device_name: event.device_name,
        dev_eui: event.dev_eui.to_hex(),
        dev_addr: event.dev_addr,
        adr: event.adr,
        data_rate: event.data_rate,
        fcnt: event.fcnt,
        fport: event.fport,
        confirmed: event.confirmed,
        data: event.data.into(),
        frequency_hz: event.frequency_hz,
    })
}

/// Builds the MQTT topic an application's uplink events are published on, with a wildcard in
/// place of the device: `application/<id>/device/+/event/up`.
#[napi(js_name = "chirpstackUplinkTopic")]
pub fn chirpstack_uplink_topic(application_id: String) -> String {
    uplink_topic(&application_id)
}
