//! A LoRaWAN relay, TS011-1.0.1, for JavaScript: the wake-on-radio frames an end device and
//! a relay exchange, the uplinks a relay forwards, and the timing that keeps a wake-on-radio
//! preamble short.
//!
//! Every call is a pure function of its arguments. Keys cross as a pair of 16-byte buffers,
//! and times as microsecond numbers, which stay exact below 2^53.

use crate::checked;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use pamoja_lora::region::RelayChannel;
use pamoja_lorawan::device::Heard;
use pamoja_lorawan::mac::relay_second_channel;
use pamoja_lorawan::relay::{
    self, open_wor_ack, t_offset_ms, unsynchronized_preamble_symbols, wor_ack, wor_join_request,
    wor_uplink, CadPeriodicity, CadToRx, Carrier, Forward, ForwardedUplink, Listen, Relay,
    RelayConfig, RelayError, RelayHeard, RelaySettings, Scan, StateSync, Synchronization,
    UplinkMetadata, Wake, Wor, WorChannel, WorKeys, XtalAccuracy,
};
use pamoja_lorawan::LorawanError;

use crate::lora::{lora_link_of, LoraLink};
use crate::lora_region::{LoraChannelPlan, LoraRelayChannel};
use crate::lorawan::{LorawanDevice, LorawanSession, LorawanWorKeys};
use crate::lorawan_device::{
    heard_out, made_from, next_out, thrown as device_thrown, transmission_out, window_in,
    LorawanDeviceSettings, LorawanHeard, LorawanNext, LorawanReceiveWindow, LorawanTransmission,
};
use pamoja_lorawan::device::EndDevice;

/// The port every message between a relay and its network uses.
#[napi]
pub const LORAWAN_LA_FPORT_RELAY: u8 = relay::LA_FPORT_RELAY;

/// How many end devices a relay verifies wake-on-radio frames for.
#[napi]
pub const LORAWAN_TRUSTED_ED_NUMBER: u32 = relay::TRUSTED_ED_NUMBER as u32;

/// How many WOR frames go without an acknowledgment before the uplink goes anyway.
#[napi]
pub const LORAWAN_WOR_ATTEMPTS_WO_ACK: u8 = relay::WOR_ATTEMPTS_WO_ACK;

/// The gap between a WOR frame, or its acknowledgment, and the LoRaWAN frame after it.
#[napi]
pub const LORAWAN_WOR_DATA_DELAY_US: u32 = relay::WOR_DATA_DELAY_US;

/// The gap between a WOR frame and its acknowledgment.
#[napi]
pub const LORAWAN_WOR_ACK_DELAY_US: u32 = relay::WOR_ACK_DELAY_US;

/// The gap between a relay hearing an uplink and forwarding it.
#[napi]
pub const LORAWAN_RELAY_FWD_DELAY_US: u32 = relay::RELAY_FWD_DELAY_US;

/// How long after an uplink an end device's RXR window opens at the latest.
#[napi]
pub const LORAWAN_RXR_DELAY_US: u32 = relay::RXR_DELAY_US;

/// The bytes a forwarded uplink adds in front of the end device's frame.
#[napi]
pub const LORAWAN_FORWARD_OVERHEAD: u32 = relay::FORWARD_OVERHEAD as u32;

/// The shortest WOR preamble, in symbols.
#[napi]
pub const LORAWAN_MIN_WOR_PREAMBLE_SYMBOLS: u16 = relay::MIN_WOR_PREAMBLE_SYMBOLS;

/// Where a frame goes and how fast.
#[napi(object)]
pub struct LorawanCarrier {
    /// The frequency in hertz.
    pub frequency_hz: checked::u32,
    /// The data rate.
    pub data_rate: checked::u8,
}

/// Which WOR frame a relay heard.
#[napi(string_enum)]
pub enum LorawanWorKind {
    /// Ahead of a join request.
    JoinRequest,
    /// Ahead of a Class A uplink.
    Uplink,
}

/// A wake-on-radio frame, as a relay reads it.
#[napi(object)]
pub struct LorawanWor {
    /// Which frame it is.
    pub kind: LorawanWorKind,
    /// Where and how fast a join request follows; an uplink's carrier is sealed until
    /// `lorawanRelayWorOpen` reads it.
    pub uplink: Option<LorawanCarrier>,
    /// The address an uplink WOR names.
    pub dev_addr: Option<u32>,
    /// The low sixteen bits of its frame counter.
    pub wfcnt: Option<u32>,
}

/// How often a relay scans a channel, TS011-1.0.1 table 18.
#[napi(string_enum)]
pub enum LorawanCadPeriodicity {
    /// Once a second.
    Ms1000,
    /// Every 500 milliseconds.
    Ms500,
    /// Every 250 milliseconds.
    Ms250,
    /// Every 100 milliseconds.
    Ms100,
    /// Every 50 milliseconds.
    Ms50,
    /// Every 20 milliseconds.
    Ms20,
}

/// How many symbols a relay takes from detecting activity to receiving, table 15.
#[napi(string_enum)]
pub enum LorawanCadToRx {
    /// Two symbols.
    Symbols2,
    /// Four.
    Symbols4,
    /// Six.
    Symbols6,
    /// Eight.
    Symbols8,
}

/// How accurate a relay's crystal is, table 17.
#[napi(string_enum)]
pub enum LorawanXtalAccuracy {
    /// Better than 10 parts per million.
    Ppm10,
    /// Better than 20.
    Ppm20,
    /// Better than 30.
    Ppm30,
    /// Better than 40.
    Ppm40,
}

/// Whether a relay will forward the uplink, table 16.
#[napi(string_enum)]
pub enum LorawanRelayForward {
    /// It has room to.
    Available,
    /// A limit is reached; try again in 30 minutes.
    RetryIn30Minutes,
    /// A limit is reached; try again in 60 minutes.
    RetryIn60Minutes,
    /// Forwarding is off.
    Disabled,
}

/// What a relay tells an end device about itself in a WOR ACK, table 14.
#[napi(object)]
pub struct LorawanStateSync {
    /// How long it takes to start receiving.
    pub cad_to_rx: LorawanCadToRx,
    /// Whether it forwards.
    pub forward: LorawanRelayForward,
    /// The data rate it forwards at.
    pub relay_data_rate: checked::u8,
    /// How accurate its crystal is.
    pub xtal_accuracy: LorawanXtalAccuracy,
    /// How often it scans.
    pub cad_periodicity: LorawanCadPeriodicity,
    /// Milliseconds from the start of the scan to the end of the WOR preamble.
    pub t_offset_ms: checked::u16,
}

/// Which of a relay's channels a WOR frame arrived on.
#[napi(string_enum)]
pub enum LorawanWorChannel {
    /// The default channel.
    Default,
    /// The second channel.
    Second,
}

/// What a relay heard of an uplink it forwards.
#[napi(object)]
pub struct LorawanUplinkMetadata {
    /// The channel the WOR frame came in on.
    pub wor_channel: LorawanWorChannel,
    /// The uplink's signal strength in dBm, carried from -142 to -15.
    pub rssi_dbm: checked::i32,
    /// Its signal-to-noise ratio in dB, carried from -20 to 11.
    pub snr_db: checked::i32,
    /// The data rate it arrived at.
    pub data_rate: checked::u8,
}

/// An end device's uplink as a relay forwards it on port 226.
#[napi(object)]
pub struct LorawanForwardedUplink {
    /// What the relay heard of it.
    pub metadata: LorawanUplinkMetadata,
    /// The frequency it arrived on, in hertz.
    pub frequency_hz: checked::u32,
    /// The end device's frame.
    pub phy_payload: Buffer,
}

/// What an end device knows of a relay's scans once a WOR ACK has arrived.
#[napi(object)]
pub struct LorawanSynchronization {
    /// When the relay scanned, in the device's microseconds.
    pub reference_us: f64,
    /// How often it scans.
    pub cad_periodicity: LorawanCadPeriodicity,
    /// How accurate its crystal is.
    pub relay_xtal: LorawanXtalAccuracy,
    /// How long it takes to start receiving.
    pub cad_to_rx: LorawanCadToRx,
}

/// When a synchronized end device's next WOR frame goes out.
#[napi(object)]
pub struct LorawanWorSlot {
    /// When to start sending, in microseconds.
    pub start_us: f64,
    /// The preamble length in symbols.
    pub preamble_symbols: u16,
}

fn refused(error: LorawanError) -> Error {
    Error::from_reason(error.to_string())
}

fn micros(value: f64, what: &str) -> Result<u64> {
    if (0.0..=9_007_199_254_740_991.0).contains(&value) && value.fract() == 0.0 {
        Ok(value as u64)
    } else {
        Err(Error::new(
            Status::InvalidArg,
            format!("{what} {value} is not a whole number of microseconds"),
        ))
    }
}

fn sixteen(bytes: &Buffer, what: &str) -> Result<[u8; 16]> {
    <[u8; 16]>::try_from(bytes.as_ref())
        .map_err(|_| Error::from_reason(format!("{what} must be exactly 16 bytes")))
}

fn keys_in(keys: &LorawanWorKeys) -> Result<WorKeys> {
    Ok(WorKeys::new(
        sixteen(&keys.integrity, "integrity")?,
        sixteen(&keys.encryption, "encryption")?,
    ))
}

fn keys_out(keys: &WorKeys) -> LorawanWorKeys {
    LorawanWorKeys {
        integrity: keys.integrity().to_vec().into(),
        encryption: keys.encryption().to_vec().into(),
    }
}

fn carrier_in(carrier: &LorawanCarrier) -> Carrier {
    Carrier::new(carrier.frequency_hz.get(), carrier.data_rate.get())
}

pub(crate) fn carrier_out(carrier: Carrier) -> LorawanCarrier {
    LorawanCarrier {
        frequency_hz: carrier.frequency_hz.into(),
        data_rate: carrier.data_rate.into(),
    }
}

pub(crate) fn periodicity_in(value: &LorawanCadPeriodicity) -> CadPeriodicity {
    match value {
        LorawanCadPeriodicity::Ms1000 => CadPeriodicity::Ms1000,
        LorawanCadPeriodicity::Ms500 => CadPeriodicity::Ms500,
        LorawanCadPeriodicity::Ms250 => CadPeriodicity::Ms250,
        LorawanCadPeriodicity::Ms100 => CadPeriodicity::Ms100,
        LorawanCadPeriodicity::Ms50 => CadPeriodicity::Ms50,
        LorawanCadPeriodicity::Ms20 => CadPeriodicity::Ms20,
    }
}

pub(crate) fn periodicity_out(value: CadPeriodicity) -> LorawanCadPeriodicity {
    match value {
        CadPeriodicity::Ms1000 => LorawanCadPeriodicity::Ms1000,
        CadPeriodicity::Ms500 => LorawanCadPeriodicity::Ms500,
        CadPeriodicity::Ms250 => LorawanCadPeriodicity::Ms250,
        CadPeriodicity::Ms100 => LorawanCadPeriodicity::Ms100,
        CadPeriodicity::Ms50 => LorawanCadPeriodicity::Ms50,
        CadPeriodicity::Ms20 => LorawanCadPeriodicity::Ms20,
    }
}

pub(crate) fn receive_in(value: &LorawanCadToRx) -> CadToRx {
    match value {
        LorawanCadToRx::Symbols2 => CadToRx::Symbols2,
        LorawanCadToRx::Symbols4 => CadToRx::Symbols4,
        LorawanCadToRx::Symbols6 => CadToRx::Symbols6,
        LorawanCadToRx::Symbols8 => CadToRx::Symbols8,
    }
}

pub(crate) fn receive_out(value: CadToRx) -> LorawanCadToRx {
    match value {
        CadToRx::Symbols2 => LorawanCadToRx::Symbols2,
        CadToRx::Symbols4 => LorawanCadToRx::Symbols4,
        CadToRx::Symbols6 => LorawanCadToRx::Symbols6,
        CadToRx::Symbols8 => LorawanCadToRx::Symbols8,
    }
}

pub(crate) fn xtal_in(value: &LorawanXtalAccuracy) -> XtalAccuracy {
    match value {
        LorawanXtalAccuracy::Ppm10 => XtalAccuracy::Ppm10,
        LorawanXtalAccuracy::Ppm20 => XtalAccuracy::Ppm20,
        LorawanXtalAccuracy::Ppm30 => XtalAccuracy::Ppm30,
        LorawanXtalAccuracy::Ppm40 => XtalAccuracy::Ppm40,
    }
}

pub(crate) fn xtal_out(value: XtalAccuracy) -> LorawanXtalAccuracy {
    match value {
        XtalAccuracy::Ppm10 => LorawanXtalAccuracy::Ppm10,
        XtalAccuracy::Ppm20 => LorawanXtalAccuracy::Ppm20,
        XtalAccuracy::Ppm30 => LorawanXtalAccuracy::Ppm30,
        XtalAccuracy::Ppm40 => LorawanXtalAccuracy::Ppm40,
    }
}

fn state_in(state: &LorawanStateSync) -> StateSync {
    StateSync {
        cad_to_rx: receive_in(&state.cad_to_rx),
        forward: match state.forward {
            LorawanRelayForward::Available => Forward::Available,
            LorawanRelayForward::RetryIn30Minutes => Forward::RetryIn30Minutes,
            LorawanRelayForward::RetryIn60Minutes => Forward::RetryIn60Minutes,
            LorawanRelayForward::Disabled => Forward::Disabled,
        },
        relay_data_rate: state.relay_data_rate.get(),
        xtal_accuracy: xtal_in(&state.xtal_accuracy),
        cad_periodicity: periodicity_in(&state.cad_periodicity),
        t_offset_ms: state.t_offset_ms.get(),
    }
}

/// The forwarding state of a relay, as JavaScript names it.
pub(crate) fn forward_out(forward: Forward) -> LorawanRelayForward {
    match forward {
        Forward::Available => LorawanRelayForward::Available,
        Forward::RetryIn30Minutes => LorawanRelayForward::RetryIn30Minutes,
        Forward::RetryIn60Minutes => LorawanRelayForward::RetryIn60Minutes,
        Forward::Disabled => LorawanRelayForward::Disabled,
    }
}

fn state_out(state: StateSync) -> LorawanStateSync {
    LorawanStateSync {
        cad_to_rx: receive_out(state.cad_to_rx),
        forward: match state.forward {
            Forward::Available => LorawanRelayForward::Available,
            Forward::RetryIn30Minutes => LorawanRelayForward::RetryIn30Minutes,
            Forward::RetryIn60Minutes => LorawanRelayForward::RetryIn60Minutes,
            Forward::Disabled => LorawanRelayForward::Disabled,
        },
        relay_data_rate: state.relay_data_rate.into(),
        xtal_accuracy: xtal_out(state.xtal_accuracy),
        cad_periodicity: periodicity_out(state.cad_periodicity),
        t_offset_ms: state.t_offset_ms.into(),
    }
}

fn clamp_i16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

fn clamp_i8(value: i32) -> i8 {
    value.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8
}

/// Derives an end device's root relay session key from its network session key,
/// TS011-1.0.1 section 4.4.
#[napi(js_name = "lorawanRelayRootWorSKey")]
pub fn lorawan_relay_root_wor_s_key(network_key: Buffer) -> Result<Buffer> {
    Ok(relay::root_wor_s_key(&sixteen(&network_key, "networkKey")?)
        .to_vec()
        .into())
}

/// Derives an end device's wake-on-radio keys from its root relay session key, section 4.5.
#[napi(js_name = "lorawanRelayWorKeys")]
pub fn lorawan_relay_wor_keys(root_key: Buffer, dev_addr: checked::u32) -> Result<LorawanWorKeys> {
    Ok(keys_out(&WorKeys::derive(
        &sixteen(&root_key, "rootKey")?,
        dev_addr.get(),
    )))
}

/// Builds the WOR frame ahead of a join request, section 5.3.1.
#[napi(js_name = "lorawanRelayWorJoinRequest")]
pub fn lorawan_relay_wor_join_request(uplink: LorawanCarrier) -> Result<Buffer> {
    wor_join_request(carrier_in(&uplink))
        .map(|frame| frame.to_vec().into())
        .map_err(refused)
}

/// Builds the WOR frame ahead of a Class A uplink, section 5.3.2.
#[napi(js_name = "lorawanRelayWorUplink")]
pub fn lorawan_relay_wor_uplink(
    keys: LorawanWorKeys,
    dev_addr: checked::u32,
    wfcnt: checked::u32,
    uplink: LorawanCarrier,
    wor: LorawanCarrier,
) -> Result<Buffer> {
    wor_uplink(
        &keys_in(&keys)?,
        dev_addr.get(),
        wfcnt.get(),
        carrier_in(&uplink),
        carrier_in(&wor),
    )
    .map(|frame| frame.to_vec().into())
    .map_err(refused)
}

/// Reads a WOR frame, leaving an uplink's carrier sealed.
#[napi(js_name = "lorawanRelayWorParse")]
pub fn lorawan_relay_wor_parse(frame: Buffer) -> Result<LorawanWor> {
    match Wor::parse(frame.as_ref()).map_err(refused)? {
        Wor::JoinRequest { uplink } => Ok(LorawanWor {
            kind: LorawanWorKind::JoinRequest,
            uplink: Some(carrier_out(uplink)),
            dev_addr: None,
            wfcnt: None,
        }),
        Wor::Uplink(sealed) => Ok(LorawanWor {
            kind: LorawanWorKind::Uplink,
            uplink: None,
            dev_addr: Some(sealed.dev_addr()),
            wfcnt: Some(u32::from(sealed.wfcnt())),
        }),
    }
}

/// Checks a WOR frame ahead of a Class A uplink and reads where the uplink follows.
#[napi(js_name = "lorawanRelayWorOpen")]
pub fn lorawan_relay_wor_open(
    frame: Buffer,
    keys: LorawanWorKeys,
    wfcnt: checked::u32,
    wor: LorawanCarrier,
) -> Result<LorawanCarrier> {
    let Wor::Uplink(sealed) = Wor::parse(frame.as_ref()).map_err(refused)? else {
        return Err(Error::from_reason(
            "a join request WOR carries nothing sealed".to_owned(),
        ));
    };
    sealed
        .open(&keys_in(&keys)?, wfcnt.get(), carrier_in(&wor))
        .map(carrier_out)
        .map_err(refused)
}

/// Builds a relay's WOR ACK, section 6.2.
#[napi(js_name = "lorawanRelayWorAck")]
pub fn lorawan_relay_wor_ack(
    keys: LorawanWorKeys,
    dev_addr: checked::u32,
    wfcnt: checked::u32,
    ack: LorawanCarrier,
    uplink: LorawanCarrier,
    state: LorawanStateSync,
) -> Result<Buffer> {
    wor_ack(
        &keys_in(&keys)?,
        dev_addr.get(),
        wfcnt.get(),
        carrier_in(&ack),
        carrier_in(&uplink),
        state_in(&state),
    )
    .map(|frame| frame.to_vec().into())
    .map_err(refused)
}

/// Checks and reads a WOR ACK, section 6.2.
#[napi(js_name = "lorawanRelayWorAckOpen")]
pub fn lorawan_relay_wor_ack_open(
    frame: Buffer,
    keys: LorawanWorKeys,
    dev_addr: checked::u32,
    wfcnt: checked::u32,
    ack: LorawanCarrier,
    uplink: LorawanCarrier,
) -> Result<LorawanStateSync> {
    open_wor_ack(
        frame.as_ref(),
        &keys_in(&keys)?,
        dev_addr.get(),
        wfcnt.get(),
        carrier_in(&ack),
        carrier_in(&uplink),
    )
    .map(state_out)
    .map_err(refused)
}

/// Writes an uplink a relay forwards on port 226, section 9.1.
#[napi(js_name = "lorawanRelayForwardEncode")]
pub fn lorawan_relay_forward_encode(forwarded: LorawanForwardedUplink) -> Result<Buffer> {
    let uplink = ForwardedUplink {
        metadata: UplinkMetadata {
            wor_channel: match forwarded.metadata.wor_channel {
                LorawanWorChannel::Default => WorChannel::Default,
                LorawanWorChannel::Second => WorChannel::Second,
            },
            rssi_dbm: clamp_i16(forwarded.metadata.rssi_dbm.get()),
            snr_db: clamp_i8(forwarded.metadata.snr_db.get()),
            data_rate: forwarded.metadata.data_rate.get(),
        },
        frequency_hz: forwarded.frequency_hz.get(),
        phy_payload: forwarded.phy_payload.as_ref(),
    };
    let mut out = vec![0u8; relay::FORWARD_OVERHEAD + uplink.phy_payload.len()];
    uplink.encode(&mut out).map_err(refused)?;
    Ok(out.into())
}

/// Reads an uplink a relay forwarded, section 9.1.
#[napi(js_name = "lorawanRelayForwardParse")]
pub fn lorawan_relay_forward_parse(payload: Buffer) -> Result<LorawanForwardedUplink> {
    let forwarded = ForwardedUplink::parse(payload.as_ref()).map_err(refused)?;
    Ok(LorawanForwardedUplink {
        metadata: LorawanUplinkMetadata {
            wor_channel: match forwarded.metadata.wor_channel {
                WorChannel::Default => LorawanWorChannel::Default,
                WorChannel::Second => LorawanWorChannel::Second,
            },
            rssi_dbm: i32::from(forwarded.metadata.rssi_dbm).into(),
            snr_db: i32::from(forwarded.metadata.snr_db).into(),
            data_rate: forwarded.metadata.data_rate.into(),
        },
        frequency_hz: forwarded.frequency_hz.into(),
        phy_payload: forwarded.phy_payload.to_vec().into(),
    })
}

/// The WOR preamble of an end device that does not know when the relay scans, section 5.2.
#[napi(js_name = "lorawanRelayUnsynchronizedPreamble")]
pub fn lorawan_relay_unsynchronized_preamble(
    cad_periodicity: LorawanCadPeriodicity,
    symbol_us: f64,
    cad_to_rx: LorawanCadToRx,
) -> Result<u16> {
    Ok(unsynchronized_preamble_symbols(
        periodicity_in(&cad_periodicity),
        micros(symbol_us, "symbolUs")?,
        receive_in(&cad_to_rx),
    ))
}

/// The offset a relay reports in a WOR ACK, appendix 1, or null when the preamble ended
/// before the scan or more than eleven bits of milliseconds after it.
#[napi(js_name = "lorawanRelayTOffsetMs")]
pub fn lorawan_relay_t_offset_ms(scan_start_us: f64, preamble_end_us: f64) -> Result<Option<u16>> {
    Ok(t_offset_ms(
        micros(scan_start_us, "scanStartUs")?,
        micros(preamble_end_us, "preambleEndUs")?,
    ))
}

/// Works out when a relay scanned from the WOR ACK that answered a frame.
#[napi(js_name = "lorawanRelaySynchronization")]
pub fn lorawan_relay_synchronization(
    wor_start_us: f64,
    preamble_symbols: checked::u16,
    symbol_us: f64,
    state: LorawanStateSync,
) -> Result<LorawanSynchronization> {
    let sync = Synchronization::from_ack(
        micros(wor_start_us, "worStartUs")?,
        preamble_symbols.get(),
        micros(symbol_us, "symbolUs")?,
        &state_in(&state),
    );
    Ok(LorawanSynchronization {
        reference_us: sync.reference_us as f64,
        cad_periodicity: periodicity_out(sync.cad_periodicity),
        relay_xtal: xtal_out(sync.relay_xtal),
        cad_to_rx: receive_out(sync.cad_to_rx),
    })
}

/// Picks the relay scan a synchronized end device aims its next WOR frame at, appendix 1,
/// or null once the drift exceeds a period.
#[napi(js_name = "lorawanRelayNextWor")]
pub fn lorawan_relay_next_wor(
    synchronization: LorawanSynchronization,
    now_us: f64,
    device_xtal_ppm: checked::u32,
    symbol_us: f64,
    other_channel: bool,
) -> Result<Option<LorawanWorSlot>> {
    let sync = Synchronization {
        reference_us: micros(synchronization.reference_us, "referenceUs")?,
        cad_periodicity: periodicity_in(&synchronization.cad_periodicity),
        relay_xtal: xtal_in(&synchronization.relay_xtal),
        cad_to_rx: receive_in(&synchronization.cad_to_rx),
    };
    Ok(sync
        .next_wor(
            micros(now_us, "nowUs")?,
            device_xtal_ppm.get(),
            micros(symbol_us, "symbolUs")?,
            other_channel,
        )
        .map(|slot| LorawanWorSlot {
            start_us: slot.start_us as f64,
            preamble_symbols: slot.preamble_symbols,
        }))
}

/// Reads the second channel a relay or end device configuration describes, or null when the
/// index is not 1 or the offset is reserved.
#[napi(js_name = "lorawanRelaySecondChannel")]
pub fn lorawan_relay_second_channel(
    second_channel_index: checked::u8,
    data_rate: checked::u8,
    ack_offset: checked::u8,
    frequency_hz: checked::u32,
) -> Option<LoraRelayChannel> {
    relay_second_channel(
        second_channel_index.get(),
        data_rate.get(),
        ack_offset.get(),
        frequency_hz.get(),
    )
    .map(|channel| LoraRelayChannel {
        wor_frequency_hz: channel.wor_frequency_hz.into(),
        ack_frequency_hz: channel.ack_frequency_hz.into(),
        data_rate: channel.data_rate.into(),
    })
}

/// A scan for wake-on-radio frames, due next.
#[napi(object)]
pub struct LorawanScan {
    /// When to start detecting, in microseconds.
    pub start_us: f64,
    /// Which channel: `default` or `second`.
    pub channel: LorawanWorChannel,
    /// Where to listen, and how fast.
    pub carrier: LorawanCarrier,
    /// The LoRa settings of a wake-on-radio frame, heard with inverted IQ.
    pub link: LoraLink,
    /// The longest preamble an end device sends on the channel, in symbols.
    pub preamble_symbols: checked::u16,
}

/// The acknowledgment a relay answers a wake-on-radio frame with.
#[napi(object)]
pub struct LorawanAcknowledgment {
    /// The frame, seven bytes.
    pub frame: Buffer,
    /// When to start sending it, in microseconds.
    pub start_us: f64,
    /// Where it goes, and how fast.
    pub carrier: LorawanCarrier,
    /// Its LoRa settings, sent with inverted IQ.
    pub link: LoraLink,
    /// The power to ask of the radio, conducted, in dBm.
    pub output_dbm: i32,
    /// How long it holds the air, in microseconds.
    pub airtime_us: f64,
}

/// When and where to listen for the uplink a wake-on-radio frame announced.
#[napi(object)]
pub struct LorawanListen {
    /// When the uplink starts, in microseconds.
    pub start_us: f64,
    /// Where it arrives, and how fast.
    pub carrier: LorawanCarrier,
    /// Its LoRa settings, heard with standard IQ.
    pub link: LoraLink,
    /// The longest frame the relay forwards; stop receiving anything longer.
    pub max_len: u32,
}

/// What a wake-on-radio frame led a relay to do.
#[napi(string_enum)]
pub enum LorawanWakeKind {
    /// A join request from a device the relay's filters let through.
    JoinRequest,
    /// An uplink from a trusted device.
    Uplink,
    /// A frame from a device the relay does not know, which it tells its network about.
    Notified,
}

/// What a wake-on-radio frame led a relay to do.
#[napi(object)]
pub struct LorawanWake {
    /// What the frame led to.
    pub kind: LorawanWakeKind,
    /// The device, for an uplink or a notification.
    pub dev_addr: Option<u32>,
    /// The wake-on-radio frame counter it carried, for an uplink.
    pub wfcnt: Option<u32>,
    /// Whether the relay forwards the uplink, which the acknowledgment reports.
    pub forward: Option<LorawanRelayForward>,
    /// The acknowledgment to send, where there is one.
    pub acknowledgment: Option<LorawanAcknowledgment>,
    /// Where and when to listen for the uplink, where the relay will.
    pub listen: Option<LorawanListen>,
}

/// A downlink for an end device, to send in its relay window.
#[napi(object)]
pub struct LorawanRxrDownlink {
    /// The frame to send.
    pub frame: Buffer,
    /// When to start sending it, in microseconds.
    pub start_us: f64,
    /// Where it goes, and how fast.
    pub carrier: LorawanCarrier,
    /// Its LoRa settings, sent with inverted IQ and a payload CRC.
    pub link: LoraLink,
    /// The power to ask of the radio, conducted, in dBm.
    pub output_dbm: i32,
    /// How long it holds the air, in microseconds.
    pub airtime_us: f64,
}

/// What a frame a relay's own device heard turned out to be.
#[napi(string_enum)]
pub enum LorawanRelayHeardKind {
    /// The relay's own device read it.
    Device,
    /// A downlink for an end device, to send in its relay window.
    Downlink,
    /// A downlink for an end device the relay cannot send on.
    Undeliverable,
}

/// What a frame a relay's own device heard turned out to be.
#[napi(object)]
pub struct LorawanRelayHeard {
    /// What the frame turned out to be.
    pub kind: LorawanRelayHeardKind,
    /// What the relay's own device made of it.
    pub heard: Option<LorawanHeard>,
    /// The downlink to send the end device on, for `downlink`.
    pub downlink: Option<LorawanRxrDownlink>,
    /// Why one could not be sent on, for `undeliverable`.
    pub reason: Option<String>,
}

/// A LoRaWAN relay: an end device that also listens for others, TS011-1.0.1.
///
/// One turn runs like this: `nextScan` says when and where to listen, a wake-on-radio frame
/// heard there goes to `heardWor`, the uplink behind it to `heardUplink`, and `forward`
/// sends that to the network. What the relay's own windows hear goes to `heardIn`, which
/// turns a downlink for the end device into one to send in its relay window.
#[napi]
pub struct LorawanRelay {
    inner: Relay<'static>,
}

#[napi]
impl LorawanRelay {
    /// Makes a relay whose own device is activated by personalization.
    #[napi(factory)]
    pub fn personalized(
        env: Env,
        plan: &LoraChannelPlan,
        session: &LorawanSession,
        settings: LorawanDeviceSettings,
        xtal_accuracy: LorawanXtalAccuracy,
        cad_to_rx: LorawanCadToRx,
    ) -> Result<LorawanRelay> {
        let (plan, made) = made_from(plan, &settings)?;
        let device = EndDevice::personalized(plan, session.inner, made)
            .map_err(|error| device_thrown(&env, error))?;
        Ok(LorawanRelay {
            inner: Relay::new(
                device,
                RelaySettings::new(xtal_in(&xtal_accuracy), receive_in(&cad_to_rx)),
            ),
        })
    }

    /// Makes a relay whose own device joins over the air.
    #[napi(factory)]
    pub fn over_the_air(
        env: Env,
        plan: &LoraChannelPlan,
        credentials: &LorawanDevice,
        settings: LorawanDeviceSettings,
        xtal_accuracy: LorawanXtalAccuracy,
        cad_to_rx: LorawanCadToRx,
    ) -> Result<LorawanRelay> {
        let (plan, made) = made_from(plan, &settings)?;
        let device = EndDevice::new(plan, credentials.inner.clone(), made)
            .map_err(|error| device_thrown(&env, error))?;
        Ok(LorawanRelay {
            inner: Relay::new(
                device,
                RelaySettings::new(xtal_in(&xtal_accuracy), receive_in(&cad_to_rx)),
            ),
        })
    }

    /// Starts scanning, or changes what a running relay scans from its next scan on.
    #[napi]
    pub fn start(
        &mut self,
        env: Env,
        cad_periodicity: LorawanCadPeriodicity,
        default_channel_index: checked::u8,
        second_channel: Option<LoraRelayChannel>,
    ) -> Result<()> {
        let Some(default_channel) = self.inner.region_channel(default_channel_index.get()) else {
            return Err(Error::new(
                Status::InvalidArg,
                "the region does not define that wake-on-radio channel",
            ));
        };
        let mut config = RelayConfig::new(periodicity_in(&cad_periodicity), default_channel);
        if let Some(second) = second_channel {
            config = config.with_second_channel(RelayChannel::new(
                second.wor_frequency_hz.get(),
                second.ack_frequency_hz.get(),
                second.data_rate.get(),
            ));
        }
        self.inner
            .start(config)
            .map_err(|error| relay_thrown(&env, error))
    }

    /// Stops scanning. A forwarded uplink already waiting still goes out.
    #[napi]
    pub fn stop(&mut self) {
        self.inner.stop();
    }

    /// Whether the relay is scanning.
    #[napi(getter)]
    pub fn running(&self) -> bool {
        self.inner.config().is_some()
    }

    /// Trusts an end device, as an `UpdateUplinkListReq` with the same fields does.
    #[napi]
    pub fn trust(
        &mut self,
        index: checked::u8,
        dev_addr: checked::u32,
        root_wor_s_key: Buffer,
        next_wfcnt: checked::u32,
        reload_rate: checked::u8,
        bucket_size: checked::u8,
    ) -> Result<()> {
        let key = sixteen(&root_wor_s_key, "rootWorSKey")?;
        if self.inner.trust(
            index.get(),
            dev_addr.get(),
            &key,
            next_wfcnt.get(),
            reload_rate.get(),
            bucket_size.get(),
        ) {
            Ok(())
        } else {
            Err(Error::new(
                Status::InvalidArg,
                "a relay trusts sixteen devices, at indexes 0 to 15",
            ))
        }
    }

    /// Says when and where to scan next, or `null` while the relay is stopped.
    #[napi]
    pub fn next_scan(&mut self, now_us: f64) -> Result<Option<LorawanScan>> {
        let now_us = micros(now_us, "nowUs")?;
        Ok(self.inner.next_scan(now_us).map(|scan| LorawanScan {
            start_us: scan.start_us as f64,
            channel: channel_out(scan.channel),
            carrier: carrier_out(scan.carrier),
            link: lora_link_of(scan.link),
            preamble_symbols: scan.preamble_symbols.into(),
        }))
    }

    /// Reads a wake-on-radio frame a scan heard.
    #[napi]
    pub fn heard_wor(
        &mut self,
        env: Env,
        scan: LorawanScan,
        frame: Buffer,
        rssi_dbm: checked::i32,
        snr_db: checked::i32,
        ended_us: f64,
    ) -> Result<LorawanWake> {
        let ended_us = micros(ended_us, "endedUs")?;
        let scan = Scan {
            start_us: micros(scan.start_us, "startUs")?,
            channel: channel_in(&scan.channel),
            carrier: Carrier::new(
                scan.carrier.frequency_hz.get(),
                scan.carrier.data_rate.get(),
            ),
            link: crate::lora::settings(&scan.link),
            preamble_symbols: scan.preamble_symbols.get(),
        };
        let wake = self
            .inner
            .heard_wor(
                &scan,
                frame.as_ref(),
                rssi_dbm
                    .get()
                    .clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
                snr_db.get().clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8,
                ended_us,
            )
            .map_err(|error| relay_thrown(&env, error))?;
        Ok(wake_out(wake))
    }

    /// Reads the uplink a wake-on-radio frame announced, and holds it to forward.
    ///
    /// Returns when to `forward` it: fifty milliseconds after it ended.
    #[napi]
    pub fn heard_uplink(
        &mut self,
        env: Env,
        frame: Buffer,
        rssi_dbm: checked::i32,
        snr_db: checked::i32,
        ended_us: f64,
    ) -> Result<f64> {
        let ended_us = micros(ended_us, "endedUs")?;
        self.inner
            .heard_uplink(
                frame.as_ref(),
                rssi_dbm
                    .get()
                    .clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
                snr_db.get().clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8,
                ended_us,
            )
            .map(|due_us| due_us as f64)
            .map_err(|error| relay_thrown(&env, error))
    }

    /// Clears the uplink a wake-on-radio frame announced, once listening heard nothing.
    #[napi]
    pub fn uplink_missed(&mut self) {
        self.inner.uplink_missed();
    }

    /// When the forwarded uplink waiting to go out is due, or `null` with nothing waiting.
    #[napi(getter)]
    pub fn forward_due(&self) -> Option<f64> {
        self.inner.forward_due().map(|due_us| due_us as f64)
    }

    /// Sends the uplink the relay is holding, in its own uplink on port 226.
    #[napi]
    pub fn forward(&mut self, env: Env, now_us: f64) -> Result<LorawanTransmission> {
        let now_us = micros(now_us, "nowUs")?;
        self.inner
            .forward(now_us)
            .map(transmission_out)
            .map_err(|error| relay_thrown(&env, error))
    }

    /// Reads a frame the relay's own device heard, acting on the relay commands in it.
    #[napi]
    pub fn heard_in(
        &mut self,
        env: Env,
        window: LorawanReceiveWindow,
        frame: Buffer,
        snr_db: checked::i32,
    ) -> Result<LorawanRelayHeard> {
        let snr_db = snr_db.get().clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
        let heard = self
            .inner
            .heard_in(window_in(window), frame.as_ref(), snr_db)
            .map_err(|error| relay_thrown(&env, error))?;
        Ok(match heard {
            RelayHeard::Device(heard) => LorawanRelayHeard {
                kind: LorawanRelayHeardKind::Device,
                heard: Some(heard_out(
                    heard,
                    self.inner.device().dev_addr().unwrap_or(0),
                )),
                downlink: None,
                reason: None,
            },
            RelayHeard::Downlink { delivery, downlink } => LorawanRelayHeard {
                kind: LorawanRelayHeardKind::Downlink,
                heard: Some(heard_out(
                    Heard::Data(delivery),
                    self.inner.device().dev_addr().unwrap_or(0),
                )),
                downlink: Some(LorawanRxrDownlink {
                    frame: downlink.frame().to_vec().into(),
                    start_us: downlink.start_us as f64,
                    carrier: carrier_out(downlink.carrier),
                    link: lora_link_of(downlink.link),
                    output_dbm: i32::from(downlink.output_dbm),
                    airtime_us: downlink.airtime_us as f64,
                }),
                reason: None,
            },
            RelayHeard::Undeliverable { delivery, reason } => LorawanRelayHeard {
                kind: LorawanRelayHeardKind::Undeliverable,
                heard: Some(heard_out(
                    Heard::Data(delivery),
                    self.inner.device().dev_addr().unwrap_or(0),
                )),
                downlink: None,
                reason: Some(reason.to_string()),
            },
        })
    }

    /// Says what comes next once the relay's own windows closed with nothing in them.
    #[napi]
    pub fn nothing_heard(&mut self, env: Env, now_us: f64) -> Result<LorawanNext> {
        let now_us = micros(now_us, "nowUs")?;
        self.inner
            .nothing_heard(now_us)
            .map(next_out)
            .map_err(|error| relay_thrown(&env, error))
    }

    /// Makes the relay's own join request.
    #[napi]
    pub fn join(
        &mut self,
        env: Env,
        dev_nonce: checked::u16,
        now_us: f64,
    ) -> Result<LorawanTransmission> {
        let now_us = micros(now_us, "nowUs")?;
        self.inner
            .device_mut()
            .join(dev_nonce.get(), now_us)
            .map(transmission_out)
            .map_err(|error| device_thrown(&env, error))
    }

    /// Sends one of the relay's own uplinks, which also carries what it owes its network.
    #[napi]
    pub fn send(
        &mut self,
        env: Env,
        port: checked::u8,
        payload: Buffer,
        confirmed: bool,
        now_us: f64,
    ) -> Result<LorawanTransmission> {
        let now_us = micros(now_us, "nowUs")?;
        self.inner
            .device_mut()
            .send(port.get(), payload.as_ref(), confirmed, now_us)
            .map(transmission_out)
            .map_err(|error| device_thrown(&env, error))
    }

    /// Sends an uplink with no payload, carrying whatever the relay owes its network.
    #[napi]
    pub fn send_empty(&mut self, env: Env, now_us: f64) -> Result<LorawanTransmission> {
        let now_us = micros(now_us, "nowUs")?;
        self.inner
            .device_mut()
            .send_empty(now_us)
            .map(transmission_out)
            .map_err(|error| device_thrown(&env, error))
    }

    /// The address the relay's own device is on the network by.
    #[napi(getter)]
    pub fn dev_addr(&self) -> Option<u32> {
        self.inner.device().dev_addr()
    }

    /// Whether the relay's own device has joined.
    #[napi(getter)]
    pub fn joined(&self) -> bool {
        self.inner.device().is_joined()
    }

    /// The data rate the relay forwards at, which its acknowledgments report.
    #[napi(getter)]
    pub fn data_rate(&self) -> u8 {
        self.inner.device().data_rate()
    }
}

/// Describes what a wake-on-radio frame led to the way JavaScript holds it.
fn wake_out(wake: Wake) -> LorawanWake {
    match wake {
        Wake::JoinRequest { listen } => LorawanWake {
            kind: LorawanWakeKind::JoinRequest,
            dev_addr: None,
            wfcnt: None,
            forward: None,
            acknowledgment: None,
            listen: Some(listen_out(listen)),
        },
        Wake::Uplink {
            dev_addr,
            wfcnt,
            forward,
            acknowledgment,
            listen,
        } => LorawanWake {
            kind: LorawanWakeKind::Uplink,
            dev_addr: Some(dev_addr),
            wfcnt: Some(wfcnt),
            forward: Some(forward_out(forward)),
            acknowledgment: acknowledgment.map(|ack| LorawanAcknowledgment {
                frame: ack.frame.to_vec().into(),
                start_us: ack.start_us as f64,
                carrier: carrier_out(ack.carrier),
                link: lora_link_of(ack.link),
                output_dbm: i32::from(ack.output_dbm),
                airtime_us: ack.airtime_us as f64,
            }),
            listen: listen.map(listen_out),
        },
        Wake::Notified { dev_addr } => LorawanWake {
            kind: LorawanWakeKind::Notified,
            dev_addr: Some(dev_addr),
            wfcnt: None,
            forward: None,
            acknowledgment: None,
            listen: None,
        },
    }
}

/// Describes where an uplink arrives the way JavaScript holds it.
fn listen_out(listen: Listen) -> LorawanListen {
    LorawanListen {
        start_us: listen.start_us as f64,
        carrier: carrier_out(listen.carrier),
        link: lora_link_of(listen.link),
        max_len: listen.max_len as u32,
    }
}

/// Which channel a scan is on, as JavaScript names it.
fn channel_out(channel: WorChannel) -> LorawanWorChannel {
    match channel {
        WorChannel::Default => LorawanWorChannel::Default,
        WorChannel::Second => LorawanWorChannel::Second,
    }
}

/// Reads which channel a scan is on.
fn channel_in(channel: &LorawanWorChannel) -> WorChannel {
    match channel {
        LorawanWorChannel::Default => WorChannel::Default,
        LorawanWorChannel::Second => WorChannel::Second,
    }
}

/// Throws what a relay refused, with its reason as the error's `code`.
fn relay_thrown(env: &Env, error: RelayError) -> Error {
    let _ = env;
    Error::new(Status::GenericFailure, error.to_string())
}
