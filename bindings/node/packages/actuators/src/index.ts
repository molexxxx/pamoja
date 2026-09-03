/**
 * Ergonomic facade over the generated actuator-driver binding.
 *
 * These are the command-encode half of two parts that move something: a PCA9685
 * driving up to sixteen servos, LEDs, or valves, and a stepper motor walked one
 * coil pattern at a time. Applying the bytes is the caller's job; working out
 * which bytes is this layer's.
 *
 * @packageDocumentation
 */

import {
  PCA9685_CHANNELS,
  PCA9685_COUNTS,
  PCA9685_INTERNAL_OSC_HZ,
  pca9685ChannelRegister,
  pca9685FrequencyForPrescale,
  pca9685PrescaleForFrequency,
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

export { Stepper }

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
  /** One coil energised at a time: four steps, least torque and least power. */
  Wave: 'Wave',
  /** Two adjacent coils at a time: four steps, most torque. */
  FullStep: 'FullStep',
  /** Alternating one and two coils: eight steps, double resolution. */
  HalfStep: 'HalfStep',
} as const

/** A stepper drive pattern, trading torque, smoothness, and resolution. */
export type StepDrive = StepDriveName

/** Which way to step a motor. */
export const StepDirection = {
  /** Advance the sequence, turning the shaft one way. */
  Forward: 'Forward',
  /** Reverse the sequence, turning the shaft the other way. */
  Backward: 'Backward',
} as const

/** Which way to step a motor. */
export type StepDirection = StepDirectionName

/** An NXP PCA9685 16-channel PWM controller, for servos, LEDs, and valves. */
export const pca9685 = {
  /** The part's internal oscillator frequency, in hertz. */
  internalOscHz: PCA9685_INTERNAL_OSC_HZ,
  /** How many channels it drives. */
  channels: PCA9685_CHANNELS,
  /** How many counts each period is divided into. */
  counts: PCA9685_COUNTS,

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
   * @param oscHz - The oscillator frequency, usually {@link internalOscHz}.
   * @returns The prescale register value.
   */
  prescaleForFrequency(updateRateHz: number, oscHz: number = PCA9685_INTERNAL_OSC_HZ): number {
    return pca9685PrescaleForFrequency(updateRateHz, oscHz)
  },

  /**
   * Returns the update rate a prescale value produces.
   *
   * @param prescale - The prescale register value.
   * @param oscHz - The oscillator frequency, usually {@link internalOscHz}.
   * @returns The frequency in hertz.
   */
  frequencyForPrescale(prescale: number, oscHz: number = PCA9685_INTERNAL_OSC_HZ): number {
    return pca9685FrequencyForPrescale(prescale, oscHz)
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
