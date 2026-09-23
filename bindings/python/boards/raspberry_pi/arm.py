"""The inspection rover's arm: a shoulder servo and an elbow servo on a PCA9685 board on the
header's I2C bus, reaching for the controls on an inverter cabinet's panel.

Wire the PCA9685 board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, with the
shoulder servo on channel 0, the elbow servo on channel 1, and V+ from a 5 V supply whose ground
is joined to the Pi's. See docs/guides/motion.md.
"""

# ANCHOR: example
import math
import time

from pamoja.actuators import Pca9685, pca9685, pwm
from pamoja.hal import I2cBus
from pamoja.kit import Elbow, ServoMap, TwoLinkArm

# The PCA9685 channels the shoulder and elbow servos are plugged into.
SHOULDER_CHANNEL = 0
ELBOW_CHANNEL = 1

# The panel's controls, each in meters out from the shoulder and up from it.
PANEL = (
    ("reset button", 0.35, 0.20),
    ("breaker", 0.45, 0.10),
    ("door latch", 0.20, 0.35),
    ("fan switch", 0.70, 0.00),
)


def main() -> None:
    # The PCA9685 on the header's I2C bus, at 50 Hz for the servos.
    bus = I2cBus.open("/dev/i2c-1")
    controller = Pca9685(bus, pca9685.DEFAULT_ADDRESS, frequency_hz=50)

    # Links of 0.30 m and 0.25 m. The servos were fitted with both joints at 0, so a joint angle
    # of 0 is each servo's center, 90 degrees.
    arm = TwoLinkArm(0.30, 0.25)
    servo = ServoMap.standard()

    def pulse(joint: float) -> int:
        return servo.pulse(90.0 + math.degrees(joint))

    # Each control the arm can reach, it holds for two seconds; one it cannot, it skips rather
    # than drive a servo into its end stop.
    for name, x, y in PANEL:
        joints = arm.joints_for(x, y, Elbow.UP)
        if joints is None:
            print(f"{name:12}  out of reach, skipped")
            continue
        shoulder, elbow = joints
        controller.set_channel(SHOULDER_CHANNEL, pwm.servo(pulse(shoulder), 50))
        controller.set_channel(ELBOW_CHANNEL, pwm.servo(pulse(elbow), 50))
        print(f"{name:12}  shoulder {pulse(shoulder)} us, elbow {pulse(elbow)} us")
        time.sleep(2)

    # Back to the pose the arm was fitted in. The PCA9685 goes on sending both pulses after the
    # program exits, so the servos hold the arm there.
    controller.set_channel(SHOULDER_CHANNEL, pwm.servo(pulse(0.0), 50))
    controller.set_channel(ELBOW_CHANNEL, pwm.servo(pulse(0.0), 50))
    print(f"parked        both servos at {pulse(0.0)} us")


if __name__ == "__main__":
    main()
# ANCHOR_END: example
