//! Generated Node bindings for LoRa link math.
//!
//! These mirror the `pamoja-lora` Rust API: the time a transmission spends on air,
//! and the silence a regional duty-cycle limit then forces. Both are what keeps a
//! long-range node inside its budget, and both are pure arithmetic.
//!
//! A link is a small value rather than a resource, so it crosses as a plain
//! object. Times are microsecond counts, exact as JavaScript numbers at every
//! setting a real link uses.
//!
//! The link budget crosses as plain numbers of decibels, resolved to the hundredth of
//! a decibel the Rust crate holds.

use crate::checked::{self, OptionalWhole};
use napi_derive::napi;
use pamoja_lora::budget::{self, Decibels, Fcc15247, LinkBudget};
use pamoja_lora::LinkSettings;

/// The radio settings of a LoRa link.
#[napi(object)]
pub struct LoraLink {
    /// The spreading factor, 5 (fastest) to 12 (longest range).
    pub spreading_factor: checked::u8,
    /// The channel bandwidth in hertz, such as `125000`.
    pub bandwidth_hz: checked::u32,
    /// The coding-rate denominator, 5 to 8, for 4/5 to 4/8.
    pub coding_rate_denominator: checked::u8,
    /// The preamble length in symbols; the LoRa default is 8.
    pub preamble_symbols: checked::u16,
    /// Whether the frame carries an explicit header.
    pub explicit_header: bool,
    /// Whether the frame carries a CRC.
    pub crc: bool,
}

/// Returns the settings for a spreading factor and bandwidth, with LoRa defaults.
///
/// The defaults are coding rate 4/5, an eight-symbol preamble, an explicit header,
/// and CRC on, which is a typical uplink. The spreading factor is clamped to 5-12.
#[napi]
pub fn lora_link_default(spreading_factor: checked::u8, bandwidth_hz: checked::u32) -> LoraLink {
    let settings = LinkSettings::new(spreading_factor.get(), bandwidth_hz.get());
    LoraLink {
        spreading_factor: settings.spreading_factor().into(),
        bandwidth_hz: settings.bandwidth_hz().into(),
        coding_rate_denominator: 5.into(),
        preamble_symbols: 8.into(),
        explicit_header: true,
        crc: true,
    }
}

/// Returns the duration of one symbol on a link, in microseconds.
#[napi]
pub fn lora_symbol_time_us(link: LoraLink) -> f64 {
    settings(&link).symbol_time_us() as f64
}

/// Reports whether a link uses low data rate optimization.
///
/// It is on when a symbol lasts longer than 16 ms, which is SF11 and SF12 at 125 kHz and
/// SF12 at 250 kHz. The airtime assumes it, so a radio set up from these settings must
/// turn it on too.
#[napi]
pub fn lora_low_data_rate_optimization(link: LoraLink) -> bool {
    settings(&link).low_data_rate_optimization()
}

/// Returns the time on air of a payload, in microseconds.
///
/// This is the channel occupancy a transmission costs, which sets both the
/// duty-cycle budget and most of the energy the transmission spends.
#[napi]
pub fn lora_airtime_us(link: LoraLink, payload_len: checked::u32) -> f64 {
    settings(&link).airtime_us(payload_len.get() as usize) as f64
}

/// Returns the minimum silence after a transmission to honor a duty-cycle limit.
///
/// The limit is in parts per thousand, so `10` is 1%. A limit of `0` forbids
/// transmitting at all, which comes back as `null` rather than as a silence no
/// caller could ever wait out, and one of 1000 or more, the whole of the time, owes
/// no silence.
#[napi]
pub fn lora_min_off_time_us(
    link: LoraLink,
    payload_len: checked::u32,
    duty_cycle_permille: checked::u32,
) -> Option<f64> {
    if duty_cycle_permille.get() == 0 {
        return None;
    }
    Some(
        settings(&link).min_off_time_us(payload_len.get() as usize, duty_cycle_permille.get())
            as f64,
    )
}

/// Returns how many transmissions of a payload fit in an hour under a duty-cycle limit.
///
/// A transmission really costs its airtime plus the silence the limit forces after it. A
/// limit of `0` forbids transmitting, which comes back as `0`.
#[napi]
pub fn lora_messages_per_hour(
    link: LoraLink,
    payload_len: checked::u32,
    duty_cycle_permille: checked::u32,
) -> f64 {
    settings(&link).messages_per_hour(payload_len.get() as usize, duty_cycle_permille.get()) as f64
}

/// Describes link settings the way JavaScript holds them.
pub(crate) fn lora_link_of(settings: LinkSettings) -> LoraLink {
    LoraLink {
        spreading_factor: settings.spreading_factor().into(),
        bandwidth_hz: settings.bandwidth_hz().into(),
        coding_rate_denominator: settings.coding_rate_denominator().into(),
        preamble_symbols: settings.preamble_symbols().into(),
        explicit_header: settings.explicit_header(),
        crc: settings.crc(),
    }
}

/// Rebuilds the Rust link settings, clamping every value to its LoRa range.
pub(crate) fn settings(link: &LoraLink) -> LinkSettings {
    let mut settings = LinkSettings::new(link.spreading_factor.get(), link.bandwidth_hz.get())
        .with_coding_rate(link.coding_rate_denominator.get())
        .with_preamble(link.preamble_symbols.get());
    if !link.explicit_header {
        settings = settings.implicit_header();
    }
    if !link.crc {
        settings = settings.without_crc();
    }
    settings
}

/// The gains and losses of a LoRa link, from the transmitting radio to the receiving one.
#[napi(object)]
pub struct LoraLinkBudget {
    /// The power the transmitting radio delivers at its antenna port, in dBm.
    pub transmit_power_dbm: f64,
    /// The gain of the transmitting antenna over an isotropic antenna, in dBi.
    pub transmit_antenna_gain_dbi: f64,
    /// The loss in the cable and connectors between the transmitting radio and its
    /// antenna, in dB.
    pub transmit_cable_loss_db: f64,
    /// The gain of the receiving antenna over an isotropic antenna, in dBi.
    pub receive_antenna_gain_dbi: f64,
    /// The loss in the cable and connectors between the receiving antenna and its radio,
    /// in dB.
    pub receive_cable_loss_db: f64,
    /// The noise figure of the receiver, in dB.
    pub noise_figure_db: f64,
}

/// Returns a budget of 0 dBm between isotropic antennas with no cable loss.
///
/// The receiver has the 6 dB noise figure typical of a Semtech sub-GHz radio.
#[napi]
pub fn lora_link_budget_default() -> LoraLinkBudget {
    let budget = LinkBudget::default();
    LoraLinkBudget {
        transmit_power_dbm: db(budget.transmit_power_dbm),
        transmit_antenna_gain_dbi: db(budget.transmit_antenna_gain_dbi),
        transmit_cable_loss_db: db(budget.transmit_cable_loss_db),
        receive_antenna_gain_dbi: db(budget.receive_antenna_gain_dbi),
        receive_cable_loss_db: db(budget.receive_cable_loss_db),
        noise_figure_db: db(budget.noise_figure_db),
    }
}

/// Returns the equivalent isotropically radiated power of a budget, in dBm.
#[napi]
pub fn lora_eirp_dbm(budget: LoraLinkBudget) -> f64 {
    db(link_budget(&budget).eirp_dbm())
}

/// Returns the power that reaches the receiving radio across a path, in dBm.
#[napi]
pub fn lora_received_dbm(budget: LoraLinkBudget, path_loss_db: f64) -> f64 {
    db(link_budget(&budget).received_dbm(decibels(path_loss_db)))
}

/// Returns the weakest signal the receiver of a budget can demodulate on a link, in dBm.
#[napi]
pub fn lora_sensitivity_dbm(budget: LoraLinkBudget, link: LoraLink) -> f64 {
    db(link_budget(&budget).sensitivity_dbm(settings(&link)))
}

/// Returns the most path loss a link survives, in dB.
#[napi]
pub fn lora_max_path_loss_db(budget: LoraLinkBudget, link: LoraLink) -> f64 {
    db(link_budget(&budget).max_path_loss_db(settings(&link)))
}

/// Returns how far above the sensitivity a signal arrives across a path, in dB.
#[napi]
pub fn lora_margin_db(budget: LoraLinkBudget, link: LoraLink, path_loss_db: f64) -> f64 {
    db(link_budget(&budget).margin_db(settings(&link), decibels(path_loss_db)))
}

/// Returns the most transmit power that keeps the EIRP of a budget under a ceiling, in dBm.
#[napi]
pub fn lora_max_transmit_power_dbm(budget: LoraLinkBudget, eirp_ceiling_dbm: f64) -> f64 {
    db(link_budget(&budget).max_transmit_power_dbm(decibels(eirp_ceiling_dbm)))
}

/// Returns the thermal noise power in a channel, in dBm.
#[napi]
pub fn lora_noise_floor_dbm(bandwidth_hz: checked::u32) -> f64 {
    db(budget::noise_floor_dbm(bandwidth_hz.get()))
}

/// Returns the signal-to-noise ratio the LoRa demodulator needs at a spreading factor, in dB.
#[napi]
pub fn lora_demodulator_snr_db(spreading_factor: checked::u8) -> f64 {
    db(budget::demodulator_snr_db(spreading_factor.get()))
}

/// Returns the free-space basic transmission loss between isotropic antennas, in dB.
#[napi]
pub fn lora_free_space_loss_db(distance_m: checked::u32, frequency_hz: checked::u32) -> f64 {
    db(budget::free_space_loss_db(
        distance_m.get(),
        frequency_hz.get(),
    ))
}

/// Returns the radius of the first Fresnel ellipsoid at a point on a path, in millimeters.
#[napi]
pub fn lora_fresnel_radius_mm(
    near_m: checked::u32,
    far_m: checked::u32,
    frequency_hz: checked::u32,
) -> u32 {
    budget::fresnel_radius_mm(near_m.get(), far_m.get(), frequency_hz.get())
}

/// Returns the most conducted power 47 CFR 15.247 allows a 902-928 MHz transmitter
/// through an antenna, in dBm.
///
/// Without a channel count the transmitter uses digital modulation. A frequency hopping
/// system on fewer than 25 channels comes back as `null`, because paragraph (b)(2) sets
/// no limit for it.
#[napi]
pub fn lora_fcc_max_conducted_dbm(
    antenna_gain_dbi: f64,
    hopping_channels: Option<checked::u16>,
) -> Option<f64> {
    let system = match hopping_channels.get() {
        None => Fcc15247::DigitalModulation,
        Some(channels) => Fcc15247::FrequencyHopping { channels },
    };
    system.max_conducted_dbm(decibels(antenna_gain_dbi)).map(db)
}

/// Rebuilds the Rust link budget from the decibels JavaScript holds.
pub(crate) fn link_budget(budget: &LoraLinkBudget) -> LinkBudget {
    LinkBudget {
        transmit_power_dbm: decibels(budget.transmit_power_dbm),
        transmit_antenna_gain_dbi: decibels(budget.transmit_antenna_gain_dbi),
        transmit_cable_loss_db: decibels(budget.transmit_cable_loss_db),
        receive_antenna_gain_dbi: decibels(budget.receive_antenna_gain_dbi),
        receive_cable_loss_db: decibels(budget.receive_cable_loss_db),
        noise_figure_db: decibels(budget.noise_figure_db),
    }
}

/// Resolves a number of decibels to the hundredth of a decibel Rust holds.
pub(crate) fn decibels(value: f64) -> Decibels {
    Decibels::from_hundredths((value * 100.0).round() as i32)
}

/// Returns a level as a number of decibels.
pub(crate) fn db(value: Decibels) -> f64 {
    f64::from(value.hundredths()) / 100.0
}
