"""The Semtech SX1276, SX1277, SX1278, and SX1279, as the register values they take.

These radios, and modules built on them such as the RFM95W, are driven through registers.
Each SPI transaction starts with an address byte whose top bit is set for a write, followed
by the data, and the address advances with each byte except at the FIFO. These functions
give the addresses, the values a LoRa link and an output power put in them, and the
readings decoded, from the SX1276/77/78/79 datasheet (Rev 7), so a program with its own SPI
access can drive the chip.
"""

from __future__ import annotations

from enum import Enum, IntEnum, IntFlag

from pamoja._native import (
    LinkBudget,
    LoraLink,
    Sx127xHighBwOptimize as HighBwOptimize,
    Sx127xModem as Modem,
    Sx127xModemStatus as ModemStatus,
    Sx127xPacketStatus as PacketStatus,
    Sx127xSpuriousReception as SpuriousReception,
    Sx127xTxPower as TxPower,
)
from pamoja._native import sx127x_automatic_if as _automatic_if
from pamoja._native import sx127x_constants as _constants
from pamoja._native import sx127x_frequency_from_word as _frequency_from_word
from pamoja._native import sx127x_frequency_word as _frequency_word
from pamoja._native import sx127x_fsk_op_mode as _fsk_op_mode
from pamoja._native import sx127x_high_bw_optimize as _high_bw_optimize
from pamoja._native import sx127x_image_cal_start as _image_cal_start
from pamoja._native import sx127x_invert_iq as _invert_iq
from pamoja._native import sx127x_invert_iq_2 as _invert_iq_2
from pamoja._native import sx127x_irq_flags as _irq_flags
from pamoja._native import sx127x_lora_op_mode as _lora_op_mode
from pamoja._native import sx127x_mode_from_op_mode as _mode_from_op_mode
from pamoja._native import sx127x_modem as _modem
from pamoja._native import sx127x_modem_status as _modem_status
from pamoja._native import sx127x_ocp_register as _ocp_register
from pamoja._native import sx127x_packet_status as _packet_status
from pamoja._native import sx127x_read_address as _read_address
from pamoja._native import sx127x_registers as _registers
from pamoja._native import sx127x_rssi_dbm as _rssi_dbm
from pamoja._native import sx127x_spurious_reception as _spurious_reception
from pamoja._native import sx127x_symbol_timeout as _symbol_timeout
from pamoja._native import sx127x_tx_power as _tx_power
from pamoja._native import sx127x_tx_power_under_ceiling as _tx_power_under_ceiling
from pamoja._native import sx127x_write_address as _write_address

__all__ = [
    "IMAGE_CAL_RUNNING",
    "IMAGE_CAL_START",
    "LNA_BOOSTED",
    "PA_DAC_DEFAULT",
    "PA_DAC_HIGH_POWER",
    "SYNC_WORD_PRIVATE",
    "SYNC_WORD_PUBLIC",
    "TCXO_INPUT_ON",
    "VERSION",
    "WRITE",
    "Dio0",
    "HighBwOptimize",
    "Irq",
    "Mode",
    "Modem",
    "ModemStatus",
    "PaOutput",
    "PacketStatus",
    "Register",
    "SpuriousReception",
    "TxPower",
    "automatic_if",
    "frequency_from_word",
    "frequency_word",
    "fsk_op_mode",
    "high_bw_optimize",
    "image_cal_start",
    "invert_iq",
    "invert_iq_2",
    "lora_op_mode",
    "mode_from_op_mode",
    "modem",
    "modem_status",
    "ocp_register",
    "packet_status",
    "read_address",
    "rssi_dbm",
    "spurious_reception",
    "symbol_timeout",
    "tx_power",
    "tx_power_under_ceiling",
    "write_address",
]

_REGISTERS = _registers()
_CONSTANTS = _constants()
_IRQ_FLAGS = _irq_flags()

#: The RegVersion value of an SX1276, SX1277, SX1278, or SX1279.
VERSION = _CONSTANTS["version"]
#: The bit of an address byte that makes an access a write.
WRITE = _CONSTANTS["write"]
#: The sync word the datasheet reserves for LoRaWAN networks.
SYNC_WORD_PUBLIC = _CONSTANTS["syncWordPublic"]
#: The private sync word, and the chip's reset value.
SYNC_WORD_PRIVATE = _CONSTANTS["syncWordPrivate"]
#: RegPaDac at its reset value.
PA_DAC_DEFAULT = _CONSTANTS["paDacDefault"]
#: RegPaDac with the +20 dBm setting on PA_BOOST.
PA_DAC_HIGH_POWER = _CONSTANTS["paDacHighPower"]
#: The RegImageCal bit that starts a calibration.
IMAGE_CAL_START = _CONSTANTS["imageCalStart"]
#: The RegImageCal bit set while a calibration runs.
IMAGE_CAL_RUNNING = _CONSTANTS["imageCalRunning"]
#: RegLna with maximum gain and the high frequency LNA boost.
LNA_BOOSTED = _CONSTANTS["lnaBoosted"]
#: RegTcxo for a module clocked by a TCXO.
TCXO_INPUT_ON = _CONSTANTS["tcxoInputOn"]


class Register(IntEnum):
    """The register addresses, from Table 41 of the datasheet with the LoRa page selected."""

    #: RegFifo, the LoRa data buffer, read or written at RegFifoAddrPtr.
    FIFO = _REGISTERS["fifo"]
    #: RegOpMode: LoRa or FSK, the register page, and the operating mode.
    OP_MODE = _REGISTERS["opMode"]
    #: RegFrfMsb, the top byte of the carrier word.
    FRF_MSB = _REGISTERS["frfMsb"]
    #: RegFrfMid, the middle byte of the carrier word.
    FRF_MID = _REGISTERS["frfMid"]
    #: RegFrfLsb, the low byte of the carrier word.
    FRF_LSB = _REGISTERS["frfLsb"]
    #: RegPaConfig: the amplifier output, its maximum, and the power.
    PA_CONFIG = _REGISTERS["paConfig"]
    #: RegPaRamp, the amplifier ramp time.
    PA_RAMP = _REGISTERS["paRamp"]
    #: RegOcp, the amplifier current limit.
    OCP = _REGISTERS["ocp"]
    #: RegLna, the LNA gain and current.
    LNA = _REGISTERS["lna"]
    #: RegFifoAddrPtr, where the next RegFifo access lands.
    FIFO_ADDR_PTR = _REGISTERS["fifoAddrPtr"]
    #: RegFifoTxBaseAddr, where a transmitted payload starts.
    FIFO_TX_BASE_ADDR = _REGISTERS["fifoTxBaseAddr"]
    #: RegFifoRxBaseAddr, where received payloads start.
    FIFO_RX_BASE_ADDR = _REGISTERS["fifoRxBaseAddr"]
    #: RegFifoRxCurrentAddr, where the last received packet starts.
    FIFO_RX_CURRENT_ADDR = _REGISTERS["fifoRxCurrentAddr"]
    #: RegIrqFlagsMask, the interrupts masked off.
    IRQ_FLAGS_MASK = _REGISTERS["irqFlagsMask"]
    #: RegIrqFlags, the interrupts raised, each cleared by writing it back as a 1.
    IRQ_FLAGS = _REGISTERS["irqFlags"]
    #: RegRxNbBytes, the payload length of the last packet received.
    RX_NB_BYTES = _REGISTERS["rxNbBytes"]
    #: RegModemStat, the live state of the modem.
    MODEM_STAT = _REGISTERS["modemStat"]
    #: RegPktSnrValue, the SNR of the last packet in quarters of a decibel.
    PKT_SNR_VALUE = _REGISTERS["pktSnrValue"]
    #: RegPktRssiValue, the RSSI of the last packet.
    PKT_RSSI_VALUE = _REGISTERS["pktRssiValue"]
    #: RegRssiValue, the RSSI the receiver hears right now.
    RSSI_VALUE = _REGISTERS["rssiValue"]
    #: RegHopChannel, the PLL lock and the CRC the last header announced.
    HOP_CHANNEL = _REGISTERS["hopChannel"]
    #: RegModemConfig1: bandwidth, coding rate, and header mode.
    MODEM_CONFIG_1 = _REGISTERS["modemConfig1"]
    #: RegModemConfig2: spreading factor, CRC, and the top bits of the symbol timeout.
    MODEM_CONFIG_2 = _REGISTERS["modemConfig2"]
    #: RegSymbTimeoutLsb, the low byte of the symbol timeout.
    SYMB_TIMEOUT_LSB = _REGISTERS["symbTimeoutLsb"]
    #: RegPreambleMsb, the high byte of the preamble length.
    PREAMBLE_MSB = _REGISTERS["preambleMsb"]
    #: RegPreambleLsb, the low byte of the preamble length.
    PREAMBLE_LSB = _REGISTERS["preambleLsb"]
    #: RegPayloadLength, the payload length to send.
    PAYLOAD_LENGTH = _REGISTERS["payloadLength"]
    #: RegMaxPayloadLength, the longest payload a received header may announce.
    MAX_PAYLOAD_LENGTH = _REGISTERS["maxPayloadLength"]
    #: RegModemConfig3: low data rate optimization and the automatic gain control.
    MODEM_CONFIG_3 = _REGISTERS["modemConfig3"]
    #: RegRssiWideband, a wideband RSSI sample.
    RSSI_WIDEBAND = _REGISTERS["rssiWideband"]
    #: RegIfFreq2, which the spurious reception erratum sets.
    IF_FREQ_2 = _REGISTERS["ifFreq2"]
    #: RegIfFreq1, which the spurious reception erratum clears.
    IF_FREQ_1 = _REGISTERS["ifFreq1"]
    #: RegDetectOptimize: the automatic IF and the detection optimization.
    DETECT_OPTIMIZE = _REGISTERS["detectOptimize"]
    #: RegInvertIQ, the IQ polarity of each path.
    INVERT_IQ = _REGISTERS["invertIq"]
    #: RegHighBwOptimize1, which the 500 kHz erratum sets.
    HIGH_BW_OPTIMIZE_1 = _REGISTERS["highBwOptimize1"]
    #: RegDetectionThreshold, the LoRa detection threshold.
    DETECTION_THRESHOLD = _REGISTERS["detectionThreshold"]
    #: RegSyncWord, the LoRa sync word.
    SYNC_WORD = _REGISTERS["syncWord"]
    #: RegHighBwOptimize2, which the 500 kHz erratum sets.
    HIGH_BW_OPTIMIZE_2 = _REGISTERS["highBwOptimize2"]
    #: RegInvertIQ2, which completes an IQ inversion.
    INVERT_IQ_2 = _REGISTERS["invertIq2"]
    #: RegImageCal, at the address of RegInvertIQ2 on the FSK page.
    IMAGE_CAL = _REGISTERS["imageCal"]
    #: RegDioMapping1, the events DIO0 to DIO3 signal.
    DIO_MAPPING_1 = _REGISTERS["dioMapping1"]
    #: RegDioMapping2, the events DIO4 and DIO5 signal.
    DIO_MAPPING_2 = _REGISTERS["dioMapping2"]
    #: RegVersion, the silicon revision.
    VERSION = _REGISTERS["version"]
    #: RegTcxo, a crystal or a TCXO on XTA.
    TCXO = _REGISTERS["tcxo"]
    #: RegPaDac, the +20 dBm setting of PA_BOOST.
    PA_DAC = _REGISTERS["paDac"]


class Dio0(IntEnum):
    """The RegDioMapping1 values that route an event to DIO0, from Table 18 of the datasheet."""

    #: DIO0 signals RxDone.
    RX_DONE = _CONSTANTS["dio0RxDone"]
    #: DIO0 signals TxDone.
    TX_DONE = _CONSTANTS["dio0TxDone"]
    #: DIO0 signals CadDone.
    CAD_DONE = _CONSTANTS["dio0CadDone"]


class Irq(IntFlag):
    """The LoRa interrupt flags of RegIrqFlags."""

    #: A single reception timed out before a preamble arrived.
    RX_TIMEOUT = _IRQ_FLAGS["rxTimeout"]
    #: A packet has been received.
    RX_DONE = _IRQ_FLAGS["rxDone"]
    #: The payload failed its CRC.
    PAYLOAD_CRC_ERROR = _IRQ_FLAGS["payloadCrcError"]
    #: A valid header was received.
    VALID_HEADER = _IRQ_FLAGS["validHeader"]
    #: The payload has been transmitted.
    TX_DONE = _IRQ_FLAGS["txDone"]
    #: Channel activity detection finished.
    CAD_DONE = _IRQ_FLAGS["cadDone"]
    #: Frequency hopping moved to the next channel.
    FHSS_CHANGE_CHANNEL = _IRQ_FLAGS["fhssChangeChannel"]
    #: Channel activity detection heard a LoRa signal.
    CAD_DETECTED = _IRQ_FLAGS["cadDetected"]


class PaOutput(str, Enum):
    """The amplifier output a module wires to its antenna.

    The SPI interface cannot see which output a module uses, so the caller names it.
    """

    #: The high efficiency amplifier on RFO_LF or RFO_HF, -4 to +15 dBm.
    RFO = "Rfo"
    #: The regulated amplifier on PA_BOOST, +2 to +20 dBm, as on the RFM95W.
    PA_BOOST = "PaBoost"


class Mode(str, Enum):
    """The operating modes of RegOpMode."""

    #: Only the SPI interface and the registers are powered; the only mode that may switch modems.
    SLEEP = "Sleep"
    #: The oscillator and the baseband are on.
    STANDBY = "Standby"
    #: The PLL is locked for transmit.
    FS_TX = "FsTx"
    #: One packet goes out, then the chip returns to standby.
    TX = "Tx"
    #: The PLL is locked for receive.
    FS_RX = "FsRx"
    #: The receiver takes packet after packet.
    RX_CONTINUOUS = "RxContinuous"
    #: The receiver waits for one packet or the symbol timeout.
    RX_SINGLE = "RxSingle"
    #: Channel activity detection looks for a LoRa preamble.
    CAD = "Cad"


def frequency_word(frequency_hz: int) -> int:
    """Return the 24-bit RegFrf word for a frequency.

    :param frequency_hz: The carrier frequency in hertz.
    :returns: The frequency times 2^19 over the 32 MHz crystal, rounded to the nearest step.

    >>> hex(frequency_word(868_100_000))
    '0xd90666'
    """
    return _frequency_word(frequency_hz)


def frequency_from_word(word: int) -> int:
    """Return the frequency a RegFrf word selects.

    :param word: The 24-bit frequency word.
    :returns: The carrier frequency in hertz.
    """
    return _frequency_from_word(word)


def read_address(address: int) -> int:
    """Return the address byte that reads a register.

    :param address: The register address.
    :returns: The address with the write bit clear.
    """
    return _read_address(address)


def write_address(address: int) -> int:
    """Return the address byte that writes a register.

    :param address: The register address.
    :returns: The address with the write bit set.
    """
    return _write_address(address)


def lora_op_mode(mode: Mode | str) -> int:
    """Return the RegOpMode value for a LoRa operating mode.

    :param mode: The operating mode.
    :returns: The register value, with the LoRa register page selected.
    :raises ValueError: If no mode goes by that name.
    """
    return _lora_op_mode(Mode(mode).value)


def fsk_op_mode(mode: Mode | str) -> int:
    """Return the RegOpMode value for an FSK operating mode, which image calibration needs.

    :param mode: The operating mode.
    :returns: The register value.
    :raises ValueError: If no mode goes by that name.
    """
    return _fsk_op_mode(Mode(mode).value)


def mode_from_op_mode(op_mode: int) -> Mode:
    """Return the operating mode a RegOpMode value holds.

    :param op_mode: The register value.
    :returns: The mode.
    """
    return Mode(_mode_from_op_mode(op_mode))


def modem(link: LoraLink, frequency_hz: int, symbol_timeout: int = 0) -> Modem:
    """Return the LoRa modem registers for a link at a carrier.

    :param link: The link settings.
    :param frequency_hz: The carrier frequency, which rules out 250 and 500 kHz below 175 MHz.
    :param symbol_timeout: A single reception timeout in symbols, whose top bits go in
        RegModemConfig2.
    :returns: RegModemConfig1 to 3 and the SF6 detection settings.
    :raises PamojaError: If the SX127x cannot use the link at the carrier.
    """
    return _modem(link, frequency_hz, symbol_timeout)


def symbol_timeout(link: LoraLink, timeout_us: int) -> int:
    """Return a single reception timeout in the link's symbols.

    :param link: The link settings, whose symbol time counts the timeout.
    :param timeout_us: How long to listen for a preamble, in microseconds.
    :returns: The timeout rounded up to whole symbols, from 4 to 1023.
    """
    return _symbol_timeout(link, timeout_us)


def tx_power(output: PaOutput | str, output_dbm: int) -> TxPower:
    """Choose the amplifier settings for an output power.

    :param output: The amplifier output the module uses.
    :param output_dbm: The output power wanted, in dBm.
    :returns: The settings, clamped to what the output delivers.
    :raises ValueError: If no output goes by that name.
    """
    return _tx_power(PaOutput(output).value, output_dbm)


def tx_power_under_ceiling(
    output: PaOutput | str, budget: LinkBudget, eirp_ceiling_dbm: float
) -> TxPower:
    """Choose the amplifier settings that keep a link's EIRP at or under a ceiling.

    :param output: The amplifier output the module uses.
    :param budget: The link budget, whose transmitting antenna and cable apply.
    :param eirp_ceiling_dbm: The EIRP limit in dBm.
    :returns: The settings, rounded down to whole decibels.
    :raises ValueError: If no output goes by that name.
    """
    return _tx_power_under_ceiling(PaOutput(output).value, budget, eirp_ceiling_dbm)


def ocp_register(milliamps: int) -> int:
    """Return RegOcp for a current limit.

    :param milliamps: The most current the amplifier may draw.
    :returns: The register value with the protection on.
    """
    return _ocp_register(milliamps)


def invert_iq(receive: bool, transmit: bool) -> int:
    """Return RegInvertIQ for the IQ polarity of each path.

    :param receive: Whether to invert the receive path, as a LoRaWAN device does for downlinks.
    :param transmit: Whether to invert the transmit path, as a gateway does.
    :returns: The register value, with the transmit bit set for normal IQ as the reference
        drivers have it.
    """
    return _invert_iq(receive, transmit)


def invert_iq_2(inverted: bool) -> int:
    """Return RegInvertIQ2 for the path in use.

    :param inverted: Whether that path is inverted.
    :returns: 0x19 when inverted, else 0x1D.
    """
    return _invert_iq_2(inverted)


def high_bw_optimize(link: LoraLink, frequency_hz: int) -> HighBwOptimize:
    """Return the writes of the 500 kHz sensitivity erratum.

    :param link: The link settings, whose bandwidth decides.
    :param frequency_hz: The carrier frequency in hertz.
    :returns: RegHighBwOptimize1 and, where it is written, RegHighBwOptimize2.
    :raises PamojaError: If the SX127x has no such bandwidth.
    """
    return _high_bw_optimize(link, frequency_hz)


def spurious_reception(link: LoraLink) -> SpuriousReception:
    """Return the receive settings of the spurious reception erratum.

    :param link: The link settings, whose bandwidth decides.
    :returns: The automatic IF, the hand-set IF, and the carrier offset.
    :raises PamojaError: If the SX127x has no such bandwidth.
    """
    return _spurious_reception(link)


def image_cal_start(current: int) -> int:
    """Return RegImageCal to start a calibration.

    :param current: The register's current value.
    :returns: The value with ImageCalStart set and AutoImageCalOn clear.
    """
    return _image_cal_start(current)


def automatic_if(current: int, on: bool) -> int:
    """Return RegDetectOptimize with AutomaticIFOn set or clear.

    :param current: The register's current value.
    :param on: Whether the automatic IF stays on.
    :returns: The register value.
    """
    return _automatic_if(current, on)


def packet_status(answer: bytes, frequency_hz: int) -> PacketStatus:
    """Decode RegPktSnrValue and RegPktRssiValue, read together.

    :param answer: The two register values, SNR first.
    :param frequency_hz: The carrier the packet was heard at, which picks the RF port's offset.
    :returns: The three signal levels, exact to a hundredth of a decibel.
    :raises ValueError: If the answer is not two bytes.
    """
    return _packet_status(bytes(answer), frequency_hz)


def rssi_dbm(byte: int, frequency_hz: int) -> float:
    """Decode RegRssiValue.

    :param byte: The register value.
    :param frequency_hz: The carrier the receiver is tuned to.
    :returns: The signal power the receiver hears right now, in dBm.
    """
    return _rssi_dbm(byte, frequency_hz)


def modem_status(byte: int) -> ModemStatus:
    """Decode RegModemStat.

    :param byte: The register value.
    :returns: The modem's live state.
    """
    return _modem_status(byte)
