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
  GpioLine,
  I2C_RESERVED_BELOW,
  I2C_RESERVED_FROM,
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

/**
 * A GPIO line opened on a Linux board, through the kernel's GPIO character device: the
 * line a {@link Switch} or a {@link Contact} sits over on a Raspberry Pi or any Linux
 * board.
 *
 * `GpioLine.openOutput(chip, line, initial)` drives `initial` from the moment the line is
 * taken, so an active-low relay is opened `PinLevel.High` and stays off;
 * `GpioLine.openInput(chip, line)` opens one to read. The chip is a device file such as
 * `/dev/gpiochip0`, and the line is the GPIO or BCM number a Raspberry Pi pinout gives.
 * A line is held by one process at a time and `close()` hands it back. On Raspberry Pi
 * OS a user in the `gpio` group opens lines without root. Opening throws on any platform
 * but Linux, and names the chip and the line when either cannot be opened.
 */
export { GpioLine }

/** The physical voltage level on a pin. */
export const PinLevel = {
  /** A low level, near ground. */
  Low: 'Low' as PinLevelName,
  /** A high level, near the supply voltage. */
  High: 'High' as PinLevelName,
} as const

/** The physical voltage level on a pin. */
export type PinLevel = PinLevelName

/** The signal transition that triggers a pin interrupt. */
export const PinEdge = {
  /** A low-to-high transition. */
  Rising: 'Rising' as PinEdgeName,
  /** A high-to-low transition. */
  Falling: 'Falling' as PinEdgeName,
  /** Either transition. */
  Both: 'Both' as PinEdgeName,
} as const

/** The signal transition that triggers a pin interrupt. */
export type PinEdge = PinEdgeName

/** Whether a signal is asserted by a high or a low physical level. */
export const PinPolarity = {
  /** A high level means asserted. */
  ActiveHigh: 'ActiveHigh' as PinPolarityName,
  /** A low level means asserted, the wiring of most buttons and relay boards. */
  ActiveLow: 'ActiveLow' as PinPolarityName,
} as const

/** Whether a signal is asserted by a high or a low physical level. */
export type PinPolarity = PinPolarityName

/** I2C addressing per the NXP I2C-bus specification (UM10204). */
export const i2c = {
  /** The lowest 7-bit address the specification keeps for itself. */
  RESERVED_FROM: I2C_RESERVED_FROM,

  /** The first 7-bit address above the reserved block at the bottom of the range. */
  RESERVED_BELOW: I2C_RESERVED_BELOW,

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

/**
 * A line a pin library drives: `onoff` or `rpio` on a Raspberry Pi, a vendor SDK on a
 * microcontroller, or a {@link PinScript} in a test. Anything with this one method
 * can sit under a {@link Switch}.
 */
export interface OutputLine {
  /**
   * Drives the line to a level.
   *
   * @param level - The level to drive.
   */
  drive(level: PinLevel): void
}

/**
 * A line a pin library reads, which a {@link Contact} sits over.
 */
export interface InputLine {
  /**
   * Reads the line's level now.
   *
   * @returns The level on the line.
   */
  read(): PinLevel
}

/**
 * A two-state output over any line, with its polarity said once: a relay, an LED, a
 * solenoid valve, a buzzer. `set(true)` asserts it, which drives the line low for an
 * active-low part, so no call site inverts a level by hand.
 *
 * @example
 * ```ts
 * const pump = Switch.activeLow(new PinScript())
 * pump.set(true)
 * pump.isAsserted // true, and the line was driven low
 * ```
 */
export class Switch<L extends OutputLine = OutputLine> {
  #line: L
  #polarity: PinPolarity
  #asserted = false

  /**
   * Wraps a line, starting deasserted. Nothing is driven until {@link Switch.set}.
   *
   * @param line - The line the part is wired to.
   * @param polarity - How the part is wired.
   */
  constructor(line: L, polarity: PinPolarity) {
    this.#line = line
    this.#polarity = polarity
  }

  /**
   * A switch whose part is asserted by a high level.
   *
   * @param line - The line the part is wired to.
   * @returns The switch.
   */
  static activeHigh<L extends OutputLine>(line: L): Switch<L> {
    return new Switch(line, PinPolarity.ActiveHigh)
  }

  /**
   * A switch whose part is asserted by a low level, the wiring of most relay boards.
   *
   * @param line - The line the part is wired to.
   * @returns The switch.
   */
  static activeLow<L extends OutputLine>(line: L): Switch<L> {
    return new Switch(line, PinPolarity.ActiveLow)
  }

  /** How the part is wired. */
  get polarity(): PinPolarity {
    return this.#polarity
  }

  /** Whether the part was last set on. */
  get isAsserted(): boolean {
    return this.#asserted
  }

  /**
   * Turns the part on or off, driving whichever level that means for its wiring.
   *
   * @param asserted - `true` to turn it on.
   * @throws Whatever the line throws when it cannot be driven.
   */
  set(asserted: boolean): void {
    this.#line.drive(pin.levelFor(this.#polarity, asserted))
    this.#asserted = asserted
  }

  /**
   * Hands the line back, for a test to read what was driven or a program to reuse it.
   *
   * @returns The line.
   */
  release(): L {
    return this.#line
  }
}

/**
 * A two-state input over any line, with its polarity said once: a button, a float
 * switch, a reed switch, a limit switch. {@link Contact.isAsserted} answers whether it
 * is closed, pressed, or tripped, whatever level that takes on the wire.
 *
 * @example
 * ```ts
 * const float = Contact.activeLow(new PinScript([PinLevel.Low]))
 * float.isAsserted() // true: a switch to ground reads low when closed
 * ```
 */
export class Contact<L extends InputLine = InputLine> {
  #line: L
  #polarity: PinPolarity

  /**
   * Wraps a line.
   *
   * @param line - The line the part is wired to.
   * @param polarity - How the part is wired.
   */
  constructor(line: L, polarity: PinPolarity) {
    this.#line = line
    this.#polarity = polarity
  }

  /**
   * A contact that reads high when asserted.
   *
   * @param line - The line the part is wired to.
   * @returns The contact.
   */
  static activeHigh<L extends InputLine>(line: L): Contact<L> {
    return new Contact(line, PinPolarity.ActiveHigh)
  }

  /**
   * A contact that reads low when asserted, the wiring of a switch to ground with a
   * pull-up.
   *
   * @param line - The line the part is wired to.
   * @returns The contact.
   */
  static activeLow<L extends InputLine>(line: L): Contact<L> {
    return new Contact(line, PinPolarity.ActiveLow)
  }

  /** How the part is wired. */
  get polarity(): PinPolarity {
    return this.#polarity
  }

  /**
   * Reads the raw level on the line.
   *
   * @returns The level.
   * @throws Whatever the line throws when it cannot be read.
   */
  level(): PinLevel {
    return this.#line.read()
  }

  /**
   * Reads the line and reports whether the part is asserted.
   *
   * @returns Whether it is closed, pressed, or tripped.
   * @throws Whatever the line throws when it cannot be read.
   */
  isAsserted(): boolean {
    return pin.isAsserted(this.#polarity, this.#line.read())
  }

  /**
   * Hands the line back.
   *
   * @returns The line.
   */
  release(): L {
    return this.#line
  }
}

/**
 * A line for running with nothing plugged in: it answers the reads it was given, in
 * order, and records every level it is driven to. It is what the examples and tests
 * put under a {@link Switch} or a {@link Contact}, and the one thing a real node
 * replaces with its board's pin library. It behaves as `pamoja_hal::script::PinScript`
 * does in Rust: it starts released, high, and once its reads run out a read answers
 * the level it was last driven to.
 */
export class PinScript implements OutputLine, InputLine {
  #inputs: PinLevel[]
  #driven: PinLevel[] = []
  #level: PinLevel = PinLevel.High

  /**
   * Creates a released line.
   *
   * @param inputs - The levels to answer reads with, in order.
   */
  constructor(inputs: readonly PinLevel[] = []) {
    this.#inputs = [...inputs]
  }

  /** Every level the line was driven to, oldest first. */
  get driven(): readonly PinLevel[] {
    return this.#driven
  }

  /** The level the line was last driven to, high while it has never been driven. */
  get level(): PinLevel {
    return this.#level
  }

  /** How many scripted reads are left. */
  get remaining(): number {
    return this.#inputs.length
  }

  /**
   * Records a driven level.
   *
   * @param level - The level driven.
   */
  drive(level: PinLevel): void {
    this.#level = level
    this.#driven.push(level)
  }

  /**
   * Answers the next scripted level, or the driven level once the script runs out.
   *
   * @returns The level.
   */
  read(): PinLevel {
    return this.#inputs.shift() ?? this.#level
  }
}
