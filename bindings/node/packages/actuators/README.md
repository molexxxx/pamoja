# @pamoja/actuators

PCA9685 PWM and servo pulses, and stepper coil sequencing. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/actuators.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/actuators
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-actuators`](https://crates.io/crates/pamoja-actuators) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html), [docs.rs](https://docs.rs/pamoja-actuators), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-actuators) |
| TypeScript | [`@pamoja/actuators`](https://www.npmjs.com/package/@pamoja/actuators) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-actuators) |
| Python | [`pamoja-actuators`](https://pypi.org/project/pamoja-actuators/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-actuators) |
| C# | [`Pamoja.Actuators`](https://www.nuget.org/packages/Pamoja.Actuators) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-actuators) |

## Documentation

- [`@pamoja/actuators` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html), every class, function, and type this package exports.
- [The Actuator drivers guide](https://pamoja.molex.cloud/docs/guides/actuators.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
