//! Generated Node bindings for LoRa link math.
//!
//! These mirror the `pamoja-lora` Rust API: the time a transmission spends on air,
//! and the silence a regional duty-cycle limit then forces. Both are what keeps a
//! long-range node inside its budget, and both are pure arithmetic.
//!
//! A link is a small value rather than a resource, so it crosses as a plain
//! object. Times are microsecond counts, exact as JavaScript numbers at every
//! setting a real link uses.

use napi_derive::napi;
use pamoja_lora::LinkSettings;

/// The radio settings of a LoRa link.
#[napi(object)]
pub struct LoraLink {
    /// The spreading factor, 7 (fastest) to 12 (longest range).
    pub spreading_factor: u8,
    /// The channel bandwidth in hertz, such as `125000`.
    pub bandwidth_hz: u32,
    /// The coding-rate denominator, 5 to 8, for 4/5 to 4/8.
    pub coding_rate_denominator: u8,
    /// The preamble length in symbols; the LoRa default is 8.
    pub preamble_symbols: u16,
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
pub fn lora_link_default(spreading_factor: u8, bandwidth_hz: u32) -> LoraLink {
    let settings = LinkSettings::new(spreading_factor, bandwidth_hz);
    LoraLink {
        spreading_factor: settings.spreading_factor(),
        bandwidth_hz: settings.bandwidth_hz(),
        coding_rate_denominator: 5,
        preamble_symbols: 8,
        explicit_header: true,
        crc: true,
    }
}

/// Returns the duration of one symbol on a link, in microseconds.
#[napi]
pub fn lora_symbol_time_us(link: LoraLink) -> f64 {
    settings(&link).symbol_time_us() as f64
}

/// Returns the time on air of a payload, in microseconds.
///
/// This is the channel occupancy a transmission costs, which sets both the
/// duty-cycle budget and most of the energy the transmission spends.
#[napi]
pub fn lora_airtime_us(link: LoraLink, payload_len: u32) -> f64 {
    settings(&link).airtime_us(payload_len as usize) as f64
}

/// Returns the minimum silence after a transmission to honor a duty-cycle limit.
///
/// The limit is in parts per thousand, so `10` is 1%. A limit of `0` forbids
/// transmitting at all, which comes back as `null` rather than as a silence no
/// caller could ever wait out.
#[napi]
pub fn lora_min_off_time_us(
    link: LoraLink,
    payload_len: u32,
    duty_cycle_permille: u32,
) -> Option<f64> {
    if duty_cycle_permille == 0 {
        return None;
    }
    Some(settings(&link).min_off_time_us(payload_len as usize, duty_cycle_permille) as f64)
}

/// Rebuilds the Rust link settings, clamping every value to its LoRa range.
fn settings(link: &LoraLink) -> LinkSettings {
    let mut settings = LinkSettings::new(link.spreading_factor, link.bandwidth_hz)
        .with_coding_rate(link.coding_rate_denominator)
        .with_preamble(link.preamble_symbols);
    if !link.explicit_header {
        settings = settings.implicit_header();
    }
    if !link.crc {
        settings = settings.without_crc();
    }
    settings
}
