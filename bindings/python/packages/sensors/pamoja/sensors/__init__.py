"""Idiomatic sensor-driver facade.

These are the decode half of eleven parts a field node is likely to have wired to
it, turning the register bytes a bus driver read into the physical reading the
manufacturer's datasheet says they mean. The BME280 also has a driver, which runs
the datasheet's whole conversation over an `I2cBus` from :mod:`pamoja.hal`, and a
simulated part that answers it with nothing plugged in.
"""

from __future__ import annotations

import enum
from typing import List, Optional

from pamoja.hal import CommandPart, I2cBus, I2cPart, WordPart

from pamoja._native import Ads1115Config, Bme280Calibration, Bme280Measurement, Ds18b20Reading
from pamoja._native import Bme280 as _NativeBme280
from pamoja._native import Bme280Config, Bme280CtrlMeas
from pamoja._native import bme280_config_bits as _bme280_config_bits
from pamoja._native import bme280_config_from_bits as _bme280_config_from_bits
from pamoja._native import bme280_ctrl_hum_bits as _bme280_ctrl_hum_bits
from pamoja._native import bme280_ctrl_hum_from_bits as _bme280_ctrl_hum_from_bits
from pamoja._native import bme280_ctrl_meas_bits as _bme280_ctrl_meas_bits
from pamoja._native import bme280_ctrl_meas_from_bits as _bme280_ctrl_meas_from_bits
from pamoja._native import bme280_filter_coefficient as _bme280_filter_coefficient
from pamoja._native import bme280_image_updating as _bme280_image_updating
from pamoja._native import bme280_max_measurement_micros as _bme280_max_measurement_micros
from pamoja._native import bme280_measuring as _bme280_measuring
from pamoja._native import bme280_oversampling_factor as _bme280_oversampling_factor
from pamoja._native import bme280_sim_burst as _bme280_sim_burst
from pamoja._native import bme280_sim_burst_for as _bme280_sim_burst_for
from pamoja._native import bme280_sim_calibration as _bme280_sim_calibration
from pamoja._native import (
    bme280_sim_calibration_humidity as _bme280_sim_calibration_humidity,
)
from pamoja._native import bme280_sim_part as _bme280_sim_part
from pamoja._native import bme280_sim_reporting as _bme280_sim_reporting
from pamoja._native import bme280_standby_micros as _bme280_standby_micros
from pamoja._native import (
    bme280_typical_measurement_micros as _bme280_typical_measurement_micros,
)
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
from pamoja._native import Ads1115 as _NativeAds1115
from pamoja._native import Ads1115Sample, Ina219Config, Ina219Reading, Ina226Reading
from pamoja._native import Bmp280 as _NativeBmp280
from pamoja._native import Ds18b20Thermometer as _NativeDs18b20Thermometer
from pamoja._native import Hdc1080 as _NativeHdc1080
from pamoja._native import Ina219 as _NativeIna219
from pamoja._native import Ina226 as _NativeIna226
from pamoja._native import Opt3001 as _NativeOpt3001
from pamoja._native import Opt3001Reading, Tmp117Alerts, Tmp117Reading
from pamoja._native import Scd4x as _NativeScd4x
from pamoja._native import Sht3x as _NativeSht3x
from pamoja._native import Tmp117 as _NativeTmp117
from pamoja._native import ads1115_conversion_micros as _ads1115_conversion_micros
from pamoja._native import ads1115_sim_part as _ads1115_sim_part
from pamoja._native import ads1115_sim_reporting as _ads1115_sim_reporting
from pamoja._native import bmp280_sim_burst as _bmp280_sim_burst
from pamoja._native import bmp280_sim_burst_for as _bmp280_sim_burst_for
from pamoja._native import bmp280_sim_calibration as _bmp280_sim_calibration
from pamoja._native import bmp280_sim_part as _bmp280_sim_part
from pamoja._native import bmp280_sim_reporting as _bmp280_sim_reporting
from pamoja._native import ds18b20_parse_w1_slave as _ds18b20_parse_w1_slave
from pamoja._native import ds18b20_w1_slave_text as _ds18b20_w1_slave_text
from pamoja._native import hdc1080_sim_part as _hdc1080_sim_part
from pamoja._native import hdc1080_sim_reporting as _hdc1080_sim_reporting
from pamoja._native import ina219_adc_conversion_micros as _ina219_adc_conversion_micros
from pamoja._native import ina219_address as _ina219_address
from pamoja._native import ina219_config_bits as _ina219_config_bits
from pamoja._native import ina219_config_from_bits as _ina219_config_from_bits
from pamoja._native import ina219_conversion_micros as _ina219_conversion_micros
from pamoja._native import ina219_gain_range_millivolts as _ina219_gain_range_millivolts
from pamoja._native import ina219_sim_part as _ina219_sim_part
from pamoja._native import ina219_sim_reporting as _ina219_sim_reporting
from pamoja._native import ina226_sim_part as _ina226_sim_part
from pamoja._native import ina226_sim_reporting as _ina226_sim_reporting
from pamoja._native import opt3001_sim_part as _opt3001_sim_part
from pamoja._native import opt3001_sim_reporting as _opt3001_sim_reporting
from pamoja._native import scd4x_sim_part as _scd4x_sim_part
from pamoja._native import scd4x_sim_reporting as _scd4x_sim_reporting
from pamoja._native import sht3x_sim_part as _sht3x_sim_part
from pamoja._native import sht3x_sim_reporting as _sht3x_sim_reporting
from pamoja._native import tmp117_sim_part as _tmp117_sim_part
from pamoja._native import tmp117_sim_reporting as _tmp117_sim_reporting

__all__ = [
    "Ads1115",
    "Ads1115Config",
    "Ads1115DataRate",
    "Ads1115Mux",
    "Ads1115Pga",
    "Ads1115Sample",
    "Bme280",
    "Bme280Calibration",
    "Bme280Config",
    "Bme280CtrlMeas",
    "Bme280Filter",
    "Bme280Measurement",
    "Bme280Mode",
    "Bme280Oversampling",
    "Bme280Standby",
    "Bmp280",
    "Bmp280Calibration",
    "Bmp280Coefficients",
    "Bmp280Config",
    "Bmp280CtrlMeas",
    "Bmp280Oversampling",
    "Bmp280RawMeasurement",
    "Bmp280Reading",
    "Ds18b20Reading",
    "Ds18b20Thermometer",
    "Hdc1080",
    "Hdc1080Config",
    "Hdc1080HumidityResolution",
    "Hdc1080Measurement",
    "Hdc1080TemperatureResolution",
    "Ina219",
    "Ina219Adc",
    "Ina219BusRange",
    "Ina219Config",
    "Ina219Gain",
    "Ina219Mode",
    "Ina219Reading",
    "Ina226",
    "Ina226AlertFunction",
    "Ina226Averaging",
    "Ina226Config",
    "Ina226ConversionTime",
    "Ina226DieId",
    "Ina226Mode",
    "Ina226MaskEnable",
    "Ina226Reading",
    "Opt3001",
    "Opt3001Config",
    "Opt3001ConversionTime",
    "Opt3001Reading",
    "Scd4x",
    "Scd4xMeasurement",
    "Sht3x",
    "Sht3xMeasurement",
    "Sht3xRate",
    "Sht3xRepeatability",
    "Sht3xStatus",
    "Tmp117",
    "Tmp117Alerts",
    "Tmp117Averaging",
    "Tmp117Config",
    "Tmp117Reading",
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


class Bme280Oversampling(enum.IntEnum):
    """How many samples a BME280 measurement averages, as its register code."""

    #: The measurement is skipped.
    SKIPPED = 0
    #: One sample.
    X1 = 1
    #: Two samples.
    X2 = 2
    #: Four samples.
    X4 = 3
    #: Eight samples.
    X8 = 4
    #: Sixteen samples.
    X16 = 5


class Bme280Mode(enum.IntEnum):
    """A BME280 power mode, as its register code."""

    #: No measurements; the power-on default.
    SLEEP = 0
    #: One measurement, then back to sleep.
    FORCED = 1
    #: Measurements on a cycle, a standby period apart.
    NORMAL = 3


class Bme280Filter(enum.IntEnum):
    """The IIR filter that smooths a BME280's pressure and temperature, as its code."""

    #: No filtering.
    OFF = 0
    #: Coefficient 2.
    X2 = 1
    #: Coefficient 4.
    X4 = 2
    #: Coefficient 8.
    X8 = 3
    #: Coefficient 16.
    X16 = 4


class Bme280Standby(enum.IntEnum):
    """The period a BME280 in normal mode rests between measurements, as its code."""

    #: 0.5 ms.
    MS_0_5 = 0
    #: 62.5 ms.
    MS_62_5 = 1
    #: 125 ms.
    MS_125 = 2
    #: 250 ms.
    MS_250 = 3
    #: 500 ms.
    MS_500 = 4
    #: 1000 ms.
    MS_1000 = 5
    #: 10 ms.
    MS_10 = 6
    #: 20 ms.
    MS_20 = 7


class Bmp280Oversampling(enum.IntEnum):
    """How many samples a BMP280 measurement averages, as its register code."""

    #: The measurement is skipped.
    SKIPPED = 0
    #: One sample.
    X1 = 1
    #: Two samples.
    X2 = 2
    #: Four samples.
    X4 = 3
    #: Eight samples.
    X8 = 4
    #: Sixteen samples.
    X16 = 5


class Tmp117Averaging(enum.IntEnum):
    """How many conversions a TMP117 averages into one result, as its register code."""

    #: No averaging: each result is one 15.5 ms conversion.
    NONE = 0
    #: Eight conversions, the factory setting.
    X8 = 1
    #: Thirty-two conversions.
    X32 = 2
    #: Sixty-four conversions.
    X64 = 3


class Opt3001ConversionTime(enum.IntEnum):
    """How long an OPT3001 conversion integrates, in milliseconds."""

    #: 100 ms, for speed.
    MS_100 = 100
    #: 800 ms, for resolution; the part's reset setting.
    MS_800 = 800


class Hdc1080TemperatureResolution(enum.IntEnum):
    """An HDC1080 temperature resolution, as its bit count."""

    #: 11 bits, 3.65 ms.
    BITS_11 = 11
    #: 14 bits, 6.35 ms.
    BITS_14 = 14


class Hdc1080HumidityResolution(enum.IntEnum):
    """An HDC1080 humidity resolution, as its bit count."""

    #: 8 bits, 2.5 ms.
    BITS_8 = 8
    #: 11 bits, 3.85 ms.
    BITS_11 = 11
    #: 14 bits, 6.5 ms.
    BITS_14 = 14


class Ina219BusRange(enum.IntEnum):
    """An INA219 bus-voltage range, as its register code."""

    #: 0 to 16 V.
    V16 = 0
    #: 0 to 32 V, the reset setting.
    V32 = 1


class Ina219Gain(enum.IntEnum):
    """An INA219 shunt gain, by the shunt-voltage range it gives, as its register code."""

    #: Gain 1, 40 mV either side of zero.
    DIV1 = 0
    #: Gain 1/2, 80 mV.
    DIV2 = 1
    #: Gain 1/4, 160 mV.
    DIV4 = 2
    #: Gain 1/8, 320 mV, the reset setting.
    DIV8 = 3


class Ina219Adc(enum.IntEnum):
    """An INA219 converter setting: a resolution, or samples averaged at 12 bits."""

    #: 9 bits, 84 us.
    BITS_9 = 0b0000
    #: 10 bits, 148 us.
    BITS_10 = 0b0001
    #: 11 bits, 276 us.
    BITS_11 = 0b0010
    #: 12 bits, 532 us, the reset setting.
    BITS_12 = 0b0011
    #: 2 samples averaged, 1.06 ms.
    SAMPLES_2 = 0b1001
    #: 4 samples averaged, 2.13 ms.
    SAMPLES_4 = 0b1010
    #: 8 samples averaged, 4.26 ms.
    SAMPLES_8 = 0b1011
    #: 16 samples averaged, 8.51 ms.
    SAMPLES_16 = 0b1100
    #: 32 samples averaged, 17.02 ms.
    SAMPLES_32 = 0b1101
    #: 64 samples averaged, 34.05 ms.
    SAMPLES_64 = 0b1110
    #: 128 samples averaged, 68.10 ms.
    SAMPLES_128 = 0b1111


class Ina219Mode(enum.IntEnum):
    """An INA219 operating mode, as its register code."""

    #: No conversions, lowest power.
    POWER_DOWN = 0
    #: One shunt conversion.
    SHUNT_TRIGGERED = 1
    #: One bus conversion.
    BUS_TRIGGERED = 2
    #: One shunt and one bus conversion.
    SHUNT_AND_BUS_TRIGGERED = 3
    #: The converter disabled.
    ADC_OFF = 4
    #: Shunt conversions back to back.
    SHUNT_CONTINUOUS = 5
    #: Bus conversions back to back.
    BUS_CONTINUOUS = 6
    #: Shunt and bus conversions back to back, the reset setting.
    SHUNT_AND_BUS_CONTINUOUS = 7


class Ina226Averaging(enum.IntEnum):
    """How many samples an INA226 folds into each result, as its register code."""

    #: Every conversion reported, the reset setting.
    SAMPLES_1 = 0
    #: 4 samples.
    SAMPLES_4 = 1
    #: 16 samples.
    SAMPLES_16 = 2
    #: 64 samples.
    SAMPLES_64 = 3
    #: 128 samples.
    SAMPLES_128 = 4
    #: 256 samples.
    SAMPLES_256 = 5
    #: 512 samples.
    SAMPLES_512 = 6
    #: 1024 samples.
    SAMPLES_1024 = 7


class Ina226ConversionTime(enum.IntEnum):
    """An INA226 conversion time, for the bus or the shunt voltage, as its register code."""

    #: 140 us.
    US_140 = 0
    #: 204 us.
    US_204 = 1
    #: 332 us.
    US_332 = 2
    #: 588 us.
    US_588 = 3
    #: 1.1 ms, the reset setting.
    US_1100 = 4
    #: 2.116 ms.
    US_2116 = 5
    #: 4.156 ms.
    US_4156 = 6
    #: 8.244 ms.
    US_8244 = 7


class Ina226Mode(enum.IntEnum):
    """An INA226 operating mode, as its register code."""

    #: No conversions; the registers stay readable and writable.
    POWER_DOWN = 0
    #: One shunt conversion.
    SHUNT_TRIGGERED = 1
    #: One bus conversion.
    BUS_TRIGGERED = 2
    #: One shunt and one bus conversion.
    SHUNT_AND_BUS_TRIGGERED = 3
    #: Shunt conversions back to back.
    SHUNT_CONTINUOUS = 5
    #: Bus conversions back to back.
    BUS_CONTINUOUS = 6
    #: Shunt and bus conversions back to back, the reset setting.
    SHUNT_AND_BUS_CONTINUOUS = 7


class Ads1115Mux(enum.IntEnum):
    """An ADS1115 input multiplexer setting, as its register code."""

    #: AIN0 against AIN1, the reset setting.
    AIN0_AIN1 = 0
    #: AIN0 against AIN3.
    AIN0_AIN3 = 1
    #: AIN1 against AIN3.
    AIN1_AIN3 = 2
    #: AIN2 against AIN3.
    AIN2_AIN3 = 3
    #: AIN0 against ground.
    AIN0_GND = 4
    #: AIN1 against ground.
    AIN1_GND = 5
    #: AIN2 against ground.
    AIN2_GND = 6
    #: AIN3 against ground.
    AIN3_GND = 7


class Ads1115Pga(enum.IntEnum):
    """An ADS1115 full-scale range, as its register code."""

    #: 6.144 V either side of zero.
    FSR_6_144 = 0
    #: 4.096 V.
    FSR_4_096 = 1
    #: 2.048 V, the reset setting.
    FSR_2_048 = 2
    #: 1.024 V.
    FSR_1_024 = 3
    #: 0.512 V.
    FSR_0_512 = 4
    #: 0.256 V.
    FSR_0_256 = 5


class Ads1115DataRate(enum.IntEnum):
    """An ADS1115 data rate, as its register code."""

    #: 8 samples per second.
    SPS_8 = 0
    #: 16 samples per second.
    SPS_16 = 1
    #: 32 samples per second.
    SPS_32 = 2
    #: 64 samples per second.
    SPS_64 = 3
    #: 128 samples per second, the reset setting.
    SPS_128 = 4
    #: 250 samples per second.
    SPS_250 = 5
    #: 475 samples per second.
    SPS_475 = 6
    #: 860 samples per second.
    SPS_860 = 7


class Bme280:
    """A Bosch BME280 driven over an :class:`~pamoja.hal.I2cBus`, measuring on demand in
    forced mode.

    Nothing is sent until :meth:`init` or the first :meth:`measure`. The driver holds its
    own share of the bus, and releases the interpreter while the part answers.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([bme280.sim.part(bme280.ADDRESS_PRIMARY)])
    >>> sensor = Bme280(bus, bme280.ADDRESS_PRIMARY)
    >>> f"{sensor.measure().celsius:.2f}"
    '20.44'
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        bus: I2cBus,
        address: int,
        *,
        temperature: Bme280Oversampling = Bme280Oversampling.X1,
        pressure: Bme280Oversampling = Bme280Oversampling.X1,
        humidity: Bme280Oversampling = Bme280Oversampling.X1,
        filter: Bme280Filter = Bme280Filter.OFF,
    ) -> None:
        """Make a driver for the part at ``address`` on ``bus``.

        :param bus: The bus the part is on.
        :param address: ``bme280.ADDRESS_PRIMARY`` with SDO low, or
            ``bme280.ADDRESS_SECONDARY`` with SDO high.
        :param temperature: The temperature oversampling.
        :param pressure: The pressure oversampling.
        :param humidity: The humidity oversampling.
        :param filter: The IIR filter.
        """
        self._native = _NativeBme280(
            bus._native,
            address,
            int(temperature),
            int(pressure),
            int(humidity),
            int(filter),
        )

    def init(self) -> None:
        """Reset the part, check it is a BME280, read its calibration, and write the
        settings in the order the datasheet requires, leaving the part asleep.

        :raises PamojaError: If nothing answers at the address, another part does, or
            the calibration never finishes loading.
        """
        self._native.init()

    def measure(self) -> Bme280Measurement:
        """Run one forced measurement and compensate it, initializing the part first if
        :meth:`init` has not run.

        :returns: The reading.
        :raises PamojaError: As :meth:`init`, and when the part is still measuring after
            the datasheet's time.
        """
        return self._native.measure()


class _Bme280Sim:
    """A BME280 that is not there, for a bus with nothing plugged in."""

    __slots__ = ()

    #: The status a simulated part reports when it is neither measuring nor loading.
    STATUS_IDLE = 0x00

    def part(self, address: int) -> I2cPart:
        """Make a part holding a real BME280's calibration and one measurement it took,
        which compensate to 20.44 C, 848.05 hPa, and 44.65 %.

        :param address: The address it answers to.
        :returns: The part, to put on a simulated bus.
        """
        return I2cPart._wrap(_bme280_sim_part(address))

    def reporting(
        self, address: int, celsius: float, hectopascals: float, relative_humidity: float
    ) -> I2cPart:
        """Make a part that reads what it is asked to, to within what its converter can
        represent.

        :param address: The address it answers to.
        :param celsius: The temperature it reports.
        :param hectopascals: The pressure it reports.
        :param relative_humidity: The humidity it reports, as a percentage.
        :returns: The part, to put on a simulated bus.
        """
        return I2cPart._wrap(
            _bme280_sim_reporting(address, celsius, hectopascals, relative_humidity)
        )

    def calibration(self) -> bytes:
        """The 26-byte temperature and pressure calibration block a simulated part holds."""
        return bytes(_bme280_sim_calibration())

    def calibration_humidity(self) -> bytes:
        """The 7-byte humidity calibration block a simulated part holds."""
        return bytes(_bme280_sim_calibration_humidity())

    def burst(self) -> bytes:
        """The eight data registers a simulated part holds: one measurement a real part
        took."""
        return bytes(_bme280_sim_burst())

    def burst_for(self, celsius: float, hectopascals: float, relative_humidity: float) -> bytes:
        """Build the eight data registers that compensate to a reading against the
        simulated calibration.

        :param celsius: The temperature.
        :param hectopascals: The pressure.
        :param relative_humidity: The humidity, as a percentage.
        :returns: The bytes a burst read would return.
        """
        return bytes(_bme280_sim_burst_for(celsius, hectopascals, relative_humidity))


class _Bme280:
    """A Bosch BME280 temperature, pressure, and humidity sensor."""

    __slots__ = ()

    #: The address a BME280 answers on with its SDO pin low.
    ADDRESS_PRIMARY = 0x76
    #: The address it answers on with SDO high.
    ADDRESS_SECONDARY = 0x77
    #: The value its chip-ID register reads, which tells it from a BMP280.
    CHIP_ID = 0x60
    #: The word written to the reset register to restart the part.
    RESET_WORD = 0xB6
    #: How long the part takes to start after a reset, in microseconds.
    STARTUP_MICROS = 2_000
    #: How many bytes the temperature and pressure calibration block holds.
    CALIBRATION_TEMP_PRESS_LENGTH = 26
    #: How many bytes the humidity calibration block holds.
    CALIBRATION_HUMIDITY_LENGTH = 7
    #: How many bytes one measurement burst holds.
    DATA_LENGTH = 8
    #: The chip-ID register.
    REGISTER_CHIP_ID = 0xD0
    #: The reset register.
    REGISTER_RESET = 0xE0
    #: The first of the 26 temperature and pressure calibration bytes.
    REGISTER_CALIB_TEMP_PRESS = 0x88
    #: The first of the 7 humidity calibration bytes.
    REGISTER_CALIB_HUMIDITY = 0xE1
    #: The humidity control register, ``ctrl_hum``.
    REGISTER_CTRL_HUM = 0xF2
    #: The status register.
    REGISTER_STATUS = 0xF3
    #: The measurement control register, ``ctrl_meas``.
    REGISTER_CTRL_MEAS = 0xF4
    #: The configuration register, ``config``.
    REGISTER_CONFIG = 0xF5
    #: The first of the 8 data bytes a burst read covers.
    REGISTER_DATA = 0xF7

    #: The oversampling codes.
    Oversampling = Bme280Oversampling
    #: The power mode codes.
    Mode = Bme280Mode
    #: The IIR filter codes.
    Filter = Bme280Filter
    #: The normal-mode standby codes.
    Standby = Bme280Standby

    #: A BME280 that is not there, for a bus with nothing plugged in.
    sim = _Bme280Sim()

    def calibration(self, temp_press: bytes, humidity: bytes) -> Bme280Calibration:
        """Read the factory calibration out of the registers, once at start-up.

        :param temp_press: The 26-byte temperature and pressure calibration block.
        :param humidity: The 7-byte humidity calibration block.
        :returns: The calibration, to reuse for every measurement.
        :raises ValueError: If either block is the wrong length.
        """
        return Bme280Calibration(bytes(temp_press), bytes(humidity))

    def measuring(self, status: int) -> bool:
        """Report whether a status register says a conversion is running.

        :param status: The status register.
        :returns: Whether a measurement is in progress.
        """
        return _bme280_measuring(status)

    def image_updating(self, status: int) -> bool:
        """Report whether a status register says the calibration image is loading.

        :param status: The status register.
        :returns: Whether the calibration is still being copied.
        """
        return _bme280_image_updating(status)

    def ctrl_meas_bits(self, ctrl: Bme280CtrlMeas) -> int:
        """Pack a ``ctrl_meas`` register value.

        :param ctrl: The oversampling codes and the power mode.
        :returns: The register value to write.
        """
        return _bme280_ctrl_meas_bits(ctrl)

    def ctrl_meas_from_bits(self, bits: int) -> Bme280CtrlMeas:
        """Parse a ``ctrl_meas`` register value.

        :param bits: The register value, as read from the part.
        :returns: The oversampling codes and the power mode.
        """
        return _bme280_ctrl_meas_from_bits(bits)

    def ctrl_hum_bits(self, humidity: int) -> int:
        """Pack a ``ctrl_hum`` register value, which takes effect only after the next
        ``ctrl_meas`` write.

        :param humidity: The humidity oversampling code.
        :returns: The register value to write.
        """
        return _bme280_ctrl_hum_bits(int(humidity))

    def ctrl_hum_from_bits(self, bits: int) -> Bme280Oversampling:
        """Parse a ``ctrl_hum`` register value.

        :param bits: The register value, as read from the part.
        :returns: The humidity oversampling.
        """
        return Bme280Oversampling(_bme280_ctrl_hum_from_bits(bits))

    def config_bits(self, config: Bme280Config) -> int:
        """Pack a ``config`` register value.

        :param config: The standby period, filter, and interface settings.
        :returns: The register value to write.
        """
        return _bme280_config_bits(config)

    def config_from_bits(self, bits: int) -> Bme280Config:
        """Parse a ``config`` register value.

        :param bits: The register value, as read from the part.
        :returns: The standby period, filter, and interface settings.
        """
        return _bme280_config_from_bits(bits)

    def oversampling_factor(self, code: int) -> int:
        """Return how many samples an oversampling code averages.

        :param code: The oversampling code.
        :returns: The factor, or 0 when the code skips the measurement.
        """
        return _bme280_oversampling_factor(int(code))

    def standby_micros(self, code: int) -> int:
        """Return the standby period a code selects in normal mode.

        :param code: The standby code.
        :returns: The period in microseconds.
        """
        return _bme280_standby_micros(int(code))

    def filter_coefficient(self, code: int) -> int:
        """Return the IIR coefficient a filter code selects.

        :param code: The filter code.
        :returns: The coefficient, or 0 when the filter is off.
        """
        return _bme280_filter_coefficient(int(code))

    def max_measurement_micros(self, temperature: int, pressure: int, humidity: int) -> int:
        """Return the longest one measurement can take, which is how long a driver waits
        after forcing one.

        :param temperature: The temperature oversampling code.
        :param pressure: The pressure oversampling code.
        :param humidity: The humidity oversampling code.
        :returns: The datasheet's maximum in microseconds.
        """
        return _bme280_max_measurement_micros(int(temperature), int(pressure), int(humidity))

    def typical_measurement_micros(self, temperature: int, pressure: int, humidity: int) -> int:
        """Return the typical time one measurement takes.

        :param temperature: The temperature oversampling code.
        :param pressure: The pressure oversampling code.
        :param humidity: The humidity oversampling code.
        :returns: The datasheet's typical time in microseconds.
        """
        return _bme280_typical_measurement_micros(
            int(temperature), int(pressure), int(humidity)
        )


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

    def parse_w1_slave(self, text: str) -> Ds18b20Reading:
        """Decode the text the Linux kernel's ``w1_therm`` driver serves for a thermometer:
        the scratchpad in hex with the kernel's checksum verdict, then the temperature.

        :param text: The ``w1_slave`` file's contents.
        :returns: The reading, its CRC checked here as well.
        :raises PamojaError: If the kernel or this decoder rejects the CRC, or the text is
            not in the driver's format.
        """
        return _ds18b20_parse_w1_slave(text)

    def w1_slave_text(self, data: bytes) -> str:
        """Render the text the Linux kernel's ``w1_therm`` driver serves for a scratchpad
        it read cleanly, the inverse of :meth:`parse_w1_slave`: the bytes with the CRC the
        kernel computed and ``YES``, then the bytes again with ``t=`` and the temperature
        in millidegrees.

        :param data: The nine scratchpad bytes, the ninth their CRC.
        :returns: The ``w1_slave`` file's two lines.
        :raises ValueError: If the bytes are not nine.
        :raises PamojaError: If the CRC does not match.
        """
        return _ds18b20_w1_slave_text(bytes(data))


class _Ina219Sim:
    """An INA219 that is not there, for a bus with nothing plugged in.

    A monitor's current and power registers count in steps the calibration sets, so
    :meth:`reporting` takes the same shunt and largest current a driver is given.
    """

    __slots__ = ()

    #: The shunt :meth:`part` sits across, in milliohms: the common breakout's.
    SHUNT_MILLIOHMS = 100
    #: The largest current :meth:`part` is sized for, in microamps.
    MAX_MICROAMPS = 3_200_000
    #: The bus voltage :meth:`part` reports, in millivolts.
    BUS_MILLIVOLTS = 12_000
    #: The current :meth:`part` reports, in microamps.
    MICROAMPS = 500_000

    def part(self, address: int) -> WordPart:
        """Make a part carrying 500 mA at 12 V through the shunt a driver starts with.

        :param address: The address it answers to.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_ina219_sim_part(address))

    def reporting(
        self,
        address: int,
        shunt_milliohms: int,
        max_microamps: int,
        bus_millivolts: int,
        microamps: int,
    ) -> WordPart:
        """Make a part that reads what it is asked to, on the steps its registers count
        in: 4 mV of bus, 10 uV of shunt, and the calibration's current step.

        :param address: The address it answers to.
        :param shunt_milliohms: The shunt, as the driver is given it.
        :param max_microamps: The largest current, as the driver is given it.
        :param bus_millivolts: The bus voltage it reports.
        :param microamps: The current it reports; negative flows the other way.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(
            _ina219_sim_reporting(
                address, shunt_milliohms, max_microamps, bus_millivolts, microamps
            )
        )


class _Ina219:
    """A TI INA219 current, voltage, and power monitor."""

    __slots__ = ()

    #: The address with A1 and A0 tied to ground; the pins add to it.
    BASE_ADDRESS = 0x40
    #: An address pin tied to GND, as the code :meth:`address` takes.
    PIN_GROUND = 0
    #: An address pin tied to VS+.
    PIN_SUPPLY = 1
    #: An address pin tied to SDA.
    PIN_SDA = 2
    #: An address pin tied to SCL.
    PIN_SCL = 3
    #: The configuration register's power-on value.
    CONFIG_RESET = 0x399F
    #: The configuration register: range, gain, converter settings, and mode.
    REGISTER_CONFIGURATION = 0x00
    #: The shunt voltage, 10 uV per count.
    REGISTER_SHUNT_VOLTAGE = 0x01
    #: The bus voltage in bits 15:3, 4 mV per count, with two flags below.
    REGISTER_BUS_VOLTAGE = 0x02
    #: The power, scaled by the calibration.
    REGISTER_POWER = 0x03
    #: The current, scaled by the calibration.
    REGISTER_CURRENT = 0x04
    #: The calibration, which sets the current and power scale.
    REGISTER_CALIBRATION = 0x05

    #: The bus-voltage range codes.
    BusRange = Ina219BusRange
    #: The shunt gain codes.
    Gain = Ina219Gain
    #: The converter codes.
    Adc = Ina219Adc
    #: The operating-mode codes.
    Mode = Ina219Mode
    #: An INA219 that is not there, for a bus with nothing plugged in.
    sim = _Ina219Sim()

    def address(self, a1: int, a0: int) -> int:
        """Return the 7-bit address the A1 and A0 pins select, from Table 1 of the
        datasheet.

        :param a1: What the A1 pin is tied to, one of the ``PIN_`` codes.
        :param a0: What the A0 pin is tied to.
        :returns: The address, 0x40 to 0x4F.
        :raises ValueError: If either pin code is not 0, 1, 2, or 3.
        """
        return _ina219_address(a1, a0)

    def config_bits(self, config: Ina219Config) -> int:
        """Assemble the configuration register value.

        :param config: The range, gain, converter, and mode codes.
        :returns: The register value to write.
        """
        return _ina219_config_bits(config)

    def config_from_bits(self, bits: int) -> Ina219Config:
        """Parse a configuration register value.

        :param bits: The register value, as read from the part.
        :returns: The settings. Every value decodes.
        """
        return _ina219_config_from_bits(bits)

    def conversion_micros(self, config: Ina219Config) -> int:
        """Return how long one conversion cycle takes: the shunt and bus conversions the
        mode runs, one after the other.

        :param config: The settings.
        :returns: The time in microseconds.
        """
        return _ina219_conversion_micros(config)

    def adc_conversion_micros(self, adc: Ina219Adc) -> int:
        """Return how long one conversion takes at a converter setting.

        :param adc: The setting.
        :returns: The time in microseconds, from the datasheet's table.
        """
        return _ina219_adc_conversion_micros(int(adc))

    def gain_range_millivolts(self, gain: Ina219Gain) -> int:
        """Return the shunt-voltage range a gain selects.

        :param gain: The gain.
        :returns: The range in millivolts either side of zero.
        """
        return _ina219_gain_range_millivolts(int(gain))

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


class _Ads1115Sim:
    """An ADS1115 that is not there, for a bus with nothing plugged in.

    A conversion comes back as a count of the range the gain selects, so :meth:`reporting`
    takes the same gain a driver is given.
    """

    __slots__ = ()

    #: The voltage :meth:`part` reports: half a 3.3 V supply.
    VOLTS = 1.65

    def part(self, address: int) -> WordPart:
        """Make a part reading :attr:`VOLTS` at the range a driver starts with.

        :param address: The address it answers to.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_ads1115_sim_part(address))

    def reporting(self, address: int, pga: Ads1115Pga, volts: float) -> WordPart:
        """Make a part that reads what it is asked to, on the nearest of the 32768 steps
        either side of zero the range divides into.

        :param address: The address it answers to.
        :param pga: The range the driver converts at.
        :param volts: The voltage it reports, held to the range.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_ads1115_sim_reporting(address, int(pga), volts))


class _Ads1115:
    """A TI ADS1115 16-bit analog-to-digital converter."""

    __slots__ = ()

    #: The address with ADDR tied to ground.
    ADDRESS_GND = 0x48
    #: The address with ADDR tied to VDD.
    ADDRESS_VDD = 0x49
    #: The address with ADDR tied to SDA.
    ADDRESS_SDA = 0x4A
    #: The address with ADDR tied to SCL.
    ADDRESS_SCL = 0x4B
    #: The conversion register.
    REGISTER_CONVERSION = 0x00
    #: The configuration register.
    REGISTER_CONFIG = 0x01
    #: The comparator's low threshold register.
    REGISTER_LO_THRESH = 0x02
    #: The comparator's high threshold register.
    REGISTER_HI_THRESH = 0x03

    #: The input multiplexer codes.
    Mux = Ads1115Mux
    #: The full-scale range codes.
    Pga = Ads1115Pga
    #: The data-rate codes.
    DataRate = Ads1115DataRate
    #: An ADS1115 that is not there, for a bus with nothing plugged in.
    sim = _Ads1115Sim()

    def conversion_micros(self, data_rate: Ads1115DataRate) -> int:
        """Return how long a conversion takes at a data rate: one period plus the
        datasheet's ten percent rate variation.

        :param data_rate: The data rate.
        :returns: The time in microseconds.
        """
        return _ads1115_conversion_micros(int(data_rate))

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


class _Bmp280Sim:
    """A BMP280 that is not there, for a bus with nothing plugged in.

    A BMP280 is a BME280 without humidity, so :meth:`part` holds the temperature and
    pressure half of a real part and reads 20.44 C and 848.05 hPa.
    """

    __slots__ = ()

    #: The status a simulated part reports when it is neither measuring nor loading.
    STATUS_IDLE = 0x00

    def part(self, address: int) -> I2cPart:
        """Make a part holding a real part's trimming and one measurement it took.

        :param address: The address it answers to.
        :returns: The part, to put on a simulated bus.
        """
        return I2cPart._wrap(_bmp280_sim_part(address))

    def reporting(self, address: int, celsius: float, hectopascals: float) -> I2cPart:
        """Make a part that reads what it is asked to, within a hundredth of a degree and
        of a hectopascal.

        :param address: The address it answers to.
        :param celsius: The temperature it reports.
        :param hectopascals: The pressure it reports.
        :returns: The part, to put on a simulated bus.
        """
        return I2cPart._wrap(_bmp280_sim_reporting(address, celsius, hectopascals))

    def calibration(self) -> bytes:
        """The 24 trimming bytes a simulated part holds."""
        return bytes(_bmp280_sim_calibration())

    def burst(self) -> bytes:
        """The six data registers a simulated part holds: one measurement a real part
        took."""
        return bytes(_bmp280_sim_burst())

    def burst_for(self, celsius: float, hectopascals: float) -> bytes:
        """Build the six data registers that compensate to a reading against the simulated
        trimming.

        :param celsius: The temperature.
        :param hectopascals: The pressure.
        :returns: The bytes a burst read would return.
        """
        return bytes(_bmp280_sim_burst_for(celsius, hectopascals))


class _Bmp280:
    """A Bosch BMP280 pressure and temperature sensor, the BME280 without humidity."""

    __slots__ = ()

    #: The oversampling codes.
    Oversampling = Bmp280Oversampling
    #: A BMP280 that is not there, for a bus with nothing plugged in.
    sim = _Bmp280Sim()

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


class _Sht3xSim:
    """An SHT3x that is not there, for a bus with nothing plugged in.

    It answers every single-shot command and a periodic fetch with the reading and its
    checksums, and the status command with the status a part reports after a reset.
    """

    __slots__ = ()

    #: The temperature :meth:`part` reports.
    CELSIUS = 22.5
    #: The relative humidity :meth:`part` reports, as a percentage.
    RELATIVE_HUMIDITY = 45.0

    def part(self, address: int) -> CommandPart:
        """Make a part reading :attr:`CELSIUS` and :attr:`RELATIVE_HUMIDITY`.

        :param address: The address it answers to.
        :returns: The part, to put on a simulated bus.
        """
        return CommandPart._wrap(_sht3x_sim_part(address))

    def reporting(self, address: int, celsius: float, relative_humidity: float) -> CommandPart:
        """Make a part that reads what it is asked to, within three thousandths of a degree
        and two thousandths of a percent.

        :param address: The address it answers to.
        :param celsius: The temperature it reports.
        :param relative_humidity: The humidity it reports, as a percentage.
        :returns: The part, to put on a simulated bus.
        """
        return CommandPart._wrap(_sht3x_sim_reporting(address, celsius, relative_humidity))


class _Sht3x:
    """A Sensirion SHT30, SHT31, or SHT35 temperature and humidity sensor."""

    __slots__ = ()

    #: An SHT3x that is not there, for a bus with nothing plugged in.
    sim = _Sht3xSim()

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

class _Scd4xSim:
    """An SCD4x that is not there, for a bus with nothing plugged in.

    It answers the serial number, data-ready, and measurement commands, each word with its
    checksum, and always has a result waiting.
    """

    __slots__ = ()

    #: The carbon dioxide :meth:`part` reports, in parts per million.
    CO2_PPM = 800
    #: The temperature :meth:`part` reports.
    CELSIUS = 22.5
    #: The relative humidity :meth:`part` reports, as a percentage.
    RELATIVE_HUMIDITY = 45.0
    #: The serial number every simulated part reports.
    SERIAL = 0x0000_5A4D_0C1E_2B3F

    def part(self) -> CommandPart:
        """Make a part reading :attr:`CO2_PPM`, :attr:`CELSIUS`, and
        :attr:`RELATIVE_HUMIDITY`.

        :returns: The part, to put on a simulated bus.
        """
        return CommandPart._wrap(_scd4x_sim_part())

    def reporting(self, co2_ppm: int, celsius: float, relative_humidity: float) -> CommandPart:
        """Make a part that reads what it is asked to.

        :param co2_ppm: The carbon dioxide it reports, in parts per million.
        :param celsius: The temperature it reports.
        :param relative_humidity: The humidity it reports, as a percentage.
        :returns: The part, to put on a simulated bus.
        """
        return CommandPart._wrap(_scd4x_sim_reporting(co2_ppm, celsius, relative_humidity))


class _Scd4x:
    """A Sensirion SCD40 or SCD41 carbon dioxide, temperature, and humidity sensor."""

    __slots__ = ()

    #: An SCD4x that is not there, for a bus with nothing plugged in.
    sim = _Scd4xSim()

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
    #: Reinitialize from the stored settings.
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


class _Tmp117Sim:
    """A TMP117 that is not there, for a bus with nothing plugged in.

    Its configuration register keeps the flags the part sets for itself whatever a driver
    writes, with the data-ready flag set, so every conversion reads as finished.
    """

    __slots__ = ()

    #: The temperature :meth:`part` reports.
    CELSIUS = 21.25

    def part(self, address: int) -> WordPart:
        """Make a part reading :attr:`CELSIUS`.

        :param address: The address it answers to.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_tmp117_sim_part(address))

    def reporting(self, address: int, celsius: float) -> WordPart:
        """Make a part that reads what it is asked to, to the nearest 7.8125 millidegrees.

        :param address: The address it answers to.
        :param celsius: The temperature it reports.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_tmp117_sim_reporting(address, celsius))


class _Tmp117:
    """A TI TMP117 precision thermometer."""

    __slots__ = ()

    #: The averaging codes.
    Averaging = Tmp117Averaging
    #: A TMP117 that is not there, for a bus with nothing plugged in.
    sim = _Tmp117Sim()

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

class _Hdc1080Sim:
    """An HDC1080 that is not there, for a bus with nothing plugged in."""

    __slots__ = ()

    #: The temperature :meth:`part` reports.
    CELSIUS = 22.5
    #: The relative humidity :meth:`part` reports, as a percentage.
    RELATIVE_HUMIDITY = 45.0

    def part(self) -> WordPart:
        """Make a part reading :attr:`CELSIUS` and :attr:`RELATIVE_HUMIDITY`, at the one
        address an HDC1080 has.

        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_hdc1080_sim_part())

    def reporting(self, celsius: float, relative_humidity: float) -> WordPart:
        """Make a part that reads what it is asked to, within three thousandths of a degree
        and two thousandths of a percent.

        :param celsius: The temperature it reports.
        :param relative_humidity: The humidity it reports, as a percentage.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_hdc1080_sim_reporting(celsius, relative_humidity))


class _Hdc1080:
    """A TI HDC1080 temperature and humidity sensor."""

    __slots__ = ()

    #: The temperature resolutions.
    TemperatureResolution = Hdc1080TemperatureResolution
    #: The humidity resolutions.
    HumidityResolution = Hdc1080HumidityResolution
    #: An HDC1080 that is not there, for a bus with nothing plugged in.
    sim = _Hdc1080Sim()

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


class _Opt3001Sim:
    """An OPT3001 that is not there, for a bus with nothing plugged in.

    Its configuration register keeps the flags the part sets for itself whatever a driver
    writes, with the conversion-ready flag set, so every conversion reads as finished.
    """

    __slots__ = ()

    #: The illuminance :meth:`part` reports.
    LUX = 380.0

    def part(self, address: int) -> WordPart:
        """Make a part reading :attr:`LUX`.

        :param address: The address it answers to.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_opt3001_sim_part(address))

    def reporting(self, address: int, lux: float) -> WordPart:
        """Make a part that reads what it is asked to, to the nearest step its exponent and
        mantissa represent.

        :param address: The address it answers to.
        :param lux: The illuminance it reports.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_opt3001_sim_reporting(address, lux))


class _Opt3001:
    """A TI OPT3001 ambient light sensor."""

    __slots__ = ()

    #: The conversion times.
    ConversionTime = Opt3001ConversionTime
    #: An OPT3001 that is not there, for a bus with nothing plugged in.
    sim = _Opt3001Sim()

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


class _Ina226Sim:
    """An INA226 that is not there, for a bus with nothing plugged in.

    It carries TI's manufacturer id and the INA226 die id, its conversion-ready flag is set,
    and :meth:`reporting` takes the same shunt and largest current a driver is given.
    """

    __slots__ = ()

    #: The shunt :meth:`part` sits across, in milliohms.
    SHUNT_MILLIOHMS = 100
    #: The largest current :meth:`part` is sized for, in microamps.
    MAX_MICROAMPS = 3_200_000
    #: The bus voltage :meth:`part` reports, in microvolts.
    BUS_MICROVOLTS = 12_000_000
    #: The current :meth:`part` reports, in microamps.
    MICROAMPS = 500_000

    def part(self, address: int) -> WordPart:
        """Make a part carrying 500 mA at 12 V through the shunt a driver starts with.

        :param address: The address it answers to.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(_ina226_sim_part(address))

    def reporting(
        self,
        address: int,
        shunt_milliohms: int,
        max_microamps: int,
        bus_microvolts: int,
        microamps: int,
    ) -> WordPart:
        """Make a part that reads what it is asked to, on the steps its registers count
        in: 1.25 mV of bus, 2.5 uV of shunt, and the calibration's current step.

        :param address: The address it answers to.
        :param shunt_milliohms: The shunt, as the driver is given it.
        :param max_microamps: The largest current, as the driver is given it.
        :param bus_microvolts: The bus voltage it reports.
        :param microamps: The current it reports; negative flows the other way.
        :returns: The part, to put on a simulated bus.
        """
        return WordPart._wrap(
            _ina226_sim_reporting(
                address, shunt_milliohms, max_microamps, bus_microvolts, microamps
            )
        )


class _Ina226:
    """A TI INA226 current, voltage, and power monitor."""

    __slots__ = ()

    #: An INA226 that is not there, for a bus with nothing plugged in.
    sim = _Ina226Sim()

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


class Bmp280:
    """A Bosch BMP280 driven over an :class:`~pamoja.hal.I2cBus`, measuring on demand in
    forced mode.

    Nothing is sent until :meth:`init` or the first :meth:`measure`. The driver holds its
    own share of the bus, and releases the interpreter while the part answers.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([bmp280.sim.part(bmp280.ADDRESS_PRIMARY)])
    >>> f"{Bmp280(bus, bmp280.ADDRESS_PRIMARY).measure().celsius:.2f}"
    '20.44'
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        bus: I2cBus,
        address: int,
        *,
        temperature: Bmp280Oversampling = Bmp280Oversampling.X1,
        pressure: Bmp280Oversampling = Bmp280Oversampling.X1,
        filter: int = 0,
    ) -> None:
        """Make a driver for the part at ``address`` on ``bus``.

        :param bus: The bus the part is on.
        :param address: ``bmp280.ADDRESS_PRIMARY`` with SDO low, or
            ``bmp280.ADDRESS_SECONDARY`` with SDO high.
        :param temperature: The temperature oversampling.
        :param pressure: The pressure oversampling.
        :param filter: The IIR filter's three-bit code, ``0`` for the filter off, written
            as given.
        """
        self._native = _NativeBmp280(
            bus._native, address, int(temperature), int(pressure), int(filter)
        )

    def init(self) -> None:
        """Reset the part, check it is a BMP280, read its trimming, and write the settings,
        leaving the part asleep.

        :raises PamojaError: If nothing answers at the address, another part does, or the
            trimming never finishes loading.
        """
        self._native.init()

    def measure(self) -> Bmp280Reading:
        """Run one forced measurement and compensate it, initializing the part first if
        :meth:`init` has not run.

        :returns: The reading.
        :raises PamojaError: As :meth:`init`, and when the part is still measuring after
            the datasheet's time.
        """
        return self._native.measure()

    @property
    def coefficients(self) -> Optional[Bmp280Coefficients]:
        """The trimming coefficients read at initialization, or ``None`` before it."""
        return self._native.coefficients


class Tmp117:
    """A Texas Instruments TMP117 driven over an :class:`~pamoja.hal.I2cBus`, converting on
    demand in one-shot mode and powered down between conversions.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([tmp117.sim.reporting(tmp117.ADDRESS_ADD0_GND, -18.5)])
    >>> Tmp117(bus, tmp117.ADDRESS_ADD0_GND).measure().celsius
    -18.5
    """

    __slots__ = ("_native",)

    def __init__(
        self, bus: I2cBus, address: int, *, averaging: Tmp117Averaging = Tmp117Averaging.X8
    ) -> None:
        """Make a driver for the part at ``address`` on ``bus``.

        :param bus: The bus the part is on.
        :param address: One of the ``tmp117.ADDRESS_ADD0_*`` constants, by where ADD0 is
            tied.
        :param averaging: How many conversions are averaged into each result.
        """
        self._native = _NativeTmp117(bus._native, address, int(averaging))

    def init(self) -> None:
        """Check the part is a TMP117, wait for its EEPROM, and write the settings with the
        part in shutdown.

        :raises PamojaError: If nothing answers, the device id is not a TMP117's, or the
            EEPROM never reports ready.
        """
        self._native.init()

    def measure(self) -> Tmp117Reading:
        """Run one conversion and read the temperature, initializing the part first if
        :meth:`init` has not run.

        :returns: The temperature.
        :raises PamojaError: As :meth:`init`, and when the conversion never finishes.
        """
        return self._native.measure()

    def set_alert_limits(self, high_celsius: float, low_celsius: float) -> None:
        """Write the high and low limits the part compares each result against.

        :param high_celsius: The high limit; the factory value is 192 C.
        :param low_celsius: The low limit; the factory value is -256 C.
        :raises PamojaError: If the transfer fails.
        """
        self._native.set_alert_limits(high_celsius, low_celsius)

    def alerts(self) -> Tmp117Alerts:
        """Read the alert flags, including results the driver's own reads saw since the
        last call.

        :returns: Whether a result was above the high limit or below the low limit.
        :raises PamojaError: If the transfer fails.
        """
        return self._native.alerts()

    @property
    def silicon_revision(self) -> Optional[int]:
        """The silicon revision read at initialization, or ``None`` before it."""
        return self._native.silicon_revision


class Opt3001:
    """A Texas Instruments OPT3001 driven over an :class:`~pamoja.hal.I2cBus`, measuring
    illuminance on demand in single-shot mode.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([opt3001.sim.reporting(opt3001.ADDRESS_GND, 1200.0)])
    >>> Opt3001(bus, opt3001.ADDRESS_GND).measure().lux
    1200.0
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        bus: I2cBus,
        address: int,
        *,
        conversion_time: Opt3001ConversionTime = Opt3001ConversionTime.MS_800,
        range_number: int = 12,
    ) -> None:
        """Make a driver for the part at ``address`` on ``bus``.

        :param bus: The bus the part is on.
        :param address: One of the ``opt3001.ADDRESS_*`` constants, by where ADDR is tied.
        :param conversion_time: How long each conversion integrates.
        :param range_number: A fixed full-scale range, ``0`` to ``11``, or ``12`` to let
            the part choose.
        """
        self._native = _NativeOpt3001(
            bus._native,
            address,
            Opt3001ConversionTime(conversion_time) == Opt3001ConversionTime.MS_800,
            range_number,
        )

    def init(self) -> None:
        """Check the part is an OPT3001 and write the settings with the part in shutdown.

        :raises PamojaError: If nothing answers, or either id register is not an OPT3001's.
        """
        self._native.init()

    def measure(self) -> Opt3001Reading:
        """Run one conversion and read the illuminance, initializing the part first if
        :meth:`init` has not run.

        :returns: The illuminance.
        :raises PamojaError: As :meth:`init`, and when the conversion never finishes.
        """
        return self._native.measure()

    def set_limits(self, low_milli_lux: int, high_milli_lux: int) -> None:
        """Write the low and high limits the part's interrupt pin compares each result
        against.

        :param low_milli_lux: The low limit, in millilux.
        :param high_milli_lux: The high limit, in millilux.
        :raises PamojaError: If the transfer fails.
        """
        self._native.set_limits(low_milli_lux, high_milli_lux)

    @property
    def configuration(self) -> Opt3001Config:
        """The configuration the driver writes, with the part in shutdown."""
        return self._native.configuration


class Hdc1080:
    """A Texas Instruments HDC1080 driven over an :class:`~pamoja.hal.I2cBus`, measuring
    temperature then humidity from one trigger at its one address.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([hdc1080.sim.reporting(4.0, 91.0)])
    >>> round(Hdc1080(bus).measure().relative_humidity, 2)
    91.0
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        bus: I2cBus,
        *,
        temperature_resolution: Hdc1080TemperatureResolution = (
            Hdc1080TemperatureResolution.BITS_14
        ),
        humidity_resolution: Hdc1080HumidityResolution = Hdc1080HumidityResolution.BITS_14,
    ) -> None:
        """Make a driver for the part on ``bus``.

        :param bus: The bus the part is on.
        :param temperature_resolution: The temperature resolution, which sets its time.
        :param humidity_resolution: The humidity resolution, which sets its time.
        :raises ValueError: For a resolution the part does not have.
        """
        self._native = _NativeHdc1080(
            bus._native, int(temperature_resolution), int(humidity_resolution)
        )

    def init(self) -> None:
        """Check the part is an HDC1080 and write the configuration.

        :raises PamojaError: If nothing answers, or either id register is not an HDC1080's.
        """
        self._native.init()

    def measure(self) -> Hdc1080Measurement:
        """Trigger one acquisition of both channels and read them, initializing the part
        first if :meth:`init` has not run.

        :returns: The temperature and humidity.
        :raises PamojaError: As :meth:`init`, and when the part does not acknowledge the
            read, which it refuses until its results are ready.
        """
        return self._native.measure()

    def heater(self, on: bool) -> None:
        """Switch the on-die heater, which runs only during acquisitions, on or off.

        :param on: Whether the heater runs.
        :raises PamojaError: If the transfer fails.
        """
        self._native.heater(on)

    @property
    def configuration(self) -> Hdc1080Config:
        """The configuration the driver writes."""
        return self._native.configuration


class Ina219:
    """A Texas Instruments INA219 driven over an :class:`~pamoja.hal.I2cBus`: shunt and bus
    voltage, current, and power on demand.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([ina219.sim.part(ina219.BASE_ADDRESS)])
    >>> Ina219(bus, ina219.BASE_ADDRESS).measure().bus_millivolts
    12000
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        bus: I2cBus,
        address: int,
        *,
        shunt_milliohms: int = 100,
        max_microamps: int = 3_200_000,
        current_lsb_microamps: Optional[int] = None,
        config: Optional[Ina219Config] = None,
    ) -> None:
        """Make a driver for the part at ``address`` on ``bus``.

        :param bus: The bus the part is on.
        :param address: ``ina219.BASE_ADDRESS`` plus what A1 and A0 add.
        :param shunt_milliohms: The shunt resistance; the common breakout's is 100.
        :param max_microamps: The largest current the shunt will carry, which sets the
            finest current step.
        :param current_lsb_microamps: A current step to use instead, such as a round 100.
        :param config: The range, gain, and converter settings; the power-on ones unless
            given.
        """
        self._native = _NativeIna219(
            bus._native,
            address,
            shunt_milliohms,
            max_microamps,
            current_lsb_microamps,
            config,
        )

    def init(self) -> None:
        """Reset the part, write the configuration and the calibration, and read the
        calibration back, the identity check a part with no id register allows.

        :raises PamojaError: If nothing answers, or the calibration does not hold.
        """
        self._native.init()

    def measure(self) -> Ina219Reading:
        """Trigger one shunt and bus conversion and read every result, initializing the
        part first if :meth:`init` has not run.

        :returns: The registers as read, and what they mean.
        :raises PamojaError: As :meth:`init`, and when the conversion-ready flag never sets.
        """
        return self._native.measure()

    @property
    def current_lsb_microamps(self) -> int:
        """The current step the driver programs, in microamps per count."""
        return self._native.current_lsb_microamps

    @property
    def calibration_word(self) -> int:
        """The calibration word the driver programs."""
        return self._native.calibration_word


class Ina226:
    """A Texas Instruments INA226 driven over an :class:`~pamoja.hal.I2cBus`: shunt and bus
    voltage, current, and power on demand.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([ina226.sim.part(ina226.BASE_ADDRESS)])
    >>> Ina226(bus, ina226.BASE_ADDRESS).measure().bus_microvolts
    12000000
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        bus: I2cBus,
        address: int,
        *,
        shunt_milliohms: int = 100,
        max_microamps: int = 3_200_000,
        current_lsb_microamps: Optional[int] = None,
        config: Optional[Ina226Config] = None,
    ) -> None:
        """Make a driver for the part at ``address`` on ``bus``.

        :param bus: The bus the part is on.
        :param address: The address A1 and A0 select, which ``ina226.address`` works out.
        :param shunt_milliohms: The shunt resistance.
        :param max_microamps: The largest current the shunt will carry, which sets the
            finest current step.
        :param current_lsb_microamps: A current step to use instead.
        :param config: The averaging and conversion times; the power-on ones unless given.
        """
        self._native = _NativeIna226(
            bus._native,
            address,
            shunt_milliohms,
            max_microamps,
            current_lsb_microamps,
            config,
        )

    def init(self) -> None:
        """Reset the part, check it is an INA226, and program the configuration and
        calibration.

        :raises PamojaError: If nothing answers, the id registers are not an INA226's, or
            the calibration does not read back.
        """
        self._native.init()

    def measure(self) -> Ina226Reading:
        """Trigger one shunt and bus conversion and read every result, initializing the
        part first if :meth:`init` has not run.

        :returns: The registers as read, and what they mean.
        :raises PamojaError: As :meth:`init`, and when the conversion-ready flag never sets.
        """
        return self._native.measure()

    def set_alert(self, mask: Ina226MaskEnable, limit: int) -> None:
        """Program the alert pin: which limit it watches, and the limit.

        :param mask: The mask/enable settings, one alert function at a time.
        :param limit: The alert-limit register, in the units of the register the function
            watches.
        :raises PamojaError: If the transfer fails.
        """
        self._native.set_alert(mask, limit)

    @property
    def current_lsb_microamps(self) -> int:
        """The current step the driver programs, in microamps per count."""
        return self._native.current_lsb_microamps

    @property
    def calibration_word(self) -> int:
        """The calibration word the driver programs."""
        return self._native.calibration_word

    @property
    def identity(self) -> Optional[Ina226DieId]:
        """The die id read at initialization, or ``None`` before it."""
        return self._native.identity


class Ads1115:
    """A Texas Instruments ADS1115 driven over an :class:`~pamoja.hal.I2cBus`, converting
    one input on demand.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([ads1115.sim.part(ads1115.ADDRESS_GND)])
    >>> round(Ads1115(bus, ads1115.ADDRESS_GND).sample().volts, 3)
    1.65
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        bus: I2cBus,
        address: int,
        *,
        mux: Ads1115Mux = Ads1115Mux.AIN0_AIN1,
        pga: Ads1115Pga = Ads1115Pga.FSR_2_048,
        data_rate: Ads1115DataRate = Ads1115DataRate.SPS_128,
    ) -> None:
        """Make a driver for the part at ``address`` on ``bus``.

        :param bus: The bus the part is on.
        :param address: One of the ``ads1115.ADDRESS_*`` constants, by where ADDR is tied.
        :param mux: The input the part converts.
        :param pga: The full-scale range, and with it the size of one count.
        :param data_rate: The data rate, and with it the conversion time and the noise.
        """
        self._native = _NativeAds1115(
            bus._native, address, int(mux), int(pga), int(data_rate)
        )

    def init(self) -> None:
        """Write the input, range, and data rate, and read the configuration back, the
        identity check a part with no id register allows.

        :raises PamojaError: If nothing answers, or the configuration reads back
            differently.
        """
        self._native.init()

    def sample(self) -> Ads1115Sample:
        """Run one conversion of the configured input, initializing the part first if
        :meth:`init` has not run.

        :returns: The conversion, with the range it ran at.
        :raises PamojaError: As :meth:`init`, and when the conversion never finishes.
        """
        return self._native.sample()

    def sample_input(self, mux: Ads1115Mux) -> Ads1115Sample:
        """Convert another input once, leaving the configured input as it was.

        :param mux: The input for this one conversion.
        :returns: The conversion.
        :raises PamojaError: As :meth:`sample`.
        """
        return self._native.sample_input(int(mux))

    @property
    def config(self) -> Ads1115Config:
        """The configuration the driver writes."""
        return self._native.config


class Sht3x:
    """A Sensirion SHT3x driven over an :class:`~pamoja.hal.I2cBus`, measuring on demand in
    single-shot mode.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([sht3x.sim.reporting(sht3x.ADDRESS_A, 30.0, 70.0)])
    >>> round(Sht3x(bus, sht3x.ADDRESS_A).measure().celsius, 1)
    30.0
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        bus: I2cBus,
        address: int,
        *,
        repeatability: Sht3xRepeatability = Sht3xRepeatability.HIGH,
    ) -> None:
        """Make a driver for the part at ``address`` on ``bus``.

        :param bus: The bus the part is on.
        :param address: ``sht3x.ADDRESS_A`` with ADDR low, ``sht3x.ADDRESS_B`` with it
            high.
        :param repeatability: How repeatable each measurement is, against how long it
            takes.
        """
        self._native = _NativeSht3x(
            bus._native, address, Sht3xRepeatability(repeatability).value
        )

    def init(self) -> None:
        """Soft-reset the part and read its status; a status word whose checksum holds is
        what confirms an SHT3x answers, as the part has no id register.

        :raises PamojaError: If nothing answers, or the status word fails its checksum.
        """
        self._native.init()

    def measure(self) -> Sht3xMeasurement:
        """Run one single-shot measurement, initializing the part first if :meth:`init`
        has not run.

        :returns: The checksum-checked temperature and humidity.
        :raises PamojaError: As :meth:`init`, and when a data word fails its checksum.
        """
        return self._native.measure()

    def read_status(self) -> Sht3xStatus:
        """Read the status register, which :attr:`last_status` keeps as well.

        :returns: The status.
        :raises PamojaError: If the transfer fails or the word fails its checksum.
        """
        return self._native.read_status()

    def heater_on(self) -> None:
        """Switch the plausibility-check heater on, initializing the part first if needed.

        :raises PamojaError: As :meth:`init`.
        """
        self._native.heater_on()

    def heater_off(self) -> None:
        """Switch the heater off, which is its state after any reset.

        :raises PamojaError: As :meth:`init`.
        """
        self._native.heater_off()

    @property
    def last_status(self) -> Optional[Sht3xStatus]:
        """The status register as it was last read, or ``None`` before it has been."""
        return self._native.last_status


class Scd4x:
    """A Sensirion SCD40 or SCD41 driven over an :class:`~pamoja.hal.I2cBus` in periodic
    measurement, a result every five seconds.

    >>> from pamoja.hal import I2cBus
    >>> bus = I2cBus.simulated([scd4x.sim.reporting(1450, 24.0, 55.0)])
    >>> Scd4x(bus).measure().co2_ppm
    1450
    """

    __slots__ = ("_native",)

    def __init__(self, bus: I2cBus) -> None:
        """Make a driver for the part on ``bus``, which has one address.

        :param bus: The bus the part is on.
        """
        self._native = _NativeScd4x(bus._native)

    def init(self) -> None:
        """Stop any running measurement, read the serial number, and start periodic
        measurement.

        :raises PamojaError: If nothing answers, or the serial number fails its checksum.
        """
        self._native.init()

    def measure(self) -> Scd4xMeasurement:
        """Wait for the next periodic result and read it, initializing the part first if
        :meth:`init` has not run.

        :returns: The carbon dioxide, temperature, and humidity.
        :raises PamojaError: As :meth:`init`, when no result becomes ready, and when a word
            fails its checksum.
        """
        return self._native.measure()

    def measure_single_shot(self) -> Scd4xMeasurement:
        """Run one on-demand measurement on an SCD41, which takes five seconds. The part
        must not be measuring periodically: call :meth:`stop` first, or use this in place
        of :meth:`init`.

        :returns: The measurement.
        :raises PamojaError: As :meth:`measure`.
        """
        return self._native.measure_single_shot()

    def data_ready(self) -> bool:
        """Ask the part whether a periodic result is waiting.

        :returns: Whether :meth:`measure` would read without waiting.
        :raises PamojaError: If the transfer fails or the word fails its checksum.
        """
        return self._native.data_ready()

    def stop(self) -> None:
        """Stop periodic measurement, after which the part takes its settings commands.

        :raises PamojaError: If the transfer fails.
        """
        self._native.stop()

    def start(self) -> None:
        """Start periodic measurement.

        :raises PamojaError: If the transfer fails.
        """
        self._native.start()

    def set_temperature_offset(self, milli_celsius: int) -> None:
        """Set the temperature offset that compensates the part's own warmth, until power
        is lost.

        :param milli_celsius: The offset to subtract, in millidegrees.
        :raises PamojaError: If the transfer fails.
        """
        self._native.set_temperature_offset(milli_celsius)

    def set_sensor_altitude(self, meters: int) -> None:
        """Set the altitude the part corrects its carbon dioxide reading for.

        :param meters: The altitude above sea level.
        :raises PamojaError: If the transfer fails.
        """
        self._native.set_sensor_altitude(meters)

    @property
    def serial(self) -> Optional[int]:
        """The 48-bit serial number read at initialization, or ``None`` before it."""
        return self._native.serial


class Ds18b20Thermometer:
    """A DS18B20 the Linux kernel serves as a ``w1_slave`` file.

    On a Raspberry Pi the ``w1-gpio`` overlay (``dtoverlay=w1-gpio`` in ``config.txt``)
    puts a 1-Wire bus on GPIO 4, and the kernel lists every DS18B20 it finds under
    ``/sys/bus/w1/devices`` as a directory named by its family code and serial. Reading the
    directory's ``w1_slave`` file makes the kernel run a conversion and print the
    scratchpad, which :meth:`read` decodes. The file is only text, so a test can write one
    anywhere and read it with :meth:`at`.
    """

    __slots__ = ("_native",)

    def __init__(self, native: _NativeDs18b20Thermometer) -> None:
        """Wrap a native thermometer. Use :meth:`for_serial`, :meth:`at`, or
        :meth:`discover`."""
        self._native = native

    @classmethod
    def for_serial(cls, serial: str) -> Ds18b20Thermometer:
        """Name a thermometer by the serial in its directory name.

        :param serial: The twelve hex digits after ``28-``.
        :returns: The thermometer, reading ``/sys/bus/w1/devices/28-<serial>/w1_slave``.
        """
        return cls(_NativeDs18b20Thermometer.for_serial(serial))

    @classmethod
    def at(cls, path: str) -> Ds18b20Thermometer:
        """Name a thermometer by the path of its ``w1_slave`` file.

        :param path: The file to read.
        :returns: The thermometer.
        """
        return cls(_NativeDs18b20Thermometer.at(str(path)))

    @classmethod
    def discover(cls, devices: Optional[str] = None) -> List[Ds18b20Thermometer]:
        """List every DS18B20 the kernel has found, one per ``28-`` directory.

        :param devices: The directory the kernel lists its 1-Wire devices in, or ``None``
            for ``/sys/bus/w1/devices``.
        :returns: The thermometers, sorted by directory name.
        :raises PamojaError: If the directory cannot be listed, which usually means the
            1-Wire overlay is off.
        """
        found = _NativeDs18b20Thermometer.discover(None if devices is None else str(devices))
        return [cls(native) for native in found]

    @property
    def path(self) -> str:
        """The path of the file the thermometer reads."""
        return self._native.path

    @property
    def serial(self) -> Optional[str]:
        """The serial the kernel named the thermometer's directory after.

        The twelve hex digits after ``28-``, which tell one probe from another and stay with
        the part for life, or ``None`` when the file does not sit in a DS18B20's directory, as
        one named by :meth:`at` may not.

        >>> Ds18b20Thermometer.for_serial("000005e2fdc3").serial
        '000005e2fdc3'
        >>> Ds18b20Thermometer.at("/tmp/w1_slave").serial is None
        True
        """
        return self._native.serial

    def read(self) -> Ds18b20Reading:
        """Read the file, which makes the kernel run a conversion, and decode it.

        :returns: The checksum-checked reading.
        :raises PamojaError: If the file cannot be read, because the overlay is off, the
            probe is gone, or the process may not read it; or if the kernel or this decoder
            rejects the checksum.
        """
        return self._native.read()


#: A Bosch BME280 temperature, pressure, and humidity sensor.
bme280 = _Bme280()

#: A Maxim DS18B20 1-Wire thermometer.
ds18b20 = _Ds18b20()

#: A TI INA219 current, voltage, and power monitor.
ina219 = _Ina219()

#: A TI ADS1115 16-bit analog-to-digital converter.
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
