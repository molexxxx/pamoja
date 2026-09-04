# pamoja-actuators

PCA9685 PWM and servo pulses, and stepper coil sequencing. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-actuators`](https://crates.io/crates/pamoja-actuators) | [docs.rs](https://docs.rs/pamoja-actuators), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html) |
| TypeScript | [`@pamoja/actuators`](https://www.npmjs.com/package/@pamoja/actuators) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html) |
| Python | [`pamoja-actuators`](https://pypi.org/project/pamoja-actuators/) | [`pamoja.actuators`](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html) |
| C# | [`Pamoja.Actuators`](https://www.nuget.org/packages/Pamoja.Actuators) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.Pca9685.html) |

## Documentation

- [The Actuator drivers guide](https://pamoja.molex.cloud/docs/guides/actuators.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
