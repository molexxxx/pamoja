// The inspection rover's arm: a shoulder servo and an elbow servo on a PCA9685 board on the
// header's I2C bus, reaching for the controls on an inverter cabinet's panel. Wire the PCA9685
// board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, with the shoulder servo on
// channel 0, the elbow servo on channel 1, and V+ from a 5 V supply whose ground is joined to
// the Pi's. See docs/guides/motion.md.

// ANCHOR: example
import { setTimeout as sleep } from 'node:timers/promises'
import { Pca9685, pca9685, pwm } from '@pamoja/actuators'
import { I2cBus } from '@pamoja/hal'
import { Elbow, ServoMap, TwoLinkArm } from '@pamoja/kit'

// The PCA9685 channels the shoulder and elbow servos are plugged into.
const SHOULDER_CHANNEL = 0
const ELBOW_CHANNEL = 1

// The panel's controls, each in meters out from the shoulder and up from it.
const PANEL = [
  { name: 'reset button', x: 0.35, y: 0.2 },
  { name: 'breaker', x: 0.45, y: 0.1 },
  { name: 'door latch', x: 0.2, y: 0.35 },
  { name: 'fan switch', x: 0.7, y: 0.0 },
] as const

async function main(): Promise<void> {
  // The PCA9685 on the header's I2C bus, at 50 Hz for the servos.
  const bus = I2cBus.open('/dev/i2c-1')
  const controller = new Pca9685(bus, pca9685.defaultAddress, { frequencyHz: 50 })

  // Links of 0.30 m and 0.25 m. The servos were fitted with both joints at 0, so a joint angle
  // of 0 is each servo's center, 90 degrees.
  const arm = new TwoLinkArm(0.3, 0.25)
  const servo = ServoMap.standard()
  const pulse = (joint: number) => servo.pulse(90 + (joint * 180) / Math.PI)

  // Each control the arm can reach, it holds for two seconds; one it cannot, it skips rather
  // than drive a servo into its end stop.
  for (const { name, x, y } of PANEL) {
    const joints = arm.jointsFor(x, y, Elbow.Up)
    if (joints === null) {
      console.log(`${name.padEnd(12)}  out of reach, skipped`)
      continue
    }
    await controller.setChannel(SHOULDER_CHANNEL, pwm.servo(pulse(joints.shoulder), 50))
    await controller.setChannel(ELBOW_CHANNEL, pwm.servo(pulse(joints.elbow), 50))
    console.log(`${name.padEnd(12)}  shoulder ${pulse(joints.shoulder)} us, elbow ${pulse(joints.elbow)} us`)
    await sleep(2_000)
  }

  // Back to the pose the arm was fitted in. The PCA9685 goes on sending both pulses after the
  // program exits, so the servos hold the arm there.
  await controller.setChannel(SHOULDER_CHANNEL, pwm.servo(pulse(0), 50))
  await controller.setChannel(ELBOW_CHANNEL, pwm.servo(pulse(0), 50))
  console.log(`parked        both servos at ${pulse(0)} us`)
}

main().catch((error: Error) => {
  console.error(error.message)
  process.exitCode = 1
})
// ANCHOR_END: example
