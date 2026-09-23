/**
 * Ergonomic facade over the generated actuator-driver binding.
 *
 * Two parts that move something: a PCA9685 driving up to sixteen servos, LEDs, or valves,
 * and a stepper motor walked one coil pattern at a time. The `Pca9685` class drives the
 * part over an `I2cBus` from `@pamoja/hal`, and `pca9685.sim.part` stands one up on a
 * simulated bus that keeps the datasheet's rules. The `pwm` builders make the four register
 * bytes of one channel's setting.
 *
 * @packageDocumentation
 */

import type { I2cBus, I2cPart } from '@pamoja/hal'
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
  type StepDirection as StepDirectionName,
  type StepDrive as StepDriveName,
  Stepper,
  stepperStepCount,
  stepperStepsForDegrees,
} from '@pamoja/native'

export { Pca9685, Stepper }
export type { I2cBus, Pca9685Settings }

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
   * The setting that holds a channel continuously low, the power-on state.
   *
   * @returns The four register bytes. This is not the same as a zero duty, which
   * still glitches high for one count.
   */
  fullOff(): Buffer {
    return pwmFullOff()
  },
}
