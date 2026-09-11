//! Link budgets: how much signal a LoRa link has to spend, and where it goes.
//!
//! A link closes while the power reaching the receiver stays above the weakest signal the
//! receiver can still demodulate. Everything between the transmitting radio and that
//! demodulator adds or takes away decibels: the transmit power, the antenna and cable at
//! each end, and the path itself. [`LinkBudget`] keeps that account, and the functions
//! here give each term on its own.
//!
//! Every value is a [`Decibels`], held to a hundredth of a decibel and computed with a
//! fixed-point logarithm, so a node can budget its own link with no floating point. Each
//! term comes from the document that defines it:
//!
//! - [`free_space_loss_db`] is the free-space basic transmission loss of Recommendation
//!   ITU-R P.525-5, equation (5), `20 log10(4 pi d / lambda)`.
//! - [`fresnel_radius_mm`] is the radius of the first Fresnel ellipsoid, from
//!   Recommendation ITU-R P.526-16, equation (2).
//! - [`noise_floor_dbm`] is the thermal noise in a channel, -174 dBm/Hz plus `10 log10` of
//!   the bandwidth, as Semtech application note AN1200.22 derives it.
//! - [`demodulator_snr_db`] is the signal-to-noise ratio the LoRa demodulator needs at each
//!   spreading factor, from Table 6-1 of the Semtech SX1261/2 datasheet.
//! - [`Fcc15247`] is the conducted power limit and antenna gain rule of 47 CFR 15.247(b).
//!
//! A receiver's sensitivity is its noise floor, raised by its noise figure and lowered by
//! the demodulator SNR. With the 6 dB noise figure AN1200.22 takes as typical, that sum
//! lands within about half a decibel of the SX1261/2 datasheet at 125 and 250 kHz, and
//! within 3 dB of every SF7 to SF12 figure in it and in the SX1276 datasheet. The
//! datasheet figures are measured, so where a deployment has them for its own radio, they
//! are the better number.
//!
//! # Examples
//!
//! ```
//! use pamoja_lora::budget::{self, Decibels, LinkBudget};
//! use pamoja_lora::LinkSettings;
//!
//! // SF12 at 125 kHz, sent at 14 dBm between antennas with no gain.
//! let link = LinkSettings::new(12, 125_000);
//! let budget = LinkBudget {
//!     transmit_power_dbm: Decibels::from_db(14),
//!     ..LinkBudget::default()
//! };
//!
//! // With the default 6 dB noise figure the receiver hears down to -137.03 dBm, so the
//! // link survives 151.03 dB of path loss.
//! assert_eq!(budget.sensitivity_dbm(link).to_string(), "-137.03");
//! assert_eq!(budget.max_path_loss_db(link).to_string(), "151.03");
//!
//! // Five kilometers of free space at 868.1 MHz takes 105.20 dB of that.
//! let loss = budget::free_space_loss_db(5_000, 868_100_000);
//! assert_eq!(loss.to_string(), "105.20");
//! assert_eq!(budget.margin_db(link, loss).to_string(), "45.83");
//! ```

use core::fmt;
use core::ops::{Add, Neg, Sub};

use crate::LinkSettings;

// log10(2) in Q64 fixed point.
const LOG10_2_Q64: u128 = 5_553_023_288_523_357_132;

// log10(4 pi / c) in Q64 fixed point, with c in meters per second.
const LOG10_4PI_OVER_C_Q64: i128 = -136_092_899_020_721_497_209;

// The speed of light in meters per second, exact by the definition of the meter.
const SPEED_OF_LIGHT_M_PER_S: u128 = 299_792_458;

// Fraction bits of the binary logarithm; at 40 the error stays below 1e-9 of a hundredth.
const LOG2_FRACTION_BITS: u32 = 40;

// The conducted limits of 47 CFR 15.247(b)(2) and (b)(3): 1 W and 0.25 W, in dBm.
const ONE_WATT_DBM: Decibels = Decibels::from_hundredths(3_000);
const QUARTER_WATT_DBM: Decibels = Decibels::from_hundredths(2_398);

// The antenna gain 47 CFR 15.247(b)(4) allows before the conducted power comes down.
const FCC_GAIN_ALLOWANCE_DBI: Decibels = Decibels::from_db(6);

// The typical LoRa demodulator SNR from SF5 to SF12, SX1261/2 datasheet Table 6-1.
const DEMODULATOR_SNR: [Decibels; 8] = [
    Decibels::from_hundredths(-250),
    Decibels::from_hundredths(-500),
    Decibels::from_hundredths(-750),
    Decibels::from_hundredths(-1_000),
    Decibels::from_hundredths(-1_250),
    Decibels::from_hundredths(-1_500),
    Decibels::from_hundredths(-1_750),
    Decibels::from_hundredths(-2_000),
];

/// The thermal noise power density at room temperature, in dBm per hertz.
///
/// Semtech AN1200.22 derives it as `10 log10(k T 1000)` with T at 293 K, and works with
/// -174.
pub const THERMAL_NOISE_DBM_PER_HZ: Decibels = Decibels::from_db(-174);

/// A typical noise figure for a Semtech sub-GHz LoRa radio, in dB.
///
/// AN1200.22 takes 6 dB as the noise figure of the receiver behind the SX1272 and SX1276
/// datasheet sensitivities.
pub const RADIO_NOISE_FIGURE_DB: Decibels = Decibels::from_db(6);

/// A typical noise figure for a LoRaWAN gateway receiver, in dB.
///
/// Semtech TN1300.05 works with 3 dB for a gateway.
pub const GATEWAY_NOISE_FIGURE_DB: Decibels = Decibels::from_db(3);

/// A level, a gain, or a loss in decibels, held to a hundredth of a decibel.
///
/// One type carries absolute levels in dBm, antenna gains in dBi, and plain ratios in dB,
/// because a link budget adds and subtracts all three. The name of each value in this
/// module says which one it is. Arithmetic saturates rather than overflowing, and the
/// value prints with two decimals and no unit.
///
/// # Examples
///
/// ```
/// use pamoja_lora::budget::Decibels;
///
/// // A 2.15 dBi dipole behind half a decibel of cable.
/// let net = Decibels::from_hundredths(215) - Decibels::from_tenths(5);
/// assert_eq!(net.to_string(), "1.65");
/// assert_eq!(net.round_db(), 2);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Decibels(i32);

impl Decibels {
    /// Zero decibels: no gain and no loss.
    pub const ZERO: Self = Self(0);

    /// Creates a value from whole decibels.
    ///
    /// # Arguments
    ///
    /// * `db` - the value in decibels.
    ///
    /// # Returns
    ///
    /// The value, saturating at the range a hundredth-of-a-decibel `i32` holds.
    pub const fn from_db(db: i32) -> Self {
        Self(db.saturating_mul(100))
    }

    /// Creates a value from tenths of a decibel, the precision most antenna datasheets
    /// publish.
    ///
    /// # Arguments
    ///
    /// * `tenths` - the value in tenths of a decibel, so `21` is 2.1 dB.
    ///
    /// # Returns
    ///
    /// The value, saturating at the range a hundredth-of-a-decibel `i32` holds.
    pub const fn from_tenths(tenths: i32) -> Self {
        Self(tenths.saturating_mul(10))
    }

    /// Creates a value from hundredths of a decibel, the precision it is held to.
    ///
    /// # Arguments
    ///
    /// * `hundredths` - the value in hundredths of a decibel, so `215` is 2.15 dB.
    ///
    /// # Returns
    ///
    /// The value.
    pub const fn from_hundredths(hundredths: i32) -> Self {
        Self(hundredths)
    }

    /// Returns the value in hundredths of a decibel.
    ///
    /// # Returns
    ///
    /// The exact value this type holds.
    pub const fn hundredths(self) -> i32 {
        self.0
    }

    /// Returns the value rounded to whole decibels, halves away from zero.
    ///
    /// # Returns
    ///
    /// The nearest whole decibel, for a figure read in whole decibels. A transmit power
    /// setting under a ceiling wants [`floor_db`](Self::floor_db) instead.
    pub const fn round_db(self) -> i32 {
        self.0 / 100 + self.0 % 100 / 50
    }

    /// Returns the value rounded down to whole decibels.
    ///
    /// # Returns
    ///
    /// The largest whole decibel at or below the value, which is how a transmit power
    /// setting stays under a ceiling.
    pub const fn floor_db(self) -> i32 {
        self.0.div_euclid(100)
    }
}

impl Add for Decibels {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Decibels {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl Neg for Decibels {
    type Output = Self;

    fn neg(self) -> Self {
        Self(self.0.saturating_neg())
    }
}

impl fmt::Display for Decibels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = [0u8; 16];
        let mut start = text.len();
        let mut rest = self.0.unsigned_abs();
        let mut place = 0;
        while place < 3 || rest > 0 {
            if place == 2 {
                start -= 1;
                text[start] = b'.';
            }
            start -= 1;
            text[start] = b'0' + (rest % 10) as u8;
            rest /= 10;
            place += 1;
        }
        if self.0 < 0 {
            start -= 1;
            text[start] = b'-';
        }
        f.pad(core::str::from_utf8(&text[start..]).map_err(|_| fmt::Error)?)
    }
}

/// The gains and losses of a link, from the transmitting radio to the receiving one.
///
/// The fields are the parts of a deployment a maker chooses: how hard the radio drives,
/// which antenna goes on each end, and how much cable sits between each antenna and its
/// radio. The methods add them up against a path and a receiver. The default is 0 dBm
/// between two isotropic antennas with no cable, heard with [`RADIO_NOISE_FIGURE_DB`].
///
/// # Examples
///
/// ```
/// use pamoja_lora::budget::{Decibels, LinkBudget};
///
/// // 14 dBm into a 2.15 dBi antenna through half a decibel of pigtail.
/// let budget = LinkBudget {
///     transmit_power_dbm: Decibels::from_db(14),
///     transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
///     transmit_cable_loss_db: Decibels::from_tenths(5),
///     ..LinkBudget::default()
/// };
/// assert_eq!(budget.eirp_dbm().to_string(), "15.65");
///
/// // Under a 16 dBm EIRP ceiling, that antenna and cable leave 14.35 dBm for the radio.
/// let most = budget.max_transmit_power_dbm(Decibels::from_db(16));
/// assert_eq!(most.to_string(), "14.35");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LinkBudget {
    /// The power the transmitting radio delivers at its antenna port, in dBm.
    pub transmit_power_dbm: Decibels,
    /// The gain of the transmitting antenna over an isotropic antenna, in dBi.
    pub transmit_antenna_gain_dbi: Decibels,
    /// The loss in the cable and connectors between the transmitting radio and its
    /// antenna, in dB.
    pub transmit_cable_loss_db: Decibels,
    /// The gain of the receiving antenna over an isotropic antenna, in dBi.
    pub receive_antenna_gain_dbi: Decibels,
    /// The loss in the cable and connectors between the receiving antenna and its radio,
    /// in dB.
    pub receive_cable_loss_db: Decibels,
    /// The noise figure of the receiver, in dB, such as [`RADIO_NOISE_FIGURE_DB`] for a
    /// node or [`GATEWAY_NOISE_FIGURE_DB`] for a gateway.
    pub noise_figure_db: Decibels,
}

impl Default for LinkBudget {
    fn default() -> Self {
        Self {
            transmit_power_dbm: Decibels::ZERO,
            transmit_antenna_gain_dbi: Decibels::ZERO,
            transmit_cable_loss_db: Decibels::ZERO,
            receive_antenna_gain_dbi: Decibels::ZERO,
            receive_cable_loss_db: Decibels::ZERO,
            noise_figure_db: RADIO_NOISE_FIGURE_DB,
        }
    }
}

impl LinkBudget {
    /// Returns the equivalent isotropically radiated power, in dBm.
    ///
    /// # Returns
    ///
    /// The transmit power plus the transmitting antenna gain, less the transmitting cable
    /// loss. This is the figure regional power ceilings limit.
    pub fn eirp_dbm(&self) -> Decibels {
        self.transmit_power_dbm + self.transmit_antenna_gain_dbi - self.transmit_cable_loss_db
    }

    /// Returns the power that reaches the receiving radio across a path, in dBm.
    ///
    /// # Arguments
    ///
    /// * `path_loss_db` - the loss between the two antennas, such as
    ///   [`free_space_loss_db`].
    ///
    /// # Returns
    ///
    /// The EIRP less the path loss, plus the receiving antenna gain, less the receiving
    /// cable loss.
    pub fn received_dbm(&self, path_loss_db: Decibels) -> Decibels {
        self.eirp_dbm() - path_loss_db + self.receive_antenna_gain_dbi - self.receive_cable_loss_db
    }

    /// Returns the weakest signal the receiver can demodulate on a link, in dBm.
    ///
    /// # Arguments
    ///
    /// * `link` - the link settings, whose spreading factor and bandwidth set the floor.
    ///
    /// # Returns
    ///
    /// The [sensitivity](fn@sensitivity_dbm) at this budget's noise figure.
    pub fn sensitivity_dbm(&self, link: LinkSettings) -> Decibels {
        sensitivity_dbm(link, self.noise_figure_db)
    }

    /// Returns the most path loss the link survives, in dB.
    ///
    /// # Arguments
    ///
    /// * `link` - the link settings, whose spreading factor and bandwidth set the
    ///   sensitivity.
    ///
    /// # Returns
    ///
    /// How far the power reaching the receiver with no path loss sits above the
    /// sensitivity. A path that loses more than this does not close.
    pub fn max_path_loss_db(&self, link: LinkSettings) -> Decibels {
        self.received_dbm(Decibels::ZERO) - self.sensitivity_dbm(link)
    }

    /// Returns how far above the sensitivity a signal arrives across a path, in dB.
    ///
    /// # Arguments
    ///
    /// * `link` - the link settings, whose spreading factor and bandwidth set the
    ///   sensitivity.
    /// * `path_loss_db` - the loss between the two antennas.
    ///
    /// # Returns
    ///
    /// The link margin, which is negative where the path loses more than the link
    /// survives.
    pub fn margin_db(&self, link: LinkSettings, path_loss_db: Decibels) -> Decibels {
        self.received_dbm(path_loss_db) - self.sensitivity_dbm(link)
    }

    /// Returns the most transmit power that keeps the EIRP at or under a ceiling, in dBm.
    ///
    /// # Arguments
    ///
    /// * `eirp_ceiling_dbm` - the EIRP limit, such as the `max_eirp_dbm` a channel plan
    ///   publishes for a frequency.
    ///
    /// # Returns
    ///
    /// The ceiling less the transmitting antenna gain, plus the transmitting cable loss.
    /// A higher-gain antenna leaves less power for the radio.
    pub fn max_transmit_power_dbm(&self, eirp_ceiling_dbm: Decibels) -> Decibels {
        eirp_ceiling_dbm - self.transmit_antenna_gain_dbi + self.transmit_cable_loss_db
    }
}

/// A 902-928 MHz transmitter as 47 CFR 15.247 classes it for a power limit.
///
/// Paragraph (b) caps the peak conducted output power, and paragraph (b)(4) bases that cap
/// on an antenna of no more than 6 dBi: past that, the conducted power comes down by every
/// decibel the gain exceeds 6 dBi. Which class a device belongs to follows from how it is
/// built and certified, which this type does not decide.
///
/// # Examples
///
/// ```
/// use pamoja_lora::budget::{Decibels, Fcc15247};
///
/// // A 9 dBi Yagi takes 3 dB off the 1 W limit for digital modulation.
/// let yagi = Decibels::from_db(9);
/// let limit = Fcc15247::DigitalModulation.max_conducted_dbm(yagi);
/// assert_eq!(limit, Some(Decibels::from_db(27)));
///
/// // Paragraph (b)(2) names no limit for hopping on fewer than 25 channels.
/// let hopping = Fcc15247::FrequencyHopping { channels: 20 };
/// assert_eq!(hopping.max_conducted_dbm(yagi), None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fcc15247 {
    /// A system using digital modulation, limited to 1 W by paragraph (b)(3).
    DigitalModulation,
    /// A frequency hopping system, limited by paragraph (b)(2) to 1 W on at least 50
    /// hopping channels and to 0.25 W on 25 to 49.
    FrequencyHopping {
        /// The number of hopping channels the system uses.
        channels: u16,
    },
}

impl Fcc15247 {
    /// Returns the most conducted output power the rule allows through an antenna, in dBm.
    ///
    /// # Arguments
    ///
    /// * `antenna_gain_dbi` - the directional gain of the transmitting antenna.
    ///
    /// # Returns
    ///
    /// The limit of paragraph (b)(2) or (b)(3), less whatever the gain exceeds 6 dBi, or
    /// `None` for a hopping system on fewer than 25 channels, which paragraph (b)(2) sets
    /// no limit for.
    pub fn max_conducted_dbm(self, antenna_gain_dbi: Decibels) -> Option<Decibels> {
        let limit = match self {
            Self::DigitalModulation => ONE_WATT_DBM,
            Self::FrequencyHopping { channels } if channels >= 50 => ONE_WATT_DBM,
            Self::FrequencyHopping { channels } if channels >= 25 => QUARTER_WATT_DBM,
            Self::FrequencyHopping { .. } => return None,
        };
        let excess = antenna_gain_dbi - FCC_GAIN_ALLOWANCE_DBI;
        Some(if excess > Decibels::ZERO {
            limit - excess
        } else {
            limit
        })
    }
}

/// Returns the thermal noise power in a channel, in dBm.
///
/// Semtech AN1200.22 derives the floor as `10 log10(k T B 1000)`, which at room
/// temperature is [`THERMAL_NOISE_DBM_PER_HZ`] plus `10 log10` of the bandwidth. LoRa
/// demodulates below this floor by the processing gain its spreading factor buys, and
/// [`demodulator_snr_db`] gives how far.
///
/// # Arguments
///
/// * `bandwidth_hz` - the channel bandwidth in hertz; `0` counts as one hertz.
///
/// # Returns
///
/// The noise power in dBm.
///
/// # Examples
///
/// ```
/// use pamoja_lora::budget::noise_floor_dbm;
///
/// assert_eq!(noise_floor_dbm(125_000).to_string(), "-123.03");
/// ```
pub fn noise_floor_dbm(bandwidth_hz: u32) -> Decibels {
    THERMAL_NOISE_DBM_PER_HZ + round_q64(1000 * log10_q64(u64::from(bandwidth_hz)))
}

/// Returns the signal-to-noise ratio the LoRa demodulator needs at a spreading factor, in
/// dB.
///
/// These are the typical figures of Table 6-1 in the Semtech SX1261/2 datasheet, from
/// -2.5 dB at SF5 to -20 dB at SF12, which Semtech TN1300.05 repeats per data rate. A
/// negative ratio is a signal received below the noise, and each step up in spreading
/// factor reaches another 2.5 dB further down.
///
/// # Arguments
///
/// * `spreading_factor` - the spreading factor, clamped to 5 to 12 as
///   [`LinkSettings::new`] clamps it.
///
/// # Returns
///
/// The required SNR in dB.
pub fn demodulator_snr_db(spreading_factor: u8) -> Decibels {
    DEMODULATOR_SNR[usize::from(spreading_factor.clamp(5, 12) - 5)]
}

/// Returns the weakest signal a receiver can demodulate on a link, in dBm.
///
/// Sensitivity is the [noise floor](noise_floor_dbm) of the channel, raised by the noise
/// figure of the receiver and lowered by the [demodulator SNR](demodulator_snr_db). How
/// closely that tracks the datasheets is in the [module documentation](crate::budget).
///
/// # Arguments
///
/// * `link` - the link settings, whose spreading factor and bandwidth set the floor.
/// * `noise_figure_db` - the noise figure of the receiver, such as
///   [`RADIO_NOISE_FIGURE_DB`].
///
/// # Returns
///
/// The sensitivity in dBm.
///
/// # Examples
///
/// ```
/// use pamoja_lora::budget::{sensitivity_dbm, RADIO_NOISE_FIGURE_DB};
/// use pamoja_lora::LinkSettings;
///
/// let fastest = sensitivity_dbm(LinkSettings::new(7, 125_000), RADIO_NOISE_FIGURE_DB);
/// let furthest = sensitivity_dbm(LinkSettings::new(12, 125_000), RADIO_NOISE_FIGURE_DB);
/// assert_eq!(fastest.to_string(), "-124.53");
/// assert_eq!(furthest.to_string(), "-137.03");
/// ```
pub fn sensitivity_dbm(link: LinkSettings, noise_figure_db: Decibels) -> Decibels {
    noise_floor_dbm(link.bandwidth_hz())
        + noise_figure_db
        + demodulator_snr_db(link.spreading_factor())
}

/// Returns the free-space basic transmission loss between isotropic antennas, in dB.
///
/// This is Recommendation ITU-R P.525-5, equation (5), `20 log10(4 pi d / lambda)`: the
/// loss with nothing in the way, which grows by 6.02 dB each time the distance doubles.
/// Equation (6) writes the same loss in megahertz and kilometers as
/// `32.4 + 20 log10 f + 20 log10 d`, rounding its constant down from 32.45, so it reads
/// about 0.05 dB low. A path near the ground loses more once terrain enters the first
/// Fresnel ellipsoid, which [`fresnel_radius_mm`] sizes.
///
/// # Arguments
///
/// * `distance_m` - the distance between the antennas in meters; `0` counts as one meter.
/// * `frequency_hz` - the carrier frequency in hertz; `0` counts as one hertz.
///
/// # Returns
///
/// The loss in dB. The formula assumes the antennas sit in each other's far field, many
/// wavelengths apart.
///
/// # Examples
///
/// ```
/// use pamoja_lora::budget::free_space_loss_db;
///
/// assert_eq!(free_space_loss_db(1_000, 868_100_000).to_string(), "91.22");
/// assert_eq!(free_space_loss_db(2_000, 868_100_000).to_string(), "97.24");
/// ```
pub fn free_space_loss_db(distance_m: u32, frequency_hz: u32) -> Decibels {
    let ratio = log10_q64(u64::from(distance_m))
        + log10_q64(u64::from(frequency_hz))
        + LOG10_4PI_OVER_C_Q64;
    round_q64(2000 * ratio)
}

/// Returns the radius of the first Fresnel ellipsoid at a point on a path, in millimeters.
///
/// Recommendation ITU-R P.526-16 treats a path as line of sight, with negligible
/// diffraction, when no obstacle enters the first Fresnel ellipsoid, and starts the
/// diffraction zone where the clearance falls to 60% of its radius. The radius is widest
/// halfway along the path. This is equation (2), `sqrt(lambda d1 d2 / (d1 + d2))`;
/// equation (3) in practical units rounds its constant to 550 and so reads about half a
/// percent wide.
///
/// # Arguments
///
/// * `near_m` - the distance from one antenna to the point, in meters.
/// * `far_m` - the distance from the point to the other antenna, in meters.
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The radius in millimeters, rounded to the nearest, or `0` for a path of no length or a
/// frequency of zero.
///
/// # Examples
///
/// ```
/// use pamoja_lora::budget::fresnel_radius_mm;
///
/// // Halfway along a 5 km link at 868.1 MHz the radius is 20.78 m, and 60% of it,
/// // 12.47 m, is the clearance that keeps the path out of the diffraction zone.
/// let radius = fresnel_radius_mm(2_500, 2_500, 868_100_000);
/// assert_eq!(radius, 20_777);
/// assert_eq!(radius * 6 / 10, 12_466);
/// ```
pub fn fresnel_radius_mm(near_m: u32, far_m: u32, frequency_hz: u32) -> u32 {
    let path_m = u128::from(near_m) + u128::from(far_m);
    if path_m == 0 || frequency_hz == 0 {
        return 0;
    }
    let four_squared_mm2 =
        4 * SPEED_OF_LIGHT_M_PER_S * u128::from(near_m) * u128::from(far_m) * 1_000_000
            / (u128::from(frequency_hz) * path_m);
    u32::try_from(four_squared_mm2.isqrt().div_ceil(2)).unwrap_or(u32::MAX)
}

/// Returns `log10(value)` in Q64 fixed point, counting zero as one.
///
/// The binary logarithm comes from squaring the normalized mantissa once per fraction bit,
/// and converts to base ten through `LOG10_2_Q64`.
fn log10_q64(value: u64) -> i128 {
    let value = value.max(1);
    let whole = 63 - value.leading_zeros();
    let mut mantissa = u128::from(value) << (63 - whole);
    let mut fraction: u128 = 0;
    for _ in 0..LOG2_FRACTION_BITS {
        mantissa = (mantissa * mantissa) >> 63;
        fraction <<= 1;
        if mantissa >= 1 << 64 {
            mantissa >>= 1;
            fraction |= 1;
        }
    }
    let log2_q40 = (u128::from(whole) << LOG2_FRACTION_BITS) | fraction;
    ((log2_q40 * LOG10_2_Q64) >> LOG2_FRACTION_BITS) as i128
}

/// Rounds a Q64 count of hundredths of a decibel to the nearest hundredth.
fn round_q64(hundredths_q64: i128) -> Decibels {
    let rounded = (hundredths_q64 + (1 << 63)) >> 64;
    Decibels(rounded.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn hundredths_of(value: f64) -> i32 {
        (value * 100.0).round() as i32
    }

    #[test]
    fn the_fixed_point_logarithm_matches_the_float_one() {
        let mut values: Vec<u64> = (1..=20_000).collect();
        let mut power = 1u64;
        while let Some(next) = power.checked_mul(3) {
            values.extend([next - 1, next, next + 1]);
            power = next;
        }
        values.extend([125_000, 868_100_000, 299_792_458, u64::MAX]);
        for value in values {
            assert_eq!(
                round_q64(1000 * log10_q64(value)).hundredths(),
                hundredths_of(10.0 * (value as f64).log10()),
                "10 log10({value})"
            );
        }
    }

    #[test]
    fn powers_of_ten_come_out_whole() {
        for exponent in 0..=19u32 {
            assert_eq!(
                round_q64(1000 * log10_q64(10u64.pow(exponent))),
                Decibels::from_db(10 * exponent as i32)
            );
        }
    }

    #[test]
    fn the_noise_floor_is_the_an1200_22_formula() {
        assert_eq!(noise_floor_dbm(125_000), Decibels::from_hundredths(-12_303));
        assert_eq!(noise_floor_dbm(500_000), Decibels::from_hundredths(-11_701));
        for bandwidth in [7_810u32, 10_420, 62_500, 125_000, 250_000, 500_000] {
            assert_eq!(
                noise_floor_dbm(bandwidth).hundredths(),
                hundredths_of(-174.0 + 10.0 * f64::from(bandwidth).log10()),
                "{bandwidth} Hz"
            );
        }
        assert_eq!(noise_floor_dbm(0), THERMAL_NOISE_DBM_PER_HZ);
    }

    #[test]
    fn the_demodulator_snr_is_the_sx1261_2_table() {
        let table = [
            (5, -250),
            (6, -500),
            (7, -750),
            (8, -1_000),
            (9, -1_250),
            (10, -1_500),
            (11, -1_750),
            (12, -2_000),
        ];
        for (spreading_factor, hundredths) in table {
            assert_eq!(
                demodulator_snr_db(spreading_factor),
                Decibels::from_hundredths(hundredths)
            );
        }
        assert_eq!(demodulator_snr_db(4), demodulator_snr_db(5));
        assert_eq!(demodulator_snr_db(13), demodulator_snr_db(12));
    }

    #[test]
    fn sensitivity_lands_near_the_sx1261_2_datasheet() {
        // SX1261/2 datasheet Rev 2.2, Table 3-8, RXS_LB, in dBm.
        let rows = [
            (10_420, 7, -134),
            (10_420, 12, -148),
            (125_000, 7, -124),
            (125_000, 12, -137),
            (250_000, 7, -121),
            (250_000, 12, -134),
            (500_000, 7, -117),
            (500_000, 12, -129),
        ];
        for (bandwidth, spreading_factor, datasheet) in rows {
            let link = LinkSettings::new(spreading_factor, bandwidth);
            let computed = sensitivity_dbm(link, RADIO_NOISE_FIGURE_DB);
            let gap = (computed - Decibels::from_db(datasheet)).hundredths().abs();
            let tolerance = if bandwidth == 125_000 || bandwidth == 250_000 {
                60
            } else {
                300
            };
            assert!(
                gap <= tolerance,
                "SF{spreading_factor} at {bandwidth} Hz is {computed} against {datasheet}"
            );
        }
        assert_eq!(
            sensitivity_dbm(LinkSettings::new(12, 125_000), RADIO_NOISE_FIGURE_DB),
            Decibels::from_hundredths(-13_703)
        );
    }

    #[test]
    fn sensitivity_lands_near_the_sx1276_datasheet() {
        // SX1276/77/78/79 datasheet Rev 7, RFS_L125_HF, RFS_L250_HF, and RFS_L500_HF,
        // SF7 to SF12, in dBm.
        let rows = [
            (125_000, [-123, -126, -129, -132, -133, -136]),
            (250_000, [-120, -123, -125, -128, -130, -133]),
            (500_000, [-116, -119, -122, -125, -128, -130]),
        ];
        for (bandwidth, figures) in rows {
            for (spreading_factor, datasheet) in (7..=12).zip(figures) {
                let link = LinkSettings::new(spreading_factor, bandwidth);
                let computed = sensitivity_dbm(link, RADIO_NOISE_FIGURE_DB);
                let gap = (computed - Decibels::from_db(datasheet)).hundredths().abs();
                assert!(
                    gap <= 300,
                    "SF{spreading_factor} at {bandwidth} Hz is {computed} against {datasheet}"
                );
            }
        }
    }

    #[test]
    fn a_gateway_hears_three_decibels_deeper() {
        let link = LinkSettings::new(12, 125_000);
        assert_eq!(
            sensitivity_dbm(link, GATEWAY_NOISE_FIGURE_DB),
            Decibels::from_hundredths(-14_003)
        );
    }

    #[test]
    fn free_space_loss_is_p525_equation_5() {
        let distances = [
            1u32,
            10,
            100,
            1_000,
            2_500,
            5_000,
            15_000,
            100_000,
            1_000_000,
            u32::MAX,
        ];
        let frequencies = [
            433_175_000u32,
            868_100_000,
            915_000_000,
            923_200_000,
            2_400_000_000,
        ];
        for distance in distances {
            for frequency in frequencies {
                let wavelength = 299_792_458.0 / f64::from(frequency);
                let exact = 20.0 * (4.0 * PI * f64::from(distance) / wavelength).log10();
                assert_eq!(
                    free_space_loss_db(distance, frequency).hundredths(),
                    hundredths_of(exact),
                    "{distance} m at {frequency} Hz"
                );
            }
        }
        assert_eq!(
            free_space_loss_db(5_000, 868_100_000),
            Decibels::from_hundredths(10_520)
        );
        assert_eq!(
            free_space_loss_db(0, 868_100_000),
            free_space_loss_db(1, 868_100_000)
        );
    }

    #[test]
    fn equation_6_rounds_its_constant_and_reads_low() {
        let paths = [
            (1_000u32, 868_100_000u32),
            (10_000, 915_000_000),
            (100_000, 433_175_000),
        ];
        for (distance, frequency) in paths {
            let equation_6 = 32.4
                + 20.0 * (f64::from(frequency) / 1e6).log10()
                + 20.0 * (f64::from(distance) / 1e3).log10();
            let gap =
                free_space_loss_db(distance, frequency).hundredths() - hundredths_of(equation_6);
            assert!(
                (4..=5).contains(&gap),
                "{distance} m at {frequency} Hz reads {gap} hundredths from equation (6)"
            );
        }
    }

    #[test]
    fn the_fresnel_radius_is_p526_equation_2() {
        let points = [
            (1u32, 1u32),
            (2_500, 2_500),
            (1_000, 9_000),
            (50_000, 50_000),
            (10, 4_000_000_000),
        ];
        for (near, far) in points {
            for frequency in [433_175_000u32, 868_100_000, 915_000_000, 2_483_500_000] {
                let wavelength = 299_792_458.0 / f64::from(frequency);
                let exact = (wavelength * f64::from(near) * f64::from(far)
                    / (f64::from(near) + f64::from(far)))
                .sqrt()
                    * 1000.0;
                assert_eq!(
                    i64::from(fresnel_radius_mm(near, far, frequency)),
                    exact.round() as i64,
                    "{near} m and {far} m at {frequency} Hz"
                );
            }
        }
        assert_eq!(fresnel_radius_mm(2_500, 2_500, 868_100_000), 20_777);
        assert_eq!(fresnel_radius_mm(0, 0, 868_100_000), 0);
        assert_eq!(fresnel_radius_mm(2_500, 2_500, 0), 0);
    }

    #[test]
    fn equation_3_rounds_its_constant_and_reads_wide() {
        let radius = fresnel_radius_mm(2_500, 2_500, 868_100_000);
        let equation_3 = 550.0 * (2.5 * 2.5 / (5.0 * 868.1f64)).sqrt() * 1000.0;
        let wide = equation_3 / f64::from(radius) - 1.0;
        assert!(
            (0.004..0.005).contains(&wide),
            "equation (3) reads {wide} wide"
        );
    }

    #[test]
    fn a_budget_adds_up_from_radio_to_radio() {
        let budget = LinkBudget {
            transmit_power_dbm: Decibels::from_db(14),
            transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
            transmit_cable_loss_db: Decibels::from_tenths(5),
            receive_antenna_gain_dbi: Decibels::from_db(3),
            receive_cable_loss_db: Decibels::from_db(1),
            noise_figure_db: RADIO_NOISE_FIGURE_DB,
        };
        let link = LinkSettings::new(12, 125_000);
        let path = Decibels::from_hundredths(10_520);
        assert_eq!(budget.eirp_dbm(), Decibels::from_hundredths(1_565));
        assert_eq!(budget.received_dbm(path), Decibels::from_hundredths(-8_755));
        assert_eq!(
            budget.sensitivity_dbm(link),
            Decibels::from_hundredths(-13_703)
        );
        assert_eq!(
            budget.max_path_loss_db(link),
            Decibels::from_hundredths(15_468)
        );
        assert_eq!(
            budget.margin_db(link, path),
            Decibels::from_hundredths(4_948)
        );
        assert_eq!(
            budget.max_transmit_power_dbm(Decibels::from_db(16)),
            Decibels::from_hundredths(1_435)
        );
    }

    #[test]
    fn the_default_budget_is_isotropic_with_a_radio_noise_figure() {
        let budget = LinkBudget::default();
        assert_eq!(budget.eirp_dbm(), Decibels::ZERO);
        assert_eq!(budget.noise_figure_db, RADIO_NOISE_FIGURE_DB);
    }

    #[cfg(feature = "eu868")]
    #[test]
    fn the_eu868_ceiling_leaves_less_power_for_a_higher_gain_antenna() {
        let plan = crate::region::Region::Eu868.plan();
        let ceiling = Decibels::from_db(i32::from(plan.max_eirp_dbm(868_100_000)));
        let whip = LinkBudget {
            transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
            ..LinkBudget::default()
        };
        let collinear = LinkBudget {
            transmit_antenna_gain_dbi: Decibels::from_db(6),
            ..LinkBudget::default()
        };
        assert_eq!(
            whip.max_transmit_power_dbm(ceiling),
            Decibels::from_hundredths(1_385)
        );
        assert_eq!(
            collinear.max_transmit_power_dbm(ceiling),
            Decibels::from_db(10)
        );
    }

    #[test]
    fn one_watt_and_a_quarter_watt_in_dbm() {
        assert_eq!(round_q64(1000 * log10_q64(1_000)), ONE_WATT_DBM);
        assert_eq!(round_q64(1000 * log10_q64(250)), QUARTER_WATT_DBM);
    }

    #[test]
    fn fcc_15_247_takes_off_what_the_gain_exceeds_6_dbi() {
        let digital = Fcc15247::DigitalModulation;
        assert_eq!(
            digital.max_conducted_dbm(Decibels::from_hundredths(215)),
            Some(Decibels::from_db(30))
        );
        assert_eq!(
            digital.max_conducted_dbm(Decibels::from_db(6)),
            Some(Decibels::from_db(30))
        );
        assert_eq!(
            digital.max_conducted_dbm(Decibels::from_db(9)),
            Some(Decibels::from_db(27))
        );
        let hopping = |channels| Fcc15247::FrequencyHopping { channels };
        assert_eq!(
            hopping(64).max_conducted_dbm(Decibels::from_db(9)),
            Some(Decibels::from_db(27))
        );
        assert_eq!(
            hopping(50).max_conducted_dbm(Decibels::ZERO),
            Some(Decibels::from_db(30))
        );
        assert_eq!(
            hopping(49).max_conducted_dbm(Decibels::ZERO),
            Some(Decibels::from_hundredths(2_398))
        );
        assert_eq!(
            hopping(25).max_conducted_dbm(Decibels::from_db(8)),
            Some(Decibels::from_hundredths(2_198))
        );
        assert_eq!(hopping(24).max_conducted_dbm(Decibels::ZERO), None);
    }

    #[test]
    fn decibels_print_to_the_hundredth() {
        assert_eq!(Decibels::from_hundredths(-13_703).to_string(), "-137.03");
        assert_eq!(Decibels::from_hundredths(5).to_string(), "0.05");
        assert_eq!(Decibels::from_hundredths(-50).to_string(), "-0.50");
        assert_eq!(Decibels::ZERO.to_string(), "0.00");
        assert_eq!(Decibels::from_tenths(1_052).to_string(), "105.20");
        assert_eq!(
            Decibels::from_hundredths(i32::MIN).to_string(),
            "-21474836.48"
        );
        assert_eq!(
            format!("{:>9}", Decibels::from_hundredths(-13_703)),
            "  -137.03"
        );
    }

    #[test]
    fn decibels_round_halves_away_from_zero() {
        assert_eq!(Decibels::from_hundredths(13_749).round_db(), 137);
        assert_eq!(Decibels::from_hundredths(13_750).round_db(), 138);
        assert_eq!(Decibels::from_hundredths(-13_749).round_db(), -137);
        assert_eq!(Decibels::from_hundredths(-13_750).round_db(), -138);
    }

    #[test]
    fn decibels_floor_toward_negative_infinity() {
        assert_eq!(Decibels::from_hundredths(1_435).floor_db(), 14);
        assert_eq!(Decibels::from_hundredths(1_400).floor_db(), 14);
        assert_eq!(Decibels::from_hundredths(-1).floor_db(), -1);
        assert_eq!(Decibels::from_hundredths(-100).floor_db(), -1);
        assert_eq!(Decibels::from_hundredths(-101).floor_db(), -2);
    }

    #[test]
    fn decibel_arithmetic_saturates() {
        assert_eq!(Decibels::from_db(i32::MAX).hundredths(), i32::MAX);
        assert_eq!(
            (Decibels::from_hundredths(i32::MAX) + Decibels::from_db(1)).hundredths(),
            i32::MAX
        );
        assert_eq!(
            (-Decibels::from_hundredths(i32::MIN)).hundredths(),
            i32::MAX
        );
        assert_eq!(Decibels::from_tenths(21), Decibels::from_hundredths(210));
    }
}
