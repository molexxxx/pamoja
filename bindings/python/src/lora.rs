//! Generated Python bindings for LoRa link math.
//!
//! These mirror the `pamoja-lora` Rust API: the time a transmission spends on air,
//! and the silence a regional duty-cycle limit then forces. Both are what keeps a
//! long-range node inside its budget, and both are pure arithmetic.
//!
//! A link is a small value rather than a resource, so it crosses as a read-only
//! object built from its settings.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use pamoja_lora::LinkSettings;

/// The radio settings of a LoRa link.
#[gen_stub_pyclass]
#[pyclass]
pub struct LoraLink {
    /// The spreading factor, 7 (fastest) to 12 (longest range).
    #[pyo3(get)]
    spreading_factor: u8,
    /// The channel bandwidth in hertz, such as `125_000`.
    #[pyo3(get)]
    bandwidth_hz: u32,
    /// The coding-rate denominator, 5 to 8, for 4/5 to 4/8.
    #[pyo3(get)]
    coding_rate_denominator: u8,
    /// The preamble length in symbols; the LoRa default is 8.
    #[pyo3(get)]
    preamble_symbols: u16,
    /// Whether the frame carries an explicit header.
    #[pyo3(get)]
    explicit_header: bool,
    /// Whether the frame carries a CRC.
    #[pyo3(get)]
    crc: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl LoraLink {
    /// Creates link settings, clamping every value to its LoRa range.
    ///
    /// The defaults are coding rate 4/5, an eight-symbol preamble, an explicit
    /// header, and CRC on, which is a typical uplink.
    #[new]
    #[pyo3(signature = (
        spreading_factor,
        bandwidth_hz,
        coding_rate_denominator = 5,
        preamble_symbols = 8,
        explicit_header = true,
        crc = true,
    ))]
    fn new(
        spreading_factor: u8,
        bandwidth_hz: u32,
        coding_rate_denominator: u8,
        preamble_symbols: u16,
        explicit_header: bool,
        crc: bool,
    ) -> Self {
        LoraLink {
            spreading_factor: spreading_factor.clamp(5, 12),
            bandwidth_hz,
            coding_rate_denominator: coding_rate_denominator.clamp(5, 8),
            preamble_symbols,
            explicit_header,
            crc,
        }
    }

    /// The duration of one symbol on this link, in microseconds.
    fn symbol_time_us(&self) -> u64 {
        self.settings().symbol_time_us()
    }

    /// The time on air of a payload, in microseconds.
    ///
    /// This is the channel occupancy a transmission costs, which sets both the
    /// duty-cycle budget and most of the energy the transmission spends.
    fn airtime_us(&self, payload_len: usize) -> u64 {
        self.settings().airtime_us(payload_len)
    }

    /// The minimum silence after a transmission to honor a duty-cycle limit.
    ///
    /// The limit is in parts per thousand, so `10` is 1%. A limit of `0` forbids
    /// transmitting at all, which comes back as `None`.
    fn min_off_time_us(&self, payload_len: usize, duty_cycle_permille: u32) -> Option<u64> {
        if duty_cycle_permille == 0 {
            return None;
        }
        Some(
            self.settings()
                .min_off_time_us(payload_len, duty_cycle_permille),
        )
    }
}

impl LoraLink {
    /// Rebuilds the Rust link settings from the fields Python holds.
    fn settings(&self) -> LinkSettings {
        let mut settings = LinkSettings::new(self.spreading_factor, self.bandwidth_hz)
            .with_coding_rate(self.coding_rate_denominator)
            .with_preamble(self.preamble_symbols);
        if !self.explicit_header {
            settings = settings.implicit_header();
        }
        if !self.crc {
            settings = settings.without_crc();
        }
        settings
    }
}
