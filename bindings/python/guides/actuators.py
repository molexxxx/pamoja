"""The actuator-driver guide example: a motion-control time-lapse rig, with a PCA9685 holding
the tilt servo and the status LED, a slider behind an A4988, and a pan head on a 28BYJ-48; see
docs/guides/actuators.md."""

# ANCHOR: example
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
# ANCHOR_END: example

assert controller.prescale == 121, "round(25 MHz / 4096 / 50) - 1"
assert abs(controller.frequency - 50.0288) < 1e-3, "25 MHz / (4096 * 122)"
assert tilt.off == 266, "1300 us of a 20 ms period in 4096 counts"
assert status.off == 256
assert slide == 400, "an eighth of 3200 microsteps"
assert pan_step == 23, "2 degrees of 4096 half-steps, rounded"
assert slider.position == 1_200
assert pan.position == 69
assert pulses == 1_200, "one rising edge a microstep"
assert slider_delay.total_micros == 1_200 * (10 + 10 + 500), "setup, pulse, interval"
assert pan_delay.total_micros == 69 * 4_000
assert held.register(pca9685.REGISTER_PRE_SCALE) == 121
assert fast == 5, "round(25 MHz / 4096 / 1000) - 1"
assert all_off and asleep
assert parked.register(pca9685.REGISTER_MODE1) == (
    pca9685.MODE1_AUTO_INCREMENT | pca9685.MODE1_SLEEP
), "no channel was running, so RESTART stays clear"
assert bus.waited_micros == 500, "the oscillator's one start-up"
