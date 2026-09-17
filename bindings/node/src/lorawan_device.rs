//! Generated Node bindings for a LoRaWAN Class A end device, without a radio.
//!
//! [`LorawanEndDevice`] joins, chooses a channel and data rate for each uplink, says when and
//! where to listen for the answer, reads what comes back, and does what the network's MAC
//! commands ask. It owns no radio and no clock: every call takes the time in microseconds and
//! hands back what to put on the air, so the same device runs over any radio, or none.
//!
//! A device runs on a published channel plan, `LoraChannelPlan.forRegion` or
//! `LoraChannelPlan.forCn470`. A call that cannot be done throws an `Error` whose `code` names
//! why, such as `Wait`, with what goes with it, such as `untilUs`.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use pamoja_lorawan::device::{
    Battery, DeviceError, EndDevice, Heard, Next, ReceiveWindow, RelayExchange, RelayStatus, Saved,
    Settings, StateError, Transmission, Window, WorNext,
};
use pamoja_lorawan::relay::{RelayActivation, RelaySync};
use pamoja_lorawan::Version;

use crate::lora::{lora_link_of, LoraLink};
use crate::lora_region::LoraChannelPlan;
use crate::lorawan::{LorawanDevice, LorawanSession};
use crate::lorawan_link::LorawanVersion;
use crate::lorawan_relay::{
    forward_out, periodicity_out, receive_out, xtal_out, LorawanCadPeriodicity, LorawanCadToRx,
    LorawanRelayForward, LorawanXtalAccuracy,
};

/// What a device's radio can do, and how it takes part.
///
/// Only the output power range is required. The rest start as a typical node: TS001-1.0.4,
/// adaptive data rate on, an antenna with no gain over its cable, a radio that tunes 137 to
/// 1020 MHz as an SX1276 does, the region's duty cycle kept, and no repeater in the path.
#[napi(object)]
pub struct LorawanDeviceSettings {
    /// The lowest power the radio puts out, conducted, in dBm.
    pub min_output_dbm: i32,
    /// The highest, conducted, in dBm.
    pub max_output_dbm: i32,
    /// The link layer revision the network was told the device follows.
    pub version: Option<LorawanVersion>,
    /// Whether the network manages the data rate and power.
    pub adr: Option<bool>,
    /// The antenna gain less the cable and connector losses, in dB.
    pub antenna_gain_db: Option<i32>,
    /// The lowest frequency the radio and its front end can use, in hertz.
    pub lowest_hz: Option<u32>,
    /// The highest, in hertz.
    pub highest_hz: Option<u32>,
    /// Whether to hold the device to the region's sub-band duty cycles.
    pub regional_duty_cycle: Option<bool>,
    /// Whether to size payloads for a path through a relay.
    pub behind_repeater: Option<bool>,
    /// A seed for the random choices of channel and retry delay, ideally from a hardware
    /// random source. The device identifier is mixed in.
    pub seed: Option<u32>,
}

/// The frame counters a device carries over a restart.
#[napi(object)]
pub struct LorawanFrameCounters {
    /// The next uplink frame counter.
    pub up: u32,
    /// The last downlink frame counter accepted, if any was.
    pub down: Option<u32>,
}

/// When and where to listen for a downlink.
#[napi(object)]
pub struct LorawanWindow {
    /// How long after the end of the transmission the window opens, in microseconds.
    pub delay_us: u32,
    /// The carrier, in hertz.
    pub frequency_hz: u32,
    /// The downlink data rate, as the region numbers them.
    pub data_rate: u8,
    /// The LoRa settings to listen with: no payload CRC, and inverted IQ, as RP002-1.0.5
    /// table 112 has for a downlink.
    pub link: LoraLink,
}

/// The frame that wakes a relay, and where it goes.
#[napi(object)]
pub struct LorawanWakeUp {
    /// The frame: five bytes ahead of a join request, fifteen ahead of an uplink.
    pub frame: Buffer,
    /// When to start sending it, in microseconds.
    pub start_us: f64,
    /// Where it goes, in hertz.
    pub frequency_hz: u32,
    /// The data rate it goes out at.
    pub data_rate: u8,
    /// Its LoRa settings, with the preamble this frame needs, sent with inverted IQ.
    pub link: LoraLink,
    /// The power to ask of the radio, conducted, in dBm.
    pub output_dbm: i32,
    /// How long it holds the air, in microseconds.
    pub airtime_us: f64,
}

/// When and where a relay's acknowledgment would arrive.
#[napi(object)]
pub struct LorawanAckWindow {
    /// When it starts, in microseconds.
    pub start_us: f64,
    /// Where it arrives, in hertz.
    pub frequency_hz: u32,
    /// The data rate it arrives at.
    pub data_rate: u8,
    /// The LoRa settings to listen with.
    pub link: LoraLink,
    /// How long it lasts, in microseconds.
    pub airtime_us: f64,
}

/// The wake-on-radio exchange an uplink under a relay goes out behind, TS011-1.0.1
/// section 5.
#[napi(object)]
pub struct LorawanRelayExchange {
    /// The frame that wakes the relay.
    pub wake_up: LorawanWakeUp,
    /// Where the relay's acknowledgment would arrive, or `null` ahead of a join request,
    /// which no relay acknowledges.
    pub ack: Option<LorawanAckWindow>,
    /// When the uplink itself goes out, in microseconds, whether or not the acknowledgment
    /// arrives.
    pub uplink_start_us: f64,
    /// The relay window, timed from the end of the uplink like the other two.
    pub rxr: LorawanWindow,
}

/// What a relay's acknowledgment said about itself, TS011-1.0.1 table 14.
#[napi(object)]
pub struct LorawanRelayStatus {
    /// How often it scans.
    pub cad_periodicity: LorawanCadPeriodicity,
    /// How accurate its crystal is.
    pub xtal_accuracy: LorawanXtalAccuracy,
    /// How long it takes to start receiving.
    pub cad_to_rx: LorawanCadToRx,
    /// The data rate it forwards at, which bounds what the device may send.
    pub relay_data_rate: u8,
    /// Whether it will forward.
    pub forward: LorawanRelayForward,
}

/// How an end device decides whether to send through a relay, TS011-1.0.1 section 10.2.
#[napi(string_enum)]
pub enum LorawanRelayActivation {
    /// Never, the default.
    Disabled,
    /// Always.
    Enabled,
    /// Only after `smartEnableLevel` uplinks in a row went unanswered.
    Dynamic,
    /// However the device itself decides, which `useRelay` sets.
    DeviceControlled,
}

/// What an end device knows of its relay's scans, TS011-1.0.1 section 3.9.
#[napi(string_enum)]
pub enum LorawanRelaySync {
    /// Nothing: no wake-on-radio frame has gone out yet.
    Initialized,
    /// It sent one, but no acknowledgment came back, so its preamble spans a whole scan
    /// period.
    Unsynchronized,
    /// It knows when the relay scans, so a short preamble reaches it.
    Synchronized,
}

/// What an end device does once its wake-on-radio frame went unanswered.
#[napi(object)]
pub struct LorawanWorNext {
    /// `true` to send the uplink at the time the exchange named anyway.
    pub uplink: bool,
    /// The next exchange, when the relay is woken again first.
    pub wake_up: Option<LorawanRelayExchange>,
}

/// A frame to put on the air, and where to listen afterward.
#[napi(object)]
pub struct LorawanTransmission {
    /// The frame.
    pub frame: Buffer,
    /// The carrier, in hertz.
    pub frequency_hz: u32,
    /// The data rate, as the region numbers them.
    pub data_rate: u8,
    /// The LoRa settings: an eight-symbol preamble, an explicit header and a payload CRC,
    /// sent with standard IQ.
    pub link: LoraLink,
    /// The power to ask of the radio, conducted, in dBm.
    pub output_dbm: i32,
    /// How long the frame holds the air, in microseconds.
    pub airtime_us: f64,
    /// The first receive window.
    pub rx1: LorawanWindow,
    /// The second, which opens only if nothing for this device arrived in the first.
    pub rx2: LorawanWindow,
    /// Whether the application payload went out in this frame. When the answers the device
    /// owed left no room, it did not, and has to be sent again.
    pub carries_payload: bool,
    /// The wake-on-radio exchange this frame goes out behind, for a device under a relay.
    pub relay: Option<LorawanRelayExchange>,
}

/// How well the network heard a link check.
#[napi(object)]
pub struct LorawanLinkCheck {
    /// How far above the demodulation floor the best gateway heard it, in dB.
    pub margin_db: u8,
    /// How many gateways heard it.
    pub gateways: u8,
}

/// The time a network gave a device.
#[napi(object)]
pub struct LorawanDeviceTime {
    /// Whole seconds since the GPS epoch, 1980-01-06 00:00:00 UTC, at the end of the uplink
    /// that asked.
    pub gps_seconds: u32,
    /// The fraction of a second, in 256ths.
    pub fraction: u8,
}

/// A downlink, read and acted on.
#[napi(object)]
pub struct LorawanDelivery {
    /// The application port the payload arrived on, or `null` for a frame that carried only
    /// MAC commands or nothing.
    pub port: Option<u8>,
    /// The application payload, decrypted.
    pub payload: Buffer,
    /// Whether the network acknowledged the confirmed uplink this answered.
    pub acknowledged: bool,
    /// Whether the network asked for this downlink to be acknowledged, which the next uplink
    /// does by itself.
    pub confirmed: bool,
    /// Whether the network has more waiting.
    pub more_pending: bool,
    /// The answer to a link check the device asked for.
    pub link_check: Option<LorawanLinkCheck>,
    /// The answer to a time request the device asked for.
    pub device_time: Option<LorawanDeviceTime>,
}

/// What a frame heard in a receive window turned out to be.
/// Which receive window a frame arrived in.
#[napi(string_enum)]
pub enum LorawanReceiveWindow {
    /// The first window, on the uplink's downlink channel.
    Rx1,
    /// The second, on the fixed frequency and data rate.
    Rx2,
    /// The relay window, which a device under a relay opens last, TS011-1.0.1 chapter 7.
    Rxr,
}

#[napi(string_enum)]
pub enum LorawanHeardKind {
    /// A join accept: the device is on the network.
    Joined,
    /// A data frame for this device.
    Data,
}

/// What a frame heard in a receive window turned out to be.
#[napi(object)]
pub struct LorawanHeard {
    /// A join or a data frame.
    pub kind: LorawanHeardKind,
    /// The address the device is on the network by.
    pub dev_addr: u32,
    /// For a data frame, what it carried.
    pub delivery: Option<LorawanDelivery>,
}

/// What to do once both receive windows closed with nothing for the device.
#[napi(string_enum)]
pub enum LorawanNextKind {
    /// Send the same frame again with `repeat`, no sooner than `notBeforeUs`.
    Repeat,
    /// The uplink is finished.
    Done,
    /// A confirmed uplink went out every time it may without an acknowledgment.
    Unacknowledged,
    /// The join got no answer; join again with a new nonce, no sooner than `notBeforeUs`.
    JoinAgain,
}

/// What to do once both receive windows closed with nothing for the device.
#[napi(object)]
pub struct LorawanNext {
    /// What to do.
    pub kind: LorawanNextKind,
    /// For a repeat or another join, the earliest time to send, in microseconds.
    pub not_before_us: Option<f64>,
}

/// A channel a device may send on.
#[napi(object)]
pub struct LorawanChannel {
    /// The channel's index in the device's table.
    pub index: u32,
    /// Where uplinks go out, in hertz.
    pub uplink_hz: u32,
    /// Where the first receive window listens, in hertz.
    pub downlink_hz: u32,
    /// The slowest data rate the channel carries.
    pub min_data_rate: u8,
    /// The fastest.
    pub max_data_rate: u8,
}

/// Where the second receive window listens.
#[napi(object)]
pub struct LorawanRx2 {
    /// The frequency, in hertz.
    pub frequency_hz: u32,
    /// The data rate.
    pub data_rate: u8,
}

/// The lowest and highest frequency a device transmits or listens on.
#[napi(object)]
pub struct LorawanFrequencySpan {
    /// The lowest, in hertz.
    pub lowest_hz: u32,
    /// The highest, in hertz.
    pub highest_hz: u32,
}

/// A LoRaWAN Class A end device.
///
/// One exchange runs like this: `join` or `send` returns a transmission to put on the air
/// and two receive windows timed from its end. A frame heard in either window goes to
/// `heard`. If neither window held one, `nothingHeard` says whether to send the same frame
/// again with `repeat`, or move on.
#[napi]
pub struct LorawanEndDevice {
    inner: EndDevice<'static>,
}

#[napi]
impl LorawanEndDevice {
    /// Makes a device that joins over the air.
    ///
    /// Throws if the plan was built rather than published, or the settings run backward.
    #[napi(factory)]
    pub fn over_the_air(
        env: Env,
        plan: &LoraChannelPlan,
        credentials: &LorawanDevice,
        settings: LorawanDeviceSettings,
        counters: Option<LorawanFrameCounters>,
    ) -> Result<Self> {
        let (plan, settings) = made_from(plan, &settings)?;
        let device = EndDevice::new(plan, credentials.inner.clone(), settings)
            .map_err(|error| thrown(&env, error))?;
        Ok(Self {
            inner: with_counters(device, counters),
        })
    }

    /// Makes a device activated by personalization, with its session provisioned.
    ///
    /// Such a device never resets its frame counters, TS001-1.0.4 section 4.3.1.5, so one that
    /// lost power passes the counters it kept.
    #[napi(factory)]
    pub fn personalized(
        env: Env,
        plan: &LoraChannelPlan,
        session: &LorawanSession,
        settings: LorawanDeviceSettings,
        counters: Option<LorawanFrameCounters>,
    ) -> Result<Self> {
        let (plan, settings) = made_from(plan, &settings)?;
        let device = EndDevice::personalized(plan, session.inner, settings)
            .map_err(|error| thrown(&env, error))?;
        Ok(Self {
            inner: with_counters(device, counters),
        })
    }

    /// Builds a join request, with a nonce this device has never used with its join
    /// identifier.
    #[napi]
    pub fn join(&mut self, env: Env, dev_nonce: u16, now_us: f64) -> Result<LorawanTransmission> {
        let now_us = micros(now_us)?;
        self.inner
            .join(dev_nonce, now_us)
            .map(transmission_out)
            .map_err(|error| thrown(&env, error))
    }

    /// Builds an uplink carrying a payload on an application port, 1 to 223, or 224 for the
    /// certification test port.
    #[napi]
    pub fn send(
        &mut self,
        env: Env,
        port: u8,
        payload: Buffer,
        confirmed: bool,
        now_us: f64,
    ) -> Result<LorawanTransmission> {
        let now_us = micros(now_us)?;
        self.inner
            .send(port, payload.as_ref(), confirmed, now_us)
            .map(transmission_out)
            .map_err(|error| thrown(&env, error))
    }

    /// Builds an uplink with no payload, carrying the answers the device owes, an
    /// acknowledgment, or an ADR acknowledgment request.
    #[napi]
    pub fn send_empty(&mut self, env: Env, now_us: f64) -> Result<LorawanTransmission> {
        let now_us = micros(now_us)?;
        self.inner
            .send_empty(now_us)
            .map(transmission_out)
            .map_err(|error| thrown(&env, error))
    }

    /// Sends the last uplink again, the same frame on a channel chosen afresh.
    #[napi]
    pub fn repeat(&mut self, env: Env, now_us: f64) -> Result<LorawanTransmission> {
        let now_us = micros(now_us)?;
        self.inner
            .repeat(now_us)
            .map(transmission_out)
            .map_err(|error| thrown(&env, error))
    }

    /// Reads a frame heard in one of the receive windows of the last transmission.
    ///
    /// A frame that is not for this device, does not verify, or is longer than the window's
    /// data rate carries throws and leaves the transmission waiting, so the second window
    /// still opens. Without `window`, a frame may be as long as the faster window allows.
    #[napi]
    pub fn heard(
        &mut self,
        env: Env,
        frame: Buffer,
        snr_db: i32,
        window: Option<LorawanReceiveWindow>,
    ) -> Result<LorawanHeard> {
        let snr_db = snr_db.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
        let heard = match window.map(window_in) {
            Some(window) => self.inner.heard_in(window, frame.as_ref(), snr_db),
            None => self.inner.heard(frame.as_ref(), snr_db),
        }
        .map_err(|error| thrown(&env, error))?;
        Ok(heard_out(heard, self.inner.dev_addr().unwrap_or(0)))
    }

    /// Says what comes next once both receive windows closed with nothing for the device.
    #[napi]
    pub fn nothing_heard(&mut self, env: Env, now_us: f64) -> Result<LorawanNext> {
        let now_us = micros(now_us)?;
        let next = self
            .inner
            .nothing_heard(now_us)
            .map_err(|error| thrown(&env, error))?;
        Ok(next_out(next))
    }

    /// Sets what the device reports its battery as when a network asks: a level from 1,
    /// empty, to 254, full, `"external"` for a device on external power, or `null` when it
    /// cannot tell.
    #[napi]
    pub fn set_battery(&mut self, battery: Option<Either<u32, String>>) -> Result<()> {
        let battery = match battery {
            None => Battery::Unknown,
            Some(Either::A(level)) => Battery::Level(level.min(255) as u8),
            Some(Either::B(word)) if word == "external" => Battery::External,
            Some(Either::B(word)) if word == "unknown" => Battery::Unknown,
            Some(Either::B(word)) => {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("{word} is not a battery; pass a level, \"external\" or \"unknown\""),
                ))
            }
        };
        self.inner.set_battery(battery);
        Ok(())
    }

    /// Asks the network, with the next uplink, how well it hears the device.
    #[napi]
    pub fn request_link_check(&mut self) {
        self.inner.request_link_check();
    }

    /// Asks the network, with the next uplink, for the time.
    #[napi]
    pub fn request_device_time(&mut self) {
        self.inner.request_device_time();
    }

    /// Whether the device is on a network: once joined, or from the start for a
    /// personalized device.
    #[napi(getter)]
    pub fn is_joined(&self) -> bool {
        self.inner.is_joined()
    }

    /// The address the device is on the network by, or `null` before joining.
    #[napi(getter)]
    pub fn dev_addr(&self) -> Option<u32> {
        self.inner.dev_addr()
    }

    /// The data rate the next uplink goes out at, before any back-off step.
    #[napi(getter)]
    pub fn data_rate(&self) -> u8 {
        self.inner.data_rate()
    }

    /// The next uplink frame counter.
    #[napi(getter)]
    pub fn fcnt_up(&self) -> u32 {
        self.inner.fcnt_up()
    }

    /// The last downlink frame counter accepted, or `null` before any downlink.
    #[napi(getter)]
    pub fn fcnt_down(&self) -> Option<u32> {
        self.inner.fcnt_down()
    }

    /// How many times each uplink goes out, as the network last set it.
    #[napi(getter)]
    pub fn transmissions(&self) -> u8 {
        self.inner.transmissions()
    }

    /// Where the second receive window listens.
    #[napi(getter, js_name = "rx2")]
    pub fn rx2(&self) -> LorawanRx2 {
        let (frequency_hz, data_rate) = self.inner.rx2();
        LorawanRx2 {
            frequency_hz,
            data_rate,
        }
    }

    /// The delay from the end of an uplink to the first receive window, in microseconds.
    #[napi(getter)]
    pub fn receive_delay_us(&self) -> u32 {
        self.inner.receive_delay_us()
    }

    /// The lowest and highest frequency the device transmits or listens on, which is the
    /// band a radio that calibrates for one, as an SX126x does, calibrates for.
    #[napi(getter)]
    pub fn frequency_span(&self) -> LorawanFrequencySpan {
        let (lowest_hz, highest_hz) = self.inner.frequency_span();
        LorawanFrequencySpan {
            lowest_hz,
            highest_hz,
        }
    }

    /// The channels the device may send on.
    #[napi]
    pub fn channels(&self) -> Vec<LorawanChannel> {
        self.inner
            .channels()
            .map(|(index, channel)| LorawanChannel {
                index: index as u32,
                uplink_hz: channel.uplink_hz,
                downlink_hz: channel.downlink_hz,
                min_data_rate: channel.min_data_rate,
                max_data_rate: channel.max_data_rate,
            })
            .collect()
    }

    /// Saves a joined device's state, to keep across a loss of power.
    ///
    /// The bytes hold the session keys, so keep them wherever the keys would be safe.
    #[napi]
    pub fn save(&self, env: Env, now_us: f64) -> Result<Buffer> {
        let now_us = micros(now_us)?;
        self.inner
            .save(now_us)
            .map(|saved| saved.as_bytes().to_vec().into())
            .map_err(|error| thrown(&env, error))
    }

    /// Puts a saved state back on a device made the same way, on the clock it woke to.
    #[napi]
    pub fn resume(&mut self, env: Env, saved: Buffer, now_us: f64) -> Result<()> {
        let now_us = micros(now_us)?;
        let result = Saved::from_bytes(saved.as_ref())
            .map_err(DeviceError::State)
            .and_then(|saved| self.inner.resume(&saved, now_us));
        result.map_err(|error| thrown(&env, error))
    }

    /// Turns relay mode on or off, TS011-1.0.1 section 10.2 and appendix 5.
    ///
    /// From here on the decision is the caller's rather than the device's own policy, until
    /// its network takes it over with `EndDeviceConfReq` or hands it back. Returns `false`
    /// when the network holds the decision, leaving the mode as it was.
    #[napi]
    pub fn use_relay(&mut self, on: bool) -> bool {
        self.inner.use_relay(on)
    }

    /// Whether the next uplink goes through a relay.
    #[napi(getter)]
    pub fn relaying(&self) -> bool {
        self.inner.relaying()
    }

    /// How the device decides whether to use a relay.
    #[napi(getter)]
    pub fn relay_activation(&self) -> LorawanRelayActivation {
        activation_out(self.inner.relay_activation())
    }

    /// What the device knows of when its relay listens, TS011-1.0.1 section 3.9.
    #[napi(getter)]
    pub fn relay_sync(&self) -> LorawanRelaySync {
        sync_out(self.inner.relay_sync())
    }

    /// What the relay's last acknowledgment said about itself, or `null` before one
    /// arrived.
    #[napi(getter)]
    pub fn relay_status(&self) -> Option<LorawanRelayStatus> {
        self.inner.relay_status().map(relay_status_out)
    }

    /// The wake-on-radio frame counter the next frame will use, TS011-1.0.1 section 5.3.2.
    #[napi(getter)]
    pub fn wor_counter(&self) -> u32 {
        self.inner.wor_counter()
    }

    /// Reads the acknowledgment a relay answered the last wake-on-radio frame with.
    ///
    /// The device is now synchronized: it knows when the relay scans, so its next frames
    /// carry only as much preamble as the two clocks could have drifted apart.
    #[napi]
    pub fn heard_wor_ack(&mut self, env: Env, frame: Buffer) -> Result<LorawanRelayStatus> {
        self.inner
            .heard_wor_ack(frame.as_ref())
            .map(relay_status_out)
            .map_err(|error| thrown(&env, error))
    }

    /// Says what to do once the acknowledgment window closed with nothing in it: send the
    /// uplink anyway, or wake the relay again first, as the network's `BackOff` asks.
    #[napi]
    pub fn no_wor_ack(&mut self, env: Env, now_us: f64) -> Result<LorawanWorNext> {
        let now_us = micros(now_us)?;
        match self
            .inner
            .no_wor_ack(now_us)
            .map_err(|error| thrown(&env, error))?
        {
            WorNext::Uplink => Ok(LorawanWorNext {
                uplink: true,
                wake_up: None,
            }),
            WorNext::WakeUp(exchange) => Ok(LorawanWorNext {
                uplink: false,
                wake_up: Some(exchange_out(exchange)),
            }),
        }
    }
}

/// Reads the published plan and settings a device is made from.
pub(crate) fn made_from(
    plan: &LoraChannelPlan,
    settings: &LorawanDeviceSettings,
) -> Result<(&'static pamoja_lora::region::ChannelPlan<'static>, Settings)> {
    let published = plan.published_plan().ok_or_else(|| {
        Error::new(
            Status::InvalidArg,
            "a device runs on a published plan, from LoraChannelPlan.forRegion or forCn470"
                .to_owned(),
        )
    })?;
    let byte = |value: i32, what: &str| {
        i8::try_from(value).map_err(|_| {
            Error::new(
                Status::InvalidArg,
                format!("{what} of {value} dB is out of range"),
            )
        })
    };
    let min = byte(settings.min_output_dbm, "minOutputDbm")?;
    let max = byte(settings.max_output_dbm, "maxOutputDbm")?;
    let lowest_hz = settings.lowest_hz.unwrap_or(137_000_000);
    let highest_hz = settings.highest_hz.unwrap_or(1_020_000_000);
    if min > max || lowest_hz > highest_hz {
        return Err(Error::new(
            Status::InvalidArg,
            "the output power and tuning ranges must run from low to high".to_owned(),
        ));
    }
    let version = match settings.version {
        Some(LorawanVersion::V1_0_3) => Version::V1_0_3,
        Some(LorawanVersion::V1_0_4) | None => Version::V1_0_4,
    };
    let mut built = Settings::new(min, max)
        .with_version(version)
        .with_adr(settings.adr.unwrap_or(true))
        .with_antenna_gain(byte(
            settings.antenna_gain_db.unwrap_or(0),
            "antennaGainDb",
        )?)
        .with_tuning_range(lowest_hz, highest_hz)
        .with_seed(settings.seed.unwrap_or(0));
    if !settings.regional_duty_cycle.unwrap_or(true) {
        built = built.without_regional_duty_cycle();
    }
    if settings.behind_repeater.unwrap_or(false) {
        built = built.behind_repeater();
    }
    Ok((published, built))
}

/// Applies the frame counters a caller carried over.
fn with_counters(
    device: EndDevice<'static>,
    counters: Option<LorawanFrameCounters>,
) -> EndDevice<'static> {
    match counters {
        Some(counters) => device.with_frame_counters(counters.up, counters.down),
        None => device,
    }
}

/// Reads a JavaScript time in microseconds.
pub(crate) fn micros(value: f64) -> Result<u64> {
    if (0.0..=9_007_199_254_740_991.0).contains(&value) {
        Ok(value as u64)
    } else {
        Err(Error::new(
            Status::InvalidArg,
            format!("{value} is not a time in microseconds"),
        ))
    }
}

/// Describes a receive window the way JavaScript holds it.
fn window_out(window: Window) -> LorawanWindow {
    LorawanWindow {
        delay_us: window.delay_us,
        frequency_hz: window.frequency_hz,
        data_rate: window.data_rate,
        link: lora_link_of(window.link),
    }
}

/// Describes what a frame turned out to be the way JavaScript holds it.
pub(crate) fn heard_out(heard: Heard, dev_addr: u32) -> LorawanHeard {
    match heard {
        Heard::Joined { dev_addr } => LorawanHeard {
            kind: LorawanHeardKind::Joined,
            dev_addr,
            delivery: None,
        },
        Heard::Data(delivery) => LorawanHeard {
            kind: LorawanHeardKind::Data,
            dev_addr,
            delivery: Some(LorawanDelivery {
                port: delivery.port(),
                payload: delivery.payload().to_vec().into(),
                acknowledged: delivery.acknowledged(),
                confirmed: delivery.confirmed(),
                more_pending: delivery.more_pending(),
                link_check: delivery.link_check().map(|check| LorawanLinkCheck {
                    margin_db: check.margin_db,
                    gateways: check.gateways,
                }),
                device_time: delivery.device_time().map(|time| LorawanDeviceTime {
                    gps_seconds: time.gps_seconds,
                    fraction: time.fraction,
                }),
            }),
        },
    }
}

/// Describes what comes next the way JavaScript holds it.
pub(crate) fn next_out(next: Next) -> LorawanNext {
    match next {
        Next::Repeat { not_before_us } => LorawanNext {
            kind: LorawanNextKind::Repeat,
            not_before_us: Some(not_before_us as f64),
        },
        Next::Done => LorawanNext {
            kind: LorawanNextKind::Done,
            not_before_us: None,
        },
        Next::Unacknowledged => LorawanNext {
            kind: LorawanNextKind::Unacknowledged,
            not_before_us: None,
        },
        Next::JoinAgain { not_before_us } => LorawanNext {
            kind: LorawanNextKind::JoinAgain,
            not_before_us: Some(not_before_us as f64),
        },
    }
}

/// Reads the window a frame arrived in.
pub(crate) fn window_in(window: LorawanReceiveWindow) -> ReceiveWindow {
    match window {
        LorawanReceiveWindow::Rx1 => ReceiveWindow::Rx1,
        LorawanReceiveWindow::Rx2 => ReceiveWindow::Rx2,
        LorawanReceiveWindow::Rxr => ReceiveWindow::Rxr,
    }
}

/// The mode a device manages its relay by, as JavaScript names it.
pub(crate) fn activation_out(activation: RelayActivation) -> LorawanRelayActivation {
    match activation {
        RelayActivation::Disabled => LorawanRelayActivation::Disabled,
        RelayActivation::Enabled => LorawanRelayActivation::Enabled,
        RelayActivation::Dynamic => LorawanRelayActivation::Dynamic,
        RelayActivation::DeviceControlled => LorawanRelayActivation::DeviceControlled,
    }
}

/// What a device knows of its relay's scans, as JavaScript names it.
pub(crate) fn sync_out(sync: RelaySync) -> LorawanRelaySync {
    match sync {
        RelaySync::Initialized => LorawanRelaySync::Initialized,
        RelaySync::Unsynchronized => LorawanRelaySync::Unsynchronized,
        RelaySync::Synchronized => LorawanRelaySync::Synchronized,
    }
}

/// Describes a transmission the way JavaScript holds it.
pub(crate) fn transmission_out(transmission: Transmission) -> LorawanTransmission {
    LorawanTransmission {
        frame: transmission.frame.as_bytes().to_vec().into(),
        frequency_hz: transmission.frequency_hz,
        data_rate: transmission.data_rate,
        link: lora_link_of(transmission.link),
        output_dbm: i32::from(transmission.output_dbm),
        airtime_us: transmission.airtime_us as f64,
        rx1: window_out(transmission.rx1),
        rx2: window_out(transmission.rx2),
        carries_payload: transmission.carries_payload,
        relay: transmission.relay.map(exchange_out),
    }
}

/// Describes a wake-on-radio exchange the way JavaScript holds it.
pub(crate) fn exchange_out(exchange: RelayExchange) -> LorawanRelayExchange {
    LorawanRelayExchange {
        wake_up: LorawanWakeUp {
            frame: exchange.wake_up.frame().to_vec().into(),
            start_us: exchange.wake_up.start_us as f64,
            frequency_hz: exchange.wake_up.carrier.frequency_hz,
            data_rate: exchange.wake_up.carrier.data_rate,
            link: lora_link_of(exchange.wake_up.link),
            output_dbm: i32::from(exchange.wake_up.output_dbm),
            airtime_us: exchange.wake_up.airtime_us as f64,
        },
        ack: exchange.ack.map(|ack| LorawanAckWindow {
            start_us: ack.start_us as f64,
            frequency_hz: ack.carrier.frequency_hz,
            data_rate: ack.carrier.data_rate,
            link: lora_link_of(ack.link),
            airtime_us: ack.airtime_us as f64,
        }),
        uplink_start_us: exchange.uplink_start_us as f64,
        rxr: window_out(exchange.rxr),
    }
}

/// Describes what a relay said about itself the way JavaScript holds it.
pub(crate) fn relay_status_out(status: RelayStatus) -> LorawanRelayStatus {
    LorawanRelayStatus {
        cad_periodicity: periodicity_out(status.cad_periodicity),
        xtal_accuracy: xtal_out(status.xtal_accuracy),
        cad_to_rx: receive_out(status.cad_to_rx),
        relay_data_rate: status.relay_data_rate,
        forward: forward_out(status.forward),
    }
}

/// The name a device error goes by as an `Error`'s `code`.
fn code(error: DeviceError) -> &'static str {
    match error {
        DeviceError::TooManyChannels { .. } => "TooManyChannels",
        DeviceError::NoCredentials => "NoCredentials",
        DeviceError::NotJoined => "NotJoined",
        DeviceError::Busy => "Busy",
        DeviceError::NothingPending => "NothingPending",
        DeviceError::Wait { .. } => "Wait",
        DeviceError::NoChannel => "NoChannel",
        DeviceError::DataRate(_) => "DataRate",
        DeviceError::PayloadTooLong { .. } => "PayloadTooLong",
        DeviceError::CounterExhausted => "CounterExhausted",
        DeviceError::Frame(_) => "Frame",
        DeviceError::Foreign => "Foreign",
        DeviceError::Replayed => "Replayed",
        DeviceError::CounterGap => "CounterGap",
        DeviceError::Refused => "Refused",
        DeviceError::State(_) => "State",
    }
}

/// Throws a device error as an `Error` whose `code` names it, carrying what goes with it.
pub(crate) fn thrown(env: &Env, error: DeviceError) -> Error {
    let message = error.to_string();
    let raised = (|| -> Result<()> {
        let mut object = env.create_error(Error::new(Status::GenericFailure, message.clone()))?;
        object.set("code", code(error))?;
        match error {
            DeviceError::Wait { until_us } => object.set("untilUs", until_us as f64)?,
            DeviceError::TooManyChannels { max } | DeviceError::PayloadTooLong { max } => {
                object.set("max", max as u32)?
            }
            DeviceError::DataRate(rate) => object.set("dataRate", u32::from(rate))?,
            DeviceError::State(state) => {
                object.set(
                    "state",
                    match state {
                        StateError::Length => "Length",
                        StateError::Corrupt => "Corrupt",
                        StateError::Format(_) => "Format",
                        StateError::Plan => "Plan",
                    },
                )?;
                if let StateError::Format(format) = state {
                    object.set("format", u32::from(format))?;
                }
            }
            _ => {}
        }
        env.throw(object)
    })();
    match raised {
        Ok(()) => Error::new(Status::PendingException, message),
        Err(failure) => failure,
    }
}
