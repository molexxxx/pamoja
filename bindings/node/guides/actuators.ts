// The actuator-driver guide example; see docs/guides/actuators.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import {
  StepDirection,
  StepDrive,
  Stepper,
  pca9685,
  pwm,
  stepCount,
  stepsForDegrees,
} from '@pamoja/actuators'

// A servo bank wants 50 Hz. The prescale register that produces it is derived from the
// part's 25 MHz internal oscillator, so a caller names the rate it wants rather than
// working the divider out.
const prescale = pca9685.prescaleForFrequency(50)
console.log(`prescale  ${prescale} gives ${pca9685.frequencyForPrescale(prescale).toFixed(1)} Hz`)

// Each channel owns four consecutive registers, so a whole channel is written in one bus
// transaction rather than four.
const register = pca9685.channelRegister(3)
console.log(`channel 3 starts at register 0x${register.toString(16).toUpperCase()}`)

// A centred hobby servo holds its output high for 1500 us of the 20 ms period. The part
// counts in 4096 steps per period, so that is where the pulse ends.
const centred = pwm.servo(1500, 50)
console.log(`centred servo goes low at count ${pwm.counts(centred).off} of 4096`)

// Fully off carries its own flag rather than a zero duty, which would still hold the
// output high for the first count of every period.
console.log(`full off flag set: ${pwm.counts(pwm.fullOff()).off !== pwm.counts(pwm.duty(0)).off}`)

// A stepper is driven by walking a pattern of coil states. Half-step drive interleaves
// the one-coil and two-coil patterns, so it has twice as many.
const motor = new Stepper(StepDrive.HalfStep)
const bits = (coils: number) => coils.toString(2).padStart(4, '0')
console.log(`coils     ${bits(motor.coils)} at rest`)
for (let step = 0; step < 2; step += 1) {
  console.log(`coils     ${bits(motor.step(StepDirection.Forward))} after a step`)
}

// The patterns wrap, so the motor runs indefinitely either way, and an angle converts to
// whole steps: a quarter turn of a 1.8-degree motor is fifty of them.
for (let step = 2; step < stepCount(StepDrive.HalfStep); step += 1) {
  motor.step(StepDirection.Forward)
}
console.log(`coils     ${bits(motor.coils)} back at the start of the cycle`)
console.log(`a quarter turn is ${stepsForDegrees(90, 200)} steps`)
// ANCHOR_END: example

assert.equal(prescale, 0x79)
assert.equal(register, 0x12)
assert.equal(pwm.counts(centred).off, 307)
assert.notDeepEqual([...pwm.fullOff()], [...pwm.duty(0)])
assert.equal(motor.coils, 0b1000)
assert.equal(stepsForDegrees(90, 200), 50)
