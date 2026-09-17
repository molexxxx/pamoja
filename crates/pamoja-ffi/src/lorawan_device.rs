//! The C ABI for a LoRaWAN Class A end device, without a radio.
//!
//! [`PamojaLorawanEndDevice`] wraps `pamoja_lorawan::device::EndDevice`: it joins, chooses a
//! channel and data rate for each uplink, says when and where to listen for the answer, reads
//! what comes back, and does what the network's MAC commands ask. It owns no radio and no
//! clock, so every call takes the time in microseconds and hands back what to put on the air.
//!
//! A device lives as long as the program may keep it, so it runs on a published channel plan:
//! a region from [`pamoja_lora_plan_for_region`](crate::lora_region::pamoja_lora_plan_for_region)
//! or a CN470-510 plan from
//! [`pamoja_lora_plan_for_cn470`](crate::lora_region::pamoja_lora_plan_for_cn470). The plan
//! handle may be freed once the device is made.
//!
//! A call that fails returns a status and records why on the device. Read it with
//! [`pamoja_lorawan_end_device_error`], which says, for example, until when the air is not
//! free.

use std::ptr;

use pamoja_lora::LinkSettings;
use pamoja_lorawan::device::{
    Battery, DeviceError, EndDevice, Heard, Next, ReceiveWindow, RelayExchange, RelayStatus, Saved,
    Settings, StateError, Transmission, Window, WorNext, SAVED_LEN,
};
use pamoja_lorawan::relay::{RelayActivation, RelaySync, WOR_UPLINK_LEN};
use pamoja_lorawan::LorawanError;

use crate::lora::PamojaLoraLink;
use crate::lora_region::PamojaLoraPlan;
use crate::lorawan::{failed, PamojaLorawanDevice, PamojaLorawanSession};
use crate::lorawan_link::{PAMOJA_LORAWAN_VERSION_1_0_3, PAMOJA_LORAWAN_VERSION_1_0_4};
use crate::{read_bytes, set_last_error, PamojaBuffer, PamojaStatus};

/// How many bytes a saved device state takes.
pub const PAMOJA_LORAWAN_SAVED_LEN: usize = 1678;

/// A device running from an external supply.
pub const PAMOJA_LORAWAN_BATTERY_EXTERNAL: u8 = 0;
/// A device reporting a battery level from 1, empty, to 254, full.
pub const PAMOJA_LORAWAN_BATTERY_LEVEL: u8 = 1;
/// A device that cannot measure its battery.
pub const PAMOJA_LORAWAN_BATTERY_UNKNOWN: u8 = 2;

/// A join accept: the device is on the network.
pub const PAMOJA_LORAWAN_HEARD_JOINED: u8 = 0;
/// A data frame for the device.
pub const PAMOJA_LORAWAN_HEARD_DATA: u8 = 1;

/// The first receive window, on the uplink's downlink channel.
pub const PAMOJA_LORAWAN_WINDOW_RX1: u8 = 1;
/// The second receive window, on the fixed frequency and data rate.
pub const PAMOJA_LORAWAN_WINDOW_RX2: u8 = 2;
/// The relay window, which a device under a relay opens last, TS011-1.0.1 chapter 7.
pub const PAMOJA_LORAWAN_WINDOW_RXR: u8 = 3;

/// Never send through a relay, TS011-1.0.1 table 40.
pub const PAMOJA_LORAWAN_RELAY_DISABLED: u8 = 0;
/// Always send through a relay.
pub const PAMOJA_LORAWAN_RELAY_ENABLED: u8 = 1;
/// Start using one after enough uplinks go unanswered.
pub const PAMOJA_LORAWAN_RELAY_DYNAMIC: u8 = 2;
/// Leave it to the device, which is where every device starts.
pub const PAMOJA_LORAWAN_RELAY_DEVICE_CONTROLLED: u8 = 3;

/// The device knows nothing of a relay, TS011-1.0.1 section 3.9.
pub const PAMOJA_LORAWAN_RELAY_INITIALIZED: u8 = 0;
/// It knows how a relay scans, but not when.
pub const PAMOJA_LORAWAN_RELAY_UNSYNCHRONIZED: u8 = 1;
/// It knows when the relay next scans.
pub const PAMOJA_LORAWAN_RELAY_SYNCHRONIZED: u8 = 2;

/// The uplink goes out at the time the exchange named.
pub const PAMOJA_LORAWAN_WOR_NEXT_UPLINK: u8 = 0;
/// The relay is woken again first, as the network's BackOff asks.
pub const PAMOJA_LORAWAN_WOR_NEXT_WAKE_UP: u8 = 1;

/// Send the same frame again, no sooner than the time given.
pub const PAMOJA_LORAWAN_NEXT_REPEAT: u8 = 0;
/// The uplink is finished.
pub const PAMOJA_LORAWAN_NEXT_DONE: u8 = 1;
/// A confirmed uplink went out every time it may without an acknowledgment.
pub const PAMOJA_LORAWAN_NEXT_UNACKNOWLEDGED: u8 = 2;
/// The join got no answer; join again with a new nonce, no sooner than the time given.
pub const PAMOJA_LORAWAN_NEXT_JOIN_AGAIN: u8 = 3;

/// The last call succeeded.
pub const PAMOJA_LORAWAN_DEVICE_OK: u8 = 0;
/// The plan defines more channels than a device keeps.
pub const PAMOJA_LORAWAN_DEVICE_TOO_MANY_CHANNELS: u8 = 1;
/// A device activated by personalization has nothing to join with.
pub const PAMOJA_LORAWAN_DEVICE_NO_CREDENTIALS: u8 = 2;
/// The device has not joined.
pub const PAMOJA_LORAWAN_DEVICE_NOT_JOINED: u8 = 3;
/// A transmission is still waiting on its receive windows.
pub const PAMOJA_LORAWAN_DEVICE_BUSY: u8 = 4;
/// There is no transmission waiting on its windows or due to repeat.
pub const PAMOJA_LORAWAN_DEVICE_NOTHING_PENDING: u8 = 5;
/// The air is not free until `until_us`.
pub const PAMOJA_LORAWAN_DEVICE_WAIT: u8 = 6;
/// No enabled channel carries the data rate.
pub const PAMOJA_LORAWAN_DEVICE_NO_CHANNEL: u8 = 7;
/// The data rate `data_rate` is not a LoRa one the device can use.
pub const PAMOJA_LORAWAN_DEVICE_DATA_RATE: u8 = 8;
/// The payload does not fit; `max` bytes do.
pub const PAMOJA_LORAWAN_DEVICE_PAYLOAD_TOO_LONG: u8 = 9;
/// The uplink frame counter is spent, and the device has to join again.
pub const PAMOJA_LORAWAN_DEVICE_COUNTER_EXHAUSTED: u8 = 10;
/// The frame did not decode, or port 0 was asked for an application payload.
pub const PAMOJA_LORAWAN_DEVICE_FRAME: u8 = 11;
/// The frame is addressed to another device.
pub const PAMOJA_LORAWAN_DEVICE_FOREIGN: u8 = 12;
/// The frame repeats or precedes the last downlink the device accepted.
pub const PAMOJA_LORAWAN_DEVICE_REPLAYED: u8 = 13;
/// The frame counter jumped further ahead than the device follows.
pub const PAMOJA_LORAWAN_DEVICE_COUNTER_GAP: u8 = 14;
/// A join accept carries settings the region does not allow.
pub const PAMOJA_LORAWAN_DEVICE_REFUSED: u8 = 15;
/// A saved state was not resumed; `state` says why.
pub const PAMOJA_LORAWAN_DEVICE_STATE: u8 = 16;

/// A saved state of the wrong length.
pub const PAMOJA_LORAWAN_STATE_LENGTH: u8 = 1;
/// A saved state that is corrupt.
pub const PAMOJA_LORAWAN_STATE_CORRUPT: u8 = 2;
/// A saved state in a format this build does not read, `format`.
pub const PAMOJA_LORAWAN_STATE_FORMAT: u8 = 3;
/// A saved state from another channel plan.
pub const PAMOJA_LORAWAN_STATE_PLAN: u8 = 4;

/// What a device's radio can do, and how it takes part.
///
/// Fill one with [`pamoja_lorawan_device_settings`], then change what differs.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanDeviceSettings {
    /// The lowest frequency the radio and its front end can use, in hertz.
    pub lowest_hz: u32,
    /// The highest, in hertz.
    pub highest_hz: u32,
    /// A seed for the random choices of channel and retry delay, ideally from a hardware
    /// random source. The device identifier is mixed in.
    pub seed: u32,
    /// The link layer revision, [`PAMOJA_LORAWAN_VERSION_1_0_3`] or
    /// [`PAMOJA_LORAWAN_VERSION_1_0_4`].
    pub version: u8,
    /// `1` to let the network manage the data rate and power.
    pub adr: u8,
    /// The lowest power the radio puts out, conducted, in dBm.
    pub min_output_dbm: i8,
    /// The highest, conducted, in dBm.
    pub max_output_dbm: i8,
    /// The antenna gain less the cable and connector losses, in dB.
    pub antenna_gain_db: i8,
    /// `1` to hold the device to the region's sub-band duty cycles.
    pub regional_duty_cycle: u8,
    /// `1` to size payloads for a path through a relay.
    pub behind_repeater: u8,
}

/// When and where to listen for a downlink.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanWindow {
    /// How long after the end of the transmission the window opens, in microseconds.
    pub delay_us: u32,
    /// The carrier, in hertz.
    pub frequency_hz: u32,
    /// The LoRa settings to listen with: no payload CRC, and inverted IQ, as RP002-1.0.5
    /// table 112 has for a downlink.
    pub link: PamojaLoraLink,
    /// The downlink data rate, as the region numbers them.
    pub data_rate: u8,
}

/// The wake-on-radio exchange an uplink under a relay goes out behind, TS011-1.0.1
/// section 5.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanRelayExchange {
    /// When to start sending the frame that wakes the relay, in microseconds.
    pub wake_up_start_us: u64,
    /// How long that frame holds the air, in microseconds.
    pub wake_up_airtime_us: u64,
    /// When the relay's acknowledgment would start arriving, in microseconds.
    pub ack_start_us: u64,
    /// How long it would last, in microseconds.
    pub ack_airtime_us: u64,
    /// When the uplink itself goes out, in microseconds, whether or not the acknowledgment
    /// arrives.
    pub uplink_start_us: u64,
    /// The frame that wakes the relay: five bytes ahead of a join request, fifteen ahead of
    /// an uplink.
    pub wake_up_frame: [u8; 15],
    /// How many of those bytes to send.
    pub wake_up_len: u8,
    /// Where the frame goes, in hertz.
    pub wake_up_frequency_hz: u32,
    /// The data rate it goes out at.
    pub wake_up_data_rate: u8,
    /// Its LoRa settings, with the preamble this frame needs, sent with inverted IQ.
    pub wake_up_link: PamojaLoraLink,
    /// The power to ask of the radio for it, conducted, in dBm.
    pub wake_up_output_dbm: i8,
    /// `1` when an acknowledgment is expected at all; a join request is never acknowledged.
    pub has_ack: u8,
    /// Where the acknowledgment would arrive, in hertz.
    pub ack_frequency_hz: u32,
    /// The data rate it would arrive at.
    pub ack_data_rate: u8,
    /// Its LoRa settings.
    pub ack_link: PamojaLoraLink,
    /// The relay window, timed from the end of the uplink like the other two.
    pub rxr: PamojaLorawanWindow,
}

/// What a relay's acknowledgment said about itself, TS011-1.0.1 table 14.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanRelayStatus {
    /// How often it scans, as table 18 codes it.
    pub cad_periodicity: u8,
    /// How accurate its crystal is, as table 17 codes it.
    pub xtal_accuracy: u8,
    /// How long it takes to start receiving, as table 15 codes it.
    pub cad_to_rx: u8,
    /// The data rate it forwards at, which bounds what the device may send.
    pub relay_data_rate: u8,
    /// Whether it will forward, as table 16 codes it.
    pub forward: u8,
}

/// A frame to put on the air, and where to listen afterward.
///
/// The frame's bytes cross beside it as a [`PamojaBuffer`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanTransmission {
    /// How long the frame holds the air, in microseconds.
    pub airtime_us: u64,
    /// The carrier, in hertz.
    pub frequency_hz: u32,
    /// The first receive window.
    pub rx1: PamojaLorawanWindow,
    /// The second, which opens only if nothing for this device arrived in the first.
    pub rx2: PamojaLorawanWindow,
    /// The LoRa settings: an eight-symbol preamble, an explicit header and a payload CRC,
    /// sent with standard IQ.
    pub link: PamojaLoraLink,
    /// The data rate, as the region numbers them.
    pub data_rate: u8,
    /// The power to ask of the radio, conducted, in dBm.
    pub output_dbm: i8,
    /// `1` if the application payload went out in this frame; `0` if the answers the
    /// device owed left no room, and it has to be sent again.
    pub carries_payload: u8,
    /// `1` when this frame goes out behind a wake-on-radio frame, under a relay.
    pub has_relay: u8,
    /// The wake-on-radio exchange, when it does.
    pub relay: PamojaLorawanRelayExchange,
}

/// What a frame heard in a receive window turned out to be.
///
/// For a data frame the payload crosses beside it as a [`PamojaBuffer`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanHeard {
    /// For a join, the address the network gave the device.
    pub dev_addr: u32,
    /// For a time answer, whole seconds since the GPS epoch.
    pub gps_seconds: u32,
    /// [`PAMOJA_LORAWAN_HEARD_JOINED`] or [`PAMOJA_LORAWAN_HEARD_DATA`].
    pub kind: u8,
    /// `1` if the payload arrived on an application port.
    pub has_port: u8,
    /// The application port.
    pub port: u8,
    /// `1` if the network acknowledged the confirmed uplink this answered.
    pub acknowledged: u8,
    /// `1` if the network asked for this downlink to be acknowledged.
    pub confirmed: u8,
    /// `1` if the network has more waiting.
    pub more_pending: u8,
    /// `1` if the downlink answered a link check.
    pub has_link_check: u8,
    /// How far above the demodulation floor the best gateway heard the check, in dB.
    pub margin_db: u8,
    /// How many gateways heard it.
    pub gateways: u8,
    /// `1` if the downlink answered a time request.
    pub has_device_time: u8,
    /// The fraction of a second, in 256ths.
    pub fraction: u8,
}

/// What to do once both receive windows closed with nothing for the device.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanNext {
    /// For a repeat or another join, the earliest time to send, in microseconds.
    pub not_before_us: u64,
    /// One of the `PAMOJA_LORAWAN_NEXT_*` constants.
    pub kind: u8,
}

/// Why a device's last call failed.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanDeviceError {
    /// For [`PAMOJA_LORAWAN_DEVICE_WAIT`], the earliest time to try again, in microseconds.
    pub until_us: u64,
    /// For [`PAMOJA_LORAWAN_DEVICE_PAYLOAD_TOO_LONG`], the most the frame carries, and for
    /// [`PAMOJA_LORAWAN_DEVICE_TOO_MANY_CHANNELS`], the most channels a device keeps.
    pub max: u32,
    /// One of the `PAMOJA_LORAWAN_DEVICE_*` constants.
    pub kind: u8,
    /// For [`PAMOJA_LORAWAN_DEVICE_DATA_RATE`], the data rate.
    pub data_rate: u8,
    /// For [`PAMOJA_LORAWAN_DEVICE_STATE`], one of the `PAMOJA_LORAWAN_STATE_*` constants.
    pub state: u8,
    /// For [`PAMOJA_LORAWAN_STATE_FORMAT`], the format the state was saved in.
    pub format: u8,
}

/// Where a device stands, read in one call.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanEndDeviceStatus {
    /// The address the device is on the network by, meaningful when `joined` is `1`.
    pub dev_addr: u32,
    /// The next uplink frame counter.
    pub fcnt_up: u32,
    /// The last downlink frame counter accepted, meaningful when `has_fcnt_down` is `1`.
    pub fcnt_down: u32,
    /// Where the second receive window listens, in hertz.
    pub rx2_frequency_hz: u32,
    /// The delay from the end of an uplink to the first receive window, in microseconds.
    pub receive_delay_us: u32,
    /// The lowest frequency the device transmits or listens on.
    pub lowest_hz: u32,
    /// The highest.
    pub highest_hz: u32,
    /// How many channels are enabled.
    pub channel_count: u16,
    /// `1` once joined, or from the start for a personalized device.
    pub joined: u8,
    /// The data rate the next uplink goes out at, before any back-off step.
    pub data_rate: u8,
    /// `1` once a downlink has been accepted.
    pub has_fcnt_down: u8,
    /// How many times each uplink goes out, NbTrans.
    pub transmissions: u8,
    /// The data rate the second receive window listens at.
    pub rx2_data_rate: u8,
}

/// A channel a device may send on.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanChannel {
    /// Where uplinks go out, in hertz.
    pub uplink_hz: u32,
    /// Where the first receive window listens, in hertz.
    pub downlink_hz: u32,
    /// The slowest data rate the channel carries.
    pub min_data_rate: u8,
    /// The fastest.
    pub max_data_rate: u8,
}

/// An opaque handle to a LoRaWAN Class A end device.
///
/// Release it with [`pamoja_lorawan_end_device_free`].
pub struct PamojaLorawanEndDevice {
    device: EndDevice<'static>,
    error: Option<DeviceError>,
}

/// What a device is doing now, as C sees it.
pub(crate) fn status_out(device: &EndDevice<'static>) -> PamojaLorawanEndDeviceStatus {
    let (rx2_frequency_hz, rx2_data_rate) = device.rx2();
    let (lowest_hz, highest_hz) = device.frequency_span();
    PamojaLorawanEndDeviceStatus {
        dev_addr: device.dev_addr().unwrap_or(0),
        fcnt_up: device.fcnt_up(),
        fcnt_down: device.fcnt_down().unwrap_or(0),
        rx2_frequency_hz,
        receive_delay_us: device.receive_delay_us(),
        lowest_hz,
        highest_hz,
        channel_count: device.channels().count() as u16,
        joined: u8::from(device.is_joined()),
        data_rate: device.data_rate(),
        has_fcnt_down: u8::from(device.fcnt_down().is_some()),
        transmissions: device.transmissions(),
        rx2_data_rate,
    }
}

/// Fills in the settings of a typical node with a radio's output power range.
///
/// The rest start as TS001-1.0.4, adaptive data rate on, an antenna with no gain over its
/// cable, a radio that tunes 137 to 1020 MHz as an SX1276 does, the region's duty cycle
/// kept, no repeater in the path, and a seed of zero.
///
/// # Arguments
///
/// * `min_output_dbm` - the lowest power the radio puts out, conducted.
/// * `max_output_dbm` - the highest, conducted.
/// * `out_settings` - receives the settings.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_settings` is null.
///
/// # Safety
///
/// `out_settings` must point to a writable [`PamojaLorawanDeviceSettings`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_device_settings(
    min_output_dbm: i8,
    max_output_dbm: i8,
    out_settings: *mut PamojaLorawanDeviceSettings,
) -> PamojaStatus {
    if out_settings.is_null() {
        set_last_error("out_settings must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_settings = PamojaLorawanDeviceSettings {
        lowest_hz: 137_000_000,
        highest_hz: 1_020_000_000,
        seed: 0,
        version: PAMOJA_LORAWAN_VERSION_1_0_4,
        adr: 1,
        min_output_dbm,
        max_output_dbm,
        antenna_gain_db: 0,
        regional_duty_cycle: 1,
        behind_repeater: 0,
    };
    PamojaStatus::Ok
}

/// Rebuilds the Rust settings from the fields that crossed the boundary.
fn settings(crossed: &PamojaLorawanDeviceSettings) -> Result<Settings, PamojaStatus> {
    let version = match crossed.version {
        PAMOJA_LORAWAN_VERSION_1_0_3 => pamoja_lorawan::Version::V1_0_3,
        PAMOJA_LORAWAN_VERSION_1_0_4 => pamoja_lorawan::Version::V1_0_4,
        other => {
            set_last_error(format!("{other} is not a LoRaWAN version"));
            return Err(PamojaStatus::InvalidArgument);
        }
    };
    if crossed.min_output_dbm > crossed.max_output_dbm || crossed.lowest_hz > crossed.highest_hz {
        set_last_error("the output power and tuning ranges must run from low to high".to_owned());
        return Err(PamojaStatus::InvalidArgument);
    }
    let mut settings = Settings::new(crossed.min_output_dbm, crossed.max_output_dbm)
        .with_version(version)
        .with_adr(crossed.adr != 0)
        .with_antenna_gain(crossed.antenna_gain_db)
        .with_tuning_range(crossed.lowest_hz, crossed.highest_hz)
        .with_seed(crossed.seed);
    if crossed.regional_duty_cycle == 0 {
        settings = settings.without_regional_duty_cycle();
    }
    if crossed.behind_repeater != 0 {
        settings = settings.behind_repeater();
    }
    Ok(settings)
}

/// Reads the published plan and settings a new device is made from.
///
/// # Safety
///
/// `plan` must be a live plan handle or null, and `settings` must point to a readable
/// [`PamojaLorawanDeviceSettings`] or be null.
pub(crate) unsafe fn made_from(
    plan: *const PamojaLoraPlan,
    settings: *const PamojaLorawanDeviceSettings,
) -> Result<(&'static pamoja_lora::region::ChannelPlan<'static>, Settings), PamojaStatus> {
    let (Some(plan), Some(crossed)) = (plan.as_ref(), settings.as_ref()) else {
        set_last_error("plan and settings must not be null".to_owned());
        return Err(PamojaStatus::InvalidArgument);
    };
    let Some(published) = plan.published() else {
        set_last_error(
            "a device runs on a published plan, from pamoja_lora_plan_for_region or pamoja_lora_plan_for_cn470"
                .to_owned(),
        );
        return Err(PamojaStatus::InvalidArgument);
    };
    Ok((published, self::settings(crossed)?))
}

/// Hands a made device back through an out pointer.
///
/// # Safety
///
/// `slot` must be a writable pointer slot.
unsafe fn made(
    result: Result<EndDevice<'static>, DeviceError>,
    fcnt_up: u32,
    has_fcnt_down: u8,
    fcnt_down: u32,
    slot: &mut *mut PamojaLorawanEndDevice,
) -> PamojaStatus {
    match result {
        Ok(device) => {
            let down = (has_fcnt_down != 0).then_some(fcnt_down);
            *slot = Box::into_raw(Box::new(PamojaLorawanEndDevice {
                device: device.with_frame_counters(fcnt_up, down),
                error: None,
            }));
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            status_of(error)
        }
    }
}

/// Makes a device that joins over the air.
///
/// # Arguments
///
/// * `plan` - a published channel plan.
/// * `credentials` - the device's identifiers and root key, which the device copies.
/// * `settings` - what its radio can do.
/// * `fcnt_up` - the next uplink frame counter, 0 for a device that has never sent. A join
///   starts it over.
/// * `has_fcnt_down` - `1` if `fcnt_down` holds the last downlink counter accepted.
/// * `fcnt_down` - that counter.
/// * `out_device` - receives the device.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_device` set to a handle the caller must
/// release with [`pamoja_lorawan_end_device_free`].
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the plan was built rather
/// than published, the settings name no version or run backward, or the plan defines more
/// channels than a device keeps.
///
/// # Safety
///
/// `plan` and `credentials` must be live handles, `settings` must point to a readable
/// [`PamojaLorawanDeviceSettings`], and `out_device` to a writable pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_new(
    plan: *const PamojaLoraPlan,
    credentials: *const PamojaLorawanDevice,
    settings: *const PamojaLorawanDeviceSettings,
    fcnt_up: u32,
    has_fcnt_down: u8,
    fcnt_down: u32,
    out_device: *mut *mut PamojaLorawanEndDevice,
) -> PamojaStatus {
    if out_device.is_null() {
        set_last_error("out_device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_device;
    *slot = ptr::null_mut();
    let Some(credentials) = credentials.as_ref() else {
        set_last_error("credentials must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let (plan, settings) = match made_from(plan, settings) {
        Ok(pair) => pair,
        Err(status) => return status,
    };
    made(
        EndDevice::new(plan, credentials.device.clone(), settings),
        fcnt_up,
        has_fcnt_down,
        fcnt_down,
        slot,
    )
}

/// Makes a device activated by personalization, with its session provisioned.
///
/// Such a device never resets its frame counters, TS001-1.0.4 section 4.3.1.5, so a device
/// that lost power passes the ones it kept.
///
/// # Arguments
///
/// * `plan` - a published channel plan.
/// * `session` - the address and session keys it was provisioned with, which the device
///   copies.
/// * `settings` - what its radio can do.
/// * `fcnt_up` - the next uplink frame counter.
/// * `has_fcnt_down` - `1` if `fcnt_down` holds the last downlink counter accepted.
/// * `fcnt_down` - that counter.
/// * `out_device` - receives the device.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_device` set to a handle the caller must
/// release with [`pamoja_lorawan_end_device_free`].
///
/// # Errors
///
/// As [`pamoja_lorawan_end_device_new`].
///
/// # Safety
///
/// `plan` and `session` must be live handles, `settings` must point to a readable
/// [`PamojaLorawanDeviceSettings`], and `out_device` to a writable pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_personalized(
    plan: *const PamojaLoraPlan,
    session: *const PamojaLorawanSession,
    settings: *const PamojaLorawanDeviceSettings,
    fcnt_up: u32,
    has_fcnt_down: u8,
    fcnt_down: u32,
    out_device: *mut *mut PamojaLorawanEndDevice,
) -> PamojaStatus {
    if out_device.is_null() {
        set_last_error("out_device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_device;
    *slot = ptr::null_mut();
    let Some(session) = session.as_ref() else {
        set_last_error("session must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let (plan, settings) = match made_from(plan, settings) {
        Ok(pair) => pair,
        Err(status) => return status,
    };
    made(
        EndDevice::personalized(plan, session.session, settings),
        fcnt_up,
        has_fcnt_down,
        fcnt_down,
        slot,
    )
}

/// Releases a device handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `device` must be a handle from a call that made one and has not already been freed, or
/// null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_free(device: *mut PamojaLorawanEndDevice) {
    if !device.is_null() {
        drop(Box::from_raw(device));
    }
}

/// Reads why a device's last call failed.
///
/// # Arguments
///
/// * `device` - the device.
/// * `out_error` - receives the reason, with `kind` [`PAMOJA_LORAWAN_DEVICE_OK`] when the
///   last call succeeded.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `device` must be a live handle and `out_error` must point to a writable
/// [`PamojaLorawanDeviceError`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_error(
    device: *const PamojaLorawanEndDevice,
    out_error: *mut PamojaLorawanDeviceError,
) -> PamojaStatus {
    let (Some(device), false) = (device.as_ref(), out_error.is_null()) else {
        set_last_error("device and out_error must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_error = error_out(device.error);
    PamojaStatus::Ok
}

/// Describes a device error in the shape that crosses the boundary.
fn error_out(error: Option<DeviceError>) -> PamojaLorawanDeviceError {
    let mut out = PamojaLorawanDeviceError {
        until_us: 0,
        max: 0,
        kind: PAMOJA_LORAWAN_DEVICE_OK,
        data_rate: 0,
        state: 0,
        format: 0,
    };
    let Some(error) = error else {
        return out;
    };
    out.kind = match error {
        DeviceError::TooManyChannels { max } => {
            out.max = max as u32;
            PAMOJA_LORAWAN_DEVICE_TOO_MANY_CHANNELS
        }
        DeviceError::NoCredentials => PAMOJA_LORAWAN_DEVICE_NO_CREDENTIALS,
        DeviceError::NotJoined => PAMOJA_LORAWAN_DEVICE_NOT_JOINED,
        DeviceError::Busy => PAMOJA_LORAWAN_DEVICE_BUSY,
        DeviceError::NothingPending => PAMOJA_LORAWAN_DEVICE_NOTHING_PENDING,
        DeviceError::Wait { until_us } => {
            out.until_us = until_us;
            PAMOJA_LORAWAN_DEVICE_WAIT
        }
        DeviceError::NoChannel => PAMOJA_LORAWAN_DEVICE_NO_CHANNEL,
        DeviceError::DataRate(rate) => {
            out.data_rate = rate;
            PAMOJA_LORAWAN_DEVICE_DATA_RATE
        }
        DeviceError::PayloadTooLong { max } => {
            out.max = max as u32;
            PAMOJA_LORAWAN_DEVICE_PAYLOAD_TOO_LONG
        }
        DeviceError::CounterExhausted => PAMOJA_LORAWAN_DEVICE_COUNTER_EXHAUSTED,
        DeviceError::Frame(_) => PAMOJA_LORAWAN_DEVICE_FRAME,
        DeviceError::Foreign => PAMOJA_LORAWAN_DEVICE_FOREIGN,
        DeviceError::Replayed => PAMOJA_LORAWAN_DEVICE_REPLAYED,
        DeviceError::CounterGap => PAMOJA_LORAWAN_DEVICE_COUNTER_GAP,
        DeviceError::Refused => PAMOJA_LORAWAN_DEVICE_REFUSED,
        DeviceError::State(state) => {
            out.state = match state {
                StateError::Length => PAMOJA_LORAWAN_STATE_LENGTH,
                StateError::Corrupt => PAMOJA_LORAWAN_STATE_CORRUPT,
                StateError::Format(format) => {
                    out.format = format;
                    PAMOJA_LORAWAN_STATE_FORMAT
                }
                StateError::Plan => PAMOJA_LORAWAN_STATE_PLAN,
            };
            PAMOJA_LORAWAN_DEVICE_STATE
        }
    };
    out
}

/// The status a device error crosses as.
fn status_of(error: DeviceError) -> PamojaStatus {
    match error {
        DeviceError::TooManyChannels { .. }
        | DeviceError::NoCredentials
        | DeviceError::DataRate(_)
        | DeviceError::PayloadTooLong { .. }
        | DeviceError::State(StateError::Length) => PamojaStatus::InvalidArgument,
        DeviceError::Frame(frame) => failed(frame),
        DeviceError::Replayed | DeviceError::CounterGap => PamojaStatus::Auth,
        DeviceError::Refused | DeviceError::State(_) => PamojaStatus::Codec,
        DeviceError::CounterExhausted => PamojaStatus::Closed,
        DeviceError::NotJoined
        | DeviceError::Busy
        | DeviceError::NothingPending
        | DeviceError::Wait { .. }
        | DeviceError::NoChannel
        | DeviceError::Foreign => PamojaStatus::Other,
    }
}

impl PamojaLorawanEndDevice {
    /// Records how a call went and returns the status it crosses as.
    fn settle<T>(&mut self, result: Result<T, DeviceError>) -> Result<T, PamojaStatus> {
        match result {
            Ok(value) => {
                self.error = None;
                Ok(value)
            }
            Err(error) => {
                self.error = Some(error);
                set_last_error(error.to_string());
                Err(status_of(error))
            }
        }
    }
}

/// Converts link settings into the shape that crosses the boundary.
pub(crate) fn link_out(link: pamoja_lora::LinkSettings) -> PamojaLoraLink {
    PamojaLoraLink {
        bandwidth_hz: link.bandwidth_hz(),
        preamble_symbols: link.preamble_symbols(),
        spreading_factor: link.spreading_factor(),
        coding_rate_denominator: link.coding_rate_denominator(),
        explicit_header: u8::from(link.explicit_header()),
        crc: u8::from(link.crc()),
    }
}

/// Converts a receive window into the shape that crosses the boundary.
fn window_out(window: Window) -> PamojaLorawanWindow {
    PamojaLorawanWindow {
        delay_us: window.delay_us,
        frequency_hz: window.frequency_hz,
        link: link_out(window.link),
        data_rate: window.data_rate,
    }
}

/// Hands a transmission back through its two out pointers.
///
/// # Safety
///
/// Both pointers must be writable.
pub(crate) unsafe fn transmitted(
    transmission: Transmission,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
) {
    *out_frame = PamojaBuffer::into_raw(transmission.frame.as_bytes().to_vec());
    *out_transmission = PamojaLorawanTransmission {
        airtime_us: transmission.airtime_us,
        frequency_hz: transmission.frequency_hz,
        rx1: window_out(transmission.rx1),
        rx2: window_out(transmission.rx2),
        link: link_out(transmission.link),
        data_rate: transmission.data_rate,
        output_dbm: transmission.output_dbm,
        carries_payload: u8::from(transmission.carries_payload),
        has_relay: u8::from(transmission.relay.is_some()),
        relay: transmission.relay.map_or_else(empty_exchange, exchange_out),
    };
}

/// The wake-on-radio exchange of a relayed uplink, as C sees it.
fn exchange_out(exchange: RelayExchange) -> PamojaLorawanRelayExchange {
    let mut wake_up_frame = [0u8; WOR_UPLINK_LEN];
    let frame = exchange.wake_up.frame();
    wake_up_frame[..frame.len()].copy_from_slice(frame);
    PamojaLorawanRelayExchange {
        wake_up_start_us: exchange.wake_up.start_us,
        wake_up_airtime_us: exchange.wake_up.airtime_us,
        ack_start_us: exchange.ack.map_or(0, |ack| ack.start_us),
        ack_airtime_us: exchange.ack.map_or(0, |ack| ack.airtime_us),
        uplink_start_us: exchange.uplink_start_us,
        wake_up_frame,
        wake_up_len: frame.len() as u8,
        wake_up_frequency_hz: exchange.wake_up.carrier.frequency_hz,
        wake_up_data_rate: exchange.wake_up.carrier.data_rate,
        wake_up_link: link_out(exchange.wake_up.link),
        wake_up_output_dbm: exchange.wake_up.output_dbm,
        has_ack: u8::from(exchange.ack.is_some()),
        ack_frequency_hz: exchange.ack.map_or(0, |ack| ack.carrier.frequency_hz),
        ack_data_rate: exchange.ack.map_or(0, |ack| ack.carrier.data_rate),
        ack_link: exchange
            .ack
            .map_or_else(|| link_out(exchange.wake_up.link), |ack| link_out(ack.link)),
        rxr: window_out(exchange.rxr),
    }
}

/// The exchange field of a frame that goes out on its own.
fn empty_exchange() -> PamojaLorawanRelayExchange {
    PamojaLorawanRelayExchange {
        wake_up_start_us: 0,
        wake_up_airtime_us: 0,
        ack_start_us: 0,
        ack_airtime_us: 0,
        uplink_start_us: 0,
        wake_up_frame: [0; WOR_UPLINK_LEN],
        wake_up_len: 0,
        wake_up_frequency_hz: 0,
        wake_up_data_rate: 0,
        wake_up_link: link_out(LinkSettings::new(7, 125_000)),
        wake_up_output_dbm: 0,
        has_ack: 0,
        ack_frequency_hz: 0,
        ack_data_rate: 0,
        ack_link: link_out(LinkSettings::new(7, 125_000)),
        rxr: PamojaLorawanWindow {
            delay_us: 0,
            frequency_hz: 0,
            link: link_out(LinkSettings::new(7, 125_000)),
            data_rate: 0,
        },
    }
}

/// Runs a call that puts a frame on the air.
///
/// # Safety
///
/// `device` must be a live handle or null, and the out pointers writable or null.
unsafe fn transmit(
    device: *mut PamojaLorawanEndDevice,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
    call: impl FnOnce(&mut EndDevice<'static>) -> Result<Transmission, DeviceError>,
) -> PamojaStatus {
    let (Some(device), false, false) = (
        device.as_mut(),
        out_frame.is_null(),
        out_transmission.is_null(),
    ) else {
        set_last_error("device, out_frame and out_transmission must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_frame = ptr::null_mut();
    let result = call(&mut device.device);
    match device.settle(result) {
        Ok(transmission) => {
            transmitted(transmission, out_frame, out_transmission);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Builds a join request.
///
/// # Arguments
///
/// * `device` - the device.
/// * `dev_nonce` - a nonce this device has never used with its join identifier.
/// * `now_us` - the time, in microseconds.
/// * `out_frame` - receives the frame to transmit.
/// * `out_transmission` - receives where and how to transmit it, and the accept windows.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a buffer the caller must release
/// with [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_end_device_error`]: no
/// credentials, busy, or a wait for the air.
///
/// # Safety
///
/// `device` must be a live handle and the out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_join(
    device: *mut PamojaLorawanEndDevice,
    dev_nonce: u16,
    now_us: u64,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
) -> PamojaStatus {
    transmit(device, out_frame, out_transmission, |device| {
        device.join(dev_nonce, now_us)
    })
}

/// Builds an uplink carrying a payload.
///
/// # Arguments
///
/// * `device` - the device.
/// * `port` - the application port, 1 to 223, or 224 for the certification test port.
/// * `payload` - the application payload.
/// * `payload_len` - its length.
/// * `confirmed` - `1` to ask the network to acknowledge it.
/// * `now_us` - the time, in microseconds.
/// * `out_frame` - receives the frame to transmit.
/// * `out_transmission` - receives where and how to transmit it, and its windows.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a buffer the caller must release
/// with [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for port 0, which belongs to MAC commands, and
/// a failing status with the reason on [`pamoja_lorawan_end_device_error`] otherwise: not
/// joined, busy, a wait for the air, or a payload too long.
///
/// # Safety
///
/// `device` must be a live handle, `payload` must point to `payload_len` readable bytes when
/// that is non-zero, and the out pointers must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_lorawan_end_device_send(
    device: *mut PamojaLorawanEndDevice,
    port: u8,
    payload: *const u8,
    payload_len: usize,
    confirmed: u8,
    now_us: u64,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
) -> PamojaStatus {
    let payload = match read_bytes(payload, payload_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    if port == 0 {
        if let Some(device) = device.as_mut() {
            device.error = Some(DeviceError::Frame(LorawanError::MalformedFrame));
        }
        set_last_error("port 0 carries MAC commands, not an application payload".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    transmit(device, out_frame, out_transmission, |device| {
        device.send(port, &payload, confirmed != 0, now_us)
    })
}

/// Builds an uplink with no payload, carrying the answers the device owes, an
/// acknowledgment, or an ADR acknowledgment request.
///
/// # Arguments
///
/// * `device` - the device.
/// * `now_us` - the time, in microseconds.
/// * `out_frame` - receives the frame to transmit.
/// * `out_transmission` - receives where and how to transmit it, and its windows.
///
/// # Returns
///
/// As [`pamoja_lorawan_end_device_send`].
///
/// # Errors
///
/// As [`pamoja_lorawan_end_device_send`].
///
/// # Safety
///
/// `device` must be a live handle and the out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_send_empty(
    device: *mut PamojaLorawanEndDevice,
    now_us: u64,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
) -> PamojaStatus {
    transmit(device, out_frame, out_transmission, |device| {
        device.send_empty(now_us)
    })
}

/// Sends the last uplink again, the same frame on a channel chosen afresh.
///
/// # Arguments
///
/// * `device` - the device.
/// * `now_us` - the time, in microseconds.
/// * `out_frame` - receives the frame to transmit.
/// * `out_transmission` - receives where and how to transmit it, and its windows.
///
/// # Returns
///
/// As [`pamoja_lorawan_end_device_send`].
///
/// # Errors
///
/// Returns a failing status with the reason on [`pamoja_lorawan_end_device_error`]: nothing
/// due to repeat, or a wait.
///
/// # Safety
///
/// `device` must be a live handle and the out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_repeat(
    device: *mut PamojaLorawanEndDevice,
    now_us: u64,
    out_frame: *mut *mut PamojaBuffer,
    out_transmission: *mut PamojaLorawanTransmission,
) -> PamojaStatus {
    transmit(device, out_frame, out_transmission, |device| {
        device.repeat(now_us)
    })
}

/// Reads a frame heard in one of the receive windows of the last transmission, without saying
/// which.
///
/// A downlink may be as long as the faster of the two windows allows. When the radio knows the
/// window, [`pamoja_lorawan_end_device_heard_in`] holds the frame to that window's own limit.
///
/// # Arguments
///
/// * `device` - the device.
/// * `frame` - the bytes the radio received.
/// * `frame_len` - their length.
/// * `snr_db` - the frame's signal-to-noise ratio, which a `DevStatusAns` reports.
/// * `out_heard` - receives what the frame was.
/// * `out_payload` - receives the application payload of a data frame, and null for a join.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. A payload buffer, when set, must be released with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Errors
///
/// Returns a failing status with the reason on [`pamoja_lorawan_end_device_error`]: nothing
/// pending, another device's frame, a replayed or far-ahead counter, a refused join accept,
/// or a frame that did not decode.
///
/// # Safety
///
/// `device` must be a live handle, `frame` must point to `frame_len` readable bytes when that
/// is non-zero, and the out pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_heard(
    device: *mut PamojaLorawanEndDevice,
    frame: *const u8,
    frame_len: usize,
    snr_db: i8,
    out_heard: *mut PamojaLorawanHeard,
    out_payload: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let (Some(device), false, false) =
        (device.as_mut(), out_heard.is_null(), out_payload.is_null())
    else {
        set_last_error("device, out_heard and out_payload must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    heard_frame(
        device,
        None,
        frame,
        frame_len,
        snr_db,
        out_heard,
        out_payload,
    )
}

/// Reads a frame heard in a given receive window of the last transmission.
///
/// A frame that is not for this device, does not verify, or has a MACPayload longer than the
/// window's data rate carries leaves the transmission waiting, so the second window still
/// opens. TS001-1.0.4 section 4.1 has a device discard such a frame.
///
/// # Arguments
///
/// * `device` - the device.
/// * `window` - [`PAMOJA_LORAWAN_WINDOW_RX1`], [`PAMOJA_LORAWAN_WINDOW_RX2`] or
///   [`PAMOJA_LORAWAN_WINDOW_RXR`].
/// * `frame` - the bytes the radio received.
/// * `frame_len` - their length.
/// * `snr_db` - the frame's signal-to-noise ratio, which a `DevStatusAns` reports.
/// * `out_heard` - receives what the frame was.
/// * `out_payload` - receives the application payload of a data frame, and null for a join.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or [`PamojaStatus::InvalidArgument`] for a window that is
/// neither. A payload buffer, when set, must be released with
/// [`pamoja_buffer_free`](crate::pamoja_buffer_free).
///
/// # Errors
///
/// As [`pamoja_lorawan_end_device_heard`], with a frame longer than the window carries
/// reported as one that did not decode.
///
/// # Safety
///
/// `device` must be a live handle, `frame` must point to `frame_len` readable bytes when that
/// is non-zero, and the out pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_heard_in(
    device: *mut PamojaLorawanEndDevice,
    window: u8,
    frame: *const u8,
    frame_len: usize,
    snr_db: i8,
    out_heard: *mut PamojaLorawanHeard,
    out_payload: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    let (Some(device), false, false) =
        (device.as_mut(), out_heard.is_null(), out_payload.is_null())
    else {
        set_last_error("device, out_heard and out_payload must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some(window) = window_in(window) else {
        set_last_error(format!("{window} is not a receive window"));
        return PamojaStatus::InvalidArgument;
    };
    heard_frame(
        device,
        Some(window),
        frame,
        frame_len,
        snr_db,
        out_heard,
        out_payload,
    )
}

/// Reads a heard frame into the out pointers, which the caller has checked.
unsafe fn heard_frame(
    device: &mut PamojaLorawanEndDevice,
    window: Option<ReceiveWindow>,
    frame: *const u8,
    frame_len: usize,
    snr_db: i8,
    out_heard: *mut PamojaLorawanHeard,
    out_payload: *mut *mut PamojaBuffer,
) -> PamojaStatus {
    *out_payload = ptr::null_mut();
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let result = match window {
        Some(window) => device.device.heard_in(window, &frame, snr_db),
        None => device.device.heard(&frame, snr_db),
    };
    let heard = match device.settle(result) {
        Ok(heard) => heard,
        Err(status) => return status,
    };
    *out_heard = heard_out(heard, device.device.dev_addr().unwrap_or(0), out_payload);
    PamojaStatus::Ok
}

/// What a frame turned out to be, as C sees it, with its payload handed over beside it.
///
/// # Safety
///
/// `out_payload` must be a writable pointer slot.
pub(crate) unsafe fn heard_out(
    heard: Heard,
    dev_addr: u32,
    out_payload: *mut *mut PamojaBuffer,
) -> PamojaLorawanHeard {
    let mut out = PamojaLorawanHeard {
        dev_addr: 0,
        gps_seconds: 0,
        kind: PAMOJA_LORAWAN_HEARD_JOINED,
        has_port: 0,
        port: 0,
        acknowledged: 0,
        confirmed: 0,
        more_pending: 0,
        has_link_check: 0,
        margin_db: 0,
        gateways: 0,
        has_device_time: 0,
        fraction: 0,
    };
    match heard {
        Heard::Joined { dev_addr } => out.dev_addr = dev_addr,
        Heard::Data(delivery) => {
            out.kind = PAMOJA_LORAWAN_HEARD_DATA;
            out.dev_addr = dev_addr;
            out.has_port = u8::from(delivery.port().is_some());
            out.port = delivery.port().unwrap_or(0);
            out.acknowledged = u8::from(delivery.acknowledged());
            out.confirmed = u8::from(delivery.confirmed());
            out.more_pending = u8::from(delivery.more_pending());
            if let Some(check) = delivery.link_check() {
                out.has_link_check = 1;
                out.margin_db = check.margin_db;
                out.gateways = check.gateways;
            }
            if let Some(time) = delivery.device_time() {
                out.has_device_time = 1;
                out.gps_seconds = time.gps_seconds;
                out.fraction = time.fraction;
            }
            *out_payload = PamojaBuffer::into_raw(delivery.payload().to_vec());
        }
    }
    out
}

/// Reads a receive window code.
pub(crate) const fn window_in(window: u8) -> Option<ReceiveWindow> {
    match window {
        PAMOJA_LORAWAN_WINDOW_RX1 => Some(ReceiveWindow::Rx1),
        PAMOJA_LORAWAN_WINDOW_RX2 => Some(ReceiveWindow::Rx2),
        PAMOJA_LORAWAN_WINDOW_RXR => Some(ReceiveWindow::Rxr),
        _ => None,
    }
}

/// What comes next, as its code and the time that goes with it.
pub(crate) const fn next_out(next: Next) -> (u8, u64) {
    match next {
        Next::Repeat { not_before_us } => (PAMOJA_LORAWAN_NEXT_REPEAT, not_before_us),
        Next::Done => (PAMOJA_LORAWAN_NEXT_DONE, 0),
        Next::Unacknowledged => (PAMOJA_LORAWAN_NEXT_UNACKNOWLEDGED, 0),
        Next::JoinAgain { not_before_us } => (PAMOJA_LORAWAN_NEXT_JOIN_AGAIN, not_before_us),
    }
}

/// Says what comes next once both receive windows closed with nothing for the device.
///
/// # Arguments
///
/// * `device` - the device.
/// * `now_us` - the time the second window closed, in microseconds.
/// * `out_next` - receives what to do.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns a failing status with the reason on [`pamoja_lorawan_end_device_error`] when no
/// transmission waits on its windows.
///
/// # Safety
///
/// `device` must be a live handle and `out_next` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_nothing_heard(
    device: *mut PamojaLorawanEndDevice,
    now_us: u64,
    out_next: *mut PamojaLorawanNext,
) -> PamojaStatus {
    let (Some(device), false) = (device.as_mut(), out_next.is_null()) else {
        set_last_error("device and out_next must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let result = device.device.nothing_heard(now_us);
    let next = match device.settle(result) {
        Ok(next) => next,
        Err(status) => return status,
    };
    let (kind, not_before_us) = next_out(next);
    *out_next = PamojaLorawanNext {
        not_before_us,
        kind,
    };
    PamojaStatus::Ok
}

/// Sets what the device reports its battery as, when a network asks.
///
/// # Arguments
///
/// * `device` - the device.
/// * `kind` - one of the `PAMOJA_LORAWAN_BATTERY_*` constants.
/// * `level` - for [`PAMOJA_LORAWAN_BATTERY_LEVEL`], 1 for empty to 254 for full, clamped
///   into that range.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `device` is null or `kind` names nothing.
///
/// # Safety
///
/// `device` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_set_battery(
    device: *mut PamojaLorawanEndDevice,
    kind: u8,
    level: u8,
) -> PamojaStatus {
    let Some(device) = device.as_mut() else {
        set_last_error("device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let battery = match kind {
        PAMOJA_LORAWAN_BATTERY_EXTERNAL => Battery::External,
        PAMOJA_LORAWAN_BATTERY_LEVEL => Battery::Level(level),
        PAMOJA_LORAWAN_BATTERY_UNKNOWN => Battery::Unknown,
        other => {
            set_last_error(format!("{other} is not a battery kind"));
            return PamojaStatus::InvalidArgument;
        }
    };
    device.device.set_battery(battery);
    PamojaStatus::Ok
}

/// Asks the network, with the next uplink, how well it hears the device.
///
/// # Arguments
///
/// * `device` - the device.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `device` is null.
///
/// # Safety
///
/// `device` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_request_link_check(
    device: *mut PamojaLorawanEndDevice,
) -> PamojaStatus {
    let Some(device) = device.as_mut() else {
        set_last_error("device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    device.device.request_link_check();
    PamojaStatus::Ok
}

/// Asks the network, with the next uplink, for the time.
///
/// # Arguments
///
/// * `device` - the device.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `device` is null.
///
/// # Safety
///
/// `device` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_request_device_time(
    device: *mut PamojaLorawanEndDevice,
) -> PamojaStatus {
    let Some(device) = device.as_mut() else {
        set_last_error("device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    device.device.request_device_time();
    PamojaStatus::Ok
}

/// Reads where a device stands.
///
/// # Arguments
///
/// * `device` - the device.
/// * `out_status` - receives the status.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `device` must be a live handle and `out_status` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_status(
    device: *const PamojaLorawanEndDevice,
    out_status: *mut PamojaLorawanEndDeviceStatus,
) -> PamojaStatus {
    let (Some(device), false) = (device.as_ref(), out_status.is_null()) else {
        set_last_error("device and out_status must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_status = status_out(&device.device);
    PamojaStatus::Ok
}

/// Reads one of the channels a device may send on.
///
/// # Arguments
///
/// * `device` - the device.
/// * `position` - the channel's position among the enabled ones, below the count
///   [`pamoja_lorawan_end_device_status`] reports.
/// * `out_index` - receives the channel's index in the device's table.
/// * `out_channel` - receives the channel.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null or `position` is past the
/// last enabled channel.
///
/// # Safety
///
/// `device` must be a live handle and the out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_channel(
    device: *const PamojaLorawanEndDevice,
    position: u16,
    out_index: *mut u16,
    out_channel: *mut PamojaLorawanChannel,
) -> PamojaStatus {
    let (Some(device), false, false) =
        (device.as_ref(), out_index.is_null(), out_channel.is_null())
    else {
        set_last_error("device, out_index and out_channel must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let Some((index, channel)) = device.device.channels().nth(usize::from(position)) else {
        set_last_error(format!(
            "the device has no enabled channel at position {position}"
        ));
        return PamojaStatus::InvalidArgument;
    };
    *out_index = index as u16;
    *out_channel = PamojaLorawanChannel {
        uplink_hz: channel.uplink_hz,
        downlink_hz: channel.downlink_hz,
        min_data_rate: channel.min_data_rate,
        max_data_rate: channel.max_data_rate,
    };
    PamojaStatus::Ok
}

/// Saves a joined device's state, to keep across a loss of power.
///
/// The bytes hold the session keys, so keep them wherever the keys would be safe.
///
/// # Arguments
///
/// * `device` - the device.
/// * `now_us` - the time, in microseconds, which the duty cycle waits are measured from.
/// * `out_saved` - receives the state.
/// * `len` - room at `out_saved`, which must be [`PAMOJA_LORAWAN_SAVED_LEN`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null or `len` is wrong, and a
/// failing status with the reason on [`pamoja_lorawan_end_device_error`] for a device that
/// has not joined or is waiting on its windows.
///
/// # Safety
///
/// `device` must be a live handle and `out_saved` must point to `len` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_save(
    device: *mut PamojaLorawanEndDevice,
    now_us: u64,
    out_saved: *mut u8,
    len: usize,
) -> PamojaStatus {
    let (Some(device), false) = (device.as_mut(), out_saved.is_null()) else {
        set_last_error("device and out_saved must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if len != SAVED_LEN {
        set_last_error(format!("a saved state takes {SAVED_LEN} bytes, not {len}"));
        return PamojaStatus::InvalidArgument;
    }
    let result = device.device.save(now_us);
    match device.settle(result) {
        Ok(saved) => {
            ptr::copy_nonoverlapping(saved.as_bytes().as_ptr(), out_saved, SAVED_LEN);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Puts a saved state back on a device made on the same channel plan.
///
/// # Arguments
///
/// * `device` - the device, made the same way the saved one was.
/// * `saved` - the state [`pamoja_lorawan_end_device_save`] wrote.
/// * `saved_len` - its length.
/// * `now_us` - the time, in microseconds, on the clock the device woke to.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns a failing status with the reason on [`pamoja_lorawan_end_device_error`]: a state
/// of the wrong length, a corrupt one, one in a format this build does not read, one from
/// another plan, or a device busy with a transmission.
///
/// # Safety
///
/// `device` must be a live handle and `saved` must point to `saved_len` readable bytes when
/// that is non-zero.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_resume(
    device: *mut PamojaLorawanEndDevice,
    saved: *const u8,
    saved_len: usize,
    now_us: u64,
) -> PamojaStatus {
    let Some(device) = device.as_mut() else {
        set_last_error("device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let bytes = match read_bytes(saved, saved_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let result = Saved::from_bytes(&bytes)
        .map_err(DeviceError::State)
        .and_then(|saved| device.device.resume(&saved, now_us));
    match device.settle(result) {
        Ok(()) => PamojaStatus::Ok,
        Err(status) => status,
    }
}

/// Turns relay mode on or off, TS011-1.0.1 section 10.2 and appendix 5.
///
/// From this call on, the decision is the caller's rather than the device's own policy,
/// until the network takes it over with `EndDeviceConfReq` or hands it back.
///
/// # Arguments
///
/// * `device` - the device.
/// * `on` - `1` to send through a relay.
/// * `out_taken` - receives `1` when the mode changed, and `0` when the network holds the
///   decision.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle or out pointer.
///
/// # Safety
///
/// `device` must be a live handle and `out_taken` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_use_relay(
    device: *mut PamojaLorawanEndDevice,
    on: u8,
    out_taken: *mut u8,
) -> PamojaStatus {
    let (Some(device), false) = (device.as_mut(), out_taken.is_null()) else {
        set_last_error("device and out_taken must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_taken = u8::from(device.device.use_relay(on != 0));
    PamojaStatus::Ok
}

/// Reports how a device uses a relay, TS011-1.0.1 sections 3.9 and 10.2.
///
/// # Arguments
///
/// * `device` - the device.
/// * `out_relaying` - receives `1` while relay mode is on.
/// * `out_activation` - receives the mode: [`PAMOJA_LORAWAN_RELAY_DISABLED`],
///   [`PAMOJA_LORAWAN_RELAY_ENABLED`], [`PAMOJA_LORAWAN_RELAY_DYNAMIC`] or
///   [`PAMOJA_LORAWAN_RELAY_DEVICE_CONTROLLED`].
/// * `out_sync` - receives what it knows of the relay's scans:
///   [`PAMOJA_LORAWAN_RELAY_INITIALIZED`], [`PAMOJA_LORAWAN_RELAY_UNSYNCHRONIZED`] or
///   [`PAMOJA_LORAWAN_RELAY_SYNCHRONIZED`].
/// * `out_wor_counter` - receives the wake-on-radio frame counter the next frame will use.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. Every out pointer may be null.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle.
///
/// # Safety
///
/// `device` must be a live handle and every non-null out pointer writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_relay_mode(
    device: *mut PamojaLorawanEndDevice,
    out_relaying: *mut u8,
    out_activation: *mut u8,
    out_sync: *mut u8,
    out_wor_counter: *mut u32,
) -> PamojaStatus {
    let Some(device) = device.as_mut() else {
        set_last_error("device must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    if !out_relaying.is_null() {
        *out_relaying = u8::from(device.device.relaying());
    }
    if !out_activation.is_null() {
        *out_activation = match device.device.relay_activation() {
            RelayActivation::Disabled => PAMOJA_LORAWAN_RELAY_DISABLED,
            RelayActivation::Enabled => PAMOJA_LORAWAN_RELAY_ENABLED,
            RelayActivation::Dynamic => PAMOJA_LORAWAN_RELAY_DYNAMIC,
            RelayActivation::DeviceControlled => PAMOJA_LORAWAN_RELAY_DEVICE_CONTROLLED,
        };
    }
    if !out_sync.is_null() {
        *out_sync = match device.device.relay_sync() {
            RelaySync::Initialized => PAMOJA_LORAWAN_RELAY_INITIALIZED,
            RelaySync::Unsynchronized => PAMOJA_LORAWAN_RELAY_UNSYNCHRONIZED,
            RelaySync::Synchronized => PAMOJA_LORAWAN_RELAY_SYNCHRONIZED,
        };
    }
    if !out_wor_counter.is_null() {
        *out_wor_counter = device.device.wor_counter();
    }
    PamojaStatus::Ok
}

/// Returns what the relay's last acknowledgment said about itself, TS011-1.0.1 table 14.
///
/// # Arguments
///
/// * `device` - the device.
/// * `out_status` - receives what the relay said.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] when an acknowledgment has arrived, and
/// [`PamojaStatus::Other`] before one has.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle or out pointer.
///
/// # Safety
///
/// `device` must be a live handle and `out_status` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_relay_status(
    device: *mut PamojaLorawanEndDevice,
    out_status: *mut PamojaLorawanRelayStatus,
) -> PamojaStatus {
    let (Some(device), false) = (device.as_mut(), out_status.is_null()) else {
        set_last_error("device and out_status must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match device.device.relay_status() {
        Some(status) => {
            *out_status = relay_status_out(status);
            PamojaStatus::Ok
        }
        None => {
            set_last_error("no acknowledgment has arrived from a relay".to_owned());
            PamojaStatus::Other
        }
    }
}

/// Reads the acknowledgment a relay answered the last wake-on-radio frame with.
///
/// # Arguments
///
/// * `device` - the device.
/// * `frame` - the bytes the radio received in the acknowledgment window.
/// * `frame_len` - their length.
/// * `out_status` - receives what the relay said about itself.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] when it verifies.
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_end_device_error`]: no
/// frame waiting on an answer, no session, or an acknowledgment that does not verify.
///
/// # Safety
///
/// `device` must be a live handle, `frame` must point to `frame_len` readable bytes, and
/// `out_status` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_heard_wor_ack(
    device: *mut PamojaLorawanEndDevice,
    frame: *const u8,
    frame_len: usize,
    out_status: *mut PamojaLorawanRelayStatus,
) -> PamojaStatus {
    let (Some(device), false) = (device.as_mut(), out_status.is_null()) else {
        set_last_error("device and out_status must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let frame = match read_bytes(frame, frame_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let heard = device.device.heard_wor_ack(&frame);
    match device.settle(heard) {
        Ok(status) => {
            *out_status = relay_status_out(status);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Says what to do once the acknowledgment window closed with nothing in it.
///
/// # Arguments
///
/// * `device` - the device.
/// * `now_us` - the time, in microseconds.
/// * `out_next` - receives [`PAMOJA_LORAWAN_WOR_NEXT_UPLINK`] to send the uplink at the time the
///   exchange named, or [`PAMOJA_LORAWAN_WOR_NEXT_WAKE_UP`] to wake the relay again first.
/// * `out_exchange` - receives the next wake-on-radio exchange, for
///   [`PAMOJA_LORAWAN_WOR_NEXT_WAKE_UP`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns a failing status, with the reason on [`pamoja_lorawan_end_device_error`], when no
/// wake-on-radio frame waits on an answer.
///
/// # Safety
///
/// `device` must be a live handle and the out pointers writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_end_device_no_wor_ack(
    device: *mut PamojaLorawanEndDevice,
    now_us: u64,
    out_next: *mut u8,
    out_exchange: *mut PamojaLorawanRelayExchange,
) -> PamojaStatus {
    let (Some(device), false, false) =
        (device.as_mut(), out_next.is_null(), out_exchange.is_null())
    else {
        set_last_error("device, out_next and out_exchange must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let next = device.device.no_wor_ack(now_us);
    match device.settle(next) {
        Ok(WorNext::Uplink) => {
            *out_next = PAMOJA_LORAWAN_WOR_NEXT_UPLINK;
            *out_exchange = empty_exchange();
            PamojaStatus::Ok
        }
        Ok(WorNext::WakeUp(exchange)) => {
            *out_next = PAMOJA_LORAWAN_WOR_NEXT_WAKE_UP;
            *out_exchange = exchange_out(exchange);
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// What a relay said about itself, as C sees it.
pub(crate) fn relay_status_out(status: RelayStatus) -> PamojaLorawanRelayStatus {
    PamojaLorawanRelayStatus {
        cad_periodicity: status.cad_periodicity.code(),
        xtal_accuracy: status.xtal_accuracy.code(),
        cad_to_rx: status.cad_to_rx.code(),
        relay_data_rate: status.relay_data_rate,
        forward: status.forward.code(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::lora_region::{
        pamoja_lora_plan_for_region, pamoja_lora_plan_free, PAMOJA_LORA_REGION_EU868,
    };
    use crate::lorawan::{pamoja_lorawan_device_free, pamoja_lorawan_device_new};
    use crate::{pamoja_buffer_data, pamoja_buffer_free, pamoja_buffer_len};
    use pamoja_lorawan::{Downlink, JoinGrant};

    const APP_KEY: [u8; 16] = [0x2B; 16];

    /// Copies a buffer out and frees it.
    unsafe fn take(buffer: *mut PamojaBuffer) -> Vec<u8> {
        let bytes =
            std::slice::from_raw_parts(pamoja_buffer_data(buffer), pamoja_buffer_len(buffer))
                .to_vec();
        pamoja_buffer_free(buffer);
        bytes
    }

    #[test]
    fn the_saved_length_is_the_crate_value() {
        assert_eq!(PAMOJA_LORAWAN_SAVED_LEN, SAVED_LEN);
    }

    #[test]
    #[cfg(feature = "eu868")]
    fn a_device_joins_sends_hears_saves_and_resumes_across_the_boundary() {
        unsafe {
            let mut plan = ptr::null_mut();
            assert_eq!(
                pamoja_lora_plan_for_region(PAMOJA_LORA_REGION_EU868, &mut plan),
                PamojaStatus::Ok
            );
            let mut credentials = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_device_new(
                    [0x11; 8].as_ptr(),
                    8,
                    [0x22; 8].as_ptr(),
                    8,
                    APP_KEY.as_ptr(),
                    16,
                    &mut credentials
                ),
                PamojaStatus::Ok
            );
            let mut settings = std::mem::zeroed();
            assert_eq!(
                pamoja_lorawan_device_settings(2, 14, &mut settings),
                PamojaStatus::Ok
            );
            settings.seed = 7;
            let mut device = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_end_device_new(plan, credentials, &settings, 0, 0, 0, &mut device),
                PamojaStatus::Ok
            );

            let mut frame = ptr::null_mut();
            let mut transmission = std::mem::zeroed::<PamojaLorawanTransmission>();
            assert_eq!(
                pamoja_lorawan_end_device_send(
                    device,
                    2,
                    b"21.5".as_ptr(),
                    4,
                    0,
                    0,
                    &mut frame,
                    &mut transmission
                ),
                PamojaStatus::Other
            );
            let mut error = std::mem::zeroed::<PamojaLorawanDeviceError>();
            pamoja_lorawan_end_device_error(device, &mut error);
            assert_eq!(error.kind, PAMOJA_LORAWAN_DEVICE_NOT_JOINED);

            assert_eq!(
                pamoja_lorawan_end_device_join(device, 1, 0, &mut frame, &mut transmission),
                PamojaStatus::Ok
            );
            pamoja_lorawan_end_device_error(device, &mut error);
            assert_eq!(error.kind, PAMOJA_LORAWAN_DEVICE_OK);
            let request = take(frame);
            assert_eq!(request.len(), 23);
            assert_eq!(transmission.rx1.delay_us, 5_000_000);
            assert_eq!(transmission.rx2.frequency_hz, 869_525_000);
            assert_eq!(transmission.rx2.link.crc, 0);

            assert_eq!(
                pamoja_lorawan_end_device_join(device, 2, 0, &mut frame, &mut transmission),
                PamojaStatus::Other
            );
            pamoja_lorawan_end_device_error(device, &mut error);
            assert_eq!(error.kind, PAMOJA_LORAWAN_DEVICE_BUSY);

            let accept = JoinGrant::new(0x01, 0x13, 0x2601_2E43).accept(&APP_KEY, 1);
            let mut heard = std::mem::zeroed::<PamojaLorawanHeard>();
            let mut payload = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_end_device_heard(
                    device,
                    accept.as_bytes().as_ptr(),
                    accept.as_bytes().len(),
                    7,
                    &mut heard,
                    &mut payload
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                (heard.kind, heard.dev_addr),
                (PAMOJA_LORAWAN_HEARD_JOINED, 0x2601_2E43)
            );
            assert!(payload.is_null());

            let mut status = std::mem::zeroed::<PamojaLorawanEndDeviceStatus>();
            pamoja_lorawan_end_device_status(device, &mut status);
            assert_eq!(
                (status.joined, status.dev_addr, status.channel_count),
                (1, 0x2601_2E43, 3)
            );
            let mut index = 0;
            let mut channel = std::mem::zeroed::<PamojaLorawanChannel>();
            assert_eq!(
                pamoja_lorawan_end_device_channel(device, 0, &mut index, &mut channel),
                PamojaStatus::Ok
            );
            assert_eq!((index, channel.uplink_hz), (0, 868_100_000));
            assert_eq!(
                pamoja_lorawan_end_device_channel(device, 3, &mut index, &mut channel),
                PamojaStatus::InvalidArgument
            );

            assert_eq!(
                pamoja_lorawan_end_device_send(
                    device,
                    0,
                    ptr::null(),
                    0,
                    0,
                    7_000_000,
                    &mut frame,
                    &mut transmission
                ),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_lorawan_end_device_send(
                    device,
                    2,
                    b"21.5".as_ptr(),
                    4,
                    1,
                    7_000_000,
                    &mut frame,
                    &mut transmission
                ),
                PamojaStatus::Ok
            );
            let uplink = take(frame);
            assert_eq!(transmission.rx1.delay_us, 1_000_000);
            assert_eq!(transmission.carries_payload, 1);

            // The network acknowledges on port 3 with a payload.
            let down = JoinGrant::new(0x01, 0x13, 0x2601_2E43)
                .session(&APP_KEY, 1)
                .encode_downlink(&Downlink::new(0, 3, b"ok").with_ack())
                .expect("a downlink");
            assert_eq!(
                pamoja_lorawan_end_device_heard(
                    device,
                    down.as_bytes().as_ptr(),
                    down.as_bytes().len(),
                    7,
                    &mut heard,
                    &mut payload
                ),
                PamojaStatus::Ok
            );
            assert_eq!(heard.kind, PAMOJA_LORAWAN_HEARD_DATA);
            assert_eq!((heard.has_port, heard.port, heard.acknowledged), (1, 3, 1));
            assert_eq!(take(payload), b"ok");

            let mut next = std::mem::zeroed::<PamojaLorawanNext>();
            assert_eq!(
                pamoja_lorawan_end_device_nothing_heard(device, 9_000_000, &mut next),
                PamojaStatus::Other
            );
            pamoja_lorawan_end_device_error(device, &mut error);
            assert_eq!(error.kind, PAMOJA_LORAWAN_DEVICE_NOTHING_PENDING);

            let mut saved = vec![0u8; PAMOJA_LORAWAN_SAVED_LEN];
            assert_eq!(
                pamoja_lorawan_end_device_save(device, 9_000_000, saved.as_mut_ptr(), saved.len()),
                PamojaStatus::Ok
            );
            let mut next_uplink = ptr::null_mut();
            let mut next_transmission = std::mem::zeroed::<PamojaLorawanTransmission>();
            assert_eq!(
                pamoja_lorawan_end_device_send(
                    device,
                    2,
                    b"22.0".as_ptr(),
                    4,
                    0,
                    400_000_000,
                    &mut next_uplink,
                    &mut next_transmission
                ),
                PamojaStatus::Ok
            );
            let original = take(next_uplink);

            let mut woken = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_end_device_new(plan, credentials, &settings, 0, 0, 0, &mut woken),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_end_device_resume(woken, saved.as_ptr(), saved.len(), 9_000_000),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_end_device_send(
                    woken,
                    2,
                    b"22.0".as_ptr(),
                    4,
                    0,
                    400_000_000,
                    &mut next_uplink,
                    &mut next_transmission
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                take(next_uplink),
                original,
                "the woken device sends the same frame"
            );
            assert_ne!(uplink, original);

            // A 60-byte MACPayload is more than the second window's DR0 carries, but not
            // more than the first window's.
            let long = JoinGrant::new(0x01, 0x13, 0x2601_2E43)
                .session(&APP_KEY, 1)
                .encode_downlink(&Downlink::new(1, 3, &[0x5A; 52]))
                .expect("a downlink");
            let heard_long = |window| {
                let mut heard = std::mem::zeroed::<PamojaLorawanHeard>();
                let mut payload = ptr::null_mut();
                let status = pamoja_lorawan_end_device_heard_in(
                    woken,
                    window,
                    long.as_bytes().as_ptr(),
                    long.as_bytes().len(),
                    7,
                    &mut heard,
                    &mut payload,
                );
                (status, payload)
            };
            assert_eq!(
                heard_long(9).0,
                PamojaStatus::InvalidArgument,
                "a code that names no window"
            );
            assert_eq!(
                heard_long(PAMOJA_LORAWAN_WINDOW_RX2).0,
                PamojaStatus::InvalidArgument
            );
            pamoja_lorawan_end_device_error(woken, &mut error);
            assert_eq!(error.kind, PAMOJA_LORAWAN_DEVICE_FRAME);
            let (status, payload) = heard_long(PAMOJA_LORAWAN_WINDOW_RX1);
            assert_eq!(status, PamojaStatus::Ok);
            assert_eq!(take(payload), [0x5A; 52]);

            let mut blank = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_end_device_new(plan, credentials, &settings, 0, 0, 0, &mut blank),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_end_device_resume(blank, saved.as_ptr(), 10, 0),
                PamojaStatus::InvalidArgument
            );
            pamoja_lorawan_end_device_error(blank, &mut error);
            assert_eq!(
                (error.kind, error.state),
                (PAMOJA_LORAWAN_DEVICE_STATE, PAMOJA_LORAWAN_STATE_LENGTH)
            );
            saved[20] ^= 0xFF;
            assert_eq!(
                pamoja_lorawan_end_device_resume(blank, saved.as_ptr(), saved.len(), 0),
                PamojaStatus::Codec
            );
            pamoja_lorawan_end_device_error(blank, &mut error);
            assert_eq!(error.state, PAMOJA_LORAWAN_STATE_CORRUPT);

            pamoja_lorawan_end_device_free(blank);
            pamoja_lorawan_end_device_free(woken);
            pamoja_lorawan_end_device_free(device);
            pamoja_lorawan_device_free(credentials);
            pamoja_lora_plan_free(plan);
        }
    }
}
