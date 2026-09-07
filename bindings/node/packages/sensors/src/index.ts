/**
 * Ergonomic facade over the generated sensor-driver binding.
 *
 * These are the decode half of eleven parts a field node is likely to have wired
 * to it, turning the register bytes a bus driver read into the physical reading
 * the manufacturer's datasheet says they mean. Driving the bus is the caller's
 * job; getting the arithmetic right is this layer's.
 *
 * @packageDocumentation
 */

import {
  ads1115ConfigBits,
  ads1115ConfigFromBits,
  ads1115FullScaleMicrovolts,
  ads1115SamplesPerSecond,
  ads1115ToNanovolts,
  ads1115ToVolts,
  type Ads1115Config,
  Bme280Calibration,
  type Bme280Measurement,
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
  type Bmp280RawMeasurement,
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
  tmp117ConversionMicros,
  tmp117Conversions,
  tmp117CycleMicros,
  tmp117DataReady,
  tmp117DeviceId,
  tmp117EepromBusy,
  tmp117EepromUnlockBusy,
  tmp117HighAlert,
  tmp117LowAlert,
  tmp117MicroCelsius,
  tmp117NanoCelsius,
  tmp117NominalMicros,
  tmp117RawFromCelsius,
  tmp117RawFromMicroCelsius,
  tmp117Revision,
  tmp117TemperatureBytes,
  tmp117TemperatureFromBytes,
  type Tmp117Config,
  hdc1080Celsius,
  hdc1080ConfigFromRegister,
  hdc1080ConfigToRegister,
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
  type Hdc1080Config,
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
  type Opt3001Config,
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
  ina226ModeIsContinuous,
  ina226ModeMeasuresBus,
  ina226ModeMeasuresShunt,
  ina226PowerMicrowatts,
  ina226PowerRegister,
  ina226PowerRegisterFromCurrent,
  ina226PowerWatts,
  ina226ShuntMillivolts,
  ina226ShuntNanovolts,
  ina226ShuntRegister,
  ina226UpdateMicroseconds,
  type Ina226AlertFunction as Ina226AlertFunctionName,
  type Ina226Config,
  type Ina226DieId as Ina226DieIdFields,
  type Ina226MaskEnable,
} from '@pamoja/native'

export {
  Bme280Calibration,
  Bmp280Calibration,
  type Ads1115Config,
  type Bme280Measurement,
  type Bmp280Config,
  type Bmp280CtrlMeas,
  type Bmp280RawMeasurement,
  type Bmp280Reading,
  type Ds18b20Reading,
  type Hdc1080Config,
  type Hdc1080Measurement,
  type Ina226Config,
  type Ina226DieIdFields,
  type Ina226MaskEnable,
  type Opt3001Config,
  type Scd4xMeasurement,
  type Sht3xMeasurement,
  type Sht3xStatus,
  type Tmp117Config,
}

/** A Bosch BME280 temperature, pressure, and humidity sensor. */
export const bme280 = {
  /** The address a BME280 answers on with its SDO pin low. */
  addressPrimary: 0x76,
  /** The address it answers on with SDO high. */
  addressSecondary: 0x77,
  /** The value its chip-ID register reads, which confirms the part. */
  chipId: 0x60,

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
}

/** A TI INA219 current, voltage, and power monitor. */
export const ina219 = {
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
}

/** A TI ADS1115 16-bit analogue-to-digital converter. */
export const ads1115 = {
  /** The value the configuration register reads after a reset. */
  configReset: 0x8583,

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
   * part honours the most significant bit.
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
}
