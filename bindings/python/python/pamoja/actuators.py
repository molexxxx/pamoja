"""Idiomatic actuator-driver facade.

These are the command-encode half of two parts that move something: a PCA9685
driving up to sixteen servos, LEDs, or valves, and a stepper motor walked one coil
pattern at a time. Applying the bytes is the caller's job; working out which bytes
is this layer's.
"""

from __future__ import annotations

import enum

from ._core import Stepper as _NativeStepper
from ._core import pca9685_channel_register as _channel_register
from ._core import pca9685_frequency_for_prescale as _frequency_for_prescale
from ._core import pca9685_limits as _limits
from ._core import pca9685_prescale_for_frequency as _prescale_for_frequency
from ._core import pwm_duty as _pwm_duty
from ._core import pwm_from_counts as _pwm_from_counts
from ._core import pwm_full_off as _pwm_full_off
from ._core import pwm_full_on as _pwm_full_on
from ._core import pwm_servo as _pwm_servo
from ._core import stepper_step_count as _step_count
from ._core import stepper_steps_for_degrees as _steps_for_degrees

__all__ = ["Direction", "Drive", "Stepper", "pca9685", "pwm", "steps_for_degrees"]

_INTERNAL_OSC_HZ, _CHANNELS, _COUNTS = _limits()


class Drive(str, enum.Enum):
    """A stepper drive pattern, trading torque, smoothness, and resolution."""

    #: One coil energised at a time: four steps, least torque and least power.
    WAVE = "Wave"
    #: Two adjacent coils at a time: four steps, most torque.
    FULL_STEP = "FullStep"
    #: Alternating one and two coils: eight steps, double resolution.
    HALF_STEP = "HalfStep"

    @property
    def step_count(self) -> int:
        """How many steps make up one electrical cycle of this pattern."""
        return _step_count(self.value)


class Direction(str, enum.Enum):
    """Which way to step a motor."""

    #: Advance the sequence, turning the shaft one way.
    FORWARD = "Forward"
    #: Reverse the sequence, turning the shaft the other way.
    BACKWARD = "Backward"


class Stepper:
    """A stepper motor's place in its drive sequence, and how far it has turned.

    Example::

        motor = Stepper(Drive.HALF_STEP)
        coils = motor.step(Direction.FORWARD)
    """

    __slots__ = ("_native",)

    def __init__(self, drive: Drive) -> None:
        """Create a stepper at the start of a pattern, with its position at zero.

        :param drive: The coil pattern to walk.
        """
        self._native = _NativeStepper(Drive(drive).value)

    def step(self, direction: Direction) -> int:
        """Advance one step and return the four-bit coil pattern to apply.

        The most significant of the four bits is the first coil.

        :param direction: Which way to turn.
        :returns: The coil pattern.
        """
        return self._native.step(Direction(direction).value)

    @property
    def coils(self) -> int:
        """The coil pattern currently held, without advancing."""
        return self._native.coils

    @property
    def steps(self) -> int:
        """How many steps have been taken, signed by direction."""
        return self._native.steps


class _Pca9685:
    """An NXP PCA9685 16-channel PWM controller, for servos, LEDs, and valves."""

    __slots__ = ()

    #: The part's internal oscillator frequency, in hertz.
    INTERNAL_OSC_HZ = _INTERNAL_OSC_HZ
    #: How many channels it drives.
    CHANNELS = _CHANNELS
    #: How many counts each period is divided into.
    COUNTS = _COUNTS

    def channel_register(self, channel: int) -> int:
        """Return the first of a channel's four consecutive registers.

        :param channel: The channel, 0 to 15.
        :returns: The register address.
        :raises ValueError: If the channel is beyond the part.
        """
        return _channel_register(channel)

    def prescale_for_frequency(
        self, update_rate_hz: int, osc_hz: int = _INTERNAL_OSC_HZ
    ) -> int:
        """Return the prescale value that sets an update rate.

        :param update_rate_hz: The PWM frequency wanted.
        :param osc_hz: The oscillator frequency, usually the internal one.
        :returns: The prescale register value.
        """
        return _prescale_for_frequency(update_rate_hz, osc_hz)

    def frequency_for_prescale(
        self, prescale: int, osc_hz: int = _INTERNAL_OSC_HZ
    ) -> float:
        """Return the update rate a prescale value produces.

        :param prescale: The prescale register value.
        :param osc_hz: The oscillator frequency, usually the internal one.
        :returns: The frequency in hertz.
        """
        return _frequency_for_prescale(prescale, osc_hz)


class _Pwm:
    """The four register bytes for one PCA9685 channel.

    Each call returns them in the channel's own register order, so they can be
    written in a single bus transaction.
    """

    __slots__ = ()

    def from_counts(self, on: int, off: int) -> bytes:
        """Build a setting from explicit on and off counts.

        :param on: The count at which the output goes high.
        :param off: The count at which it goes low.
        :returns: The four register bytes; counts are masked to 12 bits.
        """
        return _pwm_from_counts(on, off)

    def duty(self, off: int) -> bytes:
        """Build a setting with no phase delay: on at count 0, off at ``off``.

        :param off: The count at which the output goes low, which sets the duty.
        :returns: The four register bytes.
        """
        return _pwm_duty(off)

    def servo(self, pulse_micros: int, update_rate_hz: int = 50) -> bytes:
        """Build the setting that drives a hobby servo to a pulse width.

        :param pulse_micros: The high-pulse width in microseconds. Typical travel
            is about 1000 to 2000 microseconds.
        :param update_rate_hz: The PWM frequency the controller is set to.
        :returns: The four register bytes.
        """
        return _pwm_servo(pulse_micros, update_rate_hz)

    def full_on(self) -> bytes:
        """Return the setting that holds a channel continuously high.

        :returns: The four register bytes.
        """
        return _pwm_full_on()

    def full_off(self) -> bytes:
        """Return the setting that holds a channel continuously low.

        This is the power-on state, and is not the same as a zero duty, which
        still glitches high for one count.

        :returns: The four register bytes.
        """
        return _pwm_full_off()


def steps_for_degrees(degrees: float, steps_per_revolution: int) -> int:
    """Return how many steps a rotation of ``degrees`` takes on a given motor.

    :param degrees: The angle to turn through.
    :param steps_per_revolution: The motor's steps per full revolution.
    :returns: The step count, negative for a negative angle.
    """
    return _steps_for_degrees(degrees, steps_per_revolution)


#: An NXP PCA9685 16-channel PWM controller.
pca9685 = _Pca9685()

#: The four register bytes for one PCA9685 channel.
pwm = _Pwm()
