"""Idiomatic sensor-driver facade.

These are the decode half of four parts a field node is likely to have wired to
it, turning the register bytes a bus driver read into the physical reading the
manufacturer's datasheet says they mean. Driving the bus is the caller's job;
getting the arithmetic right is this layer's.
"""

from __future__ import annotations

from pamoja._native import Ads1115Config, Bme280Calibration, Bme280Measurement, Ds18b20Reading
from pamoja._native import ads1115_config_bits as _ads1115_config_bits
from pamoja._native import ads1115_config_from_bits as _ads1115_config_from_bits
from pamoja._native import ads1115_full_scale_microvolts as _ads1115_full_scale_microvolts
from pamoja._native import ads1115_samples_per_second as _ads1115_samples_per_second
from pamoja._native import ads1115_to_nanovolts as _ads1115_to_nanovolts
from pamoja._native import ads1115_to_volts as _ads1115_to_volts
from pamoja._native import ds18b20_build_scratchpad as _ds18b20_build_scratchpad
from pamoja._native import ds18b20_celsius as _ds18b20_celsius
from pamoja._native import ds18b20_config_byte as _ds18b20_config_byte
from pamoja._native import ds18b20_crc8 as _ds18b20_crc8
from pamoja._native import ds18b20_max_conversion_micros as _ds18b20_max_conversion_micros
from pamoja._native import ds18b20_micro_celsius as _ds18b20_micro_celsius
from pamoja._native import ds18b20_parse_scratchpad as _ds18b20_parse_scratchpad
from pamoja._native import ds18b20_resolution_bits as _ds18b20_resolution_bits
from pamoja._native import ds18b20_step_micro_celsius as _ds18b20_step_micro_celsius
from pamoja._native import ina219_bus_register as _ina219_bus_register
from pamoja._native import ina219_current_register as _ina219_current_register
from pamoja._native import ina219_power_register as _ina219_power_register
from pamoja._native import ina219_shunt_register as _ina219_shunt_register
from pamoja._native import ina219_bus_millivolts as _ina219_bus_millivolts
from pamoja._native import ina219_calibration as _ina219_calibration
from pamoja._native import ina219_conversion_ready as _ina219_conversion_ready
from pamoja._native import ina219_current_microamps as _ina219_current_microamps
from pamoja._native import ina219_math_overflow as _ina219_math_overflow
from pamoja._native import (
    ina219_minimum_current_lsb_microamps as _ina219_minimum_current_lsb_microamps,
)
from pamoja._native import ina219_power_microwatts as _ina219_power_microwatts
from pamoja._native import ina219_shunt_microvolts as _ina219_shunt_microvolts

__all__ = [
    "Ads1115Config",
    "Bme280Calibration",
    "Bme280Measurement",
    "Ds18b20Reading",
    "ads1115",
    "bme280",
    "ds18b20",
    "ina219",
]


class _Bme280:
    """A Bosch BME280 temperature, pressure, and humidity sensor."""

    __slots__ = ()

    #: The address a BME280 answers on with its SDO pin low.
    ADDRESS_PRIMARY = 0x76
    #: The address it answers on with SDO high.
    ADDRESS_SECONDARY = 0x77
    #: The value its chip-ID register reads, which confirms the part.
    CHIP_ID = 0x60

    def calibration(self, temp_press: bytes, humidity: bytes) -> Bme280Calibration:
        """Read the factory calibration out of the registers, once at start-up.

        :param temp_press: The 26-byte temperature and pressure calibration block.
        :param humidity: The 7-byte humidity calibration block.
        :returns: The calibration, to reuse for every measurement.
        :raises ValueError: If either block is the wrong length.
        """
        return Bme280Calibration(bytes(temp_press), bytes(humidity))


class _Ds18b20:
    """A Maxim DS18B20 1-Wire thermometer."""

    __slots__ = ()

    #: The 1-Wire family code that identifies a DS18B20 on the bus.
    FAMILY_CODE = 0x28

    def parse_scratchpad(self, data: bytes) -> Ds18b20Reading:
        """Parse and CRC-check a nine-byte scratchpad.

        :param data: The scratchpad as the device sent it, the ninth byte its CRC.
        :returns: The decoded reading.
        :raises PamojaError: If the CRC does not match, which means the read was
            corrupted on the bus and should be repeated.
        """
        return _ds18b20_parse_scratchpad(bytes(data))

    def build_scratchpad(
        self, celsius: float, resolution_bits: int, alarm_high: int, alarm_low: int
    ) -> bytes:
        """Build the nine bytes a part in the given state puts on the bus.

        This is the inverse of :meth:`parse_scratchpad`, so a node can be written and
        tested against what a thermometer sends without one attached.

        :param celsius: The temperature the part is reading.
        :param resolution_bits: The resolution it is configured for, 9 to 12.
        :param alarm_high: The high alarm threshold in whole degrees Celsius.
        :param alarm_low: The low alarm threshold in whole degrees Celsius.
        :returns: The nine scratchpad bytes in transmission order, CRC last.
        :raises PamojaError: If the resolution is not 9, 10, 11, or 12 bits.
        """
        return bytes(
            _ds18b20_build_scratchpad(celsius, resolution_bits, alarm_high, alarm_low)
        )

    def crc8(self, data: bytes) -> int:
        """Compute the Maxim CRC-8 a 1-Wire device checks its own bytes with.

        :param data: The bytes the checksum covers.
        :returns: The checksum.
        """
        return _ds18b20_crc8(bytes(data))

    def micro_celsius(self, raw: int) -> int:
        """Convert a raw temperature register to micro-degrees Celsius.

        :param raw: The 16-bit two's-complement register.
        :returns: The temperature, exact in integer arithmetic.
        """
        return _ds18b20_micro_celsius(raw)

    def celsius(self, raw: int) -> float:
        """Convert a raw temperature register to degrees Celsius.

        :param raw: The 16-bit two's-complement register.
        :returns: The temperature.
        """
        return _ds18b20_celsius(raw)

    def config_byte(self, bits: int) -> int:
        """Return the configuration byte that selects a resolution.

        :param bits: The resolution in bits: 9, 10, 11, or 12.
        :returns: The byte to write to the configuration register.
        :raises ValueError: If the resolution is not one the part offers.
        """
        return _ds18b20_config_byte(bits)

    def resolution_bits(self, config_byte: int) -> int:
        """Return the resolution a configuration byte selects.

        :param config_byte: The byte read from the configuration register.
        :returns: The resolution in bits.
        """
        return _ds18b20_resolution_bits(config_byte)

    def step_micro_celsius(self, bits: int) -> int:
        """Return the temperature step a resolution resolves.

        :param bits: The resolution in bits.
        :returns: The step in micro-degrees Celsius.
        :raises ValueError: If the resolution is not one the part offers.
        """
        return _ds18b20_step_micro_celsius(bits)

    def max_conversion_micros(self, bits: int) -> int:
        """Return how long a conversion may take at a resolution.

        :param bits: The resolution in bits.
        :returns: The datasheet's worst case, in microseconds.
        :raises ValueError: If the resolution is not one the part offers.
        """
        return _ds18b20_max_conversion_micros(bits)


class _Ina219:
    """A TI INA219 current, voltage, and power monitor."""

    __slots__ = ()

    def calibration(self, current_lsb_microamps: int, shunt_milliohms: int) -> int:
        """Compute the calibration register for a shunt and current resolution.

        :param current_lsb_microamps: The microamps per count the current register
            should carry.
        :param shunt_milliohms: The shunt resistor value.
        :returns: The register value to write.
        """
        return _ina219_calibration(current_lsb_microamps, shunt_milliohms)

    def minimum_current_lsb_microamps(self, max_expected_microamps: int) -> int:
        """Return the smallest current resolution that still covers a maximum.

        :param max_expected_microamps: The largest current the application measures.
        :returns: The minimum current LSB in microamps.
        """
        return _ina219_minimum_current_lsb_microamps(max_expected_microamps)

    def shunt_register(self, microvolts: int) -> int:
        """Build the shunt-voltage register a monitor reports for a shunt voltage.

        The inverse of :meth:`shunt_microvolts`, so a node can be written and tested
        against what a monitor sends without one attached.

        :param microvolts: The shunt voltage in microvolts.
        :returns: The signed register value, at 10 uV per count.
        """
        return _ina219_shunt_register(microvolts)

    def bus_register(self, millivolts: int) -> int:
        """Build the bus-voltage register a monitor reports for a bus voltage.

        :param millivolts: The bus voltage in millivolts.
        :returns: The register value, with the conversion-ready flag set.
        """
        return _ina219_bus_register(millivolts)

    def current_register(self, microamps: int, current_lsb_microamps: int) -> int:
        """Build the current register a monitor reports for a current.

        :param microamps: The current in microamps.
        :param current_lsb_microamps: The current LSB the calibration was set for.
        :returns: The signed register value.
        """
        return _ina219_current_register(microamps, current_lsb_microamps)

    def power_register(self, microwatts: int, current_lsb_microamps: int) -> int:
        """Build the power register a monitor reports for a power.

        :param microwatts: The power in microwatts.
        :param current_lsb_microamps: The current LSB the calibration was set for.
        :returns: The register value.
        """
        return _ina219_power_register(microwatts, current_lsb_microamps)

    def shunt_microvolts(self, raw: int) -> int:
        """Convert a raw shunt-voltage register to microvolts.

        :param raw: The signed register value.
        :returns: The shunt voltage.
        """
        return _ina219_shunt_microvolts(raw)

    def bus_millivolts(self, raw: int) -> int:
        """Convert a raw bus-voltage register to millivolts.

        :param raw: The register value.
        :returns: The bus voltage.
        """
        return _ina219_bus_millivolts(raw)

    def conversion_ready(self, raw: int) -> bool:
        """Report whether a bus-voltage register says a conversion is ready.

        :param raw: The register value.
        :returns: Whether the conversion-ready flag is set.
        """
        return _ina219_conversion_ready(raw)

    def math_overflow(self, raw: int) -> bool:
        """Report whether a bus-voltage register flags a math overflow.

        :param raw: The register value.
        :returns: Whether the current and power readings are meaningless, which
            means the calibration needs revisiting.
        """
        return _ina219_math_overflow(raw)

    def current_microamps(self, raw: int, current_lsb_microamps: int) -> int:
        """Convert a raw current register to microamps.

        :param raw: The signed register value.
        :param current_lsb_microamps: The resolution the calibration selected.
        :returns: The current.
        """
        return _ina219_current_microamps(raw, current_lsb_microamps)

    def power_microwatts(self, raw: int, current_lsb_microamps: int) -> int:
        """Convert a raw power register to microwatts.

        :param raw: The register value.
        :param current_lsb_microamps: The resolution the calibration selected.
        :returns: The power. The power LSB is fixed at twenty times the current LSB.
        """
        return _ina219_power_microwatts(raw, current_lsb_microamps)


class _Ads1115:
    """A TI ADS1115 16-bit analogue-to-digital converter."""

    __slots__ = ()

    #: The value the configuration register reads after a reset.
    CONFIG_RESET = 0x8583

    def config_bits(self, config: Ads1115Config) -> int:
        """Assemble the 16-bit configuration register value.

        :param config: The settings to encode.
        :returns: The register value to write, most significant bit first.
        """
        return _ads1115_config_bits(config)

    def config_from_bits(self, bits: int) -> Ads1115Config:
        """Parse a 16-bit configuration register value.

        :param bits: The register value, as read from the device.
        :returns: The decoded settings. Every value decodes, so this never raises.
        """
        return _ads1115_config_from_bits(bits)

    def full_scale_microvolts(self, pga: int) -> int:
        """Return the full-scale range a gain code selects.

        :param pga: The gain code, 0 to 7.
        :returns: The full scale in microvolts.
        """
        return _ads1115_full_scale_microvolts(pga)

    def samples_per_second(self, data_rate: int) -> int:
        """Return the sample rate a data-rate code selects.

        :param data_rate: The data-rate code, 0 to 7.
        :returns: The rate in samples per second.
        """
        return _ads1115_samples_per_second(data_rate)

    def to_nanovolts(self, pga: int, raw: int) -> int:
        """Convert a raw conversion result to nanovolts.

        :param pga: The gain the conversion was taken at.
        :param raw: The signed conversion register value.
        :returns: The measured voltage, exact at every gain setting.
        """
        return _ads1115_to_nanovolts(pga, raw)

    def to_volts(self, pga: int, raw: int) -> float:
        """Convert a raw conversion result to volts.

        :param pga: The gain the conversion was taken at.
        :param raw: The signed conversion register value.
        :returns: The measured voltage.
        """
        return _ads1115_to_volts(pga, raw)


#: A Bosch BME280 temperature, pressure, and humidity sensor.
bme280 = _Bme280()

#: A Maxim DS18B20 1-Wire thermometer.
ds18b20 = _Ds18b20()

#: A TI INA219 current, voltage, and power monitor.
ina219 = _Ina219()

#: A TI ADS1115 16-bit analogue-to-digital converter.
ads1115 = _Ads1115()
