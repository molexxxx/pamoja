"""The Semtech SX1261, SX1262, and LLCC68, as the bytes they take and the answers they give.

Every command goes out in one SPI transaction framed by NSS, sent once the chip's BUSY line
is low, and a query's answer is read in the same transaction after its bytes. These
functions build the bytes and decode the answers from the SX1261/2 datasheet (Rev 2.2), so a
program with its own SPI access can drive the chip with nothing else.
"""

from __future__ import annotations

from enum import Enum, IntFlag

from pamoja._native import (
    LinkBudget,
    LoraLink,
    Sx126xPacketStatus as PacketStatus,
    Sx126xQuery as Query,
    Sx126xRxBufferStatus as RxBufferStatus,
    Sx126xStatus as Status,
    Sx126xTxPower as TxPower,
)
from pamoja._native import sx126x_calibrate_image as _calibrate_image
from pamoja._native import sx126x_clear_irq_status as _clear_irq_status
from pamoja._native import sx126x_constants as _constants
from pamoja._native import sx126x_device_errors as _device_errors
from pamoja._native import sx126x_frequency_word as _frequency_word
from pamoja._native import sx126x_get_device_errors as _get_device_errors
from pamoja._native import sx126x_get_irq_status as _get_irq_status
from pamoja._native import sx126x_get_packet_status as _get_packet_status
from pamoja._native import sx126x_get_rssi_inst as _get_rssi_inst
from pamoja._native import sx126x_get_rx_buffer_status as _get_rx_buffer_status
from pamoja._native import sx126x_get_status as _get_status
from pamoja._native import sx126x_image_calibration as _image_calibration
from pamoja._native import sx126x_irq as _irq
from pamoja._native import sx126x_packet_status as _packet_status
from pamoja._native import sx126x_ramp_time_us as _ramp_time_us
from pamoja._native import sx126x_read_buffer as _read_buffer
from pamoja._native import sx126x_read_register as _read_register
from pamoja._native import sx126x_rssi_inst_dbm as _rssi_inst_dbm
from pamoja._native import sx126x_rx_buffer_status as _rx_buffer_status
from pamoja._native import sx126x_set_dio_irq_params as _set_dio_irq_params
from pamoja._native import sx126x_set_lora_modulation_params as _set_lora_modulation_params
from pamoja._native import sx126x_set_lora_packet_params as _set_lora_packet_params
from pamoja._native import sx126x_set_pa_config as _set_pa_config
from pamoja._native import sx126x_set_packet_type_lora as _set_packet_type_lora
from pamoja._native import sx126x_set_rf_frequency as _set_rf_frequency
from pamoja._native import sx126x_set_rx as _set_rx
from pamoja._native import sx126x_set_rx_continuous as _set_rx_continuous
from pamoja._native import sx126x_set_sleep as _set_sleep
from pamoja._native import sx126x_set_standby as _set_standby
from pamoja._native import sx126x_set_tx as _set_tx
from pamoja._native import sx126x_set_tx_params as _set_tx_params
from pamoja._native import sx126x_status as _status
from pamoja._native import sx126x_timeout_steps as _timeout_steps
from pamoja._native import sx126x_tx_power as _tx_power
from pamoja._native import sx126x_tx_power_under_ceiling as _tx_power_under_ceiling
from pamoja._native import sx126x_write_buffer as _write_buffer
from pamoja._native import sx126x_write_register as _write_register

__all__ = [
    "REGISTER_LORA_SYNC_WORD",
    "RX_CONTINUOUS",
    "SYNC_WORD_PRIVATE",
    "SYNC_WORD_PUBLIC",
    "Amplifier",
    "DeviceError",
    "Irq",
    "PacketStatus",
    "Query",
    "RxBufferStatus",
    "Status",
    "TxPower",
    "calibrate_image",
    "clear_irq_status",
    "device_errors",
    "frequency_word",
    "get_device_errors",
    "get_irq_status",
    "get_packet_status",
    "get_rssi_inst",
    "get_rx_buffer_status",
    "get_status",
    "image_calibration",
    "irq",
    "packet_status",
    "ramp_time_us",
    "read_buffer",
    "read_register",
    "rssi_inst_dbm",
    "rx_buffer_status",
    "set_dio_irq_params",
    "set_lora_modulation_params",
    "set_lora_packet_params",
    "set_pa_config",
    "set_packet_type_lora",
    "set_rf_frequency",
    "set_rx",
    "set_rx_continuous",
    "set_sleep",
    "set_standby",
    "set_tx",
    "set_tx_params",
    "status",
    "timeout_steps",
    "tx_power",
    "tx_power_under_ceiling",
    "write_buffer",
    "write_register",
]

_CONSTANTS = _constants()

#: The receive timeout word that keeps the chip listening until another command stops it.
RX_CONTINUOUS = _CONSTANTS["RX_CONTINUOUS"]
#: The LoRa sync word of a public network such as LoRaWAN.
SYNC_WORD_PUBLIC = _CONSTANTS["SYNC_WORD_PUBLIC"]
#: The LoRa sync word of a private network, and the chip's reset value.
SYNC_WORD_PRIVATE = _CONSTANTS["SYNC_WORD_PRIVATE"]
#: The register that holds the most significant byte of the LoRa sync word.
REGISTER_LORA_SYNC_WORD = _CONSTANTS["REGISTER_LORA_SYNC_WORD"]


class Amplifier(str, Enum):
    """Which power amplifier a chip has.

    The SPI interface cannot tell the chips apart, so the caller names the amplifier.
    """

    #: The low power amplifier of the SX1261, up to +15 dBm.
    LOW_POWER = "LowPower"
    #: The high power amplifier of the SX1262 and the LLCC68, up to +22 dBm.
    HIGH_POWER = "HighPower"


class Irq(IntFlag):
    """The interrupt bits of the IRQ register, from Table 13-29 of the datasheet."""

    #: A packet has been sent.
    TX_DONE = _CONSTANTS["IRQ_TX_DONE"]
    #: A packet has been received.
    RX_DONE = _CONSTANTS["IRQ_RX_DONE"]
    #: A preamble has been detected.
    PREAMBLE_DETECTED = _CONSTANTS["IRQ_PREAMBLE_DETECTED"]
    #: A valid (G)FSK sync word has been detected.
    SYNC_WORD_VALID = _CONSTANTS["IRQ_SYNC_WORD_VALID"]
    #: A valid LoRa header has been received.
    HEADER_VALID = _CONSTANTS["IRQ_HEADER_VALID"]
    #: A LoRa header failed its CRC.
    HEADER_ERROR = _CONSTANTS["IRQ_HEADER_ERROR"]
    #: A packet failed its CRC.
    CRC_ERROR = _CONSTANTS["IRQ_CRC_ERROR"]
    #: Channel activity detection has finished.
    CAD_DONE = _CONSTANTS["IRQ_CAD_DONE"]
    #: Channel activity detection heard LoRa.
    CAD_DETECTED = _CONSTANTS["IRQ_CAD_DETECTED"]
    #: A transmission or a reception timed out.
    TIMEOUT = _CONSTANTS["IRQ_TIMEOUT"]
    #: A long-range FHSS hop is due.
    LR_FHSS_HOP = _CONSTANTS["IRQ_LR_FHSS_HOP"]


class DeviceError(IntFlag):
    """The device error bits GetDeviceErrors answers with."""

    #: The RC64k oscillator failed to calibrate.
    RC64K_CALIBRATION = _CONSTANTS["ERROR_RC64K_CALIBRATION"]
    #: The RC13M oscillator failed to calibrate.
    RC13M_CALIBRATION = _CONSTANTS["ERROR_RC13M_CALIBRATION"]
    #: The PLL failed to calibrate.
    PLL_CALIBRATION = _CONSTANTS["ERROR_PLL_CALIBRATION"]
    #: The ADC failed to calibrate.
    ADC_CALIBRATION = _CONSTANTS["ERROR_ADC_CALIBRATION"]
    #: Image rejection failed to calibrate.
    IMAGE_CALIBRATION = _CONSTANTS["ERROR_IMAGE_CALIBRATION"]
    #: The crystal oscillator failed to start, which a TCXO raises until it is powered.
    XOSC_START = _CONSTANTS["ERROR_XOSC_START"]
    #: The PLL failed to lock.
    PLL_LOCK = _CONSTANTS["ERROR_PLL_LOCK"]
    #: The power amplifier failed to ramp.
    PA_RAMP = _CONSTANTS["ERROR_PA_RAMP"]


def frequency_word(frequency_hz: int) -> int:
    """Return the word SetRfFrequency takes for a frequency.

    :param frequency_hz: The carrier frequency in hertz.
    :returns: The frequency times 2^25 over the 32 MHz crystal, rounded to the nearest step.

    >>> hex(frequency_word(868_100_000))
    '0x3641999a'
    """
    return _frequency_word(frequency_hz)


def timeout_steps(timeout_us: int) -> int:
    """Return the 24-bit timeout word SetTx and SetRx take for a duration.

    :param timeout_us: The duration in microseconds.
    :returns: The number of 15.625 us steps; a nonzero duration never becomes the zero word
        that disables the timeout.

    >>> timeout_steps(1_000_000)
    64000
    """
    return _timeout_steps(timeout_us)


def image_calibration(low_hz: int, high_hz: int) -> bytes:
    """Return the two CalibrateImage codes that cover a band.

    :param low_hz: The lower edge of the band in hertz.
    :param high_hz: The upper edge of the band in hertz.
    :returns: ``freq1`` and ``freq2``, 4 MHz steps that always cover the band.

    >>> image_calibration(863_000_000, 870_000_000).hex()
    'd7da'
    """
    return _image_calibration(low_hz, high_hz)


def ramp_time_us(at_least_us: int) -> int:
    """Return the shortest amplifier ramp time the chip offers that lasts at least a duration.

    :param at_least_us: The least ramp time wanted, in microseconds.
    :returns: One of the eight ramp times of Table 13-41, in microseconds.
    """
    return _ramp_time_us(at_least_us)


def tx_power(amplifier: Amplifier | str, output_dbm: int) -> TxPower:
    """Choose the amplifier settings for an output power.

    :param amplifier: The chip's amplifier.
    :param output_dbm: The output power wanted at the antenna port, in dBm.
    :returns: The configuration and the setting, clamped to what the amplifier allows.
    :raises ValueError: If no amplifier goes by that name.
    """
    return _tx_power(Amplifier(amplifier).value, output_dbm)


def tx_power_under_ceiling(
    amplifier: Amplifier | str, budget: LinkBudget, eirp_ceiling_dbm: float
) -> TxPower:
    """Choose the amplifier settings that keep a link's EIRP at or under a ceiling.

    :param amplifier: The chip's amplifier.
    :param budget: The link budget, whose transmitting antenna and cable apply.
    :param eirp_ceiling_dbm: The EIRP limit, such as a channel plan's ceiling for the
        frequency.
    :returns: The configuration and the setting, rounded down to whole decibels so the EIRP
        stays under the ceiling.
    :raises ValueError: If no amplifier goes by that name.
    """
    return _tx_power_under_ceiling(Amplifier(amplifier).value, budget, eirp_ceiling_dbm)


def set_standby() -> bytes:
    """Return SetStandby into STDBY_RC, which stops a transmission or a reception."""
    return _set_standby()


def set_packet_type_lora() -> bytes:
    """Return SetPacketType for LoRa, the first radio setting a configuration sends."""
    return _set_packet_type_lora()


def set_rf_frequency(frequency_hz: int) -> bytes:
    """Return SetRfFrequency for a carrier frequency.

    :param frequency_hz: The carrier frequency in hertz.
    :returns: The command bytes.
    """
    return _set_rf_frequency(frequency_hz)


def calibrate_image(low_hz: int, high_hz: int) -> bytes:
    """Return CalibrateImage over a band.

    :param low_hz: The lower edge of the band in hertz.
    :param high_hz: The upper edge of the band in hertz.
    :returns: The command bytes.
    """
    return _calibrate_image(low_hz, high_hz)


def set_pa_config(power: TxPower) -> bytes:
    """Return SetPaConfig for a power setting.

    :param power: The settings from :func:`tx_power` or :func:`tx_power_under_ceiling`.
    :returns: The command bytes.
    """
    return _set_pa_config(power)


def set_tx_params(power: TxPower, ramp_us: int) -> bytes:
    """Return SetTxParams for a power setting and a ramp time.

    :param power: The power settings.
    :param ramp_us: The least amplifier ramp time wanted, in microseconds.
    :returns: The command bytes.
    """
    return _set_tx_params(power, ramp_us)


def set_lora_modulation_params(link: LoraLink) -> bytes:
    """Return SetModulationParams for a LoRa link.

    :param link: The link settings, from :mod:`pamoja.lora`.
    :returns: The command bytes.
    :raises PamojaError: If the link's bandwidth is not one the SX126x offers.
    """
    return _set_lora_modulation_params(link)


def set_lora_packet_params(link: LoraLink, payload_len: int, invert_iq: bool) -> bytes:
    """Return SetPacketParams for a LoRa link and a payload.

    :param link: The link settings, whose preamble, header, and CRC the frame uses.
    :param payload_len: The payload length to send, or the most a receiver accepts.
    :param invert_iq: Whether the IQ polarity is inverted, as LoRaWAN downlinks use.
    :returns: The command bytes.
    """
    return _set_lora_packet_params(link, payload_len, invert_iq)


def set_dio_irq_params(irq: int, dio1: int, dio2: int = 0, dio3: int = 0) -> bytes:
    """Return SetDioIrqParams: which interrupts are enabled, and which DIO lines raise them.

    :param irq: The interrupts to enable, as :class:`Irq` bits.
    :param dio1: The interrupts routed to DIO1.
    :param dio2: The interrupts routed to DIO2.
    :param dio3: The interrupts routed to DIO3.
    :returns: The command bytes.
    """
    return _set_dio_irq_params(int(irq), int(dio1), int(dio2), int(dio3))


def clear_irq_status(irq: int) -> bytes:
    """Return ClearIrqStatus for a set of interrupts.

    :param irq: The interrupts to clear, as :class:`Irq` bits.
    :returns: The command bytes.
    """
    return _clear_irq_status(int(irq))


def set_tx(timeout_us: int) -> bytes:
    """Return SetTx with a timeout.

    :param timeout_us: How long the chip may transmit before it raises TIMEOUT, in
        microseconds; ``0`` disables the timeout.
    :returns: The command bytes.
    """
    return _set_tx(timeout_us)


def set_rx(timeout_us: int) -> bytes:
    """Return SetRx with a timeout.

    :param timeout_us: How long the chip listens for a packet to start, in microseconds;
        ``0`` listens for one packet with no timeout.
    :returns: The command bytes.
    """
    return _set_rx(timeout_us)


def set_rx_continuous() -> bytes:
    """Return SetRx in continuous mode, receiving packet after packet until another command."""
    return _set_rx_continuous()


def set_sleep(warm_start: bool) -> bytes:
    """Return SetSleep, without an RTC wake-up.

    :param warm_start: Whether to keep the configuration in retention while asleep.
    :returns: The command bytes.
    """
    return _set_sleep(warm_start)


def write_register(address: int, values: bytes) -> bytes:
    """Return a whole WriteRegister transaction.

    :param address: The first register's address, such as :data:`REGISTER_LORA_SYNC_WORD`.
    :param values: The register values, one byte each.
    :returns: The opcode, the address, and the values.
    """
    return _write_register(address, bytes(values))


def write_buffer(offset: int, payload: bytes) -> bytes:
    """Return a whole WriteBuffer transaction.

    :param offset: Where in the data buffer the first byte goes.
    :param payload: The bytes to write.
    :returns: The opcode, the offset, and the payload.
    """
    return _write_buffer(offset, bytes(payload))


def get_status() -> Query:
    """Return GetStatus, answered by the status byte; decode it with :func:`status`."""
    return _get_status()


def get_irq_status() -> Query:
    """Return GetIrqStatus, answered by two IRQ bytes; decode them with :func:`irq`."""
    return _get_irq_status()


def get_rx_buffer_status() -> Query:
    """Return GetRxBufferStatus, answered by two bytes; decode them with :func:`rx_buffer_status`."""
    return _get_rx_buffer_status()


def get_packet_status() -> Query:
    """Return GetPacketStatus, answered by three bytes; decode them with :func:`packet_status`."""
    return _get_packet_status()


def get_rssi_inst() -> Query:
    """Return GetRssiInst, answered by one byte; decode it with :func:`rssi_inst_dbm`."""
    return _get_rssi_inst()


def get_device_errors() -> Query:
    """Return GetDeviceErrors, answered by two bytes; decode them with :func:`device_errors`."""
    return _get_device_errors()


def read_register(address: int, length: int) -> Query:
    """Return ReadRegister for a run of consecutive registers.

    :param address: The first register's address.
    :param length: How many registers to read.
    :returns: The query.
    """
    return _read_register(address, length)


def read_buffer(offset: int, length: int) -> Query:
    """Return ReadBuffer for a run of the data buffer.

    :param offset: Where in the buffer the first byte is.
    :param length: How many bytes to read.
    :returns: The query.
    """
    return _read_buffer(offset, length)


def status(byte: int) -> Status:
    """Decode a status byte.

    :param byte: The status byte.
    :returns: The chip's mode, how its last command went, and whether that was an error.

    >>> status(0x2C).chip_mode
    'StandbyRc'
    """
    return _status(byte)


def irq(answer: bytes) -> Irq:
    """Decode a GetIrqStatus answer.

    :param answer: The two answer bytes.
    :returns: The pending interrupts.
    :raises ValueError: If the answer is not two bytes.
    """
    return Irq(_irq(bytes(answer)))


def device_errors(answer: bytes) -> DeviceError:
    """Decode a GetDeviceErrors answer.

    :param answer: The two answer bytes.
    :returns: The flagged errors.
    :raises ValueError: If the answer is not two bytes.
    """
    return DeviceError(_device_errors(bytes(answer)))


def packet_status(answer: bytes) -> PacketStatus:
    """Decode a LoRa GetPacketStatus answer.

    :param answer: RssiPkt, SnrPkt, and SignalRssiPkt.
    :returns: The three signal levels, exact to a hundredth of a decibel.
    :raises ValueError: If the answer is not three bytes.
    """
    return _packet_status(bytes(answer))


def rx_buffer_status(answer: bytes) -> RxBufferStatus:
    """Decode a GetRxBufferStatus answer.

    :param answer: PayloadLengthRx and RxStartBufferPointer.
    :returns: The payload length and where it starts in the data buffer.
    :raises ValueError: If the answer is not two bytes.
    """
    return _rx_buffer_status(bytes(answer))


def rssi_inst_dbm(byte: int) -> float:
    """Decode a GetRssiInst answer.

    :param byte: RssiInst.
    :returns: The signal power the receiver hears right now, in dBm.
    """
    return _rssi_inst_dbm(byte)
