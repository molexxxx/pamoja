"""The actuator-driver guide example; see docs/guides/actuators.md."""

# ANCHOR: example
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
print(f"centred servo goes low at count {pwm.counts(centred)[1]} of 4096")

# Fully off carries its own flag rather than a zero duty, which would still hold the
# output high for the first count of every period.
print(f"full off flag set: {pwm.counts(pwm.full_off())[1] != pwm.counts(pwm.duty(0))[1]}")

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
# ANCHOR_END: example

assert prescale == 0x79
assert pca9685.channel_register(3) == 0x12
assert pwm.counts(centred)[1] == 307
assert pwm.full_off() != pwm.duty(0)
assert motor.coils == 0b1000
assert steps_for_degrees(90.0, 200) == 50
