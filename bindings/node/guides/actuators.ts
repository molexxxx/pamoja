// The actuator-driver guide example: a motion-control time-lapse rig, with a PCA9685 holding the
// tilt servo and the status LED, a slider behind an A4988, and a pan head on a 28BYJ-48; see
// docs/guides/actuators.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
  // ANCHOR_END: example
  .then(check)

function check(seen: Awaited<ReturnType<typeof main>>): void {
  assert.equal(seen.controller.prescale, 121, 'round(25 MHz / 4096 / 50) - 1')
  assert.ok(Math.abs(seen.controller.frequency - 50.0288) < 1e-3, '25 MHz / (4096 * 122)')
  assert.equal(seen.tilt.off, 266, '1300 us of a 20 ms period in 4096 counts')
  assert.equal(seen.status.off, 256)
  assert.equal(seen.slide, 400, 'an eighth of 3200 microsteps')
  assert.equal(seen.panStep, 23, '2 degrees of 4096 half-steps, rounded')
  assert.equal(seen.slider.position, 1_200)
  assert.equal(seen.pan.position, 69)
  assert.equal(seen.pulses, 1_200, 'one rising edge a microstep')
  assert.equal(seen.sliderDelay.totalMicros, 1_200 * (10 + 10 + 500), 'setup, pulse, interval')
  assert.equal(seen.panDelay.totalMicros, 69 * 4_000)
  assert.equal(seen.held.register(pca9685.register.preScale), 121)
  assert.equal(seen.fast, 5, 'round(25 MHz / 4096 / 1000) - 1')
  assert.ok(seen.allOff && seen.asleep)
  assert.equal(
    seen.parked.register(pca9685.register.mode1),
    pca9685.mode1.autoIncrement | pca9685.mode1.sleep,
    'no channel was running, so RESTART stays clear',
  )
  assert.equal(seen.bus.waitedMicros, 500, "the oscillator's one start-up")
}
