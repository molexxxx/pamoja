# Actuator drivers

Two parts cover most of what a field node actually moves. A PCA9685 gives
sixteen PWM channels at one shared frequency, which is how a bank of servos,
dimmable lights, or the inputs of a motor driver run off a single I2C device. A
four-wire stepper turns by energising its coils in a repeating pattern. Both are
exact arithmetic against a datasheet, and both fail quietly when a constant is
wrong: a servo that buzzes against its endstop, a motor that hums without
turning. pamoja works out the register bytes and the coil patterns and writes
nothing, so the same code runs on a gateway, on a microcontroller, or in a test
with nothing wired to it.

## What the example does

It programs a PCA9685 for a bank of hobby servos, centres the servo on channel
3, and then walks a stepper motor through one half-step cycle.

It proves:

- The prescaler follows the datasheet's own worked example: 200 Hz off the
  25 MHz internal oscillator is `0x1E`, so an implementation that is wrong but
  self-consistent still fails.
- Channel registers start at `0x06` and run four apart, which puts channel 3 at
  `0x12`.
- A centred 1500 microsecond pulse at 50 Hz is 307 of the 4096 counts, and its
  four bytes come back in the channel's own register order.
- Fully off is a dedicated bit rather than a zero duty, which would still hold
  the output high for the first count of every period.
- Half-step drive walks the eight-pattern sequence, one coil then two, and wraps
  back to the pattern it started on.
- A quarter turn of a 1.8-degree motor is 50 whole steps.

## Rust

<!-- snippet: examples/tests/guides/actuators.rs#example -->
From [`examples/tests/guides/actuators.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/actuators.rs):

```rust
use pamoja_actuators::pca9685::{self, Pwm, INTERNAL_OSC_HZ};
use pamoja_actuators::stepper::{steps_for_degrees, Direction, Drive, Sequencer};

// The datasheet's worked example: 200 Hz off the 25 MHz internal oscillator is prescale
// 0x1E, the value the part powers up holding. A servo bank wants 50 Hz instead.
assert_eq!(pca9685::prescale_for_frequency(200, INTERNAL_OSC_HZ), 0x1E);
assert_eq!(pca9685::prescale_for_frequency(50, INTERNAL_OSC_HZ), 0x79);

// Each channel owns four consecutive registers from 0x06, so channel 3 starts at 0x12
// and its four bytes go out in one bus transaction.
assert_eq!(pca9685::channel_register(3), 0x12);

// A centred hobby servo holds its output high for 1500 us, 7.5 % of the 20 ms period,
// which is 307 of the 4096 counts. The bytes are on-low, on-high, off-low, off-high.
assert_eq!(Pwm::servo(1500, 50).bytes(), [0x00, 0x00, 0x33, 0x01]);

// Fully off carries its own bit rather than a zero duty, which still holds the output
// high for the first count of every period.
assert_eq!(Pwm::full_off().bytes(), [0x00, 0x00, 0x00, 0x10]);
assert_eq!(Pwm::duty(0).bytes(), [0x00, 0x00, 0x00, 0x00]);

// Half-step drive interleaves the one-coil and two-coil patterns; the most significant
// of the four bits is the first coil.
let mut motor = Sequencer::new(Drive::HalfStep);
assert_eq!(motor.coils(), 0b1000);
assert_eq!(motor.step(Direction::Forward), 0b1100);
assert_eq!(motor.step(Direction::Forward), 0b0100);

// The eight patterns of a cycle wrap, so the motor runs indefinitely either way, and an
// angle converts to whole steps: a quarter turn of a 1.8-degree motor is 50 of them.
for _ in 2..Drive::HalfStep.step_count() {
    motor.step(Direction::Forward);
}
assert_eq!(motor.coils(), 0b1000);
assert_eq!(steps_for_degrees(90.0, 200), 50);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/actuators.ts#example -->
From [`bindings/node/guides/actuators.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/actuators.ts):

```typescript
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
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/actuators.py#example -->
From [`bindings/python/guides/actuators.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/actuators.py):

```python
from pamoja.actuators import Direction, Drive, Stepper, pca9685, pwm, steps_for_degrees

# The datasheet's worked example: 200 Hz off the 25 MHz internal oscillator is prescale
# 0x1E, the value the part powers up holding. A servo bank wants 50 Hz instead.
assert pca9685.prescale_for_frequency(200) == 0x1E
assert pca9685.prescale_for_frequency(50) == 0x79

# Each channel owns four consecutive registers from 0x06, so channel 3 starts at 0x12
# and its four bytes go out in one bus transaction.
assert pca9685.channel_register(3) == 0x12

# A centred hobby servo holds its output high for 1500 us, 7.5 % of the 20 ms period,
# which is 307 of the 4096 counts. The bytes are on-low, on-high, off-low, off-high.
assert pwm.servo(1500, 50) == bytes([0x00, 0x00, 0x33, 0x01])

# Fully off carries its own bit rather than a zero duty, which still holds the output
# high for the first count of every period.
assert pwm.full_off() == bytes([0x00, 0x00, 0x00, 0x10])
assert pwm.duty(0) == bytes([0x00, 0x00, 0x00, 0x00])

# Half-step drive interleaves the one-coil and two-coil patterns; the most significant
# of the four bits is the first coil.
motor = Stepper(Drive.HALF_STEP)
assert motor.coils == 0b1000
assert motor.step(Direction.FORWARD) == 0b1100
assert motor.step(Direction.FORWARD) == 0b0100

# The eight patterns of a cycle wrap, so the motor runs indefinitely either way, and an
# angle converts to whole steps: a quarter turn of a 1.8-degree motor is 50 of them.
for _ in range(2, Drive.HALF_STEP.step_count):
    motor.step(Direction.FORWARD)
assert motor.coils == 0b1000
assert steps_for_degrees(90.0, 200) == 50
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs):

```csharp
// The datasheet's worked example: 200 Hz off the 25 MHz internal oscillator is
// prescale 0x1E, the value the part powers up holding. A servo bank wants 50 Hz.
Expect(Pca9685.PrescaleForFrequency(200) == 0x1E, "the datasheet's worked example");
Expect(Pca9685.PrescaleForFrequency(50) == 0x79, "and the usual servo rate");

// Each channel owns four consecutive registers from 0x06, so channel 3 starts at
// 0x12 and its four bytes go out in one bus transaction.
Expect(Pca9685.ChannelRegister(3) == 0x12, "the fourth channel's register block");

// A centred hobby servo holds its output high for 1500 us, 7.5 % of the 20 ms
// period, which is 307 of the 4096 counts. The bytes are on-low, on-high,
// off-low, off-high.
Expect(
    Pwm.Servo(1500, 50).SequenceEqual(new byte[] { 0x00, 0x00, 0x33, 0x01 }),
    "a centred pulse is 307 counts of the period");

// Fully off carries its own bit rather than a zero duty, which still holds the
// output high for the first count of every period.
Expect(
    Pwm.FullOff().SequenceEqual(new byte[] { 0x00, 0x00, 0x00, 0x10 }),
    "fully off is its own encoding");
Expect(
    Pwm.Duty(0).SequenceEqual(new byte[] { 0x00, 0x00, 0x00, 0x00 }),
    "which a zero duty is not");

// Half-step drive interleaves the one-coil and two-coil patterns; the most
// significant of the four bits is the first coil.
using var motor = new Stepper(StepDrive.HalfStep);
Expect(motor.Coils == 0b1000, "the cycle opens on one coil");
Expect(motor.Step(StepDirection.Forward) == 0b1100, "then a pair of them");
Expect(motor.Step(StepDirection.Forward) == 0b0100, "then the next coil alone");

// The eight patterns of a cycle wrap, so the motor runs indefinitely either way,
// and an angle converts to whole steps: a quarter turn of a 1.8-degree motor is
// 50 of them.
for (int step = 2; step < Stepper.StepCount(StepDrive.HalfStep); step++)
{
    motor.Step(StepDirection.Forward);
}

Expect(motor.Coils == 0b1000, "a full cycle returns to its first pattern");
Expect(Stepper.StepsForDegrees(90.0f, 200) == 50, "a quarter turn is fifty steps");
```
<!-- end -->

## Reference

<!-- table: reference actuators -->
- Rust: [`pamoja-actuators`](https://docs.rs/pamoja-actuators) ([site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html))
- TypeScript: [`@pamoja/actuators`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html)
- Python: [`pamoja.actuators`](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html)
- C#: [`Pca9685`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.Pca9685.html), [`Pwm`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.Pwm.html), [`Stepper`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.Stepper.html), [`StepDrive`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.StepDrive.html), [`StepDirection`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.StepDirection.html)
<!-- end -->
