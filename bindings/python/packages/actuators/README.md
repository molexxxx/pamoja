# pamoja-actuators

PCA9685 PWM and servo pulses, and stepper coil sequencing. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/actuators.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-actuators
```

```python
from pamoja import actuators
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-actuators`](https://crates.io/crates/pamoja-actuators) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html), [docs.rs](https://docs.rs/pamoja-actuators) |
| TypeScript | [`@pamoja/actuators`](https://www.npmjs.com/package/@pamoja/actuators) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html) |
| Python | [`pamoja-actuators`](https://pypi.org/project/pamoja-actuators/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html) |
| C# | [`Pamoja.Actuators`](https://www.nuget.org/packages/Pamoja.Actuators) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html) |

## Documentation

- [`pamoja.actuators` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html), every class and function in this module.
- [The Actuator drivers guide](https://pamoja.molex.cloud/docs/guides/actuators.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
