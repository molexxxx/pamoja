//! Generated Python bindings for a LoRaWAN relay and for the end devices that reach a
//! network through one, TS011-1.0.1.
//!
//! [`LorawanRelay`] is an end device that also listens for the devices around it: it scans
//! for wake-on-radio frames, acknowledges the ones it trusts, and wraps the uplinks they
//! announce in its own uplinks on port 226. The relay side of an end device is on
//! `LorawanEndDevice` itself, which builds the wake-on-radio frame ahead of each uplink.
//!
//! Coded fields cross as lowercase names, as they do in `pamoja.lorawan.relay`.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_lorawan::device::{
    AckWindow, EndDevice, Heard, ReceiveWindow, RelayExchange, RelayStatus, WakeUp,
};
use pamoja_lora::region::RelayChannel;
use pamoja_lorawan::relay::{
    Acknowledgment, Listen, Relay, RelayConfig, RelayError, RelayHeard, RelaySettings, RxrDownlink,
    Scan,
};

use crate::lora::LoraLink;
use crate::lora_region::{ChannelPlan, LoraRelayChannel};
use crate::lorawan::{LorawanDevice, LorawanSession};
use crate::lorawan_device::{
    heard_out, next_out, published, raised, LorawanDeviceSettings, LorawanHeard, LorawanNext,
    LorawanTransmission, LorawanWindow,
};
use crate::lorawan_relay::{
    carrier_out, channel_out, forward_out, periodicity_in, periodicity_out, receive_in,
    receive_out, xtal_in, xtal_out, LorawanCarrier,
};
use crate::PamojaError;

pyo3::create_exception!(
    pamoja,
    LorawanRelayError,
    PamojaError,
    "Raised when a relay cannot do what it was asked. `kind` names why: `stopped`, `busy`, `not_listening`, `nothing_held`, `nothing_awaited`, `frame`, `carrier`, `limited`, `filtered`, `foreign` or `configuration`. What the relay's own device refused raises `LorawanDeviceError` instead."
);

/// The frame that wakes a relay, and where it goes.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanWakeUp {
    inner: WakeUp,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanWakeUp {
    /// The frame: five bytes ahead of a join request, fifteen ahead of an uplink.
    #[getter]
    fn frame<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.frame())
    }

    /// When to start sending it, in microseconds.
    #[getter]
    fn start_us(&self) -> u64 {
        self.inner.start_us
    }

    /// Where it goes, and how fast.
    #[getter]
    fn carrier(&self) -> LorawanCarrier {
        carrier_out(self.inner.carrier)
    }

    /// Its LoRa settings, with the preamble this frame needs, sent with inverted IQ.
    #[getter]
    fn link(&self) -> LoraLink {
        LoraLink::from_settings(self.inner.link)
    }

    /// The power to ask of the radio, conducted, in dBm.
    #[getter]
    fn output_dbm(&self) -> i8 {
        self.inner.output_dbm
    }

    /// How long it holds the air, in microseconds.
    #[getter]
    fn airtime_us(&self) -> u64 {
        self.inner.airtime_us
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanWakeUp(start_us={}, frequency_hz={}, data_rate={})",
            self.inner.start_us, self.inner.carrier.frequency_hz, self.inner.carrier.data_rate
        )
    }
}

/// When and where a relay's acknowledgment would arrive.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanAckWindow {
    inner: AckWindow,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanAckWindow {
    /// When it starts, in microseconds.
    #[getter]
    fn start_us(&self) -> u64 {
        self.inner.start_us
    }

    /// Where it arrives, and how fast.
    #[getter]
    fn carrier(&self) -> LorawanCarrier {
        carrier_out(self.inner.carrier)
    }

    /// The LoRa settings to listen with.
    #[getter]
    fn link(&self) -> LoraLink {
        LoraLink::from_settings(self.inner.link)
    }

    /// How long it lasts, in microseconds.
    #[getter]
    fn airtime_us(&self) -> u64 {
        self.inner.airtime_us
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanAckWindow(start_us={}, frequency_hz={})",
            self.inner.start_us, self.inner.carrier.frequency_hz
        )
    }
}

/// The wake-on-radio exchange an uplink under a relay goes out behind, TS011-1.0.1 section 5.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanRelayExchange {
    inner: RelayExchange,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanRelayExchange {
    /// The frame that wakes the relay.
    #[getter]
    fn wake_up(&self) -> LorawanWakeUp {
        LorawanWakeUp {
            inner: self.inner.wake_up,
        }
    }

    /// Where the relay's acknowledgment would arrive, or `None` ahead of a join request,
    /// which no relay acknowledges.
    #[getter]
    fn ack(&self) -> Option<LorawanAckWindow> {
        self.inner.ack.map(|inner| LorawanAckWindow { inner })
    }

    /// When the uplink itself goes out, in microseconds, whether or not the acknowledgment
    /// arrives.
    #[getter]
    fn uplink_start_us(&self) -> u64 {
        self.inner.uplink_start_us
    }

    /// The relay window, timed from the end of the uplink like the other two.
    #[getter]
    fn rxr(&self) -> LorawanWindow {
        self.inner.rxr.into()
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanRelayExchange(uplink_start_us={}, rxr_delay_us={})",
            self.inner.uplink_start_us, self.inner.rxr.delay_us
        )
    }
}

impl LorawanRelayExchange {
    /// Describes an exchange the way Python holds it.
    pub(crate) fn of(inner: RelayExchange) -> Self {
        Self { inner }
    }
}

/// What a relay's acknowledgment said about itself, TS011-1.0.1 table 14.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanRelayStatus {
    /// How often it scans.
    #[pyo3(get)]
    cad_periodicity: String,
    /// How accurate its crystal is.
    #[pyo3(get)]
    xtal_accuracy: String,
    /// How long it takes to start receiving.
    #[pyo3(get)]
    cad_to_rx: String,
    /// The data rate it forwards at, which bounds what the device may send.
    #[pyo3(get)]
    relay_data_rate: u8,
    /// Whether it will forward: `available`, `retry_in_30_minutes`, `retry_in_60_minutes` or
    /// `disabled`.
    #[pyo3(get)]
    forward: String,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanRelayStatus {
    fn __repr__(&self) -> String {
        format!(
            "LorawanRelayStatus(cad_periodicity={:?}, relay_data_rate={}, forward={:?})",
            self.cad_periodicity, self.relay_data_rate, self.forward
        )
    }
}

impl LorawanRelayStatus {
    /// Describes what a relay said about itself the way Python holds it.
    pub(crate) fn of(status: RelayStatus) -> Self {
        Self {
            cad_periodicity: periodicity_out(status.cad_periodicity),
            xtal_accuracy: xtal_out(status.xtal_accuracy),
            cad_to_rx: receive_out(status.cad_to_rx),
            relay_data_rate: status.relay_data_rate,
            forward: forward_out(status.forward),
        }
    }
}

/// What an end device does once its wake-on-radio frame went unanswered.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanWorNext {
    /// `True` to send the uplink at the time the exchange named anyway.
    #[pyo3(get)]
    uplink: bool,
    wake_up: Option<RelayExchange>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanWorNext {
    /// The next exchange, when the relay is woken again first, and `None` otherwise.
    #[getter]
    fn wake_up(&self) -> Option<LorawanRelayExchange> {
        self.wake_up.map(LorawanRelayExchange::of)
    }

    fn __repr__(&self) -> String {
        format!("LorawanWorNext(uplink={})", if self.uplink { "True" } else { "False" })
    }
}

impl LorawanWorNext {
    /// Describes what comes next the way Python holds it.
    pub(crate) fn of(uplink: bool, wake_up: Option<RelayExchange>) -> Self {
        Self { uplink, wake_up }
    }
}

/// A scan for wake-on-radio frames, due next.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanScan {
    inner: Scan,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanScan {
    /// When to start detecting, in microseconds.
    #[getter]
    fn start_us(&self) -> u64 {
        self.inner.start_us
    }

    /// Which channel: `default` or `second`.
    #[getter]
    fn channel(&self) -> String {
        channel_out(self.inner.channel)
    }

    /// Where to listen, and how fast.
    #[getter]
    fn carrier(&self) -> LorawanCarrier {
        carrier_out(self.inner.carrier)
    }

    /// The LoRa settings of a wake-on-radio frame, heard with inverted IQ.
    #[getter]
    fn link(&self) -> LoraLink {
        LoraLink::from_settings(self.inner.link)
    }

    /// The longest preamble an end device sends on the channel, in symbols.
    #[getter]
    fn preamble_symbols(&self) -> u16 {
        self.inner.preamble_symbols
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanScan(start_us={}, frequency_hz={}, data_rate={})",
            self.inner.start_us, self.inner.carrier.frequency_hz, self.inner.carrier.data_rate
        )
    }
}

/// The acknowledgment a relay answers a wake-on-radio frame with.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanAcknowledgment {
    inner: Acknowledgment,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanAcknowledgment {
    /// The frame, seven bytes.
    #[getter]
    fn frame<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.frame)
    }

    /// When to start sending it, in microseconds.
    #[getter]
    fn start_us(&self) -> u64 {
        self.inner.start_us
    }

    /// Where it goes, and how fast.
    #[getter]
    fn carrier(&self) -> LorawanCarrier {
        carrier_out(self.inner.carrier)
    }

    /// Its LoRa settings, sent with inverted IQ.
    #[getter]
    fn link(&self) -> LoraLink {
        LoraLink::from_settings(self.inner.link)
    }

    /// The power to ask of the radio, conducted, in dBm.
    #[getter]
    fn output_dbm(&self) -> i8 {
        self.inner.output_dbm
    }

    /// How long it holds the air, in microseconds.
    #[getter]
    fn airtime_us(&self) -> u64 {
        self.inner.airtime_us
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanAcknowledgment(start_us={}, frequency_hz={})",
            self.inner.start_us, self.inner.carrier.frequency_hz
        )
    }
}

/// When and where a relay listens for the uplink a wake-on-radio frame announced.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanListen {
    inner: Listen,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanListen {
    /// When the uplink starts, in microseconds.
    #[getter]
    fn start_us(&self) -> u64 {
        self.inner.start_us
    }

    /// Where it arrives, and how fast.
    #[getter]
    fn carrier(&self) -> LorawanCarrier {
        carrier_out(self.inner.carrier)
    }

    /// Its LoRa settings, heard with standard IQ.
    #[getter]
    fn link(&self) -> LoraLink {
        LoraLink::from_settings(self.inner.link)
    }

    /// The longest frame the relay forwards; stop receiving anything longer.
    #[getter]
    fn max_len(&self) -> usize {
        self.inner.max_len
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanListen(start_us={}, frequency_hz={}, max_len={})",
            self.inner.start_us, self.inner.carrier.frequency_hz, self.inner.max_len
        )
    }
}

/// What a wake-on-radio frame led a relay to do.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanWake {
    /// `join_request`, `uplink` or `notified`.
    #[pyo3(get)]
    kind: String,
    /// The device, for an uplink or a notification, and `None` for a join request.
    #[pyo3(get)]
    dev_addr: Option<u32>,
    /// The wake-on-radio frame counter it carried, for an uplink.
    #[pyo3(get)]
    wfcnt: Option<u32>,
    /// Whether the relay forwards the uplink, which the acknowledgment reports.
    #[pyo3(get)]
    forward: Option<String>,
    acknowledgment: Option<Acknowledgment>,
    listen: Option<Listen>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanWake {
    /// The acknowledgment to send, where there is one.
    #[getter]
    fn acknowledgment(&self) -> Option<LorawanAcknowledgment> {
        self.acknowledgment
            .map(|inner| LorawanAcknowledgment { inner })
    }

    /// Where and when to listen for the uplink, where the relay will.
    #[getter]
    fn listen(&self) -> Option<LorawanListen> {
        self.listen.map(|inner| LorawanListen { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanWake(kind={:?}, dev_addr={:?})",
            self.kind, self.dev_addr
        )
    }
}

/// A downlink for an end device, to send in its relay window.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanRxrDownlink {
    inner: RxrDownlink,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanRxrDownlink {
    /// The frame to send.
    #[getter]
    fn frame<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.frame())
    }

    /// When to start sending it, in microseconds.
    #[getter]
    fn start_us(&self) -> u64 {
        self.inner.start_us
    }

    /// Where it goes, and how fast.
    #[getter]
    fn carrier(&self) -> LorawanCarrier {
        carrier_out(self.inner.carrier)
    }

    /// Its LoRa settings, sent with inverted IQ and a payload CRC.
    #[getter]
    fn link(&self) -> LoraLink {
        LoraLink::from_settings(self.inner.link)
    }

    /// The power to ask of the radio, conducted, in dBm.
    #[getter]
    fn output_dbm(&self) -> i8 {
        self.inner.output_dbm
    }

    /// How long it holds the air, in microseconds.
    #[getter]
    fn airtime_us(&self) -> u64 {
        self.inner.airtime_us
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanRxrDownlink(start_us={}, frequency_hz={})",
            self.inner.start_us, self.inner.carrier.frequency_hz
        )
    }
}

/// What a frame a relay's own device heard turned out to be.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanRelayHeard {
    /// `device`, `downlink` or `undeliverable`.
    #[pyo3(get)]
    kind: String,
    /// Why a downlink could not be passed on, for `undeliverable`.
    #[pyo3(get)]
    reason: Option<String>,
    heard: Option<LorawanHeard>,
    downlink: Option<RxrDownlink>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanRelayHeard {
    /// What the relay's own device made of the frame.
    #[getter]
    fn heard(&self) -> Option<LorawanHeard> {
        self.heard.clone()
    }

    /// The downlink to send the end device on, for `downlink`.
    #[getter]
    fn downlink(&self) -> Option<LorawanRxrDownlink> {
        self.downlink.map(|inner| LorawanRxrDownlink { inner })
    }

    fn __repr__(&self) -> String {
        format!("LorawanRelayHeard(kind={:?})", self.kind)
    }
}

/// A LoRaWAN relay: an end device that also listens for the devices around it.
///
/// One turn runs like this: `next_scan` says when and where to listen, a wake-on-radio frame
/// heard there goes to `heard_wor`, the uplink it announced to `heard_uplink`, and `forward`
/// wraps that in one of the relay's own uplinks on port 226. What the relay's own receive
/// windows hear goes to `heard_in`, which turns a downlink meant for an end device into one
/// to send in that device's relay window.
///
/// A call that cannot be done raises `LorawanRelayError`, or `LorawanDeviceError` when it is
/// the relay's own device that refused.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanRelay {
    inner: Relay<'static>,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanRelay {
    /// Makes a relay whose own device is activated by personalization.
    ///
    /// `xtal_accuracy` and `cad_to_rx` describe the relay's own hardware, and travel in every
    /// acknowledgment it sends, so the devices around it know how much preamble to send.
    #[staticmethod]
    #[pyo3(signature = (plan, session, settings, xtal_accuracy = "ppm40", cad_to_rx = "symbols8"))]
    fn personalized(
        plan: &ChannelPlan,
        session: &LorawanSession,
        settings: &LorawanDeviceSettings,
        xtal_accuracy: &str,
        cad_to_rx: &str,
    ) -> PyResult<Self> {
        let made = RelaySettings::new(xtal_in(xtal_accuracy)?, receive_in(cad_to_rx)?);
        let plan = published(plan)?;
        let device =
            EndDevice::personalized(plan, session.inner, settings.core()).map_err(raised)?;
        Ok(Self {
            inner: Relay::new(device, made),
        })
    }

    /// Makes a relay whose own device joins over the air.
    #[staticmethod]
    #[pyo3(signature = (plan, credentials, settings, xtal_accuracy = "ppm40", cad_to_rx = "symbols8"))]
    fn over_the_air(
        plan: &ChannelPlan,
        credentials: &LorawanDevice,
        settings: &LorawanDeviceSettings,
        xtal_accuracy: &str,
        cad_to_rx: &str,
    ) -> PyResult<Self> {
        let made = RelaySettings::new(xtal_in(xtal_accuracy)?, receive_in(cad_to_rx)?);
        let plan = published(plan)?;
        let device =
            EndDevice::new(plan, credentials.inner.clone(), settings.core()).map_err(raised)?;
        Ok(Self {
            inner: Relay::new(device, made),
        })
    }

    /// Starts scanning, or changes what a running relay scans from its next scan on.
    ///
    /// `default_channel_index` picks one of the region's wake-on-radio channels, and
    /// `second_channel` is one a network configured.
    #[pyo3(signature = (cad_periodicity = "ms1000", default_channel_index = 0, second_channel = None))]
    fn start(
        &mut self,
        cad_periodicity: &str,
        default_channel_index: u8,
        second_channel: Option<LoraRelayChannel>,
    ) -> PyResult<()> {
        let periodicity = periodicity_in(cad_periodicity)?;
        let Some(default_channel) = self.inner.region_channel(default_channel_index) else {
            return Err(PamojaError::new_err(
                "the region does not define that wake-on-radio channel",
            ));
        };
        let mut config = RelayConfig::new(periodicity, default_channel);
        if let Some(second) = second_channel {
            let (wor_frequency_hz, ack_frequency_hz, data_rate) = second.core();
            config = config.with_second_channel(RelayChannel::new(
                wor_frequency_hz,
                ack_frequency_hz,
                data_rate,
            ));
        }
        self.inner.start(config).map_err(refused)
    }

    /// Stops scanning. A forwarded uplink already waiting still goes out.
    fn stop(&mut self) {
        self.inner.stop();
    }

    /// Whether the relay is scanning.
    #[getter]
    fn running(&self) -> bool {
        self.inner.config().is_some()
    }

    /// Trusts an end device, as an `UpdateUplinkListReq` with the same fields does.
    ///
    /// `index` is the entry, 0 to 15. `reload_rate` is how many of the device's uplinks are
    /// forwarded an hour, 63 for no limit, and `bucket_size` the coded multiplier of
    /// TS011-1.0.1 table 55.
    #[pyo3(signature = (index, dev_addr, root_wor_s_key, next_wfcnt = 0, reload_rate = 63, bucket_size = 0))]
    fn trust(
        &mut self,
        index: u8,
        dev_addr: u32,
        root_wor_s_key: Vec<u8>,
        next_wfcnt: u32,
        reload_rate: u8,
        bucket_size: u8,
    ) -> PyResult<()> {
        let key: [u8; 16] = root_wor_s_key.try_into().map_err(|_| {
            PamojaError::new_err("a root relay session key is sixteen bytes".to_owned())
        })?;
        if self
            .inner
            .trust(index, dev_addr, &key, next_wfcnt, reload_rate, bucket_size)
        {
            Ok(())
        } else {
            Err(PamojaError::new_err(
                "a relay trusts sixteen devices, at indexes 0 to 15",
            ))
        }
    }

    /// Says when and where to scan next, or `None` while the relay is stopped.
    fn next_scan(&mut self, now_us: u64) -> Option<LorawanScan> {
        self.inner
            .next_scan(now_us)
            .map(|inner| LorawanScan { inner })
    }

    /// Reads a wake-on-radio frame a scan heard, `ended_us` being when the frame ended.
    fn heard_wor(
        &mut self,
        scan: &LorawanScan,
        frame: Vec<u8>,
        rssi_dbm: i16,
        snr_db: i8,
        ended_us: u64,
    ) -> PyResult<LorawanWake> {
        let wake = self
            .inner
            .heard_wor(&scan.inner, &frame, rssi_dbm, snr_db, ended_us)
            .map_err(refused)?;
        Ok(match wake {
            pamoja_lorawan::relay::Wake::JoinRequest { listen } => LorawanWake {
                kind: "join_request".to_owned(),
                dev_addr: None,
                wfcnt: None,
                forward: None,
                acknowledgment: None,
                listen: Some(listen),
            },
            pamoja_lorawan::relay::Wake::Uplink {
                dev_addr,
                wfcnt,
                forward,
                acknowledgment,
                listen,
            } => LorawanWake {
                kind: "uplink".to_owned(),
                dev_addr: Some(dev_addr),
                wfcnt: Some(wfcnt),
                forward: Some(forward_out(forward)),
                acknowledgment,
                listen,
            },
            pamoja_lorawan::relay::Wake::Notified { dev_addr } => LorawanWake {
                kind: "notified".to_owned(),
                dev_addr: Some(dev_addr),
                wfcnt: None,
                forward: None,
                acknowledgment: None,
                listen: None,
            },
        })
    }

    /// Reads the uplink a wake-on-radio frame announced, and holds it to forward.
    ///
    /// Returns when to `forward` it: fifty milliseconds after it ended.
    fn heard_uplink(
        &mut self,
        frame: Vec<u8>,
        rssi_dbm: i16,
        snr_db: i8,
        ended_us: u64,
    ) -> PyResult<u64> {
        self.inner
            .heard_uplink(&frame, rssi_dbm, snr_db, ended_us)
            .map_err(refused)
    }

    /// Clears the uplink a wake-on-radio frame announced, once listening heard nothing.
    fn uplink_missed(&mut self) {
        self.inner.uplink_missed();
    }

    /// When the forwarded uplink waiting to go out is due, or `None` with nothing waiting.
    #[getter]
    fn forward_due(&self) -> Option<u64> {
        self.inner.forward_due()
    }

    /// Sends the uplink the relay is holding, in one of its own on port 226.
    fn forward(&mut self, now_us: u64) -> PyResult<LorawanTransmission> {
        self.inner
            .forward(now_us)
            .map(LorawanTransmission::from)
            .map_err(refused)
    }

    /// Reads a frame the relay's own device heard, acting on the relay commands in it.
    ///
    /// `window` is `"rx1"`, `"rx2"` or `"rxr"`.
    fn heard_in(&mut self, window: &str, frame: Vec<u8>, snr_db: i8) -> PyResult<LorawanRelayHeard> {
        let window = match window {
            "rx1" => ReceiveWindow::Rx1,
            "rx2" => ReceiveWindow::Rx2,
            "rxr" => ReceiveWindow::Rxr,
            other => {
                return Err(PamojaError::new_err(format!(
                    "{other} is not a receive window; expected rx1, rx2 or rxr"
                )))
            }
        };
        let dev_addr = self.inner.device().dev_addr().unwrap_or(0);
        Ok(match self
            .inner
            .heard_in(window, &frame, snr_db)
            .map_err(refused)?
        {
            RelayHeard::Device(heard) => LorawanRelayHeard {
                kind: "device".to_owned(),
                reason: None,
                heard: Some(heard_out(heard, dev_addr)),
                downlink: None,
            },
            RelayHeard::Downlink { delivery, downlink } => LorawanRelayHeard {
                kind: "downlink".to_owned(),
                reason: None,
                heard: Some(heard_out(Heard::Data(delivery), dev_addr)),
                downlink: Some(downlink),
            },
            RelayHeard::Undeliverable { delivery, reason } => LorawanRelayHeard {
                kind: "undeliverable".to_owned(),
                reason: Some(reason.to_string()),
                heard: Some(heard_out(Heard::Data(delivery), dev_addr)),
                downlink: None,
            },
        })
    }

    /// Says what comes next once the relay's own windows closed with nothing in them.
    fn nothing_heard(&mut self, now_us: u64) -> PyResult<LorawanNext> {
        self.inner
            .nothing_heard(now_us)
            .map(next_out)
            .map_err(refused)
    }

    /// Makes the relay's own join request.
    fn join(&mut self, dev_nonce: u16, now_us: u64) -> PyResult<LorawanTransmission> {
        self.inner
            .device_mut()
            .join(dev_nonce, now_us)
            .map(LorawanTransmission::from)
            .map_err(raised)
    }

    /// Sends one of the relay's own uplinks, which also carries what it owes its network.
    #[pyo3(signature = (port, payload, now_us, confirmed = false))]
    fn send(
        &mut self,
        port: u8,
        payload: Vec<u8>,
        now_us: u64,
        confirmed: bool,
    ) -> PyResult<LorawanTransmission> {
        self.inner
            .device_mut()
            .send(port, &payload, confirmed, now_us)
            .map(LorawanTransmission::from)
            .map_err(raised)
    }

    /// Sends an uplink with no payload, carrying whatever the relay owes its network.
    fn send_empty(&mut self, now_us: u64) -> PyResult<LorawanTransmission> {
        self.inner
            .device_mut()
            .send_empty(now_us)
            .map(LorawanTransmission::from)
            .map_err(raised)
    }

    /// The address the relay's own device is on the network by, or `None` before joining.
    #[getter]
    fn dev_addr(&self) -> Option<u32> {
        self.inner.device().dev_addr()
    }

    /// Whether the relay's own device is on a network.
    #[getter]
    fn joined(&self) -> bool {
        self.inner.device().is_joined()
    }

    /// The data rate the relay forwards at, which its acknowledgments report.
    #[getter]
    fn data_rate(&self) -> u8 {
        self.inner.device().data_rate()
    }

    fn __repr__(&self) -> String {
        match self.inner.device().dev_addr() {
            Some(dev_addr) => format!("LorawanRelay(dev_addr={dev_addr:#010x})"),
            None => "LorawanRelay(not joined)".to_owned(),
        }
    }
}

/// Raises a relay error with its kind, or the device error underneath it.
fn refused(error: RelayError) -> PyErr {
    let kind = match error {
        RelayError::Stopped => "stopped",
        RelayError::Busy => "busy",
        RelayError::NotListening => "not_listening",
        RelayError::NothingHeld => "nothing_held",
        RelayError::NothingAwaited => "nothing_awaited",
        RelayError::Frame(_) => "frame",
        RelayError::Carrier(_) => "carrier",
        RelayError::Limited => "limited",
        RelayError::Filtered => "filtered",
        RelayError::Foreign => "foreign",
        RelayError::Configuration => "configuration",
        RelayError::Device(error) => return raised(error),
    };
    let err = LorawanRelayError::new_err(error.to_string());
    Python::attach(|py| {
        let _ = err.value(py).setattr("kind", kind);
    });
    err
}
