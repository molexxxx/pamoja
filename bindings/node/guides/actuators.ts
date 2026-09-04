// The actuator-driver guide example; see docs/guides/actuators.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import {
  StepDirection,
  StepDrive,
  Stepper,
  pca9685,
  pwm,
  stepCount,
  stepsForDegrees,
} from '@pamoja/actuators'

// The datasheet's worked example: 200 Hz off the 25 MHz internal oscillator is prescale
// 0x1E, the value the part powers up holding. A servo bank wants 50 Hz instead.
assert.equal(pca9685.prescaleForFrequency(200), 0x1e)
assert.equal(pca9685.prescaleForFrequency(50), 0x79)

// Each channel owns four consecutive registers from 0x06, so channel 3 starts at 0x12
// and its four bytes go out in one bus transaction.
assert.equal(pca9685.channelRegister(3), 0x12)

// A centred hobby servo holds its output high for 1500 us, 7.5 % of the 20 ms period,
// which is 307 of the 4096 counts. The bytes are on-low, on-high, off-low, off-high.
assert.deepEqual([...pwm.servo(1500, 50)], [0x00, 0x00, 0x33, 0x01])

// Fully off carries its own bit rather than a zero duty, which still holds the output
// high for the first count of every period.
assert.deepEqual([...pwm.fullOff()], [0x00, 0x00, 0x00, 0x10])
assert.deepEqual([...pwm.duty(0)], [0x00, 0x00, 0x00, 0x00])

// Half-step drive interleaves the one-coil and two-coil patterns; the most significant
// of the four bits is the first coil.
const halfStep = StepDrive.HalfStep as StepDrive
const forward = StepDirection.Forward as StepDirection
const motor = new Stepper(halfStep)
assert.equal(motor.coils, 0b1000)
assert.equal(motor.step(forward), 0b1100)
assert.equal(motor.step(forward), 0b0100)

// The eight patterns of a cycle wrap, so the motor runs indefinitely either way, and an
// angle converts to whole steps: a quarter turn of a 1.8-degree motor is 50 of them.
for (let step = 2; step < stepCount(halfStep); step += 1) {
  motor.step(forward)
}
assert.equal(motor.coils, 0b1000)
assert.equal(stepsForDegrees(90, 200), 50)
// ANCHOR_END: example
