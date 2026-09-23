/**
 * Ergonomic facade over the generated actuator-driver binding.
 *
 * Two parts that move something: a PCA9685 driving up to sixteen servos, LEDs, or valves,
 * and a stepper motor walked one coil pattern at a time. The `Pca9685` class drives the
 * part over an `I2cBus` from `@pamoja/hal`, and `pca9685.sim.part` stands one up on a
 * simulated bus that keeps the datasheet's rules. The `pwm` builders make the four register
 * bytes of one channel's setting. `FourWire` and `StepDir` drive a stepper through its pins,
 * over any `OutputLine` from `@pamoja/gpio`.
 *
 * @packageDocumentation
 */

import { type OutputLine, PinLevel } from '@pamoja/gpio'
import { type Delay, type I2cBus, type I2cPart, SleepDelay } from '@pamoja/hal'
import {
  PCA9685_CHANNELS,
  PCA9685_COUNTS,
  PCA9685_DEFAULT_ADDRESS,
  PCA9685_INTERNAL_OSC_HZ,
  PCA9685_MODE1_AUTO_INCREMENT,
  PCA9685_MODE1_EXTCLK,
  PCA9685_MODE1_RESET,
  PCA9685_MODE1_RESTART,
  PCA9685_MODE1_SLEEP,
  PCA9685_MODE2_RESET,
  PCA9685_OSCILLATOR_STARTUP_MICROS,
  PCA9685_PRE_SCALE_MIN,
  PCA9685_PRE_SCALE_RESET,
  PCA9685_REGISTER_ALL_LED_ON_L,
  PCA9685_REGISTER_LED0_ON_L,
  PCA9685_REGISTER_MODE1,
  PCA9685_REGISTER_MODE2,
  PCA9685_REGISTER_PRE_SCALE,
  Pca9685,
  type Pca9685Settings,
  pca9685ChannelRegister,
  pca9685FrequencyForPrescale,
  pca9685PrescaleForFrequency,
  pca9685SimPart,
  pwmCounts,
  pwmDuty,
  pwmFromCounts,
  pwmFullOff,
  pwmFullOn,
  pwmServo,
  STEPPER_DEFAULT_PULSE_MICROS,
  STEPPER_DEFAULT_STEP_MICROS,
  type StepDirection as StepDirectionName,
  type StepDrive as StepDriveName,
  Stepper,
  stepperStepCount,
  stepperStepsForDegrees,
} from '@pamoja/native'

export { Pca9685, Stepper }
export type { Delay, I2cBus, OutputLine, Pca9685Settings }

/** Stepper motor timing, as `pamoja_actuators::stepper` starts a driver with. */
export const stepper = {
  /** The pause a driver takes after each step unless given another, in microseconds. */
  defaultStepMicros: STEPPER_DEFAULT_STEP_MICROS,
  /**
   * How long a step and direction driver holds the direction before a step pulse, and the
   * pulse itself, in microseconds.
   */
  defaultPulseMicros: STEPPER_DEFAULT_PULSE_MICROS,
} as const

/** How a stepper driver is paced. */
export interface StepperTiming {
  /** The pause after each step, which sets the speed, in microseconds. */
  stepMicros?: number
  /** What waits: a `SleepDelay` unless given, or a `DelayLog` to run with nothing plugged in. */
  delay?: Delay
}

/** How a step and direction driver is paced. */
export interface StepDirTiming extends StepperTiming {
  /**
   * How long the direction line is held before the step line rises, and the step line is
   * then held high, in microseconds.
   */
  pulseMicros?: number
}

function directionOf(count: number): StepDirection {
  return count < 0 ? StepDirection.Backward : StepDirection.Forward
}

/**
 * A four-wire stepper driven coil by coil, through a transistor array such as a ULN2003 or an
 * H-bridge, as `pamoja_actuators::stepper::FourWire` drives one in Rust.
 *
 * Each step energizes the coils the drive pattern names, coil A for the pattern's high bit down
 * to coil D for its low bit, then waits the step interval. The position counts steps from where
 * the motor was when the driver was built, forward positive. A coil line is any
 * {@link OutputLine}: a `GpioLine` on a board, or a `PinScript` that records every level.
 *
 * @example
 * ```ts
 * const coils = [new PinScript(), new PinScript(), new PinScript(), new PinScript()] as const
 * const motor = new FourWire(coils, StepDrive.FullStep, { delay: new DelayLog() })
 * await motor.steps(2)
 * motor.position // 2
 * ```
 */
export class FourWire<L extends OutputLine = OutputLine> {
  #coils: readonly [L, L, L, L]
  #sequence: Stepper
  #drive: StepDrive
  #position = 0
  #stepMicros: number
  #delay: Delay

  /**
   * Wraps the four coil lines, without driving them.
   *
   * @param coils - The lines for coils A, B, C, and D, in the motor's phase order.
   * @param drive - The coil pattern to step through.
   * @param timing - The pause after each step, {@link stepper.defaultStepMicros} unless
   *   given, and what waits it.
   */
  constructor(coils: readonly [L, L, L, L], drive: StepDrive, timing: StepperTiming = {}) {
    this.#coils = coils
    this.#sequence = new Stepper(drive)
    this.#drive = drive
    this.#stepMicros = timing.stepMicros ?? stepper.defaultStepMicros
    this.#delay = timing.delay ?? new SleepDelay()
  }

  /** The step count since the driver was built, forward positive. */
  get position(): number {
    return this.#position
  }

  /** The coil pattern in use. */
  get drive(): StepDrive {
    return this.#drive
  }

  /** The pause after each step, in microseconds. */
  get stepMicros(): number {
    return this.#stepMicros
  }

  /**
   * Takes one step and waits the step interval.
   *
   * @param direction - Which way to step.
   * @throws Whatever a coil line throws when it cannot be driven.
   */
  async step(direction: StepDirection): Promise<void> {
    this.#energize(this.#sequence.step(direction))
    this.#position += direction === StepDirection.Forward ? 1 : -1
    await this.#delay.delayMicros(this.#stepMicros)
  }

  /**
   * Takes `count` steps, backward when negative.
   *
   * @param count - The signed number of steps.
   * @throws Whatever a coil line throws; the steps already taken stay counted.
   */
  async steps(count: number): Promise<void> {
    const direction = directionOf(count)
    for (let taken = 0; taken < Math.abs(count); taken += 1) {
      await this.step(direction)
    }
  }

  /**
   * Drops every coil, so the motor holds nothing and draws nothing.
   *
   * @throws Whatever a coil line throws when it cannot be driven.
   */
  idle(): void {
    this.#energize(0)
  }

  /**
   * Hands the coil lines back, for a test to read what was driven or a program to reuse them.
   *
   * @returns The lines for coils A, B, C, and D.
   */
  release(): readonly [L, L, L, L] {
    return this.#coils
  }

  #energize(coils: number): void {
    this.#coils.forEach((line, index) => {
      line.drive(coils & (0b1000 >> index) ? PinLevel.High : PinLevel.Low)
    })
  }
}

/**
 * A stepper behind a step and direction driver chip such as an A4988 or a DRV8825, as
 * `pamoja_actuators::stepper::StepDir` drives one in Rust.
 *
 * Each step sets the direction line, high for forward, and holds it for the pulse width,
 * since the chip reads the direction on the step line's rising edge and needs it settled
 * first. Then it pulses the step line high for the pulse width, brings it low, and waits the
 * step interval. Microstepping, current limiting, and enable are the chip's own pins and
 * settings, outside this driver.
 *
 * @example
 * ```ts
 * const motor = new StepDir(new PinScript(), new PinScript(), { delay: new DelayLog() })
 * await motor.steps(-1)
 * motor.position // -1
 * ```
 */
export class StepDir<S extends OutputLine = OutputLine, R extends OutputLine = OutputLine> {
  #step: S
  #direction: R
  #position = 0
  #pulseMicros: number
  #stepMicros: number
  #delay: Delay

  /**
   * Wraps the step and direction lines, without driving them.
   *
   * @param step - The line the chip counts rising edges on.
   * @param direction - The line the chip reads the direction from; high is forward.
   * @param timing - The pulse width, {@link stepper.defaultPulseMicros} unless given, the
   *   pause after each step, {@link stepper.defaultStepMicros} unless given, and what waits
   *   them.
   */
  constructor(step: S, direction: R, timing: StepDirTiming = {}) {
    this.#step = step
    this.#direction = direction
    this.#pulseMicros = timing.pulseMicros ?? stepper.defaultPulseMicros
    this.#stepMicros = timing.stepMicros ?? stepper.defaultStepMicros
    this.#delay = timing.delay ?? new SleepDelay()
  }

  /** The step count since the driver was built, forward positive. */
  get position(): number {
    return this.#position
  }

  /** How long the direction is held before each step pulse, and the pulse itself, in microseconds. */
  get pulseMicros(): number {
    return this.#pulseMicros
  }

  /** The pause after each step, in microseconds. */
  get stepMicros(): number {
    return this.#stepMicros
  }

  /**
   * Takes one step and waits the step interval.
   *
   * @param direction - Which way to step.
   * @throws Whatever a line throws when it cannot be driven.
   */
  async step(direction: StepDirection): Promise<void> {
    const forward = direction === StepDirection.Forward
    this.#direction.drive(forward ? PinLevel.High : PinLevel.Low)
    await this.#delay.delayMicros(this.#pulseMicros)
    this.#step.drive(PinLevel.High)
    await this.#delay.delayMicros(this.#pulseMicros)
    this.#step.drive(PinLevel.Low)
    this.#position += forward ? 1 : -1
    await this.#delay.delayMicros(this.#stepMicros)
  }

  /**
   * Takes `count` steps, backward when negative.
   *
   * @param count - The signed number of steps.
   * @throws Whatever a line throws; the steps already taken stay counted.
   */
  async steps(count: number): Promise<void> {
    const direction = directionOf(count)
    for (let taken = 0; taken < Math.abs(count); taken += 1) {
      await this.step(direction)
    }
  }

  /**
   * Hands the lines back.
   *
   * @returns The step line, then the direction line.
   */
  release(): readonly [S, R] {
    return [this.#step, this.#direction]
  }
}

/**
 * Returns how many steps make up one electrical cycle of a drive pattern.
 *
 * @param drive - The coil pattern.
 * @returns `4` for wave and full-step, `8` for half-step.
 */
export function stepCount(drive: StepDrive): number {
  return stepperStepCount(drive)
}

/**
 * Returns how many steps a rotation of `degrees` takes on a given motor.
 *
 * @param degrees - The angle to turn through.
 * @param stepsPerRevolution - The motor's steps per full revolution.
 * @returns The step count, negative for a negative angle.
 */
export function stepsForDegrees(degrees: number, stepsPerRevolution: number): number {
  return stepperStepsForDegrees(degrees, stepsPerRevolution)
}

/** A stepper drive pattern, trading torque, smoothness, and resolution. */
export const StepDrive = {
  /** One coil energized at a time: four steps, least torque and least power. */
  Wave: 'Wave' as StepDriveName,
  /** Two adjacent coils at a time: four steps, most torque. */
  FullStep: 'FullStep' as StepDriveName,
  /** Alternating one and two coils: eight steps, double resolution. */
  HalfStep: 'HalfStep' as StepDriveName,
} as const

/** A stepper drive pattern, trading torque, smoothness, and resolution. */
export type StepDrive = StepDriveName

/** Which way to step a motor. */
export const StepDirection = {
  /** Advance the sequence, turning the shaft one way. */
  Forward: 'Forward' as StepDirectionName,
  /** Reverse the sequence, turning the shaft the other way. */
  Backward: 'Backward' as StepDirectionName,
} as const

/** Which way to step a motor. */
export type StepDirection = StepDirectionName

/** An NXP PCA9685 16-channel PWM controller, for servos, LEDs, and valves. */
export const pca9685 = {
  /** The address it answers at with its six address pins low. */
  defaultAddress: PCA9685_DEFAULT_ADDRESS,
  /** The part's internal oscillator frequency, in hertz. */
  internalOscHz: PCA9685_INTERNAL_OSC_HZ,
  /** How many channels it drives. */
  channels: PCA9685_CHANNELS,
  /** How many counts each period is divided into. */
  counts: PCA9685_COUNTS,
  /** How long the oscillator takes to run once woken, in microseconds. */
  oscillatorStartupMicros: PCA9685_OSCILLATOR_STARTUP_MICROS,
  /** The registers a driver writes and a program reads back. */
  register: {
    /** Mode register 1: restart, clock, auto-increment, sleep, and the addresses answered. */
    mode1: PCA9685_REGISTER_MODE1,
    /** Mode register 2: how the outputs are wired and when they change. */
    mode2: PCA9685_REGISTER_MODE2,
    /** The first of channel 0's four registers; `channelRegister` gives the rest. */
    led0OnL: PCA9685_REGISTER_LED0_ON_L,
    /** The first of the four registers that load every channel at once. */
    allLedOnL: PCA9685_REGISTER_ALL_LED_ON_L,
    /** The prescaler, writable only while the part sleeps. */
    preScale: PCA9685_REGISTER_PRE_SCALE,
  },
  /** MODE1's bits. */
  mode1: {
    /** Set when the part slept with a channel running; cleared by a written 1. */
    restart: PCA9685_MODE1_RESTART,
    /** The prescaler divides the EXTCLK pin rather than the oscillator. */
    extclk: PCA9685_MODE1_EXTCLK,
    /** The register pointer moves on after each byte. */
    autoIncrement: PCA9685_MODE1_AUTO_INCREMENT,
    /** The oscillator is off and PRE_SCALE takes a write. */
    sleep: PCA9685_MODE1_SLEEP,
  },
  /** MODE1 at power-up: asleep, answering the All Call address. */
  mode1Reset: PCA9685_MODE1_RESET,
  /** MODE2 at power-up: totem-pole outputs. */
  mode2Reset: PCA9685_MODE2_RESET,
  /** PRE_SCALE at power-up: 200 Hz on the internal oscillator. */
  preScaleReset: PCA9685_PRE_SCALE_RESET,
  /** The smallest value the part loads into PRE_SCALE. */
  preScaleMin: PCA9685_PRE_SCALE_MIN,

  /**
   * Returns the first of a channel's four consecutive registers.
   *
   * @param channel - The channel, 0 to 15.
   * @returns The register address.
   * @throws If the channel is beyond the part.
   */
  channelRegister(channel: number): number {
    return pca9685ChannelRegister(channel)
  },

  /**
   * Returns the prescale value that sets an update rate.
   *
   * @param updateRateHz - The PWM frequency wanted.
   * @param oscHz - The oscillator frequency, usually `internalOscHz`.
   * @returns The prescale register value.
   */
  prescaleForFrequency(updateRateHz: number, oscHz: number = PCA9685_INTERNAL_OSC_HZ): number {
    return pca9685PrescaleForFrequency(updateRateHz, oscHz)
  },

  /**
   * Returns the update rate a prescale value produces.
   *
   * @param prescale - The prescale register value.
   * @param oscHz - The oscillator frequency, usually `internalOscHz`.
   * @returns The frequency in hertz.
   */
  frequencyForPrescale(prescale: number, oscHz: number = PCA9685_INTERNAL_OSC_HZ): number {
    return pca9685FrequencyForPrescale(prescale, oscHz)
  },

  /**
   * A PCA9685 that is not there. It holds the part's power-on registers and keeps its
   * datasheet's rules: PRE_SCALE takes a write only while the part sleeps, the register
   * pointer moves on only with auto-increment set, and one write to the ALL_LED registers
   * loads every channel.
   */
  sim: {
    /**
     * A simulated PCA9685 as it powers up: asleep at 200 Hz with every output off.
     *
     * @param address - The address it answers to, `defaultAddress` unless given.
     * @returns A part to put on a simulated bus.
     */
    part(address: number = PCA9685_DEFAULT_ADDRESS): I2cPart {
      return pca9685SimPart(address)
    },
  },
}

/**
 * The four register bytes for one PCA9685 channel.
 *
 * Each call returns a buffer in the channel's own register order, so it can be
 * written in a single bus transaction.
 */
export const pwm = {
  /**
   * Builds a setting from explicit on and off counts.
   *
   * @param on - The count at which the output goes high.
   * @param off - The count at which it goes low.
   * @returns The four register bytes; counts are masked to 12 bits.
   */
  fromCounts(on: number, off: number): Buffer {
    return pwmFromCounts(on, off)
  },

  /**
   * Builds a setting with no phase delay: on at count 0, off at `off`.
   *
   * The datasheet rules out the same count in on and off, so 0 is the full-off setting and
   * 4096 or more the full-on one.
   *
   * @param off - The count at which the output goes low, which sets the duty.
   * @returns The four register bytes.
   */
  duty(off: number): Buffer {
    return pwmDuty(off)
  },

  /**
   * Builds the setting that drives a hobby servo to a pulse width.
   *
   * @param pulseMicros - The high-pulse width in microseconds. Typical travel is
   * about 1000 to 2000 microseconds.
   * @param updateRateHz - The PWM frequency the controller is set to, usually 50.
   * @returns The four register bytes.
   */
  servo(pulseMicros: number, updateRateHz = 50): Buffer {
    return pwmServo(pulseMicros, updateRateHz)
  },

  /**
   * Reads a setting back from the four register bytes a channel holds.
   *
   * The inverse of the builders above, so a caller can read a channel off the bus and
   * see what it is set to rather than decoding the registers by hand.
   *
   * @param bytes - The four channel registers.
   * @returns The counts at which the output goes high and low.
   * @throws If the buffer is not four bytes.
   */
  counts(bytes: Uint8Array): { on: number; off: number } {
    return pwmCounts(Buffer.from(bytes))
  },

  /**
   * The setting that holds a channel continuously high.
   *
   * @returns The four register bytes.
   */
  fullOn(): Buffer {
    return pwmFullOn()
  },

  /**
   * The setting that holds a channel continuously low, the power-on state. Its flag takes
   * precedence over the full-on flag when both are set.
   *
   * @returns The four register bytes.
   */
  fullOff(): Buffer {
    return pwmFullOff()
  },
}
