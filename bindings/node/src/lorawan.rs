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
use pamoja_lorawan::mac::{MacCommand, MacCommands};
use pamoja_lorawan::{
    Device as CoreDevice, Direction, Downlink, FrameHeader, JoinAccept as CoreJoinAccept,
    JoinGrant, JoinRequest, LorawanError, MessageType, RxData, Session as CoreSession, Uplink,
};

/// The largest application payload, in bytes, a single frame can carry.
#[napi]
pub const LORAWAN_MAX_PAYLOAD: u32 = pamoja_lorawan::MAX_PAYLOAD as u32;

/// The largest LoRaWAN frame, in bytes, this build accepts.
#[napi]
pub const LORAWAN_MAX_FRAME: u32 = pamoja_lorawan::MAX_FRAME as u32;

/// The direction a frame traveled, which its MIC and encryption both fold in.
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
    /// The direction the frame traveled.
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

/// What kind of message a frame is, read from its header.
#[napi(string_enum)]
pub enum LorawanMessageType {
    /// A device asking to join a network.
    JoinRequest,
    /// A network admitting a device.
    JoinAccept,
    /// Data from a device that does not need acknowledging.
    UnconfirmedUp,
    /// Data from a device that asks to be acknowledged.
    ConfirmedUp,
    /// Data to a device that does not need acknowledging.
    UnconfirmedDown,
    /// Data to a device that asks to be acknowledged.
    ConfirmedDown,
}

/// What a frame says about itself before any key is involved.
///
/// Nothing here is authenticated, since checking the MIC needs the session key.
/// Treat it as a routing hint until `decode` has verified the frame.
#[napi(object)]
pub struct LorawanHeader {
    /// What kind of message the frame is.
    pub message_type: LorawanMessageType,
    /// Whether this is a data frame rather than part of a join exchange.
    pub is_data: bool,
    /// The device address, or `null` for a join frame.
    pub dev_addr: Option<u32>,
    /// The low 16 bits of the frame counter, or `null` for a join frame.
    pub fcnt: Option<u16>,
    /// The port, or `null` for a join frame or one carrying only options.
    pub fport: Option<u8>,
    /// Whether the frame asks to be acknowledged.
    pub confirmed: bool,
    /// Whether the frame takes part in adaptive data rate.
    pub adr: bool,
    /// Whether the frame acknowledges the last confirmed one.
    pub ack: bool,
    /// Whether the network has more downlink data waiting.
    pub fpending: bool,
    /// How many bytes of frame options the header carries.
    pub fopts_len: u32,
    /// The length of the still-encrypted payload.
    pub payload_len: u32,
}

/// A join-request a device broadcast, with its integrity already verified.
#[napi(object)]
pub struct LorawanJoinRequest {
    /// The device identifier, most-significant byte first.
    pub dev_eui: Buffer,
    /// The application identifier, most-significant byte first.
    pub app_eui: Buffer,
    /// The nonce the request carried, which a network must not accept twice.
    pub dev_nonce: u16,
}

/// What a network grants a device that joined.
#[napi(object)]
pub struct LorawanGrant {
    /// A nonce this network must not reuse for the device; low 24 bits only.
    pub app_nonce: u32,
    /// The network identifier; low 24 bits only.
    pub net_id: u32,
    /// The address to assign the device.
    pub dev_addr: u32,
    /// The downlink settings byte, defaulting to 0.
    pub dl_settings: Option<u8>,
    /// The delay before the first receive window in seconds, defaulting to 0.
    pub rx_delay: Option<u8>,
    /// The optional 16-byte channel list.
    pub cflist: Option<Buffer>,
}

/// Reads a frame far enough to route it, without any key.
///
/// A receiver holding many sessions uses this to find which one a frame belongs
/// to: the device address travels in the clear.
#[napi]
pub fn lorawan_parse_header(bytes: Buffer) -> napi::Result<LorawanHeader> {
    let header = FrameHeader::parse(bytes.as_ref()).map_err(to_napi)?;
    Ok(LorawanHeader {
        message_type: match header.message_type() {
            MessageType::JoinRequest => LorawanMessageType::JoinRequest,
            MessageType::JoinAccept => LorawanMessageType::JoinAccept,
            MessageType::UnconfirmedUp => LorawanMessageType::UnconfirmedUp,
            MessageType::ConfirmedUp => LorawanMessageType::ConfirmedUp,
            MessageType::UnconfirmedDown => LorawanMessageType::UnconfirmedDown,
            MessageType::ConfirmedDown => LorawanMessageType::ConfirmedDown,
        },
        is_data: header.message_type().is_data(),
        dev_addr: header.dev_addr(),
        fcnt: header.fcnt(),
        fport: header.fport(),
        confirmed: header.confirmed(),
        adr: header.adr(),
        ack: header.ack(),
        fpending: header.fpending(),
        fopts_len: header.fopts_len() as u32,
        payload_len: header.payload_len() as u32,
    })
}

/// Verifies a join-request and reads the identifiers out of it.
#[napi]
pub fn lorawan_parse_join_request(
    bytes: Buffer,
    app_key: Buffer,
) -> napi::Result<LorawanJoinRequest> {
    let request = JoinRequest::parse(bytes.as_ref(), &key(&app_key, "appKey")?).map_err(to_napi)?;
    Ok(LorawanJoinRequest {
        dev_eui: request.dev_eui().to_vec().into(),
        app_eui: request.app_eui().to_vec().into(),
        dev_nonce: request.dev_nonce(),
    })
}

/// Builds the signed join-accept a network sends to admit a device.
#[napi]
pub fn lorawan_grant_accept(
    grant: LorawanGrant,
    app_key: Buffer,
    dev_nonce: u16,
) -> napi::Result<Buffer> {
    let app_key = key(&app_key, "appKey")?;
    Ok(granted(grant)?
        .accept(&app_key, dev_nonce)
        .as_bytes()
        .to_vec()
        .into())
}

/// Derives the session a grant activates, the same one the device computes.
#[napi]
pub fn lorawan_grant_session(
    grant: LorawanGrant,
    app_key: Buffer,
    dev_nonce: u16,
) -> napi::Result<LorawanSession> {
    let app_key = key(&app_key, "appKey")?;
    Ok(LorawanSession {
        inner: granted(grant)?.session(&app_key, dev_nonce),
    })
}

/// Rebuilds the Rust grant from the object JavaScript supplied.
fn granted(grant: LorawanGrant) -> napi::Result<JoinGrant> {
    let mut built = JoinGrant::new(grant.app_nonce, grant.net_id, grant.dev_addr)
        .with_dl_settings(grant.dl_settings.unwrap_or(0))
        .with_rx_delay(grant.rx_delay.unwrap_or(0));
    if let Some(cflist) = grant.cflist {
        let cflist = <[u8; 16]>::try_from(cflist.as_ref())
            .map_err(|_| napi::Error::from_reason("cflist must be exactly 16 bytes".to_owned()))?;
        built = built.with_cflist(cflist);
    }
    Ok(built)
}

/// One of the commands a network and a device configure each other with.
///
/// `kind` names the command and `cid` is the identifier it travels under. Only the fields
/// that command carries are set; the rest are `null`. The same identifier means a different
/// command in each direction, so `direction` decides which one this is.
#[napi(object)]
pub struct LorawanMacCommand {
    /// Which command this is, as a name.
    pub kind: String,
    /// The identifier it travels under.
    pub cid: u8,
    /// Which way it travels.
    pub direction: LorawanDirection,
    /// How far above the floor a link check arrived, in dB.
    pub margin: Option<u8>,
    /// How many gateways heard it.
    pub gateways: Option<u8>,
    /// The data rate a network asks a device to use.
    pub data_rate: Option<u8>,
    /// The transmit power it may use, as a ceiling.
    pub tx_power: Option<u8>,
    /// Which channels may carry an uplink.
    pub channel_mask: Option<u16>,
    /// Which block of sixteen channels that mask applies to.
    pub mask_control: Option<u8>,
    /// How many times to send an unconfirmed uplink.
    pub transmissions: Option<u8>,
    /// Whether the power was set.
    pub power_ack: Option<bool>,
    /// Whether the data rate was set.
    pub data_rate_ack: Option<bool>,
    /// Whether the channel mask was usable.
    pub channel_mask_ack: Option<bool>,
    /// The share of the air a device is held to, as one over two to this.
    pub max_duty_cycle: Option<u8>,
    /// How far the first receive window sits below the uplink rate.
    pub rx1_offset: Option<u8>,
    /// The rate of the second receive window.
    pub rx2_data_rate: Option<u8>,
    /// A frequency in hertz, for the receive window and the channel commands.
    pub frequency_hz: Option<u32>,
    /// Whether the window offset was in range.
    pub rx1_offset_ack: Option<bool>,
    /// Whether the window rate was known.
    pub rx2_data_rate_ack: Option<bool>,
    /// Whether the frequency was usable.
    pub channel_ack: Option<bool>,
    /// A device battery level: 0 on external power, 255 when it cannot tell.
    pub battery: Option<u8>,
    /// The signal-to-noise ratio of the last request, in dB.
    pub snr_margin: Option<i32>,
    /// Which channel a channel command names.
    pub index: Option<u8>,
    /// The fastest rate allowed on it.
    pub max_data_rate: Option<u8>,
    /// The slowest rate allowed on it.
    pub min_data_rate: Option<u8>,
    /// Whether the device can run that range of rates.
    pub data_rate_range_ok: Option<bool>,
    /// Whether its radio can reach that frequency.
    pub frequency_ok: Option<bool>,
    /// How long a device waits before its first receive window, as the command codes it.
    pub delay: Option<u8>,
    /// The coded transmit power ceiling a region imposes.
    pub max_eirp: Option<u8>,
    /// Whether an uplink is held to 400 ms of air time.
    pub uplink_dwell: Option<bool>,
    /// Whether a downlink is.
    pub downlink_dwell: Option<bool>,
    /// Whether the channel already had an uplink frequency to pair a downlink with.
    pub uplink_frequency_exists: Option<bool>,
    /// Seconds since the GPS epoch.
    pub seconds: Option<u32>,
    /// The fraction of that second, in steps of one part in 256.
    pub fraction: Option<u8>,
}

fn blank_command(kind: &str, command: &MacCommand) -> LorawanMacCommand {
    LorawanMacCommand {
        kind: kind.to_owned(),
        cid: command.cid(),
        direction: match command.direction() {
            Direction::Uplink => LorawanDirection::Uplink,
            Direction::Downlink => LorawanDirection::Downlink,
        },
        margin: None,
        gateways: None,
        data_rate: None,
        tx_power: None,
        channel_mask: None,
        mask_control: None,
        transmissions: None,
        power_ack: None,
        data_rate_ack: None,
        channel_mask_ack: None,
        max_duty_cycle: None,
        rx1_offset: None,
        rx2_data_rate: None,
        frequency_hz: None,
        rx1_offset_ack: None,
        rx2_data_rate_ack: None,
        channel_ack: None,
        battery: None,
        snr_margin: None,
        index: None,
        max_data_rate: None,
        min_data_rate: None,
        data_rate_range_ok: None,
        frequency_ok: None,
        delay: None,
        max_eirp: None,
        uplink_dwell: None,
        downlink_dwell: None,
        uplink_frequency_exists: None,
        seconds: None,
        fraction: None,
    }
}

fn describe_command(command: MacCommand) -> LorawanMacCommand {
    match command {
        MacCommand::LinkCheckReq => blank_command("linkCheckReq", &command),
        MacCommand::LinkCheckAns { margin, gateways } => {
            let mut out = blank_command("linkCheckAns", &command);
            out.margin = Some(margin);
            out.gateways = Some(gateways);
            out
        }
        MacCommand::LinkAdrReq {
            data_rate,
            tx_power,
            channel_mask,
            mask_control,
            transmissions,
        } => {
            let mut out = blank_command("linkAdrReq", &command);
            out.data_rate = Some(data_rate);
            out.tx_power = Some(tx_power);
            out.channel_mask = Some(channel_mask);
            out.mask_control = Some(mask_control);
            out.transmissions = Some(transmissions);
            out
        }
        MacCommand::LinkAdrAns {
            power_ack,
            data_rate_ack,
            channel_mask_ack,
        } => {
            let mut out = blank_command("linkAdrAns", &command);
            out.power_ack = Some(power_ack);
            out.data_rate_ack = Some(data_rate_ack);
            out.channel_mask_ack = Some(channel_mask_ack);
            out
        }
        MacCommand::DutyCycleReq { max_duty_cycle } => {
            let mut out = blank_command("dutyCycleReq", &command);
            out.max_duty_cycle = Some(max_duty_cycle);
            out
        }
        MacCommand::DutyCycleAns => blank_command("dutyCycleAns", &command),
        MacCommand::RxParamSetupReq {
            rx1_offset,
            rx2_data_rate,
            frequency_hz,
        } => {
            let mut out = blank_command("rxParamSetupReq", &command);
            out.rx1_offset = Some(rx1_offset);
            out.rx2_data_rate = Some(rx2_data_rate);
            out.frequency_hz = Some(frequency_hz);
            out
        }
        MacCommand::RxParamSetupAns {
            rx1_offset_ack,
            rx2_data_rate_ack,
            channel_ack,
        } => {
            let mut out = blank_command("rxParamSetupAns", &command);
            out.rx1_offset_ack = Some(rx1_offset_ack);
            out.rx2_data_rate_ack = Some(rx2_data_rate_ack);
            out.channel_ack = Some(channel_ack);
            out
        }
        MacCommand::DevStatusReq => blank_command("devStatusReq", &command),
        MacCommand::DevStatusAns { battery, margin } => {
            let mut out = blank_command("devStatusAns", &command);
            out.battery = Some(battery);
            out.snr_margin = Some(i32::from(margin));
            out
        }
        MacCommand::NewChannelReq {
            index,
            frequency_hz,
            max_data_rate,
            min_data_rate,
        } => {
            let mut out = blank_command("newChannelReq", &command);
            out.index = Some(index);
            out.frequency_hz = Some(frequency_hz);
            out.max_data_rate = Some(max_data_rate);
            out.min_data_rate = Some(min_data_rate);
            out
        }
        MacCommand::NewChannelAns {
            data_rate_range_ok,
            frequency_ok,
        } => {
            let mut out = blank_command("newChannelAns", &command);
            out.data_rate_range_ok = Some(data_rate_range_ok);
            out.frequency_ok = Some(frequency_ok);
            out
        }
        MacCommand::RxTimingSetupReq { delay } => {
            let mut out = blank_command("rxTimingSetupReq", &command);
            out.delay = Some(delay);
            out
        }
        MacCommand::RxTimingSetupAns => blank_command("rxTimingSetupAns", &command),
        MacCommand::TxParamSetupReq {
            max_eirp,
            uplink_dwell,
            downlink_dwell,
        } => {
            let mut out = blank_command("txParamSetupReq", &command);
            out.max_eirp = Some(max_eirp);
            out.uplink_dwell = Some(uplink_dwell);
            out.downlink_dwell = Some(downlink_dwell);
            out
        }
        MacCommand::TxParamSetupAns => blank_command("txParamSetupAns", &command),
        MacCommand::DlChannelReq {
            index,
            frequency_hz,
        } => {
            let mut out = blank_command("dlChannelReq", &command);
            out.index = Some(index);
            out.frequency_hz = Some(frequency_hz);
            out
        }
        MacCommand::DlChannelAns {
            uplink_frequency_exists,
            frequency_ok,
        } => {
            let mut out = blank_command("dlChannelAns", &command);
            out.uplink_frequency_exists = Some(uplink_frequency_exists);
            out.frequency_ok = Some(frequency_ok);
            out
        }
        MacCommand::DeviceTimeReq => blank_command("deviceTimeReq", &command),
        MacCommand::DeviceTimeAns { seconds, fraction } => {
            let mut out = blank_command("deviceTimeAns", &command);
            out.seconds = Some(seconds);
            out.fraction = Some(fraction);
            out
        }
    }
}

fn rebuild(command: &LorawanMacCommand) -> napi::Result<MacCommand> {
    use pamoja_lorawan::mac;

    let down = matches!(command.direction, LorawanDirection::Downlink);
    let byte = |value: Option<u8>| value.unwrap_or(0);
    let flag = |value: Option<bool>| value.unwrap_or(false);

    let built = match (command.cid, down) {
        (mac::CID_LINK_CHECK, false) => MacCommand::LinkCheckReq,
        (mac::CID_LINK_CHECK, true) => MacCommand::LinkCheckAns {
            margin: byte(command.margin),
            gateways: byte(command.gateways),
        },
        (mac::CID_LINK_ADR, true) => MacCommand::LinkAdrReq {
            data_rate: byte(command.data_rate),
            tx_power: byte(command.tx_power),
            channel_mask: command.channel_mask.unwrap_or(0),
            mask_control: byte(command.mask_control),
            transmissions: byte(command.transmissions),
        },
        (mac::CID_LINK_ADR, false) => MacCommand::LinkAdrAns {
            power_ack: flag(command.power_ack),
            data_rate_ack: flag(command.data_rate_ack),
            channel_mask_ack: flag(command.channel_mask_ack),
        },
        (mac::CID_DUTY_CYCLE, true) => MacCommand::DutyCycleReq {
            max_duty_cycle: byte(command.max_duty_cycle),
        },
        (mac::CID_DUTY_CYCLE, false) => MacCommand::DutyCycleAns,
        (mac::CID_RX_PARAM_SETUP, true) => MacCommand::RxParamSetupReq {
            rx1_offset: byte(command.rx1_offset),
            rx2_data_rate: byte(command.rx2_data_rate),
            frequency_hz: command.frequency_hz.unwrap_or(0),
        },
        (mac::CID_RX_PARAM_SETUP, false) => MacCommand::RxParamSetupAns {
            rx1_offset_ack: flag(command.rx1_offset_ack),
            rx2_data_rate_ack: flag(command.rx2_data_rate_ack),
            channel_ack: flag(command.channel_ack),
        },
        (mac::CID_DEV_STATUS, true) => MacCommand::DevStatusReq,
        (mac::CID_DEV_STATUS, false) => MacCommand::DevStatusAns {
            battery: byte(command.battery),
            margin: command.snr_margin.unwrap_or(0) as i8,
        },
        (mac::CID_NEW_CHANNEL, true) => MacCommand::NewChannelReq {
            index: byte(command.index),
            frequency_hz: command.frequency_hz.unwrap_or(0),
            max_data_rate: byte(command.max_data_rate),
            min_data_rate: byte(command.min_data_rate),
        },
        (mac::CID_NEW_CHANNEL, false) => MacCommand::NewChannelAns {
            data_rate_range_ok: flag(command.data_rate_range_ok),
            frequency_ok: flag(command.frequency_ok),
        },
        (mac::CID_RX_TIMING_SETUP, true) => MacCommand::RxTimingSetupReq {
            delay: byte(command.delay),
        },
        (mac::CID_RX_TIMING_SETUP, false) => MacCommand::RxTimingSetupAns,
        (mac::CID_TX_PARAM_SETUP, true) => MacCommand::TxParamSetupReq {
            max_eirp: byte(command.max_eirp),
            uplink_dwell: flag(command.uplink_dwell),
            downlink_dwell: flag(command.downlink_dwell),
        },
        (mac::CID_TX_PARAM_SETUP, false) => MacCommand::TxParamSetupAns,
        (mac::CID_DL_CHANNEL, true) => MacCommand::DlChannelReq {
            index: byte(command.index),
            frequency_hz: command.frequency_hz.unwrap_or(0),
        },
        (mac::CID_DL_CHANNEL, false) => MacCommand::DlChannelAns {
            uplink_frequency_exists: flag(command.uplink_frequency_exists),
            frequency_ok: flag(command.frequency_ok),
        },
        (mac::CID_DEVICE_TIME, false) => MacCommand::DeviceTimeReq,
        (mac::CID_DEVICE_TIME, true) => MacCommand::DeviceTimeAns {
            seconds: command.seconds.unwrap_or(0),
            fraction: byte(command.fraction),
        },
        _ => {
            return Err(napi::Error::from_reason(format!(
                "identifier {:#04x} names no command in that direction",
                command.cid
            )))
        }
    };
    Ok(built)
}

/// Reads the commands packed into a frame options field, or a payload sent on port zero.
///
/// The same identifier means a different command in each direction, so the direction decides
/// what is read and there is no default.
///
/// A command does not carry its own length, so one this build does not know cannot be
/// stepped over. Reading stops there, and what came before it is returned.
///
/// # Arguments
///
/// * `direction` - which way the frame carrying them travels.
/// * `bytes` - the options field, or the payload.
///
/// # Returns
///
/// The commands that were readable, in order.
#[napi(js_name = "lorawanMacParse")]
pub fn lorawan_mac_parse(direction: LorawanDirection, bytes: Buffer) -> Vec<LorawanMacCommand> {
    let travel = match direction {
        LorawanDirection::Uplink => Direction::Uplink,
        LorawanDirection::Downlink => Direction::Downlink,
    };
    MacCommands::new(travel, &bytes)
        .take_while(Result::is_ok)
        .filter_map(Result::ok)
        .map(describe_command)
        .collect()
}

/// Writes one command out.
///
/// # Arguments
///
/// * `command` - the command, whose `cid` and `direction` decide which fields are read.
///
/// # Returns
///
/// The bytes it goes out as.
///
/// # Errors
///
/// When the identifier and direction name no command, or a field will not fit what carries
/// it.
#[napi(js_name = "lorawanMacEncode")]
pub fn lorawan_mac_encode(command: LorawanMacCommand) -> napi::Result<Buffer> {
    let built = rebuild(&command)?;
    let mut out = [0u8; pamoja_lorawan::mac::MAX_COMMAND];
    let written = built
        .encode(&mut out)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(Buffer::from(&out[..written]))
}
