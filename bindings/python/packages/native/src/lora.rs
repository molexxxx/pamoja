//! Generated Python bindings for LoRa link math.
//!
//! These mirror the `pamoja-lora` Rust API: the time a transmission spends on air,
//! and the silence a regional duty-cycle limit then forces. Both are what keeps a
//! long-range node inside its budget, and both are pure arithmetic.
//!
//! A link is a small value rather than a resource, so it crosses as a read-only
//! object built from its settings.
//!
//! The link budget crosses as floats of decibels, resolved to the hundredth of a
//! decibel the Rust crate holds.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use pamoja_lora::budget::{self, Decibels, Fcc15247};
use pamoja_lora::LinkSettings;

/// The radio settings of a LoRa link.
#[gen_stub_pyclass]
#[pyclass]
pub struct LoraLink {
    /// The spreading factor, 5 (fastest) to 12 (longest range).
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

impl LoraLink {
    /// Wraps the settings a channel plan reports, with the LoRa defaults.
    ///
    /// # Arguments
    ///
    /// * `settings` - the spreading factor and bandwidth a data rate selects.
    ///
    /// # Returns
    ///
    /// The link settings, at coding rate 4/5 with an eight-symbol preamble.
    pub(crate) fn from_settings(settings: LinkSettings) -> Self {
        Self {
            spreading_factor: settings.spreading_factor(),
            bandwidth_hz: settings.bandwidth_hz(),
            coding_rate_denominator: 5,
            preamble_symbols: 8,
            explicit_header: true,
            crc: true,
        }
    }
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
    pub(crate) fn settings(&self) -> LinkSettings {
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

/// The gains and losses of a LoRa link, from the transmitting radio to the receiving one.
///
/// Every value is in decibels, resolved to the hundredth of a decibel the Rust crate
/// holds.
#[gen_stub_pyclass]
#[pyclass]
pub struct LinkBudget {
    /// The power the transmitting radio delivers at its antenna port, in dBm.
    #[pyo3(get)]
    transmit_power_dbm: f64,
    /// The gain of the transmitting antenna over an isotropic antenna, in dBi.
    #[pyo3(get)]
    transmit_antenna_gain_dbi: f64,
    /// The loss in the cable and connectors between the transmitting radio and its
    /// antenna, in dB.
    #[pyo3(get)]
    transmit_cable_loss_db: f64,
    /// The gain of the receiving antenna over an isotropic antenna, in dBi.
    #[pyo3(get)]
    receive_antenna_gain_dbi: f64,
    /// The loss in the cable and connectors between the receiving antenna and its radio,
    /// in dB.
    #[pyo3(get)]
    receive_cable_loss_db: f64,
    /// The noise figure of the receiver, in dB.
    #[pyo3(get)]
    noise_figure_db: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl LinkBudget {
    /// Describes the gains and losses of a link.
    ///
    /// The defaults are 0 dBm between isotropic antennas with no cable loss, heard with
    /// the 6 dB noise figure typical of a Semtech sub-GHz radio.
    #[new]
    #[pyo3(signature = (
        transmit_power_dbm = 0.0,
        transmit_antenna_gain_dbi = 0.0,
        transmit_cable_loss_db = 0.0,
        receive_antenna_gain_dbi = 0.0,
        receive_cable_loss_db = 0.0,
        noise_figure_db = 6.0,
    ))]
    fn new(
        transmit_power_dbm: f64,
        transmit_antenna_gain_dbi: f64,
        transmit_cable_loss_db: f64,
        receive_antenna_gain_dbi: f64,
        receive_cable_loss_db: f64,
        noise_figure_db: f64,
    ) -> Self {
        Self {
            transmit_power_dbm: db(decibels(transmit_power_dbm)),
            transmit_antenna_gain_dbi: db(decibels(transmit_antenna_gain_dbi)),
            transmit_cable_loss_db: db(decibels(transmit_cable_loss_db)),
            receive_antenna_gain_dbi: db(decibels(receive_antenna_gain_dbi)),
            receive_cable_loss_db: db(decibels(receive_cable_loss_db)),
            noise_figure_db: db(decibels(noise_figure_db)),
        }
    }

    /// The equivalent isotropically radiated power, in dBm.
    ///
    /// The transmit power plus the transmitting antenna gain, less the transmitting
    /// cable loss. This is the figure regional power ceilings limit.
    fn eirp_dbm(&self) -> f64 {
        db(self.budget().eirp_dbm())
    }

    /// The power that reaches the receiving radio across a path, in dBm.
    fn received_dbm(&self, path_loss_db: f64) -> f64 {
        db(self.budget().received_dbm(decibels(path_loss_db)))
    }

    /// The weakest signal the receiver can demodulate on a link, in dBm.
    fn sensitivity_dbm(&self, link: PyRef<'_, LoraLink>) -> f64 {
        db(self.budget().sensitivity_dbm(link.settings()))
    }

    /// The most path loss the link survives, in dB.
    fn max_path_loss_db(&self, link: PyRef<'_, LoraLink>) -> f64 {
        db(self.budget().max_path_loss_db(link.settings()))
    }

    /// How far above the sensitivity a signal arrives across a path, in dB.
    ///
    /// Negative where the path loses more than the link survives.
    fn margin_db(&self, link: PyRef<'_, LoraLink>, path_loss_db: f64) -> f64 {
        db(self
            .budget()
            .margin_db(link.settings(), decibels(path_loss_db)))
    }

    /// The most transmit power that keeps the EIRP at or under a ceiling, in dBm.
    ///
    /// A higher-gain antenna leaves less power for the radio.
    fn max_transmit_power_dbm(&self, eirp_ceiling_dbm: f64) -> f64 {
        db(self
            .budget()
            .max_transmit_power_dbm(decibels(eirp_ceiling_dbm)))
    }
}

impl LinkBudget {
    /// Rebuilds the Rust link budget from the decibels Python holds.
    pub(crate) fn budget(&self) -> pamoja_lora::budget::LinkBudget {
        pamoja_lora::budget::LinkBudget {
            transmit_power_dbm: decibels(self.transmit_power_dbm),
            transmit_antenna_gain_dbi: decibels(self.transmit_antenna_gain_dbi),
            transmit_cable_loss_db: decibels(self.transmit_cable_loss_db),
            receive_antenna_gain_dbi: decibels(self.receive_antenna_gain_dbi),
            receive_cable_loss_db: decibels(self.receive_cable_loss_db),
            noise_figure_db: decibels(self.noise_figure_db),
        }
    }
}

/// The thermal noise power in a channel, in dBm.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lora_noise_floor_dbm(bandwidth_hz: u32) -> f64 {
    db(budget::noise_floor_dbm(bandwidth_hz))
}

/// The signal-to-noise ratio the LoRa demodulator needs at a spreading factor, in dB.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lora_demodulator_snr_db(spreading_factor: u8) -> f64 {
    db(budget::demodulator_snr_db(spreading_factor))
}

/// The free-space basic transmission loss between isotropic antennas, in dB.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lora_free_space_loss_db(distance_m: u32, frequency_hz: u32) -> f64 {
    db(budget::free_space_loss_db(distance_m, frequency_hz))
}

/// The radius of the first Fresnel ellipsoid at a point on a path, in millimeters.
#[gen_stub_pyfunction]
#[pyfunction]
pub fn lora_fresnel_radius_mm(near_m: u32, far_m: u32, frequency_hz: u32) -> u32 {
    budget::fresnel_radius_mm(near_m, far_m, frequency_hz)
}

/// The most conducted power 47 CFR 15.247 allows a 902-928 MHz transmitter through an
/// antenna, in dBm.
///
/// Without a channel count the transmitter uses digital modulation. A frequency hopping
/// system on fewer than 25 channels comes back as `None`, because paragraph (b)(2) sets
/// no limit for it.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (antenna_gain_dbi, hopping_channels = None))]
pub fn lora_fcc_max_conducted_dbm(
    antenna_gain_dbi: f64,
    hopping_channels: Option<u16>,
) -> Option<f64> {
    let system = match hopping_channels {
        None => Fcc15247::DigitalModulation,
        Some(channels) => Fcc15247::FrequencyHopping { channels },
    };
    system.max_conducted_dbm(decibels(antenna_gain_dbi)).map(db)
}

/// Resolves a number of decibels to the hundredth of a decibel Rust holds.
pub(crate) fn decibels(value: f64) -> Decibels {
    Decibels::from_hundredths((value * 100.0).round() as i32)
}

/// Returns a level as a number of decibels.
pub(crate) fn db(value: Decibels) -> f64 {
    f64::from(value.hundredths()) / 100.0
}
