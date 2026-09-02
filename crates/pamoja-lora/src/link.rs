//! LoRa link settings and the time-on-air they imply.

// The symbol time above which low-data-rate optimization is required, in
// microseconds. Above 16 ms per symbol (SF11 and SF12 at 125 kHz) the LoRa modem
// turns it on, which the airtime formula accounts for.
const LOW_DATA_RATE_THRESHOLD_US: u64 = 16_000;

/// The radio settings of a LoRa link, enough to compute its time-on-air.
///
/// A LoRa transmission's duration is fixed by the spreading factor, the bandwidth,
/// the coding rate, and the frame options, not by the data itself beyond its length.
/// This struct gathers those settings and computes the two numbers a long-range
/// deployment lives by: the [`airtime`](LinkSettings::airtime_us) of a payload, and
/// the [`off time`](LinkSettings::min_off_time_us) a duty-cycle limit then forces
/// before the next transmission.
///
/// A higher spreading factor reaches much further but spends far longer on air, so the
/// same payload that takes tens of milliseconds at SF7 can take most of a second at
/// SF12, with a correspondingly longer mandatory silence. The arithmetic is exact and
/// integer-only, so it runs on the smallest node.
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
///
/// // The default European long-range setup: SF12, 125 kHz, coding rate 4/5.
/// let link = LinkSettings::new(12, 125_000);
///
/// // A 10-byte payload takes just under a second on air at SF12.
/// assert_eq!(link.airtime_us(10), 991_232);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkSettings {
    spreading_factor: u8,
    bandwidth_hz: u32,
    cr_denominator: u8,
    preamble_symbols: u16,
    explicit_header: bool,
    crc: bool,
}

impl LinkSettings {
    /// Creates link settings from a spreading factor and bandwidth, with LoRa defaults.
    ///
    /// The defaults are coding rate 4/5, an 8-symbol preamble, an explicit header, and
    /// CRC on, matching a typical uplink.
    ///
    /// # Arguments
    ///
    /// * `spreading_factor` - the spreading factor; clamped to the LoRa range 5 to 12.
    ///   SF5 and SF6 carry the data rates RP002-1.0.5 added to several regions.
    /// * `bandwidth_hz` - the channel bandwidth in hertz, such as `125_000`.
    ///
    /// # Returns
    ///
    /// The link settings.
    pub fn new(spreading_factor: u8, bandwidth_hz: u32) -> Self {
        Self {
            spreading_factor: spreading_factor.clamp(5, 12),
            bandwidth_hz,
            cr_denominator: 5,
            preamble_symbols: 8,
            explicit_header: true,
            crc: true,
        }
    }

    /// Sets the coding rate by its denominator, from 4/5 to 4/8.
    ///
    /// # Arguments
    ///
    /// * `denominator` - the coding-rate denominator, clamped to 5 to 8 for 4/5 to 4/8.
    ///
    /// # Returns
    ///
    /// The updated settings, for chaining.
    pub fn with_coding_rate(mut self, denominator: u8) -> Self {
        self.cr_denominator = denominator.clamp(5, 8);
        self
    }

    /// Sets the number of preamble symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - the preamble length in symbols; the LoRa default is 8.
    ///
    /// # Returns
    ///
    /// The updated settings, for chaining.
    pub fn with_preamble(mut self, symbols: u16) -> Self {
        self.preamble_symbols = symbols;
        self
    }

    /// Uses an implicit header, which omits the header symbols from each frame.
    ///
    /// # Returns
    ///
    /// The updated settings, for chaining.
    pub fn implicit_header(mut self) -> Self {
        self.explicit_header = false;
        self
    }

    /// Turns the frame CRC off.
    ///
    /// # Returns
    ///
    /// The updated settings, for chaining.
    pub fn without_crc(mut self) -> Self {
        self.crc = false;
        self
    }

    /// Returns the spreading factor.
    ///
    /// # Returns
    ///
    /// The spreading factor, from 7 to 12.
    pub fn spreading_factor(&self) -> u8 {
        self.spreading_factor
    }

    /// Returns the channel bandwidth in hertz.
    ///
    /// # Returns
    ///
    /// The bandwidth in hertz.
    pub fn bandwidth_hz(&self) -> u32 {
        self.bandwidth_hz
    }

    /// Returns the duration of one symbol in microseconds.
    ///
    /// # Returns
    ///
    /// The symbol time, `2^spreading_factor / bandwidth`, in microseconds.
    pub fn symbol_time_us(&self) -> u64 {
        (1u64 << self.spreading_factor) * 1_000_000 / u64::from(self.bandwidth_hz)
    }

    // The number of symbols in the payload portion of the frame.
    fn payload_symbols(&self, payload_len: usize) -> u32 {
        let sf = i32::from(self.spreading_factor);
        let low_data_rate = self.symbol_time_us() > LOW_DATA_RATE_THRESHOLD_US;
        let de = i32::from(low_data_rate);
        let ih = i32::from(!self.explicit_header);
        let crc = i32::from(self.crc);

        let numerator = 8 * payload_len as i32 - 4 * sf + 28 + 16 * crc - 20 * ih;
        let denominator = 4 * (sf - 2 * de); // always positive: sf >= 5, de <= 1
        let term = if numerator > 0 {
            let groups = (numerator as u32).div_ceil(denominator as u32);
            groups * u32::from(self.cr_denominator)
        } else {
            0
        };
        8 + term
    }

    /// Returns the time on air of a payload in microseconds.
    ///
    /// This is the channel occupancy the transmission costs: how long the radio holds
    /// the air, which sets both the duty-cycle budget and a large part of the energy
    /// the transmission spends.
    ///
    /// # Arguments
    ///
    /// * `payload_len` - the payload length in bytes.
    ///
    /// # Returns
    ///
    /// The time on air in microseconds.
    pub fn airtime_us(&self, payload_len: usize) -> u64 {
        let payload_symbols = u64::from(self.payload_symbols(payload_len));
        // Work in quarter-symbols so the preamble's 4.25-symbol tail stays exact.
        let quarter_symbols = (4 * u64::from(self.preamble_symbols) + 17) + 4 * payload_symbols;
        let symbol_units = quarter_symbols * (1u64 << self.spreading_factor);
        symbol_units * 1_000_000 / (4 * u64::from(self.bandwidth_hz))
    }

    /// Returns the minimum silence after a transmission to honor a duty-cycle limit.
    ///
    /// A duty-cycle limit caps the fraction of time a node may transmit, so after a
    /// transmission of a given airtime the node must stay quiet for long enough that
    /// the airtime is no more than that fraction of the whole cycle.
    ///
    /// # Arguments
    ///
    /// * `payload_len` - the payload length in bytes.
    /// * `duty_cycle_permille` - the duty-cycle limit in parts per thousand, so `10`
    ///   is 1%.
    ///
    /// # Returns
    ///
    /// The required off time in microseconds, or [`u64::MAX`] if the limit is zero.
    pub fn min_off_time_us(&self, payload_len: usize, duty_cycle_permille: u32) -> u64 {
        if duty_cycle_permille == 0 {
            return u64::MAX;
        }
        let airtime = self.airtime_us(payload_len);
        let permille = u64::from(duty_cycle_permille);
        airtime * (1000 - permille) / permille
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn airtime_matches_the_reference_sf12() {
        // The widely published reference value for SF12, BW125, CR4/5, 8-symbol
        // preamble, explicit header, CRC on, 10-byte payload.
        let link = LinkSettings::new(12, 125_000);
        assert_eq!(link.airtime_us(10), 991_232);
    }

    #[test]
    fn airtime_matches_the_reference_sf7() {
        let link = LinkSettings::new(7, 125_000);
        assert_eq!(link.airtime_us(10), 41_216);
    }

    #[test]
    fn a_higher_spreading_factor_is_slower() {
        let slow = LinkSettings::new(12, 125_000);
        let fast = LinkSettings::new(7, 125_000);
        assert!(slow.airtime_us(20) > fast.airtime_us(20) * 10);
    }

    #[test]
    fn symbol_time_follows_spreading_factor_and_bandwidth() {
        assert_eq!(LinkSettings::new(12, 125_000).symbol_time_us(), 32_768);
        assert_eq!(LinkSettings::new(7, 125_000).symbol_time_us(), 1_024);
        assert_eq!(LinkSettings::new(7, 250_000).symbol_time_us(), 512);
    }

    #[test]
    fn the_spreading_factor_is_clamped_to_the_lora_range() {
        assert_eq!(LinkSettings::new(3, 125_000).spreading_factor(), 5);
        assert_eq!(LinkSettings::new(20, 125_000).spreading_factor(), 12);
    }

    #[test]
    fn the_fastest_spreading_factors_survive_unclamped() {
        // RP002-1.0.5 added SF6 and SF5 data rates to several regions, so
        // clamping them up to SF7 would report the wrong airtime for them.
        assert_eq!(LinkSettings::new(6, 125_000).spreading_factor(), 6);
        assert_eq!(LinkSettings::new(5, 125_000).spreading_factor(), 5);
        assert!(
            LinkSettings::new(5, 125_000).airtime_us(10)
                < LinkSettings::new(7, 125_000).airtime_us(10),
            "a lower spreading factor is faster"
        );
    }

    #[test]
    fn a_one_percent_duty_cycle_forces_ninety_nine_times_the_airtime() {
        let link = LinkSettings::new(7, 125_000);
        let airtime = link.airtime_us(10);
        assert_eq!(link.min_off_time_us(10, 10), airtime * 99);
    }

    #[test]
    fn a_zero_duty_cycle_never_allows_another_send() {
        let link = LinkSettings::new(7, 125_000);
        assert_eq!(link.min_off_time_us(10, 0), u64::MAX);
    }

    #[test]
    fn implicit_header_and_no_crc_shorten_the_frame() {
        let full = LinkSettings::new(9, 125_000);
        let lean = LinkSettings::new(9, 125_000)
            .implicit_header()
            .without_crc();
        assert!(lean.airtime_us(20) < full.airtime_us(20));
    }

    #[test]
    fn a_zero_byte_payload_still_costs_the_preamble_and_header() {
        // Even with no payload, the preamble and header occupy the air.
        let link = LinkSettings::new(7, 125_000);
        assert!(link.airtime_us(0) > 0);
        assert!(link.airtime_us(0) < link.airtime_us(10));
    }

    #[test]
    fn airtime_grows_with_the_payload() {
        let link = LinkSettings::new(10, 125_000);
        assert!(link.airtime_us(1) <= link.airtime_us(10));
        assert!(link.airtime_us(10) < link.airtime_us(64));
    }

    #[test]
    fn a_wider_bandwidth_is_faster() {
        // Doubling the bandwidth roughly halves the time on air.
        let narrow = LinkSettings::new(9, 125_000).airtime_us(20);
        let wide = LinkSettings::new(9, 250_000).airtime_us(20);
        assert!(wide < narrow);
    }
}
