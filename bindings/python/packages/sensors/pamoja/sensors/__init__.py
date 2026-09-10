"""Idiomatic sensor-driver facade.

These are the decode half of eleven parts a field node is likely to have wired to
it, turning the register bytes a bus driver read into the physical reading the
manufacturer's datasheet says they mean. Driving the bus is the caller's job;
getting the arithmetic right is this layer's.
"""

from __future__ import annotations

import enum

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
from pamoja._native import (
    Bmp280Calibration,
    Bmp280Coefficients,
    Bmp280Config,
    Bmp280CtrlMeas,
    Bmp280RawMeasurement,
    Bmp280Reading,
    Hdc1080Config,
    Hdc1080Measurement,
    Ina226Config,
    Ina226DieId,
    Ina226MaskEnable,
    Opt3001Config,
    Scd4xMeasurement,
    Sht3xMeasurement,
    Sht3xStatus,
    Tmp117Config,
)
from pamoja._native import bmp280_config_bits as _bmp280_config_bits
from pamoja._native import bmp280_config_from_bits as _bmp280_config_from_bits
from pamoja._native import bmp280_ctrl_meas_bits as _bmp280_ctrl_meas_bits
from pamoja._native import bmp280_ctrl_meas_from_bits as _bmp280_ctrl_meas_from_bits
from pamoja._native import bmp280_image_updating as _bmp280_image_updating
from pamoja._native import bmp280_measurement_bytes as _bmp280_measurement_bytes
from pamoja._native import bmp280_measuring as _bmp280_measuring
from pamoja._native import bmp280_oversampling_factor as _bmp280_oversampling_factor
from pamoja._native import bmp280_parse_measurement as _bmp280_parse_measurement
from pamoja._native import bmp280_pressure_skipped as _bmp280_pressure_skipped
from pamoja._native import bmp280_standby_micros as _bmp280_standby_micros
from pamoja._native import bmp280_temperature_skipped as _bmp280_temperature_skipped
from pamoja._native import hdc1080_celsius as _hdc1080_celsius
from pamoja._native import hdc1080_config_from_register as _hdc1080_config_from_register
from pamoja._native import hdc1080_config_to_register as _hdc1080_config_to_register
from pamoja._native import hdc1080_conversion_time_micros as _hdc1080_conversion_time_micros
from pamoja._native import (
    hdc1080_humidity_conversion_micros as _hdc1080_humidity_conversion_micros,
)
from pamoja._native import hdc1080_humidity_register as _hdc1080_humidity_register
from pamoja._native import hdc1080_measurement_bytes as _hdc1080_measurement_bytes
from pamoja._native import hdc1080_measurement_from_physical as _hdc1080_measurement_from_physical
from pamoja._native import hdc1080_milli_celsius as _hdc1080_milli_celsius
from pamoja._native import hdc1080_milli_percent as _hdc1080_milli_percent
from pamoja._native import hdc1080_parse_measurement as _hdc1080_parse_measurement
from pamoja._native import hdc1080_relative_humidity as _hdc1080_relative_humidity
from pamoja._native import hdc1080_serial_id as _hdc1080_serial_id
from pamoja._native import hdc1080_serial_id_registers as _hdc1080_serial_id_registers
from pamoja._native import (
    hdc1080_temperature_conversion_micros as _hdc1080_temperature_conversion_micros,
)
from pamoja._native import hdc1080_temperature_register as _hdc1080_temperature_register
from pamoja._native import ina226_active_alert_function as _ina226_active_alert_function
from pamoja._native import ina226_address as _ina226_address
from pamoja._native import ina226_averaging_samples as _ina226_averaging_samples
from pamoja._native import ina226_bus_microvolts as _ina226_bus_microvolts
from pamoja._native import ina226_bus_register as _ina226_bus_register
from pamoja._native import ina226_bus_volts as _ina226_bus_volts
from pamoja._native import ina226_calibration as _ina226_calibration
from pamoja._native import ina226_config_from_register as _ina226_config_from_register
from pamoja._native import ina226_config_to_register as _ina226_config_to_register
from pamoja._native import ina226_conversion_micros as _ina226_conversion_micros
from pamoja._native import ina226_current_amps as _ina226_current_amps
from pamoja._native import ina226_current_microamps as _ina226_current_microamps
from pamoja._native import ina226_current_register as _ina226_current_register
from pamoja._native import (
    ina226_current_register_from_shunt as _ina226_current_register_from_shunt,
)
from pamoja._native import ina226_die_id as _ina226_die_id
from pamoja._native import ina226_identify as _ina226_identify
from pamoja._native import ina226_is_continuous as _ina226_is_continuous
from pamoja._native import ina226_mask_enable_from_register as _ina226_mask_enable_from_register
from pamoja._native import ina226_mask_enable_to_register as _ina226_mask_enable_to_register
from pamoja._native import ina226_measures_bus as _ina226_measures_bus
from pamoja._native import ina226_measures_shunt as _ina226_measures_shunt
from pamoja._native import (
    ina226_minimum_current_lsb_microamps as _ina226_minimum_current_lsb_microamps,
)
from pamoja._native import ina226_power_microwatts as _ina226_power_microwatts
from pamoja._native import ina226_power_register as _ina226_power_register
from pamoja._native import (
    ina226_power_register_from_current as _ina226_power_register_from_current,
)
from pamoja._native import ina226_power_watts as _ina226_power_watts
from pamoja._native import ina226_shunt_millivolts as _ina226_shunt_millivolts
from pamoja._native import ina226_shunt_nanovolts as _ina226_shunt_nanovolts
from pamoja._native import ina226_shunt_register as _ina226_shunt_register
from pamoja._native import ina226_update_micros as _ina226_update_micros
from pamoja._native import opt3001_config_bits as _opt3001_config_bits
from pamoja._native import opt3001_config_from_bits as _opt3001_config_from_bits
from pamoja._native import opt3001_conversion_millis as _opt3001_conversion_millis
from pamoja._native import opt3001_fault_count as _opt3001_fault_count
from pamoja._native import opt3001_full_scale_milli_lux as _opt3001_full_scale_milli_lux
from pamoja._native import opt3001_is_automatic_range as _opt3001_is_automatic_range
from pamoja._native import opt3001_lsb_milli_lux as _opt3001_lsb_milli_lux
from pamoja._native import opt3001_lux as _opt3001_lux
from pamoja._native import opt3001_milli_lux as _opt3001_milli_lux
from pamoja._native import opt3001_raw_from_milli_lux as _opt3001_raw_from_milli_lux
from pamoja._native import opt3001_word_from_bytes as _opt3001_word_from_bytes
from pamoja._native import opt3001_word_to_bytes as _opt3001_word_to_bytes
from pamoja._native import scd4x_allowed_during_measurement as _scd4x_allowed_during_measurement
from pamoja._native import scd4x_ambient_pressure_pascals as _scd4x_ambient_pressure_pascals
from pamoja._native import scd4x_ambient_pressure_word as _scd4x_ambient_pressure_word
from pamoja._native import (
    scd4x_automatic_self_calibration_enabled as _scd4x_automatic_self_calibration_enabled,
)
from pamoja._native import (
    scd4x_automatic_self_calibration_word as _scd4x_automatic_self_calibration_word,
)
from pamoja._native import scd4x_celsius as _scd4x_celsius
from pamoja._native import scd4x_command_frame as _scd4x_command_frame
from pamoja._native import scd4x_crc as _scd4x_crc
from pamoja._native import scd4x_data_ready as _scd4x_data_ready
from pamoja._native import (
    scd4x_forced_recalibration_correction_ppm as _scd4x_forced_recalibration_correction_ppm,
)
from pamoja._native import scd4x_forced_recalibration_word as _scd4x_forced_recalibration_word
from pamoja._native import scd4x_humidity_milli_percent as _scd4x_humidity_milli_percent
from pamoja._native import scd4x_humidity_raw as _scd4x_humidity_raw
from pamoja._native import scd4x_max_duration_ms as _scd4x_max_duration_ms
from pamoja._native import scd4x_measurement_bytes as _scd4x_measurement_bytes
from pamoja._native import scd4x_measurement_from_physical as _scd4x_measurement_from_physical
from pamoja._native import scd4x_milli_celsius as _scd4x_milli_celsius
from pamoja._native import scd4x_parse_measurement as _scd4x_parse_measurement
from pamoja._native import scd4x_relative_humidity_percent as _scd4x_relative_humidity_percent
from pamoja._native import scd4x_self_test_passed as _scd4x_self_test_passed
from pamoja._native import scd4x_serial_number as _scd4x_serial_number
from pamoja._native import scd4x_serial_number_frame as _scd4x_serial_number_frame
from pamoja._native import (
    scd4x_temperature_offset_milli_celsius as _scd4x_temperature_offset_milli_celsius,
)
from pamoja._native import scd4x_temperature_offset_word as _scd4x_temperature_offset_word
from pamoja._native import scd4x_temperature_raw as _scd4x_temperature_raw
from pamoja._native import scd4x_word as _scd4x_word
from pamoja._native import scd4x_word_frame as _scd4x_word_frame
from pamoja._native import scd4x_write_frame as _scd4x_write_frame
from pamoja._native import sht3x_celsius as _sht3x_celsius
from pamoja._native import sht3x_crc as _sht3x_crc
from pamoja._native import sht3x_fahrenheit as _sht3x_fahrenheit
from pamoja._native import (
    sht3x_humidity_raw_from_milli_percent as _sht3x_humidity_raw_from_milli_percent,
)
from pamoja._native import (
    sht3x_humidity_raw_from_relative_humidity as _sht3x_humidity_raw_from_relative_humidity,
)
from pamoja._native import sht3x_interval_micros as _sht3x_interval_micros
from pamoja._native import sht3x_max_measurement_micros as _sht3x_max_measurement_micros
from pamoja._native import sht3x_measurement_bytes as _sht3x_measurement_bytes
from pamoja._native import sht3x_milli_celsius as _sht3x_milli_celsius
from pamoja._native import sht3x_milli_fahrenheit as _sht3x_milli_fahrenheit
from pamoja._native import sht3x_milli_percent as _sht3x_milli_percent
from pamoja._native import sht3x_parse_measurement as _sht3x_parse_measurement
from pamoja._native import sht3x_parse_status as _sht3x_parse_status
from pamoja._native import sht3x_periodic as _sht3x_periodic
from pamoja._native import sht3x_relative_humidity as _sht3x_relative_humidity
from pamoja._native import sht3x_single_shot as _sht3x_single_shot
from pamoja._native import sht3x_status_bytes as _sht3x_status_bytes
from pamoja._native import sht3x_status_from_bits as _sht3x_status_from_bits
from pamoja._native import (
    sht3x_temperature_raw_from_celsius as _sht3x_temperature_raw_from_celsius,
)
from pamoja._native import (
    sht3x_temperature_raw_from_milli_celsius as _sht3x_temperature_raw_from_milli_celsius,
)
from pamoja._native import (
    sht3x_temperature_raw_from_milli_fahrenheit as _sht3x_temperature_raw_from_milli_fahrenheit,
)
from pamoja._native import sht3x_typical_measurement_micros as _sht3x_typical_measurement_micros
from pamoja._native import sht3x_word as _sht3x_word
from pamoja._native import sht3x_word_bytes as _sht3x_word_bytes
from pamoja._native import tmp117_averaging_conversions as _tmp117_averaging_conversions
from pamoja._native import tmp117_averaging_micros as _tmp117_averaging_micros
from pamoja._native import tmp117_celsius as _tmp117_celsius
from pamoja._native import tmp117_config_bits as _tmp117_config_bits
from pamoja._native import tmp117_config_from_bits as _tmp117_config_from_bits
from pamoja._native import tmp117_cycle_micros as _tmp117_cycle_micros
from pamoja._native import tmp117_cycle_nominal_micros as _tmp117_cycle_nominal_micros
from pamoja._native import tmp117_data_ready as _tmp117_data_ready
from pamoja._native import tmp117_device_id as _tmp117_device_id
from pamoja._native import tmp117_eeprom_busy as _tmp117_eeprom_busy
from pamoja._native import tmp117_eeprom_unlock_busy as _tmp117_eeprom_unlock_busy
from pamoja._native import tmp117_high_alert as _tmp117_high_alert
from pamoja._native import tmp117_low_alert as _tmp117_low_alert
from pamoja._native import tmp117_micro_celsius as _tmp117_micro_celsius
from pamoja._native import tmp117_nano_celsius as _tmp117_nano_celsius
from pamoja._native import tmp117_raw_from_celsius as _tmp117_raw_from_celsius
from pamoja._native import tmp117_raw_from_micro_celsius as _tmp117_raw_from_micro_celsius
from pamoja._native import tmp117_revision as _tmp117_revision
from pamoja._native import tmp117_temperature_bytes as _tmp117_temperature_bytes
from pamoja._native import tmp117_temperature_from_bytes as _tmp117_temperature_from_bytes

__all__ = [
    "Ads1115Config",
    "Bme280Calibration",
    "Bme280Measurement",
    "Bmp280Calibration",
    "Bmp280Coefficients",
    "Bmp280Config",
    "Bmp280CtrlMeas",
    "Bmp280RawMeasurement",
    "Bmp280Reading",
    "Ds18b20Reading",
    "Hdc1080Config",
    "Hdc1080Measurement",
    "Ina226AlertFunction",
    "Ina226Config",
    "Ina226DieId",
    "Ina226MaskEnable",
    "Opt3001Config",
    "Scd4xMeasurement",
    "Sht3xMeasurement",
    "Sht3xRate",
    "Sht3xRepeatability",
    "Sht3xStatus",
    "Tmp117Config",
    "ads1115",
    "bme280",
    "bmp280",
    "ds18b20",
    "hdc1080",
    "ina219",
    "ina226",
    "opt3001",
    "scd4x",
    "sht3x",
    "tmp117",
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


class Sht3xRepeatability(str, enum.Enum):
    """How hard an SHT3x works at one measurement, traded against time and power."""

    #: The fastest and least precise setting.
    LOW = "Low"
    #: The middle setting.
    MEDIUM = "Medium"
    #: The slowest and most precise setting.
    HIGH = "High"


class Sht3xRate(str, enum.Enum):
    """How often an SHT3x in periodic mode takes a measurement."""

    #: One measurement every two seconds.
    HALF_MPS = "HalfMps"
    #: One measurement per second.
    ONE_MPS = "OneMps"
    #: Two measurements per second.
    TWO_MPS = "TwoMps"
    #: Four measurements per second.
    FOUR_MPS = "FourMps"
    #: Ten measurements per second.
    TEN_MPS = "TenMps"


class Ina226AlertFunction(str, enum.Enum):
    """The limit comparison an INA226 alert pin responds to."""

    #: The shunt voltage rose above the alert limit.
    SHUNT_OVER_LIMIT = "ShuntOverLimit"
    #: The shunt voltage fell below the alert limit.
    SHUNT_UNDER_LIMIT = "ShuntUnderLimit"
    #: The bus voltage rose above the alert limit.
    BUS_OVER_LIMIT = "BusOverLimit"
    #: The bus voltage fell below the alert limit.
    BUS_UNDER_LIMIT = "BusUnderLimit"
    #: The power rose above the alert limit.
    POWER_OVER_LIMIT = "PowerOverLimit"


class _Bmp280:
    """A Bosch BMP280 pressure and temperature sensor, the BME280 without humidity."""

    __slots__ = ()

    #: The address a BMP280 answers on with its SDO pin low.
    ADDRESS_PRIMARY = 0x76
    #: The address it answers on with SDO high.
    ADDRESS_SECONDARY = 0x77
    #: The value its chip-ID register reads, which confirms the part.
    CHIP_ID = 0x58
    #: The word written to the reset register to restart the part.
    RESET_WORD = 0xB6
    #: The raw output a channel reports when its measurement is switched off.
    SKIPPED_OUTPUT = 0x80000
    #: How many bytes the calibration block holds.
    CALIBRATION_LENGTH = 24
    #: How many bytes one measurement burst holds.
    DATA_LENGTH = 6
    #: The first of the 24 calibration bytes.
    REGISTER_CALIBRATION = 0x88
    #: The chip-ID register.
    REGISTER_CHIP_ID = 0xD0
    #: The reset register.
    REGISTER_RESET = 0xE0
    #: The status register.
    REGISTER_STATUS = 0xF3
    #: The measurement-control register.
    REGISTER_CTRL_MEAS = 0xF4
    #: The configuration register.
    REGISTER_CONFIG = 0xF5
    #: The first of the six data bytes.
    REGISTER_DATA = 0xF7

    def calibration(self, data: bytes) -> Bmp280Calibration:
        """Read the factory calibration out of the registers, once at start-up.

        :param data: The 24-byte calibration block.
        :returns: The calibration, to reuse for every measurement.
        :raises ValueError: If the block is the wrong length.
        """
        return Bmp280Calibration(bytes(data))

    def parse_measurement(self, data: bytes) -> Bmp280RawMeasurement:
        """Split a six-byte data read into its two uncompensated outputs.

        :param data: The six data registers in read order.
        :returns: The raw pressure and temperature, before compensation.
        :raises ValueError: If the read is the wrong length.
        """
        return _bmp280_parse_measurement(bytes(data))

    def measurement_bytes(self, pressure: int, temperature: int) -> bytes:
        """Build the six data-register bytes a part reports for two raw outputs.

        The inverse of :meth:`parse_measurement`, so a node can be written and tested
        against what a sensor sends without one attached.

        :param pressure: The 20-bit uncompensated pressure.
        :param temperature: The 20-bit uncompensated temperature.
        :returns: The six bytes in read order.
        """
        return bytes(_bmp280_measurement_bytes(pressure, temperature))

    def pressure_skipped(self, pressure: int) -> bool:
        """Report whether a raw pressure says the measurement is switched off.

        :param pressure: The 20-bit uncompensated pressure.
        :returns: Whether the channel's oversampling is set to skip.
        """
        return _bmp280_pressure_skipped(pressure)

    def temperature_skipped(self, temperature: int) -> bool:
        """Report whether a raw temperature says the measurement is switched off.

        :param temperature: The 20-bit uncompensated temperature.
        :returns: Whether the channel's oversampling is set to skip.
        """
        return _bmp280_temperature_skipped(temperature)

    def measuring(self, status: int) -> bool:
        """Report whether a status register says a conversion is running.

        :param status: The status register.
        :returns: Whether a measurement is in progress.
        """
        return _bmp280_measuring(status)

    def image_updating(self, status: int) -> bool:
        """Report whether a status register says the calibration image is loading.

        :param status: The status register.
        :returns: Whether the coefficients are still being copied from NVM.
        """
        return _bmp280_image_updating(status)

    def ctrl_meas_bits(self, ctrl: Bmp280CtrlMeas) -> int:
        """Pack a measurement-control register value.

        :param ctrl: The oversampling settings and power mode.
        :returns: The byte to write.
        """
        return _bmp280_ctrl_meas_bits(ctrl)

    def ctrl_meas_from_bits(self, bits: int) -> Bmp280CtrlMeas:
        """Parse a measurement-control register value.

        :param bits: The byte read from the device.
        :returns: The decoded settings.
        """
        return _bmp280_ctrl_meas_from_bits(bits)

    def config_bits(self, config: Bmp280Config) -> int:
        """Pack a configuration register value.

        :param config: The standby, filter, and interface settings.
        :returns: The byte to write.
        """
        return _bmp280_config_bits(config)

    def config_from_bits(self, bits: int) -> Bmp280Config:
        """Parse a configuration register value.

        :param bits: The byte read from the device.
        :returns: The decoded settings.
        """
        return _bmp280_config_from_bits(bits)

    def oversampling_factor(self, code: int) -> int:
        """Return how many samples an oversampling code averages.

        :param code: The oversampling code, 0 to 5.
        :returns: The sample count, or 0 when the measurement is skipped.
        """
        return _bmp280_oversampling_factor(code)

    def standby_micros(self, code: int) -> int:
        """Return the normal-mode standby period a code selects.

        :param code: The standby code, 0 to 7.
        :returns: The period in microseconds.
        """
        return _bmp280_standby_micros(code)


class _Sht3x:
    """A Sensirion SHT30, SHT31, or SHT35 temperature and humidity sensor."""

    __slots__ = ()

    #: The address the part answers on with its ADDR pin low.
    ADDRESS_A = 0x44
    #: The address it answers on with ADDR high.
    ADDRESS_B = 0x45
    #: The shortest gap the datasheet allows between two commands, in microseconds.
    MIN_COMMAND_GAP_MICROS = 1_000
    #: The status word the part powers up with.
    STATUS_DEFAULT = 0x8010
    #: Single shot, high repeatability, holding the clock until the result is ready.
    COMMAND_SINGLE_SHOT_HIGH_STRETCH = 0x2C06
    #: Single shot, medium repeatability, with clock stretching.
    COMMAND_SINGLE_SHOT_MEDIUM_STRETCH = 0x2C0D
    #: Single shot, low repeatability, with clock stretching.
    COMMAND_SINGLE_SHOT_LOW_STRETCH = 0x2C10
    #: Single shot, high repeatability, polled rather than stretched.
    COMMAND_SINGLE_SHOT_HIGH = 0x2400
    #: Single shot, medium repeatability, polled.
    COMMAND_SINGLE_SHOT_MEDIUM = 0x240B
    #: Single shot, low repeatability, polled.
    COMMAND_SINGLE_SHOT_LOW = 0x2416
    #: Periodic at one measurement every two seconds, high repeatability.
    COMMAND_PERIODIC_HALF_MPS_HIGH = 0x2032
    #: Periodic at one measurement every two seconds, medium repeatability.
    COMMAND_PERIODIC_HALF_MPS_MEDIUM = 0x2024
    #: Periodic at one measurement every two seconds, low repeatability.
    COMMAND_PERIODIC_HALF_MPS_LOW = 0x202F
    #: Periodic at one measurement per second, high repeatability.
    COMMAND_PERIODIC_ONE_MPS_HIGH = 0x2130
    #: Periodic at one measurement per second, medium repeatability.
    COMMAND_PERIODIC_ONE_MPS_MEDIUM = 0x2126
    #: Periodic at one measurement per second, low repeatability.
    COMMAND_PERIODIC_ONE_MPS_LOW = 0x212D
    #: Periodic at two measurements per second, high repeatability.
    COMMAND_PERIODIC_TWO_MPS_HIGH = 0x2236
    #: Periodic at two measurements per second, medium repeatability.
    COMMAND_PERIODIC_TWO_MPS_MEDIUM = 0x2220
    #: Periodic at two measurements per second, low repeatability.
    COMMAND_PERIODIC_TWO_MPS_LOW = 0x222B
    #: Periodic at four measurements per second, high repeatability.
    COMMAND_PERIODIC_FOUR_MPS_HIGH = 0x2334
    #: Periodic at four measurements per second, medium repeatability.
    COMMAND_PERIODIC_FOUR_MPS_MEDIUM = 0x2322
    #: Periodic at four measurements per second, low repeatability.
    COMMAND_PERIODIC_FOUR_MPS_LOW = 0x2329
    #: Periodic at ten measurements per second, high repeatability.
    COMMAND_PERIODIC_TEN_MPS_HIGH = 0x2737
    #: Periodic at ten measurements per second, medium repeatability.
    COMMAND_PERIODIC_TEN_MPS_MEDIUM = 0x2721
    #: Periodic at ten measurements per second, low repeatability.
    COMMAND_PERIODIC_TEN_MPS_LOW = 0x272A
    #: Accelerated response time, four measurements per second.
    COMMAND_PERIODIC_ART = 0x2B32
    #: Fetch the latest periodic result.
    COMMAND_FETCH_DATA = 0xE000
    #: Leave periodic mode.
    COMMAND_BREAK = 0x3093
    #: Soft reset.
    COMMAND_SOFT_RESET = 0x30A2
    #: The general-call reset, addressed to 0x00.
    COMMAND_GENERAL_CALL_RESET = 0x0006
    #: Turn the on-die heater on.
    COMMAND_HEATER_ENABLE = 0x306D
    #: Turn the on-die heater off.
    COMMAND_HEATER_DISABLE = 0x3066
    #: Read the status register.
    COMMAND_READ_STATUS = 0xF32D
    #: Clear the status register.
    COMMAND_CLEAR_STATUS = 0x3041

    def crc(self, data: bytes) -> int:
        """Compute the Sensirion CRC-8 the part appends to every word.

        :param data: The bytes the checksum covers.
        :returns: The checksum.
        """
        return _sht3x_crc(bytes(data))

    def word(self, frame: bytes) -> int:
        """Read a CRC-checked three-byte word frame.

        :param frame: The two data bytes and their checksum.
        :returns: The 16-bit word.
        :raises ValueError: If the frame is not three bytes.
        :raises PamojaError: If the checksum does not match, which means the read
            was corrupted on the bus and should be repeated.
        """
        return _sht3x_word(bytes(frame))

    def word_bytes(self, value: int) -> bytes:
        """Build the three bytes the part sends for a word: the word then its CRC.

        :param value: The 16-bit word.
        :returns: The frame in transmission order.
        """
        return bytes(_sht3x_word_bytes(value))

    def parse_measurement(self, frame: bytes) -> Sht3xMeasurement:
        """Parse and CRC-check a six-byte measurement frame.

        :param frame: The temperature word, humidity word, and their checksums.
        :returns: The decoded reading, in raw words and physical units.
        :raises ValueError: If the frame is not six bytes.
        :raises PamojaError: If either checksum does not match.
        """
        return _sht3x_parse_measurement(bytes(frame))

    def measurement_bytes(self, temperature_raw: int, humidity_raw: int) -> bytes:
        """Build the six bytes the part sends for a pair of raw words.

        The inverse of :meth:`parse_measurement`, so a node can be written and tested
        against what a sensor sends without one attached.

        :param temperature_raw: The raw temperature word.
        :param humidity_raw: The raw humidity word.
        :returns: The frame in transmission order, each word followed by its CRC.
        """
        return bytes(_sht3x_measurement_bytes(temperature_raw, humidity_raw))

    def parse_status(self, frame: bytes) -> Sht3xStatus:
        """Parse and CRC-check a three-byte status frame.

        :param frame: The status word and its checksum.
        :returns: The decoded status word and its flags.
        :raises ValueError: If the frame is not three bytes.
        :raises PamojaError: If the checksum does not match.
        """
        return _sht3x_parse_status(bytes(frame))

    def status_from_bits(self, bits: int) -> Sht3xStatus:
        """Split a status word into its flags.

        :param bits: The 16-bit status register.
        :returns: The decoded flags. Every value decodes, so this never raises.
        """
        return _sht3x_status_from_bits(bits)

    def status_bytes(self, bits: int) -> bytes:
        """Build the three bytes the part sends for a status word, CRC last.

        :param bits: The 16-bit status register.
        :returns: The frame in transmission order.
        """
        return bytes(_sht3x_status_bytes(bits))

    def milli_celsius(self, raw: int) -> int:
        """Convert a raw temperature word to milli-degrees Celsius.

        :param raw: The 16-bit temperature word.
        :returns: The temperature, exact in integer arithmetic.
        """
        return _sht3x_milli_celsius(raw)

    def celsius(self, raw: int) -> float:
        """Convert a raw temperature word to degrees Celsius.

        :param raw: The 16-bit temperature word.
        :returns: The temperature.
        """
        return _sht3x_celsius(raw)

    def milli_fahrenheit(self, raw: int) -> int:
        """Convert a raw temperature word to milli-degrees Fahrenheit.

        :param raw: The 16-bit temperature word.
        :returns: The temperature, from the datasheet's own Fahrenheit formula.
        """
        return _sht3x_milli_fahrenheit(raw)

    def fahrenheit(self, raw: int) -> float:
        """Convert a raw temperature word to degrees Fahrenheit.

        :param raw: The 16-bit temperature word.
        :returns: The temperature.
        """
        return _sht3x_fahrenheit(raw)

    def milli_percent(self, raw: int) -> int:
        """Convert a raw humidity word to milli-percent.

        :param raw: The 16-bit humidity word.
        :returns: The relative humidity, exact in integer arithmetic.
        """
        return _sht3x_milli_percent(raw)

    def relative_humidity(self, raw: int) -> float:
        """Convert a raw humidity word to a relative humidity percentage.

        :param raw: The 16-bit humidity word.
        :returns: The relative humidity.
        """
        return _sht3x_relative_humidity(raw)

    def temperature_raw_from_milli_celsius(self, milli_celsius: int) -> int:
        """Build the temperature word that decodes to a temperature.

        :param milli_celsius: The temperature in milli-degrees Celsius.
        :returns: The 16-bit word, clamped to the part's range.
        """
        return _sht3x_temperature_raw_from_milli_celsius(milli_celsius)

    def temperature_raw_from_celsius(self, celsius: float) -> int:
        """Build the temperature word that decodes to a temperature in Celsius.

        :param celsius: The temperature.
        :returns: The 16-bit word, clamped to the part's range.
        """
        return _sht3x_temperature_raw_from_celsius(celsius)

    def temperature_raw_from_milli_fahrenheit(self, milli_fahrenheit: int) -> int:
        """Build the temperature word that decodes to a temperature in Fahrenheit.

        :param milli_fahrenheit: The temperature in milli-degrees Fahrenheit.
        :returns: The 16-bit word, clamped to the part's range.
        """
        return _sht3x_temperature_raw_from_milli_fahrenheit(milli_fahrenheit)

    def humidity_raw_from_milli_percent(self, milli_percent: int) -> int:
        """Build the humidity word that decodes to a relative humidity.

        :param milli_percent: The relative humidity in milli-percent.
        :returns: The 16-bit word, clamped to full scale.
        """
        return _sht3x_humidity_raw_from_milli_percent(milli_percent)

    def humidity_raw_from_relative_humidity(self, percent: float) -> int:
        """Build the humidity word that decodes to a relative humidity percentage.

        :param percent: The relative humidity.
        :returns: The 16-bit word, clamped to full scale.
        """
        return _sht3x_humidity_raw_from_relative_humidity(percent)

    def single_shot(self, repeatability: Sht3xRepeatability, clock_stretching: bool) -> int:
        """Return the single-shot command for a repeatability and clock mode.

        :param repeatability: How hard the part works at the measurement.
        :param clock_stretching: Whether the part holds the clock until the result
            is ready, rather than making the driver poll for it.
        :returns: The 16-bit command word.
        :raises ValueError: If the repeatability is not one the part offers.
        """
        return _sht3x_single_shot(Sht3xRepeatability(repeatability).value, clock_stretching)

    def periodic(self, repeatability: Sht3xRepeatability, rate: Sht3xRate) -> int:
        """Return the periodic-mode command for a repeatability and rate.

        :param repeatability: How hard the part works at each measurement.
        :param rate: How often it measures.
        :returns: The 16-bit command word.
        :raises ValueError: If either setting is not one the part offers.
        """
        return _sht3x_periodic(
            Sht3xRepeatability(repeatability).value, Sht3xRate(rate).value
        )

    def max_measurement_micros(self, repeatability: Sht3xRepeatability) -> int:
        """Return how long a measurement may take.

        :param repeatability: The setting the measurement is taken at.
        :returns: The datasheet's worst case, in microseconds.
        :raises ValueError: If the repeatability is not one the part offers.
        """
        return _sht3x_max_measurement_micros(Sht3xRepeatability(repeatability).value)

    def typical_measurement_micros(self, repeatability: Sht3xRepeatability) -> int:
        """Return how long a measurement typically takes.

        :param repeatability: The setting the measurement is taken at.
        :returns: The datasheet's typical figure, in microseconds.
        :raises ValueError: If the repeatability is not one the part offers.
        """
        return _sht3x_typical_measurement_micros(Sht3xRepeatability(repeatability).value)

    def interval_micros(self, rate: Sht3xRate) -> int:
        """Return the gap between periodic measurements.

        :param rate: The periodic rate.
        :returns: The interval in microseconds.
        :raises ValueError: If the rate is not one the part offers.
        """
        return _sht3x_interval_micros(Sht3xRate(rate).value)

class _Scd4x:
    """A Sensirion SCD40 or SCD41 carbon dioxide, temperature, and humidity sensor."""

    __slots__ = ()

    #: The only address the part answers on.
    ADDRESS = 0x62
    #: The highest concentration the part reports, in parts per million.
    CO2_MAX_PPM = 40_000
    #: The temperature offset the part powers up with, in milli-degrees Celsius.
    DEFAULT_TEMPERATURE_OFFSET_MILLI_CELSIUS = 4_000
    #: How often periodic measurement produces a result, in milliseconds.
    PERIODIC_MEASUREMENT_INTERVAL_MS = 5_000
    #: How often low-power periodic measurement produces a result, in milliseconds.
    LOW_POWER_PERIODIC_MEASUREMENT_INTERVAL_MS = 30_000
    #: How long the part takes to become ready after power-up, in milliseconds.
    POWER_UP_TIME_MS = 1_000
    #: The word a forced recalibration returns when it did not take.
    FORCED_RECALIBRATION_FAILED = 0xFFFF
    #: Start periodic measurement.
    COMMAND_START_PERIODIC_MEASUREMENT = 0x21B1
    #: Read the latest result.
    COMMAND_READ_MEASUREMENT = 0xEC05
    #: Leave periodic measurement.
    COMMAND_STOP_PERIODIC_MEASUREMENT = 0x3F86
    #: Write the temperature offset.
    COMMAND_SET_TEMPERATURE_OFFSET = 0x241D
    #: Read the temperature offset.
    COMMAND_GET_TEMPERATURE_OFFSET = 0x2318
    #: Write the installation altitude.
    COMMAND_SET_SENSOR_ALTITUDE = 0x2427
    #: Read the installation altitude.
    COMMAND_GET_SENSOR_ALTITUDE = 0x2322
    #: Write the ambient pressure, which overrides the altitude.
    COMMAND_SET_AMBIENT_PRESSURE = 0xE000
    #: Run a forced recalibration against a known concentration.
    COMMAND_PERFORM_FORCED_RECALIBRATION = 0x362F
    #: Turn automatic self-calibration on or off.
    COMMAND_SET_AUTOMATIC_SELF_CALIBRATION_ENABLED = 0x2416
    #: Read whether automatic self-calibration is on.
    COMMAND_GET_AUTOMATIC_SELF_CALIBRATION_ENABLED = 0x2313
    #: Start low-power periodic measurement.
    COMMAND_START_LOW_POWER_PERIODIC_MEASUREMENT = 0x21AC
    #: Ask whether a result is waiting.
    COMMAND_GET_DATA_READY_STATUS = 0xE4B8
    #: Copy the settings to non-volatile memory.
    COMMAND_PERSIST_SETTINGS = 0x3615
    #: Read the serial number.
    COMMAND_GET_SERIAL_NUMBER = 0x3682
    #: Run the self test.
    COMMAND_PERFORM_SELF_TEST = 0x3639
    #: Restore the factory settings.
    COMMAND_PERFORM_FACTORY_RESET = 0x3632
    #: Reinitialise from the stored settings.
    COMMAND_REINIT = 0x3646
    #: Take one measurement and return to idle.
    COMMAND_MEASURE_SINGLE_SHOT = 0x219D
    #: Take one humidity and temperature measurement, without carbon dioxide.
    COMMAND_MEASURE_SINGLE_SHOT_RHT_ONLY = 0x2196
    #: Enter sleep, SCD41 only.
    COMMAND_POWER_DOWN = 0x36E0
    #: Leave sleep, SCD41 only.
    COMMAND_WAKE_UP = 0x36F6

    def crc(self, data: bytes) -> int:
        """Compute the Sensirion CRC-8 the part appends to every word.

        :param data: The bytes the checksum covers.
        :returns: The checksum.
        """
        return _scd4x_crc(bytes(data))

    def word(self, frame: bytes) -> int:
        """Read a CRC-checked three-byte word frame.

        :param frame: The two data bytes and their checksum.
        :returns: The 16-bit word.
        :raises ValueError: If the frame is not three bytes.
        :raises PamojaError: If the checksum does not match, which means the read
            was corrupted on the bus and should be repeated.
        """
        return _scd4x_word(bytes(frame))

    def word_frame(self, value: int) -> bytes:
        """Build the three bytes the part sends for a word: the word then its CRC.

        :param value: The 16-bit word.
        :returns: The frame in transmission order.
        """
        return bytes(_scd4x_word_frame(value))

    def command_frame(self, command: int) -> bytes:
        """Build the two bytes that send a bare command.

        :param command: The 16-bit command word.
        :returns: The bytes to write, most significant first.
        """
        return bytes(_scd4x_command_frame(command))

    def write_frame(self, command: int, value: int) -> bytes:
        """Build the five bytes that send a command with an argument.

        :param command: The 16-bit command word.
        :param value: The argument the command takes.
        :returns: The command, the argument, and the argument's CRC.
        """
        return bytes(_scd4x_write_frame(command, value))

    def max_duration_ms(self, command: int) -> int | None:
        """Return how long a command may take.

        :param command: The 16-bit command word.
        :returns: The datasheet's execution time in milliseconds, or ``None`` for a
            command that completes without one.
        """
        return _scd4x_max_duration_ms(command)

    def allowed_during_measurement(self, command: int) -> bool:
        """Report whether the part accepts a command while it is measuring.

        :param command: The 16-bit command word.
        :returns: Whether the command can be sent without stopping measurement first.
        """
        return _scd4x_allowed_during_measurement(command)

    def parse_measurement(self, frame: bytes) -> Scd4xMeasurement:
        """Parse and CRC-check a nine-byte measurement frame.

        :param frame: The three words and their checksums, as the device sent them.
        :returns: The decoded reading, in raw words and physical units.
        :raises ValueError: If the frame is not nine bytes.
        :raises PamojaError: If any checksum does not match.
        """
        return _scd4x_parse_measurement(bytes(frame))

    def measurement_from_physical(
        self, co2_ppm: int, milli_celsius: int, humidity_milli_percent: int
    ) -> Scd4xMeasurement:
        """Build the measurement a sensor reporting these values would send.

        :param co2_ppm: The carbon dioxide concentration in parts per million.
        :param milli_celsius: The temperature in milli-degrees Celsius.
        :param humidity_milli_percent: The relative humidity in milli-percent.
        :returns: The measurement, with the raw words the part would have reported.
        """
        return _scd4x_measurement_from_physical(
            co2_ppm, milli_celsius, humidity_milli_percent
        )

    def measurement_bytes(
        self, co2_ppm: int, temperature_raw: int, humidity_raw: int
    ) -> bytes:
        """Build the nine bytes the part sends for a set of raw words.

        The inverse of :meth:`parse_measurement`, so a node can be written and tested
        against what a sensor sends without one attached.

        :param co2_ppm: The carbon dioxide concentration in parts per million.
        :param temperature_raw: The raw temperature word.
        :param humidity_raw: The raw humidity word.
        :returns: The frame in transmission order, each word followed by its CRC.
        """
        return bytes(_scd4x_measurement_bytes(co2_ppm, temperature_raw, humidity_raw))

    def milli_celsius(self, raw: int) -> int:
        """Convert a raw temperature word to milli-degrees Celsius.

        :param raw: The 16-bit temperature word.
        :returns: The temperature, exact in integer arithmetic.
        """
        return _scd4x_milli_celsius(raw)

    def celsius(self, raw: int) -> float:
        """Convert a raw temperature word to degrees Celsius.

        :param raw: The 16-bit temperature word.
        :returns: The temperature.
        """
        return _scd4x_celsius(raw)

    def temperature_raw(self, milli_celsius: int) -> int:
        """Build the temperature word that decodes to a temperature.

        :param milli_celsius: The temperature in milli-degrees Celsius.
        :returns: The 16-bit word, clamped to the part's range.
        """
        return _scd4x_temperature_raw(milli_celsius)

    def humidity_milli_percent(self, raw: int) -> int:
        """Convert a raw humidity word to milli-percent.

        :param raw: The 16-bit humidity word.
        :returns: The relative humidity, exact in integer arithmetic.
        """
        return _scd4x_humidity_milli_percent(raw)

    def relative_humidity_percent(self, raw: int) -> float:
        """Convert a raw humidity word to a relative humidity percentage.

        :param raw: The 16-bit humidity word.
        :returns: The relative humidity.
        """
        return _scd4x_relative_humidity_percent(raw)

    def humidity_raw(self, milli_percent: int) -> int:
        """Build the humidity word that decodes to a relative humidity.

        :param milli_percent: The relative humidity in milli-percent.
        :returns: The 16-bit word, clamped to full scale.
        """
        return _scd4x_humidity_raw(milli_percent)

    def data_ready(self, word: int) -> bool:
        """Report whether a data-ready word says a measurement is waiting.

        :param word: The word the data-ready command returned.
        :returns: Whether a result can be read.
        """
        return _scd4x_data_ready(word)

    def temperature_offset_word(self, milli_celsius: int) -> int:
        """Build the word that programs a temperature offset.

        The offset scales by 2^16, not by the 2^16 - 1 a measurement uses.

        :param milli_celsius: The offset in milli-degrees Celsius.
        :returns: The word to write.
        """
        return _scd4x_temperature_offset_word(milli_celsius)

    def temperature_offset_milli_celsius(self, word: int) -> int:
        """Convert a temperature-offset word back to milli-degrees Celsius.

        :param word: The word the part returned.
        :returns: The offset in milli-degrees Celsius.
        """
        return _scd4x_temperature_offset_milli_celsius(word)

    def ambient_pressure_word(self, pascals: int) -> int:
        """Build the word that programs an ambient pressure.

        :param pascals: The ambient pressure.
        :returns: The word to write, in hundreds of pascals.
        """
        return _scd4x_ambient_pressure_word(pascals)

    def ambient_pressure_pascals(self, word: int) -> int:
        """Convert an ambient-pressure word back to pascals.

        :param word: The word the part holds.
        :returns: The ambient pressure.
        """
        return _scd4x_ambient_pressure_pascals(word)

    def forced_recalibration_correction_ppm(self, word: int) -> int | None:
        """Read the correction the part reports after a forced recalibration.

        :param word: The word the recalibration command returned.
        :returns: The correction in parts per million, or ``None`` if the
            recalibration did not take.
        """
        return _scd4x_forced_recalibration_correction_ppm(word)

    def forced_recalibration_word(self, correction_ppm: int | None = None) -> int:
        """Build the word the part returns for a forced-recalibration outcome.

        :param correction_ppm: The correction in parts per million, or ``None`` for
            a recalibration that did not take.
        :returns: The word.
        """
        return _scd4x_forced_recalibration_word(correction_ppm)

    def automatic_self_calibration_enabled(self, word: int) -> bool:
        """Report whether a word says automatic self-calibration is on.

        :param word: The word the part returned.
        :returns: Whether the part recalibrates itself against clean air.
        """
        return _scd4x_automatic_self_calibration_enabled(word)

    def automatic_self_calibration_word(self, enabled: bool) -> int:
        """Build the word that turns automatic self-calibration on or off.

        :param enabled: Whether to leave self-calibration running.
        :returns: The word to write.
        """
        return _scd4x_automatic_self_calibration_word(enabled)

    def self_test_passed(self, word: int) -> bool:
        """Report whether a self-test word says the part passed.

        :param word: The word the self-test command returned.
        :returns: Whether the part reported no malfunction.
        """
        return _scd4x_self_test_passed(word)

    def serial_number(self, frame: bytes) -> int:
        """Read a CRC-checked nine-byte serial-number frame.

        :param frame: The three words and their checksums.
        :returns: The 48-bit serial number.
        :raises ValueError: If the frame is not nine bytes.
        :raises PamojaError: If any checksum does not match.
        """
        return _scd4x_serial_number(bytes(frame))

    def serial_number_frame(self, serial: int) -> bytes:
        """Build the nine bytes the part sends for a serial number.

        :param serial: The 48-bit serial number.
        :returns: The frame in transmission order, each word followed by its CRC.
        """
        return bytes(_scd4x_serial_number_frame(serial))


class _Tmp117:
    """A TI TMP117 precision thermometer."""

    __slots__ = ()

    #: The value the device-ID register reads, which confirms the part.
    DEVICE_ID = 0x0117
    #: The value the configuration register reads after a reset.
    CONFIG_RESET = 0x0220
    #: The value the high-limit register holds after a reset.
    HIGH_LIMIT_RESET = 0x6000
    #: The value the low-limit register holds after a reset.
    LOW_LIMIT_RESET = 0x8000
    #: The value the result register holds before the first conversion.
    TEMP_RESULT_RESET = 0x8000
    #: The byte a general-call reset sends to address 0x00.
    GENERAL_CALL_RESET = 0x06
    #: The word written to the EEPROM-unlock register to allow a write.
    EEPROM_UNLOCK = 0x8000
    #: The address with ADD0 tied to GND.
    ADDRESS_ADD0_GND = 0x48
    #: The address with ADD0 tied to V+.
    ADDRESS_ADD0_VPLUS = 0x49
    #: The address with ADD0 tied to SDA.
    ADDRESS_ADD0_SDA = 0x4A
    #: The address with ADD0 tied to SCL.
    ADDRESS_ADD0_SCL = 0x4B
    #: The temperature result.
    REGISTER_TEMP_RESULT = 0x00
    #: The configuration register.
    REGISTER_CONFIGURATION = 0x01
    #: The high alert limit.
    REGISTER_THIGH_LIMIT = 0x02
    #: The low alert limit.
    REGISTER_TLOW_LIMIT = 0x03
    #: The EEPROM unlock register.
    REGISTER_EEPROM_UL = 0x04
    #: The first general-purpose EEPROM word.
    REGISTER_EEPROM1 = 0x05
    #: The second general-purpose EEPROM word.
    REGISTER_EEPROM2 = 0x06
    #: The temperature offset applied to every result.
    REGISTER_TEMP_OFFSET = 0x07
    #: The third general-purpose EEPROM word.
    REGISTER_EEPROM3 = 0x08
    #: The device-ID register.
    REGISTER_DEVICE_ID = 0x0F

    def nano_celsius(self, raw: int) -> int:
        """Convert a raw temperature register to nano-degrees Celsius.

        :param raw: The signed 16-bit register.
        :returns: The temperature, exact at the part's 7.8125 m°C resolution.
        """
        return _tmp117_nano_celsius(raw)

    def micro_celsius(self, raw: int) -> int:
        """Convert a raw temperature register to micro-degrees Celsius.

        :param raw: The signed 16-bit register.
        :returns: The temperature, truncated toward zero.
        """
        return _tmp117_micro_celsius(raw)

    def celsius(self, raw: int) -> float:
        """Convert a raw temperature register to degrees Celsius.

        :param raw: The signed 16-bit register.
        :returns: The temperature.
        """
        return _tmp117_celsius(raw)

    def raw_from_micro_celsius(self, micro_celsius: int) -> int:
        """Build the temperature register that decodes to a temperature.

        :param micro_celsius: The temperature in micro-degrees Celsius.
        :returns: The signed register, rounded to nearest and saturating at the
            part's range.
        """
        return _tmp117_raw_from_micro_celsius(micro_celsius)

    def raw_from_celsius(self, celsius: float) -> int:
        """Build the temperature register that decodes to a temperature in Celsius.

        :param celsius: The temperature.
        :returns: The signed register, saturating at the part's range.
        """
        return _tmp117_raw_from_celsius(celsius)

    def temperature_bytes(self, raw: int) -> bytes:
        """Build the two bytes the part sends for a temperature register.

        :param raw: The signed 16-bit register.
        :returns: The bytes, most significant first.
        """
        return bytes(_tmp117_temperature_bytes(raw))

    def temperature_from_bytes(self, data: bytes) -> int:
        """Read the two bytes the part sends for a temperature register.

        :param data: The two register bytes, most significant first.
        :returns: The signed register value.
        :raises ValueError: If the read is not two bytes.
        """
        return _tmp117_temperature_from_bytes(bytes(data))

    def device_id(self, raw: int) -> int:
        """Read the device identifier out of a device-ID register.

        :param raw: The device-ID register.
        :returns: The 12-bit identifier, :attr:`DEVICE_ID` for a TMP117.
        """
        return _tmp117_device_id(raw)

    def revision(self, raw: int) -> int:
        """Read the die revision out of a device-ID register.

        :param raw: The device-ID register.
        :returns: The 4-bit revision.
        """
        return _tmp117_revision(raw)

    def high_alert(self, config: int) -> bool:
        """Report whether a configuration register flags a high alert.

        :param config: The configuration register.
        :returns: Whether a result went above the high limit.
        """
        return _tmp117_high_alert(config)

    def low_alert(self, config: int) -> bool:
        """Report whether a configuration register flags a low alert.

        :param config: The configuration register.
        :returns: Whether a result went below the low limit.
        """
        return _tmp117_low_alert(config)

    def data_ready(self, config: int) -> bool:
        """Report whether a configuration register says a result is ready.

        :param config: The configuration register.
        :returns: Whether a conversion completed since the register was last read.
        """
        return _tmp117_data_ready(config)

    def eeprom_busy(self, config: int) -> bool:
        """Report whether a configuration register says an EEPROM write is running.

        :param config: The configuration register.
        :returns: Whether a programming cycle is still in progress.
        """
        return _tmp117_eeprom_busy(config)

    def eeprom_unlock_busy(self, unlock: int) -> bool:
        """Report whether an EEPROM unlock register says a write is running.

        :param unlock: The EEPROM unlock register.
        :returns: Whether a programming cycle is still in progress.
        """
        return _tmp117_eeprom_unlock_busy(unlock)

    def config_bits(self, config: Tmp117Config) -> int:
        """Assemble the 16-bit configuration register value.

        :param config: The settings to encode.
        :returns: The register value to write.
        """
        return _tmp117_config_bits(config)

    def config_from_bits(self, bits: int) -> Tmp117Config:
        """Parse a 16-bit configuration register value.

        :param bits: The register value, as read from the device.
        :returns: The decoded settings. Every value decodes, so this never raises.
        """
        return _tmp117_config_from_bits(bits)

    def conversions(self, averaging: int) -> int:
        """Return how many conversions an averaging code folds into one result.

        :param averaging: The averaging code, 0 to 3.
        :returns: The conversion count: 1, 8, 32, or 64.
        """
        return _tmp117_averaging_conversions(averaging)

    def conversion_micros(self, averaging: int) -> int:
        """Return how long an averaging code takes to convert.

        :param averaging: The averaging code, 0 to 3.
        :returns: The conversion time in microseconds.
        """
        return _tmp117_averaging_micros(averaging)

    def nominal_micros(self, cycle: int) -> int:
        """Return the nominal cycle a conversion-cycle code selects.

        :param cycle: The conversion-cycle code, 0 to 7.
        :returns: The nominal cycle in microseconds.
        """
        return _tmp117_cycle_nominal_micros(cycle)

    def cycle_micros(self, cycle: int, averaging: int) -> int:
        """Return the result-update interval for a cycle and averaging code.

        A cycle shorter than the conversion it asks for stretches to the conversion.

        :param cycle: The conversion-cycle code, 0 to 7.
        :param averaging: The averaging code, 0 to 3.
        :returns: The interval in microseconds.
        """
        return _tmp117_cycle_micros(cycle, averaging)

class _Hdc1080:
    """A TI HDC1080 temperature and humidity sensor."""

    __slots__ = ()

    #: The only address the part answers on.
    ADDRESS = 0x40
    #: The value the manufacturer-ID register reads.
    MANUFACTURER_ID = 0x5449
    #: The value the device-ID register reads, which confirms the part.
    DEVICE_ID = 0x1050
    #: The value the configuration register reads after a reset.
    CONFIGURATION_RESET = 0x1000
    #: The temperature result.
    REGISTER_TEMPERATURE = 0x00
    #: The humidity result.
    REGISTER_HUMIDITY = 0x01
    #: The configuration register.
    REGISTER_CONFIGURATION = 0x02
    #: The top word of the serial number.
    REGISTER_SERIAL_ID_HIGH = 0xFB
    #: The middle word of the serial number.
    REGISTER_SERIAL_ID_MID = 0xFC
    #: The bottom word of the serial number.
    REGISTER_SERIAL_ID_LOW = 0xFD
    #: The manufacturer-ID register.
    REGISTER_MANUFACTURER_ID = 0xFE
    #: The device-ID register.
    REGISTER_DEVICE_ID = 0xFF

    def milli_celsius(self, raw: int) -> int:
        """Convert a raw temperature register to milli-degrees Celsius.

        :param raw: The 16-bit register.
        :returns: The temperature, exact in integer arithmetic.
        """
        return _hdc1080_milli_celsius(raw)

    def celsius(self, raw: int) -> float:
        """Convert a raw temperature register to degrees Celsius.

        :param raw: The 16-bit register.
        :returns: The temperature.
        """
        return _hdc1080_celsius(raw)

    def milli_percent(self, raw: int) -> int:
        """Convert a raw humidity register to milli-percent.

        :param raw: The 16-bit register.
        :returns: The relative humidity, exact in integer arithmetic.
        """
        return _hdc1080_milli_percent(raw)

    def relative_humidity(self, raw: int) -> float:
        """Convert a raw humidity register to a relative humidity percentage.

        :param raw: The 16-bit register.
        :returns: The relative humidity.
        """
        return _hdc1080_relative_humidity(raw)

    def temperature_register(self, milli_celsius: int) -> int:
        """Build the temperature register that decodes to a temperature.

        :param milli_celsius: The temperature in milli-degrees Celsius.
        :returns: The 14-bit code in bits 15:2, clamped to the part's range.
        """
        return _hdc1080_temperature_register(milli_celsius)

    def humidity_register(self, milli_percent: int) -> int:
        """Build the humidity register that decodes to a relative humidity.

        :param milli_percent: The relative humidity in milli-percent.
        :returns: The 14-bit code in bits 15:2, clamped to full scale.
        """
        return _hdc1080_humidity_register(milli_percent)

    def serial_id(self, high: int, mid: int, low: int) -> int:
        """Join the three serial-ID registers into the serial number.

        :param high: The top serial-ID register.
        :param mid: The middle serial-ID register.
        :param low: The bottom serial-ID register.
        :returns: The 40-bit serial number.
        """
        return _hdc1080_serial_id(high, mid, low)

    def serial_id_registers(self, serial: int) -> tuple[int, int, int]:
        """Split a serial number back into the three serial-ID registers.

        :param serial: The 40-bit serial number.
        :returns: The high, middle, and low registers.
        """
        high, mid, low = _hdc1080_serial_id_registers(serial)
        return high, mid, low

    def parse_measurement(self, data: bytes) -> Hdc1080Measurement:
        """Parse the four bytes a sequential read returns.

        The part appends no checksum, so every read decodes.

        :param data: The temperature and humidity registers, most significant first.
        :returns: The decoded reading, in raw registers and physical units.
        :raises ValueError: If the read is not four bytes.
        """
        return _hdc1080_parse_measurement(bytes(data))

    def measurement_from_physical(
        self, milli_celsius: int, milli_percent: int
    ) -> Hdc1080Measurement:
        """Build the measurement a sensor reporting these values would send.

        :param milli_celsius: The temperature in milli-degrees Celsius.
        :param milli_percent: The relative humidity in milli-percent.
        :returns: The measurement, with the raw registers the part would have held.
        """
        return _hdc1080_measurement_from_physical(milli_celsius, milli_percent)

    def measurement_bytes(self, temperature_raw: int, humidity_raw: int) -> bytes:
        """Build the four bytes the part sends for a pair of raw registers.

        The inverse of :meth:`parse_measurement`, so a node can be written and tested
        against what a sensor sends without one attached.

        :param temperature_raw: The raw temperature register.
        :param humidity_raw: The raw humidity register.
        :returns: The four bytes in read order.
        """
        return bytes(_hdc1080_measurement_bytes(temperature_raw, humidity_raw))

    def config_from_register(self, raw: int) -> Hdc1080Config:
        """Parse a configuration register value.

        :param raw: The register value, as read from the device.
        :returns: The decoded settings.
        :raises PamojaError: If the humidity-resolution field holds the code the
            datasheet leaves undefined, which no working part reports.
        """
        return _hdc1080_config_from_register(raw)

    def config_to_register(self, config: Hdc1080Config) -> int:
        """Assemble a configuration register value.

        :param config: The settings to encode.
        :returns: The register value to write.
        :raises ValueError: If either resolution is not one the part offers.
        """
        return _hdc1080_config_to_register(config)

    def conversion_time_micros(self, config: Hdc1080Config) -> int:
        """Return how long to wait after triggering the part in this configuration.

        :param config: The settings the part is running.
        :returns: The conversion time in microseconds, both channels in sequential
            mode.
        :raises ValueError: If either resolution is not one the part offers.
        """
        return _hdc1080_conversion_time_micros(config)

    def temperature_conversion_micros(self, bits: int) -> int:
        """Return how long a temperature conversion takes.

        :param bits: The temperature resolution in bits: 14 or 11.
        :returns: The conversion time in microseconds.
        :raises ValueError: If the resolution is not one the part offers.
        """
        return _hdc1080_temperature_conversion_micros(bits)

    def humidity_conversion_micros(self, bits: int) -> int:
        """Return how long a humidity conversion takes.

        :param bits: The humidity resolution in bits: 14, 11, or 8.
        :returns: The conversion time in microseconds.
        :raises ValueError: If the resolution is not one the part offers.
        """
        return _hdc1080_humidity_conversion_micros(bits)


class _Opt3001:
    """A TI OPT3001 ambient light sensor."""

    __slots__ = ()

    #: The address the part answers on with ADDR tied to GND.
    ADDRESS_GND = 0x44
    #: The address it answers on with ADDR tied to VDD.
    ADDRESS_VDD = 0x45
    #: The address it answers on with ADDR tied to SDA.
    ADDRESS_SDA = 0x46
    #: The address it answers on with ADDR tied to SCL.
    ADDRESS_SCL = 0x47
    #: The value the manufacturer-ID register reads.
    MANUFACTURER_ID = 0x5449
    #: The value the device-ID register reads, which confirms the part.
    DEVICE_ID = 0x3001
    #: The value the configuration register reads after a reset.
    CONFIGURATION_RESET = 0xC810
    #: The value the low-limit register holds after a reset.
    LOW_LIMIT_RESET = 0x0000
    #: The value the high-limit register holds after a reset.
    HIGH_LIMIT_RESET = 0xBFFF
    #: The low-limit value that turns the INT pin into an end-of-conversion signal.
    LOW_LIMIT_END_OF_CONVERSION = 0xC000
    #: The range number that lets the part choose its own full scale.
    RANGE_AUTOMATIC = 0b1100
    #: The highest fixed range number.
    RANGE_MAX = 11
    #: The result register.
    REGISTER_RESULT = 0x00
    #: The configuration register.
    REGISTER_CONFIGURATION = 0x01
    #: The low limit.
    REGISTER_LOW_LIMIT = 0x02
    #: The high limit.
    REGISTER_HIGH_LIMIT = 0x03
    #: The manufacturer-ID register.
    REGISTER_MANUFACTURER_ID = 0x7E
    #: The device-ID register.
    REGISTER_DEVICE_ID = 0x7F

    def lsb_milli_lux(self, exponent: int) -> int | None:
        """Return the illuminance one count carries at an exponent.

        :param exponent: The result's exponent field, 0 to 11.
        :returns: The step in milli-lux, or ``None`` for a reserved exponent.
        """
        return _opt3001_lsb_milli_lux(exponent)

    def full_scale_milli_lux(self, range_number: int) -> int | None:
        """Return the full scale a range number covers.

        :param range_number: The range number, 0 to 11.
        :returns: The full scale in milli-lux, or ``None`` for a reserved range,
            which includes the automatic-range code.
        """
        return _opt3001_full_scale_milli_lux(range_number)

    def milli_lux(self, raw: int) -> int:
        """Convert a raw result register to milli-lux.

        :param raw: The register value, an exponent and a 12-bit mantissa.
        :returns: The illuminance, exact in integer arithmetic.
        """
        return _opt3001_milli_lux(raw)

    def lux(self, raw: int) -> float:
        """Convert a raw result register to lux.

        :param raw: The register value.
        :returns: The illuminance.
        """
        return _opt3001_lux(raw)

    def raw_from_milli_lux(self, milli_lux: int) -> int:
        """Build the result register that decodes to an illuminance.

        The smallest exponent that holds the value is used, so the mantissa keeps
        as much resolution as the format allows.

        :param milli_lux: The illuminance in milli-lux.
        :returns: The register value, saturating at full scale.
        """
        return _opt3001_raw_from_milli_lux(milli_lux)

    def word_from_bytes(self, data: bytes) -> int:
        """Read the two bytes the part sends for a register.

        :param data: The register bytes, most significant first.
        :returns: The register value.
        :raises ValueError: If the read is not two bytes.
        """
        return _opt3001_word_from_bytes(bytes(data))

    def word_to_bytes(self, word: int) -> bytes:
        """Build the two bytes the part sends for a register.

        :param word: The register value.
        :returns: The bytes, most significant first.
        """
        return bytes(_opt3001_word_to_bytes(word))

    def config_bits(self, config: Opt3001Config) -> int:
        """Assemble the 16-bit configuration register value.

        The read-only status bits are written as zero.

        :param config: The settings to encode.
        :returns: The register value to write.
        """
        return _opt3001_config_bits(config)

    def config_from_bits(self, bits: int) -> Opt3001Config:
        """Parse a 16-bit configuration register value.

        :param bits: The register value, as read from the device.
        :returns: The decoded settings. Every value decodes, so this never raises.
        """
        return _opt3001_config_from_bits(bits)

    def conversion_millis(self, long_conversion: bool) -> int:
        """Return the conversion time a setting selects.

        :param long_conversion: Whether the 800 ms conversion is selected rather
            than the 100 ms one.
        :returns: The conversion time in milliseconds.
        """
        return _opt3001_conversion_millis(long_conversion)

    def fault_count(self, code: int) -> int:
        """Return how many consecutive faults a fault-count code requires.

        :param code: The fault-count code, 0 to 3.
        :returns: The fault count: 1, 2, 4, or 8.
        """
        return _opt3001_fault_count(code)

    def is_automatic_range(self, range_number: int) -> bool:
        """Report whether a range number sets the full scale automatically.

        :param range_number: The range number from a configuration register.
        :returns: Whether the part chooses its own full scale.
        """
        return _opt3001_is_automatic_range(range_number)


class _Ina226:
    """A TI INA226 current, voltage, and power monitor."""

    __slots__ = ()

    #: The address with both A1 and A0 tied to GND.
    BASE_ADDRESS = 0x40
    #: The value the manufacturer-ID register reads.
    MANUFACTURER_ID = 0x5449
    #: The device identifier the die-ID register carries for an INA226.
    DEVICE_ID = 0x226
    #: The value the configuration register reads after a reset.
    CONFIG_RESET = 0x4127
    #: The shunt voltage one count carries, in nanovolts.
    SHUNT_LSB_NANOVOLTS = 2_500
    #: The bus voltage one count carries, in microvolts.
    BUS_LSB_MICROVOLTS = 1_250
    #: How many times the power LSB is the current LSB.
    POWER_LSB_RATIO = 25
    #: An address pin tied to GND.
    PIN_GROUND = 0
    #: An address pin tied to VS.
    PIN_SUPPLY = 1
    #: An address pin tied to SDA.
    PIN_SDA = 2
    #: An address pin tied to SCL.
    PIN_SCL = 3
    #: The configuration register.
    REGISTER_CONFIGURATION = 0x00
    #: The shunt voltage result.
    REGISTER_SHUNT_VOLTAGE = 0x01
    #: The bus voltage result.
    REGISTER_BUS_VOLTAGE = 0x02
    #: The power result.
    REGISTER_POWER = 0x03
    #: The current result.
    REGISTER_CURRENT = 0x04
    #: The calibration register.
    REGISTER_CALIBRATION = 0x05
    #: The Mask/Enable register.
    REGISTER_MASK_ENABLE = 0x06
    #: The alert limit.
    REGISTER_ALERT_LIMIT = 0x07
    #: The manufacturer-ID register.
    REGISTER_MANUFACTURER_ID = 0xFE
    #: The die-ID register.
    REGISTER_DIE_ID = 0xFF

    def address(self, a1: int, a0: int) -> int:
        """Return the 7-bit address the A1 and A0 pins select.

        :param a1: What the A1 pin is tied to, one of the ``PIN_`` codes.
        :param a0: What the A0 pin is tied to.
        :returns: The address, 0x40 to 0x4F.
        :raises ValueError: If either pin code is not 0, 1, 2, or 3.
        """
        return _ina226_address(a1, a0)

    def averaging_samples(self, averaging: int) -> int:
        """Return how many samples an averaging code folds into one result.

        :param averaging: The averaging code, 0 to 7.
        :returns: The sample count, from 1 to 1024.
        """
        return _ina226_averaging_samples(averaging)

    def conversion_micros(self, conversion_time: int) -> int:
        """Return the conversion time a code selects.

        :param conversion_time: The conversion-time code, 0 to 7.
        :returns: The typical conversion time in microseconds.
        """
        return _ina226_conversion_micros(conversion_time)

    def measures_shunt(self, mode: int) -> bool:
        """Report whether a mode code converts the shunt voltage.

        :param mode: The operating-mode code, 0 to 7.
        :returns: Whether the shunt input is converted.
        """
        return _ina226_measures_shunt(mode)

    def measures_bus(self, mode: int) -> bool:
        """Report whether a mode code converts the bus voltage.

        :param mode: The operating-mode code, 0 to 7.
        :returns: Whether the bus input is converted.
        """
        return _ina226_measures_bus(mode)

    def is_continuous(self, mode: int) -> bool:
        """Report whether a mode code keeps converting after the first result.

        :param mode: The operating-mode code, 0 to 7.
        :returns: Whether conversions run back to back.
        """
        return _ina226_is_continuous(mode)

    def config_from_register(self, raw: int) -> Ina226Config:
        """Parse a configuration register value.

        :param raw: The register value, as read from the device.
        :returns: The decoded settings. Every value decodes, so this never raises.
        """
        return _ina226_config_from_register(raw)

    def config_to_register(self, config: Ina226Config) -> int:
        """Assemble a configuration register value.

        :param config: The settings to encode.
        :returns: The register value to write, with reserved bit 14 set as at reset.
        """
        return _ina226_config_to_register(config)

    def update_micros(self, config: Ina226Config) -> int:
        """Return how often the part in this configuration updates its results.

        :param config: The settings the part is running.
        :returns: The update interval in microseconds, or 0 in power-down.
        """
        return _ina226_update_micros(config)

    def mask_enable_from_register(self, raw: int) -> Ina226MaskEnable:
        """Parse a Mask/Enable register value.

        :param raw: The register value, as read from the device.
        :returns: The decoded enables and flags.
        """
        return _ina226_mask_enable_from_register(raw)

    def mask_enable_to_register(self, mask: Ina226MaskEnable) -> int:
        """Assemble a Mask/Enable register value.

        :param mask: The enables and flags to encode.
        :returns: The register value to write.
        """
        return _ina226_mask_enable_to_register(mask)

    def active_alert_function(
        self, mask: Ina226MaskEnable
    ) -> Ina226AlertFunction | None:
        """Return the alert function the pin actually responds to.

        Only one limit function drives the pin at a time; when several are enabled
        the part honors the most significant bit.

        :param mask: The enables and flags the part is running.
        :returns: The function the pin follows, or ``None`` if no limit function is
            enabled.
        """
        function = _ina226_active_alert_function(mask)
        return None if function is None else Ina226AlertFunction(function)

    def die_id(self, raw: int) -> Ina226DieId:
        """Split a die-ID register into its device and revision fields.

        :param raw: The die-ID register.
        :returns: The device identifier and die revision.
        """
        return _ina226_die_id(raw)

    def identify(self, manufacturer_id: int, die_id: int) -> Ina226DieId:
        """Check that a pair of identification registers belongs to an INA226.

        :param manufacturer_id: The manufacturer-ID register.
        :param die_id: The die-ID register.
        :returns: The device identifier and die revision.
        :raises PamojaError: If either register does not carry the value the
            datasheet fixes, which means a different part answered at that address
            and its readings must not be trusted.
        """
        return _ina226_identify(manufacturer_id, die_id)

    def calibration(self, current_lsb_microamps: int, shunt_milliohms: int) -> int:
        """Compute the calibration register for a shunt and current resolution.

        :param current_lsb_microamps: The microamps per count the current register
            should carry.
        :param shunt_milliohms: The shunt resistor value.
        :returns: The register value to write.
        """
        return _ina226_calibration(current_lsb_microamps, shunt_milliohms)

    def minimum_current_lsb_microamps(self, max_expected_microamps: int) -> int:
        """Return the smallest current resolution that still covers a maximum.

        :param max_expected_microamps: The largest current the application measures.
        :returns: The minimum current LSB in microamps.
        """
        return _ina226_minimum_current_lsb_microamps(max_expected_microamps)

    def shunt_nanovolts(self, raw: int) -> int:
        """Convert a raw shunt-voltage register to nanovolts.

        :param raw: The signed register value.
        :returns: The shunt voltage, at 2.5 uV per count.
        """
        return _ina226_shunt_nanovolts(raw)

    def shunt_millivolts(self, raw: int) -> float:
        """Convert a raw shunt-voltage register to millivolts.

        :param raw: The signed register value.
        :returns: The shunt voltage.
        """
        return _ina226_shunt_millivolts(raw)

    def bus_microvolts(self, raw: int) -> int:
        """Convert a raw bus-voltage register to microvolts.

        :param raw: The register value.
        :returns: The bus voltage, at 1.25 mV per count.
        """
        return _ina226_bus_microvolts(raw)

    def bus_volts(self, raw: int) -> float:
        """Convert a raw bus-voltage register to volts.

        :param raw: The register value.
        :returns: The bus voltage.
        """
        return _ina226_bus_volts(raw)

    def current_microamps(self, raw: int, current_lsb_microamps: int) -> int:
        """Convert a raw current register to microamps.

        :param raw: The signed register value.
        :param current_lsb_microamps: The resolution the calibration selected.
        :returns: The current.
        """
        return _ina226_current_microamps(raw, current_lsb_microamps)

    def current_amps(self, raw: int, current_lsb_microamps: int) -> float:
        """Convert a raw current register to amps.

        :param raw: The signed register value.
        :param current_lsb_microamps: The resolution the calibration selected.
        :returns: The current.
        """
        return _ina226_current_amps(raw, current_lsb_microamps)

    def power_microwatts(self, raw: int, current_lsb_microamps: int) -> int:
        """Convert a raw power register to microwatts.

        :param raw: The register value.
        :param current_lsb_microamps: The resolution the calibration selected.
        :returns: The power. The power LSB is fixed at 25 times the current LSB.
        """
        return _ina226_power_microwatts(raw, current_lsb_microamps)

    def power_watts(self, raw: int, current_lsb_microamps: int) -> float:
        """Convert a raw power register to watts.

        :param raw: The register value.
        :param current_lsb_microamps: The resolution the calibration selected.
        :returns: The power.
        """
        return _ina226_power_watts(raw, current_lsb_microamps)

    def shunt_register(self, nanovolts: int) -> int:
        """Build the shunt-voltage register a monitor reports for a shunt voltage.

        The inverse of :meth:`shunt_nanovolts`, so a node can be written and tested
        against what a monitor sends without one attached.

        :param nanovolts: The shunt voltage in nanovolts.
        :returns: The signed register value.
        """
        return _ina226_shunt_register(nanovolts)

    def bus_register(self, microvolts: int) -> int:
        """Build the bus-voltage register a monitor reports for a bus voltage.

        :param microvolts: The bus voltage in microvolts.
        :returns: The register value.
        """
        return _ina226_bus_register(microvolts)

    def current_register(self, microamps: int, current_lsb_microamps: int) -> int:
        """Build the current register a monitor reports for a current.

        :param microamps: The current in microamps.
        :param current_lsb_microamps: The current LSB the calibration was set for.
        :returns: The signed register value.
        """
        return _ina226_current_register(microamps, current_lsb_microamps)

    def power_register(self, microwatts: int, current_lsb_microamps: int) -> int:
        """Build the power register a monitor reports for a power.

        :param microwatts: The power in microwatts.
        :param current_lsb_microamps: The current LSB the calibration was set for.
        :returns: The register value.
        """
        return _ina226_power_register(microwatts, current_lsb_microamps)

    def current_register_from_shunt(self, shunt: int, calibration: int) -> int:
        """Compute the current register the chip derives from a shunt reading.

        :param shunt: The signed shunt-voltage register.
        :param calibration: The calibration register the part is running.
        :returns: The signed current register.
        """
        return _ina226_current_register_from_shunt(shunt, calibration)

    def power_register_from_current(self, current: int, bus: int) -> int:
        """Compute the power register the chip derives from a current reading.

        :param current: The signed current register.
        :param bus: The bus-voltage register.
        :returns: The power register.
        """
        return _ina226_power_register_from_current(current, bus)


#: A Bosch BME280 temperature, pressure, and humidity sensor.
bme280 = _Bme280()

#: A Maxim DS18B20 1-Wire thermometer.
ds18b20 = _Ds18b20()

#: A TI INA219 current, voltage, and power monitor.
ina219 = _Ina219()

#: A TI ADS1115 16-bit analogue-to-digital converter.
ads1115 = _Ads1115()

#: A Bosch BMP280 pressure and temperature sensor.
bmp280 = _Bmp280()

#: A Sensirion SHT30, SHT31, or SHT35 temperature and humidity sensor.
sht3x = _Sht3x()

#: A Sensirion SCD40 or SCD41 carbon dioxide, temperature, and humidity sensor.
scd4x = _Scd4x()

#: A TI TMP117 precision thermometer.
tmp117 = _Tmp117()

#: A TI HDC1080 temperature and humidity sensor.
hdc1080 = _Hdc1080()

#: A TI OPT3001 ambient light sensor.
opt3001 = _Opt3001()

#: A TI INA226 current, voltage, and power monitor.
ina226 = _Ina226()
