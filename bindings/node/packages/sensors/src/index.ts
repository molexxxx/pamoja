/**
 * Ergonomic facade over the generated sensor-driver binding.
 *
 * These are eleven parts a field node is likely to have wired to it. Each has its
 * decode half, turning the register bytes a bus driver read into the physical reading
 * the manufacturer's datasheet says they mean, and each I2C part has a driver, which
 * runs the datasheet's whole conversation over an `I2cBus` from `@pamoja/hal`, and a
 * simulated part that answers it with nothing plugged in. The DS18B20 is read through
 * the files the Linux kernel's 1-Wire driver serves.
 *
 * @packageDocumentation
 */

import type { CommandPart, I2cBus, I2cPart, WordPart } from '@pamoja/hal'

import {
  Ads1115,
  ads1115ConversionMicros,
  ads1115SimPart,
  ads1115SimReporting,
  type Ads1115Sample,
  type Ads1115Settings,
  Bmp280,
  bmp280SimBurst,
  bmp280SimBurstFor,
  bmp280SimCalibration,
  bmp280SimPart,
  bmp280SimReporting,
  type Bmp280Settings,
  Ds18b20Thermometer,
  ds18b20ParseW1Slave,
  Hdc1080,
  hdc1080SimPart,
  hdc1080SimReporting,
  type Hdc1080Settings,
  Ina219,
  ina219AdcConversionMicros,
  ina219ConfigBits,
  ina219ConfigFromBits,
  ina219ConversionMicros,
  ina219GainRangeMillivolts,
  ina219SimPart,
  ina219SimReporting,
  type Ina219Configuration as Ina219Config,
  type Ina219Reading,
  type Ina219Settings,
  Ina226,
  ina226SimPart,
  ina226SimReporting,
  type Ina226Reading,
  type Ina226Settings,
  Opt3001,
  opt3001SimPart,
  opt3001SimReporting,
  type Opt3001Reading,
  type Opt3001Settings,
  Scd4x,
  scd4xSimPart,
  scd4xSimReporting,
  Sht3x,
  sht3xSimPart,
  sht3xSimReporting,
  type Sht3xSettings,
  Tmp117,
  tmp117SimPart,
  tmp117SimReporting,
  type Tmp117Alerts,
  type Tmp117Reading,
  type Tmp117Settings,
} from '@pamoja/native'

import {
  ads1115ConfigBits,
  ads1115ConfigFromBits,
  ads1115FullScaleMicrovolts,
  ads1115SamplesPerSecond,
  ads1115ToNanovolts,
  ads1115ToVolts,
  type Ads1115Config,
  Bme280,
  Bme280Calibration,
  bme280ConfigBits,
  bme280ConfigFromBits,
  bme280CtrlHumBits,
  bme280CtrlHumFromBits,
  bme280CtrlMeasBits,
  bme280CtrlMeasFromBits,
  bme280FilterCoefficient,
  bme280ImageUpdating,
  bme280MaxMeasurementMicros,
  bme280Measuring,
  bme280OversamplingFactor,
  bme280SimBurst,
  bme280SimBurstFor,
  bme280SimCalibration,
  bme280SimCalibrationHumidity,
  bme280SimPart,
  bme280SimReporting,
  bme280StandbyMicros,
  bme280TypicalMeasurementMicros,
  type Bme280Config,
  type Bme280CtrlMeas,
  type Bme280Measurement,
  type Bme280Settings,
  ds18b20BuildScratchpad,
  ds18b20Celsius,
  ds18b20ConfigByte,
  ds18b20Crc8,
  ds18b20MaxConversionMicros,
  ds18b20MicroCelsius,
  ds18b20ParseScratchpad,
  ds18b20ResolutionBits,
  ds18b20StepMicroCelsius,
  type Ds18b20Reading,
  ina219BusRegister,
  ina219CurrentRegister,
  ina219PowerRegister,
  ina219BusMillivolts,
  ina219Calibration,
  ina219ConversionReady,
  ina219CurrentMicroamps,
  ina219MathOverflow,
  ina219MinimumCurrentLsbMicroamps,
  ina219PowerMicrowatts,
  ina219ShuntMicrovolts,
  ina219ShuntRegister,
  Bmp280Calibration,
  bmp280ConfigBits,
  bmp280ConfigFromBits,
  bmp280CtrlMeasBits,
  bmp280CtrlMeasFromBits,
  bmp280ImageUpdating,
  bmp280MeasurementBytes,
  bmp280Measuring,
  bmp280OversamplingFactor,
  bmp280ParseMeasurement,
  bmp280PressureSkipped,
  bmp280StandbyMicros,
  bmp280TemperatureSkipped,
  type Bmp280Config,
  type Bmp280CtrlMeas,
  type Bmp280Measurement as Bmp280RawMeasurement,
  type Bmp280Reading,
  sht3xCelsius,
  sht3xCrc,
  sht3xFahrenheit,
  sht3xHumidityRawFromMilliPercent,
  sht3xHumidityRawFromRelativeHumidity,
  sht3xIntervalMicros,
  sht3xMaxMeasurementMicros,
  sht3xMeasurementBytes,
  sht3xMilliCelsius,
  sht3xMilliFahrenheit,
  sht3xMilliPercent,
  sht3xParseMeasurement,
  sht3xParseStatus,
  sht3xPeriodic,
  sht3xRelativeHumidity,
  sht3xSingleShot,
  sht3xStatusBytes,
  sht3xStatusFromBits,
  sht3xTemperatureRawFromCelsius,
  sht3xTemperatureRawFromMilliCelsius,
  sht3xTemperatureRawFromMilliFahrenheit,
  sht3xTypicalMeasurementMicros,
  sht3xWord,
  sht3xWordBytes,
  type Sht3xMeasurement,
  type Sht3xRate as Sht3xRateName,
  type Sht3xRepeatability as Sht3xRepeatabilityName,
  type Sht3xStatus,
  scd4xAllowedDuringMeasurement,
  scd4xAmbientPressurePascals,
  scd4xAmbientPressureWord,
  scd4xAutomaticSelfCalibrationEnabled,
  scd4xAutomaticSelfCalibrationWord,
  scd4xCelsius,
  scd4xCommandFrame,
  scd4xCrc,
  scd4xDataReady,
  scd4xForcedRecalibrationCorrectionPpm,
  scd4xForcedRecalibrationWord,
  scd4xHumidityMilliPercent,
  scd4xHumidityRaw,
  scd4xMaxDurationMs,
  scd4xMeasurementBytes,
  scd4xMeasurementFromPhysical,
  scd4xMilliCelsius,
  scd4xParseMeasurement,
  scd4xRelativeHumidityPercent,
  scd4xSelfTestPassed,
  scd4xSerialNumber,
  scd4xSerialNumberFrame,
  scd4xTemperatureOffsetMilliCelsius,
  scd4xTemperatureOffsetWord,
  scd4xTemperatureRaw,
  scd4xWord,
  scd4xWordFrame,
  scd4xWriteFrame,
  type Scd4xMeasurement,
  tmp117Celsius,
  tmp117ConfigBits,
  tmp117ConfigFromBits,
  tmp117AveragingMicros as tmp117ConversionMicros,
  tmp117AveragingConversions as tmp117Conversions,
  tmp117CycleMicros,
  tmp117DataReady,
  tmp117DeviceId,
  tmp117EepromBusy,
  tmp117EepromUnlockBusy,
  tmp117HighAlert,
  tmp117LowAlert,
  tmp117MicroCelsius,
  tmp117NanoCelsius,
  tmp117CycleNominalMicros as tmp117NominalMicros,
  tmp117RawFromCelsius,
  tmp117RawFromMicroCelsius,
  tmp117Revision,
  tmp117TemperatureBytes,
  tmp117TemperatureFromBytes,
  type Tmp117Configuration as Tmp117Config,
  hdc1080Celsius,
  hdc1080ConfigurationFromRegister as hdc1080ConfigFromRegister,
  hdc1080ConfigurationToRegister as hdc1080ConfigToRegister,
  hdc1080ConversionTimeMicros,
  hdc1080HumidityRegister,
  hdc1080MeasurementBytes,
  hdc1080MeasurementFromPhysical,
  hdc1080MilliCelsius,
  hdc1080MilliPercent,
  hdc1080ParseMeasurement,
  hdc1080RelativeHumidity,
  hdc1080SerialId,
  hdc1080SerialIdRegisters,
  hdc1080TemperatureRegister,
  type Hdc1080Configuration as Hdc1080Config,
  type Hdc1080Measurement,
  opt3001ConfigBits,
  opt3001ConfigFromBits,
  opt3001FullScaleMilliLux,
  opt3001IsAutomaticRange,
  opt3001LsbMilliLux,
  opt3001Lux,
  opt3001MilliLux,
  opt3001RawFromMilliLux,
  opt3001WordFromBytes,
  opt3001WordToBytes,
  type Opt3001Configuration as Opt3001Config,
  ina226ActiveAlertFunction,
  ina226Address,
  ina226AveragingSamples,
  ina226BusMicrovolts,
  ina226BusRegister,
  ina226BusVolts,
  ina226Calibration,
  ina226ConfigFromRegister,
  ina226ConfigToRegister,
  ina226ConversionMicros,
  ina226CurrentAmps,
  ina226CurrentMicroamps,
  ina226CurrentRegister,
  ina226CurrentRegisterFromShunt,
  ina226DieId,
  ina226Identify,
  ina226MaskEnableFromRegister,
  ina226MaskEnableToRegister,
  ina226MinimumCurrentLsbMicroamps,
  ina226IsContinuous as ina226ModeIsContinuous,
  ina226MeasuresBus as ina226ModeMeasuresBus,
  ina226MeasuresShunt as ina226ModeMeasuresShunt,
  ina226PowerMicrowatts,
  ina226PowerRegister,
  ina226PowerRegisterFromCurrent,
  ina226PowerWatts,
  ina226ShuntMillivolts,
  ina226ShuntNanovolts,
  ina226ShuntRegister,
  ina226UpdateMicros as ina226UpdateMicroseconds,
  type Ina226AlertFunction as Ina226AlertFunctionName,
  type Ina226Configuration as Ina226Config,
  type Ina226DieId as Ina226DieIdFields,
  type Ina226MaskEnable,
} from '@pamoja/native'

export {
  Bme280Calibration,
  Bmp280Calibration,
  type Ads1115Config,
  type Ads1115Sample,
  type Ads1115Settings,
  type Bme280Config,
  type Bme280CtrlMeas,
  type Bme280Measurement,
  type Bme280Settings,
  type Bmp280Config,
  type Bmp280CtrlMeas,
  type Bmp280RawMeasurement,
  type Bmp280Reading,
  type Bmp280Settings,
  type Ds18b20Reading,
  type Hdc1080Config,
  type Hdc1080Measurement,
  type Hdc1080Settings,
  type Ina219Config,
  type Ina219Reading,
  type Ina219Settings,
  type Ina226Config,
  type Ina226DieIdFields,
  type Ina226MaskEnable,
  type Ina226Reading,
  type Ina226Settings,
  type Opt3001Config,
  type Opt3001Reading,
  type Opt3001Settings,
  type Scd4xMeasurement,
  type Sht3xMeasurement,
  type Sht3xSettings,
  type Sht3xStatus,
  type Tmp117Alerts,
  type Tmp117Config,
  type Tmp117Reading,
  type Tmp117Settings,
}

/**
 * A Bosch BMP280 driven over an {@link I2cBus}, measuring on demand in forced mode.
 *
 * `new Bmp280(bus, address, settings?)` sends nothing. `init()` resets the part, checks it is
 * a BMP280, reads its trimming, and writes the settings, leaving it asleep; `measure()` runs
 * one forced measurement and resolves with the compensated reading. `coefficients` is the
 * trimming read at initialization, or `null` before it.
 */
export { Bmp280 }

/**
 * A Texas Instruments TMP117 driven over an {@link I2cBus}, converting on demand in one-shot
 * mode and powered down between conversions.
 *
 * `init()` checks the device id, waits for the EEPROM, and writes the averaging;
 * `measure()` runs one conversion and resolves with the temperature. `setAlertLimits(high,
 * low)` writes the limits and `alerts()` resolves with the flags, including any the driver's
 * own reads consumed. `siliconRevision` is read at initialization.
 */
export { Tmp117 }

/**
 * A Texas Instruments OPT3001 driven over an {@link I2cBus}, measuring illuminance on demand
 * in single-shot mode.
 *
 * `init()` checks both id registers and writes the settings; `measure()` runs one conversion,
 * 100 or 800 ms long, and resolves with the lux. `setLimits(low, high)` programs the
 * interrupt window in millilux.
 */
export { Opt3001 }

/**
 * A Texas Instruments HDC1080 driven over an {@link I2cBus}, measuring temperature then
 * humidity from one trigger at its one address.
 *
 * `init()` checks both id registers and writes the resolutions; `measure()` triggers an
 * acquisition, waits both conversion times, and resolves with both channels. `heater(on)`
 * switches the on-die heater, which runs only during acquisitions.
 */
export { Hdc1080 }

/**
 * A Texas Instruments INA219 driven over an {@link I2cBus}: shunt and bus voltage, current,
 * and power on demand.
 *
 * `init()` resets the part, writes the configuration and the calibration for the shunt and
 * the largest current, and reads the calibration back, the identity check a part with no id
 * register allows. `measure()` triggers one conversion of each and resolves with every
 * register and what it means.
 */
export { Ina219 }

/**
 * A Texas Instruments INA226 driven over an {@link I2cBus}: shunt and bus voltage, current,
 * and power on demand.
 *
 * `init()` resets the part, checks its id registers, and programs the configuration and
 * calibration; `measure()` triggers one conversion and resolves with every result.
 * `setAlert(mask, limit)` programs the alert pin, and `identity` is the die id read at
 * initialization.
 */
export { Ina226 }

/**
 * A Texas Instruments ADS1115 driven over an {@link I2cBus}, converting one input on demand.
 *
 * `init()` writes the input, range, and data rate and reads them back; `sample()` runs one
 * conversion and resolves with the count, the range it ran at, and the voltage.
 * `sampleInput(mux)` converts another input once and leaves the configured one as it was.
 */
export { Ads1115 }

/**
 * A Sensirion SHT3x driven over an {@link I2cBus}, measuring on demand in single-shot mode.
 *
 * `init()` soft-resets the part and reads its status, whose checksum is what confirms an
 * SHT3x answers; `measure()` resolves with a checksum-checked reading. `readStatus()`,
 * `heaterOn()`, and `heaterOff()` do what they say, and `lastStatus` keeps the last status
 * read.
 */
export { Sht3x }

/**
 * A Sensirion SCD40 or SCD41 driven over an {@link I2cBus} in periodic measurement.
 *
 * `init()` stops any measurement a previous run left going, reads the serial number, and
 * starts periodic measurement; `measure()` waits for the next result, one every five seconds.
 * `measureSingleShot()` runs one on-demand measurement on an SCD41, and `dataReady()`,
 * `stop()`, `start()`, `setTemperatureOffset()`, and `setSensorAltitude()` are the part's
 * own commands.
 */
export { Scd4x }

/**
 * A DS18B20 the Linux kernel serves as a `w1_slave` file.
 *
 * `Ds18b20Thermometer.discover()` lists every probe under `/sys/bus/w1/devices` once the
 * `w1-gpio` overlay is on; `forSerial(serial)` and `at(path)` name one. `read()` makes the
 * kernel run a conversion and resolves with the checksum-checked reading.
 */
export { Ds18b20Thermometer }

/**
 * A Bosch BME280 driven over an {@link I2cBus}, measuring on demand in forced mode.
 *
 * `new Bme280(bus, address, settings?)` sends nothing. `init()` resets the part, checks it is
 * a BME280, reads its calibration, and writes the settings in the order the datasheet
 * requires, leaving the part asleep; `measure()` runs one forced measurement, waits the
 * datasheet's maximum time for the settings in use, and resolves with the compensated
 * reading, initializing first if `init()` has not run. Both run on a worker thread and reject
 * when nothing answers at the address, another part does, or the part never finishes.
 */
export { Bme280 }

/** A Bosch BME280 temperature, pressure, and humidity sensor. */
export const bme280 = {
  /** The address a BME280 answers on with its SDO pin low. */
  addressPrimary: 0x76,
  /** The address it answers on with SDO high. */
  addressSecondary: 0x77,
  /** The value its chip-ID register reads, which tells it from a BMP280. */
  chipId: 0x60,
  /** The word written to the reset register to restart the part. */
  resetWord: 0xb6,
  /** How long the part takes to start after a reset, in microseconds. */
  startupMicros: 2_000,
  /** How many bytes the temperature and pressure calibration block holds. */
  calibrationTempPressLength: 26,
  /** How many bytes the humidity calibration block holds. */
  calibrationHumidityLength: 7,
  /** How many bytes one measurement burst holds. */
  dataLength: 8,
  /** The registers a driver reads and writes. */
  register: {
    /** The chip-ID register. */
    chipId: 0xd0,
    /** The reset register. */
    reset: 0xe0,
    /** The first of the 26 temperature and pressure calibration bytes. */
    calibTempPress: 0x88,
    /** The first of the 7 humidity calibration bytes. */
    calibHumidity: 0xe1,
    /** The humidity control register, `ctrl_hum`. */
    ctrlHum: 0xf2,
    /** The status register. */
    status: 0xf3,
    /** The measurement control register, `ctrl_meas`. */
    ctrlMeas: 0xf4,
    /** The configuration register, `config`. */
    config: 0xf5,
    /** The first of the 8 data bytes a burst read covers. */
    data: 0xf7,
  },
  /** The oversampling codes: how many samples each measurement averages. */
  oversampling: {
    /** The measurement is skipped. */
    skipped: 0,
    /** One sample. */
    x1: 1,
    /** Two samples. */
    x2: 2,
    /** Four samples. */
    x4: 3,
    /** Eight samples. */
    x8: 4,
    /** Sixteen samples. */
    x16: 5,
  },
  /** The power mode codes. */
  mode: {
    /** No measurements; the power-on default. */
    sleep: 0,
    /** One measurement, then back to sleep. */
    forced: 1,
    /** Measurements on a cycle, a standby period apart. */
    normal: 3,
  },
  /** The IIR filter codes, which smooth pressure and temperature across measurements. */
  filter: {
    /** No filtering. */
    off: 0,
    /** Coefficient 2. */
    x2: 1,
    /** Coefficient 4. */
    x4: 2,
    /** Coefficient 8. */
    x8: 3,
    /** Coefficient 16. */
    x16: 4,
  },
  /** The normal-mode standby codes, by the period each selects. */
  standby: {
    /** 0.5 ms. */
    ms0_5: 0,
    /** 62.5 ms. */
    ms62_5: 1,
    /** 125 ms. */
    ms125: 2,
    /** 250 ms. */
    ms250: 3,
    /** 500 ms. */
    ms500: 4,
    /** 1000 ms. */
    ms1000: 5,
    /** 10 ms. */
    ms10: 6,
    /** 20 ms. */
    ms20: 7,
  },

  /**
   * Reads the factory calibration out of the registers, once at start-up.
   *
   * @param tempPress - The 26-byte temperature and pressure calibration block.
   * @param humidity - The 7-byte humidity calibration block.
   * @returns The calibration, to reuse for every measurement.
   * @throws If either block is the wrong length.
   */
  calibration(tempPress: Uint8Array, humidity: Uint8Array): Bme280Calibration {
    return new Bme280Calibration(Buffer.from(tempPress), Buffer.from(humidity))
  },

  /**
   * Reports whether a status register says a conversion is running.
   *
   * @param status - The status register.
   * @returns Whether a measurement is in progress.
   */
  measuring(status: number): boolean {
    return bme280Measuring(status)
  },

  /**
   * Reports whether a status register says the calibration is being copied.
   *
   * @param status - The status register.
   * @returns Whether the calibration image is still loading.
   */
  imageUpdating(status: number): boolean {
    return bme280ImageUpdating(status)
  },

  /**
   * Assembles the `ctrl_meas` register value.
   *
   * @param ctrl - The oversampling codes and the power mode.
   * @returns The register value to write.
   */
  ctrlMeasBits(ctrl: Bme280CtrlMeas): number {
    return bme280CtrlMeasBits(ctrl)
  },

  /**
   * Parses a `ctrl_meas` register value.
   *
   * @param bits - The register value, as read from the part.
   * @returns The oversampling codes and the power mode.
   */
  ctrlMeasFromBits(bits: number): Bme280CtrlMeas {
    return bme280CtrlMeasFromBits(bits)
  },

  /**
   * Assembles the `ctrl_hum` register value, which takes effect only after the next
   * `ctrl_meas` write.
   *
   * @param humidity - The humidity oversampling code.
   * @returns The register value to write.
   */
  ctrlHumBits(humidity: number): number {
    return bme280CtrlHumBits(humidity)
  },

  /**
   * Parses a `ctrl_hum` register value.
   *
   * @param bits - The register value, as read from the part.
   * @returns The humidity oversampling code.
   */
  ctrlHumFromBits(bits: number): number {
    return bme280CtrlHumFromBits(bits)
  },

  /**
   * Assembles the `config` register value.
   *
   * @param config - The standby period, filter, and interface settings.
   * @returns The register value to write.
   */
  configBits(config: Bme280Config): number {
    return bme280ConfigBits(config)
  },

  /**
   * Parses a `config` register value.
   *
   * @param bits - The register value, as read from the part.
   * @returns The standby period, filter, and interface settings.
   */
  configFromBits(bits: number): Bme280Config {
    return bme280ConfigFromBits(bits)
  },

  /**
   * Returns how many samples an oversampling code averages.
   *
   * @param code - The oversampling code.
   * @returns The factor, or 0 when the code skips the measurement.
   */
  oversamplingFactor(code: number): number {
    return bme280OversamplingFactor(code)
  },

  /**
   * Returns the standby period a standby code selects in normal mode.
   *
   * @param code - The standby code.
   * @returns The period in microseconds.
   */
  standbyMicros(code: number): number {
    return bme280StandbyMicros(code)
  },

  /**
   * Returns the IIR coefficient a filter code selects.
   *
   * @param code - The filter code.
   * @returns The coefficient, or 0 when the filter is off.
   */
  filterCoefficient(code: number): number {
    return bme280FilterCoefficient(code)
  },

  /**
   * Returns the longest one measurement can take, which is how long a driver waits after
   * forcing one.
   *
   * @param temperature - The temperature oversampling code.
   * @param pressure - The pressure oversampling code.
   * @param humidity - The humidity oversampling code.
   * @returns The datasheet's maximum in microseconds.
   */
  maxMeasurementMicros(temperature: number, pressure: number, humidity: number): number {
    return bme280MaxMeasurementMicros(temperature, pressure, humidity)
  },

  /**
   * Returns the typical time one measurement takes.
   *
   * @param temperature - The temperature oversampling code.
   * @param pressure - The pressure oversampling code.
   * @param humidity - The humidity oversampling code.
   * @returns The datasheet's typical time in microseconds.
   */
  typicalMeasurementMicros(temperature: number, pressure: number, humidity: number): number {
    return bme280TypicalMeasurementMicros(temperature, pressure, humidity)
  },

  /** A BME280 that is not there, for a bus with nothing plugged in. */
  sim: {
    /** The status a simulated part reports when it is neither measuring nor loading. */
    statusIdle: 0x00,

    /**
     * A part holding a real BME280's calibration and one measurement it took, which
     * compensate to 20.44 C, 848.05 hPa, and 44.65 %.
     *
     * @param address - The address it answers to.
     * @returns The part, to put on a simulated bus.
     */
    part(address: number): I2cPart {
      return bme280SimPart(address)
    },

    /**
     * A part that reads what it is asked to, to within what its converter can represent.
     *
     * @param address - The address it answers to.
     * @param celsius - The temperature it reports.
     * @param hectopascals - The pressure it reports.
     * @param relativeHumidity - The humidity it reports, as a percentage.
     * @returns The part, to put on a simulated bus.
     */
    reporting(
      address: number,
      celsius: number,
      hectopascals: number,
      relativeHumidity: number,
    ): I2cPart {
      return bme280SimReporting(address, celsius, hectopascals, relativeHumidity)
    },

    /** The 26-byte temperature and pressure calibration block a simulated part holds. */
    calibration(): Buffer {
      return bme280SimCalibration()
    },

    /** The 7-byte humidity calibration block a simulated part holds. */
    calibrationHumidity(): Buffer {
      return bme280SimCalibrationHumidity()
    },

    /** The eight data registers a simulated part holds: one measurement a real part took. */
    burst(): Buffer {
      return bme280SimBurst()
    },

    /**
     * The eight data registers that compensate to a reading against the simulated
     * calibration.
     *
     * @param celsius - The temperature.
     * @param hectopascals - The pressure.
     * @param relativeHumidity - The humidity, as a percentage.
     * @returns The bytes a burst read would return.
     */
    burstFor(celsius: number, hectopascals: number, relativeHumidity: number): Buffer {
      return bme280SimBurstFor(celsius, hectopascals, relativeHumidity)
    },
  },
}

/** A Maxim DS18B20 1-Wire thermometer. */
export const ds18b20 = {
  /** The 1-Wire family code that identifies a DS18B20 on the bus. */
  familyCode: 0x28,

  /**
   * Parses and CRC-checks a nine-byte scratchpad.
   *
   * @param bytes - The scratchpad as the device sent it, the ninth byte its CRC.
   * @returns The decoded reading.
   * @throws If the CRC does not match, which means the read was corrupted on the
   * bus and should be repeated.
   */
  parseScratchpad(bytes: Uint8Array): Ds18b20Reading {
    return ds18b20ParseScratchpad(Buffer.from(bytes))
  },

  /**
   * Builds the nine bytes a part in the given state puts on the bus.
   *
   * This is the inverse of {@link parseScratchpad}, so a node can be written and
   * tested against what a thermometer sends without one attached.
   *
   * @param celsius - The temperature the part is reading.
   * @param resolutionBits - The resolution it is configured for, 9 to 12.
   * @param alarmHigh - The high alarm threshold in whole degrees Celsius.
   * @param alarmLow - The low alarm threshold in whole degrees Celsius.
   * @returns The nine scratchpad bytes in transmission order, CRC last.
   * @throws If the resolution is not 9, 10, 11, or 12 bits.
   */
  buildScratchpad(
    celsius: number,
    resolutionBits: number,
    alarmHigh: number,
    alarmLow: number,
  ): Buffer {
    return ds18b20BuildScratchpad(celsius, resolutionBits, alarmHigh, alarmLow)
  },

  /**
   * Computes the Maxim CRC-8 a 1-Wire device checks its own bytes with.
   *
   * @param data - The bytes the checksum covers.
   * @returns The checksum.
   */
  crc8(data: Uint8Array): number {
    return ds18b20Crc8(Buffer.from(data))
  },

  /**
   * Converts a raw temperature register to micro-degrees Celsius.
   *
   * @param raw - The 16-bit two's-complement register.
   * @returns The temperature, exact in integer arithmetic.
   */
  microCelsius(raw: number): number {
    return ds18b20MicroCelsius(raw)
  },

  /**
   * Converts a raw temperature register to degrees Celsius.
   *
   * @param raw - The 16-bit two's-complement register.
   * @returns The temperature.
   */
  celsius(raw: number): number {
    return ds18b20Celsius(raw)
  },

  /**
   * Returns the configuration byte that selects a resolution.
   *
   * @param bits - The resolution in bits: 9, 10, 11, or 12.
   * @returns The byte to write to the configuration register.
   * @throws If the resolution is not one the part offers.
   */
  configByte(bits: number): number {
    return ds18b20ConfigByte(bits)
  },

  /**
   * Returns the resolution a configuration byte selects.
   *
   * @param configByte - The byte read from the configuration register.
   * @returns The resolution in bits.
   */
  resolutionBits(configByte: number): number {
    return ds18b20ResolutionBits(configByte)
  },

  /**
   * Returns the temperature step a resolution resolves.
   *
   * @param bits - The resolution in bits.
   * @returns The step in micro-degrees Celsius.
   * @throws If the resolution is not one the part offers.
   */
  stepMicroCelsius(bits: number): number {
    return ds18b20StepMicroCelsius(bits)
  },

  /**
   * Returns how long a conversion may take at a resolution.
   *
   * @param bits - The resolution in bits.
   * @returns The datasheet's worst case, in microseconds.
   * @throws If the resolution is not one the part offers.
   */
  maxConversionMicros(bits: number): number {
    return ds18b20MaxConversionMicros(bits)
  },

  /**
   * Decodes the text the Linux kernel's `w1_therm` driver serves for a thermometer: the
   * scratchpad in hex with the kernel's checksum verdict, then the temperature.
   *
   * @param text - The `w1_slave` file's contents.
   * @returns The reading, its CRC checked here as well.
   * @throws If the kernel or this decoder rejects the CRC, or the text is not in the driver's
   *   format.
   */
  parseW1Slave(text: string): Ds18b20Reading {
    return ds18b20ParseW1Slave(text)
  },
}

/** A TI INA219 current, voltage, and power monitor. */
export const ina219 = {
  /** The address with A1 and A0 tied to ground; the pins add to it. */
  baseAddress: 0x40,
  /** The configuration register's power-on value. */
  configReset: 0x399f,
  /** The registers a driver reads and writes. */
  register: {
    /** Range, gain, converter settings, and mode. */
    configuration: 0x00,
    /** The shunt voltage, 10 uV per count. */
    shuntVoltage: 0x01,
    /** The bus voltage in bits 15:3, 4 mV per count, with two flags below. */
    busVoltage: 0x02,
    /** The power, scaled by the calibration. */
    power: 0x03,
    /** The current, scaled by the calibration. */
    current: 0x04,
    /** The calibration, which sets the current and power scale. */
    calibration: 0x05,
  },
  /** The bus-voltage range codes. */
  busRange: {
    /** 0 to 16 V. */
    v16: 0,
    /** 0 to 32 V, the reset setting. */
    v32: 1,
  },
  /** The shunt gain codes, by the shunt-voltage range each gives. */
  gain: {
    /** Gain 1, 40 mV either side of zero. */
    div1: 0,
    /** Gain 1/2, 80 mV. */
    div2: 1,
    /** Gain 1/4, 160 mV. */
    div4: 2,
    /** Gain 1/8, 320 mV, the reset setting. */
    div8: 3,
  },
  /** The converter codes: a resolution, or samples averaged at 12 bits. */
  adc: {
    /** 9 bits, 84 us. */
    bits9: 0b0000,
    /** 10 bits, 148 us. */
    bits10: 0b0001,
    /** 11 bits, 276 us. */
    bits11: 0b0010,
    /** 12 bits, 532 us, the reset setting. */
    bits12: 0b0011,
    /** 2 samples averaged, 1.06 ms. */
    samples2: 0b1001,
    /** 4 samples averaged, 2.13 ms. */
    samples4: 0b1010,
    /** 8 samples averaged, 4.26 ms. */
    samples8: 0b1011,
    /** 16 samples averaged, 8.51 ms. */
    samples16: 0b1100,
    /** 32 samples averaged, 17.02 ms. */
    samples32: 0b1101,
    /** 64 samples averaged, 34.05 ms. */
    samples64: 0b1110,
    /** 128 samples averaged, 68.10 ms. */
    samples128: 0b1111,
  },
  /** The operating-mode codes. */
  mode: {
    /** No conversions, lowest power. */
    powerDown: 0,
    /** One shunt conversion. */
    shuntTriggered: 1,
    /** One bus conversion. */
    busTriggered: 2,
    /** One shunt and one bus conversion. */
    shuntAndBusTriggered: 3,
    /** The converter disabled. */
    adcOff: 4,
    /** Shunt conversions back to back. */
    shuntContinuous: 5,
    /** Bus conversions back to back. */
    busContinuous: 6,
    /** Shunt and bus conversions back to back, the reset setting. */
    shuntAndBusContinuous: 7,
  },

  /**
   * Assembles the configuration register value.
   *
   * @param config - The range, gain, converter, and mode codes.
   * @returns The register value to write.
   */
  configBits(config: Ina219Config): number {
    return ina219ConfigBits(config)
  },

  /**
   * Parses a configuration register value.
   *
   * @param bits - The register value, as read from the part.
   * @returns The settings. Every value decodes.
   */
  configFromBits(bits: number): Ina219Config {
    return ina219ConfigFromBits(bits)
  },

  /**
   * Returns how long one conversion cycle takes: the shunt and bus conversions the mode runs,
   * one after the other.
   *
   * @param config - The settings.
   * @returns The time in microseconds.
   */
  conversionMicros(config: Ina219Config): number {
    return ina219ConversionMicros(config)
  },

  /**
   * Returns how long one conversion takes at a converter code.
   *
   * @param code - One of the {@link ina219.adc} codes.
   * @returns The time in microseconds, from the datasheet's table.
   */
  adcConversionMicros(code: number): number {
    return ina219AdcConversionMicros(code)
  },

  /**
   * Returns the shunt-voltage range a gain code selects.
   *
   * @param code - One of the {@link ina219.gain} codes.
   * @returns The range in millivolts either side of zero.
   */
  gainRangeMillivolts(code: number): number {
    return ina219GainRangeMillivolts(code)
  },

  /**
   * Computes the calibration register for a shunt and current resolution.
   *
   * @param currentLsbMicroamps - The microamps per count the current register
   * should carry.
   * @param shuntMilliohms - The shunt resistor value.
   * @returns The register value to write.
   */
  calibration(currentLsbMicroamps: number, shuntMilliohms: number): number {
    return ina219Calibration(currentLsbMicroamps, shuntMilliohms)
  },

  /**
   * Returns the smallest current resolution that still covers a maximum.
   *
   * @param maxExpectedMicroamps - The largest current the application measures.
   * @returns The minimum current LSB in microamps.
   */
  minimumCurrentLsbMicroamps(maxExpectedMicroamps: number): number {
    return ina219MinimumCurrentLsbMicroamps(maxExpectedMicroamps)
  },

  /**
   * Converts a raw shunt-voltage register to microvolts.
   *
   * @param raw - The signed register value.
   * @returns The shunt voltage.
   */
  /**
   * Builds the shunt-voltage register a monitor reports for a shunt voltage.
   *
   * The inverse of {@link shuntMicrovolts}, so a node can be written and tested
   * against what a monitor sends without one attached.
   *
   * @param microvolts - The shunt voltage in microvolts.
   * @returns The signed register value, at 10 uV per count.
   */
  shuntRegister(microvolts: number): number {
    return ina219ShuntRegister(microvolts)
  },

  /**
   * Builds the bus-voltage register a monitor reports for a bus voltage.
   *
   * @param millivolts - The bus voltage in millivolts.
   * @returns The register value, with the conversion-ready flag set.
   */
  busRegister(millivolts: number): number {
    return ina219BusRegister(millivolts)
  },

  /**
   * Builds the current register a monitor reports for a current.
   *
   * @param microamps - The current in microamps.
   * @param currentLsbMicroamps - The current LSB the calibration was set for.
   * @returns The signed register value.
   */
  currentRegister(microamps: number, currentLsbMicroamps: number): number {
    return ina219CurrentRegister(microamps, currentLsbMicroamps)
  },

  /**
   * Builds the power register a monitor reports for a power.
   *
   * @param microwatts - The power in microwatts.
   * @param currentLsbMicroamps - The current LSB the calibration was set for.
   * @returns The register value.
   */
  powerRegister(microwatts: number, currentLsbMicroamps: number): number {
    return ina219PowerRegister(microwatts, currentLsbMicroamps)
  },

  shuntMicrovolts(raw: number): number {
    return ina219ShuntMicrovolts(raw)
  },

  /**
   * Converts a raw bus-voltage register to millivolts.
   *
   * @param raw - The register value.
   * @returns The bus voltage.
   */
  busMillivolts(raw: number): number {
    return ina219BusMillivolts(raw)
  },

  /**
   * Reports whether a bus-voltage register says a conversion is ready.
   *
   * @param raw - The register value.
   * @returns Whether the conversion-ready flag is set.
   */
  conversionReady(raw: number): boolean {
    return ina219ConversionReady(raw)
  },

  /**
   * Reports whether a bus-voltage register flags a math overflow.
   *
   * @param raw - The register value.
   * @returns Whether the current and power readings are meaningless, which means
   * the calibration needs revisiting.
   */
  mathOverflow(raw: number): boolean {
    return ina219MathOverflow(raw)
  },

  /**
   * Converts a raw current register to microamps.
   *
   * @param raw - The signed register value.
   * @param currentLsbMicroamps - The resolution the calibration selected.
   * @returns The current.
   */
  currentMicroamps(raw: number, currentLsbMicroamps: number): number {
    return ina219CurrentMicroamps(raw, currentLsbMicroamps)
  },

  /**
   * Converts a raw power register to microwatts.
   *
   * @param raw - The register value.
   * @param currentLsbMicroamps - The resolution the calibration selected.
   * @returns The power. The power LSB is fixed at twenty times the current LSB.
   */
  powerMicrowatts(raw: number, currentLsbMicroamps: number): number {
    return ina219PowerMicrowatts(raw, currentLsbMicroamps)
  },

  /**
   * An INA219 that is not there, for a bus with nothing plugged in. A monitor's current and
   * power registers count in steps the calibration sets, so `reporting` takes the same shunt
   * and largest current a driver is given.
   */
  sim: {
    /** The shunt `part` sits across, in milliohms: the common breakout's. */
    shuntMilliohms: 100,
    /** The largest current `part` is sized for, in microamps. */
    maxMicroamps: 3_200_000,
    /** The bus voltage `part` reports, in millivolts. */
    busMillivolts: 12_000,
    /** The current `part` reports, in microamps. */
    microamps: 500_000,

    /**
     * A part carrying 500 mA at 12 V through the shunt a driver starts with.
     *
     * @param address - The address it answers to.
     * @returns The part, to put on a simulated bus.
     */
    part(address: number): WordPart {
      return ina219SimPart(address)
    },

    /**
     * A part that reads what it is asked to, on the steps its registers count in: 4 mV of
     * bus, 10 uV of shunt, and the calibration's current step.
     *
     * @param address - The address it answers to.
     * @param shuntMilliohms - The shunt, as the driver is given it.
     * @param maxMicroamps - The largest current, as the driver is given it.
     * @param busMillivolts - The bus voltage it reports.
     * @param microamps - The current it reports; negative flows the other way.
     * @returns The part, to put on a simulated bus.
     */
    reporting(
      address: number,
      shuntMilliohms: number,
      maxMicroamps: number,
      busMillivolts: number,
      microamps: number,
    ): WordPart {
      return ina219SimReporting(address, shuntMilliohms, maxMicroamps, busMillivolts, microamps)
    },
  },
}

/** A TI ADS1115 16-bit analog-to-digital converter. */
export const ads1115 = {
  /** The address with ADDR tied to ground. */
  addressGnd: 0x48,
  /** The address with ADDR tied to VDD. */
  addressVdd: 0x49,
  /** The address with ADDR tied to SDA. */
  addressSda: 0x4a,
  /** The address with ADDR tied to SCL. */
  addressScl: 0x4b,
  /** The value the configuration register reads after a reset. */
  configReset: 0x8583,
  /** The registers the pointer selects. */
  register: {
    /** The last conversion, two's complement. */
    conversion: 0x00,
    /** Input, gain, mode, data rate, and comparator settings. */
    config: 0x01,
    /** The comparator's low threshold. */
    loThresh: 0x02,
    /** The comparator's high threshold. */
    hiThresh: 0x03,
  },
  /** The input multiplexer codes. */
  mux: {
    /** AIN0 against AIN1, the reset setting. */
    ain0Ain1: 0,
    /** AIN0 against AIN3. */
    ain0Ain3: 1,
    /** AIN1 against AIN3. */
    ain1Ain3: 2,
    /** AIN2 against AIN3. */
    ain2Ain3: 3,
    /** AIN0 against ground. */
    ain0Gnd: 4,
    /** AIN1 against ground. */
    ain1Gnd: 5,
    /** AIN2 against ground. */
    ain2Gnd: 6,
    /** AIN3 against ground. */
    ain3Gnd: 7,
  },
  /** The gain codes, by the full-scale range each selects. */
  pga: {
    /** 6.144 V either side of zero. */
    fsr6_144: 0,
    /** 4.096 V. */
    fsr4_096: 1,
    /** 2.048 V, the reset setting. */
    fsr2_048: 2,
    /** 1.024 V. */
    fsr1_024: 3,
    /** 0.512 V. */
    fsr0_512: 4,
    /** 0.256 V. */
    fsr0_256: 5,
  },
  /** The data-rate codes, in samples per second. */
  dataRate: {
    /** 8 per second. */
    sps8: 0,
    /** 16 per second. */
    sps16: 1,
    /** 32 per second. */
    sps32: 2,
    /** 64 per second. */
    sps64: 3,
    /** 128 per second, the reset setting. */
    sps128: 4,
    /** 250 per second. */
    sps250: 5,
    /** 475 per second. */
    sps475: 6,
    /** 860 per second. */
    sps860: 7,
  },

  /**
   * Returns how long a conversion takes at a data rate: one period plus the datasheet's ten
   * percent rate variation.
   *
   * @param dataRate - One of the {@link ads1115.dataRate} codes.
   * @returns The time in microseconds.
   */
  conversionMicros(dataRate: number): number {
    return ads1115ConversionMicros(dataRate)
  },

  /**
   * Assembles the 16-bit configuration register value.
   *
   * @param config - The settings to encode.
   * @returns The register value to write, most significant bit first.
   */
  configBits(config: Ads1115Config): number {
    return ads1115ConfigBits(config)
  },

  /**
   * Parses a 16-bit configuration register value.
   *
   * @param bits - The register value, as read from the device.
   * @returns The decoded settings. Every value decodes, so this never throws.
   */
  configFromBits(bits: number): Ads1115Config {
    return ads1115ConfigFromBits(bits)
  },

  /**
   * Returns the full-scale range a gain code selects.
   *
   * @param pga - The gain code, 0 to 7.
   * @returns The full scale in microvolts.
   */
  fullScaleMicrovolts(pga: number): number {
    return ads1115FullScaleMicrovolts(pga)
  },

  /**
   * Returns the sample rate a data-rate code selects.
   *
   * @param dataRate - The data-rate code, 0 to 7.
   * @returns The rate in samples per second.
   */
  samplesPerSecond(dataRate: number): number {
    return ads1115SamplesPerSecond(dataRate)
  },

  /**
   * Converts a raw conversion result to nanovolts.
   *
   * @param pga - The gain the conversion was taken at.
   * @param raw - The signed conversion register value.
   * @returns The measured voltage, exact at every gain setting.
   */
  toNanovolts(pga: number, raw: number): number {
    return Number(ads1115ToNanovolts(pga, raw))
  },

  /**
   * Converts a raw conversion result to volts.
   *
   * @param pga - The gain the conversion was taken at.
   * @param raw - The signed conversion register value.
   * @returns The measured voltage.
   */
  toVolts(pga: number, raw: number): number {
    return ads1115ToVolts(pga, raw)
  },

  /**
   * An ADS1115 that is not there, for a bus with nothing plugged in. A conversion comes back
   * as a count of the range the gain selects, so `reporting` takes the same gain a driver is
   * given.
   */
  sim: {
    /** The voltage `part` reports: half a 3.3 V supply. */
    volts: 1.65,

    /**
     * A part reading 1.65 V at the range a driver starts with.
     *
     * @param address - The address it answers to.
     * @returns The part, to put on a simulated bus.
     */
    part(address: number): WordPart {
      return ads1115SimPart(address)
    },

    /**
     * A part that reads what it is asked to, on the nearest of the 32768 steps either side of
     * zero the range divides into.
     *
     * @param address - The address it answers to.
     * @param pga - The gain code the driver converts at.
     * @param volts - The voltage it reports, held to the range.
     * @returns The part, to put on a simulated bus.
     */
    reporting(address: number, pga: number, volts: number): WordPart {
      return ads1115SimReporting(address, pga, volts)
    },
  },
}

/** The repeatability of an SHT3x measurement, traded against time and energy. */
export const Sht3xRepeatability = {
  /** The quickest and noisiest setting. */
  Low: 'Low' as Sht3xRepeatabilityName,
  /** The middle setting. */
  Medium: 'Medium' as Sht3xRepeatabilityName,
  /** The slowest and quietest setting. */
  High: 'High' as Sht3xRepeatabilityName,
} as const

/** The repeatability of an SHT3x measurement, traded against time and energy. */
export type Sht3xRepeatability = Sht3xRepeatabilityName

/** How often an SHT3x in periodic mode takes a measurement. */
export const Sht3xRate = {
  /** One measurement every two seconds. */
  HalfMps: 'HalfMps' as Sht3xRateName,
  /** One measurement per second. */
  OneMps: 'OneMps' as Sht3xRateName,
  /** Two measurements per second. */
  TwoMps: 'TwoMps' as Sht3xRateName,
  /** Four measurements per second. */
  FourMps: 'FourMps' as Sht3xRateName,
  /** Ten measurements per second. */
  TenMps: 'TenMps' as Sht3xRateName,
} as const

/** How often an SHT3x in periodic mode takes a measurement. */
export type Sht3xRate = Sht3xRateName

/** The limit comparison an INA226's alert pin responds to. */
export const Ina226AlertFunction = {
  /** The shunt voltage rose above the alert limit. */
  ShuntOverLimit: 'ShuntOverLimit' as Ina226AlertFunctionName,
  /** The shunt voltage fell below the alert limit. */
  ShuntUnderLimit: 'ShuntUnderLimit' as Ina226AlertFunctionName,
  /** The bus voltage rose above the alert limit. */
  BusOverLimit: 'BusOverLimit' as Ina226AlertFunctionName,
  /** The bus voltage fell below the alert limit. */
  BusUnderLimit: 'BusUnderLimit' as Ina226AlertFunctionName,
  /** The power rose above the alert limit. */
  PowerOverLimit: 'PowerOverLimit' as Ina226AlertFunctionName,
} as const

/** The limit comparison an INA226's alert pin responds to. */
export type Ina226AlertFunction = Ina226AlertFunctionName

/** A Bosch BMP280 pressure and temperature sensor, the BME280 without humidity. */
export const bmp280 = {
  /** The address a BMP280 answers on with its SDO pin low. */
  addressPrimary: 0x76,
  /** The address it answers on with SDO high. */
  addressSecondary: 0x77,
  /** The value its chip-ID register reads, which confirms the part. */
  chipId: 0x58,
  /** The word written to the reset register to restart the part. */
  resetWord: 0xb6,
  /** The raw output a channel reports when its measurement is switched off. */
  skippedOutput: 0x80000,
  /** How many bytes the calibration block holds. */
  calibrationLength: 24,
  /** How many bytes one measurement burst holds. */
  dataLength: 6,
  /** The registers a driver reads and writes. */
  register: {
    /** The first of the 24 calibration bytes. */
    calibration: 0x88,
    /** The chip-ID register. */
    chipId: 0xd0,
    /** The reset register. */
    reset: 0xe0,
    /** The status register. */
    status: 0xf3,
    /** The measurement-control register. */
    ctrlMeas: 0xf4,
    /** The configuration register. */
    config: 0xf5,
    /** The first of the six data bytes. */
    data: 0xf7,
  },
  /** The oversampling codes: how many samples each measurement averages. */
  oversampling: {
    /** The measurement is skipped. */
    skipped: 0,
    /** One sample. */
    x1: 1,
    /** Two samples. */
    x2: 2,
    /** Four samples. */
    x4: 3,
    /** Eight samples. */
    x8: 4,
    /** Sixteen samples. */
    x16: 5,
  },

  /**
   * A BMP280 that is not there, for a bus with nothing plugged in. A BMP280 is a BME280
   * without humidity, so `part` holds the temperature and pressure half of a real part and
   * reads 20.44 C and 848.05 hPa.
   */
  sim: {
    /** The status a simulated part reports when it is neither measuring nor loading. */
    statusIdle: 0x00,

    /**
     * A part holding a real part's trimming and one measurement it took.
     *
     * @param address - The address it answers to.
     * @returns The part, to put on a simulated bus.
     */
    part(address: number): I2cPart {
      return bmp280SimPart(address)
    },

    /**
     * A part that reads what it is asked to, within a hundredth of a degree and of a
     * hectopascal.
     *
     * @param address - The address it answers to.
     * @param celsius - The temperature it reports.
     * @param hectopascals - The pressure it reports.
     * @returns The part, to put on a simulated bus.
     */
    reporting(address: number, celsius: number, hectopascals: number): I2cPart {
      return bmp280SimReporting(address, celsius, hectopascals)
    },

    /** The 24 trimming bytes a simulated part holds. */
    calibration(): Buffer {
      return bmp280SimCalibration()
    },

    /** The six data registers a simulated part holds: one measurement a real part took. */
    burst(): Buffer {
      return bmp280SimBurst()
    },

    /**
     * The six data registers that compensate to a reading against the simulated trimming.
     *
     * @param celsius - The temperature.
     * @param hectopascals - The pressure.
     * @returns The bytes a burst read would return.
     */
    burstFor(celsius: number, hectopascals: number): Buffer {
      return bmp280SimBurstFor(celsius, hectopascals)
    },
  },

  /**
   * Reads the factory calibration out of the registers, once at start-up.
   *
   * @param bytes - The 24-byte calibration block.
   * @returns The calibration, to reuse for every measurement.
   * @throws If the block is the wrong length.
   */
  calibration(bytes: Uint8Array): Bmp280Calibration {
    return new Bmp280Calibration(Buffer.from(bytes))
  },

  /**
   * Splits a six-byte data read into its two uncompensated outputs.
   *
   * @param bytes - The six data registers in read order.
   * @returns The raw pressure and temperature, before compensation.
   * @throws If the read is the wrong length.
   */
  parseMeasurement(bytes: Uint8Array): Bmp280RawMeasurement {
    return bmp280ParseMeasurement(Buffer.from(bytes))
  },

  /**
   * Builds the six data-register bytes a part reports for two raw outputs.
   *
   * The inverse of {@link parseMeasurement}, so a node can be written and tested
   * against what a sensor sends without one attached.
   *
   * @param pressure - The 20-bit uncompensated pressure.
   * @param temperature - The 20-bit uncompensated temperature.
   * @returns The six bytes in read order.
   */
  measurementBytes(pressure: number, temperature: number): Buffer {
    return bmp280MeasurementBytes(pressure, temperature)
  },

  /**
   * Reports whether a raw pressure says the measurement is switched off.
   *
   * @param pressure - The 20-bit uncompensated pressure.
   * @returns Whether the channel's oversampling is set to skip.
   */
  pressureSkipped(pressure: number): boolean {
    return bmp280PressureSkipped(pressure)
  },

  /**
   * Reports whether a raw temperature says the measurement is switched off.
   *
   * @param temperature - The 20-bit uncompensated temperature.
   * @returns Whether the channel's oversampling is set to skip.
   */
  temperatureSkipped(temperature: number): boolean {
    return bmp280TemperatureSkipped(temperature)
  },

  /**
   * Reports whether a status register says a conversion is running.
   *
   * @param status - The status register.
   * @returns Whether a measurement is in progress.
   */
  measuring(status: number): boolean {
    return bmp280Measuring(status)
  },

  /**
   * Reports whether a status register says the calibration is being copied.
   *
   * @param status - The status register.
   * @returns Whether the image registers are being reloaded.
   */
  imageUpdating(status: number): boolean {
    return bmp280ImageUpdating(status)
  },

  /**
   * Assembles the `ctrl_meas` register value.
   *
   * @param ctrl - The oversampling codes and power mode to encode.
   * @returns The register value to write.
   */
  ctrlMeasBits(ctrl: Bmp280CtrlMeas): number {
    return bmp280CtrlMeasBits(ctrl)
  },

  /**
   * Parses a `ctrl_meas` register value.
   *
   * @param bits - The register value, as read from the device.
   * @returns The decoded settings.
   */
  ctrlMeasFromBits(bits: number): Bmp280CtrlMeas {
    return bmp280CtrlMeasFromBits(bits)
  },

  /**
   * Assembles the `config` register value.
   *
   * @param config - The standby time, filter, and interface settings.
   * @returns The register value to write.
   */
  configBits(config: Bmp280Config): number {
    return bmp280ConfigBits(config)
  },

  /**
   * Parses a `config` register value.
   *
   * @param bits - The register value, as read from the device.
   * @returns The decoded settings.
   */
  configFromBits(bits: number): Bmp280Config {
    return bmp280ConfigFromBits(bits)
  },

  /**
   * Returns how many samples an oversampling code averages.
   *
   * @param code - The oversampling code, 0 to 7.
   * @returns The sample count, or `0` when the measurement is switched off.
   */
  oversamplingFactor(code: number): number {
    return bmp280OversamplingFactor(code)
  },

  /**
   * Returns the standby time a standby code selects in normal mode.
   *
   * @param code - The standby code, 0 to 7.
   * @returns The standby time in microseconds.
   */
  standbyMicros(code: number): number {
    return bmp280StandbyMicros(code)
  },
}

/** A Sensirion SHT3x humidity and temperature sensor. */
export const sht3x = {
  /** The address the part answers on with its ADDR pin low. */
  addressA: 0x44,
  /** The address it answers on with ADDR high. */
  addressB: 0x45,
  /** The shortest gap the datasheet allows between two commands, in microseconds. */
  minCommandGapMicros: 1000,
  /** The status word the part powers up with. */
  statusDefault: 0x8010,
  /** The command words a driver writes. */
  command: {
    /** Single shot, high repeatability, holding the clock until the result is ready. */
    singleShotHighStretch: 0x2c06,
    /** Single shot, medium repeatability, with clock stretching. */
    singleShotMediumStretch: 0x2c0d,
    /** Single shot, low repeatability, with clock stretching. */
    singleShotLowStretch: 0x2c10,
    /** Single shot, high repeatability, polled rather than stretched. */
    singleShotHigh: 0x2400,
    /** Single shot, medium repeatability, polled. */
    singleShotMedium: 0x240b,
    /** Single shot, low repeatability, polled. */
    singleShotLow: 0x2416,
    /** Periodic at one measurement every two seconds, high repeatability. */
    periodicHalfMpsHigh: 0x2032,
    /** Periodic at one measurement every two seconds, medium repeatability. */
    periodicHalfMpsMedium: 0x2024,
    /** Periodic at one measurement every two seconds, low repeatability. */
    periodicHalfMpsLow: 0x202f,
    /** Periodic at one measurement per second, high repeatability. */
    periodicOneMpsHigh: 0x2130,
    /** Periodic at one measurement per second, medium repeatability. */
    periodicOneMpsMedium: 0x2126,
    /** Periodic at one measurement per second, low repeatability. */
    periodicOneMpsLow: 0x212d,
    /** Periodic at two measurements per second, high repeatability. */
    periodicTwoMpsHigh: 0x2236,
    /** Periodic at two measurements per second, medium repeatability. */
    periodicTwoMpsMedium: 0x2220,
    /** Periodic at two measurements per second, low repeatability. */
    periodicTwoMpsLow: 0x222b,
    /** Periodic at four measurements per second, high repeatability. */
    periodicFourMpsHigh: 0x2334,
    /** Periodic at four measurements per second, medium repeatability. */
    periodicFourMpsMedium: 0x2322,
    /** Periodic at four measurements per second, low repeatability. */
    periodicFourMpsLow: 0x2329,
    /** Periodic at ten measurements per second, high repeatability. */
    periodicTenMpsHigh: 0x2737,
    /** Periodic at ten measurements per second, medium repeatability. */
    periodicTenMpsMedium: 0x2721,
    /** Periodic at ten measurements per second, low repeatability. */
    periodicTenMpsLow: 0x272a,
    /** Accelerated response time, four measurements per second. */
    periodicArt: 0x2b32,
    /** Fetch the latest periodic result. */
    fetchData: 0xe000,
    /** Leave periodic mode. */
    break: 0x3093,
    /** Soft reset. */
    softReset: 0x30a2,
    /** The general-call reset, addressed to 0x00. */
    generalCallReset: 0x0006,
    /** Turn the on-die heater on. */
    heaterEnable: 0x306d,
    /** Turn the on-die heater off. */
    heaterDisable: 0x3066,
    /** Read the status register. */
    readStatus: 0xf32d,
    /** Clear the status register. */
    clearStatus: 0x3041,
  },

  /**
   * Computes the Sensirion CRC-8 the part appends to every word.
   *
   * @param data - The bytes the checksum covers.
   * @returns The checksum.
   */
  crc(data: Uint8Array): number {
    return sht3xCrc(Buffer.from(data))
  },

  /**
   * Reads a CRC-checked word off the bus.
   *
   * @param frame - The two data bytes and their checksum.
   * @returns The word.
   * @throws If the checksum does not match, which means the read was corrupted on
   * the bus and should be repeated.
   */
  word(frame: Uint8Array): number {
    return sht3xWord(Buffer.from(frame))
  },

  /**
   * Builds the three bytes the part sends for a word, its checksum last.
   *
   * @param value - The word.
   * @returns The two data bytes and their checksum.
   */
  wordBytes(value: number): Buffer {
    return sht3xWordBytes(value)
  },

  /**
   * Parses and CRC-checks a six-byte measurement frame.
   *
   * @param bytes - The frame as the device sent it, each word followed by its CRC.
   * @returns The raw words and the physical values they carry.
   * @throws If either checksum does not match.
   */
  parseMeasurement(bytes: Uint8Array): Sht3xMeasurement {
    return sht3xParseMeasurement(Buffer.from(bytes))
  },

  /**
   * Builds the six bytes the part sends for a pair of raw words.
   *
   * The inverse of {@link parseMeasurement}.
   *
   * @param temperatureRaw - The raw temperature word.
   * @param humidityRaw - The raw humidity word.
   * @returns The frame, each word followed by its CRC.
   */
  measurementBytes(temperatureRaw: number, humidityRaw: number): Buffer {
    return sht3xMeasurementBytes(temperatureRaw, humidityRaw)
  },

  /**
   * Parses and CRC-checks a three-byte status frame.
   *
   * @param bytes - The status word and its checksum.
   * @returns The status word and its flags.
   * @throws If the checksum does not match.
   */
  parseStatus(bytes: Uint8Array): Sht3xStatus {
    return sht3xParseStatus(Buffer.from(bytes))
  },

  /**
   * Splits a status word into its flags.
   *
   * @param bits - The status register, as read from the device.
   * @returns The flags the word carries.
   */
  statusFromBits(bits: number): Sht3xStatus {
    return sht3xStatusFromBits(bits)
  },

  /**
   * Builds the three bytes the part sends for a status word.
   *
   * @param bits - The status register.
   * @returns The word and its checksum.
   */
  statusBytes(bits: number): Buffer {
    return sht3xStatusBytes(bits)
  },

  /**
   * Converts a raw temperature word to milli-degrees Celsius.
   *
   * @param raw - The 16-bit word.
   * @returns The temperature, exact in integer arithmetic.
   */
  milliCelsius(raw: number): number {
    return sht3xMilliCelsius(raw)
  },

  /**
   * Converts a raw temperature word to degrees Celsius.
   *
   * @param raw - The 16-bit word.
   * @returns The temperature.
   */
  celsius(raw: number): number {
    return sht3xCelsius(raw)
  },

  /**
   * Converts a raw temperature word to milli-degrees Fahrenheit.
   *
   * @param raw - The 16-bit word.
   * @returns The temperature, exact in integer arithmetic.
   */
  milliFahrenheit(raw: number): number {
    return sht3xMilliFahrenheit(raw)
  },

  /**
   * Converts a raw temperature word to degrees Fahrenheit.
   *
   * @param raw - The 16-bit word.
   * @returns The temperature.
   */
  fahrenheit(raw: number): number {
    return sht3xFahrenheit(raw)
  },

  /**
   * Converts a raw humidity word to milli-percent relative humidity.
   *
   * @param raw - The 16-bit word.
   * @returns The humidity, exact in integer arithmetic.
   */
  milliPercent(raw: number): number {
    return sht3xMilliPercent(raw)
  },

  /**
   * Converts a raw humidity word to a relative humidity percentage.
   *
   * @param raw - The 16-bit word.
   * @returns The humidity.
   */
  relativeHumidity(raw: number): number {
    return sht3xRelativeHumidity(raw)
  },

  /**
   * Builds the raw temperature word for a temperature in milli-degrees Celsius.
   *
   * @param milliCelsius - The temperature.
   * @returns The 16-bit word, clamped to the part's range.
   */
  temperatureRawFromMilliCelsius(milliCelsius: number): number {
    return sht3xTemperatureRawFromMilliCelsius(milliCelsius)
  },

  /**
   * Builds the raw temperature word for a temperature in degrees Celsius.
   *
   * @param celsius - The temperature.
   * @returns The 16-bit word, clamped to the part's range.
   */
  temperatureRawFromCelsius(celsius: number): number {
    return sht3xTemperatureRawFromCelsius(celsius)
  },

  /**
   * Builds the raw temperature word for a temperature in milli-degrees Fahrenheit.
   *
   * @param milliFahrenheit - The temperature.
   * @returns The 16-bit word, clamped to the part's range.
   */
  temperatureRawFromMilliFahrenheit(milliFahrenheit: number): number {
    return sht3xTemperatureRawFromMilliFahrenheit(milliFahrenheit)
  },

  /**
   * Builds the raw humidity word for a humidity in milli-percent.
   *
   * @param milliPercent - The relative humidity.
   * @returns The 16-bit word, clamped to the part's range.
   */
  humidityRawFromMilliPercent(milliPercent: number): number {
    return sht3xHumidityRawFromMilliPercent(milliPercent)
  },

  /**
   * Builds the raw humidity word for a relative humidity percentage.
   *
   * @param percent - The relative humidity.
   * @returns The 16-bit word, clamped to the part's range.
   */
  humidityRawFromRelativeHumidity(percent: number): number {
    return sht3xHumidityRawFromRelativeHumidity(percent)
  },

  /**
   * Returns the command word for a single-shot measurement.
   *
   * @param repeatability - How much the part should average.
   * @param clockStretching - Whether the part holds the clock until the result is
   * ready, rather than answering a poll with a NACK.
   * @returns The command word.
   */
  singleShot(repeatability: Sht3xRepeatability, clockStretching: boolean): number {
    return sht3xSingleShot(repeatability, clockStretching)
  },

  /**
   * Returns the command word that starts periodic measurement.
   *
   * @param repeatability - How much the part should average.
   * @param rate - How often it should measure.
   * @returns The command word.
   */
  periodic(repeatability: Sht3xRepeatability, rate: Sht3xRate): number {
    return sht3xPeriodic(repeatability, rate)
  },

  /**
   * Returns how long a measurement may take.
   *
   * @param repeatability - The setting in force.
   * @returns The datasheet's worst case, in microseconds.
   */
  maxMeasurementMicros(repeatability: Sht3xRepeatability): number {
    return sht3xMaxMeasurementMicros(repeatability)
  },

  /**
   * Returns how long a measurement typically takes.
   *
   * @param repeatability - The setting in force.
   * @returns The datasheet's typical case, in microseconds.
   */
  typicalMeasurementMicros(repeatability: Sht3xRepeatability): number {
    return sht3xTypicalMeasurementMicros(repeatability)
  },

  /**
   * Returns the interval between periodic measurements.
   *
   * @param rate - The rate in force.
   * @returns The interval in microseconds.
   */
  intervalMicros(rate: Sht3xRate): number {
    return sht3xIntervalMicros(rate)
  },

  /**
   * An SHT3x that is not there, for a bus with nothing plugged in. It answers every
   * single-shot command and a periodic fetch with the reading and its checksums, and the
   * status command with the status a part reports after a reset.
   */
  sim: {
    /** The temperature `part` reports. */
    celsius: 22.5,
    /** The relative humidity `part` reports, as a percentage. */
    relativeHumidity: 45.0,

    /**
     * A part reading 22.5 C and 45 %.
     *
     * @param address - The address it answers to.
     * @returns The part, to put on a simulated bus.
     */
    part(address: number): CommandPart {
      return sht3xSimPart(address)
    },

    /**
     * A part that reads what it is asked to, within three thousandths of a degree and two
     * thousandths of a percent.
     *
     * @param address - The address it answers to.
     * @param celsius - The temperature it reports.
     * @param relativeHumidity - The humidity it reports, as a percentage.
     * @returns The part, to put on a simulated bus.
     */
    reporting(address: number, celsius: number, relativeHumidity: number): CommandPart {
      return sht3xSimReporting(address, celsius, relativeHumidity)
    },
  },
}

/** A Sensirion SCD40 or SCD41 photoacoustic CO2, humidity, and temperature sensor. */
export const scd4x = {
  /** The only address the part answers on. */
  address: 0x62,
  /** The highest concentration the part reports, in parts per million. */
  co2MaxPpm: 40000,
  /** The temperature offset the part powers up with, in milli-degrees Celsius. */
  defaultTemperatureOffsetMilliCelsius: 4000,
  /** How often periodic measurement produces a result, in milliseconds. */
  periodicMeasurementIntervalMs: 5000,
  /** How often low-power periodic measurement produces a result, in milliseconds. */
  lowPowerPeriodicMeasurementIntervalMs: 30000,
  /** How long the part takes to become ready after power-up, in milliseconds. */
  powerUpTimeMs: 1000,
  /** The word a forced recalibration returns when it did not take. */
  forcedRecalibrationFailed: 0xffff,
  /** The command words a driver writes. */
  command: {
    /** Start periodic measurement. */
    startPeriodicMeasurement: 0x21b1,
    /** Read the latest result. */
    readMeasurement: 0xec05,
    /** Leave periodic measurement. */
    stopPeriodicMeasurement: 0x3f86,
    /** Write the temperature offset. */
    setTemperatureOffset: 0x241d,
    /** Read the temperature offset. */
    getTemperatureOffset: 0x2318,
    /** Write the installation altitude. */
    setSensorAltitude: 0x2427,
    /** Read the installation altitude. */
    getSensorAltitude: 0x2322,
    /** Write the ambient pressure, which overrides the altitude. */
    setAmbientPressure: 0xe000,
    /** Run a forced recalibration against a known concentration. */
    performForcedRecalibration: 0x362f,
    /** Turn automatic self-calibration on or off. */
    setAutomaticSelfCalibrationEnabled: 0x2416,
    /** Read whether automatic self-calibration is on. */
    getAutomaticSelfCalibrationEnabled: 0x2313,
    /** Start low-power periodic measurement. */
    startLowPowerPeriodicMeasurement: 0x21ac,
    /** Ask whether a result is waiting. */
    getDataReadyStatus: 0xe4b8,
    /** Copy the settings to non-volatile memory. */
    persistSettings: 0x3615,
    /** Read the serial number. */
    getSerialNumber: 0x3682,
    /** Run the self test. */
    performSelfTest: 0x3639,
    /** Restore the factory settings. */
    performFactoryReset: 0x3632,
    /** Reinitialise from the stored settings. */
    reinit: 0x3646,
    /** Take one measurement and return to idle. */
    measureSingleShot: 0x219d,
    /** Take one humidity and temperature measurement, without CO2. */
    measureSingleShotRhtOnly: 0x2196,
    /** Enter sleep, SCD41 only. */
    powerDown: 0x36e0,
    /** Leave sleep, SCD41 only. */
    wakeUp: 0x36f6,
  },

  /**
   * Computes the Sensirion CRC-8 the part appends to every word.
   *
   * @param data - The bytes the checksum covers.
   * @returns The checksum.
   */
  crc(data: Uint8Array): number {
    return scd4xCrc(Buffer.from(data))
  },

  /**
   * Reads a CRC-checked word off the bus.
   *
   * @param frame - The two data bytes and their checksum.
   * @returns The word.
   * @throws If the checksum does not match.
   */
  word(frame: Uint8Array): number {
    return scd4xWord(Buffer.from(frame))
  },

  /**
   * Builds the three bytes the part sends for a word, its checksum last.
   *
   * @param value - The word.
   * @returns The two data bytes and their checksum.
   */
  wordFrame(value: number): Buffer {
    return scd4xWordFrame(value)
  },

  /**
   * Builds the two bytes that send a command with no argument.
   *
   * @param command - The command word.
   * @returns The bytes to write.
   */
  commandFrame(command: number): Buffer {
    return scd4xCommandFrame(command)
  },

  /**
   * Builds the five bytes that send a command with an argument.
   *
   * @param command - The command word.
   * @param value - The argument.
   * @returns The command, the argument, and the argument's checksum.
   */
  writeFrame(command: number, value: number): Buffer {
    return scd4xWriteFrame(command, value)
  },

  /**
   * Returns how long a command may take before its result can be read.
   *
   * @param command - The command word.
   * @returns The datasheet's worst case in milliseconds, or `null` for a command
   * that returns immediately.
   */
  maxDurationMs(command: number): number | null {
    return scd4xMaxDurationMs(command)
  },

  /**
   * Reports whether the part accepts a command while it is measuring.
   *
   * @param command - The command word.
   * @returns Whether the command can be sent without stopping measurement first.
   */
  allowedDuringMeasurement(command: number): boolean {
    return scd4xAllowedDuringMeasurement(command)
  },

  /**
   * Parses and CRC-checks a nine-byte measurement frame.
   *
   * @param bytes - The frame as the device sent it, each word followed by its CRC.
   * @returns The raw words and the physical values they carry.
   * @throws If any checksum does not match.
   */
  parseMeasurement(bytes: Uint8Array): Scd4xMeasurement {
    return scd4xParseMeasurement(Buffer.from(bytes))
  },

  /**
   * Builds the measurement a part reporting these physical values would send.
   *
   * @param co2Ppm - The CO2 concentration in parts per million.
   * @param milliCelsius - The temperature in milli-degrees Celsius.
   * @param humidityMilliPercent - The relative humidity in milli-percent.
   * @returns The measurement, with the raw words the part would carry.
   */
  measurementFromPhysical(
    co2Ppm: number,
    milliCelsius: number,
    humidityMilliPercent: number,
  ): Scd4xMeasurement {
    return scd4xMeasurementFromPhysical(co2Ppm, milliCelsius, humidityMilliPercent)
  },

  /**
   * Builds the nine bytes the part sends for three raw words.
   *
   * The inverse of {@link parseMeasurement}.
   *
   * @param co2Ppm - The CO2 concentration in parts per million.
   * @param temperatureRaw - The raw temperature word.
   * @param humidityRaw - The raw humidity word.
   * @returns The frame, each word followed by its CRC.
   */
  measurementBytes(co2Ppm: number, temperatureRaw: number, humidityRaw: number): Buffer {
    return scd4xMeasurementBytes(co2Ppm, temperatureRaw, humidityRaw)
  },

  /**
   * Converts a raw temperature word to milli-degrees Celsius.
   *
   * @param raw - The 16-bit word.
   * @returns The temperature, exact in integer arithmetic.
   */
  milliCelsius(raw: number): number {
    return scd4xMilliCelsius(raw)
  },

  /**
   * Converts a raw temperature word to degrees Celsius.
   *
   * @param raw - The 16-bit word.
   * @returns The temperature.
   */
  celsius(raw: number): number {
    return scd4xCelsius(raw)
  },

  /**
   * Builds the raw temperature word for a temperature in milli-degrees Celsius.
   *
   * @param milliCelsius - The temperature.
   * @returns The 16-bit word, clamped to the part's range.
   */
  temperatureRaw(milliCelsius: number): number {
    return scd4xTemperatureRaw(milliCelsius)
  },

  /**
   * Converts a raw humidity word to milli-percent relative humidity.
   *
   * @param raw - The 16-bit word.
   * @returns The humidity, exact in integer arithmetic.
   */
  humidityMilliPercent(raw: number): number {
    return scd4xHumidityMilliPercent(raw)
  },

  /**
   * Converts a raw humidity word to a relative humidity percentage.
   *
   * @param raw - The 16-bit word.
   * @returns The humidity.
   */
  relativeHumidityPercent(raw: number): number {
    return scd4xRelativeHumidityPercent(raw)
  },

  /**
   * Builds the raw humidity word for a humidity in milli-percent.
   *
   * @param milliPercent - The relative humidity.
   * @returns The 16-bit word, clamped to the part's range.
   */
  humidityRaw(milliPercent: number): number {
    return scd4xHumidityRaw(milliPercent)
  },

  /**
   * Reports whether a data-ready word says a measurement is waiting.
   *
   * @param word - The word the data-ready command returned.
   * @returns Whether a result can be read.
   */
  dataReady(word: number): boolean {
    return scd4xDataReady(word)
  },

  /**
   * Builds the temperature-offset word for an offset in milli-degrees Celsius.
   *
   * @param milliCelsius - The offset to subtract from the reading.
   * @returns The word to write.
   */
  temperatureOffsetWord(milliCelsius: number): number {
    return scd4xTemperatureOffsetWord(milliCelsius)
  },

  /**
   * Converts a temperature-offset word to milli-degrees Celsius.
   *
   * @param word - The word the part returned.
   * @returns The offset.
   */
  temperatureOffsetMilliCelsius(word: number): number {
    return scd4xTemperatureOffsetMilliCelsius(word)
  },

  /**
   * Builds the ambient-pressure word for a pressure in pascals.
   *
   * @param pascals - The ambient pressure the part should compensate for.
   * @returns The word to write, in hundreds of pascals.
   */
  ambientPressureWord(pascals: number): number {
    return scd4xAmbientPressureWord(pascals)
  },

  /**
   * Converts an ambient-pressure word to pascals.
   *
   * @param word - The word the part holds.
   * @returns The pressure.
   */
  ambientPressurePascals(word: number): number {
    return scd4xAmbientPressurePascals(word)
  },

  /**
   * Reads the correction a forced recalibration applied.
   *
   * @param word - The word the recalibration returned.
   * @returns The correction in parts per million, or `null` if the recalibration
   * did not take, which means the part had not measured for long enough.
   */
  forcedRecalibrationCorrectionPpm(word: number): number | null {
    return scd4xForcedRecalibrationCorrectionPpm(word)
  },

  /**
   * Builds the word a part returns for a forced-recalibration outcome.
   *
   * @param correctionPpm - The correction applied, or `null` for a failure.
   * @returns The word.
   */
  forcedRecalibrationWord(correctionPpm: number | null): number {
    return scd4xForcedRecalibrationWord(correctionPpm)
  },

  /**
   * Reports whether a self-calibration word says the feature is on.
   *
   * @param word - The word the part returned.
   * @returns Whether automatic self-calibration is enabled.
   */
  automaticSelfCalibrationEnabled(word: number): boolean {
    return scd4xAutomaticSelfCalibrationEnabled(word)
  },

  /**
   * Builds the word that turns automatic self-calibration on or off.
   *
   * @param enabled - Whether the feature should run.
   * @returns The word to write.
   */
  automaticSelfCalibrationWord(enabled: boolean): number {
    return scd4xAutomaticSelfCalibrationWord(enabled)
  },

  /**
   * Reports whether a self-test word says the part passed.
   *
   * @param word - The word the self test returned.
   * @returns Whether the part reported no malfunction.
   */
  selfTestPassed(word: number): boolean {
    return scd4xSelfTestPassed(word)
  },

  /**
   * Reads a serial number out of the nine bytes the part sends.
   *
   * @param frame - The three words and their checksums.
   * @returns The 48-bit serial number.
   * @throws If any checksum does not match.
   */
  serialNumber(frame: Uint8Array): number {
    return Number(scd4xSerialNumber(Buffer.from(frame)))
  },

  /**
   * Builds the nine bytes the part sends for a serial number.
   *
   * @param serial - The 48-bit serial number.
   * @returns The three words, each followed by its CRC.
   */
  serialNumberFrame(serial: number): Buffer {
    return scd4xSerialNumberFrame(serial)
  },

  /**
   * An SCD4x that is not there, for a bus with nothing plugged in. It answers the serial
   * number, data-ready, and measurement commands, each word with its checksum, and always has
   * a result waiting.
   */
  sim: {
    /** The carbon dioxide `part` reports, in parts per million. */
    co2Ppm: 800,
    /** The temperature `part` reports. */
    celsius: 22.5,
    /** The relative humidity `part` reports, as a percentage. */
    relativeHumidity: 45.0,
    /** The serial number every simulated part reports. */
    serial: 0x0000_5a4d_0c1e_2b3f,

    /**
     * A part reading 800 ppm, 22.5 C, and 45 %.
     *
     * @returns The part, to put on a simulated bus.
     */
    part(): CommandPart {
      return scd4xSimPart()
    },

    /**
     * A part that reads what it is asked to.
     *
     * @param co2Ppm - The carbon dioxide it reports, in parts per million.
     * @param celsius - The temperature it reports.
     * @param relativeHumidity - The humidity it reports, as a percentage.
     * @returns The part, to put on a simulated bus.
     */
    reporting(co2Ppm: number, celsius: number, relativeHumidity: number): CommandPart {
      return scd4xSimReporting(co2Ppm, celsius, relativeHumidity)
    },
  },
}

/** A TI TMP117 high-accuracy digital temperature sensor. */
export const tmp117 = {
  /** The value the device-ID register reads, which confirms the part. */
  deviceId: 0x0117,
  /** The value the configuration register reads after a reset. */
  configReset: 0x0220,
  /** The value the high-limit register holds after a reset. */
  highLimitReset: 0x6000,
  /** The value the low-limit register holds after a reset. */
  lowLimitReset: 0x8000,
  /** The value the result register holds before the first conversion. */
  tempResultReset: 0x8000,
  /** The byte a general-call reset sends to address 0x00. */
  generalCallReset: 0x06,
  /** The word written to the EEPROM-unlock register to allow a write. */
  eepromUnlock: 0x8000,
  /** The addresses the ADD0 pin selects. */
  address: {
    /** ADD0 tied to GND. */
    add0Gnd: 0x48,
    /** ADD0 tied to V+. */
    add0Vplus: 0x49,
    /** ADD0 tied to SDA. */
    add0Sda: 0x4a,
    /** ADD0 tied to SCL. */
    add0Scl: 0x4b,
  },
  /** The registers a driver reads and writes. */
  register: {
    /** The temperature result. */
    tempResult: 0x00,
    /** The configuration register. */
    configuration: 0x01,
    /** The high alert limit. */
    thighLimit: 0x02,
    /** The low alert limit. */
    tlowLimit: 0x03,
    /** The EEPROM unlock register. */
    eepromUl: 0x04,
    /** The first general-purpose EEPROM word. */
    eeprom1: 0x05,
    /** The second general-purpose EEPROM word. */
    eeprom2: 0x06,
    /** The temperature offset applied to every result. */
    tempOffset: 0x07,
    /** The third general-purpose EEPROM word. */
    eeprom3: 0x08,
    /** The device-ID register. */
    deviceId: 0x0f,
  },

  /**
   * Converts a raw temperature register to nano-degrees Celsius.
   *
   * @param raw - The signed 16-bit register.
   * @returns The temperature, exact at the part's 7.8125 m°C resolution.
   */
  nanoCelsius(raw: number): number {
    return Number(tmp117NanoCelsius(raw))
  },

  /**
   * Converts a raw temperature register to micro-degrees Celsius.
   *
   * @param raw - The signed 16-bit register.
   * @returns The temperature, truncated toward zero.
   */
  microCelsius(raw: number): number {
    return tmp117MicroCelsius(raw)
  },

  /**
   * Converts a raw temperature register to degrees Celsius.
   *
   * @param raw - The signed 16-bit register.
   * @returns The temperature.
   */
  celsius(raw: number): number {
    return tmp117Celsius(raw)
  },

  /**
   * Builds the raw register for a temperature in micro-degrees Celsius.
   *
   * @param microCelsius - The temperature.
   * @returns The nearest register value, saturating at the part's range.
   */
  rawFromMicroCelsius(microCelsius: number): number {
    return tmp117RawFromMicroCelsius(microCelsius)
  },

  /**
   * Builds the raw register for a temperature in degrees Celsius.
   *
   * @param celsius - The temperature.
   * @returns The nearest register value, saturating at the part's range.
   */
  rawFromCelsius(celsius: number): number {
    return tmp117RawFromCelsius(celsius)
  },

  /**
   * Builds the two bytes the part sends for a temperature register.
   *
   * @param raw - The signed 16-bit register.
   * @returns The register, most significant byte first.
   */
  temperatureBytes(raw: number): Buffer {
    return tmp117TemperatureBytes(raw)
  },

  /**
   * Reads a temperature register out of the two bytes the part sends.
   *
   * @param bytes - The register, most significant byte first.
   * @returns The signed register value.
   * @throws If the read is not two bytes.
   */
  temperatureFromBytes(bytes: Uint8Array): number {
    return tmp117TemperatureFromBytes(Buffer.from(bytes))
  },

  /**
   * Returns the device identifier a device-ID register carries.
   *
   * @param raw - The device-ID register.
   * @returns The 12-bit identifier, which is `0x117` for a TMP117.
   */
  deviceIdFromRegister(raw: number): number {
    return tmp117DeviceId(raw)
  },

  /**
   * Returns the die revision a device-ID register carries.
   *
   * @param raw - The device-ID register.
   * @returns The 4-bit revision.
   */
  revision(raw: number): number {
    return tmp117Revision(raw)
  },

  /**
   * Reports whether a configuration register flags a high alert.
   *
   * @param config - The configuration register.
   * @returns Whether the result went above the high limit.
   */
  highAlert(config: number): boolean {
    return tmp117HighAlert(config)
  },

  /**
   * Reports whether a configuration register flags a low alert.
   *
   * @param config - The configuration register.
   * @returns Whether the result went below the low limit.
   */
  lowAlert(config: number): boolean {
    return tmp117LowAlert(config)
  },

  /**
   * Reports whether a configuration register says a conversion is ready.
   *
   * @param config - The configuration register.
   * @returns Whether a new result is waiting.
   */
  dataReady(config: number): boolean {
    return tmp117DataReady(config)
  },

  /**
   * Reports whether a configuration register says an EEPROM write is running.
   *
   * @param config - The configuration register.
   * @returns Whether the part is still writing.
   */
  eepromBusy(config: number): boolean {
    return tmp117EepromBusy(config)
  },

  /**
   * Reports whether an EEPROM-unlock register says a write is running.
   *
   * @param unlock - The EEPROM unlock register.
   * @returns Whether the part is still writing.
   */
  eepromUnlockBusy(unlock: number): boolean {
    return tmp117EepromUnlockBusy(unlock)
  },

  /**
   * Assembles the 16-bit configuration register value.
   *
   * @param config - The settings to encode.
   * @returns The register value to write.
   */
  configBits(config: Tmp117Config): number {
    return tmp117ConfigBits(config)
  },

  /**
   * Parses a 16-bit configuration register value.
   *
   * @param bits - The register value, as read from the device.
   * @returns The decoded settings. Every value decodes, so this never throws.
   */
  configFromBits(bits: number): Tmp117Config {
    return tmp117ConfigFromBits(bits)
  },

  /**
   * Returns how many conversions an averaging code folds into one result.
   *
   * @param averaging - The averaging code, 0 to 3.
   * @returns The conversion count: 1, 8, 32, or 64.
   */
  conversions(averaging: number): number {
    return tmp117Conversions(averaging)
  },

  /**
   * Returns how long an averaging code takes to convert.
   *
   * @param averaging - The averaging code, 0 to 3.
   * @returns The active conversion time in microseconds.
   */
  conversionMicros(averaging: number): number {
    return tmp117ConversionMicros(averaging)
  },

  /**
   * Returns the nominal cycle a conversion-cycle code names.
   *
   * @param cycle - The conversion-cycle code, 0 to 7.
   * @returns The cycle with no averaging, in microseconds.
   */
  nominalMicros(cycle: number): number {
    return tmp117NominalMicros(cycle)
  },

  /**
   * Returns the result interval for a cycle and averaging pair.
   *
   * @param cycle - The conversion-cycle code, 0 to 7.
   * @param averaging - The averaging code, 0 to 3.
   * @returns The interval in microseconds, never shorter than the conversions take.
   */
  cycleMicros(cycle: number, averaging: number): number {
    return tmp117CycleMicros(cycle, averaging)
  },

  /** The averaging codes: how many conversions are averaged into one result. */
  averaging: {
    /** No averaging: each result is one 15.5 ms conversion. */
    none: 0,
    /** Eight conversions, the factory setting. */
    x8: 1,
    /** Thirty-two conversions. */
    x32: 2,
    /** Sixty-four conversions. */
    x64: 3,
  },

  /**
   * A TMP117 that is not there, for a bus with nothing plugged in. Its configuration register
   * keeps the flags the part sets for itself whatever a driver writes, with the data-ready
   * flag set, so every conversion reads as finished.
   */
  sim: {
    /** The temperature `part` reports. */
    celsius: 21.25,

    /**
     * A part reading 21.25 C.
     *
     * @param address - The address it answers to.
     * @returns The part, to put on a simulated bus.
     */
    part(address: number): WordPart {
      return tmp117SimPart(address)
    },

    /**
     * A part that reads what it is asked to, to the nearest 7.8125 millidegrees.
     *
     * @param address - The address it answers to.
     * @param celsius - The temperature it reports.
     * @returns The part, to put on a simulated bus.
     */
    reporting(address: number, celsius: number): WordPart {
      return tmp117SimReporting(address, celsius)
    },
  },
}

/** A TI HDC1080 low-power humidity and temperature sensor. */
export const hdc1080 = {
  /** The only address the part answers on. */
  address: 0x40,
  /** The value the manufacturer-ID register reads. */
  manufacturerId: 0x5449,
  /** The value the device-ID register reads, which confirms the part. */
  deviceId: 0x1050,
  /** The value the configuration register reads after a reset. */
  configurationReset: 0x1000,
  /** The registers a driver reads and writes. */
  register: {
    /** The temperature result. */
    temperature: 0x00,
    /** The humidity result. */
    humidity: 0x01,
    /** The configuration register. */
    configuration: 0x02,
    /** The top word of the serial number. */
    serialIdHigh: 0xfb,
    /** The middle word of the serial number. */
    serialIdMid: 0xfc,
    /** The bottom word of the serial number. */
    serialIdLow: 0xfd,
    /** The manufacturer-ID register. */
    manufacturerId: 0xfe,
    /** The device-ID register. */
    deviceId: 0xff,
  },

  /**
   * Converts a raw temperature register to milli-degrees Celsius.
   *
   * @param raw - The 16-bit register.
   * @returns The temperature, exact in integer arithmetic.
   */
  milliCelsius(raw: number): number {
    return hdc1080MilliCelsius(raw)
  },

  /**
   * Converts a raw temperature register to degrees Celsius.
   *
   * @param raw - The 16-bit register.
   * @returns The temperature.
   */
  celsius(raw: number): number {
    return hdc1080Celsius(raw)
  },

  /**
   * Converts a raw humidity register to milli-percent relative humidity.
   *
   * @param raw - The 16-bit register.
   * @returns The humidity, exact in integer arithmetic.
   */
  milliPercent(raw: number): number {
    return hdc1080MilliPercent(raw)
  },

  /**
   * Converts a raw humidity register to a relative humidity percentage.
   *
   * @param raw - The 16-bit register.
   * @returns The humidity.
   */
  relativeHumidity(raw: number): number {
    return hdc1080RelativeHumidity(raw)
  },

  /**
   * Builds the temperature register for a temperature in milli-degrees Celsius.
   *
   * @param milliCelsius - The temperature.
   * @returns The register value, its 14-bit code in bits 15:2 and clamped to the
   * part's range.
   */
  temperatureRegister(milliCelsius: number): number {
    return hdc1080TemperatureRegister(milliCelsius)
  },

  /**
   * Builds the humidity register for a humidity in milli-percent.
   *
   * @param milliPercent - The relative humidity.
   * @returns The register value, its 14-bit code in bits 15:2 and clamped to the
   * part's range.
   */
  humidityRegister(milliPercent: number): number {
    return hdc1080HumidityRegister(milliPercent)
  },

  /**
   * Joins the three serial-ID registers into one serial number.
   *
   * @param high - The top register.
   * @param mid - The middle register.
   * @param low - The bottom register.
   * @returns The 41-bit serial number.
   */
  serialId(high: number, mid: number, low: number): number {
    return Number(hdc1080SerialId(high, mid, low))
  },

  /**
   * Splits a serial number back into its three registers.
   *
   * @param serial - The 41-bit serial number.
   * @returns The high, middle, and low registers, in that order.
   */
  serialIdRegisters(serial: number): number[] {
    return hdc1080SerialIdRegisters(serial)
  },

  /**
   * Parses a four-byte measurement frame.
   *
   * The part has no checksum, so every frame decodes.
   *
   * @param bytes - The temperature and humidity registers, each most significant
   * byte first.
   * @returns The raw registers and the physical values they carry.
   * @throws If the read is not four bytes.
   */
  parseMeasurement(bytes: Uint8Array): Hdc1080Measurement {
    return hdc1080ParseMeasurement(Buffer.from(bytes))
  },

  /**
   * Builds the four bytes the part sends for a pair of raw registers.
   *
   * The inverse of {@link parseMeasurement}.
   *
   * @param temperature - The raw temperature register.
   * @param humidity - The raw humidity register.
   * @returns The frame in read order.
   */
  measurementBytes(temperature: number, humidity: number): Buffer {
    return hdc1080MeasurementBytes(temperature, humidity)
  },

  /**
   * Builds the measurement a part reporting these physical values would send.
   *
   * @param milliCelsius - The temperature in milli-degrees Celsius.
   * @param milliPercent - The relative humidity in milli-percent.
   * @returns The measurement, with the raw registers the part would carry.
   */
  measurementFromPhysical(milliCelsius: number, milliPercent: number): Hdc1080Measurement {
    return hdc1080MeasurementFromPhysical(milliCelsius, milliPercent)
  },

  /**
   * Parses the configuration register.
   *
   * @param raw - The register value, as read from the device.
   * @returns The decoded settings.
   * @throws If the humidity-resolution field holds the code the datasheet leaves
   * undefined, which means the value did not come from a working part.
   */
  configFromRegister(raw: number): Hdc1080Config {
    return hdc1080ConfigFromRegister(raw)
  },

  /**
   * Assembles the configuration register value.
   *
   * @param config - The settings to encode.
   * @returns The register value to write.
   * @throws If either resolution is not one the part offers.
   */
  configToRegister(config: Hdc1080Config): number {
    return hdc1080ConfigToRegister(config)
  },

  /**
   * Returns how long to wait after a trigger before the result is readable.
   *
   * @param config - The settings in force.
   * @returns The conversion time in microseconds.
   * @throws If either resolution is not one the part offers.
   */
  conversionTimeMicros(config: Hdc1080Config): number {
    return hdc1080ConversionTimeMicros(config)
  },

  /** An HDC1080 that is not there, for a bus with nothing plugged in. */
  sim: {
    /** The temperature `part` reports. */
    celsius: 22.5,
    /** The relative humidity `part` reports, as a percentage. */
    relativeHumidity: 45.0,

    /**
     * A part reading 22.5 C and 45 %, at the one address an HDC1080 has.
     *
     * @returns The part, to put on a simulated bus.
     */
    part(): WordPart {
      return hdc1080SimPart()
    },

    /**
     * A part that reads what it is asked to, within three thousandths of a degree and two
     * thousandths of a percent.
     *
     * @param celsius - The temperature it reports.
     * @param relativeHumidity - The humidity it reports, as a percentage.
     * @returns The part, to put on a simulated bus.
     */
    reporting(celsius: number, relativeHumidity: number): WordPart {
      return hdc1080SimReporting(celsius, relativeHumidity)
    },
  },
}

/** A TI OPT3001 ambient light sensor with a human-eye response. */
export const opt3001 = {
  /** The address the part answers on with ADDR tied to GND. */
  addressGnd: 0x44,
  /** The address it answers on with ADDR tied to VDD. */
  addressVdd: 0x45,
  /** The address it answers on with ADDR tied to SDA. */
  addressSda: 0x46,
  /** The address it answers on with ADDR tied to SCL. */
  addressScl: 0x47,
  /** The value the manufacturer-ID register reads. */
  manufacturerId: 0x5449,
  /** The value the device-ID register reads, which confirms the part. */
  deviceId: 0x3001,
  /** The value the configuration register reads after a reset. */
  configurationReset: 0xc810,
  /** The value the low-limit register holds after a reset. */
  lowLimitReset: 0x0000,
  /** The value the high-limit register holds after a reset. */
  highLimitReset: 0xbfff,
  /** The low-limit value that turns the INT pin into an end-of-conversion signal. */
  lowLimitEndOfConversion: 0xc000,
  /** The range number that lets the part choose its own full scale. */
  rangeAutomatic: 0b1100,
  /** The highest fixed range number. */
  rangeMax: 11,
  /** The registers a driver reads and writes. */
  register: {
    /** The result register. */
    result: 0x00,
    /** The configuration register. */
    configuration: 0x01,
    /** The low limit. */
    lowLimit: 0x02,
    /** The high limit. */
    highLimit: 0x03,
    /** The manufacturer-ID register. */
    manufacturerId: 0x7e,
    /** The device-ID register. */
    deviceId: 0x7f,
  },

  /**
   * Returns the illuminance one count carries at a given exponent.
   *
   * @param exponent - The result's exponent field, 0 to 11.
   * @returns The step in milli-lux, or `null` for a reserved exponent.
   */
  lsbMilliLux(exponent: number): number | null {
    return opt3001LsbMilliLux(exponent)
  },

  /**
   * Returns the full scale a range number covers.
   *
   * @param rangeNumber - The range, 0 to 11.
   * @returns The full scale in milli-lux, or `null` for a reserved range.
   */
  fullScaleMilliLux(rangeNumber: number): number | null {
    return opt3001FullScaleMilliLux(rangeNumber)
  },

  /**
   * Converts a raw result register to milli-lux.
   *
   * @param raw - The register value, exponent and mantissa together.
   * @returns The illuminance, exact in integer arithmetic.
   */
  milliLux(raw: number): number {
    return opt3001MilliLux(raw)
  },

  /**
   * Converts a raw result register to lux.
   *
   * @param raw - The register value.
   * @returns The illuminance.
   */
  lux(raw: number): number {
    return opt3001Lux(raw)
  },

  /**
   * Builds the result register that reports an illuminance.
   *
   * The inverse of {@link milliLux}, using the smallest exponent that fits and
   * truncating the mantissa the way the part does.
   *
   * @param milliLux - The illuminance.
   * @returns The register value, saturating at the part's full scale.
   */
  rawFromMilliLux(milliLux: number): number {
    return opt3001RawFromMilliLux(milliLux)
  },

  /**
   * Reads a register out of the two bytes the part sends.
   *
   * @param bytes - The register, most significant byte first.
   * @returns The register value.
   * @throws If the read is not two bytes.
   */
  wordFromBytes(bytes: Uint8Array): number {
    return opt3001WordFromBytes(Buffer.from(bytes))
  },

  /**
   * Builds the two bytes a register is written as.
   *
   * @param word - The register value.
   * @returns The register, most significant byte first.
   */
  wordToBytes(word: number): Buffer {
    return opt3001WordToBytes(word)
  },

  /**
   * Assembles the 16-bit configuration register value.
   *
   * The read-only status fields are written as zero.
   *
   * @param config - The settings to encode.
   * @returns The register value to write.
   * @throws If the conversion time or fault count is not one the part offers.
   */
  configBits(config: Opt3001Config): number {
    return opt3001ConfigBits(config)
  },

  /**
   * Parses a 16-bit configuration register value.
   *
   * @param bits - The register value, as read from the device.
   * @returns The decoded settings, status flags included.
   */
  configFromBits(bits: number): Opt3001Config {
    return opt3001ConfigFromBits(bits)
  },

  /**
   * Reports whether a range number lets the part choose its own full scale.
   *
   * @param rangeNumber - The range field of the configuration register.
   * @returns Whether the range is set to automatic.
   */
  isAutomaticRange(rangeNumber: number): boolean {
    return opt3001IsAutomaticRange(rangeNumber)
  },

  /**
   * An OPT3001 that is not there, for a bus with nothing plugged in. Its configuration
   * register keeps the flags the part sets for itself whatever a driver writes, with the
   * conversion-ready flag set, so every conversion reads as finished.
   */
  sim: {
    /** The illuminance `part` reports. */
    lux: 380.0,

    /**
     * A part reading 380 lux.
     *
     * @param address - The address it answers to.
     * @returns The part, to put on a simulated bus.
     */
    part(address: number): WordPart {
      return opt3001SimPart(address)
    },

    /**
     * A part that reads what it is asked to, to the nearest step its exponent and mantissa
     * represent.
     *
     * @param address - The address it answers to.
     * @param lux - The illuminance it reports.
     * @returns The part, to put on a simulated bus.
     */
    reporting(address: number, lux: number): WordPart {
      return opt3001SimReporting(address, lux)
    },
  },
}

/** A TI INA226 current, voltage, and power monitor with an alert pin. */
export const ina226 = {
  /** The address with both A1 and A0 tied to GND. */
  baseAddress: 0x40,
  /** The value the manufacturer-ID register reads. */
  manufacturerId: 0x5449,
  /** The device identifier the die-ID register carries for an INA226. */
  deviceId: 0x226,
  /** The value the configuration register reads after a reset. */
  configReset: 0x4127,
  /** The shunt voltage one count carries, in nanovolts. */
  shuntLsbNanovolts: 2500,
  /** The bus voltage one count carries, in microvolts. */
  busLsbMicrovolts: 1250,
  /** How many times the power LSB is the current LSB. */
  powerLsbRatio: 25,
  /** What an address pin is tied to, as the code the address helper takes. */
  pin: {
    /** Tied to GND. */
    ground: 0,
    /** Tied to VS. */
    supply: 1,
    /** Tied to SDA. */
    sda: 2,
    /** Tied to SCL. */
    scl: 3,
  },
  /** The registers a driver reads and writes. */
  register: {
    /** The configuration register. */
    configuration: 0x00,
    /** The shunt voltage result. */
    shuntVoltage: 0x01,
    /** The bus voltage result. */
    busVoltage: 0x02,
    /** The power result. */
    power: 0x03,
    /** The current result. */
    current: 0x04,
    /** The calibration register. */
    calibration: 0x05,
    /** The Mask/Enable register. */
    maskEnable: 0x06,
    /** The alert limit. */
    alertLimit: 0x07,
    /** The manufacturer-ID register. */
    manufacturerId: 0xfe,
    /** The die-ID register. */
    dieId: 0xff,
  },

  /**
   * Returns the 7-bit address the A1 and A0 pins select.
   *
   * @param a1 - What the A1 pin is tied to, one of {@link pin}.
   * @param a0 - What the A0 pin is tied to.
   * @returns The address, 0x40 to 0x4F.
   * @throws If either pin code is not 0, 1, 2, or 3.
   */
  address(a1: number, a0: number): number {
    return ina226Address(a1, a0)
  },

  /**
   * Returns how many samples an averaging code folds into one result.
   *
   * @param averaging - The averaging code, 0 to 7.
   * @returns The sample count, 1 to 1024.
   */
  averagingSamples(averaging: number): number {
    return ina226AveragingSamples(averaging)
  },

  /**
   * Returns how long a conversion-time code takes.
   *
   * @param conversionTime - The conversion-time code, 0 to 7.
   * @returns The typical conversion time in microseconds.
   */
  conversionMicros(conversionTime: number): number {
    return ina226ConversionMicros(conversionTime)
  },

  /**
   * Reports whether a mode code converts the shunt voltage.
   *
   * @param mode - The mode code, 0 to 7.
   * @returns Whether the shunt input is measured.
   */
  modeMeasuresShunt(mode: number): boolean {
    return ina226ModeMeasuresShunt(mode)
  },

  /**
   * Reports whether a mode code converts the bus voltage.
   *
   * @param mode - The mode code, 0 to 7.
   * @returns Whether the bus input is measured.
   */
  modeMeasuresBus(mode: number): boolean {
    return ina226ModeMeasuresBus(mode)
  },

  /**
   * Reports whether a mode code keeps converting after the first result.
   *
   * @param mode - The mode code, 0 to 7.
   * @returns Whether the mode is one of the three continuous ones.
   */
  modeIsContinuous(mode: number): boolean {
    return ina226ModeIsContinuous(mode)
  },

  /**
   * Parses the configuration register.
   *
   * @param raw - The register value, as read from the device.
   * @returns The decoded settings; reserved bits are ignored.
   */
  configFromRegister(raw: number): Ina226Config {
    return ina226ConfigFromRegister(raw)
  },

  /**
   * Assembles the configuration register value.
   *
   * @param config - The settings to encode.
   * @returns The register value to write, with reserved bit 14 set as at reset.
   */
  configToRegister(config: Ina226Config): number {
    return ina226ConfigToRegister(config)
  },

  /**
   * Returns how often the result registers update with these settings.
   *
   * @param config - The settings in force.
   * @returns The update interval in microseconds, or `0` in power-down.
   */
  updateMicroseconds(config: Ina226Config): number {
    return ina226UpdateMicroseconds(config)
  },

  /**
   * Parses the Mask/Enable register.
   *
   * @param raw - The register value, as read from the device.
   * @returns The decoded enables and flags.
   */
  maskEnableFromRegister(raw: number): Ina226MaskEnable {
    return ina226MaskEnableFromRegister(raw)
  },

  /**
   * Assembles the Mask/Enable register value.
   *
   * @param mask - The enables and flags to encode.
   * @returns The register value to write.
   */
  maskEnableToRegister(mask: Ina226MaskEnable): number {
    return ina226MaskEnableToRegister(mask)
  },

  /**
   * Returns the limit comparison the alert pin actually responds to.
   *
   * Only one limit function drives the pin at a time; with several enabled the
   * part honors the most significant bit.
   *
   * @param mask - The enables in force.
   * @returns The highest-priority enabled function, or `null` if none is enabled.
   */
  activeAlertFunction(mask: Ina226MaskEnable): Ina226AlertFunction | null {
    return ina226ActiveAlertFunction(mask)
  },

  /**
   * Splits a die-ID register into its device and revision fields.
   *
   * @param raw - The die-ID register.
   * @returns The 12-bit device identifier and the 4-bit revision.
   */
  dieId(raw: number): Ina226DieIdFields {
    return ina226DieId(raw)
  },

  /**
   * Checks that a pair of identification registers belongs to an INA226.
   *
   * A node reads both at start-up, so a wrong part on the address, or a bus that
   * reads all ones, is caught before its readings are trusted.
   *
   * @param manufacturerId - The manufacturer-ID register.
   * @param dieId - The die-ID register.
   * @returns The decoded die ID.
   * @throws If either register does not carry the value the datasheet fixes.
   */
  identify(manufacturerId: number, dieId: number): Ina226DieIdFields {
    return ina226Identify(manufacturerId, dieId)
  },

  /**
   * Computes the calibration register for a shunt and current resolution.
   *
   * @param currentLsbMicroamps - The microamps per count the current register
   * should carry.
   * @param shuntMilliohms - The shunt resistor value.
   * @returns The register value to write.
   */
  calibration(currentLsbMicroamps: number, shuntMilliohms: number): number {
    return ina226Calibration(currentLsbMicroamps, shuntMilliohms)
  },

  /**
   * Returns the smallest current resolution that still covers a maximum.
   *
   * @param maxExpectedMicroamps - The largest current the application measures.
   * @returns The minimum current LSB in microamps.
   */
  minimumCurrentLsbMicroamps(maxExpectedMicroamps: number): number {
    return ina226MinimumCurrentLsbMicroamps(maxExpectedMicroamps)
  },

  /**
   * Converts a raw shunt-voltage register to nanovolts.
   *
   * @param raw - The signed register value.
   * @returns The shunt voltage, at 2.5 uV per count.
   */
  shuntNanovolts(raw: number): number {
    return ina226ShuntNanovolts(raw)
  },

  /**
   * Converts a raw shunt-voltage register to millivolts.
   *
   * @param raw - The signed register value.
   * @returns The shunt voltage.
   */
  shuntMillivolts(raw: number): number {
    return ina226ShuntMillivolts(raw)
  },

  /**
   * Converts a raw bus-voltage register to microvolts.
   *
   * @param raw - The register value.
   * @returns The bus voltage, at 1.25 mV per count.
   */
  busMicrovolts(raw: number): number {
    return ina226BusMicrovolts(raw)
  },

  /**
   * Converts a raw bus-voltage register to volts.
   *
   * @param raw - The register value.
   * @returns The bus voltage.
   */
  busVolts(raw: number): number {
    return ina226BusVolts(raw)
  },

  /**
   * Converts a raw current register to microamps.
   *
   * @param raw - The signed register value.
   * @param currentLsbMicroamps - The resolution the calibration selected.
   * @returns The current.
   */
  currentMicroamps(raw: number, currentLsbMicroamps: number): number {
    return ina226CurrentMicroamps(raw, currentLsbMicroamps)
  },

  /**
   * Converts a raw current register to amps.
   *
   * @param raw - The signed register value.
   * @param currentLsbMicroamps - The resolution the calibration selected.
   * @returns The current.
   */
  currentAmps(raw: number, currentLsbMicroamps: number): number {
    return ina226CurrentAmps(raw, currentLsbMicroamps)
  },

  /**
   * Converts a raw power register to microwatts.
   *
   * @param raw - The register value.
   * @param currentLsbMicroamps - The resolution the calibration selected.
   * @returns The power. The power LSB is fixed at 25 times the current LSB.
   */
  powerMicrowatts(raw: number, currentLsbMicroamps: number): number {
    return ina226PowerMicrowatts(raw, currentLsbMicroamps)
  },

  /**
   * Converts a raw power register to watts.
   *
   * @param raw - The register value.
   * @param currentLsbMicroamps - The resolution the calibration selected.
   * @returns The power.
   */
  powerWatts(raw: number, currentLsbMicroamps: number): number {
    return ina226PowerWatts(raw, currentLsbMicroamps)
  },

  /**
   * Builds the shunt-voltage register a monitor reports for a shunt voltage.
   *
   * The inverse of {@link shuntNanovolts}, so a node can be written and tested
   * against what a monitor sends without one attached.
   *
   * @param nanovolts - The shunt voltage.
   * @returns The signed register value.
   */
  shuntRegister(nanovolts: number): number {
    return ina226ShuntRegister(nanovolts)
  },

  /**
   * Builds the bus-voltage register a monitor reports for a bus voltage.
   *
   * @param microvolts - The bus voltage.
   * @returns The register value.
   */
  busRegister(microvolts: number): number {
    return ina226BusRegister(microvolts)
  },

  /**
   * Builds the current register a monitor reports for a current.
   *
   * @param microamps - The current.
   * @param currentLsbMicroamps - The current LSB the calibration was set for.
   * @returns The signed register value.
   */
  currentRegister(microamps: number, currentLsbMicroamps: number): number {
    return ina226CurrentRegister(microamps, currentLsbMicroamps)
  },

  /**
   * Builds the power register a monitor reports for a power.
   *
   * @param microwatts - The power.
   * @param currentLsbMicroamps - The current LSB the calibration was set for.
   * @returns The register value.
   */
  powerRegister(microwatts: number, currentLsbMicroamps: number): number {
    return ina226PowerRegister(microwatts, currentLsbMicroamps)
  },

  /**
   * Computes the current register the part derives from a shunt reading.
   *
   * @param shunt - The signed shunt-voltage register.
   * @param calibration - The calibration register in force.
   * @returns The signed current register.
   */
  currentRegisterFromShunt(shunt: number, calibration: number): number {
    return ina226CurrentRegisterFromShunt(shunt, calibration)
  },

  /**
   * Computes the power register the part derives from a current and a bus voltage.
   *
   * @param current - The signed current register.
   * @param bus - The bus-voltage register.
   * @returns The power register.
   */
  powerRegisterFromCurrent(current: number, bus: number): number {
    return ina226PowerRegisterFromCurrent(current, bus)
  },

  /**
   * An INA226 that is not there, for a bus with nothing plugged in. It carries TI's
   * manufacturer id and the INA226 die id, its conversion-ready flag is set, and `reporting`
   * takes the same shunt and largest current a driver is given.
   */
  sim: {
    /** The shunt `part` sits across, in milliohms. */
    shuntMilliohms: 100,
    /** The largest current `part` is sized for, in microamps. */
    maxMicroamps: 3_200_000,
    /** The bus voltage `part` reports, in microvolts. */
    busMicrovolts: 12_000_000,
    /** The current `part` reports, in microamps. */
    microamps: 500_000,

    /**
     * A part carrying 500 mA at 12 V through the shunt a driver starts with.
     *
     * @param address - The address it answers to.
     * @returns The part, to put on a simulated bus.
     */
    part(address: number): WordPart {
      return ina226SimPart(address)
    },

    /**
     * A part that reads what it is asked to, on the steps its registers count in: 1.25 mV of
     * bus, 2.5 uV of shunt, and the calibration's current step.
     *
     * @param address - The address it answers to.
     * @param shuntMilliohms - The shunt, as the driver is given it.
     * @param maxMicroamps - The largest current, as the driver is given it.
     * @param busMicrovolts - The bus voltage it reports.
     * @param microamps - The current it reports; negative flows the other way.
     * @returns The part, to put on a simulated bus.
     */
    reporting(
      address: number,
      shuntMilliohms: number,
      maxMicroamps: number,
      busMicrovolts: number,
      microamps: number,
    ): WordPart {
      return ina226SimReporting(address, shuntMilliohms, maxMicroamps, busMicrovolts, microamps)
    },
  },
}
