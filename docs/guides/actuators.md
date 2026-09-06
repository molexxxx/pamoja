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

It sets a PCA9685 up for a bank of hobby servos, finds where channel 3's four
registers begin, builds the pulse that centres the servo on that channel, and
separates a channel held fully off from one sitting at zero duty. Then it walks
a stepper through a complete half-step cycle.

The register values are derived from what a caller already knows. The prescale
byte comes from the 50 Hz update rate and the part's 25 MHz internal oscillator,
and feeding it back reports the frequency it really produces. The servo pulse is
named in microseconds and converted to the count the output stays high for. The
coil patterns come from the drive itself, so the sequence is walked rather than
written out as a table of bits.

It proves:

- 50 Hz off the 25 MHz internal oscillator is prescale 121 (`0x79`), the value
  the datasheet's formula gives, so a divider that is wrong but round-trips
  consistently still fails.
- Channel 3's registers begin at `0x12`, four along from each channel before it.
- A centred 1500 microsecond pulse at 50 Hz goes low at count 307 of the 4096
  counts in a period.
- Fully off is its own encoding rather than a zero duty, which would still hold
  the output high for the first count of every period.
- Half-step drive alternates one energised coil with two, `1000` then `1100`
  then `0100`, and eight steps wrap back to the pattern it started on.
- A quarter turn of a 1.8-degree motor is 50 whole steps.

## Rust

<!-- snippet: examples/tests/guides/actuators.rs#example -->
From [`examples/tests/guides/actuators.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/actuators.rs):

```rust
use pamoja_actuators::pca9685::{self, Pwm, INTERNAL_OSC_HZ};
use pamoja_actuators::stepper::{steps_for_degrees, Direction, Drive, Sequencer};

// A servo bank wants 50 Hz. The prescale register that produces it is derived from
// the part's 25 MHz internal oscillator, so a caller names the rate it wants rather
// than working the divider out.
let prescale = pca9685::prescale_for_frequency(50, INTERNAL_OSC_HZ);
let rate = pca9685::frequency_for_prescale(prescale, INTERNAL_OSC_HZ);
println!("prescale  {prescale} gives {rate:.1} Hz");

// Each channel owns four consecutive registers, so a whole channel is written in one
// bus transaction rather than four.
let first_register = pca9685::channel_register(3);
println!("channel 3 starts at register {first_register:#04X}");

// A centred hobby servo holds its output high for 1500 us of the 20 ms period. The
// part counts in 4096 steps per period, so that is where the pulse ends.
let centred = Pwm::servo(1500, 50);
println!("centred servo goes low at count {} of 4096", centred.off());

// Fully off carries its own flag rather than a zero duty, which would still hold the
// output high for the first count of every period.
let flagged = Pwm::full_off().off() != Pwm::duty(0).off();
println!("full off flag set: {flagged}");

// A stepper is driven by walking a pattern of coil states. Half-step drive
// interleaves the one-coil and two-coil patterns, so it has twice as many.
let mut motor = Sequencer::new(Drive::HalfStep);
let at_rest = motor.coils();
println!("coils     {at_rest:04b} at rest");
for _ in 0..2 {
    let coils = motor.step(Direction::Forward);
    println!("coils     {coils:04b} after a step");
}

// The patterns wrap, so the motor runs indefinitely either way, and an angle converts
// to whole steps: a quarter turn of a 1.8-degree motor is fifty of them.
for _ in 2..Drive::HalfStep.step_count() {
    motor.step(Direction::Forward);
}
let wrapped = motor.coils();
let quarter_turn = steps_for_degrees(90.0, 200);
println!("coils     {wrapped:04b} back at the start of the cycle");
println!("a quarter turn is {quarter_turn} steps");
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/actuators.ts#example -->
From [`bindings/node/guides/actuators.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/actuators.ts):

```typescript
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
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/actuators.py#example -->
From [`bindings/python/guides/actuators.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/actuators.py):

```python
from pamoja.actuators import Direction, Drive, Stepper, pca9685, pwm, steps_for_degrees

# A servo bank wants 50 Hz. The prescale register that produces it is derived from the
# part's 25 MHz internal oscillator, so a caller names the rate it wants rather than
# working the divider out.
prescale = pca9685.prescale_for_frequency(50)
print(f"prescale  {prescale} gives {pca9685.frequency_for_prescale(prescale):.1f} Hz")

# Each channel owns four consecutive registers, so a whole channel is written in one bus
# transaction rather than four.
print(f"channel 3 starts at register 0x{pca9685.channel_register(3):02X}")

# A centred hobby servo holds its output high for 1500 us of the 20 ms period. The part
# counts in 4096 steps per period, so that is where the pulse ends.
centred = pwm.servo(1500, 50)
print(f"centred servo goes low at count {pwm.counts(centred).off} of 4096")

# Fully off carries its own flag rather than a zero duty, which would still hold the
# output high for the first count of every period.
print(f"full off flag set: {pwm.counts(pwm.full_off()).off != pwm.counts(pwm.duty(0)).off}")

# A stepper is driven by walking a pattern of coil states. Half-step drive interleaves
# the one-coil and two-coil patterns, so it has twice as many.
motor = Stepper(Drive.HALF_STEP)
print(f"coils     {motor.coils:04b} at rest")
for _ in range(2):
    print(f"coils     {motor.step(Direction.FORWARD):04b} after a step")

# The patterns wrap, so the motor runs indefinitely either way, and an angle converts to
# whole steps: a quarter turn of a 1.8-degree motor is fifty of them.
for _ in range(2, Drive.HALF_STEP.step_count):
    motor.step(Direction.FORWARD)
print(f"coils     {motor.coils:04b} back at the start of the cycle")
print(f"a quarter turn is {steps_for_degrees(90.0, 200)} steps")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs):

```csharp
// A servo bank wants 50 Hz. The prescale register that produces it is derived
// from the part's 25 MHz internal oscillator, so a caller names the rate it wants
// rather than working the divider out.
byte prescale = Pca9685.PrescaleForFrequency(50);
Console.WriteLine(
    $"prescale  {prescale} gives {Pca9685.FrequencyForPrescale(prescale):F1} Hz");

// Each channel owns four consecutive registers, so a whole channel is written in
// one bus transaction rather than four.
Console.WriteLine($"channel 3 starts at register 0x{Pca9685.ChannelRegister(3):X2}");

// A centred hobby servo holds its output high for 1500 us of the 20 ms period.
// The part counts in 4096 steps per period, so that is where the pulse ends.
byte[] centred = Pwm.Servo(1500, 50);
Console.WriteLine($"centred servo goes low at count {Pwm.Counts(centred).Off} of 4096");

// Fully off carries its own flag rather than a zero duty, which would still hold
// the output high for the first count of every period.
bool flagged = Pwm.Counts(Pwm.FullOff()).Off != Pwm.Counts(Pwm.Duty(0)).Off;
Console.WriteLine($"full off flag set: {flagged}");

// A stepper is driven by walking a pattern of coil states. Half-step drive
// interleaves the one-coil and two-coil patterns, so it has twice as many.
using var motor = new Stepper(StepDrive.HalfStep);
Console.WriteLine($"coils     {Convert.ToString(motor.Coils, 2).PadLeft(4, '0')} at rest");
for (int step = 0; step < 2; step++)
{
    byte coils = motor.Step(StepDirection.Forward);
    Console.WriteLine(
        $"coils     {Convert.ToString(coils, 2).PadLeft(4, '0')} after a step");
}

// The patterns wrap, so the motor runs indefinitely either way, and an angle
// converts to whole steps: a quarter turn of a 1.8-degree motor is fifty of them.
for (int step = 2; step < Stepper.StepCount(StepDrive.HalfStep); step++)
{
    motor.Step(StepDirection.Forward);
}

Console.WriteLine(
    $"coils     {Convert.ToString(motor.Coils, 2).PadLeft(4, '0')} back at the start "
    + "of the cycle");
Console.WriteLine($"a quarter turn is {Stepper.StepsForDegrees(90.0f, 200)} steps");
```
<!-- end -->

## Reference

<!-- table: reference actuators -->
- Rust: [`pamoja-actuators`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-actuators)
- TypeScript: [`@pamoja/actuators`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-actuators)
- Python: [`pamoja.actuators`](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-actuators)
- C#: [`Pamoja.Actuators`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-actuators)
- Hardware: [PCA9685](https://pamoja.molex.cloud/docs/hardware.html#pca9685), [ULN2003A](https://pamoja.molex.cloud/docs/hardware.html#uln2003), [A4988](https://pamoja.molex.cloud/docs/hardware.html#a4988), [DRV8825](https://pamoja.molex.cloud/docs/hardware.html#drv8825), [I2C](https://pamoja.molex.cloud/docs/hardware.html#i2c)
<!-- end -->
