//! Generated Python bindings for a LoRaWAN Class A end device, without a radio.
//!
//! [`LorawanEndDevice`] joins, chooses a channel and data rate for each uplink, says when and
//! where to listen for the answer, reads what comes back, and does what the network's MAC
//! commands ask. It owns no radio and no clock: every call takes the time in microseconds and
//! hands back what to put on the air.
//!
//! A device runs on a published channel plan, `ChannelPlan.for_region` or
//! `ChannelPlan.for_cn470`. A call that cannot be done raises `LorawanDeviceError`, whose
//! `kind` names why, such as `wait`, with what goes with it, such as `until_us`.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_lorawan::device::{
    Battery, Delivery, DeviceError, EndDevice, Heard, Next, ReceiveWindow, RelayExchange, Saved,
    Settings, StateError, Transmission, Window, WorNext,
};
use pamoja_lorawan::relay::{RelayActivation, RelaySync};
use pamoja_lorawan::Version;

use crate::lora::LoraLink;
use crate::lora_region::ChannelPlan;
use crate::lorawan::{LorawanDevice, LorawanSession};
use crate::lorawan_relay_node::{LorawanRelayExchange, LorawanRelayStatus, LorawanWorNext};
use crate::PamojaError;

pyo3::create_exception!(
    pamoja,
    LorawanDeviceError,
    PamojaError,
    "Raised when an end device cannot do what it was asked. `kind` names why: `too_many_channels`, `no_credentials`, `not_joined`, `busy`, `nothing_pending`, `wait`, `no_channel`, `data_rate`, `payload_too_long`, `counter_exhausted`, `frame`, `foreign`, `replayed`, `counter_gap`, `refused` or `state`. `until_us`, `max`, `data_rate`, `state` and `format` carry what goes with it, and are `None` otherwise."
);

/// What a device's radio can do, and how it takes part.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct LorawanDeviceSettings {
    /// The lowest power the radio puts out, conducted, in dBm.
    #[pyo3(get)]
    min_output_dbm: i8,
    /// The highest, conducted, in dBm.
    #[pyo3(get)]
    max_output_dbm: i8,
    /// The link layer revision, `1.0.3` or `1.0.4`.
    #[pyo3(get)]
    version: String,
    /// Whether the network manages the data rate and power.
    #[pyo3(get)]
    adr: bool,
    /// The antenna gain less the cable and connector losses, in dB.
    #[pyo3(get)]
    antenna_gain_db: i8,
    /// The lowest frequency the radio and its front end can use, in hertz.
    #[pyo3(get)]
    lowest_hz: u32,
    /// The highest, in hertz.
    #[pyo3(get)]
    highest_hz: u32,
    /// Whether the device is held to the region's sub-band duty cycles.
    #[pyo3(get)]
    regional_duty_cycle: bool,
    /// Whether payloads are sized for a path through a relay.
    #[pyo3(get)]
    behind_repeater: bool,
    /// The seed for the random choices of channel and retry delay.
    #[pyo3(get)]
    seed: u32,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanDeviceSettings {
    /// Settings for a radio with an output power range.
    ///
    /// The rest start as a typical node: TS001-1.0.4, adaptive data rate on, an antenna with
    /// no gain over its cable, a radio that tunes 137 to 1020 MHz as an SX1276 does, the
    /// region's duty cycle kept, and no repeater in the path. The seed is best taken from a
    /// hardware random source; the device identifier is mixed in.
    #[new]
    #[pyo3(signature = (
        min_output_dbm,
        max_output_dbm,
        version = "1.0.4",
        adr = true,
        antenna_gain_db = 0,
        lowest_hz = 137_000_000,
        highest_hz = 1_020_000_000,
        regional_duty_cycle = true,
        behind_repeater = false,
        seed = 0,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        min_output_dbm: i8,
        max_output_dbm: i8,
        version: &str,
        adr: bool,
        antenna_gain_db: i8,
        lowest_hz: u32,
        highest_hz: u32,
        regional_duty_cycle: bool,
        behind_repeater: bool,
        seed: u32,
    ) -> PyResult<Self> {
        if !matches!(version, "1.0.3" | "1.0.4") {
            return Err(PamojaError::new_err(format!(
                "{version} is not a LoRaWAN version; expected 1.0.3 or 1.0.4"
            )));
        }
        if min_output_dbm > max_output_dbm || lowest_hz > highest_hz {
            return Err(PamojaError::new_err(
                "the output power and tuning ranges must run from low to high",
            ));
        }
        Ok(Self {
            min_output_dbm,
            max_output_dbm,
            version: version.to_owned(),
            adr,
            antenna_gain_db,
            lowest_hz,
            highest_hz,
            regional_duty_cycle,
            behind_repeater,
            seed,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanDeviceSettings(min_output_dbm={}, max_output_dbm={}, version={:?})",
            self.min_output_dbm, self.max_output_dbm, self.version
        )
    }
}

impl LorawanDeviceSettings {
    /// The Rust settings these describe.
    pub(crate) fn core(&self) -> Settings {
        let version = if self.version == "1.0.3" {
            Version::V1_0_3
        } else {
            Version::V1_0_4
        };
        let mut settings = Settings::new(self.min_output_dbm, self.max_output_dbm)
            .with_version(version)
            .with_adr(self.adr)
            .with_antenna_gain(self.antenna_gain_db)
            .with_tuning_range(self.lowest_hz, self.highest_hz)
            .with_seed(self.seed);
        if !self.regional_duty_cycle {
            settings = settings.without_regional_duty_cycle();
        }
        if self.behind_repeater {
            settings = settings.behind_repeater();
        }
        settings
    }
}

/// When and where to listen for a downlink.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanWindow {
    /// How long after the end of the transmission the window opens, in microseconds.
    #[pyo3(get)]
    delay_us: u32,
    /// The carrier, in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// The downlink data rate, as the region numbers them.
    #[pyo3(get)]
    data_rate: u8,
    /// The LoRa settings to listen with: no payload CRC, and inverted IQ, as RP002-1.0.5 table
    /// 112 has for a downlink.
    #[pyo3(get)]
    link: LoraLink,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanWindow {
    fn __repr__(&self) -> String {
        format!(
            "LorawanWindow(delay_us={}, frequency_hz={}, data_rate={})",
            self.delay_us, self.frequency_hz, self.data_rate
        )
    }
}

impl From<Window> for LorawanWindow {
    fn from(window: Window) -> Self {
        Self {
            delay_us: window.delay_us,
            frequency_hz: window.frequency_hz,
            data_rate: window.data_rate,
            link: LoraLink::from_settings(window.link),
        }
    }
}

/// A frame to put on the air, and where to listen afterward.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanTransmission {
    frame: Vec<u8>,
    /// The carrier, in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// The data rate, as the region numbers them.
    #[pyo3(get)]
    data_rate: u8,
    /// The LoRa settings: an eight-symbol preamble, an explicit header and a payload CRC,
    /// sent with standard IQ.
    #[pyo3(get)]
    link: LoraLink,
    /// The power to ask of the radio, conducted, in dBm.
    #[pyo3(get)]
    output_dbm: i8,
    /// How long the frame holds the air, in microseconds.
    #[pyo3(get)]
    airtime_us: u64,
    /// The first receive window.
    rx1: Window,
    /// The second, which opens only if nothing for this device arrived in the first.
    rx2: Window,
    /// Whether the application payload went out in this frame. When the answers the device
    /// owed left no room, it did not, and has to be sent again.
    #[pyo3(get)]
    carries_payload: bool,
    /// The wake-on-radio exchange this frame goes out behind, for a device under a relay.
    relay: Option<RelayExchange>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanTransmission {
    /// The frame.
    #[getter]
    fn frame<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.frame)
    }

    /// The first receive window.
    #[getter]
    fn rx1(&self) -> LorawanWindow {
        self.rx1.into()
    }

    /// The second receive window, which opens only if nothing for this device arrived in the
    /// first.
    #[getter]
    fn rx2(&self) -> LorawanWindow {
        self.rx2.into()
    }

    /// The wake-on-radio exchange this frame goes out behind, and `None` for a frame that
    /// goes straight to a gateway.
    #[getter]
    fn relay(&self) -> Option<LorawanRelayExchange> {
        self.relay.map(LorawanRelayExchange::of)
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanTransmission(frequency_hz={}, data_rate={}, output_dbm={}, airtime_us={})",
            self.frequency_hz, self.data_rate, self.output_dbm, self.airtime_us
        )
    }
}

impl From<Transmission> for LorawanTransmission {
    fn from(transmission: Transmission) -> Self {
        Self {
            frame: transmission.frame.as_bytes().to_vec(),
            frequency_hz: transmission.frequency_hz,
            data_rate: transmission.data_rate,
            link: LoraLink::from_settings(transmission.link),
            output_dbm: transmission.output_dbm,
            airtime_us: transmission.airtime_us,
            rx1: transmission.rx1,
            rx2: transmission.rx2,
            carries_payload: transmission.carries_payload,
            relay: transmission.relay,
        }
    }
}

/// A downlink, read and acted on.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanDelivery {
    inner: Delivery,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanDelivery {
    /// The application port the payload arrived on, or `None` for a frame that carried only
    /// MAC commands or nothing.
    #[getter]
    fn port(&self) -> Option<u8> {
        self.inner.port()
    }

    /// The application payload, decrypted.
    #[getter]
    fn payload<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.payload())
    }

    /// Whether the network acknowledged the confirmed uplink this answered.
    #[getter]
    fn acknowledged(&self) -> bool {
        self.inner.acknowledged()
    }

    /// Whether the network asked for this downlink to be acknowledged, which the next uplink
    /// does by itself.
    #[getter]
    fn confirmed(&self) -> bool {
        self.inner.confirmed()
    }

    /// Whether the network has more waiting.
    #[getter]
    fn more_pending(&self) -> bool {
        self.inner.more_pending()
    }

    /// The answer to a link check the device asked for, as the margin in dB and the gateway
    /// count, or `None`.
    #[getter]
    fn link_check(&self) -> Option<(u8, u8)> {
        self.inner
            .link_check()
            .map(|check| (check.margin_db, check.gateways))
    }

    /// The answer to a time request the device asked for, as whole seconds since the GPS
    /// epoch and 256ths of a second, or `None`.
    #[getter]
    fn device_time(&self) -> Option<(u32, u8)> {
        self.inner
            .device_time()
            .map(|time| (time.gps_seconds, time.fraction))
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanDelivery(port={:?}, payload_len={}, acknowledged={})",
            self.inner.port(),
            self.inner.payload().len(),
            if self.inner.acknowledged() {
                "True"
            } else {
                "False"
            }
        )
    }
}

/// What a frame heard in a receive window turned out to be.
#[gen_stub_pyclass]
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct LorawanHeard {
    /// `joined` for a join accept, `data` for a data frame.
    #[pyo3(get)]
    kind: String,
    /// The address the device is on the network by.
    #[pyo3(get)]
    dev_addr: u32,
    delivery: Option<Delivery>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanHeard {
    /// For a data frame, what it carried, and `None` for a join.
    #[getter]
    fn delivery(&self) -> Option<LorawanDelivery> {
        self.delivery.map(|inner| LorawanDelivery { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanHeard(kind={:?}, dev_addr={:#010x})",
            self.kind, self.dev_addr
        )
    }
}

/// What to do once both receive windows closed with nothing for the device.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanNext {
    /// `repeat`, `done`, `unacknowledged` or `join_again`.
    #[pyo3(get)]
    kind: String,
    /// For a repeat or another join, the earliest time to send, in microseconds.
    #[pyo3(get)]
    not_before_us: Option<u64>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanNext {
    fn __repr__(&self) -> String {
        format!(
            "LorawanNext(kind={:?}, not_before_us={:?})",
            self.kind, self.not_before_us
        )
    }
}

/// A channel a device may send on.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanChannel {
    /// The channel's index in the device's table.
    #[pyo3(get)]
    index: usize,
    /// Where uplinks go out, in hertz.
    #[pyo3(get)]
    uplink_hz: u32,
    /// Where the first receive window listens, in hertz.
    #[pyo3(get)]
    downlink_hz: u32,
    /// The slowest data rate the channel carries.
    #[pyo3(get)]
    min_data_rate: u8,
    /// The fastest.
    #[pyo3(get)]
    max_data_rate: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanChannel {
    fn __repr__(&self) -> String {
        format!(
            "LorawanChannel(index={}, uplink_hz={}, downlink_hz={})",
            self.index, self.uplink_hz, self.downlink_hz
        )
    }
}

/// A LoRaWAN Class A end device.
///
/// One exchange runs like this: `join` or `send` returns a transmission to put on the air and
/// two receive windows timed from its end. A frame heard in either window goes to `heard`. If
/// neither window held one, `nothing_heard` says whether to send the same frame again with
/// `repeat`, or move on.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanEndDevice {
    inner: EndDevice<'static>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanEndDevice {
    /// Makes a device that joins over the air.
    ///
    /// `fcnt_up` and `fcnt_down` carry frame counters over a restart; a join starts them over.
    /// Raises `PamojaError` if the plan was built rather than published.
    #[staticmethod]
    #[pyo3(signature = (plan, credentials, settings, fcnt_up = 0, fcnt_down = None))]
    fn over_the_air(
        plan: &ChannelPlan,
        credentials: &LorawanDevice,
        settings: &LorawanDeviceSettings,
        fcnt_up: u32,
        fcnt_down: Option<u32>,
    ) -> PyResult<Self> {
        let plan = published(plan)?;
        let device =
            EndDevice::new(plan, credentials.inner.clone(), settings.core()).map_err(raised)?;
        Ok(Self {
            inner: device.with_frame_counters(fcnt_up, fcnt_down),
        })
    }

    /// Makes a device activated by personalization, with its session provisioned.
    ///
    /// Such a device never resets its frame counters, TS001-1.0.4 section 4.3.1.5, so one that
    /// lost power passes the counters it kept.
    #[staticmethod]
    #[pyo3(signature = (plan, session, settings, fcnt_up = 0, fcnt_down = None))]
    fn personalized(
        plan: &ChannelPlan,
        session: &LorawanSession,
        settings: &LorawanDeviceSettings,
        fcnt_up: u32,
        fcnt_down: Option<u32>,
    ) -> PyResult<Self> {
        let plan = published(plan)?;
        let device =
            EndDevice::personalized(plan, session.inner, settings.core()).map_err(raised)?;
        Ok(Self {
            inner: device.with_frame_counters(fcnt_up, fcnt_down),
        })
    }

    /// Builds a join request, with a nonce this device has never used with its join
    /// identifier.
    fn join(&mut self, dev_nonce: u16, now_us: u64) -> PyResult<LorawanTransmission> {
        self.inner
            .join(dev_nonce, now_us)
            .map(LorawanTransmission::from)
            .map_err(raised)
    }

    /// Builds an uplink carrying a payload on an application port, 1 to 223, or 224 for the
    /// certification test port.
    #[pyo3(signature = (port, payload, now_us, confirmed = false))]
    fn send(
        &mut self,
        port: u8,
        payload: Vec<u8>,
        now_us: u64,
        confirmed: bool,
    ) -> PyResult<LorawanTransmission> {
        self.inner
            .send(port, &payload, confirmed, now_us)
            .map(LorawanTransmission::from)
            .map_err(raised)
    }

    /// Builds an uplink with no payload, carrying the answers the device owes, an
    /// acknowledgment, or an ADR acknowledgment request.
    fn send_empty(&mut self, now_us: u64) -> PyResult<LorawanTransmission> {
        self.inner
            .send_empty(now_us)
            .map(LorawanTransmission::from)
            .map_err(raised)
    }

    /// Sends the last uplink again, the same frame on a channel chosen afresh.
    fn repeat(&mut self, now_us: u64) -> PyResult<LorawanTransmission> {
        self.inner
            .repeat(now_us)
            .map(LorawanTransmission::from)
            .map_err(raised)
    }

    /// Reads a frame heard in one of the receive windows of the last transmission.
    ///
    /// A frame that is not for this device, does not verify, or is longer than the window's
    /// data rate carries raises and leaves the transmission waiting, so the second window
    /// still opens. `window` is `"rx1"` or `"rx2"`; without it, a frame may be as long as the
    /// faster window allows.
    #[pyo3(signature = (frame, snr_db, window = None))]
    fn heard(
        &mut self,
        frame: Vec<u8>,
        snr_db: i8,
        window: Option<&str>,
    ) -> PyResult<LorawanHeard> {
        let result = match window {
            Some("rx1") => self.inner.heard_in(ReceiveWindow::Rx1, &frame, snr_db),
            Some("rx2") => self.inner.heard_in(ReceiveWindow::Rx2, &frame, snr_db),
            Some("rxr") => self.inner.heard_in(ReceiveWindow::Rxr, &frame, snr_db),
            Some(other) => {
                return Err(PamojaError::new_err(format!(
                    "{other} is not a receive window; expected rx1, rx2 or rxr"
                )))
            }
            None => self.inner.heard(&frame, snr_db),
        };
        let dev_addr = self.inner.dev_addr().unwrap_or(0);
        Ok(heard_out(result.map_err(raised)?, dev_addr))
    }

    /// Says what comes next once both receive windows closed with nothing for the device.
    fn nothing_heard(&mut self, now_us: u64) -> PyResult<LorawanNext> {
        self.inner.nothing_heard(now_us).map(next_out).map_err(raised)
    }
    /// Turns relay mode on or off, TS011-1.0.1 section 10.2 and appendix 5.
    ///
    /// From here on the decision is the caller's rather than the device's own policy, until
    /// its network takes it over with an `EndDeviceConfReq` or hands it back. Returns `False`
    /// when the network holds the decision, leaving the mode as it was.
    fn use_relay(&mut self, on: bool) -> bool {
        self.inner.use_relay(on)
    }

    /// Reads the acknowledgment a relay answered the last wake-on-radio frame with.
    ///
    /// The device is now synchronized: it knows when the relay scans, so its next frames
    /// carry only as much preamble as the two clocks could have drifted apart.
    fn heard_wor_ack(&mut self, frame: Vec<u8>) -> PyResult<LorawanRelayStatus> {
        self.inner
            .heard_wor_ack(&frame)
            .map(LorawanRelayStatus::of)
            .map_err(raised)
    }

    /// Says what to do once the acknowledgment window closed with nothing in it: send the
    /// uplink anyway, or wake the relay again first, as the network's `BackOff` asks.
    fn no_wor_ack(&mut self, now_us: u64) -> PyResult<LorawanWorNext> {
        Ok(match self.inner.no_wor_ack(now_us).map_err(raised)? {
            WorNext::Uplink => LorawanWorNext::of(true, None),
            WorNext::WakeUp(exchange) => LorawanWorNext::of(false, Some(exchange)),
        })
    }

    /// Whether the next uplink goes through a relay.
    #[getter]
    fn relaying(&self) -> bool {
        self.inner.relaying()
    }

    /// How the device decides whether to use a relay: `disabled`, `enabled`, `dynamic` or
    /// `device_controlled`.
    #[getter]
    fn relay_activation(&self) -> String {
        match self.inner.relay_activation() {
            RelayActivation::Disabled => "disabled",
            RelayActivation::Enabled => "enabled",
            RelayActivation::Dynamic => "dynamic",
            RelayActivation::DeviceControlled => "device_controlled",
        }
        .to_owned()
    }

    /// What the device knows of when its relay listens: `initialized`, `unsynchronized` or
    /// `synchronized`, TS011-1.0.1 section 3.9.
    #[getter]
    fn relay_sync(&self) -> String {
        match self.inner.relay_sync() {
            RelaySync::Initialized => "initialized",
            RelaySync::Unsynchronized => "unsynchronized",
            RelaySync::Synchronized => "synchronized",
        }
        .to_owned()
    }

    /// What the relay's last acknowledgment said about itself, or `None` before one arrived.
    #[getter]
    fn relay_status(&self) -> Option<LorawanRelayStatus> {
        self.inner.relay_status().map(LorawanRelayStatus::of)
    }

    /// The wake-on-radio frame counter the next frame will use, TS011-1.0.1 section 5.3.2.
    #[getter]
    fn wor_counter(&self) -> u32 {
        self.inner.wor_counter()
    }

    /// Sets what the device reports its battery as when a network asks: a level from 1,
    /// empty, to 254, full, or `None` when it cannot tell. `external` marks a device on
    /// external power, whatever the level.
    #[pyo3(signature = (level = None, external = false))]
    fn set_battery(&mut self, level: Option<u8>, external: bool) {
        self.inner.set_battery(match (level, external) {
            (_, true) => Battery::External,
            (Some(level), false) => Battery::Level(level),
            (None, false) => Battery::Unknown,
        });
    }

    /// Asks the network, with the next uplink, how well it hears the device.
    fn request_link_check(&mut self) {
        self.inner.request_link_check();
    }

    /// Asks the network, with the next uplink, for the time.
    fn request_device_time(&mut self) {
        self.inner.request_device_time();
    }

    /// Whether the device is on a network: once joined, or from the start for a personalized
    /// device.
    #[getter]
    fn is_joined(&self) -> bool {
        self.inner.is_joined()
    }

    /// The address the device is on the network by, or `None` before joining.
    #[getter]
    fn dev_addr(&self) -> Option<u32> {
        self.inner.dev_addr()
    }

    /// The data rate the next uplink goes out at, before any back-off step.
    #[getter]
    fn data_rate(&self) -> u8 {
        self.inner.data_rate()
    }

    /// The next uplink frame counter.
    #[getter]
    fn fcnt_up(&self) -> u32 {
        self.inner.fcnt_up()
    }

    /// The last downlink frame counter accepted, or `None` before any downlink.
    #[getter]
    fn fcnt_down(&self) -> Option<u32> {
        self.inner.fcnt_down()
    }

    /// How many times each uplink goes out, as the network last set it.
    #[getter]
    fn transmissions(&self) -> u8 {
        self.inner.transmissions()
    }

    /// Where the second receive window listens, as a frequency in hertz and a data rate.
    #[getter]
    fn rx2(&self) -> (u32, u8) {
        self.inner.rx2()
    }

    /// The delay from the end of an uplink to the first receive window, in microseconds.
    #[getter]
    fn receive_delay_us(&self) -> u32 {
        self.inner.receive_delay_us()
    }

    /// The lowest and highest frequency the device transmits or listens on, in hertz.
    #[getter]
    fn frequency_span(&self) -> (u32, u32) {
        self.inner.frequency_span()
    }

    /// The channels the device may send on.
    fn channels(&self) -> Vec<LorawanChannel> {
        self.inner
            .channels()
            .map(|(index, channel)| LorawanChannel {
                index,
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
    fn save<'py>(&self, py: Python<'py>, now_us: u64) -> PyResult<Bound<'py, PyBytes>> {
        let saved = self.inner.save(now_us).map_err(raised)?;
        Ok(PyBytes::new(py, saved.as_bytes()))
    }

    /// Puts a saved state back on a device made the same way, on the clock it woke to.
    fn resume(&mut self, saved: Vec<u8>, now_us: u64) -> PyResult<()> {
        Saved::from_bytes(&saved)
            .map_err(DeviceError::State)
            .and_then(|saved| self.inner.resume(&saved, now_us))
            .map_err(raised)
    }

    fn __repr__(&self) -> String {
        match self.inner.dev_addr() {
            Some(dev_addr) => format!("LorawanEndDevice(dev_addr={dev_addr:#010x})"),
            None => "LorawanEndDevice(not joined)".to_owned(),
        }
    }
}

/// Describes what a frame turned out to be the way Python holds it.
pub(crate) fn heard_out(heard: Heard, dev_addr: u32) -> LorawanHeard {
    match heard {
        Heard::Joined { dev_addr } => LorawanHeard {
            kind: "joined".to_owned(),
            dev_addr,
            delivery: None,
        },
        Heard::Data(delivery) => LorawanHeard {
            kind: "data".to_owned(),
            dev_addr,
            delivery: Some(delivery),
        },
    }
}

/// Describes what comes next the way Python holds it.
pub(crate) fn next_out(next: Next) -> LorawanNext {
    match next {
        Next::Repeat { not_before_us } => LorawanNext {
            kind: "repeat".to_owned(),
            not_before_us: Some(not_before_us),
        },
        Next::Done => LorawanNext {
            kind: "done".to_owned(),
            not_before_us: None,
        },
        Next::Unacknowledged => LorawanNext {
            kind: "unacknowledged".to_owned(),
            not_before_us: None,
        },
        Next::JoinAgain { not_before_us } => LorawanNext {
            kind: "join_again".to_owned(),
            not_before_us: Some(not_before_us),
        },
    }
}

/// The published plan a device is made on.
pub(crate) fn published(
    plan: &ChannelPlan,
) -> PyResult<&'static pamoja_lora::region::ChannelPlan<'static>> {
    plan.published_plan().ok_or_else(|| {
        PamojaError::new_err(
            "a device runs on a published plan, from ChannelPlan.for_region or ChannelPlan.for_cn470",
        )
    })
}

/// Raises a device error with its kind and what goes with it.
pub(crate) fn raised(error: DeviceError) -> PyErr {
    let kind = match error {
        DeviceError::TooManyChannels { .. } => "too_many_channels",
        DeviceError::NoCredentials => "no_credentials",
        DeviceError::NotJoined => "not_joined",
        DeviceError::Busy => "busy",
        DeviceError::NothingPending => "nothing_pending",
        DeviceError::Wait { .. } => "wait",
        DeviceError::NoChannel => "no_channel",
        DeviceError::DataRate(_) => "data_rate",
        DeviceError::PayloadTooLong { .. } => "payload_too_long",
        DeviceError::CounterExhausted => "counter_exhausted",
        DeviceError::Frame(_) => "frame",
        DeviceError::Foreign => "foreign",
        DeviceError::Replayed => "replayed",
        DeviceError::CounterGap => "counter_gap",
        DeviceError::Refused => "refused",
        DeviceError::State(_) => "state",
    };
    let err = LorawanDeviceError::new_err(error.to_string());
    Python::attach(|py| {
        let value = err.value(py);
        let until_us = match error {
            DeviceError::Wait { until_us } => Some(until_us),
            _ => None,
        };
        let max = match error {
            DeviceError::TooManyChannels { max } | DeviceError::PayloadTooLong { max } => Some(max),
            _ => None,
        };
        let data_rate = match error {
            DeviceError::DataRate(rate) => Some(rate),
            _ => None,
        };
        let (state, format) = match error {
            DeviceError::State(StateError::Length) => (Some("length"), None),
            DeviceError::State(StateError::Corrupt) => (Some("corrupt"), None),
            DeviceError::State(StateError::Format(format)) => (Some("format"), Some(format)),
            DeviceError::State(StateError::Plan) => (Some("plan"), None),
            _ => (None, None),
        };
        let _ = value.setattr("kind", kind);
        let _ = value.setattr("until_us", until_us);
        let _ = value.setattr("max", max);
        let _ = value.setattr("data_rate", data_rate);
        let _ = value.setattr("state", state);
        let _ = value.setattr("format", format);
    });
    err
}
