"""A pan and tilt head for a time-lapse: a tilt servo and a status LED on a PCA9685 board on the
header's I2C bus, and a 28BYJ-48 pan motor on four GPIO lines through a ULN2003 board.

Wire the PCA9685 board's SDA to GPIO2, SCL to GPIO3, VCC to 3V3, and GND to ground, with the
servo on channel 0, an LED on channel 15, and V+ from a 5 V supply whose ground is joined to the
Pi's. Wire the ULN2003 board's IN1 to IN4 to GPIO5, GPIO6, GPIO13, and GPIO26. See
docs/guides/actuators.md.
"""

# ANCHOR: example
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
# ANCHOR_END: example
