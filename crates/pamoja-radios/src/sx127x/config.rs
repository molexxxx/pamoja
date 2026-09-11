//! The settings an SX127x is configured with in LoRa mode, and the values its registers take.
//!
//! Every value comes from the SX1276/77/78/79 datasheet (Rev 7): the carrier word of RegFrf,
//! the bandwidth, coding rate, spreading factor, header, CRC, and low data rate settings of
//! RegModemConfig1 to 3, the amplifier modes of Tables 33 and 34, the current limit of
//! Table 37, the SF6 detection settings, and the sync word. Two sets of values follow
//! Semtech's LoRaMac-node reference driver instead, and say so where they are defined: the
//! IQ polarity bit of the transmit path, which the datasheet describes the wrong way round,
//! and the register writes of the errata for 500 kHz sensitivity and spurious reception.
//! The LoRa settings are built from a [`LinkSettings`], so a radio sends exactly the frames
//! `pamoja-lora` computes the airtime of.

use pamoja_lora::budget::{Decibels, LinkBudget};
use pamoja_lora::LinkSettings;

use super::status::MID_BAND_HZ;

/// The crystal frequency the synthesizer divides, in hertz.
pub const XTAL_HZ: u32 = 32_000_000;

/// The top of the lowest band, 137 to 175 MHz, where the 250 and 500 kHz bandwidths are not
/// supported.
pub const LOW_BAND_TOP_HZ: u32 = 175_000_000;

/// Returns the 24-bit RegFrf word for a frequency.
///
/// The datasheet defines the carrier as the word times the crystal frequency over 2^19, a
/// step of 61.035 Hz. The word is rounded to the nearest step.
///
/// # Arguments
///
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The frequency word.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::config::frequency_word;
///
/// // 434 MHz is the RegFrf reset value the datasheet gives.
/// assert_eq!(frequency_word(434_000_000), 0x6C_8000);
/// assert_eq!(frequency_word(868_100_000), 0xD9_0666);
/// assert_eq!(frequency_word(915_000_000), 0xE4_C000);
/// ```
pub const fn frequency_word(frequency_hz: u32) -> u32 {
    ((((frequency_hz as u64) << 19) + (XTAL_HZ as u64 / 2)) / XTAL_HZ as u64) as u32
}

/// Returns the frequency a RegFrf word selects.
///
/// # Arguments
///
/// * `word` - the 24-bit frequency word.
///
/// # Returns
///
/// The carrier frequency in hertz, rounded to the nearest hertz.
pub const fn frequency_from_word(word: u32) -> u32 {
    ((word as u64 * XTAL_HZ as u64 + (1 << 18)) >> 19) as u32
}

/// Returns the three bytes written to RegFrfMsb, RegFrfMid, and RegFrfLsb for a frequency.
///
/// # Arguments
///
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The frequency word, most significant byte first.
pub const fn frequency_bytes(frequency_hz: u32) -> [u8; 3] {
    let word = frequency_word(frequency_hz);
    [(word >> 16) as u8, (word >> 8) as u8, word as u8]
}

/// A LoRa signal bandwidth, the Bw bits of RegModemConfig1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoraBandwidth {
    /// 7.8 kHz (0000).
    Khz7_8,
    /// 10.4 kHz (0001).
    Khz10_4,
    /// 15.6 kHz (0010).
    Khz15_6,
    /// 20.8 kHz (0011).
    Khz20_8,
    /// 31.25 kHz (0100).
    Khz31_25,
    /// 41.7 kHz (0101).
    Khz41_7,
    /// 62.5 kHz (0110).
    Khz62_5,
    /// 125 kHz (0111).
    Khz125,
    /// 250 kHz (1000).
    Khz250,
    /// 500 kHz (1001).
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

    /// Returns the Bw bits.
    ///
    /// # Returns
    ///
    /// The four bit code, 0 to 9.
    pub const fn code(self) -> u8 {
        match self {
            LoraBandwidth::Khz7_8 => 0,
            LoraBandwidth::Khz10_4 => 1,
            LoraBandwidth::Khz15_6 => 2,
            LoraBandwidth::Khz20_8 => 3,
            LoraBandwidth::Khz31_25 => 4,
            LoraBandwidth::Khz41_7 => 5,
            LoraBandwidth::Khz62_5 => 6,
            LoraBandwidth::Khz125 => 7,
            LoraBandwidth::Khz250 => 8,
            LoraBandwidth::Khz500 => 9,
        }
    }

    /// Returns the bandwidth in hertz, rounded to the nearest hertz.
    ///
    /// # Returns
    ///
    /// The bandwidth: 500 kHz halved, or 125 kHz divided by 3 and halved, as many times as
    /// the setting takes.
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
    /// * `hz` - the bandwidth in hertz, within 1% of one the chip supports.
    ///
    /// # Returns
    ///
    /// The setting, or `None` for a bandwidth the SX127x does not offer.
    pub fn from_hz(hz: u32) -> Option<LoraBandwidth> {
        LoraBandwidth::ALL.into_iter().find(|bandwidth| {
            let nominal = u64::from(bandwidth.hz());
            u64::from(hz).abs_diff(nominal) * 100 <= nominal
        })
    }

    /// Reports whether the bandwidth is available at a carrier frequency.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the carrier frequency in hertz.
    ///
    /// # Returns
    ///
    /// `false` for 250 and 500 kHz at or below [`LOW_BAND_TOP_HZ`], which the datasheet
    /// excludes in the lowest band, and `true` otherwise.
    pub const fn in_band(self, frequency_hz: u32) -> bool {
        !(frequency_hz <= LOW_BAND_TOP_HZ
            && matches!(self, LoraBandwidth::Khz250 | LoraBandwidth::Khz500))
    }
}

/// Why a link's settings cannot be put on an SX127x.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModulationError {
    /// The bandwidth, in hertz, is not one the SX127x offers, or not one it offers at the
    /// carrier frequency.
    Bandwidth(u32),
    /// The spreading factor is below SF6, the lowest the SX127x offers.
    SpreadingFactor(u8),
    /// SF6 with an explicit header, which the datasheet does not allow.
    ExplicitHeaderAtSf6,
}

/// The LoRa modem settings of a link, as RegModemConfig1 to 3 and the SF6 detection
/// registers carry them.
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
/// use pamoja_radios::sx127x::config::LoraModulation;
///
/// // SF7 at 125 kHz, coding rate 4/5, an explicit header, and a CRC.
/// let modulation = LoraModulation::from_link(&LinkSettings::new(7, 125_000)).unwrap();
/// assert_eq!(modulation.modem_config_1(), 0x72);
/// assert_eq!(modulation.modem_config_2(0), 0x74);
/// assert_eq!(modulation.modem_config_3(), 0x04);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoraModulation {
    /// The spreading factor, 6 to 12.
    pub spreading_factor: u8,
    /// The signal bandwidth.
    pub bandwidth: LoraBandwidth,
    /// The coding rate denominator, 5 to 8 for 4/5 to 4/8.
    pub coding_rate_denominator: u8,
    /// Whether packets carry an explicit header.
    pub explicit_header: bool,
    /// Whether packets carry a payload CRC.
    pub crc: bool,
    /// Whether low data rate optimization is on.
    pub low_data_rate_optimization: bool,
}

impl LoraModulation {
    /// Builds the modem settings of a link.
    ///
    /// # Arguments
    ///
    /// * `link` - the link settings, whose spreading factor, bandwidth, coding rate, header,
    ///   CRC, and low data rate optimization the radio must match.
    ///
    /// # Returns
    ///
    /// The settings.
    ///
    /// # Errors
    ///
    /// Returns [`ModulationError::Bandwidth`] for a bandwidth the SX127x does not offer,
    /// [`ModulationError::SpreadingFactor`] below SF6, and
    /// [`ModulationError::ExplicitHeaderAtSf6`] for SF6 with an explicit header.
    pub fn from_link(link: &LinkSettings) -> Result<LoraModulation, ModulationError> {
        let bandwidth = LoraBandwidth::from_hz(link.bandwidth_hz())
            .ok_or(ModulationError::Bandwidth(link.bandwidth_hz()))?;
        let spreading_factor = link.spreading_factor();
        if spreading_factor < 6 {
            return Err(ModulationError::SpreadingFactor(spreading_factor));
        }
        if spreading_factor == 6 && link.explicit_header() {
            return Err(ModulationError::ExplicitHeaderAtSf6);
        }
        Ok(LoraModulation {
            spreading_factor,
            bandwidth,
            coding_rate_denominator: link.coding_rate_denominator(),
            explicit_header: link.explicit_header(),
            crc: link.crc(),
            low_data_rate_optimization: link.low_data_rate_optimization(),
        })
    }

    /// Returns RegModemConfig1: the bandwidth in bits 7 to 4, the coding rate in bits 3 to 1,
    /// and ImplicitHeaderModeOn in bit 0.
    ///
    /// # Returns
    ///
    /// The register value.
    pub const fn modem_config_1(&self) -> u8 {
        let coding_rate = if self.coding_rate_denominator < 5 {
            1
        } else if self.coding_rate_denominator > 8 {
            4
        } else {
            self.coding_rate_denominator - 4
        };
        (self.bandwidth.code() << 4) | (coding_rate << 1) | (!self.explicit_header as u8)
    }

    /// Returns RegModemConfig2: the spreading factor in bits 7 to 4, RxPayloadCrcOn in bit 2,
    /// and the top two bits of the symbol timeout in bits 1 and 0.
    ///
    /// # Arguments
    ///
    /// * `symbol_timeout` - the single reception timeout in symbols, from
    ///   [`symbol_timeout`].
    ///
    /// # Returns
    ///
    /// The register value; TxContinuousMode is off.
    pub const fn modem_config_2(&self, symbol_timeout: u16) -> u8 {
        (self.spreading_factor << 4)
            | ((self.crc as u8) << 2)
            | ((symbol_timeout >> 8) as u8 & 0x03)
    }

    /// Returns RegModemConfig3: LowDataRateOptimize in bit 3, and AgcAutoOn in bit 2 so the
    /// automatic gain control sets the LNA gain.
    ///
    /// # Returns
    ///
    /// The register value.
    pub const fn modem_config_3(&self) -> u8 {
        ((self.low_data_rate_optimization as u8) << 3) | MODEM_CONFIG_3_AGC_AUTO_ON
    }

    /// Returns RegDetectOptimize with the DetectionOptimize bits set for the spreading
    /// factor: 0x05 for SF6 and 0x03 for SF7 to SF12.
    ///
    /// # Arguments
    ///
    /// * `current` - the register's current value, whose other bits are kept.
    ///
    /// # Returns
    ///
    /// The register value.
    pub const fn detect_optimize(&self, current: u8) -> u8 {
        let optimize = if self.spreading_factor == 6 {
            0x05
        } else {
            0x03
        };
        (current & 0xF8) | optimize
    }

    /// Returns RegDetectionThreshold for the spreading factor: 0x0C for SF6 and 0x0A for
    /// SF7 to SF12.
    ///
    /// # Returns
    ///
    /// The register value.
    pub const fn detection_threshold(&self) -> u8 {
        if self.spreading_factor == 6 {
            0x0C
        } else {
            0x0A
        }
    }
}

/// RegModemConfig3 bit 2: the LNA gain comes from the automatic gain control.
pub const MODEM_CONFIG_3_AGC_AUTO_ON: u8 = 0x04;

/// The shortest single reception timeout the datasheet allows, in symbols.
pub const SYMBOL_TIMEOUT_MIN: u16 = 4;

/// The longest single reception timeout RegModemConfig2 and RegSymbTimeoutLsb hold, in
/// symbols.
pub const SYMBOL_TIMEOUT_MAX: u16 = 1023;

/// Returns the single reception timeout for a duration, in the link's symbols.
///
/// # Arguments
///
/// * `link` - the link settings, whose symbol time counts the timeout.
/// * `timeout_us` - how long to listen for a preamble, in microseconds.
///
/// # Returns
///
/// The timeout rounded up to whole symbols, from [`SYMBOL_TIMEOUT_MIN`] to
/// [`SYMBOL_TIMEOUT_MAX`].
///
/// # Examples
///
/// ```
/// use pamoja_lora::LinkSettings;
/// use pamoja_radios::sx127x::config::symbol_timeout;
///
/// // SF7 at 125 kHz sends a symbol every 1.024 ms.
/// let link = LinkSettings::new(7, 125_000);
/// assert_eq!(symbol_timeout(&link, 100_000), 98);
/// assert_eq!(symbol_timeout(&link, 0), 4);
/// assert_eq!(symbol_timeout(&link, 10_000_000), 1023);
/// ```
pub fn symbol_timeout(link: &LinkSettings, timeout_us: u64) -> u16 {
    let symbols = timeout_us.div_ceil(link.symbol_time_us().max(1));
    symbols.clamp(u64::from(SYMBOL_TIMEOUT_MIN), u64::from(SYMBOL_TIMEOUT_MAX)) as u16
}

/// Returns RegPreambleMsb and RegPreambleLsb for a link.
///
/// # Arguments
///
/// * `link` - the link settings.
///
/// # Returns
///
/// The preamble length in symbols, most significant byte first; the modem adds 4.25
/// symbols of its own.
pub fn preamble_bytes(link: &LinkSettings) -> [u8; 2] {
    link.preamble_symbols().to_be_bytes()
}

/// A LoRa sync word, written to RegSyncWord.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyncWord {
    /// 0x34, which the datasheet reserves for LoRaWAN networks.
    Public,
    /// 0x12, for a private network, and the chip's reset value.
    Private,
    /// Another value.
    Custom(u8),
}

impl SyncWord {
    /// Returns the register value.
    ///
    /// # Returns
    ///
    /// The RegSyncWord byte.
    pub const fn to_byte(self) -> u8 {
        match self {
            SyncWord::Public => 0x34,
            SyncWord::Private => 0x12,
            SyncWord::Custom(word) => word,
        }
    }
}

/// Which amplifier output a module wires to its antenna, from Table 33.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaOutput {
    /// The high efficiency amplifier on RFO_LF or RFO_HF, -4 to +15 dBm.
    Rfo,
    /// The regulated amplifier on PA_BOOST, +2 to +17 dBm, or +20 dBm with the high power
    /// setting of Table 34. The RFM95W wires its antenna here.
    PaBoost,
}

impl PaOutput {
    /// Returns the output powers the amplifier delivers.
    ///
    /// # Returns
    ///
    /// The lowest and highest in dBm.
    pub const fn range_dbm(self) -> (i8, i8) {
        match self {
            PaOutput::Rfo => (-4, 15),
            PaOutput::PaBoost => (2, 20),
        }
    }
}

/// RegPaDac at its reset value.
pub const PA_DAC_DEFAULT: u8 = 0x84;

/// RegPaDac with the +20 dBm setting of Table 34 on PA_BOOST.
pub const PA_DAC_HIGH_POWER: u8 = 0x87;

/// Returns RegOcp for a current limit, from Table 37.
///
/// # Arguments
///
/// * `milliamps` - the most current the amplifier may draw.
///
/// # Returns
///
/// The register value with OcpOn set and the trim that gives the limit: 45 + 5 * trim mA up
/// to 120 mA, -30 + 10 * trim mA up to 240 mA, and 240 mA above.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::config::ocp_register;
///
/// // 100 mA is the RegOcp reset value.
/// assert_eq!(ocp_register(100), 0x2B);
/// assert_eq!(ocp_register(140), 0x31);
/// ```
pub const fn ocp_register(milliamps: u16) -> u8 {
    let trim = if milliamps <= 45 {
        0
    } else if milliamps <= 120 {
        (milliamps - 45) / 5
    } else if milliamps <= 240 {
        (milliamps + 30) / 10
    } else {
        28
    };
    0x20 | trim as u8
}

/// The amplifier settings that produce an output power: RegPaConfig, RegPaDac, and RegOcp.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::config::{PaOutput, TxPower, PA_DAC_HIGH_POWER};
///
/// // +20 dBm on the PA_BOOST pin of an RFM95W.
/// let power = TxPower::for_output(PaOutput::PaBoost, 20);
/// assert_eq!(power.pa_config, 0xFF);
/// assert_eq!(power.pa_dac, PA_DAC_HIGH_POWER);
/// assert_eq!(power.output_dbm, 20);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TxPower {
    /// RegPaConfig: PaSelect in bit 7, MaxPower in bits 6 to 4, OutputPower in bits 3 to 0.
    pub pa_config: u8,
    /// RegPaDac: [`PA_DAC_HIGH_POWER`] above +17 dBm on PA_BOOST, else [`PA_DAC_DEFAULT`].
    pub pa_dac: u8,
    /// RegOcp: a 140 mA limit with the high power setting, whose amplifier draws 120 mA at
    /// +20 dBm, and the 100 mA reset limit otherwise.
    pub ocp: u8,
    /// The output power these settings produce, in dBm.
    pub output_dbm: i8,
}

impl TxPower {
    /// Chooses the settings for an output power.
    ///
    /// The steps follow Semtech's LoRaMac-node. On RFO, MaxPower 7 gives a +15 dBm maximum
    /// and OutputPower the power itself, and at 0 dBm and below MaxPower 0 and OutputPower
    /// the power plus 4, which lands 0.2 dB under. On PA_BOOST, OutputPower is the power
    /// less 2 up to +17 dBm, and with the high power setting the power less 5 above it.
    ///
    /// # Arguments
    ///
    /// * `output` - the amplifier output the module uses.
    /// * `output_dbm` - the output power wanted, clamped to what the output delivers.
    ///
    /// # Returns
    ///
    /// The settings.
    pub const fn for_output(output: PaOutput, output_dbm: i8) -> TxPower {
        let (low, high) = output.range_dbm();
        let dbm = if output_dbm < low {
            low
        } else if output_dbm > high {
            high
        } else {
            output_dbm
        };
        match output {
            PaOutput::Rfo if dbm > 0 => TxPower {
                pa_config: 0x70 | dbm as u8,
                pa_dac: PA_DAC_DEFAULT,
                ocp: ocp_register(100),
                output_dbm: dbm,
            },
            PaOutput::Rfo => TxPower {
                pa_config: (dbm + 4) as u8,
                pa_dac: PA_DAC_DEFAULT,
                ocp: ocp_register(100),
                output_dbm: dbm,
            },
            PaOutput::PaBoost if dbm > 17 => TxPower {
                pa_config: 0xF0 | (dbm - 5) as u8,
                pa_dac: PA_DAC_HIGH_POWER,
                ocp: ocp_register(140),
                output_dbm: dbm,
            },
            PaOutput::PaBoost => TxPower {
                pa_config: 0xF0 | (dbm - 2) as u8,
                pa_dac: PA_DAC_DEFAULT,
                ocp: ocp_register(100),
                output_dbm: dbm,
            },
        }
    }

    /// Chooses the settings that keep a link's EIRP at or under a ceiling.
    ///
    /// # Arguments
    ///
    /// * `output` - the amplifier output the module uses.
    /// * `budget` - the link budget, whose transmitting antenna and cable apply.
    /// * `eirp_ceiling_dbm` - the EIRP limit, such as a channel plan's ceiling for the
    ///   frequency in use.
    ///
    /// # Returns
    ///
    /// The settings, rounded down to whole decibels so the EIRP stays under the ceiling.
    pub fn under_ceiling(
        output: PaOutput,
        budget: &LinkBudget,
        eirp_ceiling_dbm: Decibels,
    ) -> TxPower {
        let most = budget.max_transmit_power_dbm(eirp_ceiling_dbm).floor_db();
        let dbm = most.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
        TxPower::for_output(output, dbm)
    }

    /// Returns the amplifier output these settings select.
    ///
    /// # Returns
    ///
    /// [`PaOutput::PaBoost`] when PaSelect is set, else [`PaOutput::Rfo`].
    pub const fn output(&self) -> PaOutput {
        if self.pa_config & 0x80 != 0 {
            PaOutput::PaBoost
        } else {
            PaOutput::Rfo
        }
    }
}

/// The reserved bits of RegInvertIQ at their reset value.
const INVERT_IQ_RESERVED: u8 = 0x26;

/// Returns RegInvertIQ for the IQ polarity of each path.
///
/// Bit 6 inverts the receive path as the datasheet describes. Bit 0 of the transmit path
/// works the other way from its description: Semtech's LoRaMac-node sets it for normal IQ
/// and clears it to invert, which RadioLib and arduino-LoRa also do after finding the
/// datasheet's reading inverts the wrong frames.
///
/// # Arguments
///
/// * `receive` - whether to invert the receive path, as a LoRaWAN device does for downlinks.
/// * `transmit` - whether to invert the transmit path, as a gateway does.
///
/// # Returns
///
/// The register value.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::config::invert_iq;
///
/// assert_eq!(invert_iq(false, false), 0x27);
/// assert_eq!(invert_iq(true, true), 0x66);
/// ```
pub const fn invert_iq(receive: bool, transmit: bool) -> u8 {
    INVERT_IQ_RESERVED | if receive { 0x40 } else { 0x00 } | if transmit { 0x00 } else { 0x01 }
}

/// Returns RegInvertIQ2, which the chip needs set to 0x19 while a path is inverted.
///
/// # Arguments
///
/// * `inverted` - whether the path in use is inverted.
///
/// # Returns
///
/// 0x19 when inverted, else its reset value 0x1D.
pub const fn invert_iq_2(inverted: bool) -> u8 {
    if inverted {
        0x19
    } else {
        0x1D
    }
}

/// RegDioMapping1 with DIO0 signaling RxDone, from Table 18.
pub const DIO0_RX_DONE: u8 = 0x00;
/// RegDioMapping1 with DIO0 signaling TxDone.
pub const DIO0_TX_DONE: u8 = 0x40;
/// RegDioMapping1 with DIO0 signaling CadDone.
pub const DIO0_CAD_DONE: u8 = 0x80;

/// RegImageCal bit 6: starts an image and RSSI calibration.
pub const IMAGE_CAL_START: u8 = 0x40;
/// RegImageCal bit 5: set while a calibration runs.
pub const IMAGE_CAL_RUNNING: u8 = 0x20;

/// Returns RegImageCal to start a calibration.
///
/// # Arguments
///
/// * `current` - the register's current value.
///
/// # Returns
///
/// The value with ImageCalStart set and AutoImageCalOn clear, since the datasheet recommends
/// triggering calibration deliberately rather than on a temperature change.
pub const fn image_cal_start(current: u8) -> u8 {
    (current & 0x3F) | IMAGE_CAL_START
}

/// RegLna with the maximum gain and the 150% LNA current of the high frequency port, which
/// LoRaMac-node sets at startup; with AgcAutoOn the gain is the AGC's.
pub const LNA_BOOSTED: u8 = 0x23;

/// RegTcxo with TcxoInputOn set, for a module clocked by a TCXO on XTA.
pub const TCXO_INPUT_ON: u8 = 0x19;

/// RegHighBwOptimize1 and, where it changes, RegHighBwOptimize2 for a bandwidth.
///
/// These are the writes of erratum 2.1, sensitivity optimization with a 500 kHz bandwidth, as
/// Semtech's LoRaMac-node makes them: 0x02 and 0x64 above [`MID_BAND_HZ`], 0x02 and 0x7F
/// below, and 0x03 alone for any other bandwidth.
///
/// # Arguments
///
/// * `bandwidth` - the signal bandwidth.
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The RegHighBwOptimize1 value, and the RegHighBwOptimize2 value when one is written.
pub const fn high_bw_optimize(bandwidth: LoraBandwidth, frequency_hz: u32) -> (u8, Option<u8>) {
    match bandwidth {
        LoraBandwidth::Khz500 if frequency_hz > MID_BAND_HZ => (0x02, Some(0x64)),
        LoraBandwidth::Khz500 => (0x02, Some(0x7F)),
        _ => (0x03, None),
    }
}

/// The receive settings of erratum 2.3, receiver spurious reception of a LoRa signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpuriousReception {
    /// Whether AutomaticIFOn, bit 7 of RegDetectOptimize, stays on.
    pub automatic_if: bool,
    /// The value for RegIfFreq2 at 0x2F, with RegIfFreq1 at 0x30 cleared, when the IF is set
    /// by hand.
    pub if_freq_2: Option<u8>,
    /// How far above the carrier to receive, in hertz.
    pub offset_hz: u32,
}

/// Returns the receive settings of erratum 2.3 for a bandwidth, as Semtech's LoRaMac-node
/// applies them.
///
/// At 500 kHz the automatic IF stays on. Below it the IF is set by hand, and at 41.7 kHz and
/// narrower the receiver also tunes one bandwidth above the carrier.
///
/// # Arguments
///
/// * `bandwidth` - the signal bandwidth.
///
/// # Returns
///
/// The settings.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx127x::config::{spurious_reception, LoraBandwidth};
///
/// let narrow = spurious_reception(LoraBandwidth::Khz20_8);
/// assert_eq!((narrow.if_freq_2, narrow.offset_hz), (Some(0x44), 20_830));
/// assert!(spurious_reception(LoraBandwidth::Khz500).automatic_if);
/// ```
pub const fn spurious_reception(bandwidth: LoraBandwidth) -> SpuriousReception {
    let (if_freq_2, offset_hz) = match bandwidth {
        LoraBandwidth::Khz500 => {
            return SpuriousReception {
                automatic_if: true,
                if_freq_2: None,
                offset_hz: 0,
            }
        }
        LoraBandwidth::Khz7_8 => (0x48, 7_810),
        LoraBandwidth::Khz10_4 => (0x44, 10_420),
        LoraBandwidth::Khz15_6 => (0x44, 15_620),
        LoraBandwidth::Khz20_8 => (0x44, 20_830),
        LoraBandwidth::Khz31_25 => (0x44, 31_250),
        LoraBandwidth::Khz41_7 => (0x44, 41_670),
        LoraBandwidth::Khz62_5 | LoraBandwidth::Khz125 | LoraBandwidth::Khz250 => (0x40, 0),
    };
    SpuriousReception {
        automatic_if: false,
        if_freq_2: Some(if_freq_2),
        offset_hz,
    }
}

/// Returns RegDetectOptimize with AutomaticIFOn as erratum 2.3 wants it.
///
/// # Arguments
///
/// * `current` - the register's current value, whose other bits are kept.
/// * `automatic_if` - whether the automatic IF stays on.
///
/// # Returns
///
/// The register value.
pub const fn automatic_if(current: u8, automatic_if: bool) -> u8 {
    if automatic_if {
        current | 0x80
    } else {
        current & 0x7F
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_words_round_trip_to_within_half_a_step() {
        for frequency in [
            137_000_000,
            433_175_000,
            868_100_000,
            902_300_000,
            1_020_000_000,
        ] {
            let back = frequency_from_word(frequency_word(frequency));
            assert!(
                back.abs_diff(frequency) <= 31,
                "{frequency} came back as {back}"
            );
        }
        assert_eq!(frequency_bytes(868_100_000), [0xD9, 0x06, 0x66]);
    }

    #[test]
    fn bandwidths_take_the_codes_of_modem_config_1_and_match_within_one_percent() {
        assert_eq!(LoraBandwidth::from_hz(125_000), Some(LoraBandwidth::Khz125));
        assert_eq!(LoraBandwidth::from_hz(10_420), Some(LoraBandwidth::Khz10_4));
        assert_eq!(LoraBandwidth::from_hz(203_125), None);
        assert_eq!(LoraBandwidth::Khz7_8.code(), 0);
        assert_eq!(LoraBandwidth::Khz500.code(), 9);
        assert!(!LoraBandwidth::Khz500.in_band(169_000_000));
        assert!(LoraBandwidth::Khz125.in_band(169_000_000));
        assert!(LoraBandwidth::Khz500.in_band(433_000_000));
    }

    #[test]
    fn a_slow_link_turns_on_low_data_rate_optimization() {
        let slow = LoraModulation::from_link(&LinkSettings::new(12, 125_000)).unwrap();
        assert_eq!(slow.modem_config_1(), 0x72);
        assert_eq!(slow.modem_config_2(0), 0xC4);
        assert_eq!(slow.modem_config_3(), 0x0C);
    }

    #[test]
    fn the_modem_settings_carry_coding_rate_header_crc_and_timeout() {
        let link = LinkSettings::new(9, 250_000)
            .with_coding_rate(8)
            .implicit_header()
            .without_crc();
        let modulation = LoraModulation::from_link(&link).unwrap();
        assert_eq!(modulation.modem_config_1(), 0x89);
        assert_eq!(modulation.modem_config_2(0x3FF), 0x93);
        assert_eq!(modulation.detect_optimize(0xC5), 0xC3);
        assert_eq!(modulation.detection_threshold(), 0x0A);
    }

    #[test]
    fn sf6_needs_an_implicit_header_and_its_own_detection_settings() {
        assert_eq!(
            LoraModulation::from_link(&LinkSettings::new(6, 125_000)),
            Err(ModulationError::ExplicitHeaderAtSf6)
        );
        let sf6 =
            LoraModulation::from_link(&LinkSettings::new(6, 125_000).implicit_header()).unwrap();
        assert_eq!(sf6.detect_optimize(0xC3), 0xC5);
        assert_eq!(sf6.detection_threshold(), 0x0C);
        assert_eq!(
            LoraModulation::from_link(&LinkSettings::new(5, 125_000)),
            Err(ModulationError::SpreadingFactor(5))
        );
    }

    #[test]
    fn the_current_limit_follows_table_37() {
        assert_eq!(ocp_register(45), 0x20);
        assert_eq!(ocp_register(120), 0x2F);
        assert_eq!(ocp_register(130), 0x30);
        assert_eq!(ocp_register(240), 0x3B);
        assert_eq!(ocp_register(300), 0x3C);
    }

    #[test]
    fn each_output_power_lands_on_the_formula_of_its_amplifier() {
        let rfo = TxPower::for_output(PaOutput::Rfo, 14);
        assert_eq!((rfo.pa_config, rfo.pa_dac, rfo.ocp), (0x7E, 0x84, 0x2B));
        assert_eq!(TxPower::for_output(PaOutput::Rfo, 0).pa_config, 0x04);
        assert_eq!(TxPower::for_output(PaOutput::Rfo, -9).pa_config, 0x00);
        assert_eq!(TxPower::for_output(PaOutput::Rfo, 30).output_dbm, 15);

        let boost = TxPower::for_output(PaOutput::PaBoost, 17);
        assert_eq!(
            (boost.pa_config, boost.pa_dac, boost.ocp),
            (0xFF, 0x84, 0x2B)
        );
        assert_eq!(TxPower::for_output(PaOutput::PaBoost, 2).pa_config, 0xF0);
        let high = TxPower::for_output(PaOutput::PaBoost, 18);
        assert_eq!((high.pa_config, high.pa_dac, high.ocp), (0xFD, 0x87, 0x31));
        assert_eq!(TxPower::for_output(PaOutput::PaBoost, 0).output_dbm, 2);
        assert_eq!(boost.output(), PaOutput::PaBoost);
        assert_eq!(rfo.output(), PaOutput::Rfo);
    }

    #[test]
    fn the_ceiling_leaves_whole_decibels_under_the_eirp_limit() {
        let whip = LinkBudget {
            transmit_antenna_gain_dbi: Decibels::from_hundredths(215),
            transmit_cable_loss_db: Decibels::from_tenths(5),
            ..LinkBudget::default()
        };
        let power = TxPower::under_ceiling(PaOutput::PaBoost, &whip, Decibels::from_db(16));
        assert_eq!(power.output_dbm, 14);
        assert_eq!(power.pa_config, 0xFC);
    }

    #[test]
    fn iq_polarity_follows_the_reference_drivers_not_the_description() {
        assert_eq!(invert_iq(true, false), 0x67);
        assert_eq!(invert_iq(false, true), 0x26);
        assert_eq!(invert_iq_2(true), 0x19);
        assert_eq!(invert_iq_2(false), 0x1D);
    }

    #[test]
    fn the_errata_writes_match_the_reference_driver() {
        assert_eq!(
            high_bw_optimize(LoraBandwidth::Khz500, 915_000_000),
            (0x02, Some(0x64))
        );
        assert_eq!(
            high_bw_optimize(LoraBandwidth::Khz500, 433_000_000),
            (0x02, Some(0x7F))
        );
        assert_eq!(
            high_bw_optimize(LoraBandwidth::Khz125, 868_100_000),
            (0x03, None)
        );
        assert_eq!(
            spurious_reception(LoraBandwidth::Khz7_8),
            SpuriousReception {
                automatic_if: false,
                if_freq_2: Some(0x48),
                offset_hz: 7_810
            }
        );
        assert_eq!(spurious_reception(LoraBandwidth::Khz125).offset_hz, 0);
        assert_eq!(automatic_if(0xC3, false), 0x43);
        assert_eq!(automatic_if(0x43, true), 0xC3);
        assert_eq!(image_cal_start(0x82), 0x42);
    }
}
