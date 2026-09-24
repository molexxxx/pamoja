//! The network side of one site, for Python.
//!
//! A gateway forwards packets without reading them, because it holds no keys. This is the
//! other half: a [`GatewayNetwork`] holds the devices it admits, the sessions it has granted
//! and the counters it has seen, so a packet handed to it comes back as one of three things.
//! A device joined and its accept is ready to transmit, a session frame arrived and was
//! decrypted, or the frame belongs to a network this site never granted, which a gateway
//! hears all the time and is not an error.

use std::sync::Mutex;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_gateway::network::{Event, Network, Registration, Rx1Channels, Slot, Windows};
use pamoja_lora::region::ChannelBlock;

use crate::gateway::{rxpk_of, txpk_to_py, GatewayRxpk, GatewayTxpk};
use crate::lora::LoraLink;
use crate::lora_region::ChannelPlan;
use crate::lorawan::{describe_mac, rebuild_command, LorawanMacCommand};
use crate::lorawan_relay::channel_out;
use crate::PamojaError;

/// Where and when a downlink answers an uplink, in the concentrator's own terms.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewaySlot {
    /// The concentrator timestamp to transmit at, in microseconds.
    #[pyo3(get)]
    timestamp_us: u32,
    /// The frequency to transmit on, in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// The settings to transmit with.
    #[pyo3(get)]
    link: Py<LoraLink>,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewaySlot {
    /// Builds a window to answer in.
    #[new]
    fn new(timestamp_us: u32, frequency_hz: u32, link: Py<LoraLink>) -> Self {
        Self {
            timestamp_us,
            frequency_hz,
            link,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "GatewaySlot(timestamp_us={}, frequency_hz={})",
            self.timestamp_us, self.frequency_hz
        )
    }
}

/// What a relay added to an uplink it forwarded, TS011-1.0.1 section 9.1.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayRelayed {
    /// The relay's own address.
    #[pyo3(get)]
    relay: u32,
    /// What it heard of the uplink, in dBm.
    #[pyo3(get)]
    rssi_dbm: i16,
    /// Its signal-to-noise ratio, in dB.
    #[pyo3(get)]
    snr_db: i8,
    /// The data rate it arrived at.
    #[pyo3(get)]
    data_rate: u8,
    /// The wake-on-radio channel the device woke the relay on: `default` or `second`.
    #[pyo3(get)]
    wor_channel: String,
    /// The frequency it arrived on, in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayRelayed {
    fn __repr__(&self) -> String {
        format!(
            "GatewayRelayed(relay={:#010x}, rssi_dbm={}, snr_db={})",
            self.relay, self.rssi_dbm, self.snr_db
        )
    }
}

/// A device a relay heard and could not verify, TS011-1.0.1 section 10.7.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayNotice {
    /// The relay that heard it.
    #[pyo3(get)]
    relay: u32,
    /// The address the wake-on-radio frame named.
    #[pyo3(get)]
    dev_addr: u32,
    /// The frame's signal strength in dBm.
    #[pyo3(get)]
    rssi_dbm: i16,
    /// Its signal-to-noise ratio in dB.
    #[pyo3(get)]
    snr_db: i8,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayNotice {
    fn __repr__(&self) -> String {
        format!(
            "GatewayNotice(relay={:#010x}, dev_addr={:#010x})",
            self.relay, self.dev_addr
        )
    }
}

/// What a forwarded packet turned out to be, and where its answer goes.
#[gen_stub_pyclass]
#[pyclass(frozen)]
pub struct GatewayNetworkEvent {
    /// `joined`, `data`, or `foreign`.
    #[pyo3(get)]
    outcome: String,
    /// The device that joined, as sixteen hexadecimal digits.
    #[pyo3(get)]
    dev_eui: Option<String>,
    /// The address granted, or the address a frame claimed.
    #[pyo3(get)]
    dev_addr: u32,
    /// The counter the frame carried, reconstructed to its full width.
    #[pyo3(get)]
    fcnt: Option<u32>,
    /// The port the frame was sent on, absent for a frame carrying only options.
    #[pyo3(get)]
    fport: Option<u8>,
    /// What the device sent, decrypted.
    #[pyo3(get)]
    payload: Option<Py<PyBytes>>,
    /// Whether the device asked to be acknowledged.
    #[pyo3(get)]
    confirmed: Option<bool>,
    /// Where an answer goes, for a join or for data.
    #[pyo3(get)]
    slot: Option<Py<GatewaySlot>>,
    /// The packet carrying the accept, for a join.
    #[pyo3(get)]
    accept: Option<Py<GatewayTxpk>>,
    /// What a relay added to an uplink it carried, and `None` for one a gateway heard
    /// directly.
    #[pyo3(get)]
    relay: Option<Py<GatewayRelayed>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayNetworkEvent {
    fn __repr__(&self) -> String {
        format!(
            "GatewayNetworkEvent(outcome={}, dev_addr={:#010x})",
            self.outcome, self.dev_addr
        )
    }
}

/// The network side of one site: what a server does with what a gateway forwarded.
#[gen_stub_pyclass]
#[pyclass]
pub struct GatewayNetwork {
    inner: Mutex<Network>,
}

#[gen_stub_pymethods]
#[pymethods]
impl GatewayNetwork {
    /// Opens the network side of a site on a channel plan.
    ///
    /// The plan is copied into the network, so it holds its band for as long as it runs.
    #[new]
    #[pyo3(signature = (plan, net_id, receive_delay_us=None, join_delay_us=None, rx1_data_rate_offset=None, downstream=None, first_dev_addr=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        plan: &ChannelPlan,
        net_id: u32,
        receive_delay_us: Option<u32>,
        join_delay_us: Option<u32>,
        rx1_data_rate_offset: Option<u8>,
        downstream: Option<(u32, u32, u16)>,
        first_dev_addr: Option<u32>,
    ) -> Self {
        let mut windows = Windows::new();
        if let Some(micros) = receive_delay_us {
            windows = windows.with_receive_delay_us(micros);
        }
        if let Some(micros) = join_delay_us {
            windows = windows.with_join_delay_us(micros);
        }
        if let Some(offset) = rx1_data_rate_offset {
            windows = windows.with_rx1_data_rate_offset(offset);
        }
        if let Some((start_hz, step_hz, count)) = downstream {
            windows = windows.with_rx1_channels(Rx1Channels::Downstream(ChannelBlock::new(
                start_hz, step_hz, count, 0, 0,
            )));
        }

        let mut network = plan
            .with(|plan| Network::new(plan, net_id))
            .with_windows(windows);
        if let Some(dev_addr) = first_dev_addr {
            network = network.with_first_dev_addr(dev_addr);
        }
        Self {
            inner: Mutex::new(network),
        }
    }

    /// Admits a device, so a join request signed with its key is accepted.
    fn register(&self, dev_eui: Vec<u8>, app_eui: Vec<u8>, app_key: Vec<u8>) -> PyResult<()> {
        let dev_eui = eight(&dev_eui, "dev_eui")?;
        let app_eui = eight(&app_eui, "app_eui")?;
        let app_key = sixteen(&app_key)?;
        self.locked()?
            .register(Registration::new(dev_eui, app_eui, app_key));
        Ok(())
    }

    /// Reads a packet the gateway forwarded.
    fn uplink(
        &self,
        py: Python<'_>,
        heard: PyRef<'_, GatewayRxpk>,
    ) -> PyResult<GatewayNetworkEvent> {
        let heard = rxpk_of(py, heard)?;
        let event = self
            .locked()?
            .uplink(&heard)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        event_to_py(py, event)
    }

    /// Builds a downlink for a device, encrypted with its session.
    fn answer(
        &self,
        py: Python<'_>,
        dev_addr: u32,
        slot: PyRef<'_, GatewaySlot>,
        fport: u8,
        payload: Vec<u8>,
    ) -> PyResult<GatewayTxpk> {
        let window = Slot {
            timestamp_us: slot.timestamp_us,
            frequency_hz: slot.frequency_hz,
            link: slot.link.bind(py).borrow().settings(),
        };
        let downlink = self
            .locked()?
            .answer(dev_addr, window, fport, &payload)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        txpk_to_py(py, &downlink)
    }

    /// Takes the devices relays have reported hearing and could not verify, TS011-1.0.1
    /// section 10.7.
    ///
    /// A relay carries these in the MAC commands of its own uplinks, alongside whatever else
    /// that uplink was for, so they wait here until they are read.
    fn notices(&self) -> PyResult<Vec<GatewayNotice>> {
        Ok(self
            .locked()?
            .notices()
            .into_iter()
            .map(|notice| GatewayNotice {
                relay: notice.relay,
                dev_addr: notice.dev_addr,
                rssi_dbm: notice.rssi_dbm,
                snr_db: notice.snr_db,
            })
            .collect())
    }

    /// Builds a downlink carrying MAC commands, which is how a network configures a relay.
    ///
    /// The commands ride in the frame options where they fit, and in a frame of their own on
    /// port 0 where they do not.
    fn command(
        &self,
        py: Python<'_>,
        dev_addr: u32,
        slot: PyRef<'_, GatewaySlot>,
        commands: Vec<PyRef<'_, LorawanMacCommand>>,
    ) -> PyResult<GatewayTxpk> {
        let window = Slot {
            timestamp_us: slot.timestamp_us,
            frequency_hz: slot.frequency_hz,
            link: slot.link.bind(py).borrow().settings(),
        };
        let mut built = Vec::with_capacity(commands.len());
        for command in &commands {
            built.push(rebuild_command(command)?);
        }
        let downlink = self
            .locked()?
            .command(dev_addr, window, &built)
            .map_err(|error| PamojaError::new_err(error.to_string()))?;
        txpk_to_py(py, &downlink)
    }

    /// Builds the command that tells a relay to trust a device, with the key that lets it
    /// verify the device's wake-on-radio frames, TS011-1.0.1 section 10.4.
    ///
    /// `index` is the entry in the relay's list, `reload_rate` how many of the device's
    /// uplinks it forwards an hour, 63 for no limit, and `bucket_size` the coded multiplier
    /// of table 55.
    #[pyo3(signature = (dev_addr, index, reload_rate = 63, bucket_size = 0))]
    fn trust_command(
        &self,
        dev_addr: u32,
        index: u8,
        reload_rate: u8,
        bucket_size: u8,
    ) -> PyResult<LorawanMacCommand> {
        self.locked()?
            .trust_command(dev_addr, index, reload_rate, bucket_size)
            .map(describe_mac)
            .map_err(|error| PamojaError::new_err(error.to_string()))
    }
}

impl GatewayNetwork {
    /// Takes the network, or reports a lock another thread left poisoned.
    fn locked(&self) -> PyResult<std::sync::MutexGuard<'_, Network>> {
        self.inner
            .lock()
            .map_err(|_| PamojaError::new_err("the network was left locked by a failed call"))
    }
}

/// Writes an event to Python.
fn event_to_py(py: Python<'_>, event: Event) -> PyResult<GatewayNetworkEvent> {
    Ok(match event {
        Event::Joined {
            dev_eui,
            dev_addr,
            accept,
        } => GatewayNetworkEvent {
            outcome: "joined".to_owned(),
            dev_eui: Some(hex(&dev_eui)),
            dev_addr,
            fcnt: None,
            fport: None,
            payload: None,
            confirmed: None,
            slot: None,
            accept: Some(Py::new(py, txpk_to_py(py, &accept)?)?),
            relay: None,
        },
        Event::Data {
            dev_addr,
            fcnt,
            fport,
            payload,
            confirmed,
            slot,
            relay,
        } => GatewayNetworkEvent {
            outcome: "data".to_owned(),
            dev_eui: None,
            dev_addr,
            fcnt: Some(fcnt),
            fport,
            payload: Some(PyBytes::new(py, &payload).unbind()),
            confirmed: Some(confirmed),
            slot: Some(Py::new(
                py,
                GatewaySlot {
                    timestamp_us: slot.timestamp_us,
                    frequency_hz: slot.frequency_hz,
                    link: Py::new(py, LoraLink::from_settings(slot.link))?,
                },
            )?),
            accept: None,
            relay: relay
                .map(|relayed| {
                    Py::new(
                        py,
                        GatewayRelayed {
                            relay: relayed.relay,
                            rssi_dbm: relayed.metadata.rssi_dbm,
                            snr_db: relayed.metadata.snr_db,
                            data_rate: relayed.metadata.data_rate,
                            wor_channel: channel_out(relayed.metadata.wor_channel),
                            frequency_hz: relayed.frequency_hz,
                        },
                    )
                })
                .transpose()?,
        },
        Event::Foreign { dev_addr } => GatewayNetworkEvent {
            outcome: "foreign".to_owned(),
            dev_eui: None,
            dev_addr,
            fcnt: None,
            fport: None,
            payload: None,
            confirmed: None,
            slot: None,
            accept: None,
            relay: None,
        },
    })
}

/// Writes bytes as hexadecimal, the way an identifier is written down.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Reads an eight-byte identifier.
fn eight(bytes: &[u8], name: &str) -> PyResult<[u8; 8]> {
    bytes.try_into().map_err(|_| {
        PyValueError::new_err(format!("{name} must be eight bytes, not {}", bytes.len()))
    })
}

/// Reads a sixteen-byte key.
fn sixteen(bytes: &[u8]) -> PyResult<[u8; 16]> {
    bytes.try_into().map_err(|_| {
        PyValueError::new_err(format!(
            "app_key must be sixteen bytes, not {}",
            bytes.len()
        ))
    })
}
