//! Generated Node bindings for LoRaWAN 1.0.x MAC framing.
//!
//! These mirror the `pamoja-lorawan` Rust API: the secured frame a long-range node
//! puts on the air, and the over-the-air activation that hands it its session
//! keys.
//!
//! A session and a device hold key material, so they are classes and the keys
//! never come back out. An encoded frame crosses as the buffer to transmit, and a
//! decoded one as a plain object carrying its header fields and its recovered
//! payload.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_lorawan::{
    Device as CoreDevice, Direction, Downlink, JoinAccept as CoreJoinAccept, LorawanError, RxData,
    Session as CoreSession, Uplink,
};

/// The largest application payload, in bytes, a single frame can carry.
#[napi]
pub const LORAWAN_MAX_PAYLOAD: u32 = pamoja_lorawan::MAX_PAYLOAD as u32;

/// The largest LoRaWAN frame, in bytes, this build accepts.
#[napi]
pub const LORAWAN_MAX_FRAME: u32 = pamoja_lorawan::MAX_FRAME as u32;

/// The direction a frame travelled, which its MIC and encryption both fold in.
#[napi(string_enum)]
pub enum LorawanDirection {
    /// From an end device up to the network.
    Uplink,
    /// From the network down to an end device.
    Downlink,
}

/// The header flags and frame options a sender sets on a data frame.
///
/// Every field is optional and defaults off. `fpending` applies to a downlink
/// only and is ignored on an uplink.
#[napi(object)]
pub struct LorawanOptions {
    /// Ask the far end to acknowledge this frame.
    pub confirmed: Option<bool>,
    /// Mark the frame as taking part in adaptive data rate.
    pub adr: Option<bool>,
    /// Acknowledge the last confirmed frame from the far end.
    pub ack: Option<bool>,
    /// Tell the device more downlink data is waiting.
    pub fpending: Option<bool>,
    /// MAC commands to carry in the header, at most 15 bytes.
    pub fopts: Option<Buffer>,
}

/// A decoded data frame, with its payload decrypted.
#[napi(object)]
pub struct LorawanRxData {
    /// The direction the frame travelled.
    pub direction: LorawanDirection,
    /// The device address the frame carries.
    pub dev_addr: u32,
    /// The low 16 bits of the frame counter.
    pub fcnt: u16,
    /// Whether the frame asks to be acknowledged.
    pub confirmed: bool,
    /// Whether the frame takes part in adaptive data rate.
    pub adr: bool,
    /// Whether the frame acknowledges the last confirmed one.
    pub ack: bool,
    /// Whether the network has more downlink data waiting.
    pub fpending: bool,
    /// The port the frame was sent on, or `null` when it carries only options.
    pub fport: Option<u8>,
    /// The MAC commands the header carried.
    pub fopts: Buffer,
    /// The decrypted application payload.
    pub payload: Buffer,
}

/// An activated LoRaWAN session: a device address and its two session keys.
#[napi]
pub struct LorawanSession {
    inner: CoreSession,
}

#[napi]
impl LorawanSession {
    /// Creates a session from a device address and its two 16-byte session keys.
    ///
    /// `nwkSKey` authenticates frames and `appSKey` encrypts payloads.
    #[napi(constructor)]
    pub fn new(dev_addr: u32, nwk_skey: Buffer, app_skey: Buffer) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreSession::new(
                dev_addr,
                key(&nwk_skey, "nwkSKey")?,
                key(&app_skey, "appSKey")?,
            ),
        })
    }

    /// The device address this session is bound to.
    #[napi(getter)]
    pub fn dev_addr(&self) -> u32 {
        self.inner.dev_addr()
    }

    /// Encodes an uplink, encrypting the payload and appending the MIC.
    #[napi]
    pub fn encode_uplink(
        &self,
        fcnt: u32,
        fport: u8,
        payload: Buffer,
        options: Option<LorawanOptions>,
    ) -> napi::Result<Buffer> {
        let options = options.unwrap_or_else(none);
        let fopts = options.fopts.as_ref().map(Buffer::as_ref).unwrap_or(&[]);
        let mut uplink = Uplink::new(fcnt, fport, payload.as_ref()).with_fopts(fopts);
        if options.confirmed.unwrap_or(false) {
            uplink = uplink.confirmed();
        }
        if options.adr.unwrap_or(false) {
            uplink = uplink.with_adr();
        }
        if options.ack.unwrap_or(false) {
            uplink = uplink.with_ack();
        }
        self.inner
            .encode_uplink(&uplink)
            .map(|frame| frame.as_bytes().to_vec().into())
            .map_err(to_napi)
    }

    /// Encodes a downlink, encrypting the payload and appending the MIC.
    #[napi]
    pub fn encode_downlink(
        &self,
        fcnt: u32,
        fport: u8,
        payload: Buffer,
        options: Option<LorawanOptions>,
    ) -> napi::Result<Buffer> {
        let options = options.unwrap_or_else(none);
        let fopts = options.fopts.as_ref().map(Buffer::as_ref).unwrap_or(&[]);
        let mut downlink = Downlink::new(fcnt, fport, payload.as_ref()).with_fopts(fopts);
        if options.confirmed.unwrap_or(false) {
            downlink = downlink.confirmed();
        }
        if options.adr.unwrap_or(false) {
            downlink = downlink.with_adr();
        }
        if options.ack.unwrap_or(false) {
            downlink = downlink.with_ack();
        }
        if options.fpending.unwrap_or(false) {
            downlink = downlink.with_fpending();
        }
        self.inner
            .encode_downlink(&downlink)
            .map(|frame| frame.as_bytes().to_vec().into())
            .map_err(to_napi)
    }

    /// Verifies a received frame, then decrypts it.
    ///
    /// `fcnt` is the full 32-bit counter expected for this frame; its low 16 bits
    /// must match the counter the frame carries.
    #[napi]
    pub fn decode(&self, bytes: Buffer, fcnt: u32) -> napi::Result<LorawanRxData> {
        self.inner
            .decode(bytes.as_ref(), fcnt)
            .map(describe)
            .map_err(to_napi)
    }
}

/// The root credentials over-the-air activation is built on.
#[napi]
pub struct LorawanDevice {
    inner: CoreDevice,
}

#[napi]
impl LorawanDevice {
    /// Creates a device from its two 8-byte EUIs and its 16-byte application key.
    #[napi(constructor)]
    pub fn new(dev_eui: Buffer, app_eui: Buffer, app_key: Buffer) -> napi::Result<Self> {
        Ok(Self {
            inner: CoreDevice::new(
                eui(&dev_eui, "devEui")?,
                eui(&app_eui, "appEui")?,
                key(&app_key, "appKey")?,
            ),
        })
    }

    /// Builds the join request this device broadcasts to activate.
    ///
    /// `devNonce` must never repeat for a device, since the network rejects a
    /// replayed one.
    #[napi]
    pub fn join_request(&self, dev_nonce: u16) -> Buffer {
        self.inner
            .join_request(dev_nonce)
            .as_bytes()
            .to_vec()
            .into()
    }

    /// Turns the join accept a network sent into the settings it grants.
    ///
    /// `devNonce` is the nonce the matching join request carried.
    #[napi]
    pub fn accept_join(&self, bytes: Buffer, dev_nonce: u16) -> napi::Result<LorawanJoinAccept> {
        self.inner
            .accept_join(bytes.as_ref(), dev_nonce)
            .map(|accept| LorawanJoinAccept { inner: accept })
            .map_err(to_napi)
    }
}

/// An accepted join: the network settings, and the session it grants.
#[napi]
pub struct LorawanJoinAccept {
    inner: CoreJoinAccept,
}

#[napi]
impl LorawanJoinAccept {
    /// The device address the network assigned.
    #[napi(getter)]
    pub fn dev_addr(&self) -> u32 {
        self.inner.dev_addr()
    }

    /// The identifier of the network that accepted the join.
    #[napi(getter)]
    pub fn net_id(&self) -> u32 {
        self.inner.net_id()
    }

    /// The downlink settings byte, carrying the second receive window data rate
    /// and the first window offset.
    #[napi(getter)]
    pub fn dl_settings(&self) -> u8 {
        self.inner.dl_settings()
    }

    /// The delay before the first receive window, in seconds.
    #[napi(getter)]
    pub fn rx_delay(&self) -> u8 {
        self.inner.rx_delay()
    }

    /// The activated session this join grants, with its keys already derived.
    #[napi]
    pub fn session(&self) -> LorawanSession {
        LorawanSession {
            inner: self.inner.session(),
        }
    }
}

/// Every option off, the default for a plain unconfirmed frame.
fn none() -> LorawanOptions {
    LorawanOptions {
        confirmed: None,
        adr: None,
        ack: None,
        fpending: None,
        fopts: None,
    }
}

/// Reads every field off a decoded frame into the object JavaScript receives.
fn describe(rx: RxData) -> LorawanRxData {
    LorawanRxData {
        direction: match rx.direction() {
            Direction::Uplink => LorawanDirection::Uplink,
            Direction::Downlink => LorawanDirection::Downlink,
        },
        dev_addr: rx.dev_addr(),
        fcnt: rx.fcnt(),
        confirmed: rx.confirmed(),
        adr: rx.adr(),
        ack: rx.ack(),
        fpending: rx.fpending(),
        fport: rx.fport(),
        fopts: rx.fopts().to_vec().into(),
        payload: rx.payload().to_vec().into(),
    }
}

/// Copies a 16-byte key, rejecting anything else.
fn key(bytes: &Buffer, what: &str) -> napi::Result<[u8; 16]> {
    <[u8; 16]>::try_from(bytes.as_ref())
        .map_err(|_| napi::Error::from_reason(format!("{what} must be exactly 16 bytes")))
}

/// Copies an 8-byte EUI, rejecting anything else.
fn eui(bytes: &Buffer, what: &str) -> napi::Result<[u8; 8]> {
    <[u8; 8]>::try_from(bytes.as_ref())
        .map_err(|_| napi::Error::from_reason(format!("{what} must be exactly 8 bytes")))
}

/// Turns a LoRaWAN error into the JavaScript error a caller sees.
fn to_napi(error: LorawanError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
