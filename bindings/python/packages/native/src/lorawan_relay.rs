//! Generated Python bindings for a LoRaWAN relay, TS011-1.0.1: the wake-on-radio frames an
//! end device and a relay exchange, the uplinks a relay forwards, and the timing that keeps a
//! wake-on-radio preamble short.
//!
//! Every call is a pure function of its arguments. Coded fields cross as lowercase names, so
//! `pamoja.lorawan.relay` can give them string enums.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_lorawan::mac::relay_second_channel;
use pamoja_lorawan::relay::{
    self, open_wor_ack, t_offset_ms, unsynchronized_preamble_symbols, wor_ack, wor_join_request,
    wor_uplink, CadPeriodicity, CadToRx, Carrier, Forward, ForwardedUplink, StateSync,
    Synchronization, UplinkMetadata, Wor, WorChannel, WorKeys, XtalAccuracy,
};
use pamoja_lorawan::LorawanError;

use crate::lora_region::LoraRelayChannel;
use crate::lorawan::LorawanWorKeys;
use crate::PamojaError;

/// The relay constants TS011-1.0.1 and RP002-1.0.5 section 5.4.5 publish, as the module
/// constants they are published under.
pub const CONSTANTS: [(&str, u32); 9] = [
    ("LORAWAN_LA_FPORT_RELAY", relay::LA_FPORT_RELAY as u32),
    ("LORAWAN_TRUSTED_ED_NUMBER", relay::TRUSTED_ED_NUMBER as u32),
    (
        "LORAWAN_WOR_ATTEMPTS_WO_ACK",
        relay::WOR_ATTEMPTS_WO_ACK as u32,
    ),
    ("LORAWAN_WOR_DATA_DELAY_US", relay::WOR_DATA_DELAY_US),
    ("LORAWAN_WOR_ACK_DELAY_US", relay::WOR_ACK_DELAY_US),
    ("LORAWAN_RELAY_FWD_DELAY_US", relay::RELAY_FWD_DELAY_US),
    ("LORAWAN_RXR_DELAY_US", relay::RXR_DELAY_US),
    ("LORAWAN_FORWARD_OVERHEAD", relay::FORWARD_OVERHEAD as u32),
    (
        "LORAWAN_MIN_WOR_PREAMBLE_SYMBOLS",
        relay::MIN_WOR_PREAMBLE_SYMBOLS as u32,
    ),
];

/// Where a frame goes and how fast.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct LorawanCarrier {
    /// The frequency in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// The data rate.
    #[pyo3(get)]
    data_rate: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanCarrier {
    #[new]
    fn new(frequency_hz: u32, data_rate: u8) -> Self {
        Self {
            frequency_hz,
            data_rate,
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        (self.frequency_hz, self.data_rate) == (other.frequency_hz, other.data_rate)
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanCarrier(frequency_hz={}, data_rate={})",
            self.frequency_hz, self.data_rate
        )
    }
}

/// A wake-on-radio frame, as a relay reads it.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanWor {
    /// `join_request` or `uplink`.
    #[pyo3(get)]
    kind: String,
    /// Where and how fast a join request follows; an uplink's carrier stays sealed until
    /// `lorawan_relay_wor_open` reads it.
    #[pyo3(get)]
    uplink: Option<LorawanCarrier>,
    /// The address an uplink WOR names.
    #[pyo3(get)]
    dev_addr: Option<u32>,
    /// The low sixteen bits of its frame counter.
    #[pyo3(get)]
    wfcnt: Option<u16>,
}

/// What a relay tells an end device about itself in a WOR ACK, TS011-1.0.1 table 14.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct LorawanStateSync {
    /// How long the relay takes to start receiving: `symbols2` to `symbols8`.
    #[pyo3(get)]
    cad_to_rx: String,
    /// Whether it forwards: `available`, `retry_in_30_minutes`, `retry_in_60_minutes` or
    /// `disabled`.
    #[pyo3(get)]
    forward: String,
    /// The data rate it forwards at.
    #[pyo3(get)]
    relay_data_rate: u8,
    /// How accurate its crystal is: `ppm10` to `ppm40`.
    #[pyo3(get)]
    xtal_accuracy: String,
    /// How often it scans: `ms1000`, `ms500`, `ms250`, `ms100`, `ms50` or `ms20`.
    #[pyo3(get)]
    cad_periodicity: String,
    /// Milliseconds from the start of the scan to the end of the WOR preamble.
    #[pyo3(get)]
    t_offset_ms: u16,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanStateSync {
    #[new]
    fn new(
        cad_to_rx: &str,
        forward: &str,
        relay_data_rate: u8,
        xtal_accuracy: &str,
        cad_periodicity: &str,
        t_offset_ms: u16,
    ) -> PyResult<Self> {
        let state = Self {
            cad_to_rx: cad_to_rx.to_owned(),
            forward: forward.to_owned(),
            relay_data_rate,
            xtal_accuracy: xtal_accuracy.to_owned(),
            cad_periodicity: cad_periodicity.to_owned(),
            t_offset_ms,
        };
        state_in(&state)?;
        Ok(state)
    }

    fn __eq__(&self, other: &Self) -> bool {
        (
            &self.cad_to_rx,
            &self.forward,
            self.relay_data_rate,
            &self.xtal_accuracy,
            &self.cad_periodicity,
            self.t_offset_ms,
        ) == (
            &other.cad_to_rx,
            &other.forward,
            other.relay_data_rate,
            &other.xtal_accuracy,
            &other.cad_periodicity,
            other.t_offset_ms,
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanStateSync(cad_to_rx={:?}, forward={:?}, relay_data_rate={}, xtal_accuracy={:?}, cad_periodicity={:?}, t_offset_ms={})",
            self.cad_to_rx,
            self.forward,
            self.relay_data_rate,
            self.xtal_accuracy,
            self.cad_periodicity,
            self.t_offset_ms
        )
    }
}

/// What a relay heard of an uplink it forwards.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct LorawanUplinkMetadata {
    /// The channel the WOR frame came in on: `default` or `second`.
    #[pyo3(get)]
    wor_channel: String,
    /// The uplink's signal strength in dBm, carried from -142 to -15.
    #[pyo3(get)]
    rssi_dbm: i16,
    /// Its signal-to-noise ratio in dB, carried from -20 to 11.
    #[pyo3(get)]
    snr_db: i8,
    /// The data rate it arrived at.
    #[pyo3(get)]
    data_rate: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl LorawanUplinkMetadata {
    #[new]
    fn new(wor_channel: &str, rssi_dbm: i16, snr_db: i8, data_rate: u8) -> PyResult<Self> {
        channel_in(wor_channel)?;
        Ok(Self {
            wor_channel: wor_channel.to_owned(),
            rssi_dbm,
            snr_db,
            data_rate,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "LorawanUplinkMetadata(wor_channel={:?}, rssi_dbm={}, snr_db={}, data_rate={})",
            self.wor_channel, self.rssi_dbm, self.snr_db, self.data_rate
        )
    }
}

/// An end device's uplink as a relay forwards it on port 226.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanForwardedUplink {
    /// What the relay heard of it.
    #[pyo3(get)]
    metadata: LorawanUplinkMetadata,
    /// The frequency it arrived on, in hertz.
    #[pyo3(get)]
    frequency_hz: u32,
    /// The end device's frame.
    #[pyo3(get)]
    phy_payload: Vec<u8>,
}

/// What an end device knows of a relay's scans once a WOR ACK has arrived.
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct LorawanSynchronization {
    /// When the relay scanned, in the device's microseconds.
    #[pyo3(get)]
    reference_us: u64,
    /// How often it scans.
    #[pyo3(get)]
    cad_periodicity: String,
    /// How accurate its crystal is.
    #[pyo3(get)]
    relay_xtal: String,
    /// How long it takes to start receiving.
    #[pyo3(get)]
    cad_to_rx: String,
}

/// When a synchronized end device's next WOR frame goes out.
#[gen_stub_pyclass]
#[pyclass]
pub struct LorawanWorSlot {
    /// When to start sending, in microseconds.
    #[pyo3(get)]
    start_us: u64,
    /// The preamble length in symbols.
    #[pyo3(get)]
    preamble_symbols: u16,
}

fn refused(error: LorawanError) -> PyErr {
    PamojaError::new_err(error.to_string())
}

fn sixteen(bytes: &[u8], what: &str) -> PyResult<[u8; 16]> {
    <[u8; 16]>::try_from(bytes)
        .map_err(|_| PamojaError::new_err(format!("{what} must be exactly 16 bytes")))
}

fn keys_in(keys: &LorawanWorKeys) -> PyResult<WorKeys> {
    Ok(WorKeys::new(
        sixteen(&keys.integrity, "integrity")?,
        sixteen(&keys.encryption, "encryption")?,
    ))
}

fn carrier_in(carrier: &LorawanCarrier) -> Carrier {
    Carrier::new(carrier.frequency_hz, carrier.data_rate)
}

fn carrier_out(carrier: Carrier) -> LorawanCarrier {
    LorawanCarrier {
        frequency_hz: carrier.frequency_hz,
        data_rate: carrier.data_rate,
    }
}

fn unknown(what: &str, value: &str, expected: &str) -> PyErr {
    PamojaError::new_err(format!("{value} is not {what}; expected {expected}"))
}

fn periodicity_in(value: &str) -> PyResult<CadPeriodicity> {
    Ok(match value {
        "ms1000" => CadPeriodicity::Ms1000,
        "ms500" => CadPeriodicity::Ms500,
        "ms250" => CadPeriodicity::Ms250,
        "ms100" => CadPeriodicity::Ms100,
        "ms50" => CadPeriodicity::Ms50,
        "ms20" => CadPeriodicity::Ms20,
        other => {
            return Err(unknown(
                "a scan periodicity",
                other,
                "ms1000, ms500, ms250, ms100, ms50 or ms20",
            ))
        }
    })
}

fn periodicity_out(value: CadPeriodicity) -> String {
    match value {
        CadPeriodicity::Ms1000 => "ms1000",
        CadPeriodicity::Ms500 => "ms500",
        CadPeriodicity::Ms250 => "ms250",
        CadPeriodicity::Ms100 => "ms100",
        CadPeriodicity::Ms50 => "ms50",
        CadPeriodicity::Ms20 => "ms20",
    }
    .to_owned()
}

fn receive_in(value: &str) -> PyResult<CadToRx> {
    Ok(match value {
        "symbols2" => CadToRx::Symbols2,
        "symbols4" => CadToRx::Symbols4,
        "symbols6" => CadToRx::Symbols6,
        "symbols8" => CadToRx::Symbols8,
        other => {
            return Err(unknown(
                "a CAD to RX time",
                other,
                "symbols2, symbols4, symbols6 or symbols8",
            ))
        }
    })
}

fn receive_out(value: CadToRx) -> String {
    format!("symbols{}", value.symbols())
}

fn xtal_in(value: &str) -> PyResult<XtalAccuracy> {
    Ok(match value {
        "ppm10" => XtalAccuracy::Ppm10,
        "ppm20" => XtalAccuracy::Ppm20,
        "ppm30" => XtalAccuracy::Ppm30,
        "ppm40" => XtalAccuracy::Ppm40,
        other => {
            return Err(unknown(
                "a crystal accuracy",
                other,
                "ppm10, ppm20, ppm30 or ppm40",
            ))
        }
    })
}

fn xtal_out(value: XtalAccuracy) -> String {
    format!("ppm{}", value.ppm())
}

fn forward_in(value: &str) -> PyResult<Forward> {
    Ok(match value {
        "available" => Forward::Available,
        "retry_in_30_minutes" => Forward::RetryIn30Minutes,
        "retry_in_60_minutes" => Forward::RetryIn60Minutes,
        "disabled" => Forward::Disabled,
        other => {
            return Err(unknown(
                "a forwarding state",
                other,
                "available, retry_in_30_minutes, retry_in_60_minutes or disabled",
            ))
        }
    })
}

fn forward_out(value: Forward) -> String {
    match value {
        Forward::Available => "available",
        Forward::RetryIn30Minutes => "retry_in_30_minutes",
        Forward::RetryIn60Minutes => "retry_in_60_minutes",
        Forward::Disabled => "disabled",
    }
    .to_owned()
}

fn channel_in(value: &str) -> PyResult<WorChannel> {
    match value {
        "default" => Ok(WorChannel::Default),
        "second" => Ok(WorChannel::Second),
        other => Err(unknown("a WOR channel", other, "default or second")),
    }
}

fn state_in(state: &LorawanStateSync) -> PyResult<StateSync> {
    Ok(StateSync {
        cad_to_rx: receive_in(&state.cad_to_rx)?,
        forward: forward_in(&state.forward)?,
        relay_data_rate: state.relay_data_rate,
        xtal_accuracy: xtal_in(&state.xtal_accuracy)?,
        cad_periodicity: periodicity_in(&state.cad_periodicity)?,
        t_offset_ms: state.t_offset_ms,
    })
}

fn state_out(state: StateSync) -> LorawanStateSync {
    LorawanStateSync {
        cad_to_rx: receive_out(state.cad_to_rx),
        forward: forward_out(state.forward),
        relay_data_rate: state.relay_data_rate,
        xtal_accuracy: xtal_out(state.xtal_accuracy),
        cad_periodicity: periodicity_out(state.cad_periodicity),
        t_offset_ms: state.t_offset_ms,
    }
}

/// Derives an end device's root relay session key from its network session key,
/// TS011-1.0.1 section 4.4.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_root_wor_s_key<'py>(
    py: Python<'py>,
    network_key: Vec<u8>,
) -> PyResult<Bound<'py, PyBytes>> {
    let root = relay::root_wor_s_key(&sixteen(&network_key, "network_key")?);
    Ok(PyBytes::new(py, &root))
}

/// Derives an end device's wake-on-radio keys from its root relay session key, section 4.5.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_wor_keys(root_key: Vec<u8>, dev_addr: u32) -> PyResult<LorawanWorKeys> {
    let keys = WorKeys::derive(&sixteen(&root_key, "root_key")?, dev_addr);
    Ok(LorawanWorKeys {
        integrity: keys.integrity().to_vec(),
        encryption: keys.encryption().to_vec(),
    })
}

/// Builds the WOR frame ahead of a join request, section 5.3.1.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_wor_join_request<'py>(
    py: Python<'py>,
    uplink: LorawanCarrier,
) -> PyResult<Bound<'py, PyBytes>> {
    let frame = wor_join_request(carrier_in(&uplink)).map_err(refused)?;
    Ok(PyBytes::new(py, &frame))
}

/// Builds the WOR frame ahead of a Class A uplink, section 5.3.2.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_wor_uplink<'py>(
    py: Python<'py>,
    keys: LorawanWorKeys,
    dev_addr: u32,
    wfcnt: u32,
    uplink: LorawanCarrier,
    wor: LorawanCarrier,
) -> PyResult<Bound<'py, PyBytes>> {
    let frame = wor_uplink(
        &keys_in(&keys)?,
        dev_addr,
        wfcnt,
        carrier_in(&uplink),
        carrier_in(&wor),
    )
    .map_err(refused)?;
    Ok(PyBytes::new(py, &frame))
}

/// Reads a WOR frame, leaving an uplink's carrier sealed.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_wor_parse(frame: Vec<u8>) -> PyResult<LorawanWor> {
    Ok(match Wor::parse(&frame).map_err(refused)? {
        Wor::JoinRequest { uplink } => LorawanWor {
            kind: "join_request".to_owned(),
            uplink: Some(carrier_out(uplink)),
            dev_addr: None,
            wfcnt: None,
        },
        Wor::Uplink(sealed) => LorawanWor {
            kind: "uplink".to_owned(),
            uplink: None,
            dev_addr: Some(sealed.dev_addr()),
            wfcnt: Some(sealed.wfcnt()),
        },
    })
}

/// Checks a WOR frame ahead of a Class A uplink and reads where the uplink follows.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_wor_open(
    frame: Vec<u8>,
    keys: LorawanWorKeys,
    wfcnt: u32,
    wor: LorawanCarrier,
) -> PyResult<LorawanCarrier> {
    let Wor::Uplink(sealed) = Wor::parse(&frame).map_err(refused)? else {
        return Err(PamojaError::new_err(
            "a join request WOR carries nothing sealed",
        ));
    };
    sealed
        .open(&keys_in(&keys)?, wfcnt, carrier_in(&wor))
        .map(carrier_out)
        .map_err(refused)
}

/// Builds a relay's WOR ACK, section 6.2.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_wor_ack<'py>(
    py: Python<'py>,
    keys: LorawanWorKeys,
    dev_addr: u32,
    wfcnt: u32,
    ack: LorawanCarrier,
    uplink: LorawanCarrier,
    state: LorawanStateSync,
) -> PyResult<Bound<'py, PyBytes>> {
    let frame = wor_ack(
        &keys_in(&keys)?,
        dev_addr,
        wfcnt,
        carrier_in(&ack),
        carrier_in(&uplink),
        state_in(&state)?,
    )
    .map_err(refused)?;
    Ok(PyBytes::new(py, &frame))
}

/// Checks and reads a WOR ACK, section 6.2.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_wor_ack_open(
    frame: Vec<u8>,
    keys: LorawanWorKeys,
    dev_addr: u32,
    wfcnt: u32,
    ack: LorawanCarrier,
    uplink: LorawanCarrier,
) -> PyResult<LorawanStateSync> {
    open_wor_ack(
        &frame,
        &keys_in(&keys)?,
        dev_addr,
        wfcnt,
        carrier_in(&ack),
        carrier_in(&uplink),
    )
    .map(state_out)
    .map_err(refused)
}

/// Writes an uplink a relay forwards on port 226, section 9.1.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_forward_encode<'py>(
    py: Python<'py>,
    metadata: LorawanUplinkMetadata,
    frequency_hz: u32,
    phy_payload: Vec<u8>,
) -> PyResult<Bound<'py, PyBytes>> {
    let uplink = ForwardedUplink {
        metadata: UplinkMetadata {
            wor_channel: channel_in(&metadata.wor_channel)?,
            rssi_dbm: metadata.rssi_dbm,
            snr_db: metadata.snr_db,
            data_rate: metadata.data_rate,
        },
        frequency_hz,
        phy_payload: &phy_payload,
    };
    let mut out = vec![0u8; relay::FORWARD_OVERHEAD + phy_payload.len()];
    uplink.encode(&mut out).map_err(refused)?;
    Ok(PyBytes::new(py, &out))
}

/// Reads an uplink a relay forwarded, section 9.1.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_forward_parse(payload: Vec<u8>) -> PyResult<LorawanForwardedUplink> {
    let forwarded = ForwardedUplink::parse(&payload).map_err(refused)?;
    Ok(LorawanForwardedUplink {
        metadata: LorawanUplinkMetadata {
            wor_channel: match forwarded.metadata.wor_channel {
                WorChannel::Default => "default",
                WorChannel::Second => "second",
            }
            .to_owned(),
            rssi_dbm: forwarded.metadata.rssi_dbm,
            snr_db: forwarded.metadata.snr_db,
            data_rate: forwarded.metadata.data_rate,
        },
        frequency_hz: forwarded.frequency_hz,
        phy_payload: forwarded.phy_payload.to_vec(),
    })
}

/// The WOR preamble of an end device that does not know when the relay scans, section 5.2.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_unsynchronized_preamble(
    cad_periodicity: &str,
    symbol_us: u64,
    cad_to_rx: &str,
) -> PyResult<u16> {
    Ok(unsynchronized_preamble_symbols(
        periodicity_in(cad_periodicity)?,
        symbol_us,
        receive_in(cad_to_rx)?,
    ))
}

/// The offset a relay reports in a WOR ACK, appendix 1, or `None` when the preamble ended
/// before the scan or more than eleven bits of milliseconds after it.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_t_offset_ms(scan_start_us: u64, preamble_end_us: u64) -> Option<u16> {
    t_offset_ms(scan_start_us, preamble_end_us)
}

/// Works out when a relay scanned from the WOR ACK that answered a frame.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_synchronization(
    wor_start_us: u64,
    preamble_symbols: u16,
    symbol_us: u64,
    state: LorawanStateSync,
) -> PyResult<LorawanSynchronization> {
    let sync = Synchronization::from_ack(
        wor_start_us,
        preamble_symbols,
        symbol_us,
        &state_in(&state)?,
    );
    Ok(LorawanSynchronization {
        reference_us: sync.reference_us,
        cad_periodicity: periodicity_out(sync.cad_periodicity),
        relay_xtal: xtal_out(sync.relay_xtal),
        cad_to_rx: receive_out(sync.cad_to_rx),
    })
}

/// Picks the relay scan a synchronized end device aims its next WOR frame at, appendix 1,
/// or `None` once the drift exceeds a period.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (synchronization, now_us, device_xtal_ppm, symbol_us, other_channel = false))]
pub fn lorawan_relay_next_wor(
    synchronization: LorawanSynchronization,
    now_us: u64,
    device_xtal_ppm: u32,
    symbol_us: u64,
    other_channel: bool,
) -> PyResult<Option<LorawanWorSlot>> {
    let sync = Synchronization {
        reference_us: synchronization.reference_us,
        cad_periodicity: periodicity_in(&synchronization.cad_periodicity)?,
        relay_xtal: xtal_in(&synchronization.relay_xtal)?,
        cad_to_rx: receive_in(&synchronization.cad_to_rx)?,
    };
    Ok(sync
        .next_wor(now_us, device_xtal_ppm, symbol_us, other_channel)
        .map(|slot| LorawanWorSlot {
            start_us: slot.start_us,
            preamble_symbols: slot.preamble_symbols,
        }))
}

/// Reads the second channel a relay or end device configuration describes, or `None` when
/// the index is not 1 or the offset is reserved.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lorawan_relay_second_channel(
    second_channel_index: u8,
    data_rate: u8,
    ack_offset: u8,
    frequency_hz: u32,
) -> Option<LoraRelayChannel> {
    relay_second_channel(second_channel_index, data_rate, ack_offset, frequency_hz).map(|channel| {
        LoraRelayChannel::from_core(
            channel.wor_frequency_hz,
            channel.ack_frequency_hz,
            channel.data_rate,
        )
    })
}
