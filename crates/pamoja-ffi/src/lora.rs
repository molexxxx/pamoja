//! The C ABI for LoRa link math.
//!
//! These functions wrap [`pamoja_lora`] for callers that reach the SDK through the
//! flat C boundary: the time a transmission spends on air, and the silence a
//! duty-cycle limit then forces. Both are what a long-range node needs to stay
//! inside its regional budget, and both are pure arithmetic.
//!
//! A link is only scalars, so it crosses by value as [`PamojaLoraLink`] rather
//! than as a handle, which keeps the whole capability free of allocation.

use pamoja_lora::budget::{self, Decibels, Fcc15247, LinkBudget};
use pamoja_lora::LinkSettings;

/// The radio settings of a LoRa link.
///
/// Build one with [`pamoja_lora_link_default`] and adjust the fields that differ
/// from the defaults. Values outside the ranges LoRa defines are clamped when the
/// link is used: the spreading factor to 5-12 and the coding-rate denominator to
/// 5-8.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraLink {
    /// The channel bandwidth in hertz, such as `125000`.
    pub bandwidth_hz: u32,
    /// The preamble length in symbols; the LoRa default is 8.
    pub preamble_symbols: u16,
    /// The spreading factor, 5 (fastest) to 12 (longest range).
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

/// The gains and losses of a LoRa link, from the transmitting radio to the receiving one.
///
/// Every field is in hundredths of a decibel, so `1400` is 14 dBm and `215` is 2.15 dBi.
/// Build one with [`pamoja_lora_link_budget_default`] and set the fields a deployment
/// chooses: the transmit power, the antenna and cable at each end, and the noise figure of
/// the receiver.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLoraLinkBudget {
    /// The power the transmitting radio delivers at its antenna port, in hundredths of a
    /// dBm.
    pub transmit_power_centi_dbm: i32,
    /// The gain of the transmitting antenna over an isotropic antenna, in hundredths of a
    /// dBi.
    pub transmit_antenna_gain_centi_dbi: i32,
    /// The loss in the cable and connectors between the transmitting radio and its
    /// antenna, in hundredths of a dB.
    pub transmit_cable_loss_centi_db: i32,
    /// The gain of the receiving antenna over an isotropic antenna, in hundredths of a
    /// dBi.
    pub receive_antenna_gain_centi_dbi: i32,
    /// The loss in the cable and connectors between the receiving antenna and its radio,
    /// in hundredths of a dB.
    pub receive_cable_loss_centi_db: i32,
    /// The noise figure of the receiver, in hundredths of a dB.
    pub noise_figure_centi_db: i32,
}

/// Returns a link budget of 0 dBm between isotropic antennas with no cable loss.
///
/// # Returns
///
/// The budget, heard with the 6 dB noise figure typical of a Semtech sub-GHz radio.
#[no_mangle]
pub extern "C" fn pamoja_lora_link_budget_default() -> PamojaLoraLinkBudget {
    let budget = LinkBudget::default();
    PamojaLoraLinkBudget {
        transmit_power_centi_dbm: budget.transmit_power_dbm.hundredths(),
        transmit_antenna_gain_centi_dbi: budget.transmit_antenna_gain_dbi.hundredths(),
        transmit_cable_loss_centi_db: budget.transmit_cable_loss_db.hundredths(),
        receive_antenna_gain_centi_dbi: budget.receive_antenna_gain_dbi.hundredths(),
        receive_cable_loss_centi_db: budget.receive_cable_loss_db.hundredths(),
        noise_figure_centi_db: budget.noise_figure_db.hundredths(),
    }
}

/// Returns the equivalent isotropically radiated power of a budget.
///
/// # Arguments
///
/// * `budget` - the link budget.
///
/// # Returns
///
/// The transmit power plus the transmitting antenna gain, less the transmitting cable
/// loss, in hundredths of a dBm.
#[no_mangle]
pub extern "C" fn pamoja_lora_link_budget_eirp_centi_dbm(budget: PamojaLoraLinkBudget) -> i32 {
    link_budget(budget).eirp_dbm().hundredths()
}

/// Returns the power that reaches the receiving radio across a path.
///
/// # Arguments
///
/// * `budget` - the link budget.
/// * `path_loss_centi_db` - the loss between the two antennas, in hundredths of a dB.
///
/// # Returns
///
/// The received power in hundredths of a dBm.
#[no_mangle]
pub extern "C" fn pamoja_lora_link_budget_received_centi_dbm(
    budget: PamojaLoraLinkBudget,
    path_loss_centi_db: i32,
) -> i32 {
    link_budget(budget)
        .received_dbm(Decibels::from_hundredths(path_loss_centi_db))
        .hundredths()
}

/// Returns the weakest signal the receiver of a budget can demodulate on a link.
///
/// # Arguments
///
/// * `budget` - the link budget, whose noise figure applies.
/// * `link` - the link settings, whose spreading factor and bandwidth set the floor.
///
/// # Returns
///
/// The sensitivity in hundredths of a dBm.
#[no_mangle]
pub extern "C" fn pamoja_lora_link_budget_sensitivity_centi_dbm(
    budget: PamojaLoraLinkBudget,
    link: PamojaLoraLink,
) -> i32 {
    link_budget(budget)
        .sensitivity_dbm(settings(link))
        .hundredths()
}

/// Returns the most path loss a link survives.
///
/// # Arguments
///
/// * `budget` - the link budget.
/// * `link` - the link settings, whose spreading factor and bandwidth set the sensitivity.
///
/// # Returns
///
/// The loss in hundredths of a dB above which the link does not close.
#[no_mangle]
pub extern "C" fn pamoja_lora_link_budget_max_path_loss_centi_db(
    budget: PamojaLoraLinkBudget,
    link: PamojaLoraLink,
) -> i32 {
    link_budget(budget)
        .max_path_loss_db(settings(link))
        .hundredths()
}

/// Returns how far above the sensitivity a signal arrives across a path.
///
/// # Arguments
///
/// * `budget` - the link budget.
/// * `link` - the link settings, whose spreading factor and bandwidth set the sensitivity.
/// * `path_loss_centi_db` - the loss between the two antennas, in hundredths of a dB.
///
/// # Returns
///
/// The link margin in hundredths of a dB, which is negative where the path loses more
/// than the link survives.
#[no_mangle]
pub extern "C" fn pamoja_lora_link_budget_margin_centi_db(
    budget: PamojaLoraLinkBudget,
    link: PamojaLoraLink,
    path_loss_centi_db: i32,
) -> i32 {
    link_budget(budget)
        .margin_db(
            settings(link),
            Decibels::from_hundredths(path_loss_centi_db),
        )
        .hundredths()
}

/// Returns the most transmit power that keeps the EIRP of a budget at or under a ceiling.
///
/// # Arguments
///
/// * `budget` - the link budget, whose transmitting antenna and cable apply.
/// * `eirp_ceiling_centi_dbm` - the EIRP limit, such as the ceiling a channel plan
///   publishes for a frequency, in hundredths of a dBm.
///
/// # Returns
///
/// The ceiling less the transmitting antenna gain, plus the transmitting cable loss, in
/// hundredths of a dBm.
#[no_mangle]
pub extern "C" fn pamoja_lora_link_budget_max_transmit_power_centi_dbm(
    budget: PamojaLoraLinkBudget,
    eirp_ceiling_centi_dbm: i32,
) -> i32 {
    link_budget(budget)
        .max_transmit_power_dbm(Decibels::from_hundredths(eirp_ceiling_centi_dbm))
        .hundredths()
}

/// Returns the thermal noise power in a channel.
///
/// # Arguments
///
/// * `bandwidth_hz` - the channel bandwidth in hertz; `0` counts as one hertz.
///
/// # Returns
///
/// -174 dBm/Hz plus `10 log10` of the bandwidth, in hundredths of a dBm.
#[no_mangle]
pub extern "C" fn pamoja_lora_noise_floor_centi_dbm(bandwidth_hz: u32) -> i32 {
    budget::noise_floor_dbm(bandwidth_hz).hundredths()
}

/// Returns the signal-to-noise ratio the LoRa demodulator needs at a spreading factor.
///
/// # Arguments
///
/// * `spreading_factor` - the spreading factor, clamped to 5-12.
///
/// # Returns
///
/// The typical figure from Table 6-1 of the SX1261/2 datasheet, in hundredths of a dB.
#[no_mangle]
pub extern "C" fn pamoja_lora_demodulator_snr_centi_db(spreading_factor: u8) -> i32 {
    budget::demodulator_snr_db(spreading_factor).hundredths()
}

/// Returns the free-space basic transmission loss between isotropic antennas.
///
/// This is Recommendation ITU-R P.525-5, equation (5).
///
/// # Arguments
///
/// * `distance_m` - the distance between the antennas in meters; `0` counts as one meter.
/// * `frequency_hz` - the carrier frequency in hertz; `0` counts as one hertz.
///
/// # Returns
///
/// The loss in hundredths of a dB.
#[no_mangle]
pub extern "C" fn pamoja_lora_free_space_loss_centi_db(distance_m: u32, frequency_hz: u32) -> i32 {
    budget::free_space_loss_db(distance_m, frequency_hz).hundredths()
}

/// Returns the radius of the first Fresnel ellipsoid at a point on a path.
///
/// This is Recommendation ITU-R P.526-16, equation (2). The radius is widest halfway
/// along the path.
///
/// # Arguments
///
/// * `near_m` - the distance from one antenna to the point, in meters.
/// * `far_m` - the distance from the point to the other antenna, in meters.
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The radius in millimeters, or `0` for a path of no length or a frequency of zero.
#[no_mangle]
pub extern "C" fn pamoja_lora_fresnel_radius_mm(near_m: u32, far_m: u32, frequency_hz: u32) -> u32 {
    budget::fresnel_radius_mm(near_m, far_m, frequency_hz)
}

/// Returns the most conducted power 47 CFR 15.247 allows a digitally modulated
/// 902-928 MHz transmitter through an antenna.
///
/// # Arguments
///
/// * `antenna_gain_centi_dbi` - the directional gain of the transmitting antenna, in
///   hundredths of a dBi.
///
/// # Returns
///
/// The 1 W limit of paragraph (b)(3), less whatever the gain exceeds 6 dBi, in hundredths
/// of a dBm.
#[no_mangle]
pub extern "C" fn pamoja_lora_fcc_digital_max_conducted_centi_dbm(
    antenna_gain_centi_dbi: i32,
) -> i32 {
    Fcc15247::DigitalModulation
        .max_conducted_dbm(Decibels::from_hundredths(antenna_gain_centi_dbi))
        .map_or(i32::MIN, Decibels::hundredths)
}

/// Returns the most conducted power 47 CFR 15.247 allows a 902-928 MHz frequency hopping
/// transmitter through an antenna.
///
/// # Arguments
///
/// * `hopping_channels` - the number of hopping channels the system uses.
/// * `antenna_gain_centi_dbi` - the directional gain of the transmitting antenna, in
///   hundredths of a dBi.
///
/// # Returns
///
/// The limit of paragraph (b)(2), less whatever the gain exceeds 6 dBi, in hundredths of a
/// dBm, or `INT32_MIN` for fewer than 25 channels, which that paragraph sets no limit for.
#[no_mangle]
pub extern "C" fn pamoja_lora_fcc_hopping_max_conducted_centi_dbm(
    hopping_channels: u16,
    antenna_gain_centi_dbi: i32,
) -> i32 {
    Fcc15247::FrequencyHopping {
        channels: hopping_channels,
    }
    .max_conducted_dbm(Decibels::from_hundredths(antenna_gain_centi_dbi))
    .map_or(i32::MIN, Decibels::hundredths)
}

/// Rebuilds the Rust link budget from the fields that crossed the boundary.
///
/// # Arguments
///
/// * `budget` - the budget as the caller supplied it.
///
/// # Returns
///
/// The equivalent [`LinkBudget`].
pub(crate) fn link_budget(budget: PamojaLoraLinkBudget) -> LinkBudget {
    LinkBudget {
        transmit_power_dbm: Decibels::from_hundredths(budget.transmit_power_centi_dbm),
        transmit_antenna_gain_dbi: Decibels::from_hundredths(
            budget.transmit_antenna_gain_centi_dbi,
        ),
        transmit_cable_loss_db: Decibels::from_hundredths(budget.transmit_cable_loss_centi_db),
        receive_antenna_gain_dbi: Decibels::from_hundredths(budget.receive_antenna_gain_centi_dbi),
        receive_cable_loss_db: Decibels::from_hundredths(budget.receive_cable_loss_centi_db),
        noise_figure_db: Decibels::from_hundredths(budget.noise_figure_centi_db),
    }
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
pub(crate) fn settings(link: PamojaLoraLink) -> LinkSettings {
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

    #[test]
    fn the_default_budget_is_isotropic_with_a_radio_noise_figure() {
        let budget = pamoja_lora_link_budget_default();
        assert_eq!(budget.transmit_power_centi_dbm, 0);
        assert_eq!(budget.transmit_antenna_gain_centi_dbi, 0);
        assert_eq!(budget.noise_figure_centi_db, 600);
        assert_eq!(pamoja_lora_link_budget_eirp_centi_dbm(budget), 0);
    }

    #[test]
    fn a_budget_matches_the_rust_crate() {
        let budget = PamojaLoraLinkBudget {
            transmit_power_centi_dbm: 1_400,
            transmit_antenna_gain_centi_dbi: 215,
            transmit_cable_loss_centi_db: 50,
            receive_antenna_gain_centi_dbi: 600,
            receive_cable_loss_centi_db: 150,
            noise_figure_centi_db: 300,
        };
        let link = pamoja_lora_link_default(12, 125_000);
        assert_eq!(pamoja_lora_link_budget_eirp_centi_dbm(budget), 1_565);
        assert_eq!(
            pamoja_lora_link_budget_received_centi_dbm(budget, 10_520),
            -8_505
        );
        assert_eq!(
            pamoja_lora_link_budget_sensitivity_centi_dbm(budget, link),
            -14_003
        );
        assert_eq!(
            pamoja_lora_link_budget_max_path_loss_centi_db(budget, link),
            16_018
        );
        assert_eq!(
            pamoja_lora_link_budget_margin_centi_db(budget, link, 10_520),
            5_498
        );
        assert_eq!(
            pamoja_lora_link_budget_max_transmit_power_centi_dbm(budget, 1_600),
            1_435
        );
    }

    #[test]
    fn the_range_terms_match_the_rust_crate() {
        assert_eq!(pamoja_lora_noise_floor_centi_dbm(125_000), -12_303);
        assert_eq!(pamoja_lora_demodulator_snr_centi_db(12), -2_000);
        assert_eq!(
            pamoja_lora_free_space_loss_centi_db(5_000, 868_100_000),
            10_520
        );
        assert_eq!(
            pamoja_lora_fresnel_radius_mm(2_500, 2_500, 868_100_000),
            20_777
        );
    }

    #[test]
    fn the_fcc_rule_takes_off_gain_past_6_dbi() {
        assert_eq!(pamoja_lora_fcc_digital_max_conducted_centi_dbm(215), 3_000);
        assert_eq!(pamoja_lora_fcc_digital_max_conducted_centi_dbm(900), 2_700);
        assert_eq!(
            pamoja_lora_fcc_hopping_max_conducted_centi_dbm(64, 900),
            2_700
        );
        assert_eq!(
            pamoja_lora_fcc_hopping_max_conducted_centi_dbm(25, 0),
            2_398
        );
        assert_eq!(
            pamoja_lora_fcc_hopping_max_conducted_centi_dbm(24, 0),
            i32::MIN
        );
    }
}
