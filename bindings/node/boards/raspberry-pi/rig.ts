// A pan and tilt head for a time-lapse: a tilt servo and a status LED on a PCA9685 board on the
// header's I2C bus, and a 28BYJ-48 pan motor on four GPIO lines through a ULN2003 board. Wire the
// PCA9685 board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, with the servo on
// channel 0, an LED on channel 15, and V+ from a 5 V supply whose ground is joined to the Pi's.
// Wire the ULN2003 board's IN1 to IN4 to GPIO5, GPIO6, GPIO13, and GPIO26. See
// docs/guides/actuators.md.

// ANCHOR: example
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
// ANCHOR_END: example
