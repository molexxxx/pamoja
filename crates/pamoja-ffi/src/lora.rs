//! The C ABI for LoRa link math.
//!
//! These functions wrap [`pamoja_lora`] for callers that reach the SDK through the
//! flat C boundary: the time a transmission spends on air, and the silence a
//! duty-cycle limit then forces. Both are what a long-range node needs to stay
//! inside its regional budget, and both are pure arithmetic.
//!
//! A link is only scalars, so it crosses by value as [`PamojaLoraLink`] rather
//! than as a handle, which keeps the whole capability free of allocation.

use pamoja_lora::LinkSettings;

/// The radio settings of a LoRa link.
///
/// Build one with [`pamoja_lora_link_default`] and adjust the fields that differ
/// from the defaults. Values outside the ranges LoRa defines are clamped when the
/// link is used: the spreading factor to 7-12 and the coding-rate denominator to
/// 5-8.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraLink {
    /// The channel bandwidth in hertz, such as `125000`.
    pub bandwidth_hz: u32,
    /// The preamble length in symbols; the LoRa default is 8.
    pub preamble_symbols: u16,
    /// The spreading factor, 7 (fastest) to 12 (longest range).
    pub spreading_factor: u8,
    /// The coding-rate denominator, 5 to 8, for 4/5 to 4/8.
    pub coding_rate_denominator: u8,
    /// `1` for an explicit header, `0` to omit the header symbols.
    pub explicit_header: u8,
    /// `1` to append the frame CRC, `0` to leave it off.
    pub crc: u8,
}

/// Returns the settings for a spreading factor and bandwidth, with LoRa defaults.
///
/// The defaults are coding rate 4/5, an eight-symbol preamble, an explicit header,
/// and CRC on, which is a typical uplink.
///
/// # Arguments
///
/// * `spreading_factor` - the spreading factor, clamped to 5-12.
/// * `bandwidth_hz` - the channel bandwidth in hertz.
///
/// # Returns
///
/// The link settings, with the spreading factor already clamped.
#[no_mangle]
pub extern "C" fn pamoja_lora_link_default(
    spreading_factor: u8,
    bandwidth_hz: u32,
) -> PamojaLoraLink {
    let settings = LinkSettings::new(spreading_factor, bandwidth_hz);
    PamojaLoraLink {
        bandwidth_hz: settings.bandwidth_hz(),
        preamble_symbols: 8,
        spreading_factor: settings.spreading_factor(),
        coding_rate_denominator: 5,
        explicit_header: 1,
        crc: 1,
    }
}

/// Returns the duration of one symbol on a link, in microseconds.
///
/// # Arguments
///
/// * `link` - the link settings.
///
/// # Returns
///
/// The symbol time in microseconds.
#[no_mangle]
pub extern "C" fn pamoja_lora_symbol_time_us(link: PamojaLoraLink) -> u64 {
    settings(link).symbol_time_us()
}

/// Returns the time on air of a payload, in microseconds.
///
/// This is the channel occupancy a transmission costs: how long the radio holds
/// the air, which sets both the duty-cycle budget and most of the energy the
/// transmission spends.
///
/// # Arguments
///
/// * `link` - the link settings.
/// * `payload_len` - the payload length in bytes.
///
/// # Returns
///
/// The time on air in microseconds.
#[no_mangle]
pub extern "C" fn pamoja_lora_airtime_us(link: PamojaLoraLink, payload_len: usize) -> u64 {
    settings(link).airtime_us(payload_len)
}

/// Returns the minimum silence after a transmission to honor a duty-cycle limit.
///
/// # Arguments
///
/// * `link` - the link settings.
/// * `payload_len` - the payload length in bytes.
/// * `duty_cycle_permille` - the limit in parts per thousand, so `10` is 1%.
///
/// # Returns
///
/// The required off time in microseconds, or `UINT64_MAX` if the limit is zero,
/// which forbids transmitting at all.
#[no_mangle]
pub extern "C" fn pamoja_lora_min_off_time_us(
    link: PamojaLoraLink,
    payload_len: usize,
    duty_cycle_permille: u32,
) -> u64 {
    settings(link).min_off_time_us(payload_len, duty_cycle_permille)
}

/// Rebuilds the Rust link settings from the fields that crossed the boundary.
///
/// # Arguments
///
/// * `link` - the settings as the caller supplied them.
///
/// # Returns
///
/// The equivalent [`LinkSettings`], with every value clamped to its LoRa range.
fn settings(link: PamojaLoraLink) -> LinkSettings {
    let mut settings = LinkSettings::new(link.spreading_factor, link.bandwidth_hz)
        .with_coding_rate(link.coding_rate_denominator)
        .with_preamble(link.preamble_symbols);
    if link.explicit_header == 0 {
        settings = settings.implicit_header();
    }
    if link.crc == 0 {
        settings = settings.without_crc();
    }
    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_a_typical_uplink() {
        let link = pamoja_lora_link_default(12, 125_000);
        assert_eq!(link.spreading_factor, 12);
        assert_eq!(link.bandwidth_hz, 125_000);
        assert_eq!(link.coding_rate_denominator, 5);
        assert_eq!(link.preamble_symbols, 8);
        assert_eq!(link.explicit_header, 1);
        assert_eq!(link.crc, 1);
    }

    #[test]
    fn a_spreading_factor_beyond_lora_is_clamped() {
        assert_eq!(pamoja_lora_link_default(15, 125_000).spreading_factor, 12);
        assert_eq!(pamoja_lora_link_default(2, 125_000).spreading_factor, 5);
    }

    #[test]
    fn airtime_matches_the_rust_crate() {
        let link = pamoja_lora_link_default(12, 125_000);
        assert_eq!(
            pamoja_lora_airtime_us(link, 10),
            LinkSettings::new(12, 125_000).airtime_us(10)
        );
    }

    #[test]
    fn a_one_percent_duty_cycle_costs_ninety_nine_times_the_airtime() {
        let link = pamoja_lora_link_default(12, 125_000);
        let airtime = pamoja_lora_airtime_us(link, 20);
        assert_eq!(pamoja_lora_min_off_time_us(link, 20, 10), airtime * 99);
    }

    #[test]
    fn a_zero_duty_cycle_forbids_transmitting() {
        let link = pamoja_lora_link_default(7, 125_000);
        assert_eq!(pamoja_lora_min_off_time_us(link, 20, 0), u64::MAX);
    }
}
