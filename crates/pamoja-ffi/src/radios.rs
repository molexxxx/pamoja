//! The C ABI for LoRa radio chips.
//!
//! These functions wrap [`pamoja_radios`] for callers that drive a radio through the flat
//! C boundary: the bytes of each Semtech SX126x command, the chip's answers decoded, the
//! amplifier settings a regional power ceiling allows, and the silence a duty-cycle limit
//! forces after each transmission. A caller with its own SPI access sends each command in
//! one transaction, once BUSY is low, and reads a query's answer in the same transaction.
//!
//! The SX1276 family is driven through registers rather than commands, so for it these give
//! the register addresses, the values the settings put in them, and the decoded readings.
//!
//! A command is at most ten bytes, so it crosses by value as [`PamojaSx126xCommand`] and
//! costs no allocation. The duty-cycle guard keeps state between calls, so it is a handle.

use pamoja_radios::duty::DutyCycle;
use pamoja_radios::sx126x::command::{self, Command, Query};
use pamoja_radios::sx126x::config::{
    self, LoraModulation, LoraPacket, PaConfig, PacketType, PowerAmplifier, RampTime, StandbyMode,
    SyncWord, TxPower,
};
use pamoja_radios::sx126x::irq::Irq;
use pamoja_radios::sx126x::status::{
    rssi_inst_dbm, ChipMode, CommandStatus, DeviceErrors, PacketStatus, RxBufferStatus, Status,
};

use pamoja_radios::sx126x::config::llcc68_supports;
use pamoja_radios::sx127x::config::{
    self as sx127x_config, LoraBandwidth as Sx127xBandwidth, LoraModulation as Sx127xModulation,
    ModulationError, PaOutput, TxPower as Sx127xPower,
};
use pamoja_radios::sx127x::irq::IrqFlags;
use pamoja_radios::sx127x::register::{self as sx127x_register, Mode as Sx127xMode};
use pamoja_radios::sx127x::status::{
    rssi_dbm as sx127x_rssi_dbm, ModemStatus as Sx127xModemStatus,
    PacketStatus as Sx127xPacketStatus, Port,
};

use pamoja_lora::budget::Decibels;

use crate::lora::{link_budget, settings, PamojaLoraLink, PamojaLoraLinkBudget};
use crate::{set_last_error, PamojaStatus};

/// The most bytes one SX126x command takes, opcode included.
pub const PAMOJA_SX126X_COMMAND_MAX: usize = 10;

/// The receive timeout word that keeps an SX126x listening until another command stops it.
pub const PAMOJA_SX126X_RX_CONTINUOUS: u32 = 0xFF_FFFF;

/// The LoRa sync word of a public network such as LoRaWAN.
pub const PAMOJA_SX126X_SYNC_WORD_PUBLIC: u16 = 0x3444;

/// The LoRa sync word of a private network, and the chip's reset value.
pub const PAMOJA_SX126X_SYNC_WORD_PRIVATE: u16 = 0x1424;

/// The register that holds the most significant byte of the LoRa sync word.
pub const PAMOJA_SX126X_REGISTER_LORA_SYNC_WORD: u16 = 0x0740;

/// The IRQ bit raised when a packet has been sent.
pub const PAMOJA_SX126X_IRQ_TX_DONE: u16 = 1 << 0;
/// The IRQ bit raised when a packet has been received.
pub const PAMOJA_SX126X_IRQ_RX_DONE: u16 = 1 << 1;
/// The IRQ bit raised when a preamble has been detected.
pub const PAMOJA_SX126X_IRQ_PREAMBLE_DETECTED: u16 = 1 << 2;
/// The IRQ bit raised when a valid (G)FSK sync word has been detected.
pub const PAMOJA_SX126X_IRQ_SYNC_WORD_VALID: u16 = 1 << 3;
/// The IRQ bit raised when a valid LoRa header has been received.
pub const PAMOJA_SX126X_IRQ_HEADER_VALID: u16 = 1 << 4;
/// The IRQ bit raised when a LoRa header failed its CRC.
pub const PAMOJA_SX126X_IRQ_HEADER_ERROR: u16 = 1 << 5;
/// The IRQ bit raised when a packet failed its CRC.
pub const PAMOJA_SX126X_IRQ_CRC_ERROR: u16 = 1 << 6;
/// The IRQ bit raised when channel activity detection has finished.
pub const PAMOJA_SX126X_IRQ_CAD_DONE: u16 = 1 << 7;
/// The IRQ bit raised when channel activity detection heard LoRa.
pub const PAMOJA_SX126X_IRQ_CAD_DETECTED: u16 = 1 << 8;
/// The IRQ bit raised when a transmission or reception timed out.
pub const PAMOJA_SX126X_IRQ_TIMEOUT: u16 = 1 << 9;
/// The IRQ bit raised at each long-range FHSS hop.
pub const PAMOJA_SX126X_IRQ_LR_FHSS_HOP: u16 = 1 << 14;
/// Every IRQ bit the chip defines.
pub const PAMOJA_SX126X_IRQ_ALL: u16 = 0x43FF;

/// The device error bit for a failed RC64k calibration.
pub const PAMOJA_SX126X_ERROR_RC64K_CALIBRATION: u16 = 1 << 0;
/// The device error bit for a failed RC13M calibration.
pub const PAMOJA_SX126X_ERROR_RC13M_CALIBRATION: u16 = 1 << 1;
/// The device error bit for a failed PLL calibration.
pub const PAMOJA_SX126X_ERROR_PLL_CALIBRATION: u16 = 1 << 2;
/// The device error bit for a failed ADC calibration.
pub const PAMOJA_SX126X_ERROR_ADC_CALIBRATION: u16 = 1 << 3;
/// The device error bit for a failed image calibration.
pub const PAMOJA_SX126X_ERROR_IMAGE_CALIBRATION: u16 = 1 << 4;
/// The device error bit for a crystal oscillator that failed to start.
pub const PAMOJA_SX126X_ERROR_XOSC_START: u16 = 1 << 5;
/// The device error bit for a PLL that failed to lock.
pub const PAMOJA_SX126X_ERROR_PLL_LOCK: u16 = 1 << 6;
/// The device error bit for a power amplifier that failed to ramp.
pub const PAMOJA_SX126X_ERROR_PA_RAMP: u16 = 1 << 8;

// The header generator does not read the crates this one depends on, so these carry their
// value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_SX126X_COMMAND_MAX == command::MAX_LEN);
const _: () = assert!(PAMOJA_SX126X_RX_CONTINUOUS == config::RX_CONTINUOUS);
const _: () =
    assert!(PAMOJA_SX126X_SYNC_WORD_PUBLIC == u16::from_be_bytes(SyncWord::Public.to_bytes()));
const _: () =
    assert!(PAMOJA_SX126X_SYNC_WORD_PRIVATE == u16::from_be_bytes(SyncWord::Private.to_bytes()));
const _: () = assert!(PAMOJA_SX126X_REGISTER_LORA_SYNC_WORD == config::register::LORA_SYNC_WORD);
const _: () = assert!(PAMOJA_SX126X_IRQ_TX_DONE == Irq::TX_DONE.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_RX_DONE == Irq::RX_DONE.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_PREAMBLE_DETECTED == Irq::PREAMBLE_DETECTED.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_SYNC_WORD_VALID == Irq::SYNC_WORD_VALID.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_HEADER_VALID == Irq::HEADER_VALID.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_HEADER_ERROR == Irq::HEADER_ERROR.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_CRC_ERROR == Irq::CRC_ERROR.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_CAD_DONE == Irq::CAD_DONE.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_CAD_DETECTED == Irq::CAD_DETECTED.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_TIMEOUT == Irq::TIMEOUT.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_LR_FHSS_HOP == Irq::LR_FHSS_HOP.bits());
const _: () = assert!(PAMOJA_SX126X_IRQ_ALL == Irq::ALL.bits());
const _: () =
    assert!(PAMOJA_SX126X_ERROR_RC64K_CALIBRATION == DeviceErrors::RC64K_CALIBRATION.bits());
const _: () =
    assert!(PAMOJA_SX126X_ERROR_RC13M_CALIBRATION == DeviceErrors::RC13M_CALIBRATION.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_PLL_CALIBRATION == DeviceErrors::PLL_CALIBRATION.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_ADC_CALIBRATION == DeviceErrors::ADC_CALIBRATION.bits());
const _: () =
    assert!(PAMOJA_SX126X_ERROR_IMAGE_CALIBRATION == DeviceErrors::IMAGE_CALIBRATION.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_XOSC_START == DeviceErrors::XOSC_START.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_PLL_LOCK == DeviceErrors::PLL_LOCK.bits());
const _: () = assert!(PAMOJA_SX126X_ERROR_PA_RAMP == DeviceErrors::PA_RAMP.bits());

/// The RegVersion value of an SX1276, SX1277, SX1278, or SX1279.
pub const PAMOJA_SX127X_VERSION: u8 = 0x12;

/// The bit of an SX127x address byte that makes an access a write.
pub const PAMOJA_SX127X_WRITE: u8 = 0x80;

/// RegDioMapping1 with DIO0 signaling RxDone.
pub const PAMOJA_SX127X_DIO0_RX_DONE: u8 = 0x00;

/// RegDioMapping1 with DIO0 signaling TxDone.
pub const PAMOJA_SX127X_DIO0_TX_DONE: u8 = 0x40;

/// RegDioMapping1 with DIO0 signaling CadDone.
pub const PAMOJA_SX127X_DIO0_CAD_DONE: u8 = 0x80;

/// RegPaDac at its reset value.
pub const PAMOJA_SX127X_PA_DAC_DEFAULT: u8 = 0x84;

/// RegPaDac with the +20 dBm setting on PA_BOOST.
pub const PAMOJA_SX127X_PA_DAC_HIGH_POWER: u8 = 0x87;

/// The RegImageCal bit that starts a calibration.
pub const PAMOJA_SX127X_IMAGE_CAL_START: u8 = 0x40;

/// The RegImageCal bit set while a calibration runs.
pub const PAMOJA_SX127X_IMAGE_CAL_RUNNING: u8 = 0x20;

/// The sync word the datasheet reserves for LoRaWAN.
pub const PAMOJA_SX127X_SYNC_WORD_PUBLIC: u8 = 0x34;

/// The private sync word, and the chip's reset value.
pub const PAMOJA_SX127X_SYNC_WORD_PRIVATE: u8 = 0x12;

/// RegLna with maximum gain and the high frequency LNA boost.
pub const PAMOJA_SX127X_LNA_BOOSTED: u8 = 0x23;

/// RegTcxo for a module clocked by a TCXO.
pub const PAMOJA_SX127X_TCXO_INPUT_ON: u8 = 0x19;

/// The SX127x register RegFifo, the LoRa data buffer read or written at RegFifoAddrPtr.
pub const PAMOJA_SX127X_REG_FIFO: u8 = 0x00;

/// The SX127x register RegOpMode: LoRa or FSK, the register page, and the operating mode.
pub const PAMOJA_SX127X_REG_OP_MODE: u8 = 0x01;

/// The SX127x register RegFrfMsb, the top byte of the carrier word.
pub const PAMOJA_SX127X_REG_FRF_MSB: u8 = 0x06;

/// The SX127x register RegFrfMid, the middle byte of the carrier word.
pub const PAMOJA_SX127X_REG_FRF_MID: u8 = 0x07;

/// The SX127x register RegFrfLsb, the low byte of the carrier word.
pub const PAMOJA_SX127X_REG_FRF_LSB: u8 = 0x08;

/// The SX127x register RegPaConfig: the amplifier output, its maximum, and the power.
pub const PAMOJA_SX127X_REG_PA_CONFIG: u8 = 0x09;

/// The SX127x register RegPaRamp, the amplifier ramp time.
pub const PAMOJA_SX127X_REG_PA_RAMP: u8 = 0x0A;

/// The SX127x register RegOcp, the amplifier current limit.
pub const PAMOJA_SX127X_REG_OCP: u8 = 0x0B;

/// The SX127x register RegLna, the LNA gain and current.
pub const PAMOJA_SX127X_REG_LNA: u8 = 0x0C;

/// The SX127x register RegFifoAddrPtr, where the next RegFifo access lands.
pub const PAMOJA_SX127X_REG_FIFO_ADDR_PTR: u8 = 0x0D;

/// The SX127x register RegFifoTxBaseAddr, where a transmitted payload starts.
pub const PAMOJA_SX127X_REG_FIFO_TX_BASE_ADDR: u8 = 0x0E;

/// The SX127x register RegFifoRxBaseAddr, where received payloads start.
pub const PAMOJA_SX127X_REG_FIFO_RX_BASE_ADDR: u8 = 0x0F;

/// The SX127x register RegFifoRxCurrentAddr, where the last packet starts.
pub const PAMOJA_SX127X_REG_FIFO_RX_CURRENT_ADDR: u8 = 0x10;

/// The SX127x register RegIrqFlagsMask, the interrupts masked off.
pub const PAMOJA_SX127X_REG_IRQ_FLAGS_MASK: u8 = 0x11;

/// The SX127x register RegIrqFlags, the interrupts raised.
pub const PAMOJA_SX127X_REG_IRQ_FLAGS: u8 = 0x12;

/// The SX127x register RegRxNbBytes, the payload length of the last packet.
pub const PAMOJA_SX127X_REG_RX_NB_BYTES: u8 = 0x13;

/// The SX127x register RegModemStat, the live state of the modem.
pub const PAMOJA_SX127X_REG_MODEM_STAT: u8 = 0x18;

/// The SX127x register RegPktSnrValue, the SNR of the last packet.
pub const PAMOJA_SX127X_REG_PKT_SNR_VALUE: u8 = 0x19;

/// The SX127x register RegPktRssiValue, the RSSI of the last packet.
pub const PAMOJA_SX127X_REG_PKT_RSSI_VALUE: u8 = 0x1A;

/// The SX127x register RegRssiValue, the RSSI heard right now.
pub const PAMOJA_SX127X_REG_RSSI_VALUE: u8 = 0x1B;

/// The SX127x register RegHopChannel, the PLL lock and the CRC the header announced.
pub const PAMOJA_SX127X_REG_HOP_CHANNEL: u8 = 0x1C;

/// The SX127x register RegModemConfig1: bandwidth, coding rate, and header mode.
pub const PAMOJA_SX127X_REG_MODEM_CONFIG_1: u8 = 0x1D;

/// The SX127x register RegModemConfig2: spreading factor, CRC, and the timeout's top bits.
pub const PAMOJA_SX127X_REG_MODEM_CONFIG_2: u8 = 0x1E;

/// The SX127x register RegSymbTimeoutLsb, the low byte of the symbol timeout.
pub const PAMOJA_SX127X_REG_SYMB_TIMEOUT_LSB: u8 = 0x1F;

/// The SX127x register RegPreambleMsb, the high byte of the preamble length.
pub const PAMOJA_SX127X_REG_PREAMBLE_MSB: u8 = 0x20;

/// The SX127x register RegPreambleLsb, the low byte of the preamble length.
pub const PAMOJA_SX127X_REG_PREAMBLE_LSB: u8 = 0x21;

/// The SX127x register RegPayloadLength, the payload length to send.
pub const PAMOJA_SX127X_REG_PAYLOAD_LENGTH: u8 = 0x22;

/// The SX127x register RegMaxPayloadLength, the longest payload accepted.
pub const PAMOJA_SX127X_REG_MAX_PAYLOAD_LENGTH: u8 = 0x23;

/// The SX127x register RegModemConfig3: low data rate optimization and the AGC.
pub const PAMOJA_SX127X_REG_MODEM_CONFIG_3: u8 = 0x26;

/// The SX127x register RegRssiWideband, a wideband RSSI sample.
pub const PAMOJA_SX127X_REG_RSSI_WIDEBAND: u8 = 0x2C;

/// The SX127x register RegIfFreq2, which the spurious reception erratum sets.
pub const PAMOJA_SX127X_REG_IF_FREQ_2: u8 = 0x2F;

/// The SX127x register RegIfFreq1, which the spurious reception erratum clears.
pub const PAMOJA_SX127X_REG_IF_FREQ_1: u8 = 0x30;

/// The SX127x register RegDetectOptimize: the automatic IF and detection optimization.
pub const PAMOJA_SX127X_REG_DETECT_OPTIMIZE: u8 = 0x31;

/// The SX127x register RegInvertIQ, the IQ polarity of each path.
pub const PAMOJA_SX127X_REG_INVERT_IQ: u8 = 0x33;

/// The SX127x register RegHighBwOptimize1, which the 500 kHz erratum sets.
pub const PAMOJA_SX127X_REG_HIGH_BW_OPTIMIZE_1: u8 = 0x36;

/// The SX127x register RegDetectionThreshold, the LoRa detection threshold.
pub const PAMOJA_SX127X_REG_DETECTION_THRESHOLD: u8 = 0x37;

/// The SX127x register RegSyncWord, the LoRa sync word.
pub const PAMOJA_SX127X_REG_SYNC_WORD: u8 = 0x39;

/// The SX127x register RegHighBwOptimize2, which the 500 kHz erratum sets.
pub const PAMOJA_SX127X_REG_HIGH_BW_OPTIMIZE_2: u8 = 0x3A;

/// The SX127x register RegInvertIQ2, which completes an IQ inversion.
pub const PAMOJA_SX127X_REG_INVERT_IQ_2: u8 = 0x3B;

/// The SX127x register RegImageCal, at the address of RegInvertIQ2 on the FSK page.
pub const PAMOJA_SX127X_REG_IMAGE_CAL: u8 = 0x3B;

/// The SX127x register RegDioMapping1, the events DIO0 to DIO3 signal.
pub const PAMOJA_SX127X_REG_DIO_MAPPING_1: u8 = 0x40;

/// The SX127x register RegDioMapping2, the events DIO4 and DIO5 signal.
pub const PAMOJA_SX127X_REG_DIO_MAPPING_2: u8 = 0x41;

/// The SX127x register RegVersion, the silicon revision.
pub const PAMOJA_SX127X_REG_VERSION: u8 = 0x42;

/// The SX127x register RegTcxo, a crystal or a TCXO on XTA.
pub const PAMOJA_SX127X_REG_TCXO: u8 = 0x4B;

/// The SX127x register RegPaDac, the +20 dBm setting of PA_BOOST.
pub const PAMOJA_SX127X_REG_PA_DAC: u8 = 0x4D;

/// The SX127x operating mode code for sleep, the only mode that may switch between LoRa and FSK.
pub const PAMOJA_SX127X_MODE_SLEEP: u8 = 0;

/// The SX127x operating mode code for standby.
pub const PAMOJA_SX127X_MODE_STANDBY: u8 = 1;

/// The SX127x operating mode code for frequency synthesis for transmit.
pub const PAMOJA_SX127X_MODE_FS_TX: u8 = 2;

/// The SX127x operating mode code for transmit one packet.
pub const PAMOJA_SX127X_MODE_TX: u8 = 3;

/// The SX127x operating mode code for frequency synthesis for receive.
pub const PAMOJA_SX127X_MODE_FS_RX: u8 = 4;

/// The SX127x operating mode code for receive packet after packet.
pub const PAMOJA_SX127X_MODE_RX_CONTINUOUS: u8 = 5;

/// The SX127x operating mode code for receive one packet or time out.
pub const PAMOJA_SX127X_MODE_RX_SINGLE: u8 = 6;

/// The SX127x operating mode code for channel activity detection.
pub const PAMOJA_SX127X_MODE_CAD: u8 = 7;

/// The SX127x interrupt flag: channel activity detection heard a LoRa signal.
pub const PAMOJA_SX127X_IRQ_CAD_DETECTED: u8 = 0x01;

/// The SX127x interrupt flag: frequency hopping moved to the next channel.
pub const PAMOJA_SX127X_IRQ_FHSS_CHANGE_CHANNEL: u8 = 0x02;

/// The SX127x interrupt flag: channel activity detection finished.
pub const PAMOJA_SX127X_IRQ_CAD_DONE: u8 = 0x04;

/// The SX127x interrupt flag: the payload has been transmitted.
pub const PAMOJA_SX127X_IRQ_TX_DONE: u8 = 0x08;

/// The SX127x interrupt flag: a valid header was received.
pub const PAMOJA_SX127X_IRQ_VALID_HEADER: u8 = 0x10;

/// The SX127x interrupt flag: the payload failed its CRC.
pub const PAMOJA_SX127X_IRQ_PAYLOAD_CRC_ERROR: u8 = 0x20;

/// The SX127x interrupt flag: a packet has been received.
pub const PAMOJA_SX127X_IRQ_RX_DONE: u8 = 0x40;

/// The SX127x interrupt flag: a single reception timed out.
pub const PAMOJA_SX127X_IRQ_RX_TIMEOUT: u8 = 0x80;

/// The SX127x interrupt flag: every interrupt, which writing back clears.
pub const PAMOJA_SX127X_IRQ_ALL: u8 = 0xFF;

// The header generator does not read the crates this one depends on, so these carry their
// value rather than the name of the constant that defines it.
const _: () = assert!(PAMOJA_SX127X_VERSION == sx127x_register::VERSION_SX1276);
const _: () = assert!(PAMOJA_SX127X_WRITE == sx127x_register::WRITE);
const _: () = assert!(PAMOJA_SX127X_DIO0_RX_DONE == sx127x_config::DIO0_RX_DONE);
const _: () = assert!(PAMOJA_SX127X_DIO0_TX_DONE == sx127x_config::DIO0_TX_DONE);
const _: () = assert!(PAMOJA_SX127X_DIO0_CAD_DONE == sx127x_config::DIO0_CAD_DONE);
const _: () = assert!(PAMOJA_SX127X_PA_DAC_DEFAULT == sx127x_config::PA_DAC_DEFAULT);
const _: () = assert!(PAMOJA_SX127X_PA_DAC_HIGH_POWER == sx127x_config::PA_DAC_HIGH_POWER);
const _: () = assert!(PAMOJA_SX127X_IMAGE_CAL_START == sx127x_config::IMAGE_CAL_START);
const _: () = assert!(PAMOJA_SX127X_IMAGE_CAL_RUNNING == sx127x_config::IMAGE_CAL_RUNNING);
const _: () = assert!(PAMOJA_SX127X_SYNC_WORD_PUBLIC == sx127x_config::SyncWord::Public.to_byte());
const _: () =
    assert!(PAMOJA_SX127X_SYNC_WORD_PRIVATE == sx127x_config::SyncWord::Private.to_byte());
const _: () = assert!(PAMOJA_SX127X_LNA_BOOSTED == sx127x_config::LNA_BOOSTED);
const _: () = assert!(PAMOJA_SX127X_TCXO_INPUT_ON == sx127x_config::TCXO_INPUT_ON);
const _: () = assert!(PAMOJA_SX127X_REG_FIFO == sx127x_register::FIFO);
const _: () = assert!(PAMOJA_SX127X_REG_OP_MODE == sx127x_register::OP_MODE);
const _: () = assert!(PAMOJA_SX127X_REG_FRF_MSB == sx127x_register::FRF_MSB);
const _: () = assert!(PAMOJA_SX127X_REG_FRF_MID == sx127x_register::FRF_MID);
const _: () = assert!(PAMOJA_SX127X_REG_FRF_LSB == sx127x_register::FRF_LSB);
const _: () = assert!(PAMOJA_SX127X_REG_PA_CONFIG == sx127x_register::PA_CONFIG);
const _: () = assert!(PAMOJA_SX127X_REG_PA_RAMP == sx127x_register::PA_RAMP);
const _: () = assert!(PAMOJA_SX127X_REG_OCP == sx127x_register::OCP);
const _: () = assert!(PAMOJA_SX127X_REG_LNA == sx127x_register::LNA);
const _: () = assert!(PAMOJA_SX127X_REG_FIFO_ADDR_PTR == sx127x_register::FIFO_ADDR_PTR);
const _: () = assert!(PAMOJA_SX127X_REG_FIFO_TX_BASE_ADDR == sx127x_register::FIFO_TX_BASE_ADDR);
const _: () = assert!(PAMOJA_SX127X_REG_FIFO_RX_BASE_ADDR == sx127x_register::FIFO_RX_BASE_ADDR);
const _: () =
    assert!(PAMOJA_SX127X_REG_FIFO_RX_CURRENT_ADDR == sx127x_register::FIFO_RX_CURRENT_ADDR);
const _: () = assert!(PAMOJA_SX127X_REG_IRQ_FLAGS_MASK == sx127x_register::IRQ_FLAGS_MASK);
const _: () = assert!(PAMOJA_SX127X_REG_IRQ_FLAGS == sx127x_register::IRQ_FLAGS);
const _: () = assert!(PAMOJA_SX127X_REG_RX_NB_BYTES == sx127x_register::RX_NB_BYTES);
const _: () = assert!(PAMOJA_SX127X_REG_MODEM_STAT == sx127x_register::MODEM_STAT);
const _: () = assert!(PAMOJA_SX127X_REG_PKT_SNR_VALUE == sx127x_register::PKT_SNR_VALUE);
const _: () = assert!(PAMOJA_SX127X_REG_PKT_RSSI_VALUE == sx127x_register::PKT_RSSI_VALUE);
const _: () = assert!(PAMOJA_SX127X_REG_RSSI_VALUE == sx127x_register::RSSI_VALUE);
const _: () = assert!(PAMOJA_SX127X_REG_HOP_CHANNEL == sx127x_register::HOP_CHANNEL);
const _: () = assert!(PAMOJA_SX127X_REG_MODEM_CONFIG_1 == sx127x_register::MODEM_CONFIG_1);
const _: () = assert!(PAMOJA_SX127X_REG_MODEM_CONFIG_2 == sx127x_register::MODEM_CONFIG_2);
const _: () = assert!(PAMOJA_SX127X_REG_SYMB_TIMEOUT_LSB == sx127x_register::SYMB_TIMEOUT_LSB);
const _: () = assert!(PAMOJA_SX127X_REG_PREAMBLE_MSB == sx127x_register::PREAMBLE_MSB);
const _: () = assert!(PAMOJA_SX127X_REG_PREAMBLE_LSB == sx127x_register::PREAMBLE_LSB);
const _: () = assert!(PAMOJA_SX127X_REG_PAYLOAD_LENGTH == sx127x_register::PAYLOAD_LENGTH);
const _: () = assert!(PAMOJA_SX127X_REG_MAX_PAYLOAD_LENGTH == sx127x_register::MAX_PAYLOAD_LENGTH);
const _: () = assert!(PAMOJA_SX127X_REG_MODEM_CONFIG_3 == sx127x_register::MODEM_CONFIG_3);
const _: () = assert!(PAMOJA_SX127X_REG_RSSI_WIDEBAND == sx127x_register::RSSI_WIDEBAND);
const _: () = assert!(PAMOJA_SX127X_REG_IF_FREQ_2 == sx127x_register::IF_FREQ_2);
const _: () = assert!(PAMOJA_SX127X_REG_IF_FREQ_1 == sx127x_register::IF_FREQ_1);
const _: () = assert!(PAMOJA_SX127X_REG_DETECT_OPTIMIZE == sx127x_register::DETECT_OPTIMIZE);
const _: () = assert!(PAMOJA_SX127X_REG_INVERT_IQ == sx127x_register::INVERT_IQ);
const _: () = assert!(PAMOJA_SX127X_REG_HIGH_BW_OPTIMIZE_1 == sx127x_register::HIGH_BW_OPTIMIZE_1);
const _: () =
    assert!(PAMOJA_SX127X_REG_DETECTION_THRESHOLD == sx127x_register::DETECTION_THRESHOLD);
const _: () = assert!(PAMOJA_SX127X_REG_SYNC_WORD == sx127x_register::SYNC_WORD);
const _: () = assert!(PAMOJA_SX127X_REG_HIGH_BW_OPTIMIZE_2 == sx127x_register::HIGH_BW_OPTIMIZE_2);
const _: () = assert!(PAMOJA_SX127X_REG_INVERT_IQ_2 == sx127x_register::INVERT_IQ_2);
const _: () = assert!(PAMOJA_SX127X_REG_IMAGE_CAL == sx127x_register::IMAGE_CAL);
const _: () = assert!(PAMOJA_SX127X_REG_DIO_MAPPING_1 == sx127x_register::DIO_MAPPING_1);
const _: () = assert!(PAMOJA_SX127X_REG_DIO_MAPPING_2 == sx127x_register::DIO_MAPPING_2);
const _: () = assert!(PAMOJA_SX127X_REG_VERSION == sx127x_register::VERSION);
const _: () = assert!(PAMOJA_SX127X_REG_TCXO == sx127x_register::TCXO);
const _: () = assert!(PAMOJA_SX127X_REG_PA_DAC == sx127x_register::PA_DAC);
const _: () = assert!(PAMOJA_SX127X_MODE_SLEEP == Sx127xMode::Sleep.code());
const _: () = assert!(PAMOJA_SX127X_MODE_STANDBY == Sx127xMode::Standby.code());
const _: () = assert!(PAMOJA_SX127X_MODE_FS_TX == Sx127xMode::FsTx.code());
const _: () = assert!(PAMOJA_SX127X_MODE_TX == Sx127xMode::Tx.code());
const _: () = assert!(PAMOJA_SX127X_MODE_FS_RX == Sx127xMode::FsRx.code());
const _: () = assert!(PAMOJA_SX127X_MODE_RX_CONTINUOUS == Sx127xMode::RxContinuous.code());
const _: () = assert!(PAMOJA_SX127X_MODE_RX_SINGLE == Sx127xMode::RxSingle.code());
const _: () = assert!(PAMOJA_SX127X_MODE_CAD == Sx127xMode::Cad.code());
const _: () = assert!(PAMOJA_SX127X_IRQ_CAD_DETECTED == IrqFlags::CAD_DETECTED.bits());
const _: () =
    assert!(PAMOJA_SX127X_IRQ_FHSS_CHANGE_CHANNEL == IrqFlags::FHSS_CHANGE_CHANNEL.bits());
const _: () = assert!(PAMOJA_SX127X_IRQ_CAD_DONE == IrqFlags::CAD_DONE.bits());
const _: () = assert!(PAMOJA_SX127X_IRQ_TX_DONE == IrqFlags::TX_DONE.bits());
const _: () = assert!(PAMOJA_SX127X_IRQ_VALID_HEADER == IrqFlags::VALID_HEADER.bits());
const _: () = assert!(PAMOJA_SX127X_IRQ_PAYLOAD_CRC_ERROR == IrqFlags::PAYLOAD_CRC_ERROR.bits());
const _: () = assert!(PAMOJA_SX127X_IRQ_RX_DONE == IrqFlags::RX_DONE.bits());
const _: () = assert!(PAMOJA_SX127X_IRQ_RX_TIMEOUT == IrqFlags::RX_TIMEOUT.bits());
const _: () = assert!(PAMOJA_SX127X_IRQ_ALL == IrqFlags::ALL.bits());

/// The bytes of one SX126x command, in the order the chip receives them.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xCommand {
    /// The opcode and its parameters; only the first `len` bytes are the command.
    pub bytes: [u8; PAMOJA_SX126X_COMMAND_MAX],
    /// How many bytes the command takes.
    pub len: u8,
}

/// A command the chip answers in the same SPI transaction.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xQuery {
    /// The bytes to send, ending with the NOP during which the status byte comes back.
    pub command: PamojaSx126xCommand,
    /// How many bytes of answer to read after them.
    pub answer_len: u32,
}

/// The amplifier configuration and power setting that produce an output power, from
/// Table 13-21 of the SX1261/2 datasheet.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xTxPower {
    /// paDutyCycle, the conduction angle of the amplifier.
    pub pa_duty_cycle: u8,
    /// hpMax, the size of the SX1262 amplifier; no effect on the SX1261.
    pub hp_max: u8,
    /// deviceSel: `0` for the SX1262 and the LLCC68, `1` for the SX1261.
    pub device_sel: u8,
    /// paLut, reserved and always `1`.
    pub pa_lut: u8,
    /// The power byte of SetTxParams, in dBm.
    pub setting_dbm: i8,
}

/// The mode an SX126x reports in its status byte.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaSx126xChipMode {
    /// A value the datasheet leaves unused.
    Other = 0,
    /// Standby on the 13 MHz RC oscillator.
    StandbyRc = 2,
    /// Standby on the 32 MHz crystal.
    StandbyXosc = 3,
    /// Frequency synthesis.
    Fs = 4,
    /// Receiving.
    Rx = 5,
    /// Transmitting.
    Tx = 6,
}

/// How the last command went, as an SX126x reports it in its status byte.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaSx126xCommandStatus {
    /// A value the datasheet leaves unused, which includes a command that went well.
    Other = 0,
    /// A packet has been received and waits in the data buffer.
    DataAvailable = 2,
    /// A command timed out.
    Timeout = 3,
    /// A command could not be processed.
    ProcessingError = 4,
    /// A command failed to execute.
    ExecutionFailure = 5,
    /// A transmission has finished.
    TxDone = 6,
}

/// A decoded SX126x status byte.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xStatus {
    /// The mode the chip is in.
    pub chip_mode: PamojaSx126xChipMode,
    /// How the last command went.
    pub command_status: PamojaSx126xCommandStatus,
    /// `true` for a timeout, a processing error, or an execution failure.
    pub error: bool,
}

/// The signal levels of the last LoRa packet received, in hundredths of a decibel.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xPacketStatus {
    /// The RSSI averaged over the packet, in hundredths of a dBm.
    pub rssi_centi_dbm: i32,
    /// The estimated signal-to-noise ratio, in hundredths of a dB.
    pub snr_centi_db: i32,
    /// The estimated RSSI of the LoRa signal after despreading, in hundredths of a dBm.
    pub signal_rssi_centi_dbm: i32,
}

/// Where a received payload sits in the data buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSx126xRxBufferStatus {
    /// The length of the payload in bytes.
    pub payload_len: u8,
    /// The buffer offset of its first byte.
    pub start: u8,
}

/// An opaque handle to a duty-cycle guard.
///
/// Record each transmission with [`pamoja_radio_duty_cycle_transmitted`], ask
/// [`pamoja_radio_duty_cycle_ready`] before the next, and release it with
/// [`pamoja_radio_duty_cycle_free`].
pub struct PamojaRadioDutyCycle {
    guard: DutyCycle,
}

/// Returns the word SetRfFrequency takes for a frequency.
///
/// # Arguments
///
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The frequency times 2^25 over the 32 MHz crystal, rounded to the nearest step.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_frequency_word(frequency_hz: u32) -> u32 {
    config::frequency_word(frequency_hz)
}

/// Returns the 24-bit timeout word SetTx and SetRx take for a duration.
///
/// # Arguments
///
/// * `timeout_us` - the duration in microseconds.
///
/// # Returns
///
/// The number of 15.625 us steps; a nonzero duration never becomes the zero word that
/// disables the timeout, and nothing reaches [`PAMOJA_SX126X_RX_CONTINUOUS`].
#[no_mangle]
pub extern "C" fn pamoja_sx126x_timeout_steps(timeout_us: u64) -> u32 {
    config::timeout_steps(timeout_us)
}

/// Returns the two CalibrateImage codes that cover a band.
///
/// # Arguments
///
/// * `low_hz` - the lower edge of the band in hertz.
/// * `high_hz` - the upper edge of the band in hertz.
///
/// # Returns
///
/// `freq1` in the high byte and `freq2` in the low byte.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_image_calibration(low_hz: u32, high_hz: u32) -> u16 {
    u16::from_be_bytes(config::image_calibration(low_hz, high_hz))
}

/// Returns the shortest amplifier ramp time the chip offers that lasts at least a duration.
///
/// # Arguments
///
/// * `at_least_us` - the least ramp time wanted, in microseconds.
///
/// # Returns
///
/// The ramp time in microseconds, one of the eight Table 13-41 gives.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_ramp_time_us(at_least_us: u32) -> u32 {
    RampTime::at_least(at_least_us).micros()
}

/// Chooses the amplifier settings for an output power.
///
/// # Arguments
///
/// * `high_power` - `true` for the high power amplifier of the SX1262 and the LLCC68,
///   `false` for the low power amplifier of the SX1261.
/// * `output_dbm` - the output power wanted at the antenna port.
///
/// # Returns
///
/// The configuration and the power setting, clamped to what the amplifier allows.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_tx_power_for_output(
    high_power: bool,
    output_dbm: i8,
) -> PamojaSx126xTxPower {
    power_of(TxPower::for_output(amplifier(high_power), output_dbm))
}

/// Chooses the amplifier settings that keep a link's EIRP at or under a ceiling.
///
/// # Arguments
///
/// * `high_power` - `true` for the high power amplifier, `false` for the low power one.
/// * `budget` - the link budget, whose transmitting antenna and cable apply.
/// * `eirp_ceiling_centi_dbm` - the EIRP limit, in hundredths of a dBm.
///
/// # Returns
///
/// The configuration and the power setting, rounded down to whole decibels.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_tx_power_under_ceiling(
    high_power: bool,
    budget: PamojaLoraLinkBudget,
    eirp_ceiling_centi_dbm: i32,
) -> PamojaSx126xTxPower {
    power_of(TxPower::under_ceiling(
        amplifier(high_power),
        &link_budget(budget),
        Decibels::from_hundredths(eirp_ceiling_centi_dbm),
    ))
}

/// SetStandby into STDBY_RC.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_standby() -> PamojaSx126xCommand {
    command_of(command::set_standby(StandbyMode::Rc))
}

/// SetPacketType for LoRa.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_packet_type_lora() -> PamojaSx126xCommand {
    command_of(command::set_packet_type(PacketType::Lora))
}

/// SetRfFrequency for a carrier frequency.
///
/// # Arguments
///
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_rf_frequency(frequency_hz: u32) -> PamojaSx126xCommand {
    command_of(command::set_rf_frequency(config::frequency_word(
        frequency_hz,
    )))
}

/// CalibrateImage over a band.
///
/// # Arguments
///
/// * `low_hz` - the lower edge of the band in hertz.
/// * `high_hz` - the upper edge of the band in hertz.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_calibrate_image(low_hz: u32, high_hz: u32) -> PamojaSx126xCommand {
    command_of(command::calibrate_image(config::image_calibration(
        low_hz, high_hz,
    )))
}

/// SetPaConfig for a power setting.
///
/// # Arguments
///
/// * `power` - the settings from [`pamoja_sx126x_tx_power_for_output`] or
///   [`pamoja_sx126x_tx_power_under_ceiling`].
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_pa_config(power: PamojaSx126xTxPower) -> PamojaSx126xCommand {
    command_of(command::set_pa_config(tx_power(power).pa))
}

/// SetTxParams for a power setting and a ramp time.
///
/// # Arguments
///
/// * `power` - the power settings.
/// * `ramp_us` - the least amplifier ramp time wanted, in microseconds; the chip takes the
///   shortest of its eight ramp times that lasts at least this long.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_tx_params(
    power: PamojaSx126xTxPower,
    ramp_us: u32,
) -> PamojaSx126xCommand {
    command_of(command::set_tx_params(
        power.setting_dbm,
        RampTime::at_least(ramp_us),
    ))
}

/// SetModulationParams for a LoRa link.
///
/// # Arguments
///
/// * `link` - the link settings.
/// * `out_command` - receives the command.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with `*out_command` set, or [`PamojaStatus::InvalidArgument`] if
/// the link's bandwidth is not one the SX126x offers or `out_command` is null.
///
/// # Safety
///
/// `out_command` must point to a writable [`PamojaSx126xCommand`], or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sx126x_set_lora_modulation_params(
    link: PamojaLoraLink,
    out_command: *mut PamojaSx126xCommand,
) -> PamojaStatus {
    if out_command.is_null() {
        set_last_error("out_command must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match LoraModulation::from_link(&settings(link)) {
        Some(modulation) => {
            *out_command = command_of(command::set_lora_modulation_params(modulation));
            PamojaStatus::Ok
        }
        None => {
            set_last_error(format!(
                "the SX126x has no {} Hz LoRa bandwidth",
                link.bandwidth_hz
            ));
            PamojaStatus::InvalidArgument
        }
    }
}

/// SetPacketParams for a LoRa link and a payload.
///
/// # Arguments
///
/// * `link` - the link settings, whose preamble, header, and CRC the frame uses.
/// * `payload_len` - the payload length to send, or the most a receiver accepts.
/// * `invert_iq` - `true` for inverted IQ, as a LoRaWAN gateway sends downlinks.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_lora_packet_params(
    link: PamojaLoraLink,
    payload_len: u8,
    invert_iq: bool,
) -> PamojaSx126xCommand {
    command_of(command::set_lora_packet_params(LoraPacket::from_link(
        &settings(link),
        payload_len,
        invert_iq,
    )))
}

/// SetDioIrqParams: which interrupts are enabled and which DIO line each raises.
///
/// # Arguments
///
/// * `irq` - the interrupts to enable, as `PAMOJA_SX126X_IRQ_*` bits.
/// * `dio1` - the interrupts routed to DIO1.
/// * `dio2` - the interrupts routed to DIO2.
/// * `dio3` - the interrupts routed to DIO3.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_dio_irq_params(
    irq: u16,
    dio1: u16,
    dio2: u16,
    dio3: u16,
) -> PamojaSx126xCommand {
    command_of(command::set_dio_irq_params(
        Irq::from_bits(irq),
        Irq::from_bits(dio1),
        Irq::from_bits(dio2),
        Irq::from_bits(dio3),
    ))
}

/// ClearIrqStatus for a set of interrupts.
///
/// # Arguments
///
/// * `irq` - the interrupts to clear, as `PAMOJA_SX126X_IRQ_*` bits.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_clear_irq_status(irq: u16) -> PamojaSx126xCommand {
    command_of(command::clear_irq_status(Irq::from_bits(irq)))
}

/// SetTx with a timeout.
///
/// # Arguments
///
/// * `timeout_us` - how long the chip may transmit before it raises TIMEOUT, in
///   microseconds; `0` disables the timeout.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_tx(timeout_us: u64) -> PamojaSx126xCommand {
    command_of(command::set_tx(config::timeout_steps(timeout_us)))
}

/// SetRx with a timeout.
///
/// # Arguments
///
/// * `timeout_us` - how long the chip listens for a packet to start, in microseconds;
///   `0` listens for a single packet with no timeout.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_rx(timeout_us: u64) -> PamojaSx126xCommand {
    command_of(command::set_rx(config::timeout_steps(timeout_us)))
}

/// SetRx in continuous mode, receiving packet after packet until another command.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_rx_continuous() -> PamojaSx126xCommand {
    command_of(command::set_rx(config::RX_CONTINUOUS))
}

/// SetSleep, without an RTC wake-up.
///
/// # Arguments
///
/// * `warm_start` - `true` to keep the configuration in retention.
///
/// # Returns
///
/// The command.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_set_sleep(warm_start: bool) -> PamojaSx126xCommand {
    command_of(command::set_sleep(warm_start, false))
}

/// The start of a WriteRegister transaction; the register values follow it in the same
/// transaction.
///
/// # Arguments
///
/// * `address` - the first register's address.
///
/// # Returns
///
/// The opcode and the address.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_write_register_header(address: u16) -> PamojaSx126xCommand {
    command_of(command::write_register(address))
}

/// The start of a WriteBuffer transaction; the payload follows it in the same transaction.
///
/// # Arguments
///
/// * `offset` - where in the data buffer the first byte goes.
///
/// # Returns
///
/// The opcode and the offset.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_write_buffer_header(offset: u8) -> PamojaSx126xCommand {
    command_of(command::write_buffer(offset))
}

/// GetStatus, answered by the status byte.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_status() -> PamojaSx126xQuery {
    query_of(command::get_status())
}

/// GetIrqStatus, answered by the two IRQ bytes.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_irq_status() -> PamojaSx126xQuery {
    query_of(command::get_irq_status())
}

/// GetRxBufferStatus, answered by the payload length and its offset.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_rx_buffer_status() -> PamojaSx126xQuery {
    query_of(command::get_rx_buffer_status())
}

/// GetPacketStatus, answered by the three LoRa signal level bytes.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_packet_status() -> PamojaSx126xQuery {
    query_of(command::get_packet_status())
}

/// GetRssiInst, answered by the instantaneous RSSI byte.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_rssi_inst() -> PamojaSx126xQuery {
    query_of(command::get_rssi_inst())
}

/// GetDeviceErrors, answered by the two device error bytes.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_get_device_errors() -> PamojaSx126xQuery {
    query_of(command::get_device_errors())
}

/// ReadRegister for consecutive registers.
///
/// # Arguments
///
/// * `address` - the first register's address.
/// * `len` - how many registers to read.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_read_register(address: u16, len: u8) -> PamojaSx126xQuery {
    query_of(command::read_register(address, usize::from(len)))
}

/// ReadBuffer for a run of the data buffer.
///
/// # Arguments
///
/// * `offset` - where in the buffer the first byte is.
/// * `len` - how many bytes to read.
///
/// # Returns
///
/// The query.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_read_buffer(offset: u8, len: u8) -> PamojaSx126xQuery {
    query_of(command::read_buffer(offset, usize::from(len)))
}

/// Decodes a status byte.
///
/// # Arguments
///
/// * `byte` - the status byte.
///
/// # Returns
///
/// The chip mode, how the last command went, and whether that was an error.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_status_from_byte(byte: u8) -> PamojaSx126xStatus {
    let status = Status::from_byte(byte);
    PamojaSx126xStatus {
        chip_mode: chip_mode(status.chip_mode),
        command_status: command_status(status.command_status),
        error: status.is_error(),
    }
}

/// Decodes a GetIrqStatus answer.
///
/// # Arguments
///
/// * `high` - the first answer byte, IrqStatus(15:8).
/// * `low` - the second answer byte, IrqStatus(7:0).
///
/// # Returns
///
/// The pending interrupts, as `PAMOJA_SX126X_IRQ_*` bits.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_irq_from_bytes(high: u8, low: u8) -> u16 {
    Irq::from_bytes([high, low]).bits()
}

/// Decodes a GetDeviceErrors answer.
///
/// # Arguments
///
/// * `high` - the first answer byte.
/// * `low` - the second answer byte.
///
/// # Returns
///
/// The flagged errors, as `PAMOJA_SX126X_ERROR_*` bits.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_device_errors_from_bytes(high: u8, low: u8) -> u16 {
    DeviceErrors::from_bytes([high, low]).bits()
}

/// Decodes a LoRa GetPacketStatus answer.
///
/// # Arguments
///
/// * `rssi_pkt` - RssiPkt.
/// * `snr_pkt` - SnrPkt.
/// * `signal_rssi_pkt` - SignalRssiPkt.
///
/// # Returns
///
/// The three signal levels, exact to a hundredth of a decibel.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_packet_status_from_bytes(
    rssi_pkt: u8,
    snr_pkt: u8,
    signal_rssi_pkt: u8,
) -> PamojaSx126xPacketStatus {
    let status = PacketStatus::from_bytes([rssi_pkt, snr_pkt, signal_rssi_pkt]);
    PamojaSx126xPacketStatus {
        rssi_centi_dbm: status.rssi_dbm.hundredths(),
        snr_centi_db: status.snr_db.hundredths(),
        signal_rssi_centi_dbm: status.signal_rssi_dbm.hundredths(),
    }
}

/// Decodes a GetRxBufferStatus answer.
///
/// # Arguments
///
/// * `payload_len` - PayloadLengthRx.
/// * `start` - RxStartBufferPointer.
///
/// # Returns
///
/// The payload length and where it starts in the data buffer.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_rx_buffer_status_from_bytes(
    payload_len: u8,
    start: u8,
) -> PamojaSx126xRxBufferStatus {
    let status = RxBufferStatus::from_bytes([payload_len, start]);
    PamojaSx126xRxBufferStatus {
        payload_len: status.payload_len,
        start: status.start,
    }
}

/// Decodes a GetRssiInst answer.
///
/// # Arguments
///
/// * `byte` - RssiInst.
///
/// # Returns
///
/// The instantaneous RSSI in hundredths of a dBm.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_rssi_inst_centi_dbm(byte: u8) -> i32 {
    rssi_inst_dbm(byte).hundredths()
}

/// Creates a duty-cycle guard, ready to transmit at once.
///
/// # Arguments
///
/// * `permille` - the limit in parts per thousand, so `10` is 1%; `0` forbids
///   transmitting and `1000` or more imposes no silence.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_radio_duty_cycle_free`].
#[no_mangle]
pub extern "C" fn pamoja_radio_duty_cycle_new(permille: u32) -> *mut PamojaRadioDutyCycle {
    Box::into_raw(Box::new(PamojaRadioDutyCycle {
        guard: DutyCycle::new(permille),
    }))
}

/// Records a transmission and the silence it owes.
///
/// # Arguments
///
/// * `guard` - the duty-cycle guard.
/// * `started_us` - when the transmission started, in microseconds on the caller's clock.
/// * `link` - the settings the frame was sent with.
/// * `payload_len` - the payload length in bytes.
///
/// # Returns
///
/// The frame's time on air in microseconds, or 0 if `guard` is null.
///
/// # Safety
///
/// `guard` must be a live handle from [`pamoja_radio_duty_cycle_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_transmitted(
    guard: *mut PamojaRadioDutyCycle,
    started_us: u64,
    link: PamojaLoraLink,
    payload_len: usize,
) -> u64 {
    if guard.is_null() {
        return 0;
    }
    (*guard)
        .guard
        .transmitted(started_us, &settings(link), payload_len)
}

/// Returns how long the radio must still stay silent.
///
/// # Arguments
///
/// * `guard` - the duty-cycle guard.
/// * `now_us` - the current time in microseconds on the caller's clock.
///
/// # Returns
///
/// The remaining silence in microseconds, zero when a transmission may start, or
/// `UINT64_MAX` when the limit forbids transmitting or `guard` is null.
///
/// # Safety
///
/// `guard` must be a live handle from [`pamoja_radio_duty_cycle_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_wait_us(
    guard: *const PamojaRadioDutyCycle,
    now_us: u64,
) -> u64 {
    if guard.is_null() {
        return u64::MAX;
    }
    (*guard).guard.wait_us(now_us)
}

/// Reports whether a transmission may start now.
///
/// # Arguments
///
/// * `guard` - the duty-cycle guard.
/// * `now_us` - the current time in microseconds on the caller's clock.
///
/// # Returns
///
/// `true` once the silence the last transmission owed has passed, or `false` if `guard`
/// is null.
///
/// # Safety
///
/// `guard` must be a live handle from [`pamoja_radio_duty_cycle_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_ready(
    guard: *const PamojaRadioDutyCycle,
    now_us: u64,
) -> bool {
    !guard.is_null() && (*guard).guard.ready(now_us)
}

/// Returns the earliest time the next transmission may start.
///
/// # Arguments
///
/// * `guard` - the duty-cycle guard.
///
/// # Returns
///
/// A time in microseconds on the caller's clock, or `UINT64_MAX` when the limit forbids
/// transmitting or `guard` is null.
///
/// # Safety
///
/// `guard` must be a live handle from [`pamoja_radio_duty_cycle_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_earliest_us(
    guard: *const PamojaRadioDutyCycle,
) -> u64 {
    if guard.is_null() {
        return u64::MAX;
    }
    (*guard).guard.earliest_us()
}

/// Releases a duty-cycle guard handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `guard` must be a handle from [`pamoja_radio_duty_cycle_new`] that has not already been
/// freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_radio_duty_cycle_free(guard: *mut PamojaRadioDutyCycle) {
    if !guard.is_null() {
        drop(Box::from_raw(guard));
    }
}

/// Copies a command's bytes into the value that crosses the boundary.
fn command_of(command: Command) -> PamojaSx126xCommand {
    let bytes = command.as_bytes();
    let mut out = [0u8; PAMOJA_SX126X_COMMAND_MAX];
    out[..bytes.len()].copy_from_slice(bytes);
    PamojaSx126xCommand {
        bytes: out,
        len: bytes.len() as u8,
    }
}

/// Copies a query into the value that crosses the boundary.
fn query_of(query: Query) -> PamojaSx126xQuery {
    PamojaSx126xQuery {
        command: command_of(query.command),
        answer_len: query.answer_len as u32,
    }
}

/// Names the amplifier a flag selects.
fn amplifier(high_power: bool) -> PowerAmplifier {
    if high_power {
        PowerAmplifier::HighPower
    } else {
        PowerAmplifier::LowPower
    }
}

/// Flattens power settings for the boundary.
fn power_of(power: TxPower) -> PamojaSx126xTxPower {
    PamojaSx126xTxPower {
        pa_duty_cycle: power.pa.duty_cycle,
        hp_max: power.pa.hp_max,
        device_sel: power.pa.device,
        pa_lut: power.pa.lut,
        setting_dbm: power.setting_dbm,
    }
}

/// Rebuilds power settings from the fields that crossed the boundary.
fn tx_power(power: PamojaSx126xTxPower) -> TxPower {
    TxPower {
        pa: PaConfig {
            duty_cycle: power.pa_duty_cycle,
            hp_max: power.hp_max,
            device: power.device_sel,
            lut: power.pa_lut,
        },
        setting_dbm: power.setting_dbm,
    }
}

/// Maps a decoded chip mode to the value that crosses the boundary.
fn chip_mode(mode: ChipMode) -> PamojaSx126xChipMode {
    if mode == ChipMode::StandbyRc {
        PamojaSx126xChipMode::StandbyRc
    } else if mode == ChipMode::StandbyXosc {
        PamojaSx126xChipMode::StandbyXosc
    } else if mode == ChipMode::Fs {
        PamojaSx126xChipMode::Fs
    } else if mode == ChipMode::Rx {
        PamojaSx126xChipMode::Rx
    } else if mode == ChipMode::Tx {
        PamojaSx126xChipMode::Tx
    } else {
        PamojaSx126xChipMode::Other
    }
}

/// Maps a decoded command status to the value that crosses the boundary.
fn command_status(status: CommandStatus) -> PamojaSx126xCommandStatus {
    if status == CommandStatus::DataAvailable {
        PamojaSx126xCommandStatus::DataAvailable
    } else if status == CommandStatus::Timeout {
        PamojaSx126xCommandStatus::Timeout
    } else if status == CommandStatus::ProcessingError {
        PamojaSx126xCommandStatus::ProcessingError
    } else if status == CommandStatus::ExecutionFailure {
        PamojaSx126xCommandStatus::ExecutionFailure
    } else if status == CommandStatus::TxDone {
        PamojaSx126xCommandStatus::TxDone
    } else {
        PamojaSx126xCommandStatus::Other
    }
}

/// The amplifier settings of an SX127x: RegPaConfig, RegPaDac, and RegOcp.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaSx127xTxPower {
    /// RegPaConfig: PaSelect, MaxPower, and OutputPower.
    pub pa_config: u8,
    /// RegPaDac: the +20 dBm setting above +17 dBm on PA_BOOST, else its reset value.
    pub pa_dac: u8,
    /// RegOcp: the current limit.
    pub ocp: u8,
    /// The output power the settings produce, in dBm.
    pub output_dbm: i8,
}

/// The LoRa modem registers of an SX127x for a link.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaSx127xModem {
    /// RegModemConfig1: bandwidth, coding rate, and header mode.
    pub modem_config_1: u8,
    /// RegModemConfig2: spreading factor, CRC, and the top bits of the symbol timeout.
    pub modem_config_2: u8,
    /// RegModemConfig3: low data rate optimization and the AGC.
    pub modem_config_3: u8,
    /// The DetectionOptimize bits of RegDetectOptimize, 0x05 for SF6 and 0x03 otherwise, to
    /// put in its low three bits.
    pub detection_optimize: u8,
    /// RegDetectionThreshold.
    pub detection_threshold: u8,
}

/// The signal levels of a packet an SX127x received, in hundredths of a decibel.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaSx127xPacketStatus {
    /// The RSSI averaged over the packet, in hundredths of a dBm.
    pub rssi_centi_dbm: i32,
    /// The estimated signal-to-noise ratio, in hundredths of a dB.
    pub snr_centi_db: i32,
    /// The strength of the packet itself, in hundredths of a dBm.
    pub signal_rssi_centi_dbm: i32,
}

/// The live state of an SX127x LoRa modem, from RegModemStat.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaSx127xModemStatus {
    /// The coding rate denominator the last header announced, 5 to 8, or 0 for a reserved
    /// value.
    pub coding_rate_denominator: u8,
    /// The modem is clear.
    pub clear: bool,
    /// The header of the packet under way is valid.
    pub header_valid: bool,
    /// A reception is under way.
    pub rx_ongoing: bool,
    /// The modem has synchronized on the end of the preamble.
    pub signal_synchronized: bool,
    /// A LoRa preamble has been detected.
    pub signal_detected: bool,
}

/// The writes of the SX127x 500 kHz sensitivity erratum.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaSx127xHighBwOptimize {
    /// The RegHighBwOptimize1 value.
    pub optimize_1: u8,
    /// The RegHighBwOptimize2 value, when `has_optimize_2` is set.
    pub optimize_2: u8,
    /// Whether RegHighBwOptimize2 is written.
    pub has_optimize_2: bool,
}

/// The receive settings of the SX127x spurious reception erratum.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaSx127xSpuriousReception {
    /// Whether AutomaticIFOn, bit 7 of RegDetectOptimize, stays on.
    pub automatic_if: bool,
    /// Whether RegIfFreq2 is set by hand, with RegIfFreq1 cleared.
    pub has_if_freq_2: bool,
    /// The RegIfFreq2 value, when `has_if_freq_2` is set.
    pub if_freq_2: u8,
    /// How far above the carrier to receive, in hertz.
    pub offset_hz: u32,
}

/// Reports whether an LLCC68 supports a link's spreading factor at its bandwidth.
///
/// # Arguments
///
/// * `link` - the link settings.
///
/// # Returns
///
/// `true` when the SX126x offers the bandwidth and the LLCC68 supports the pair: up to SF9
/// at 125 kHz, SF10 at 250 kHz, and SF11 at 500 kHz.
#[no_mangle]
pub extern "C" fn pamoja_sx126x_llcc68_supports(link: PamojaLoraLink) -> bool {
    LoraModulation::from_link(&settings(link)).is_some_and(|modulation| {
        llcc68_supports(modulation.spreading_factor, modulation.bandwidth)
    })
}

/// Returns the 24-bit RegFrf word an SX127x takes for a frequency.
///
/// # Arguments
///
/// * `frequency_hz` - the carrier frequency in hertz.
///
/// # Returns
///
/// The word, in steps of 32 MHz over 2^19, rounded to the nearest step.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_frequency_word(frequency_hz: u32) -> u32 {
    sx127x_config::frequency_word(frequency_hz)
}

/// Returns the frequency an SX127x RegFrf word selects.
///
/// # Arguments
///
/// * `word` - the 24-bit frequency word.
///
/// # Returns
///
/// The carrier frequency in hertz, rounded to the nearest hertz.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_frequency_from_word(word: u32) -> u32 {
    sx127x_config::frequency_from_word(word)
}

/// Returns the SX127x address byte that reads a register.
///
/// # Arguments
///
/// * `address` - the register address.
///
/// # Returns
///
/// The address with the write bit clear.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_read_address(address: u8) -> u8 {
    sx127x_register::read_address(address)
}

/// Returns the SX127x address byte that writes a register.
///
/// # Arguments
///
/// * `address` - the register address.
///
/// # Returns
///
/// The address with the write bit set.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_write_address(address: u8) -> u8 {
    sx127x_register::write_address(address)
}

/// Returns the RegOpMode value for a LoRa operating mode.
///
/// # Arguments
///
/// * `mode` - a `PAMOJA_SX127X_MODE_` code; only its low three bits are read.
///
/// # Returns
///
/// The RegOpMode value, with the LoRa register page selected.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_lora_op_mode(mode: u8) -> u8 {
    sx127x_register::lora_op_mode(Sx127xMode::from_op_mode(mode))
}

/// Returns the RegOpMode value for an FSK operating mode, which image calibration needs.
///
/// # Arguments
///
/// * `mode` - a `PAMOJA_SX127X_MODE_` code; only its low three bits are read.
///
/// # Returns
///
/// The RegOpMode value.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_fsk_op_mode(mode: u8) -> u8 {
    sx127x_register::fsk_op_mode(Sx127xMode::from_op_mode(mode))
}

/// Returns the operating mode a RegOpMode value holds.
///
/// # Arguments
///
/// * `op_mode` - the RegOpMode value.
///
/// # Returns
///
/// The `PAMOJA_SX127X_MODE_` code.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_mode_from_op_mode(op_mode: u8) -> u8 {
    Sx127xMode::from_op_mode(op_mode).code()
}

/// Returns the LoRa modem registers for a link at a carrier.
///
/// # Arguments
///
/// * `link` - the link settings.
/// * `frequency_hz` - the carrier frequency in hertz, which rules out 250 and 500 kHz in the
///   lowest band.
/// * `symbol_timeout` - the single reception timeout in symbols, whose top bits go in
///   RegModemConfig2.
/// * `out_modem` - receives the registers.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with `*out_modem` set, or [`PamojaStatus::InvalidArgument`] if the
/// SX127x cannot use the link at the carrier or `out_modem` is null.
///
/// # Safety
///
/// `out_modem` must point to a writable [`PamojaSx127xModem`], or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sx127x_modem(
    link: PamojaLoraLink,
    frequency_hz: u32,
    symbol_timeout: u16,
    out_modem: *mut PamojaSx127xModem,
) -> PamojaStatus {
    if out_modem.is_null() {
        set_last_error("out_modem must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match sx127x_modulation(link, frequency_hz) {
        Ok(modulation) => {
            *out_modem = PamojaSx127xModem {
                modem_config_1: modulation.modem_config_1(),
                modem_config_2: modulation.modem_config_2(symbol_timeout),
                modem_config_3: modulation.modem_config_3(),
                detection_optimize: modulation.detect_optimize(0),
                detection_threshold: modulation.detection_threshold(),
            };
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(modulation_message(error));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Returns the SX127x single reception timeout for a duration, in the link's symbols.
///
/// # Arguments
///
/// * `link` - the link settings.
/// * `timeout_us` - how long to listen for a preamble, in microseconds.
///
/// # Returns
///
/// The timeout rounded up to whole symbols, from 4 to 1023.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_symbol_timeout(link: PamojaLoraLink, timeout_us: u64) -> u16 {
    sx127x_config::symbol_timeout(&settings(link), timeout_us)
}

/// Returns the SX127x amplifier settings for an output power.
///
/// # Arguments
///
/// * `pa_boost` - `true` for the PA_BOOST output, `false` for RFO.
/// * `output_dbm` - the output power wanted, clamped to what the output delivers.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_tx_power_for_output(
    pa_boost: bool,
    output_dbm: i8,
) -> PamojaSx127xTxPower {
    sx127x_power_of(Sx127xPower::for_output(pa_output(pa_boost), output_dbm))
}

/// Returns the SX127x amplifier settings that keep a link's EIRP at or under a ceiling.
///
/// # Arguments
///
/// * `pa_boost` - `true` for the PA_BOOST output, `false` for RFO.
/// * `budget` - the link budget, whose transmitting antenna and cable apply.
/// * `eirp_ceiling_centi_dbm` - the EIRP limit in hundredths of a dBm.
///
/// # Returns
///
/// The settings, rounded down to whole decibels.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_tx_power_under_ceiling(
    pa_boost: bool,
    budget: PamojaLoraLinkBudget,
    eirp_ceiling_centi_dbm: i32,
) -> PamojaSx127xTxPower {
    sx127x_power_of(Sx127xPower::under_ceiling(
        pa_output(pa_boost),
        &link_budget(budget),
        Decibels::from_hundredths(eirp_ceiling_centi_dbm),
    ))
}

/// Returns RegOcp for a current limit.
///
/// # Arguments
///
/// * `milliamps` - the most current the amplifier may draw.
///
/// # Returns
///
/// The register value with the protection on.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_ocp_register(milliamps: u16) -> u8 {
    sx127x_config::ocp_register(milliamps)
}

/// Returns RegInvertIQ for the IQ polarity of each path.
///
/// # Arguments
///
/// * `receive` - whether to invert the receive path.
/// * `transmit` - whether to invert the transmit path.
///
/// # Returns
///
/// The register value, with the transmit bit set for normal IQ as the reference drivers
/// have it.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_invert_iq(receive: bool, transmit: bool) -> u8 {
    sx127x_config::invert_iq(receive, transmit)
}

/// Returns RegInvertIQ2 for the path in use.
///
/// # Arguments
///
/// * `inverted` - whether that path is inverted.
///
/// # Returns
///
/// 0x19 when inverted, else 0x1D.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_invert_iq_2(inverted: bool) -> u8 {
    sx127x_config::invert_iq_2(inverted)
}

/// Returns the writes of the 500 kHz sensitivity erratum for a link at a carrier.
///
/// # Arguments
///
/// * `link` - the link settings, whose bandwidth decides.
/// * `frequency_hz` - the carrier frequency in hertz.
/// * `out_optimize` - receives the writes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with `*out_optimize` set, or [`PamojaStatus::InvalidArgument`] if the
/// SX127x has no such bandwidth or `out_optimize` is null.
///
/// # Safety
///
/// `out_optimize` must point to a writable [`PamojaSx127xHighBwOptimize`], or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sx127x_high_bw_optimize(
    link: PamojaLoraLink,
    frequency_hz: u32,
    out_optimize: *mut PamojaSx127xHighBwOptimize,
) -> PamojaStatus {
    if out_optimize.is_null() {
        set_last_error("out_optimize must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(bandwidth) = sx127x_bandwidth(link) else {
        return PamojaStatus::InvalidArgument;
    };
    let (optimize_1, optimize_2) = sx127x_config::high_bw_optimize(bandwidth, frequency_hz);
    *out_optimize = PamojaSx127xHighBwOptimize {
        optimize_1,
        optimize_2: optimize_2.unwrap_or(0),
        has_optimize_2: optimize_2.is_some(),
    };
    PamojaStatus::Ok
}

/// Returns the receive settings of the spurious reception erratum for a link.
///
/// # Arguments
///
/// * `link` - the link settings, whose bandwidth decides.
/// * `out_erratum` - receives the settings.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with `*out_erratum` set, or [`PamojaStatus::InvalidArgument`] if the
/// SX127x has no such bandwidth or `out_erratum` is null.
///
/// # Safety
///
/// `out_erratum` must point to a writable [`PamojaSx127xSpuriousReception`], or be null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_sx127x_spurious_reception(
    link: PamojaLoraLink,
    out_erratum: *mut PamojaSx127xSpuriousReception,
) -> PamojaStatus {
    if out_erratum.is_null() {
        set_last_error("out_erratum must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(bandwidth) = sx127x_bandwidth(link) else {
        return PamojaStatus::InvalidArgument;
    };
    let erratum = sx127x_config::spurious_reception(bandwidth);
    *out_erratum = PamojaSx127xSpuriousReception {
        automatic_if: erratum.automatic_if,
        has_if_freq_2: erratum.if_freq_2.is_some(),
        if_freq_2: erratum.if_freq_2.unwrap_or(0),
        offset_hz: erratum.offset_hz,
    };
    PamojaStatus::Ok
}

/// Returns RegImageCal to start a calibration.
///
/// # Arguments
///
/// * `current` - the register's current value.
///
/// # Returns
///
/// The value with ImageCalStart set and AutoImageCalOn clear.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_image_cal_start(current: u8) -> u8 {
    sx127x_config::image_cal_start(current)
}

/// Returns RegDetectOptimize with AutomaticIFOn set or clear.
///
/// # Arguments
///
/// * `current` - the register's current value.
/// * `automatic_if` - whether the automatic IF stays on.
///
/// # Returns
///
/// The register value.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_automatic_if(current: u8, automatic_if: bool) -> u8 {
    sx127x_config::automatic_if(current, automatic_if)
}

/// Decodes RegPktSnrValue and RegPktRssiValue.
///
/// # Arguments
///
/// * `pkt_snr` - the RegPktSnrValue byte.
/// * `pkt_rssi` - the RegPktRssiValue byte.
/// * `frequency_hz` - the carrier the packet was received on, which picks the RF port's
///   offset.
///
/// # Returns
///
/// The three levels in hundredths of a decibel.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_packet_status_from_bytes(
    pkt_snr: u8,
    pkt_rssi: u8,
    frequency_hz: u32,
) -> PamojaSx127xPacketStatus {
    let status =
        Sx127xPacketStatus::from_bytes([pkt_snr, pkt_rssi], Port::for_frequency(frequency_hz));
    PamojaSx127xPacketStatus {
        rssi_centi_dbm: status.rssi_dbm.hundredths(),
        snr_centi_db: status.snr_db.hundredths(),
        signal_rssi_centi_dbm: status.signal_rssi_dbm.hundredths(),
    }
}

/// Decodes RegRssiValue.
///
/// # Arguments
///
/// * `byte` - the RegRssiValue byte.
/// * `frequency_hz` - the carrier the receiver is tuned to, which picks the RF port's offset.
///
/// # Returns
///
/// The RSSI in hundredths of a dBm.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_rssi_centi_dbm(byte: u8, frequency_hz: u32) -> i32 {
    sx127x_rssi_dbm(byte, Port::for_frequency(frequency_hz)).hundredths()
}

/// Decodes RegModemStat.
///
/// # Arguments
///
/// * `byte` - the register value.
///
/// # Returns
///
/// The modem's state.
#[no_mangle]
pub extern "C" fn pamoja_sx127x_modem_status_from_byte(byte: u8) -> PamojaSx127xModemStatus {
    let status = Sx127xModemStatus::from_byte(byte);
    PamojaSx127xModemStatus {
        coding_rate_denominator: status.coding_rate_denominator.unwrap_or(0),
        clear: status.clear,
        header_valid: status.header_valid,
        rx_ongoing: status.rx_ongoing,
        signal_synchronized: status.signal_synchronized,
        signal_detected: status.signal_detected,
    }
}

/// Builds the SX127x modem settings of a link, refusing a bandwidth outside the carrier's band.
fn sx127x_modulation(
    link: PamojaLoraLink,
    frequency_hz: u32,
) -> Result<Sx127xModulation, ModulationError> {
    let modulation = Sx127xModulation::from_link(&settings(link))?;
    if modulation.bandwidth.in_band(frequency_hz) {
        Ok(modulation)
    } else {
        Err(ModulationError::Bandwidth(link.bandwidth_hz))
    }
}

/// Finds a link's SX127x bandwidth, recording the error when there is none.
fn sx127x_bandwidth(link: PamojaLoraLink) -> Option<Sx127xBandwidth> {
    let bandwidth = Sx127xBandwidth::from_hz(link.bandwidth_hz);
    if bandwidth.is_none() {
        set_last_error(format!(
            "the SX127x has no {} Hz LoRa bandwidth",
            link.bandwidth_hz
        ));
    }
    bandwidth
}

/// Says why an SX127x cannot use a link.
fn modulation_message(error: ModulationError) -> String {
    match error {
        ModulationError::Bandwidth(hz) => {
            format!("the SX127x has no {hz} Hz LoRa bandwidth at this carrier")
        }
        ModulationError::SpreadingFactor(sf) => format!("the SX127x has no SF{sf}"),
        ModulationError::ExplicitHeaderAtSf6 => "SF6 needs an implicit header".to_owned(),
    }
}

/// Names the amplifier output a flag selects.
fn pa_output(pa_boost: bool) -> PaOutput {
    if pa_boost {
        PaOutput::PaBoost
    } else {
        PaOutput::Rfo
    }
}

/// Flattens SX127x power settings for the boundary.
fn sx127x_power_of(power: Sx127xPower) -> PamojaSx127xTxPower {
    PamojaSx127xTxPower {
        pa_config: power.pa_config,
        pa_dac: power.pa_dac,
        ocp: power.ocp,
        output_dbm: power.output_dbm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lora::{pamoja_lora_link_budget_default, pamoja_lora_link_default};

    fn bytes(command: PamojaSx126xCommand) -> Vec<u8> {
        command.bytes[..usize::from(command.len)].to_vec()
    }

    #[test]
    fn commands_carry_the_datasheet_bytes() {
        assert_eq!(
            bytes(pamoja_sx126x_set_rf_frequency(868_100_000)),
            [0x86, 0x36, 0x41, 0x99, 0x9A]
        );
        let power = pamoja_sx126x_tx_power_for_output(true, 14);
        assert_eq!(
            bytes(pamoja_sx126x_set_pa_config(power)),
            [0x95, 0x04, 0x07, 0x00, 0x01]
        );
        assert_eq!(
            bytes(pamoja_sx126x_set_tx_params(power, 40)),
            [0x8E, 0x0E, 0x02]
        );
        assert_eq!(
            bytes(pamoja_sx126x_set_rx_continuous()),
            [0x82, 0xFF, 0xFF, 0xFF]
        );
        let query = pamoja_sx126x_get_irq_status();
        assert_eq!(bytes(query.command), [0x12, 0x00]);
        assert_eq!(query.answer_len, 2);
    }

    #[test]
    fn a_bandwidth_the_chip_lacks_is_refused() {
        let mut command = PamojaSx126xCommand {
            bytes: [0; PAMOJA_SX126X_COMMAND_MAX],
            len: 0,
        };
        let link = pamoja_lora_link_default(7, 125_000);
        let status = unsafe { pamoja_sx126x_set_lora_modulation_params(link, &mut command) };
        assert_eq!(status, PamojaStatus::Ok);
        assert_eq!(bytes(command), [0x8B, 0x07, 0x04, 0x01, 0x00]);

        let narrow = pamoja_lora_link_default(7, 203_125);
        let status = unsafe { pamoja_sx126x_set_lora_modulation_params(narrow, &mut command) };
        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn power_under_a_ceiling_takes_off_the_antenna() {
        let whip = PamojaLoraLinkBudget {
            transmit_antenna_gain_centi_dbi: 215,
            transmit_cable_loss_centi_db: 50,
            ..pamoja_lora_link_budget_default()
        };
        let power = pamoja_sx126x_tx_power_under_ceiling(true, whip, 1_600);
        assert_eq!(power.setting_dbm, 14);
        assert_eq!(
            pamoja_sx126x_tx_power_for_output(false, 15).pa_duty_cycle,
            0x06
        );
        assert_eq!(pamoja_sx126x_ramp_time_us(100), 200);
    }

    #[test]
    fn answers_decode_as_the_datasheet_gives() {
        let status = pamoja_sx126x_status_from_byte(0x2C);
        assert_eq!(status.chip_mode, PamojaSx126xChipMode::StandbyRc);
        assert_eq!(status.command_status, PamojaSx126xCommandStatus::TxDone);
        assert!(!status.error);
        assert_eq!(
            pamoja_sx126x_irq_from_bytes(0x02, 0x62),
            PAMOJA_SX126X_IRQ_TIMEOUT
                | PAMOJA_SX126X_IRQ_CRC_ERROR
                | PAMOJA_SX126X_IRQ_HEADER_ERROR
                | PAMOJA_SX126X_IRQ_RX_DONE
        );
        let packet = pamoja_sx126x_packet_status_from_bytes(0xDB, 0xF6, 0xE0);
        assert_eq!(packet.rssi_centi_dbm, -10_950);
        assert_eq!(packet.snr_centi_db, -250);
        assert_eq!(pamoja_sx126x_rssi_inst_centi_dbm(0xDB), -10_950);
    }

    #[test]
    fn a_duty_cycle_guard_holds_the_radio_silent() {
        let guard = pamoja_radio_duty_cycle_new(10);
        let link = pamoja_lora_link_default(12, 125_000);
        unsafe {
            assert_eq!(
                pamoja_radio_duty_cycle_transmitted(guard, 0, link, 10),
                991_232
            );
            assert_eq!(pamoja_radio_duty_cycle_wait_us(guard, 0), 99_123_200);
            assert!(!pamoja_radio_duty_cycle_ready(guard, 99_000_000));
            assert!(pamoja_radio_duty_cycle_ready(guard, 99_123_200));
            pamoja_radio_duty_cycle_free(guard);
            assert!(!pamoja_radio_duty_cycle_ready(std::ptr::null(), 0));
        }
    }

    #[test]
    fn sx127x_register_values_follow_the_datasheet() {
        assert_eq!(pamoja_sx127x_frequency_word(868_100_000), 0xD9_0666);
        assert_eq!(pamoja_sx127x_frequency_from_word(0x6C_8000), 434_000_000);
        assert_eq!(pamoja_sx127x_lora_op_mode(PAMOJA_SX127X_MODE_TX), 0x8B);
        assert_eq!(pamoja_sx127x_fsk_op_mode(PAMOJA_SX127X_MODE_STANDBY), 0x09);
        assert_eq!(
            pamoja_sx127x_mode_from_op_mode(0x8D),
            PAMOJA_SX127X_MODE_RX_CONTINUOUS
        );
        assert_eq!(pamoja_sx127x_write_address(PAMOJA_SX127X_REG_OP_MODE), 0x81);
        assert_eq!(pamoja_sx127x_read_address(0x81), PAMOJA_SX127X_REG_OP_MODE);

        let mut modem = PamojaSx127xModem::default();
        let link = pamoja_lora_link_default(7, 125_000);
        assert_eq!(
            unsafe { pamoja_sx127x_modem(link, 868_100_000, 0, &mut modem) },
            PamojaStatus::Ok
        );
        assert_eq!(
            (
                modem.modem_config_1,
                modem.modem_config_2,
                modem.modem_config_3
            ),
            (0x72, 0x74, 0x04)
        );
        assert_eq!(
            (modem.detection_optimize, modem.detection_threshold),
            (0x03, 0x0A)
        );

        let power = pamoja_sx127x_tx_power_for_output(true, 20);
        assert_eq!(
            (power.pa_config, power.pa_dac, power.ocp),
            (0xFF, 0x87, 0x31)
        );
        assert_eq!(pamoja_sx127x_invert_iq(true, true), 0x66);
        assert_eq!(pamoja_sx127x_invert_iq_2(false), 0x1D);
        let packet = pamoja_sx127x_packet_status_from_bytes(0xF6, 0x30, 868_100_000);
        assert_eq!(
            (
                packet.rssi_centi_dbm,
                packet.snr_centi_db,
                packet.signal_rssi_centi_dbm
            ),
            (-10_900, -250, -11_150)
        );
        assert_eq!(pamoja_sx127x_rssi_centi_dbm(0x30, 433_175_000), -11_600);
        assert_eq!(
            pamoja_sx127x_modem_status_from_byte(0x30).coding_rate_denominator,
            5
        );
    }

    #[test]
    fn sx127x_refuses_what_the_chip_cannot_do_and_the_llcc68_limits_hold() {
        let mut modem = PamojaSx127xModem::default();
        let sf5 = pamoja_lora_link_default(5, 125_000);
        assert_eq!(
            unsafe { pamoja_sx127x_modem(sf5, 868_100_000, 0, &mut modem) },
            PamojaStatus::InvalidArgument
        );
        let wide = pamoja_lora_link_default(7, 500_000);
        assert_eq!(
            unsafe { pamoja_sx127x_modem(wide, 169_400_000, 0, &mut modem) },
            PamojaStatus::InvalidArgument
        );
        assert!(pamoja_sx126x_llcc68_supports(pamoja_lora_link_default(
            9, 125_000
        )));
        assert!(!pamoja_sx126x_llcc68_supports(pamoja_lora_link_default(
            10, 125_000
        )));

        let mut erratum = PamojaSx127xSpuriousReception::default();
        let narrow = pamoja_lora_link_default(9, 20_833);
        assert_eq!(
            unsafe { pamoja_sx127x_spurious_reception(narrow, &mut erratum) },
            PamojaStatus::Ok
        );
        assert!(erratum.has_if_freq_2);
        assert_eq!((erratum.if_freq_2, erratum.offset_hz), (0x44, 20_830));
        let mut optimize = PamojaSx127xHighBwOptimize::default();
        assert_eq!(
            unsafe { pamoja_sx127x_high_bw_optimize(wide, 915_000_000, &mut optimize) },
            PamojaStatus::Ok
        );
        assert_eq!((optimize.optimize_1, optimize.optimize_2), (0x02, 0x64));
        assert!(optimize.has_optimize_2);
    }
}
