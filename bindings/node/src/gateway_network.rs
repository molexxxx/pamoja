//! Generated Node bindings for the network side of one site.
//!
//! A gateway forwards packets without reading them, because it holds no keys. This is the
//! other half: a [`GatewayNetwork`] holds the devices it admits, the sessions it has granted
//! and the counters it has seen, so a packet handed to it comes back as one of three things.
//! A device joined and its accept is ready to transmit, a session frame arrived and was
//! decrypted, or the frame belongs to a network this site never granted, which a gateway
//! hears all the time and is not an error.
//!
//! Where and when to answer comes back with the event, so a downlink goes out in the window
//! the uplink opened without any of it being worked out by hand.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_gateway::network::{Event, Network, Registration, Rx1Channels, Slot, Windows};
use pamoja_lora::region::ChannelBlock;

use crate::gateway::{rxpk_of, txpk_to_js, GatewayRxpk, GatewayTxpk};
use crate::lora::{lora_link_of, settings, LoraLink};
use crate::lora_region::LoraChannelPlan;

/// What a forwarded packet turned out to be.
#[napi(string_enum, js_name = "GatewayNetworkOutcome")]
pub enum GatewayNetworkOutcome {
    /// A device joined, and its accept is ready to transmit.
    Joined,
    /// A session frame arrived, decrypted.
    Data,
    /// The frame belongs to a device this site never granted.
    Foreign,
}

/// The channels the first receive window answers on, which the region decides.
#[napi(string_enum, js_name = "GatewayRx1Channels")]
pub enum GatewayRx1Channels {
    /// The window answers on the frequency the uplink arrived on.
    SameAsUplink,
    /// The window answers on a run of downlink channels, chosen by the uplink channel number
    /// modulo how many the run holds.
    Downstream,
}

/// When and where a network answers, and at what rate.
#[napi(object, js_name = "GatewayNetworkWindows")]
pub struct GatewayNetworkWindows {
    /// The delay before the first receive window, in microseconds; one second by default.
    pub receive_delay_us: Option<u32>,
    /// The delay before the window a join accept is sent in; five seconds by default.
    pub join_delay_us: Option<u32>,
    /// The offset between the uplink data rate and the rate the first window answers at.
    pub rx1_data_rate_offset: Option<u8>,
    /// Which channels the first window answers on; the uplink frequency by default.
    pub rx1_channels: Option<GatewayRx1Channels>,
    /// The first downlink channel, in hertz, when the channels are downstream.
    pub downstream_start_hz: Option<u32>,
    /// The spacing between those channels, in hertz.
    pub downstream_step_hz: Option<u32>,
    /// How many there are.
    pub downstream_count: Option<u16>,
}

/// Where and when a downlink answers an uplink, in the concentrator's own terms.
#[napi(object, js_name = "GatewaySlot")]
pub struct GatewaySlot {
    /// The concentrator timestamp to transmit at, in microseconds.
    pub timestamp_us: u32,
    /// The frequency to transmit on, in hertz.
    pub frequency_hz: u32,
    /// The settings to transmit with.
    pub link: LoraLink,
}

/// What a forwarded packet turned out to be, and where its answer goes.
#[napi(object, js_name = "GatewayNetworkEvent")]
pub struct GatewayNetworkEvent {
    /// Which of the three this was.
    pub outcome: GatewayNetworkOutcome,
    /// The device that joined, as sixteen hexadecimal digits.
    pub dev_eui: Option<String>,
    /// The address granted, or the address a frame claimed.
    pub dev_addr: u32,
    /// The counter the frame carried, reconstructed to its full width.
    pub fcnt: Option<u32>,
    /// The port the frame was sent on, absent for a frame carrying only options.
    pub fport: Option<u8>,
    /// What the device sent, decrypted.
    pub payload: Option<Buffer>,
    /// Whether the device asked to be acknowledged.
    pub confirmed: Option<bool>,
    /// Where an answer goes, for a join or for data.
    pub slot: Option<GatewaySlot>,
    /// The packet carrying the accept, for a join.
    pub accept: Option<GatewayTxpk>,
}

/// The network side of one site: what a server does with what a gateway forwarded.
#[napi(js_name = "GatewayNetwork")]
pub struct GatewayNetwork {
    inner: Network,
}

#[napi]
impl GatewayNetwork {
    /// Opens the network side of a site on a channel plan.
    ///
    /// The plan is copied into the network, so it holds its band for as long as it runs.
    #[napi(constructor)]
    pub fn new(
        plan: &LoraChannelPlan,
        net_id: u32,
        windows: Option<GatewayNetworkWindows>,
        first_dev_addr: Option<u32>,
    ) -> Self {
        let mut network = plan.with(|plan| Network::new(plan, net_id));
        if let Some(windows) = windows {
            network = network.with_windows(windows_of(windows));
        }
        if let Some(dev_addr) = first_dev_addr {
            network = network.with_first_dev_addr(dev_addr);
        }
        Self { inner: network }
    }

    /// Admits a device, so a join request signed with its key is accepted.
    #[napi]
    pub fn register(
        &mut self,
        dev_eui: Buffer,
        app_eui: Buffer,
        app_key: Buffer,
    ) -> napi::Result<()> {
        let dev_eui = eight(&dev_eui, "devEui")?;
        let app_eui = eight(&app_eui, "appEui")?;
        let app_key = sixteen(&app_key)?;
        self.inner
            .register(Registration::new(dev_eui, app_eui, app_key));
        Ok(())
    }

    /// Reads a packet the gateway forwarded.
    #[napi]
    pub fn uplink(&mut self, heard: GatewayRxpk) -> napi::Result<GatewayNetworkEvent> {
        let event = self
            .inner
            .uplink(&rxpk_of(heard))
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        Ok(event_to_js(event))
    }

    /// Builds a downlink for a device, encrypted with its session.
    #[napi]
    pub fn answer(
        &mut self,
        dev_addr: u32,
        slot: GatewaySlot,
        fport: u8,
        payload: Buffer,
    ) -> napi::Result<GatewayTxpk> {
        let downlink = self
            .inner
            .answer(dev_addr, slot_of(slot), fport, &payload)
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        Ok(txpk_to_js(&downlink))
    }
}

/// Reads the windows a network answers in.
fn windows_of(windows: GatewayNetworkWindows) -> Windows {
    let mut built = Windows::new();
    if let Some(micros) = windows.receive_delay_us {
        built = built.with_receive_delay_us(micros);
    }
    if let Some(micros) = windows.join_delay_us {
        built = built.with_join_delay_us(micros);
    }
    if let Some(offset) = windows.rx1_data_rate_offset {
        built = built.with_rx1_data_rate_offset(offset);
    }
    if let Some(GatewayRx1Channels::Downstream) = windows.rx1_channels {
        built = built.with_rx1_channels(Rx1Channels::Downstream(ChannelBlock::new(
            windows.downstream_start_hz.unwrap_or(0),
            windows.downstream_step_hz.unwrap_or(0),
            windows.downstream_count.unwrap_or(0),
            0,
            0,
        )));
    }
    built
}

/// Reads a window from JavaScript.
fn slot_of(slot: GatewaySlot) -> Slot {
    Slot {
        timestamp_us: slot.timestamp_us,
        frequency_hz: slot.frequency_hz,
        link: settings(&slot.link),
    }
}

/// Writes a window to JavaScript.
fn slot_to_js(slot: Slot) -> GatewaySlot {
    GatewaySlot {
        timestamp_us: slot.timestamp_us,
        frequency_hz: slot.frequency_hz,
        link: lora_link_of(slot.link),
    }
}

/// Writes an event to JavaScript.
fn event_to_js(event: Event) -> GatewayNetworkEvent {
    match event {
        Event::Joined {
            dev_eui,
            dev_addr,
            accept,
        } => GatewayNetworkEvent {
            outcome: GatewayNetworkOutcome::Joined,
            dev_eui: Some(hex(&dev_eui)),
            dev_addr,
            fcnt: None,
            fport: None,
            payload: None,
            confirmed: None,
            slot: None,
            accept: Some(txpk_to_js(&accept)),
        },
        Event::Data {
            dev_addr,
            fcnt,
            fport,
            payload,
            confirmed,
            slot,
        } => GatewayNetworkEvent {
            outcome: GatewayNetworkOutcome::Data,
            dev_eui: None,
            dev_addr,
            fcnt: Some(fcnt),
            fport,
            payload: Some(Buffer::from(payload)),
            confirmed: Some(confirmed),
            slot: Some(slot_to_js(slot)),
            accept: None,
        },
        Event::Foreign { dev_addr } => GatewayNetworkEvent {
            outcome: GatewayNetworkOutcome::Foreign,
            dev_eui: None,
            dev_addr,
            fcnt: None,
            fport: None,
            payload: None,
            confirmed: None,
            slot: None,
            accept: None,
        },
    }
}

/// Writes bytes as hexadecimal, the way an identifier is written down.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Reads an eight-byte identifier.
fn eight(bytes: &[u8], name: &str) -> napi::Result<[u8; 8]> {
    bytes.try_into().map_err(|_| {
        napi::Error::from_reason(format!("{name} must be eight bytes, not {}", bytes.len()))
    })
}

/// Reads a sixteen-byte key.
fn sixteen(bytes: &[u8]) -> napi::Result<[u8; 16]> {
    bytes.try_into().map_err(|_| {
        napi::Error::from_reason(format!("appKey must be sixteen bytes, not {}", bytes.len()))
    })
}
