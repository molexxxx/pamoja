/**
 * Ergonomic facade over the generated sensor-driver binding.
 *
 * These are the decode half of four parts a field node is likely to have wired
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
  ds18b20Celsius,
  ds18b20ConfigByte,
  ds18b20Crc8,
  ds18b20MaxConversionMicros,
  ds18b20MicroCelsius,
  ds18b20ParseScratchpad,
  ds18b20ResolutionBits,
  ds18b20StepMicroCelsius,
  type Ds18b20Reading,
  ina219BusMillivolts,
  ina219Calibration,
  ina219ConversionReady,
  ina219CurrentMicroamps,
  ina219MathOverflow,
  ina219MinimumCurrentLsbMicroamps,
  ina219PowerMicrowatts,
  ina219ShuntMicrovolts,
} from '../index'

export { Bme280Calibration, type Ads1115Config, type Bme280Measurement, type Ds18b20Reading }

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
