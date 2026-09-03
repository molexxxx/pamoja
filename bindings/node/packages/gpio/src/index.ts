/**
 * Ergonomic facade over the generated on-board bus binding.
 *
 * Before a node reaches any network it talks to the chips wired to its own board.
 * Three interfaces cover almost everything cheap hardware uses, and each carries
 * one small piece of logic that is a classic field bug when it is wrong: the I2C
 * address byte, the SPI clock mode, and whether a relay is active high or active
 * low.
 *
 * @packageDocumentation
 */

import {
  i2cAddressFrame,
  i2cAddressFrameLen,
  i2cAddressIsGeneralCall,
  i2cAddressIsReserved,
  pinEdgeTriggeredBy,
  type PinEdge as PinEdgeName,
  pinLevelFromBool,
  pinLevelInverted,
  type PinLevel as PinLevelName,
  pinPolarityIsAsserted,
  pinPolarityLevel,
  type PinPolarity as PinPolarityName,
  type SpiClock,
  spiModeClock,
  spiModeFromClock,
} from '@pamoja/native'

export { type SpiClock }

/** The physical voltage level on a pin. */
export const PinLevel = {
  /** A low level, near ground. */
  Low: 'Low',
  /** A high level, near the supply voltage. */
  High: 'High',
} as const

/** The physical voltage level on a pin. */
export type PinLevel = PinLevelName

/** The signal transition that triggers a pin interrupt. */
export const PinEdge = {
  /** A low-to-high transition. */
  Rising: 'Rising',
  /** A high-to-low transition. */
  Falling: 'Falling',
  /** Either transition. */
  Both: 'Both',
} as const

/** The signal transition that triggers a pin interrupt. */
export type PinEdge = PinEdgeName

/** Whether a signal is asserted by a high or a low physical level. */
export const PinPolarity = {
  /** A high level means asserted. */
  ActiveHigh: 'ActiveHigh',
  /** A low level means asserted, the wiring of most buttons and relay boards. */
  ActiveLow: 'ActiveLow',
} as const

/** Whether a signal is asserted by a high or a low physical level. */
export type PinPolarity = PinPolarityName

/** I2C addressing per the NXP I2C-bus specification (UM10204). */
export const i2c = {
  /**
   * Returns the address bytes a controller puts on the bus for a transfer.
   *
   * @param address - The device address.
   * @param options - `read` selects the direction, `tenBit` the address width.
   * @returns One byte for a 7-bit address, two for a 10-bit one.
   * @throws If the address is outside its width's range.
   */
  addressFrame(
    address: number,
    options: { read?: boolean; tenBit?: boolean } = {},
  ): Buffer {
    return i2cAddressFrame(address, options.tenBit ?? false, options.read ?? false)
  },

  /**
   * Returns how many bytes an address frame occupies.
   *
   * @param address - The device address.
   * @param tenBit - Whether it is a 10-bit address.
   * @returns `1` for a 7-bit address, `2` for a 10-bit one.
   * @throws If the address is outside its width's range.
   */
  frameLen(address: number, tenBit = false): number {
    return i2cAddressFrameLen(address, tenBit)
  },

  /**
   * Reports whether an address falls in a range the specification reserves.
   *
   * UM10204 reserves `0x00..=0x07` and `0x78..=0x7F`, leaving `0x08..=0x77` for
   * ordinary devices.
   *
   * @param address - The device address.
   * @param tenBit - Whether it is a 10-bit address, which is never reserved.
   * @returns Whether the address is reserved.
   * @throws If the address is outside its width's range.
   */
  isReserved(address: number, tenBit = false): boolean {
    return i2cAddressIsReserved(address, tenBit)
  },

  /**
   * Reports whether an address is the general call address `0x00`.
   *
   * @param address - The device address.
   * @param tenBit - Whether it is a 10-bit address.
   * @returns Whether this is the broadcast every device listens to.
   * @throws If the address is outside its width's range.
   */
  isGeneralCall(address: number, tenBit = false): boolean {
    return i2cAddressIsGeneralCall(address, tenBit)
  },
}

/** The four SPI clock modes, as the `(CPOL, CPHA)` pair datasheets quote. */
export const spi = {
  /**
   * Returns the clock polarity and phase a mode number names.
   *
   * @param mode - The mode number, 0 to 3.
   * @returns The pair.
   * @throws If the mode number is above 3.
   */
  clockFor(mode: number): SpiClock {
    return spiModeClock(mode)
  },

  /**
   * Returns the mode number a clock polarity and phase name.
   *
   * @param cpol - Whether the clock idles high.
   * @param cpha - Whether data is sampled on the trailing edge.
   * @returns The mode number, 0 to 3. Every pair names a mode.
   */
  modeFor(cpol: boolean, cpha: boolean): number {
    return spiModeFromClock(cpol, cpha)
  },
}

/** The GPIO pin model: levels, interrupt edges, and active polarity. */
export const pin = {
  /**
   * Returns the level a boolean names.
   *
   * @param high - `true` for high, `false` for low.
   * @returns The level.
   */
  levelFrom(high: boolean): PinLevel {
    return pinLevelFromBool(high)
  },

  /**
   * Returns the opposite level.
   *
   * @param level - The level to invert.
   * @returns The other level.
   */
  invert(level: PinLevel): PinLevel {
    return pinLevelInverted(level)
  },

  /**
   * Reports whether a transition fires an interrupt trigger.
   *
   * @param edge - The trigger configured on the pin.
   * @param from - The level before the change.
   * @param to - The level after it.
   * @returns Whether the trigger fires.
   */
  triggers(edge: PinEdge, from: PinLevel, to: PinLevel): boolean {
    return pinEdgeTriggeredBy(edge, from, to)
  },

  /**
   * Returns the physical level that represents a logical state.
   *
   * @param polarity - How the signal is wired.
   * @param asserted - Whether the signal should be asserted.
   * @returns The level to drive, inverted for active-low wiring.
   */
  levelFor(polarity: PinPolarity, asserted: boolean): PinLevel {
    return pinPolarityLevel(polarity, asserted)
  },

  /**
   * Reports whether a physical level means the signal is asserted.
   *
   * @param polarity - How the signal is wired.
   * @param level - The level read on the pin.
   * @returns Whether the signal is asserted.
   */
  isAsserted(polarity: PinPolarity, level: PinLevel): boolean {
    return pinPolarityIsAsserted(polarity, level)
  },
}
