//! The settings an SX126x is configured with, and the values its registers take.
//!
//! Every value here comes from the SX1261/2 datasheet (Rev 2.2): the frequency word of
//! section 13.4.1, the timeout steps of 13.1.4, the image calibration codes of 9.2.1, the
//! LoRa modulation and packet parameters of 13.4.5 and 13.4.6, the power amplifier
//! settings of 13.1.14 and 13.4.4, the sync words of Table 12-1, and the register values
//! the chapter 15 workarounds write. The LoRa settings are built from a
//! [`LinkSettings`], so a radio sends exactly the frames `pamoja-lora` computes the
//! airtime of.

use pamoja_lora::budget::{Decibels, LinkBudget};
use pamoja_lora::LinkSettings;

/// The crystal frequency the synthesizer divides, in hertz.
pub const XTAL_HZ: u32 = 32_000_000;

/// Returns the word SetRfFrequency takes for a frequency.
///
/// The datasheet defines the frequency as the word times the crystal frequency over
/// 2^25, so one step of the word is 0.95 Hz. The word is rounded to the nearest step.
///
/// # Arguments
///
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The 32-bit frequency word.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx126x::config::frequency_word;
///
/// assert_eq!(frequency_word(868_100_000), 0x3641_999A);
/// assert_eq!(frequency_word(915_000_000), 0x3930_0000);
/// ```
pub const fn frequency_word(frequency_hz: u32) -> u32 {
    (((frequency_hz as u64) << 25) + (XTAL_HZ as u64 / 2)).div_euclid(XTAL_HZ as u64) as u32
}

/// Returns the frequency a SetRfFrequency word selects.
///
/// # Arguments
///
/// * `word` - the 32-bit frequency word.
///
/// # Returns
///
/// The carrier frequency in hertz, rounded to the nearest hertz.
pub const fn frequency_from_word(word: u32) -> u32 {
    ((word as u64 * XTAL_HZ as u64 + (1 << 24)) >> 25) as u32
}

/// The timeout word that disables a transmit or receive timeout.
pub const NO_TIMEOUT: u32 = 0;

/// The receive timeout word that keeps the radio listening until told otherwise.
pub const RX_CONTINUOUS: u32 = 0xFF_FFFF;

/// Returns the 24-bit timeout word for a duration.
///
/// SetTx, SetRx, and SetDIO3AsTCXOCtrl count time in steps of 15.625 us. A nonzero
/// duration never becomes a zero word, which would disable the timeout, and a duration
/// past the longest the chip counts, about 262 seconds, stops one step short of
/// [`RX_CONTINUOUS`].
///
/// # Arguments
///
/// * `micros` - the duration in microseconds.
///
/// # Returns
///
/// The number of steps, from 0 to 0xFFFFFE.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx126x::config::timeout_steps;
///
/// assert_eq!(timeout_steps(1_000_000), 64_000);
/// assert_eq!(timeout_steps(0), 0);
/// ```
pub const fn timeout_steps(micros: u64) -> u32 {
    if micros == 0 {
        return NO_TIMEOUT;
    }
    let steps = micros.saturating_mul(64) / 1000;
    if steps == 0 {
        1
    } else if steps >= RX_CONTINUOUS as u64 {
        RX_CONTINUOUS - 1
    } else {
        steps as u32
    }
}

/// Returns the two CalibrateImage codes that cover a band.
///
/// Section 9.2.1 calibrates image rejection between two codes in steps of 4 MHz, taking
/// the floor of the lower edge and the ceiling of the upper one, so the calibrated range
/// always covers the band.
///
/// # Arguments
///
/// * `low_hz` - the lower edge of the band in hertz.
/// * `high_hz` - the upper edge of the band in hertz.
///
/// # Returns
///
/// `freq1` and `freq2`, the parameters of CalibrateImage.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx126x::config::image_calibration;
///
/// assert_eq!(image_calibration(863_000_000, 870_000_000), [0xD7, 0xDA]);
/// assert_eq!(image_calibration(902_000_000, 928_000_000), [0xE1, 0xE8]);
/// ```
pub const fn image_calibration(low_hz: u32, high_hz: u32) -> [u8; 2] {
    const STEP_HZ: u32 = 4_000_000;
    let low = low_hz / STEP_HZ;
    let high = high_hz.div_ceil(STEP_HZ);
    [clamp_byte(low), clamp_byte(high)]
}

const fn clamp_byte(value: u32) -> u8 {
    if value > 0xFF {
        0xFF
    } else {
        value as u8
    }
}

/// The modem a SetPacketType command selects, from Table 13-38.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PacketType {
    /// GFSK (0x00).
    Gfsk,
    /// LoRa (0x01).
    Lora,
    /// Long Range FHSS (0x03).
    LrFhss,
}

impl PacketType {
    /// Returns the parameter byte.
    ///
    /// # Returns
    ///
    /// The PacketType value.
    pub const fn code(self) -> u8 {
        match self {
            PacketType::Gfsk => 0x00,
            PacketType::Lora => 0x01,
            PacketType::LrFhss => 0x03,
        }
    }
}

/// The standby mode a SetStandby command selects, from Table 13-4.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StandbyMode {
    /// Running on the 13 MHz RC oscillator (0).
    Rc,
    /// Running on the 32 MHz crystal (1).
    Xosc,
}

impl StandbyMode {
    /// Returns the parameter byte.
    ///
    /// # Returns
    ///
    /// The StdbyConfig value.
    pub const fn code(self) -> u8 {
        match self {
            StandbyMode::Rc => 0,
            StandbyMode::Xosc => 1,
        }
    }
}

/// How the chip regulates its supply, from Table 13-16.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegulatorMode {
    /// Only the LDO, for every mode (0).
    Ldo,
    /// The DC-DC converter and the LDO, for STBY_XOSC, FS, RX, and TX (1).
    DcDc,
}

impl RegulatorMode {
    /// Returns the parameter byte.
    ///
    /// # Returns
    ///
    /// The regModeParam value.
    pub const fn code(self) -> u8 {
        match self {
            RegulatorMode::Ldo => 0,
            RegulatorMode::DcDc => 1,
        }
    }
}

/// The mode the chip returns to after a transmission or reception, from Table 13-23.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FallbackMode {
    /// Frequency synthesis (0x40).
    Fs,
    /// Standby on the crystal (0x30).
    StandbyXosc,
    /// Standby on the RC oscillator, the default (0x20).
    StandbyRc,
}

impl FallbackMode {
    /// Returns the parameter byte.
    ///
    /// # Returns
    ///
    /// The fallbackMode value.
    pub const fn code(self) -> u8 {
        match self {
            FallbackMode::Fs => 0x40,
            FallbackMode::StandbyXosc => 0x30,
            FallbackMode::StandbyRc => 0x20,
        }
    }
}

/// The voltage DIO3 supplies a TCXO with, from Table 13-35.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TcxoVoltage {
    /// 1.6 V (0x00).
    V1_6,
    /// 1.7 V (0x01).
    V1_7,
    /// 1.8 V (0x02).
    V1_8,
    /// 2.2 V (0x03).
    V2_2,
    /// 2.4 V (0x04).
    V2_4,
    /// 2.7 V (0x05).
    V2_7,
    /// 3.0 V (0x06).
    V3_0,
    /// 3.3 V (0x07).
    V3_3,
}

impl TcxoVoltage {
    /// Returns the parameter byte.
    ///
    /// # Returns
    ///
    /// The tcxoVoltage value.
    pub const fn code(self) -> u8 {
        match self {
            TcxoVoltage::V1_6 => 0x00,
            TcxoVoltage::V1_7 => 0x01,
            TcxoVoltage::V1_8 => 0x02,
            TcxoVoltage::V2_2 => 0x03,
            TcxoVoltage::V2_4 => 0x04,
            TcxoVoltage::V2_7 => 0x05,
            TcxoVoltage::V3_0 => 0x06,
            TcxoVoltage::V3_3 => 0x07,
        }
    }
}

/// How long the power amplifier takes to ramp up, from Table 13-41.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RampTime {
    /// 10 us (0x00).
    Us10,
    /// 20 us (0x01).
    Us20,
    /// 40 us (0x02).
    Us40,
    /// 80 us (0x03).
    Us80,
    /// 200 us (0x04).
    Us200,
    /// 800 us (0x05).
    Us800,
    /// 1700 us (0x06).
    Us1700,
    /// 3400 us (0x07).
    Us3400,
}

impl RampTime {
    /// Returns the parameter byte.
    ///
    /// # Returns
    ///
    /// The RampTime value.
    pub const fn code(self) -> u8 {
        match self {
            RampTime::Us10 => 0x00,
            RampTime::Us20 => 0x01,
            RampTime::Us40 => 0x02,
            RampTime::Us80 => 0x03,
            RampTime::Us200 => 0x04,
            RampTime::Us800 => 0x05,
            RampTime::Us1700 => 0x06,
            RampTime::Us3400 => 0x07,
        }
    }

    /// Returns the ramp time in microseconds.
    ///
    /// # Returns
    ///
    /// The duration Table 13-41 gives.
    pub const fn micros(self) -> u32 {
        match self {
            RampTime::Us10 => 10,
            RampTime::Us20 => 20,
            RampTime::Us40 => 40,
            RampTime::Us80 => 80,
            RampTime::Us200 => 200,
            RampTime::Us800 => 800,
            RampTime::Us1700 => 1_700,
            RampTime::Us3400 => 3_400,
        }
    }

    /// Returns the shortest ramp time that lasts at least a duration.
    ///
    /// # Arguments
    ///
    /// * `micros` - the least ramp time wanted, in microseconds; past 3400 us, the longest
    ///   ramp the chip offers.
    ///
    /// # Returns
    ///
    /// The ramp time.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx126x::config::RampTime;
    ///
    /// assert_eq!(RampTime::at_least(40), RampTime::Us40);
    /// assert_eq!(RampTime::at_least(100), RampTime::Us200);
    /// assert_eq!(RampTime::at_least(5_000).micros(), 3_400);
    /// ```
    pub const fn at_least(micros: u32) -> RampTime {
        if micros <= 10 {
            RampTime::Us10
        } else if micros <= 20 {
            RampTime::Us20
        } else if micros <= 40 {
            RampTime::Us40
        } else if micros <= 80 {
            RampTime::Us80
        } else if micros <= 200 {
            RampTime::Us200
        } else if micros <= 800 {
            RampTime::Us800
        } else if micros <= 1_700 {
            RampTime::Us1700
        } else {
            RampTime::Us3400
        }
    }
}

/// A LoRa signal bandwidth, from Table 13-48.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoraBandwidth {
    /// 7.81 kHz (0x00).
    Khz7_8,
    /// 10.42 kHz (0x08).
    Khz10_4,
    /// 15.63 kHz (0x01).
    Khz15_6,
    /// 20.83 kHz (0x09).
    Khz20_8,
    /// 31.25 kHz (0x02).
    Khz31_25,
    /// 41.67 kHz (0x0A).
    Khz41_7,
    /// 62.5 kHz (0x03).
    Khz62_5,
    /// 125 kHz (0x04).
    Khz125,
    /// 250 kHz (0x05).
    Khz250,
    /// 500 kHz (0x06).
    Khz500,
}

impl LoraBandwidth {
    const ALL: [LoraBandwidth; 10] = [
        LoraBandwidth::Khz7_8,
        LoraBandwidth::Khz10_4,
        LoraBandwidth::Khz15_6,
        LoraBandwidth::Khz20_8,
        LoraBandwidth::Khz31_25,
        LoraBandwidth::Khz41_7,
        LoraBandwidth::Khz62_5,
        LoraBandwidth::Khz125,
        LoraBandwidth::Khz250,
        LoraBandwidth::Khz500,
    ];

    /// Returns the parameter byte.
    ///
    /// # Returns
    ///
    /// The BW value of SetModulationParams.
    pub const fn code(self) -> u8 {
        match self {
            LoraBandwidth::Khz7_8 => 0x00,
            LoraBandwidth::Khz10_4 => 0x08,
            LoraBandwidth::Khz15_6 => 0x01,
            LoraBandwidth::Khz20_8 => 0x09,
            LoraBandwidth::Khz31_25 => 0x02,
            LoraBandwidth::Khz41_7 => 0x0A,
            LoraBandwidth::Khz62_5 => 0x03,
            LoraBandwidth::Khz125 => 0x04,
            LoraBandwidth::Khz250 => 0x05,
            LoraBandwidth::Khz500 => 0x06,
        }
    }

    /// Returns the bandwidth in hertz, rounded to the nearest hertz.
    ///
    /// # Returns
    ///
    /// The bandwidth: 500 kHz halved, or 125 kHz divided by 3 and halved, as many times
    /// as the setting takes.
    pub const fn hz(self) -> u32 {
        match self {
            LoraBandwidth::Khz7_8 => 7_813,
            LoraBandwidth::Khz10_4 => 10_417,
            LoraBandwidth::Khz15_6 => 15_625,
            LoraBandwidth::Khz20_8 => 20_833,
            LoraBandwidth::Khz31_25 => 31_250,
            LoraBandwidth::Khz41_7 => 41_667,
            LoraBandwidth::Khz62_5 => 62_500,
            LoraBandwidth::Khz125 => 125_000,
            LoraBandwidth::Khz250 => 250_000,
            LoraBandwidth::Khz500 => 500_000,
        }
    }

    /// Finds the setting for a bandwidth in hertz.
    ///
    /// # Arguments
    ///
    /// * `hz` - the bandwidth in hertz, within 1% of one the chip supports, so both
    ///   `10_417` and the datasheet's rounded `10_420` select 10.42 kHz.
    ///
    /// # Returns
    ///
    /// The setting, or `None` for a bandwidth the SX126x does not offer.
    pub fn from_hz(hz: u32) -> Option<LoraBandwidth> {
        LoraBandwidth::ALL.into_iter().find(|bandwidth| {
            let nominal = u64::from(bandwidth.hz());
            u64::from(hz).abs_diff(nominal) * 100 <= nominal
        })
    }
}

/// A LoRa coding rate, from Table 13-49.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodingRate {
    /// 4/5 (0x01).
    Cr4_5,
    /// 4/6 (0x02).
    Cr4_6,
    /// 4/7 (0x03).
    Cr4_7,
    /// 4/8 (0x04).
    Cr4_8,
    /// 4/5 with the long interleaver (0x05).
    Cr4_5LongInterleaver,
    /// 4/6 with the long interleaver (0x06).
    Cr4_6LongInterleaver,
    /// 4/8 with the long interleaver (0x07).
    Cr4_8LongInterleaver,
}

impl CodingRate {
    /// Returns the parameter byte.
    ///
    /// # Returns
    ///
    /// The CR value of SetModulationParams.
    pub const fn code(self) -> u8 {
        match self {
            CodingRate::Cr4_5 => 0x01,
            CodingRate::Cr4_6 => 0x02,
            CodingRate::Cr4_7 => 0x03,
            CodingRate::Cr4_8 => 0x04,
            CodingRate::Cr4_5LongInterleaver => 0x05,
            CodingRate::Cr4_6LongInterleaver => 0x06,
            CodingRate::Cr4_8LongInterleaver => 0x07,
        }
    }

    /// Returns the standard-interleaver coding rate for a denominator.
    ///
    /// The airtime `pamoja-lora` computes assumes the standard interleaver, so a link's
    /// coding rate always maps to one of the first four settings.
    ///
    /// # Arguments
    ///
    /// * `denominator` - the coding-rate denominator, clamped to 5 to 8.
    ///
    /// # Returns
    ///
    /// The coding rate.
    pub const fn from_denominator(denominator: u8) -> CodingRate {
        match denominator {
            0..=5 => CodingRate::Cr4_5,
            6 => CodingRate::Cr4_6,
            7 => CodingRate::Cr4_7,
            _ => CodingRate::Cr4_8,
        }
    }
}

/// The LoRa parameters of a SetModulationParams command, from Tables 13-47 to 13-50.
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
/// use pamoja_radios::sx126x::config::LoraModulation;
///
/// // SF12 at 125 kHz needs low data rate optimization.
/// let modulation = LoraModulation::from_link(&LinkSettings::new(12, 125_000)).unwrap();
/// assert_eq!(modulation.to_params(), [0x0C, 0x04, 0x01, 0x01]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoraModulation {
    /// The spreading factor, 5 to 12.
    pub spreading_factor: u8,
    /// The signal bandwidth.
    pub bandwidth: LoraBandwidth,
    /// The coding rate.
    pub coding_rate: CodingRate,
    /// Whether low data rate optimization is on.
    pub low_data_rate_optimization: bool,
}

impl LoraModulation {
    /// Builds the modulation parameters of a link.
    ///
    /// # Arguments
    ///
    /// * `link` - the link settings, whose spreading factor, bandwidth, coding rate, and
    ///   low data rate optimization the radio must match.
    ///
    /// # Returns
    ///
    /// The parameters, or `None` when the link's bandwidth is not one the SX126x
    /// offers.
    pub fn from_link(link: &LinkSettings) -> Option<LoraModulation> {
        Some(LoraModulation {
            spreading_factor: link.spreading_factor(),
            bandwidth: LoraBandwidth::from_hz(link.bandwidth_hz())?,
            coding_rate: CodingRate::from_denominator(link.coding_rate_denominator()),
            low_data_rate_optimization: link.low_data_rate_optimization(),
        })
    }

    /// Returns the four parameter bytes: SF, BW, CR, and LowDataRateOptimize.
    ///
    /// # Returns
    ///
    /// ModParam1 to ModParam4.
    pub const fn to_params(&self) -> [u8; 4] {
        [
            self.spreading_factor,
            self.bandwidth.code(),
            self.coding_rate.code(),
            self.low_data_rate_optimization as u8,
        ]
    }
}

/// The LoRa parameters of a SetPacketParams command, from Tables 13-66 to 13-70.
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
/// use pamoja_radios::sx126x::config::LoraPacket;
///
/// // An eight-symbol preamble, an explicit header, a 20-byte payload, CRC on, standard IQ.
/// let packet = LoraPacket::from_link(&LinkSettings::new(7, 125_000), 20, false);
/// assert_eq!(packet.to_params(), [0x00, 0x08, 0x00, 0x14, 0x01, 0x00]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoraPacket {
    /// The preamble length in symbols.
    pub preamble_symbols: u16,
    /// Whether the frame carries an explicit header; implicit when `false`.
    pub explicit_header: bool,
    /// The payload length to send, or the most a receiver accepts.
    pub payload_len: u8,
    /// Whether the frame carries a CRC.
    pub crc: bool,
    /// Whether the IQ polarity is inverted, as LoRaWAN downlinks use.
    pub invert_iq: bool,
}

impl LoraPacket {
    /// Builds the packet parameters of a link for a payload.
    ///
    /// # Arguments
    ///
    /// * `link` - the link settings, whose preamble, header, and CRC the frame uses.
    /// * `payload_len` - the payload length to send, or the most to accept.
    /// * `invert_iq` - `true` for inverted IQ polarity.
    ///
    /// # Returns
    ///
    /// The parameters.
    pub fn from_link(link: &LinkSettings, payload_len: u8, invert_iq: bool) -> LoraPacket {
        LoraPacket {
            preamble_symbols: link.preamble_symbols(),
            explicit_header: link.explicit_header(),
            payload_len,
            crc: link.crc(),
            invert_iq,
        }
    }

    /// Returns the six parameter bytes: the preamble length, most significant byte first,
    /// then the header type, the payload length, the CRC type, and the IQ setup.
    ///
    /// # Returns
    ///
    /// PacketParam1 to PacketParam6.
    pub const fn to_params(&self) -> [u8; 6] {
        let preamble = self.preamble_symbols.to_be_bytes();
        [
            preamble[0],
            preamble[1],
            !self.explicit_header as u8,
            self.payload_len,
            self.crc as u8,
            self.invert_iq as u8,
        ]
    }
}

/// Which power amplifier a chip has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PowerAmplifier {
    /// The low power amplifier of the SX1261, up to +15 dBm.
    LowPower,
    /// The high power amplifier of the SX1262 and the LLCC68, up to +22 dBm.
    HighPower,
}

impl PowerAmplifier {
    /// Returns the output power range SetTxParams accepts, from section 13.4.4.
    ///
    /// # Returns
    ///
    /// The lowest and highest settings in dBm: -17 to +14 for the low power amplifier
    /// and -9 to +22 for the high power one.
    pub const fn setting_range_dbm(self) -> (i8, i8) {
        match self {
            PowerAmplifier::LowPower => (-17, 14),
            PowerAmplifier::HighPower => (-9, 22),
        }
    }
}

/// The parameters of a SetPaConfig command, from Table 13-20.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaConfig {
    /// The duty cycle, or conduction angle, of the amplifier.
    pub duty_cycle: u8,
    /// The size of the SX1262's amplifier, 0x00 to 0x07; no effect on the SX1261.
    pub hp_max: u8,
    /// The device: 0 for the SX1262, 1 for the SX1261.
    pub device: u8,
    /// Reserved, always 0x01.
    pub lut: u8,
}

impl PaConfig {
    /// The SX1262 at +22 dBm, from Table 13-21.
    pub const SX1262_22_DBM: PaConfig = PaConfig::new(0x04, 0x07, 0x00);
    /// The SX1262 at +20 dBm, from Table 13-21.
    pub const SX1262_20_DBM: PaConfig = PaConfig::new(0x03, 0x05, 0x00);
    /// The SX1262 at +17 dBm, from Table 13-21.
    pub const SX1262_17_DBM: PaConfig = PaConfig::new(0x02, 0x03, 0x00);
    /// The SX1262 at +14 dBm, from Table 13-21.
    pub const SX1262_14_DBM: PaConfig = PaConfig::new(0x02, 0x02, 0x00);
    /// The SX1261 at +15 dBm, from Table 13-21.
    pub const SX1261_15_DBM: PaConfig = PaConfig::new(0x06, 0x00, 0x01);
    /// The SX1261 at +14 dBm, from Table 13-21.
    pub const SX1261_14_DBM: PaConfig = PaConfig::new(0x04, 0x00, 0x01);
    /// The SX1261 at +10 dBm, from Table 13-21.
    pub const SX1261_10_DBM: PaConfig = PaConfig::new(0x01, 0x00, 0x01);

    const fn new(duty_cycle: u8, hp_max: u8, device: u8) -> PaConfig {
        PaConfig {
            duty_cycle,
            hp_max,
            device,
            lut: 0x01,
        }
    }

    /// Returns the four parameter bytes.
    ///
    /// # Returns
    ///
    /// paDutyCycle, hpMax, deviceSel, and paLut.
    pub const fn to_params(&self) -> [u8; 4] {
        [self.duty_cycle, self.hp_max, self.device, self.lut]
    }
}

/// The amplifier configuration and power setting that produce an output power.
///
/// # Examples
///
/// ```
/// use pamoja_lora::budget::{Decibels, LinkBudget};
/// use pamoja_radios::sx126x::config::{PaConfig, PowerAmplifier, TxPower};
///
/// // A 2.15 dBi whip on half a decibel of pigtail under a 16 dBm EIRP ceiling.
/// let whip = LinkBudget {
///     transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
///     transmit_cable_loss_db: Decibels::from_tenths(5),
///     ..LinkBudget::default()
/// };
/// let power = TxPower::under_ceiling(PowerAmplifier::HighPower, &whip, Decibels::from_db(16));
/// assert_eq!(power.setting_dbm, 14);
/// assert_eq!(power.pa, PaConfig::SX1262_22_DBM);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TxPower {
    /// The SetPaConfig parameters.
    pub pa: PaConfig,
    /// The power byte of SetTxParams, in dBm.
    pub setting_dbm: i8,
}

impl TxPower {
    /// Chooses the settings for an output power.
    ///
    /// The high power amplifier keeps its +22 dBm configuration and takes the power in
    /// SetTxParams, clamped to -9 to +22 dBm. The low power amplifier uses its +14 dBm
    /// configuration with the power clamped to -17 to +14 dBm, and its +15 dBm
    /// configuration, which Table 13-21 drives with a setting of +14 dBm, for anything
    /// above.
    ///
    /// # Arguments
    ///
    /// * `amplifier` - the chip's power amplifier.
    /// * `output_dbm` - the output power wanted at the antenna port.
    ///
    /// # Returns
    ///
    /// The configuration and the setting.
    pub const fn for_output(amplifier: PowerAmplifier, output_dbm: i8) -> TxPower {
        match amplifier {
            PowerAmplifier::HighPower => TxPower {
                pa: PaConfig::SX1262_22_DBM,
                setting_dbm: clamp_power(output_dbm, -9, 22),
            },
            PowerAmplifier::LowPower if output_dbm >= 15 => TxPower {
                pa: PaConfig::SX1261_15_DBM,
                setting_dbm: 14,
            },
            PowerAmplifier::LowPower => TxPower {
                pa: PaConfig::SX1261_14_DBM,
                setting_dbm: clamp_power(output_dbm, -17, 14),
            },
        }
    }

    /// Chooses the settings that keep a link's EIRP at or under a ceiling.
    ///
    /// # Arguments
    ///
    /// * `amplifier` - the chip's power amplifier.
    /// * `budget` - the link budget, whose transmitting antenna and cable apply.
    /// * `eirp_ceiling_dbm` - the EIRP limit, such as a channel plan's ceiling for the
    ///   frequency in use.
    ///
    /// # Returns
    ///
    /// The configuration and the setting, rounded down to whole decibels so the EIRP
    /// stays under the ceiling.
    pub fn under_ceiling(
        amplifier: PowerAmplifier,
        budget: &LinkBudget,
        eirp_ceiling_dbm: Decibels,
    ) -> TxPower {
        let most = budget.max_transmit_power_dbm(eirp_ceiling_dbm).floor_db();
        let output = most.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
        TxPower::for_output(amplifier, output)
    }
}

const fn clamp_power(dbm: i8, low: i8, high: i8) -> i8 {
    if dbm < low {
        low
    } else if dbm > high {
        high
    } else {
        dbm
    }
}

/// A LoRa sync word, written to the two sync word registers at 0x0740, from Table 12-1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyncWord {
    /// 0x3444, for a public network such as LoRaWAN.
    Public,
    /// 0x1424, for a private network, and the chip's reset value.
    Private,
    /// Another two-byte value.
    Custom(u16),
}

impl SyncWord {
    /// Returns the two register bytes, most significant first.
    ///
    /// # Returns
    ///
    /// The values of the sync word registers at 0x0740 and 0x0741.
    pub const fn to_bytes(self) -> [u8; 2] {
        match self {
            SyncWord::Public => [0x34, 0x44],
            SyncWord::Private => [0x14, 0x24],
            SyncWord::Custom(word) => word.to_be_bytes(),
        }
    }
}

/// The register addresses the driver reads and writes, from Table 12-1.
pub mod register {
    /// The most significant byte of the LoRa sync word; the least follows at 0x0741.
    pub const LORA_SYNC_WORD: u16 = 0x0740;
    /// The IQ polarity setup, bit 2 of which the inverted IQ workaround sets.
    pub const IQ_POLARITY: u16 = 0x0736;
    /// The TX modulation register, bit 2 of which the 500 kHz workaround sets.
    pub const TX_MODULATION: u16 = 0x0889;
    /// The receive gain.
    pub const RX_GAIN: u16 = 0x08AC;
    /// The PA clamping configuration the antenna mismatch workaround raises.
    pub const TX_CLAMP_CONFIG: u16 = 0x08D8;
    /// The over current protection level.
    pub const OCP_CONFIGURATION: u16 = 0x08E7;
    /// The RTC control register the implicit header timeout workaround stops.
    pub const RTC_CONTROL: u16 = 0x0902;
    /// The trimming capacitor on the XTA pin.
    pub const XTA_TRIM: u16 = 0x0911;
    /// The trimming capacitor on the XTB pin.
    pub const XTB_TRIM: u16 = 0x0912;
    /// The event mask the implicit header timeout workaround clears.
    pub const EVENT_MASK: u16 = 0x0944;
}

/// The receive gain register value for power saving, the default, from Table 9-3.
pub const RX_GAIN_POWER_SAVING: u8 = 0x94;

/// The receive gain register value for boosted sensitivity, from Table 9-3.
pub const RX_GAIN_BOOSTED: u8 = 0x96;

/// The RTC control value that stops the timer, from section 15.3.
pub const RTC_STOP: u8 = 0x00;

/// Returns the TX modulation register value for a transmission, from section 15.1.
///
/// Bit 2 is cleared for a 500 kHz LoRa bandwidth and set for every other bandwidth.
///
/// # Arguments
///
/// * `current` - the register value read back from the chip.
/// * `bandwidth` - the LoRa bandwidth about to be used.
///
/// # Returns
///
/// The value to write.
pub const fn tx_modulation(current: u8, bandwidth: LoraBandwidth) -> u8 {
    match bandwidth {
        LoraBandwidth::Khz500 => current & !0x04,
        _ => current | 0x04,
    }
}

/// Returns the TX clamp register value that resists antenna mismatch, from section 15.2.
///
/// Bits 4 to 1 are set, which the datasheet asks of the SX1262 after a power on reset or
/// a cold wake.
///
/// # Arguments
///
/// * `current` - the register value read back from the chip.
///
/// # Returns
///
/// The value to write.
pub const fn tx_clamp(current: u8) -> u8 {
    current | 0x1E
}

/// Returns the IQ polarity register value for a polarity, from section 15.4.
///
/// Bit 2 is cleared for inverted IQ and set for standard IQ.
///
/// # Arguments
///
/// * `current` - the register value read back from the chip.
/// * `invert_iq` - `true` for inverted IQ polarity.
///
/// # Returns
///
/// The value to write.
pub const fn iq_polarity(current: u8, invert_iq: bool) -> u8 {
    if invert_iq {
        current & !0x04
    } else {
        current | 0x04
    }
}

/// Returns the event mask value that clears a pending RTC timeout, from section 15.3.
///
/// The datasheet names the register; the bit, bit 1, is the one Semtech's reference
/// `sx126x_driver` sets in `sx126x_stop_rtc`.
///
/// # Arguments
///
/// * `current` - the register value read back from the chip.
///
/// # Returns
///
/// The value to write.
pub const fn event_clear(current: u8) -> u8 {
    current | 0x02
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frequency_word_matches_the_reference_driver() {
        // Semtech's sx126x_convert_freq_in_hz_to_pll_step, evaluated for these frequencies.
        let words = [
            (433_175_000, 0x1B12_CCCD),
            (868_100_000, 0x3641_999A),
            (868_300_000, 0x3644_CCCD),
            (868_500_000, 0x3648_0000),
            (869_525_000, 0x3658_6666),
            (902_300_000, 0x3864_CCCD),
            (915_000_000, 0x3930_0000),
            (923_300_000, 0x39B4_CCCD),
            (150_000_000, 0x0960_0000),
            (960_000_000, 0x3C00_0000),
        ];
        for (hz, word) in words {
            assert_eq!(frequency_word(hz), word, "{hz} Hz");
            assert!(frequency_from_word(word).abs_diff(hz) <= 1, "{hz} Hz back");
        }
    }

    #[test]
    fn the_timeout_counts_in_steps_of_15625_nanoseconds() {
        assert_eq!(timeout_steps(15_625), 1_000);
        assert_eq!(timeout_steps(3_000_000), 192_000);
        assert_eq!(timeout_steps(1), 1);
        assert_eq!(timeout_steps(u64::MAX), RX_CONTINUOUS - 1);
    }

    #[test]
    fn the_image_calibration_takes_the_floor_and_the_ceiling() {
        assert_eq!(image_calibration(430_000_000, 440_000_000), [0x6B, 0x6E]);
        assert_eq!(image_calibration(470_000_000, 510_000_000), [0x75, 0x80]);
        assert_eq!(image_calibration(779_000_000, 787_000_000), [0xC2, 0xC5]);
        assert_eq!(image_calibration(863_000_000, 870_000_000), [0xD7, 0xDA]);
        assert_eq!(image_calibration(902_000_000, 928_000_000), [0xE1, 0xE8]);
    }

    #[test]
    fn every_bandwidth_round_trips_through_its_frequency() {
        for bandwidth in LoraBandwidth::ALL {
            assert_eq!(LoraBandwidth::from_hz(bandwidth.hz()), Some(bandwidth));
        }
        assert_eq!(LoraBandwidth::from_hz(10_420), Some(LoraBandwidth::Khz10_4));
        assert_eq!(LoraBandwidth::from_hz(7_810), Some(LoraBandwidth::Khz7_8));
        assert_eq!(LoraBandwidth::from_hz(200_000), None);
    }

    #[test]
    fn the_codes_follow_the_datasheet_tables() {
        let bandwidths = [
            (LoraBandwidth::Khz7_8, 0x00),
            (LoraBandwidth::Khz10_4, 0x08),
            (LoraBandwidth::Khz15_6, 0x01),
            (LoraBandwidth::Khz20_8, 0x09),
            (LoraBandwidth::Khz31_25, 0x02),
            (LoraBandwidth::Khz41_7, 0x0A),
            (LoraBandwidth::Khz62_5, 0x03),
            (LoraBandwidth::Khz125, 0x04),
            (LoraBandwidth::Khz250, 0x05),
            (LoraBandwidth::Khz500, 0x06),
        ];
        for (bandwidth, code) in bandwidths {
            assert_eq!(bandwidth.code(), code);
        }
        assert_eq!(CodingRate::from_denominator(5).code(), 0x01);
        assert_eq!(CodingRate::from_denominator(8).code(), 0x04);
        assert_eq!(CodingRate::Cr4_8LongInterleaver.code(), 0x07);
        assert_eq!(RampTime::Us3400.code(), 0x07);
        assert_eq!(TcxoVoltage::V1_8.code(), 0x02);
        assert_eq!(FallbackMode::Fs.code(), 0x40);
        assert_eq!(PacketType::LrFhss.code(), 0x03);
    }

    #[test]
    fn a_link_becomes_its_modulation_and_packet_parameters() {
        let slow = LinkSettings::new(11, 125_000).with_coding_rate(6);
        assert_eq!(
            LoraModulation::from_link(&slow).map(|m| m.to_params()),
            Some([0x0B, 0x04, 0x02, 0x01])
        );
        let fast = LinkSettings::new(7, 500_000);
        assert_eq!(
            LoraModulation::from_link(&fast).map(|m| m.to_params()),
            Some([0x07, 0x06, 0x01, 0x00])
        );
        assert_eq!(
            LoraModulation::from_link(&LinkSettings::new(7, 200_000)),
            None
        );
        let bare = LinkSettings::new(9, 125_000)
            .with_preamble(300)
            .implicit_header()
            .without_crc();
        assert_eq!(
            LoraPacket::from_link(&bare, 51, true).to_params(),
            [0x01, 0x2C, 0x01, 0x33, 0x00, 0x01]
        );
    }

    #[test]
    fn the_amplifier_settings_follow_table_13_21_and_section_13_4_4() {
        assert_eq!(
            PaConfig::SX1262_22_DBM.to_params(),
            [0x04, 0x07, 0x00, 0x01]
        );
        assert_eq!(
            PaConfig::SX1262_20_DBM.to_params(),
            [0x03, 0x05, 0x00, 0x01]
        );
        assert_eq!(
            PaConfig::SX1262_17_DBM.to_params(),
            [0x02, 0x03, 0x00, 0x01]
        );
        assert_eq!(
            PaConfig::SX1262_14_DBM.to_params(),
            [0x02, 0x02, 0x00, 0x01]
        );
        assert_eq!(
            PaConfig::SX1261_15_DBM.to_params(),
            [0x06, 0x00, 0x01, 0x01]
        );
        assert_eq!(
            PaConfig::SX1261_14_DBM.to_params(),
            [0x04, 0x00, 0x01, 0x01]
        );
        assert_eq!(
            PaConfig::SX1261_10_DBM.to_params(),
            [0x01, 0x00, 0x01, 0x01]
        );

        let high = |dbm| TxPower::for_output(PowerAmplifier::HighPower, dbm);
        assert_eq!(high(22).setting_dbm, 22);
        assert_eq!(high(30).setting_dbm, 22);
        assert_eq!(high(-20).setting_dbm, -9);
        let low = |dbm| TxPower::for_output(PowerAmplifier::LowPower, dbm);
        assert_eq!(
            low(15),
            TxPower {
                pa: PaConfig::SX1261_15_DBM,
                setting_dbm: 14
            }
        );
        assert_eq!(
            low(14),
            TxPower {
                pa: PaConfig::SX1261_14_DBM,
                setting_dbm: 14
            }
        );
        assert_eq!(low(-30).setting_dbm, -17);
        assert_eq!(PowerAmplifier::LowPower.setting_range_dbm(), (-17, 14));
    }

    #[test]
    fn the_power_under_a_ceiling_rounds_down_for_the_antenna() {
        let collinear = LinkBudget {
            transmit_antenna_gain_dbi: Decibels::from_db(6),
            ..LinkBudget::default()
        };
        let power =
            TxPower::under_ceiling(PowerAmplifier::HighPower, &collinear, Decibels::from_db(16));
        assert_eq!(power.setting_dbm, 10);
        let yagi = LinkBudget {
            transmit_antenna_gain_dbi: Decibels::from_hundredths(915),
            ..LinkBudget::default()
        };
        let power = TxPower::under_ceiling(PowerAmplifier::HighPower, &yagi, Decibels::from_db(30));
        assert_eq!(power.setting_dbm, 20);
    }

    #[test]
    fn the_sync_words_and_workaround_values_follow_the_datasheet() {
        assert_eq!(SyncWord::Public.to_bytes(), [0x34, 0x44]);
        assert_eq!(SyncWord::Private.to_bytes(), [0x14, 0x24]);
        assert_eq!(SyncWord::Custom(0x1234).to_bytes(), [0x12, 0x34]);
        assert_eq!(tx_modulation(0x01, LoraBandwidth::Khz125), 0x05);
        assert_eq!(tx_modulation(0x05, LoraBandwidth::Khz500), 0x01);
        assert_eq!(tx_clamp(0xC8), 0xDE);
        assert_eq!(iq_polarity(0x0D, true), 0x09);
        assert_eq!(iq_polarity(0x09, false), 0x0D);
        assert_eq!(event_clear(0x00), 0x02);
    }
}
