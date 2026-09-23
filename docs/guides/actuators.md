# Actuator drivers

A servo, a dimmed LED, and a stepper motor each take a signal their datasheet defines to the
microsecond. A hobby servo turns to the width of a pulse it is sent every 20 ms. A PCA9685
makes sixteen of those pulses at once from a 25 MHz clock, but it takes a new frequency only
while its oscillator sleeps, and it holds each channel as an on count and an off count in four
registers. A stepper turns only as far as its coils are walked, one pattern at a time, and a
step and direction chip reads the direction on the step line's rising edge, so the direction
has to settle first. pamoja carries both halves for each part: the arithmetic that turns a
pulse width, a duty, or an angle into register bytes and coil patterns, and drivers that talk
to the part in its datasheet's order, from all four languages.

Three kinds of part are covered. The PCA9685 runs on the one I2C bus the
[buses guide](hal.md) introduces, and has a simulated twin that keeps its datasheet's rules.
Four-wire steppers such as the 28BYJ-48 run through a transistor array like the ULN2003 or
through an H-bridge, and bipolar steppers through a step and direction chip such as the A4988
or the DRV8825. The stepper drivers take any output line, a GPIO line on a Linux board or a pin
script that records every level, and any delay, one that sleeps or one that only counts, so a
program that moves three motors is written and tested with nothing plugged in.

## What the example does

It builds a motion-control time-lapse rig: a camera on a slider that creeps along a rail
between frames, on a pan head that turns it, with a servo that tilts it and a status LED that
lights for each exposure.

The PCA9685 answers at `0x40` with its address pins low, and holds the tilt servo on channel 0
and the LED on channel 15. The slider is a 1.8-degree motor behind an A4988 with its three
microstep pins high, sixteen microsteps to a step, pulling a GT2 belt over a 20-tooth pulley.
The pan head is a 28BYJ-48 through a ULN2003, half-stepped. The controller runs at 50 Hz, the
rate hobby servos expect, and the driver starts it the way the datasheet asks: asleep to take
the prescale, then awake, then a 500 µs wait for the oscillator before it restarts the
channels. The example reads each channel back from the part to show what it holds.

The shoot is four frames. For each one the LED goes to full on for the exposure and back to its
glow after. Between frames the slider moves 5 mm and the pan head 2 degrees. The steppers'
lines are pin scripts that record every level, and their delays count every wait without
sleeping through it, so the program reports 624 ms of slider motion and 276 ms of pan and still
finishes at once.

Three scenes follow. A new prescale is written straight to the bus while the oscillator runs,
as a driver that skipped the sleep would write it, and the part drops it. A channel the part
does not have is refused before anything reaches the bus. And the rig parks: every channel off
in one transfer, the oscillator asleep, and the pan head's coils let go.

It proves:

- 50 Hz from the 25 MHz internal oscillator is prescale 121, and the part runs at 50.03 Hz once
  the prescaler has rounded. The oscillator's 500 µs start-up is the only wait the controller
  asks for.
- A 1300 µs pulse at 50 Hz goes low at count 266 of the 4096 in a period, and a glow of a
  sixteenth is high for 256 counts.
- The slider's 5 mm is 400 microsteps, an eighth of the 3200 in a turn. The A4988 sees one
  rising edge a microstep, 1200 over the shoot, with DIR high for forward.
- The pan head's 2 degrees rounds to 23 half-steps of the 4096 in a turn, so the head stands at
  2.02, 4.04, and 6.06 degrees, not a round 2, 4, and 6.
- A step and direction driver holds the direction for its pulse width before the step line
  rises, holds the step line high for as long, 10 µs each by default, then waits the step
  interval: 520 µs a microstep at the rig's 500 µs interval, 624 ms over the shoot.
- A prescale written while the oscillator runs is dropped, as the datasheet says, and the
  servos stay at 50 Hz.
- A channel past 15 is refused with its reason, and the bus never sees the transfer.
- Parked, both channels read back full off, MODE1 says the oscillator sleeps, and all four pan
  coils are low.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example actuators" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example actuators</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- actuators" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- actuators</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/actuators.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/actuators.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- actuators" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- actuators</code></div>
</div>
<!-- end -->

## Rust

In Rust the drivers are in `pamoja-actuators`. `pca9685::Pca9685` is generic over the
`embedded-hal` I2C and delay traits, so on a host it takes a clone of
`pamoja_hal::bus::I2cBus` and `bus.delay()`, and on a microcontroller the HAL's own peripheral
and timer. Its settings are builder methods, `with_frequency`, `with_oscillator`, and
`with_outputs`, and a channel's setting is a `Pwm` from `Pwm::servo`, `Pwm::duty`,
`Pwm::full_on`, `Pwm::full_off`, or `Pwm::from_counts`. `stepper::FourWire` and
`stepper::StepDir` are generic over any `OutputPin` and any `DelayNs`: lines from
`pamoja_hal::linux::output` and `linux::delay()` on a Raspberry Pi, and `PinScript` and
`DelayLog` from `pamoja_hal::script` in a test, which `release` hands back to be read. Every
call returns a `Result` whose error is a `DriverError`: `Command` with the reason when the part
cannot take what was asked, and `Bus` with the bus's or the pin's own error. Each driver is also
an `Actuator` from `pamoja-core`. The PCA9685 takes an `Output` naming a channel and a `Pwm`, a
stepper a signed count of steps, and `map_command` puts an angle or a `bool` in front of either,
so a profile's control loop drives them like any other output.

<!-- snippet: examples/guides/actuators.rs#example -->
From [`examples/guides/actuators.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/actuators.rs):

```rust
use pamoja_actuators::pca9685::{self, mode1, register, Pca9685, Pwm, INTERNAL_OSC_HZ};
use pamoja_actuators::stepper::{steps_for_degrees, Drive, FourWire, StepDir};
use pamoja_actuators::DriverError;
use pamoja_hal::bus::I2cBus;
use pamoja_hal::digital::PinState;
use pamoja_hal::i2c::I2c;
use pamoja_hal::script::{DelayLog, PinScript};
use pamoja_hal::sim::I2cPart;

// Where the rig's parts connect: the PCA9685 answers at 0x40 with its six address pins
// low, the tilt servo is on its channel 0, and the status LED on channel 15.
let address = pca9685::DEFAULT_I2C_ADDRESS;
const TILT: u8 = 0;
const STATUS: u8 = 15;
let glow = Pwm::duty(pca9685::COUNTS / 16);

// The rig with nothing plugged in: a PCA9685 that powers up and keeps its datasheet's
// rules the way the part does. On a Raspberry Pi the bus is I2cBus::open("/dev/i2c-1")
// and nothing after this statement changes.
let bus = I2cBus::simulated([pca9685::sim::part(address)]);

// A hobby servo wants a pulse every 20 ms, 50 Hz. The driver works the prescale out from
// that and writes it with the oscillator asleep, since only then does the part take it,
// then wakes the oscillator and waits the 500 us it needs to settle.
let mut controller = Pca9685::new(bus.clone(), address, bus.delay()).with_frequency(50);
controller.init()?;
println!(
    "controller   prescale {} for {:.1} Hz, awake after {} us",
    controller.prescale(),
    controller.frequency(),
    bus.waited_micros()
);

// A servo turns to the width of the pulse it is sent, and this one points the camera a
// little below level at 1300 us. The LED glows at a sixteenth of full brightness while
// the rig waits. Reading the channels back shows what the part now holds.
controller.set_channel(TILT, Pwm::servo(1_300, 50))?;
controller.set_channel(STATUS, glow)?;
let tilt = controller.channel(TILT)?;
let status = controller.channel(STATUS)?;
println!(
    "tilt         1300 us pulse, low at count {} of {}",
    tilt.off(),
    pca9685::COUNTS
);
println!(
    "status       glowing, high for {} of {} counts",
    status.off(),
    pca9685::COUNTS
);

// The slider: a 1.8-degree motor, 200 steps a turn, behind an A4988 with MS1 to MS3
// high, which splits each step into sixteen. A 20-tooth GT2 pulley pulls 40 mm of belt
// a turn, so 5 mm between frames is an eighth of a turn.
const SLIDER_STEPS_PER_TURN: u32 = 200 * 16;
const BELT_MM_PER_TURN: f32 = 40.0;
let slide = steps_for_degrees(360.0 * 5.0 / BELT_MM_PER_TURN, SLIDER_STEPS_PER_TURN);
let mut slider = StepDir::new(PinScript::new([]), PinScript::new([]), DelayLog::new())
    .with_step_interval(500);

// The pan head: a 28BYJ-48 through a ULN2003, half-stepped, 4096 half-steps a turn
// through its gearbox. With a camera on it, it steps every 4 ms, half the 500 Hz its
// pack is rated to start at with no load.
const PAN_STEPS_PER_TURN: u32 = 4096;
let pan_step = steps_for_degrees(2.0, PAN_STEPS_PER_TURN);
let coils = (
    PinScript::new([]),
    PinScript::new([]),
    PinScript::new([]),
    PinScript::new([]),
);
let mut pan = FourWire::new(coils, DelayLog::new(), Drive::HalfStep).with_step_interval(4_000);

// Four frames. The LED lights for each exposure, and between frames the rig slides and
// pans while it glows. The stepper lines record every level, and the delays count every
// wait without sleeping through it.
for frame in 1..=4 {
    controller.set_channel(STATUS, Pwm::full_on())?;
    println!(
        "frame {frame}      slider {:.1} mm, pan {:.2} degrees",
        slider.position() as f32 * BELT_MM_PER_TURN / SLIDER_STEPS_PER_TURN as f32,
        pan.position() as f32 * 360.0 / PAN_STEPS_PER_TURN as f32
    );
    controller.set_channel(STATUS, glow)?;
    if frame < 4 {
        slider.steps(slide)?;
        pan.steps(pan_step)?;
    }
}

// A four-wire motor draws current for as long as its coils hold, so the pan head
// drops them once the shoot is over.
pan.idle()?;
let slid = slider.position();
let panned = pan.position();
let ((step_line, direction_line), slider_delay) = slider.release();
let ((a, b, c, d), pan_delay) = pan.release();
let pulses = step_line
    .driven()
    .iter()
    .filter(|level| **level == PinState::High)
    .count();
println!(
    "slider       {pulses} pulses on STEP, DIR {:?}",
    direction_line.level()
);
println!(
    "pan head     {panned} half-steps, coils {:?} {:?} {:?} {:?}",
    a.level(),
    b.level(),
    c.level(),
    d.level()
);
println!(
    "moving       slider {} ms, pan head {} ms",
    slider_delay.total_millis(),
    pan_delay.total_millis()
);

// The part takes a new prescale only while its oscillator sleeps. Written while it
// runs, as a driver that skipped the sleep would write it, the value is dropped and the
// servos stay at 50 Hz.
let mut raw = bus.clone();
let fast = pca9685::prescale_for_frequency(1_000, INTERNAL_OSC_HZ);
raw.write(address, &[register::PRE_SCALE, fast])?;
let held: I2cPart = bus.part(address).ok_or("the controller left the bus")?;
println!(
    "prescale     written while awake, still {}",
    held.register(register::PRE_SCALE)
);

// A channel the part does not have is refused before anything reaches the bus.
match controller.set_channel(16, Pwm::full_on()) {
    Err(DriverError::Command(reason)) => println!("channel 16   {reason}"),
    other => println!("channel 16   {other:?}, which should never happen"),
}

// The shoot is over: every channel off in one transfer through the ALL_LED registers,
// then the oscillator asleep. The part keeps its registers while it sleeps.
controller.set_all(Pwm::full_off())?;
controller.sleep()?;
let parked: I2cPart = bus.part(address).ok_or("the controller left the bus")?;
let all_off = controller.channel(TILT)? == Pwm::full_off()
    && controller.channel(STATUS)? == Pwm::full_off();
let asleep = parked.register(register::MODE1) & mode1::SLEEP != 0;
println!(
    "parked       every channel {}, oscillator {}",
    if all_off { "off" } else { "still on" },
    if asleep { "asleep" } else { "running" }
);
```
<!-- end -->

## TypeScript

In TypeScript the drivers are in `@pamoja/actuators`. The part's constants and its simulated
twin sit on the lowercase `pca9685` object, `pca9685.defaultAddress`,
`pca9685.register.preScale`, and `pca9685.sim.part(address)`, and the setting builders on `pwm`
return a `Buffer` of a channel's four register bytes, which `pwm.counts` reads back as
`{ on, off }`. `Pca9685` takes its settings as an options object,
`{ frequencyHz, oscillatorHz, totemPole, inverted, changeOnAck }`, runs each call on a worker
thread, and returns promises that reject with the reason. `FourWire` and `StepDir` are written
in TypeScript over any object with a `drive(level)` method, a `GpioLine` or a `PinScript` from
`@pamoja/gpio`, and walk the same native coil sequence as the Rust driver. Their `step` and
`steps` return promises because they wait: on a `SleepDelay` from `@pamoja/hal` unless handed a
`DelayLog`, which the program keeps to read afterward.

<!-- snippet: bindings/node/guides/actuators.ts#example -->
From [`bindings/node/guides/actuators.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/actuators.ts):

```typescript
import {
  FourWire,
  Pca9685,
  StepDir,
  StepDrive,
  pca9685,
  pwm,
  stepsForDegrees,
} from '@pamoja/actuators'
import { PinLevel, PinScript } from '@pamoja/gpio'
import { DelayLog, I2cBus, I2cPart } from '@pamoja/hal'

// Where the rig's parts connect: the PCA9685 answers at 0x40 with its six address pins low, the
// tilt servo is on its channel 0, and the status LED on channel 15.
const address = pca9685.defaultAddress
const TILT = 0
const STATUS = 15
const glow = pwm.duty(pca9685.counts / 16)

async function main() {
  // The rig with nothing plugged in: a PCA9685 that powers up and keeps its datasheet's rules
  // the way the part does. On a Raspberry Pi the bus is I2cBus.open('/dev/i2c-1') and nothing
  // after this statement changes.
  const bus = I2cBus.simulated([pca9685.sim.part(address)])

  // A hobby servo wants a pulse every 20 ms, 50 Hz. The driver works the prescale out from that
  // and writes it with the oscillator asleep, since only then does the part take it, then wakes
  // the oscillator and waits the 500 us it needs to settle.
  const controller = new Pca9685(bus, address, { frequencyHz: 50 })
  await controller.init()
  console.log(
    `controller   prescale ${controller.prescale} for ${controller.frequency.toFixed(1)} Hz, ` +
      `awake after ${bus.waitedMicros} us`,
  )

  // A servo turns to the width of the pulse it is sent, and this one points the camera a little
  // below level at 1300 us. The LED glows at a sixteenth of full brightness while the rig waits.
  // Reading the channels back shows what the part now holds.
  await controller.setChannel(TILT, pwm.servo(1_300, 50))
  await controller.setChannel(STATUS, glow)
  const tilt = pwm.counts(await controller.channel(TILT))
  const status = pwm.counts(await controller.channel(STATUS))
  console.log(`tilt         1300 us pulse, low at count ${tilt.off} of ${pca9685.counts}`)
  console.log(`status       glowing, high for ${status.off} of ${pca9685.counts} counts`)

  // The slider: a 1.8-degree motor, 200 steps a turn, behind an A4988 with MS1 to MS3 high,
  // which splits each step into sixteen. A 20-tooth GT2 pulley pulls 40 mm of belt a turn, so
  // 5 mm between frames is an eighth of a turn.
  const sliderStepsPerTurn = 200 * 16
  const beltMmPerTurn = 40
  const slide = stepsForDegrees((360 * 5) / beltMmPerTurn, sliderStepsPerTurn)
  const sliderDelay = new DelayLog()
  const slider = new StepDir(new PinScript(), new PinScript(), {
    stepMicros: 500,
    delay: sliderDelay,
  })

  // The pan head: a 28BYJ-48 through a ULN2003, half-stepped, 4096 half-steps a turn through
  // its gearbox. With a camera on it, it steps every 4 ms, half the 500 Hz its pack is rated
  // to start at with no load.
  const panStepsPerTurn = 4096
  const panStep = stepsForDegrees(2, panStepsPerTurn)
  const coils = [new PinScript(), new PinScript(), new PinScript(), new PinScript()] as const
  const panDelay = new DelayLog()
  const pan = new FourWire(coils, StepDrive.HalfStep, { stepMicros: 4_000, delay: panDelay })

  // Four frames. The LED lights for each exposure, and between frames the rig slides and pans
  // while it glows. The stepper lines record every level, and the delays count every wait
  // without sleeping through it.
  for (let frame = 1; frame <= 4; frame += 1) {
    await controller.setChannel(STATUS, pwm.fullOn())
    const mm = (slider.position * beltMmPerTurn) / sliderStepsPerTurn
    const degrees = (pan.position * 360) / panStepsPerTurn
    console.log(`frame ${frame}      slider ${mm.toFixed(1)} mm, pan ${degrees.toFixed(2)} degrees`)
    await controller.setChannel(STATUS, glow)
    if (frame < 4) {
      await slider.steps(slide)
      await pan.steps(panStep)
    }
  }

  // A four-wire motor draws current for as long as its coils hold, so the pan head drops them
  // once the shoot is over.
  pan.idle()
  const [stepLine, directionLine] = slider.release()
  const [a, b, c, d] = pan.release()
  const pulses = stepLine.driven.filter((level) => level === PinLevel.High).length
  console.log(`slider       ${pulses} pulses on STEP, DIR ${directionLine.level}`)
  console.log(`pan head     ${pan.position} half-steps, coils ${a.level} ${b.level} ${c.level} ${d.level}`)
  console.log(`moving       slider ${sliderDelay.totalMillis} ms, pan head ${panDelay.totalMillis} ms`)

  // The part takes a new prescale only while its oscillator sleeps. Written while it runs, as a
  // driver that skipped the sleep would write it, the value is dropped and the servos stay at
  // 50 Hz.
  const fast = pca9685.prescaleForFrequency(1_000)
  bus.write(address, Buffer.from([pca9685.register.preScale, fast]))
  const held = bus.part(address)
  if (!(held instanceof I2cPart)) {
    throw new Error('the controller left the bus')
  }
  console.log(`prescale     written while awake, still ${held.register(pca9685.register.preScale)}`)

  // A channel the part does not have is refused before anything reaches the bus.
  try {
    await controller.setChannel(16, pwm.fullOn())
    console.log('channel 16   accepted, which should never happen')
  } catch (error) {
    console.log(`channel 16   ${(error as Error).message}`)
  }

  // The shoot is over: every channel off in one transfer through the ALL_LED registers, then the
  // oscillator asleep. The part keeps its registers while it sleeps.
  await controller.setAll(pwm.fullOff())
  await controller.sleep()
  const parked = bus.part(address) as I2cPart
  const allOff =
    (await controller.channel(TILT)).equals(pwm.fullOff()) &&
    (await controller.channel(STATUS)).equals(pwm.fullOff())
  const asleep = (parked.register(pca9685.register.mode1) & pca9685.mode1.sleep) !== 0
  console.log(
    `parked       every channel ${allOff ? 'off' : 'still on'}, ` +
      `oscillator ${asleep ? 'asleep' : 'running'}`,
  )

  return {
    bus,
    controller,
    tilt,
    status,
    slide,
    panStep,
    slider,
    pan,
    pulses,
    sliderDelay,
    panDelay,
    held,
    fast,
    parked,
    allOff,
    asleep,
  }
}

main()
```
<!-- end -->

## Python

In Python the drivers are `Pca9685`, `FourWire`, and `StepDir` in `pamoja.actuators`. The
part's constants and its simulated twin sit on the module-level `pca9685` object,
`pca9685.DEFAULT_ADDRESS`, `pca9685.REGISTER_PRE_SCALE`, and `pca9685.sim.part(address)`, and
the setting builders on `pwm` return a channel's four register bytes as `bytes`. Settings are
keyword arguments: `frequency_hz=` for the controller, and `step_micros=`, `pulse_micros=`, and
`delay=` for the steppers. The controller's calls release the interpreter while the bus is
busy. The steppers take any object with a `drive(level)` method, a `GpioLine` or a `PinScript`
from `pamoja.gpio`, and wait with a `SleepDelay` from `pamoja.hal` unless handed a `DelayLog`. A
channel the part does not have raises `ValueError`, and a bus failure raises `PamojaError` from
`pamoja.core`.

<!-- snippet: bindings/python/guides/actuators.py#example -->
From [`bindings/python/guides/actuators.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/actuators.py):

```python
from pamoja.actuators import Drive, FourWire, Pca9685, StepDir, pca9685, pwm, steps_for_degrees
from pamoja.gpio import Level, PinScript
from pamoja.hal import DelayLog, I2cBus

# Where the rig's parts connect: the PCA9685 answers at 0x40 with its six address pins low, the
# tilt servo is on its channel 0, and the status LED on channel 15.
ADDRESS = pca9685.DEFAULT_ADDRESS
TILT = 0
STATUS = 15
GLOW = pwm.duty(pca9685.COUNTS // 16)

# The rig with nothing plugged in: a PCA9685 that powers up and keeps its datasheet's rules the
# way the part does. On a Raspberry Pi the bus is I2cBus.open("/dev/i2c-1") and nothing after
# this statement changes.
bus = I2cBus.simulated([pca9685.sim.part(ADDRESS)])

# A hobby servo wants a pulse every 20 ms, 50 Hz. The driver works the prescale out from that and
# writes it with the oscillator asleep, since only then does the part take it, then wakes the
# oscillator and waits the 500 us it needs to settle.
controller = Pca9685(bus, ADDRESS, frequency_hz=50)
controller.init()
print(
    f"controller   prescale {controller.prescale} for {controller.frequency:.1f} Hz, "
    f"awake after {bus.waited_micros} us"
)

# A servo turns to the width of the pulse it is sent, and this one points the camera a little
# below level at 1300 us. The LED glows at a sixteenth of full brightness while the rig waits.
# Reading the channels back shows what the part now holds.
controller.set_channel(TILT, pwm.servo(1_300, 50))
controller.set_channel(STATUS, GLOW)
tilt = pwm.counts(controller.channel(TILT))
status = pwm.counts(controller.channel(STATUS))
print(f"tilt         1300 us pulse, low at count {tilt.off} of {pca9685.COUNTS}")
print(f"status       glowing, high for {status.off} of {pca9685.COUNTS} counts")

# The slider: a 1.8-degree motor, 200 steps a turn, behind an A4988 with MS1 to MS3 high, which
# splits each step into sixteen. A 20-tooth GT2 pulley pulls 40 mm of belt a turn, so 5 mm
# between frames is an eighth of a turn.
SLIDER_STEPS_PER_TURN = 200 * 16
BELT_MM_PER_TURN = 40.0
slide = steps_for_degrees(360.0 * 5.0 / BELT_MM_PER_TURN, SLIDER_STEPS_PER_TURN)
slider_delay = DelayLog()
slider = StepDir(PinScript(), PinScript(), step_micros=500, delay=slider_delay)

# The pan head: a 28BYJ-48 through a ULN2003, half-stepped, 4096 half-steps a turn through its
# gearbox. With a camera on it, it steps every 4 ms, half the 500 Hz its pack is rated to start
# at with no load.
PAN_STEPS_PER_TURN = 4096
pan_step = steps_for_degrees(2.0, PAN_STEPS_PER_TURN)
coils = (PinScript(), PinScript(), PinScript(), PinScript())
pan_delay = DelayLog()
pan = FourWire(coils, Drive.HALF_STEP, step_micros=4_000, delay=pan_delay)

# Four frames. The LED lights for each exposure, and between frames the rig slides and pans
# while it glows. The stepper lines record every level, and the delays count every wait without
# sleeping through it.
for frame in range(1, 5):
    controller.set_channel(STATUS, pwm.full_on())
    mm = slider.position * BELT_MM_PER_TURN / SLIDER_STEPS_PER_TURN
    degrees = pan.position * 360.0 / PAN_STEPS_PER_TURN
    print(f"frame {frame}      slider {mm:.1f} mm, pan {degrees:.2f} degrees")
    controller.set_channel(STATUS, GLOW)
    if frame < 4:
        slider.steps(slide)
        pan.steps(pan_step)

# A four-wire motor draws current for as long as its coils hold, so the pan head drops them once
# the shoot is over.
pan.idle()
step_line, direction_line = slider.release()
a, b, c, d = pan.release()
pulses = step_line.driven.count(Level.HIGH)
print(f"slider       {pulses} pulses on STEP, DIR {direction_line.level.value}")
levels = " ".join(line.level.value for line in (a, b, c, d))
print(f"pan head     {pan.position} half-steps, coils {levels}")
print(f"moving       slider {slider_delay.total_millis} ms, pan head {pan_delay.total_millis} ms")

# The part takes a new prescale only while its oscillator sleeps. Written while it runs, as a
# driver that skipped the sleep would write it, the value is dropped and the servos stay at
# 50 Hz.
fast = pca9685.prescale_for_frequency(1_000)
bus.write(ADDRESS, bytes([pca9685.REGISTER_PRE_SCALE, fast]))
held = bus.part(ADDRESS)
print(f"prescale     written while awake, still {held.register(pca9685.REGISTER_PRE_SCALE)}")

# A channel the part does not have is refused before anything reaches the bus.
try:
    controller.set_channel(16, pwm.full_on())
    print("channel 16   accepted, which should never happen")
except ValueError as error:
    print(f"channel 16   {error}")

# The shoot is over: every channel off in one transfer through the ALL_LED registers, then the
# oscillator asleep. The part keeps its registers while it sleeps.
controller.set_all(pwm.full_off())
controller.sleep()
parked = bus.part(ADDRESS)
all_off = all(controller.channel(channel) == pwm.full_off() for channel in (TILT, STATUS))
asleep = parked.register(pca9685.REGISTER_MODE1) & pca9685.MODE1_SLEEP != 0
print(
    f"parked       every channel {'off' if all_off else 'still on'}, "
    f"oscillator {'asleep' if asleep else 'running'}"
)
```
<!-- end -->

## C#

In C# the drivers are `Pca9685`, `FourWire<TLine>`, and `StepDir<TLine>` in `Pamoja.Actuators`.
The part's constants are static members of `Pca9685`, `Pca9685.DefaultAddress`,
`Pca9685.Register.PreScale`, and `Pca9685.Mode1.Sleep`, its simulated twin is
`Pca9685.Sim.Part(address)`, and the setting builders on the static `Pwm` class return
`byte[]`. Settings are optional constructor parameters, best passed by name: `frequencyHz:` and
`outputs:` for the controller, and `stepMicros:`, `pulseMicros:`, and `delay:` for the
steppers. `TLine` is the lines' type, `GpioLine` on a board and `PinScript` in a test, and
anything that implements `IOutputLine` will do. The steppers wait on an `IDelay`, a
`SleepDelay` unless handed a `DelayLog`, both from `Pamoja.Hal`. The bus, the part, the
controller, and a `FourWire` hold native handles and are `IDisposable`. A failure throws
`PamojaException` with the reason as its message.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ActuatorsGuide.cs):

```csharp
// Where the rig's parts connect: the PCA9685 answers at 0x40 with its six address pins
// low, the tilt servo is on its channel 0, and the status LED on channel 15.
const byte Address = Pca9685.DefaultAddress;
const byte Tilt = 0;
const byte Status = 15;
byte[] glow = Pwm.Duty(Pca9685.Counts / 16);

// The rig with nothing plugged in: a PCA9685 that powers up and keeps its datasheet's
// rules the way the part does. The bus keeps a copy of the part, so the program lets go
// of its own. On a Raspberry Pi the bus is I2cBus.Open("/dev/i2c-1") and nothing after
// this statement changes.
I2cBus bus;
using (I2cPart part = Pca9685.Sim.Part(Address))
{
    bus = I2cBus.Simulated(part);
}

using (bus)
{
    // A hobby servo wants a pulse every 20 ms, 50 Hz. The driver works the prescale out
    // from that and writes it with the oscillator asleep, since only then does the part
    // take it, then wakes the oscillator and waits the 500 us it needs to settle.
    using var controller = new Pca9685(bus, Address, frequencyHz: 50);
    controller.Init();
    Console.WriteLine(Invariant(
        $"controller   prescale {controller.Prescale} for {controller.Frequency:F1} Hz, awake after {bus.WaitedMicros} us"));

    // A servo turns to the width of the pulse it is sent, and this one points the camera
    // a little below level at 1300 us. The LED glows at a sixteenth of full brightness
    // while the rig waits. Reading the channels back shows what the part now holds.
    controller.SetChannel(Tilt, Pwm.Servo(1_300, 50));
    controller.SetChannel(Status, glow);
    var tilt = Pwm.Counts(controller.Channel(Tilt));
    var status = Pwm.Counts(controller.Channel(Status));
    Console.WriteLine($"tilt         1300 us pulse, low at count {tilt.Off} of {Pca9685.Counts}");
    Console.WriteLine($"status       glowing, high for {status.Off} of {Pca9685.Counts} counts");

    // The slider: a 1.8-degree motor, 200 steps a turn, behind an A4988 with MS1 to MS3
    // high, which splits each step into sixteen. A 20-tooth GT2 pulley pulls 40 mm of
    // belt a turn, so 5 mm between frames is an eighth of a turn.
    const uint SliderStepsPerTurn = 200 * 16;
    const float BeltMmPerTurn = 40.0f;
    int slide = Stepper.StepsForDegrees(360.0f * 5.0f / BeltMmPerTurn, SliderStepsPerTurn);
    var sliderDelay = new DelayLog();
    var slider = new StepDir<PinScript>(new PinScript(), new PinScript(), stepMicros: 500, delay: sliderDelay);

    // The pan head: a 28BYJ-48 through a ULN2003, half-stepped, 4096 half-steps a turn
    // through its gearbox. With a camera on it, it steps every 4 ms, half the 500 Hz its
    // pack is rated to start at with no load.
    const uint PanStepsPerTurn = 4096;
    int panStep = Stepper.StepsForDegrees(2.0f, PanStepsPerTurn);
    var coils = (new PinScript(), new PinScript(), new PinScript(), new PinScript());
    var panDelay = new DelayLog();
    using var pan = new FourWire<PinScript>(coils, StepDrive.HalfStep, stepMicros: 4_000, delay: panDelay);

    // Four frames. The LED lights for each exposure, and between frames the rig slides
    // and pans while it glows. The stepper lines record every level, and the delays count
    // every wait without sleeping through it.
    for (int frame = 1; frame <= 4; frame++)
    {
        controller.SetChannel(Status, Pwm.FullOn());
        float mm = slider.Position * BeltMmPerTurn / SliderStepsPerTurn;
        float degrees = pan.Position * 360.0f / PanStepsPerTurn;
        Console.WriteLine(Invariant($"frame {frame}      slider {mm:F1} mm, pan {degrees:F2} degrees"));
        controller.SetChannel(Status, glow);
        if (frame < 4)
        {
            slider.Steps(slide);
            pan.Steps(panStep);
        }
    }

    // A four-wire motor draws current for as long as its coils hold, so the pan head
    // drops them once the shoot is over.
    pan.Idle();
    var (stepLine, directionLine) = slider.Release();
    var (a, b, c, d) = pan.Release();
    int pulses = stepLine.Driven.Count(level => level == PinLevel.High);
    Console.WriteLine($"slider       {pulses} pulses on STEP, DIR {directionLine.Level}");
    Console.WriteLine($"pan head     {pan.Position} half-steps, coils {a.Level} {b.Level} {c.Level} {d.Level}");
    Console.WriteLine($"moving       slider {sliderDelay.TotalMillis} ms, pan head {panDelay.TotalMillis} ms");

    // The part takes a new prescale only while its oscillator sleeps. Written while it
    // runs, as a driver that skipped the sleep would write it, the value is dropped and
    // the servos stay at 50 Hz.
    byte fast = Pca9685.PrescaleForFrequency(1_000);
    bus.Write(Address, [Pca9685.Register.PreScale, fast]);
    byte stillHeld;
    using (I2cPart held = bus.Part<I2cPart>(Address)!)
    {
        stillHeld = held.Register(Pca9685.Register.PreScale);
    }

    Console.WriteLine($"prescale     written while awake, still {stillHeld}");

    // A channel the part does not have is refused before anything reaches the bus.
    try
    {
        controller.SetChannel(16, Pwm.FullOn());
        Console.WriteLine("channel 16   accepted, which should never happen");
    }
    catch (PamojaException error)
    {
        Console.WriteLine($"channel 16   {error.Message}");
    }

    // The shoot is over: every channel off in one transfer through the ALL_LED
    // registers, then the oscillator asleep. The part keeps its registers while it
    // sleeps.
    controller.SetAll(Pwm.FullOff());
    controller.Sleep();
    byte parkedMode;
    using (I2cPart parked = bus.Part<I2cPart>(Address)!)
    {
        parkedMode = parked.Register(Pca9685.Register.Mode1);
    }

    bool allOff = controller.Channel(Tilt).SequenceEqual(Pwm.FullOff())
        && controller.Channel(Status).SequenceEqual(Pwm.FullOff());
    bool asleep = (parkedMode & Pca9685.Mode1.Sleep) != 0;
    Console.WriteLine(
        $"parked       every channel {(allOff ? "off" : "still on")}, oscillator {(asleep ? "asleep" : "running")}");
```
<!-- end -->

## On a board

The same drivers on a Raspberry Pi: a [PCA9685](../hardware.md#pca9685) breakout on the
header's I2C bus with the tilt servo and an LED, and a 28BYJ-48 on a
[ULN2003](../hardware.md#uln2003) board, the pack the hardware page lists, on four GPIO lines as
the pan head. The program runs a real shoot, twelve frames across a 90-degree pan, one every
five seconds, with the LED lit while the head holds still for the camera. Then it pans back to
the start and parks.

| PCA9685 breakout | Raspberry Pi |
| --- | --- |
| VCC | a 3V3 pin |
| GND | a ground pin |
| SDA | GPIO2 |
| SCL | GPIO3 |
| V+ | a 5 V supply, its ground joined to the Pi's |
| channel 0 | the tilt servo |
| channel 15 | an LED |

| ULN2003 board | Raspberry Pi |
| --- | --- |
| IN1 | GPIO5 |
| IN2 | GPIO6 |
| IN3 | GPIO13 |
| IN4 | GPIO26 |
| + and - | the same 5 V supply |

VCC is the breakout's logic supply, and on
[Adafruit's board](https://learn.adafruit.com/16-channel-pwm-servo-driver/pinouts) the pull-up
resistors on SDA and SCL go to it as well, so it goes to 3V3: the Pi's pins take 3.3 V, not 5 V.
The servos draw from V+, which Adafruit rates at 5 to 6 V. A servo draws the most current as it
starts to move and while it holds against a load, far more than the header should supply, so
V+ and the motor board run from a supply of their own, with its ground joined to the Pi's so
every signal shares one reference. Adafruit's board puts a 220 ohm resistor in series with
every output, so an LED plugs straight into channel 15; on a board without them, add one.
[Turn the I2C interface on](../boards/raspberry-pi.md#turning-the-buses-on) and check
`i2cdetect -y 1` before running anything: it should show `40`, and `70` as well until the driver
has started the part once. The four coil lines need nothing turned on.

### Rust

<!-- snippet: examples/boards/raspberry-pi/src/bin/rig.rs#example -->
From [`examples/boards/raspberry-pi/src/bin/rig.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/boards/raspberry-pi/src/bin/rig.rs):

```rust
use pamoja_actuators::pca9685::{self, Pca9685, Pwm};
use pamoja_actuators::stepper::{steps_for_degrees, Drive, FourWire};
use pamoja_hal::bus::I2cBus;
use pamoja_hal::digital::PinState;
use pamoja_hal::linux;

/// The GPIO chip the header's lines live on, numbered as the BCM numbers.
const CHIP: &str = "/dev/gpiochip0";

/// The lines the ULN2003 board's IN1 to IN4 are wired to, coils A to D.
const PAN_LINES: [u32; 4] = [5, 6, 13, 26];

/// The PCA9685 channels the tilt servo and the LED are plugged into.
const TILT: u8 = 0;
const STATUS: u8 = 15;

/// The shoot: twelve frames across a 90-degree pan, one every five seconds.
const FRAMES: u32 = 12;
const SWEEP_DEGREES: f32 = 90.0;
const INTERVAL: Duration = Duration::from_secs(5);

fn main() -> Result<(), Box<dyn Error>> {
    // The PCA9685 on the header's I2C bus, at 50 Hz for the servo. The first channel written
    // runs the datasheet's start-up: sleep, prescale, wake, 500 us, restart.
    let bus = I2cBus::open("/dev/i2c-1")?;
    let mut controller =
        Pca9685::new(bus.clone(), pca9685::DEFAULT_I2C_ADDRESS, bus.delay()).with_frequency(50);
    let glow = Pwm::duty(pca9685::COUNTS / 16);
    controller.set_channel(TILT, Pwm::servo(1_300, 50))?;
    controller.set_channel(STATUS, glow)?;

    // Each coil line is taken low, so the motor holds nothing until its first step. The
    // 28BYJ-48 turns 4096 half-steps a turn through its gearbox, and with a camera on it,
    // it steps every 4 ms.
    let open = |line| linux::output(CHIP, line, "pamoja-pan", PinState::Low);
    let coils = (
        open(PAN_LINES[0])?,
        open(PAN_LINES[1])?,
        open(PAN_LINES[2])?,
        open(PAN_LINES[3])?,
    );
    let mut pan = FourWire::new(coils, linux::delay(), Drive::HalfStep).with_step_interval(4_000);
    let per_frame = steps_for_degrees(SWEEP_DEGREES / (FRAMES - 1) as f32, 4096);

    // The LED lights while the head holds still for the camera, and glows while it moves.
    for frame in 1..=FRAMES {
        controller.set_channel(STATUS, Pwm::full_on())?;
        println!(
            "frame {frame:2}  pan {:6.2} degrees",
            pan.position() as f32 * 360.0 / 4096.0
        );
        thread::sleep(INTERVAL);
        controller.set_channel(STATUS, glow)?;
        if frame < FRAMES {
            pan.steps(per_frame)?;
        }
    }

    // Back to the start, then everything off: the coils dropped, every channel off in one
    // transfer, and the oscillator asleep.
    pan.steps(-pan.position())?;
    pan.idle()?;
    controller.set_all(Pwm::full_off())?;
    controller.sleep()?;
    println!("parked at {} half-steps", pan.position());
    Ok(())
}
```
<!-- end -->

```sh
cd examples/boards/raspberry-pi
cargo run --release --bin rig
```

### TypeScript

<!-- snippet: bindings/node/boards/raspberry-pi/rig.ts#example -->
From [`bindings/node/boards/raspberry-pi/rig.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/boards/raspberry-pi/rig.ts):

```typescript
import { setTimeout as sleep } from 'node:timers/promises'
import { FourWire, Pca9685, StepDrive, pca9685, pwm, stepsForDegrees } from '@pamoja/actuators'
import { GpioLine, PinLevel } from '@pamoja/gpio'
import { I2cBus } from '@pamoja/hal'

// The GPIO chip the header's lines live on, numbered as the BCM numbers.
const CHIP = '/dev/gpiochip0'

// The lines the ULN2003 board's IN1 to IN4 are wired to, coils A to D.
const PAN_LINES = [5, 6, 13, 26] as const

// The PCA9685 channels the tilt servo and the LED are plugged into.
const TILT = 0
const STATUS = 15

// The shoot: twelve frames across a 90-degree pan, one every five seconds.
const FRAMES = 12
const SWEEP_DEGREES = 90
const INTERVAL_MS = 5_000

async function main(): Promise<void> {
  // The PCA9685 on the header's I2C bus, at 50 Hz for the servo. The first channel written runs
  // the datasheet's start-up: sleep, prescale, wake, 500 us, restart.
  const bus = I2cBus.open('/dev/i2c-1')
  const controller = new Pca9685(bus, pca9685.defaultAddress, { frequencyHz: 50 })
  const glow = pwm.duty(pca9685.counts / 16)
  await controller.setChannel(TILT, pwm.servo(1_300, 50))
  await controller.setChannel(STATUS, glow)

  // Each coil line is taken low, so the motor holds nothing until its first step. The 28BYJ-48
  // turns 4096 half-steps a turn through its gearbox, and with a camera on it, it steps every
  // 4 ms.
  const open = (line: number) => GpioLine.openOutput(CHIP, line, PinLevel.Low)
  const coils = [open(PAN_LINES[0]), open(PAN_LINES[1]), open(PAN_LINES[2]), open(PAN_LINES[3])] as const
  const pan = new FourWire(coils, StepDrive.HalfStep, { stepMicros: 4_000 })
  const perFrame = stepsForDegrees(SWEEP_DEGREES / (FRAMES - 1), 4096)

  // The LED lights while the head holds still for the camera, and glows while it moves.
  for (let frame = 1; frame <= FRAMES; frame += 1) {
    await controller.setChannel(STATUS, pwm.fullOn())
    const degrees = (pan.position * 360) / 4096
    console.log(`frame ${String(frame).padStart(2)}  pan ${degrees.toFixed(2).padStart(6)} degrees`)
    await sleep(INTERVAL_MS)
    await controller.setChannel(STATUS, glow)
    if (frame < FRAMES) {
      await pan.steps(perFrame)
    }
  }

  // Back to the start, then everything off: the coils dropped, every channel off in one
  // transfer, and the oscillator asleep.
  await pan.steps(-pan.position)
  pan.idle()
  await controller.setAll(pwm.fullOff())
  await controller.sleep()
  console.log(`parked at ${pan.position} half-steps`)
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
```
<!-- end -->

```sh
npm --prefix bindings/node run boards
node bindings/node/build/boards/raspberry-pi/rig.js
```

### Python

<!-- snippet: bindings/python/boards/raspberry_pi/rig.py#example -->
From [`bindings/python/boards/raspberry_pi/rig.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/boards/raspberry_pi/rig.py):

```python
import time

from pamoja.actuators import Drive, FourWire, Pca9685, pca9685, pwm, steps_for_degrees
from pamoja.gpio import GpioLine, Level
from pamoja.hal import I2cBus

# The GPIO chip the header's lines live on, numbered as the BCM numbers.
CHIP = "/dev/gpiochip0"

# The lines the ULN2003 board's IN1 to IN4 are wired to, coils A to D.
PAN_LINES = (5, 6, 13, 26)

# The PCA9685 channels the tilt servo and the LED are plugged into.
TILT = 0
STATUS = 15

# The shoot: twelve frames across a 90-degree pan, one every five seconds.
FRAMES = 12
SWEEP_DEGREES = 90.0
INTERVAL_SECONDS = 5


def main() -> None:
    # The PCA9685 on the header's I2C bus, at 50 Hz for the servo. The first channel written
    # runs the datasheet's start-up: sleep, prescale, wake, 500 us, restart.
    bus = I2cBus.open("/dev/i2c-1")
    controller = Pca9685(bus, pca9685.DEFAULT_ADDRESS, frequency_hz=50)
    glow = pwm.duty(pca9685.COUNTS // 16)
    controller.set_channel(TILT, pwm.servo(1_300, 50))
    controller.set_channel(STATUS, glow)

    # Each coil line is taken low, so the motor holds nothing until its first step. The
    # 28BYJ-48 turns 4096 half-steps a turn through its gearbox, and with a camera on it, it
    # steps every 4 ms.
    a, b, c, d = (GpioLine.open_output(CHIP, line, Level.LOW) for line in PAN_LINES)
    pan = FourWire((a, b, c, d), Drive.HALF_STEP, step_micros=4_000)
    per_frame = steps_for_degrees(SWEEP_DEGREES / (FRAMES - 1), 4096)

    # The LED lights while the head holds still for the camera, and glows while it moves.
    for frame in range(1, FRAMES + 1):
        controller.set_channel(STATUS, pwm.full_on())
        print(f"frame {frame:2}  pan {pan.position * 360.0 / 4096:6.2f} degrees")
        time.sleep(INTERVAL_SECONDS)
        controller.set_channel(STATUS, glow)
        if frame < FRAMES:
            pan.steps(per_frame)

    # Back to the start, then everything off: the coils dropped, every channel off in one
    # transfer, and the oscillator asleep.
    pan.steps(-pan.position)
    pan.idle()
    controller.set_all(pwm.full_off())
    controller.sleep()
    print(f"parked at {pan.position} half-steps")


if __name__ == "__main__":
    main()
```
<!-- end -->

```sh
python bindings/python/boards/raspberry_pi/rig.py
```

### C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Rig.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Rig.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Boards/RaspberryPi/Rig.cs):

```csharp
using System.Globalization;

using Pamoja.Actuators;
using Pamoja.Gpio;
using Pamoja.Hal;

namespace Boards.RaspberryPi;

/// <summary>
/// A pan and tilt head for a time-lapse: a tilt servo and a status LED on a PCA9685 board on the
/// header's I2C bus, and a 28BYJ-48 pan motor on four GPIO lines through a ULN2003 board. Wire the
/// PCA9685 board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, with the servo on
/// channel 0, an LED on channel 15, and V+ from a 5 V supply whose ground is joined to the Pi's.
/// Wire the ULN2003 board's IN1 to IN4 to GPIO5, GPIO6, GPIO13, and GPIO26.
/// </summary>
public static class Rig
{
    // The GPIO chip the header's lines live on, numbered as the BCM numbers.
    private const string Chip = "/dev/gpiochip0";

    // The PCA9685 channels the tilt servo and the LED are plugged into.
    private const byte Tilt = 0;
    private const byte Status = 15;

    // The shoot: twelve frames across a 90-degree pan, one every five seconds.
    private const int Frames = 12;
    private const float SweepDegrees = 90.0f;

    // The lines the ULN2003 board's IN1 to IN4 are wired to, coils A to D.
    private static readonly uint[] PanLines = [5, 6, 13, 26];

    private static readonly TimeSpan Interval = TimeSpan.FromSeconds(5);

    /// <summary>Runs the shoot, then parks the head.</summary>
    public static void Run()
    {
        // The PCA9685 on the header's I2C bus, at 50 Hz for the servo. The first channel
        // written runs the datasheet's start-up: sleep, prescale, wake, 500 us, restart.
        using I2cBus bus = I2cBus.Open("/dev/i2c-1");
        using var controller = new Pca9685(bus, Pca9685.DefaultAddress, frequencyHz: 50);
        byte[] glow = Pwm.Duty(Pca9685.Counts / 16);
        controller.SetChannel(Tilt, Pwm.Servo(1_300, 50));
        controller.SetChannel(Status, glow);

        // Each coil line is taken low, so the motor holds nothing until its first step. The
        // 28BYJ-48 turns 4096 half-steps a turn through its gearbox, and with a camera on it,
        // it steps every 4 ms.
        using GpioLine a = GpioLine.OpenOutput(Chip, PanLines[0], PinLevel.Low);
        using GpioLine b = GpioLine.OpenOutput(Chip, PanLines[1], PinLevel.Low);
        using GpioLine c = GpioLine.OpenOutput(Chip, PanLines[2], PinLevel.Low);
        using GpioLine d = GpioLine.OpenOutput(Chip, PanLines[3], PinLevel.Low);
        using var pan = new FourWire<GpioLine>((a, b, c, d), StepDrive.HalfStep, stepMicros: 4_000);
        int perFrame = Stepper.StepsForDegrees(SweepDegrees / (Frames - 1), 4096);

        // The LED lights while the head holds still for the camera, and glows while it moves.
        for (int frame = 1; frame <= Frames; frame++)
        {
            controller.SetChannel(Status, Pwm.FullOn());
            Console.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"frame {frame,2}  pan {pan.Position * 360.0f / 4096,6:F2} degrees"));
            Thread.Sleep(Interval);
            controller.SetChannel(Status, glow);
            if (frame < Frames)
            {
                pan.Steps(perFrame);
            }
        }

        // Back to the start, then everything off: the coils dropped, every channel off in one
        // transfer, and the oscillator asleep.
        pan.Steps(-pan.Position);
        pan.Idle();
        controller.SetAll(Pwm.FullOff());
        controller.Sleep();
        Console.WriteLine($"parked at {pan.Position} half-steps");
    }
}
```
<!-- end -->

```sh
dotnet run --project bindings/dotnet/samples/Pamoja.Boards -- raspberry-pi/rig
```

## Values at a glance

**The PCA9685's registers**, from its datasheet, Rev. 4. A channel's four registers hold a
12-bit count at which the output goes high and one at which it goes low, each with a flag in
bit 4 of its high byte that holds the output fully on or fully off:

| Register | Address | At power-up | What it holds |
| --- | --- | --- | --- |
| MODE1 | `0x00` | `0x11`, asleep and answering All Call | RESTART, EXTCLK, auto-increment, SLEEP, and which addresses the part answers |
| MODE2 | `0x01` | `0x04`, totem-pole outputs | output inversion, when outputs change, the output stage |
| LEDn_ON_L to LEDn_OFF_H | `0x06` + 4 × n | on at 0, with the full-off flag set | channel n's on count, off count, and flags |
| ALL_LED_ON_L to ALL_LED_OFF_H | `0xFA` to `0xFD` | reads 0 | one write loads every channel |
| PRE_SCALE | `0xFE` | `0x1E`, the 200 Hz the datasheet names | the frequency, written only while SLEEP is set |

**What a frequency comes out as.** The prescale is 25 MHz / (4096 × rate) - 1, rounded, from 3
to 255, and every channel shares it. The part then runs at 25 MHz / (4096 × (prescale + 1)),
which is why the 200 Hz the datasheet names for the power-up prescale is 196.89 Hz:

| Asked for | Prescale | Runs at |
| --- | --- | --- |
| 24 Hz, the slowest | 253 | 24.03 Hz |
| 50 Hz, for hobby servos | 121 | 50.03 Hz |
| 60 Hz | 101 | 59.84 Hz |
| 100 Hz | 60 | 100.06 Hz |
| 200 Hz, the power-up rate | 30 | 196.89 Hz |
| 333 Hz | 17 | 339.08 Hz |
| 1000 Hz | 5 | 1017.25 Hz |
| 1526 Hz, the fastest | 3 | 1525.88 Hz |

**Servo pulses at 50 Hz.** A period is 20 ms, so one count is 4.88 µs:

| Pulse | Goes low at count |
| --- | --- |
| 500 µs | 102 |
| 1000 µs | 204 |
| 1300 µs, the rig's tilt | 266 |
| 1500 µs, the usual center | 307 |
| 2000 µs | 409 |
| 2500 µs | 512 |

Which pulse widths reach which angles belongs to the servo: 1000 to 2000 µs is the usual range,
and a pulse past a servo's end stops drives it into them. To command an angle rather than a
width, the helpers' `ServoMap` turns one into the other, as the
[motion guide's arm](motion.md#on-a-board) does.

**A channel's settings.** The datasheet says the on and off counts should never hold the same
value, and that full off wins when both flags are set, so 0 % and 100 % are always the flags:

| Built with | What the channel does |
| --- | --- |
| `duty(n)`, n from 1 to 4095 | high from count 0 until count n |
| `duty(0)`, or `full_off()` | held low by the full-off flag, the power-up state |
| `duty(4096)` or more, or `full_on()` | held high by the full-on flag |
| `servo(width, rate)` | a pulse of that width, `width × 4096 × rate / 1,000,000` counts, each period |
| `from_counts(on, off)` | high from count `on` until count `off`, to offset one channel's pulse from another's |

**The coil patterns**, bit 3 for coil A on IN1 down to bit 0 for coil D on IN4. A four-wire
driver starts at the first pattern without driving it; its first step forward drives the
second:

| Drive | Steps a cycle | Patterns, forward | Coils on |
| --- | --- | --- | --- |
| Wave | 4 | `1000`, `0100`, `0010`, `0001` | one: the least torque and the least current |
| Full step | 4 | `1100`, `0110`, `0011`, `1001` | two: the most torque |
| Half step | 8 | `1000`, `1100`, `0100`, `0110`, `0010`, `0011`, `0001`, `1001` | one, then two: twice the resolution |

**Step and direction timing**, from the A4988 datasheet, Rev. 8, and the DRV8825's, SLVSA73F.
The drivers hold the direction and the pulse for 10 µs each unless told otherwise, which covers
both chips:

| | A4988 | DRV8825 |
| --- | --- | --- |
| STEP high, at least | 1 µs | 1.9 µs |
| STEP low, at least | 1 µs | 1.9 µs |
| DIR settled before the rising edge | 200 ns | 650 ns |
| Fastest step rate | not given | 250 kHz |
| Wait after SLEEP goes high | 1 ms | 1.7 ms |
| Microsteps | full, 1/2, 1/4, 1/8, 1/16 | full, 1/2, 1/4, 1/8, 1/16, 1/32 |

**The microstep pins.** The A4988 reads MS1, MS2, and MS3, and the DRV8825 MODE0, MODE1, and
MODE2, each with its own pull-downs, so a pin left open reads low:

| Microsteps | A4988 MS1, MS2, MS3 | DRV8825 MODE2, MODE1, MODE0 |
| --- | --- | --- |
| full step | low, low, low | low, low, low, at 71 % current |
| 1/2 | high, low, low | low, low, high |
| 1/4 | low, high, low | low, high, low |
| 1/8 | high, high, low | low, high, high |
| 1/16 | high, high, high | high, low, low |
| 1/32 | not offered | high, low, high, or high, high, either |

**The rig's motors.** Steps a turn come from the motor, and a distance from what it turns:

| Motor | Steps a turn | Where the number comes from |
| --- | --- | --- |
| 28BYJ-48, half-stepped | 4096 | 5.625 degrees a half-step through a 1/64 gearbox, from [Seeed's page for the pack](https://wiki.seeedstudio.com/Gear_Stepper_Motor_Driver_Pack/) |
| 1.8-degree motor, full steps | 200 | its step angle |
| the same at 1/16 | 3200 | 200 × 16 |

A GT2 belt has a 2 mm pitch, so a 20-tooth pulley pulls 40 mm a turn, and at sixteen microsteps
a step the slider moves 80 microsteps a millimeter.

**Settings each driver takes.** A setting left out keeps the default in the last column:

### Rust

Builder methods, each returning the driver:

| Driver | Setting | Method | Default |
| --- | --- | --- | --- |
| `Pca9685` | frequency | `with_frequency(hz)` | 200 Hz |
| `Pca9685` | the clock it divides | `with_oscillator(hz)` | `INTERNAL_OSC_HZ`, 25 MHz |
| `Pca9685` | output wiring | `with_outputs(Outputs { totem_pole, inverted, change_on_ack })` | totem-pole, not inverted, changing on the stop |
| `FourWire` | pause after each step | `with_step_interval(micros)` | `DEFAULT_STEP_MICROS`, 2000 |
| `StepDir` | pulse width | `with_pulse_width(micros)` | `DEFAULT_PULSE_MICROS`, 10 |
| `StepDir` | pause after each step | `with_step_interval(micros)` | `DEFAULT_STEP_MICROS`, 2000 |

### TypeScript

Fields of the options object each constructor takes last:

| Driver | Fields | Defaults |
| --- | --- | --- |
| `Pca9685` | `frequencyHz`, `oscillatorHz`, `totemPole`, `inverted`, `changeOnAck` | 200, 25000000, `true`, `false`, `false` |
| `FourWire` | `stepMicros`, `delay` | `stepper.defaultStepMicros`, a `SleepDelay` |
| `StepDir` | `pulseMicros`, `stepMicros`, `delay` | `stepper.defaultPulseMicros`, `stepper.defaultStepMicros`, a `SleepDelay` |

### Python

Keyword arguments to the constructor:

| Driver | Arguments | Defaults |
| --- | --- | --- |
| `Pca9685` | `frequency_hz`, `oscillator_hz`, `totem_pole`, `inverted`, `change_on_ack` | 200, 25000000, `True`, `False`, `False` |
| `FourWire` | `step_micros`, `delay` | `stepper.DEFAULT_STEP_MICROS`, a `SleepDelay` |
| `StepDir` | `pulse_micros`, `step_micros`, `delay` | `stepper.DEFAULT_PULSE_MICROS`, `stepper.DEFAULT_STEP_MICROS`, a `SleepDelay` |

### C#

Optional parameters of the constructor, best passed by name:

| Driver | Parameters | Defaults |
| --- | --- | --- |
| `Pca9685` | `frequencyHz`, `oscillatorHz`, `outputs` | 200, `Pca9685.InternalOscHz`, `Pca9685Outputs.PowerOn` |
| `FourWire<TLine>` | `stepMicros`, `delay` | `Stepper.DefaultStepMicros`, a `SleepDelay` |
| `StepDir<TLine>` | `pulseMicros`, `stepMicros`, `delay` | `Stepper.DefaultPulseMicros`, `Stepper.DefaultStepMicros`, a `SleepDelay` |

<!-- languages end -->

**The same calls in each language:**

| To | Rust | TypeScript | Python | C# |
| --- | --- | --- | --- | --- |
| build the controller | `Pca9685::new(bus.clone(), a, bus.delay())` | `new Pca9685(bus, a, settings)` | `Pca9685(bus, a, **settings)` | `new Pca9685(bus, a, ...)` |
| start it | `init()` | `await init()` | `init()` | `Init()` |
| load a channel | `set_channel(n, pwm)` | `await setChannel(n, pwm)` | `set_channel(n, pwm)` | `SetChannel(n, pwm)` |
| read one back | `channel(n)` | `await channel(n)` | `channel(n)` | `Channel(n)` |
| load every channel | `set_all(pwm)` | `await setAll(pwm)` | `set_all(pwm)` | `SetAll(pwm)` |
| stop and start the oscillator | `sleep()`, `wake()` | `await sleep()`, `await wake()` | `sleep()`, `wake()` | `Sleep()`, `Wake()` |
| reset every PCA9685 on the bus | `software_reset()` | `await softwareReset()` | `software_reset()` | `SoftwareReset()` |
| stand one up | `pca9685::sim::part(a)` | `pca9685.sim.part(a)` | `pca9685.sim.part(a)` | `Pca9685.Sim.Part(a)` |
| a servo pulse | `Pwm::servo(1_500, 50)` | `pwm.servo(1500)` | `pwm.servo(1500)` | `Pwm.Servo(1500)` |
| drive four coil lines | `FourWire::new((a, b, c, d), delay, drive)` | `new FourWire([a, b, c, d], drive)` | `FourWire((a, b, c, d), drive)` | `new FourWire<GpioLine>((a, b, c, d), drive)` |
| drive a step and direction chip | `StepDir::new(step, dir, delay)` | `new StepDir(step, dir)` | `StepDir(step, dir)` | `new StepDir<GpioLine>(step, dir)` |
| move | `steps(n)` | `await steps(n)` | `steps(n)` | `Steps(n)` |
| where it is | `position()` | `position` | `position` | `Position` |
| let four coils go | `idle()` | `idle()` | `idle()` | `Idle()` |
| an angle in steps | `steps_for_degrees(90.0, 200)` | `stepsForDegrees(90, 200)` | `steps_for_degrees(90, 200)` | `Stepper.StepsForDegrees(90, 200)` |
| count waits without sleeping | `DelayLog::new()` | `new DelayLog()` | `DelayLog()` | `new DelayLog()` |

**Underneath the drivers** is the encode half, which needs no bus: the prescale formula, a
channel's first register, the `Pwm` builders and reader, and the coil sequencer, `Sequencer` in
Rust and `Stepper` in the bindings. A microcontroller that reaches a PCA9685 some other way, or
a gateway that sends a node the bytes to write, calls these directly.

## When it goes wrong

What a driver refuses, and what it says:

| What happened | The message | What to check |
| --- | --- | --- |
| a channel past 15 | `the PCA9685 has sixteen channels, 0 to 15` | the channel number; the part has 0 to 15 |
| nothing answered at the address | `nothing answered at 0x40` on a simulated bus, a remote I/O error on a Raspberry Pi | the wiring, VCC, and the address jumpers; [Buses](hal.md#when-it-goes-wrong) lists the bus's own errors |
| the software reset on a simulated bus | `nothing answered at 0x00` | nothing; the general call reaches only real parts |
| a line that cannot be driven | the line's own error | the chip and line numbers, and whether another program holds the line |

How each language hands those over:

| Language | A refused command | A bus or line failure |
| --- | --- | --- |
| Rust | `Err(DriverError::Command(reason))` | `Err(DriverError::Bus(error))` |
| TypeScript | a rejected promise | a rejected promise from the controller, or the line's own error from `step` and `steps` |
| Python | `ValueError` | `PamojaError`, or the line's own exception |
| C# | `PamojaException` | `PamojaException`, or the line's own exception |

The mistakes that cost an afternoon:

- **The frequency will not change.** PRE_SCALE takes a write only while the oscillator sleeps,
  and ignores one at any other time, which is the scene in the example. The driver puts the
  part to sleep to write it; a program that writes the register itself has to do the same.
- **The servos twitch, or the Pi resets, when they move.** They are running from the Pi's 5 V
  pins. A servo draws the most current as it starts to move and while it holds a load, and
  that pulls down the supply the Pi runs on. Give V+ a supply of its own, with the grounds
  joined.
- **A servo buzzes at one end of its travel.** The pulse asks for more than its end stops
  allow, and it pushes against them. Narrow the range until it goes quiet.
- **`i2cdetect` shows `70` as well as `40`.** Every PCA9685 answers the All Call address,
  `0x70`, from power-up. The driver turns that off when it starts the part, so it goes away after
  the first run until the next power cycle. A second board on the bus needs its own address
  from its jumpers.
- **The LED flickers in the pictures.** A glow is PWM at the controller's one frequency, 50 Hz
  here for the servos, and a shutter faster than 1/50 s catches it partway through a period.
  Hold it at full on while a frame is exposed, as the example does.
- **The 28BYJ-48 hums and does not turn.** Either it is stepped faster than it can start with
  its load, or its coils are out of order. Seeed rates its pack to start above 500 steps a
  second with nothing on the shaft; lengthen the step interval. IN1 to IN4 have to be coils A to
  D in the motor's order, which the pack's keyed connector gets right.
- **A long pan lands short or long.** Steps a turn come from the gearbox's stated ratio. The
  error in one turn is small, but it adds up: put a mark on the shaft, count the steps for one
  full turn, and use that count.
- **A step goes the wrong way just after a change of direction.** The chip reads DIR on the
  rising edge of STEP and needs it settled first, 200 ns on an A4988 and 650 ns on a DRV8825.
  The drivers hold DIR for the pulse width before the edge, so this comes from code that drives
  the pins itself.
- **A stepper gets hot standing still.** A four-wire motor through a ULN2003 draws current
  whenever a coil is on, moving or not; `idle` lets the coils go. A step and direction chip
  holds its motor at the current limit set through VREF until its ENABLE pin is taken high or
  its SLEEP pin low.
- **The first steps after waking the chip go missing.** The A4988 needs 1 ms after SLEEP goes
  high before it takes a step, and the DRV8825 1.7 ms. The drivers do not drive SLEEP, so the
  program that does has to wait.

## Where next

<!-- table: next actuators -->
- [I2C, SPI, and GPIO](gpio.md): I2C address frames with reserved-range checks, the four SPI clock modes, active-high or active-low switches and contacts, and GPIO lines opened on a Linux board.
- [Rules](rules.md): Rules between nodes as a file.
- [Buses](hal.md): The embedded-hal traits every driver takes, a bit-banged 1-Wire bus, simulated parts and scripted buses that stand in for hardware, the Linux backends over i2c-dev, spidev, and the GPIO character device, one I2C bus and one serial port a program and its drivers share, and delays that sleep or only count.
- Beside it: [Hardware](../hardware.md).
- Also in Sensing and actuation: [Sensor drivers](sensors.md), [Your own device](device.md).
<!-- end -->

## Reference

<!-- table: reference actuators -->
- Rust: [`pamoja-actuators`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_actuators/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-actuators)
- TypeScript: [`@pamoja/actuators`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_actuators.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-actuators)
- Python: [`pamoja.actuators`](https://pamoja.molex.cloud/docs/reference/python/pamoja/actuators.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-actuators)
- C#: [`Pamoja.Actuators`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Actuators.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-actuators)
- Hardware: [PCA9685](https://pamoja.molex.cloud/docs/hardware.html#pca9685), [ULN2003A](https://pamoja.molex.cloud/docs/hardware.html#uln2003), [A4988](https://pamoja.molex.cloud/docs/hardware.html#a4988), [DRV8825](https://pamoja.molex.cloud/docs/hardware.html#drv8825)
<!-- end -->
