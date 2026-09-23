"""Idiomatic actuator-driver facade.

Two parts that move something: a PCA9685 driving up to sixteen servos, LEDs, or valves,
and a stepper motor walked one coil pattern at a time. :class:`Pca9685` drives the part
over an :class:`~pamoja.hal.I2cBus`, and ``pca9685.sim.part`` stands one up on a simulated
bus that keeps the datasheet's rules. The ``pwm`` builders make the four register bytes of
one channel's setting. :class:`FourWire` and :class:`StepDir` drive a stepper through its
pins, over any :class:`~pamoja.gpio.OutputLine`.
"""

from __future__ import annotations

import enum
from typing import Generic, NamedTuple, Optional, Tuple, TypeVar

from pamoja._native import Pca9685 as _NativePca9685
from pamoja._native import Stepper as _NativeStepper
from pamoja._native import pca9685_sim_part as _pca9685_sim_part
from pamoja._native import pca9685_channel_register as _channel_register
from pamoja._native import pca9685_frequency_for_prescale as _frequency_for_prescale
from pamoja._native import pca9685_limits as _limits
from pamoja._native import pca9685_prescale_for_frequency as _prescale_for_frequency
from pamoja._native import pwm_counts as _pwm_counts
from pamoja._native import pwm_duty as _pwm_duty
from pamoja._native import pwm_from_counts as _pwm_from_counts
from pamoja._native import pwm_full_off as _pwm_full_off
from pamoja._native import pwm_full_on as _pwm_full_on
from pamoja._native import pwm_servo as _pwm_servo
from pamoja._native import stepper_step_count as _step_count
from pamoja._native import stepper_steps_for_degrees as _steps_for_degrees
from pamoja._native import stepper_timing as _stepper_timing
from pamoja.gpio import Level, OutputLine
from pamoja.hal import Delay, I2cBus, I2cPart, SleepDelay

__all__ = [
    "Direction",
    "Drive",
    "FourWire",
    "Pca9685",
    "Pwm",
    "StepDir",
    "Stepper",
    "pca9685",
    "pwm",
    "stepper",
    "steps_for_degrees",
]

_INTERNAL_OSC_HZ, _CHANNELS, _COUNTS = _limits()
_DEFAULT_STEP_MICROS, _DEFAULT_PULSE_MICROS = _stepper_timing()

_Out = TypeVar("_Out", bound=OutputLine)
_Step = TypeVar("_Step", bound=OutputLine)
_Dir = TypeVar("_Dir", bound=OutputLine)


class Drive(str, enum.Enum):
    """A stepper drive pattern, trading torque, smoothness, and resolution."""

    #: One coil energized at a time: four steps, least torque and least power.
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


def _direction_of(count: int) -> Direction:
    return Direction.BACKWARD if count < 0 else Direction.FORWARD


class FourWire(Generic[_Out]):
    """A four-wire stepper driven coil by coil, through a transistor array such as a
    ULN2003 or an H-bridge, as ``pamoja_actuators::stepper::FourWire`` drives one in Rust.

    Each step energizes the coils the drive pattern names, coil A for the pattern's high
    bit down to coil D for its low bit, then waits the step interval. The position counts
    steps from where the motor was when the driver was built, forward positive. A coil
    line is any :class:`~pamoja.gpio.OutputLine`: a :class:`~pamoja.gpio.GpioLine` on a
    board, or a :class:`~pamoja.gpio.PinScript` that records every level.

    >>> from pamoja.gpio import PinScript
    >>> from pamoja.hal import DelayLog
    >>> coils = (PinScript(), PinScript(), PinScript(), PinScript())
    >>> motor = FourWire(coils, Drive.FULL_STEP, delay=DelayLog())
    >>> motor.steps(2)
    >>> motor.position
    2
    """

    __slots__ = ("_coils", "_delay", "_drive", "_position", "_sequence", "_step_micros")

    def __init__(
        self,
        coils: Tuple[_Out, _Out, _Out, _Out],
        drive: Drive,
        *,
        step_micros: int = _DEFAULT_STEP_MICROS,
        delay: Optional[Delay] = None,
    ) -> None:
        """Wrap the four coil lines, without driving them.

        :param coils: The lines for coils A, B, C, and D, in the motor's phase order.
        :param drive: The coil pattern to step through.
        :param step_micros: The pause after each step, which sets the speed.
        :param delay: What waits: a :class:`~pamoja.hal.SleepDelay` unless given, or a
            :class:`~pamoja.hal.DelayLog` to run with nothing plugged in.
        :raises ValueError: If there are not four coil lines.
        """
        lines = tuple(coils)
        if len(lines) != 4:
            raise ValueError(f"a four-wire stepper has four coil lines, not {len(lines)}")
        a, b, c, d = lines
        self._coils = (a, b, c, d)
        self._drive = Drive(drive)
        self._sequence = _NativeStepper(self._drive.value)
        self._position = 0
        self._step_micros = step_micros
        self._delay: Delay = SleepDelay() if delay is None else delay

    @property
    def position(self) -> int:
        """The step count since the driver was built, forward positive."""
        return self._position

    @property
    def drive(self) -> Drive:
        """The coil pattern in use."""
        return self._drive

    @property
    def step_micros(self) -> int:
        """The pause after each step, in microseconds."""
        return self._step_micros

    def step(self, direction: Direction) -> None:
        """Take one step and wait the step interval.

        :param direction: Which way to step.
        :raises Exception: Whatever a coil line raises when it cannot be driven.
        """
        direction = Direction(direction)
        self._energize(self._sequence.step(direction.value))
        self._position += 1 if direction is Direction.FORWARD else -1
        self._delay.delay_micros(self._step_micros)

    def steps(self, count: int) -> None:
        """Take ``count`` steps, backward when negative.

        :param count: The signed number of steps.
        :raises Exception: Whatever a coil line raises; the steps already taken stay
            counted.
        """
        direction = _direction_of(count)
        for _ in range(abs(count)):
            self.step(direction)

    def idle(self) -> None:
        """Drop every coil, so the motor holds nothing and draws nothing.

        :raises Exception: Whatever a coil line raises when it cannot be driven.
        """
        self._energize(0)

    def release(self) -> Tuple[_Out, _Out, _Out, _Out]:
        """Hand the coil lines back, for a test to read what was driven or a program to
        reuse them.

        :returns: The lines for coils A, B, C, and D.
        """
        return self._coils

    def _energize(self, coils: int) -> None:
        for index, line in enumerate(self._coils):
            line.drive(Level.HIGH if coils & (0b1000 >> index) else Level.LOW)


class StepDir(Generic[_Step, _Dir]):
    """A stepper behind a step and direction driver chip such as an A4988 or a DRV8825,
    as ``pamoja_actuators::stepper::StepDir`` drives one in Rust.

    Each step sets the direction line, high for forward, and holds it for the pulse
    width, since the chip reads the direction on the step line's rising edge and needs it
    settled first. Then it pulses the step line high for the pulse width, brings it low,
    and waits the step interval. Microstepping, current limiting, and enable are the
    chip's own pins and settings, outside this driver.

    >>> from pamoja.gpio import PinScript
    >>> from pamoja.hal import DelayLog
    >>> motor = StepDir(PinScript(), PinScript(), delay=DelayLog())
    >>> motor.steps(-1)
    >>> motor.position
    -1
    """

    __slots__ = (
        "_delay",
        "_direction",
        "_position",
        "_pulse_micros",
        "_step",
        "_step_micros",
    )

    def __init__(
        self,
        step: _Step,
        direction: _Dir,
        *,
        pulse_micros: int = _DEFAULT_PULSE_MICROS,
        step_micros: int = _DEFAULT_STEP_MICROS,
        delay: Optional[Delay] = None,
    ) -> None:
        """Wrap the step and direction lines, without driving them.

        :param step: The line the chip counts rising edges on.
        :param direction: The line the chip reads the direction from; high is forward.
        :param pulse_micros: How long the direction line is held before the step line
            rises, and the step line is then held high.
        :param step_micros: The pause after each step, which sets the speed.
        :param delay: What waits: a :class:`~pamoja.hal.SleepDelay` unless given, or a
            :class:`~pamoja.hal.DelayLog` to run with nothing plugged in.
        """
        self._step = step
        self._direction = direction
        self._position = 0
        self._pulse_micros = pulse_micros
        self._step_micros = step_micros
        self._delay: Delay = SleepDelay() if delay is None else delay

    @property
    def position(self) -> int:
        """The step count since the driver was built, forward positive."""
        return self._position

    @property
    def pulse_micros(self) -> int:
        """How long the direction is held before each step pulse, and the pulse itself, in
        microseconds."""
        return self._pulse_micros

    @property
    def step_micros(self) -> int:
        """The pause after each step, in microseconds."""
        return self._step_micros

    def step(self, direction: Direction) -> None:
        """Take one step and wait the step interval.

        :param direction: Which way to step.
        :raises Exception: Whatever a line raises when it cannot be driven.
        """
        forward = Direction(direction) is Direction.FORWARD
        self._direction.drive(Level.HIGH if forward else Level.LOW)
        self._delay.delay_micros(self._pulse_micros)
        self._step.drive(Level.HIGH)
        self._delay.delay_micros(self._pulse_micros)
        self._step.drive(Level.LOW)
        self._position += 1 if forward else -1
        self._delay.delay_micros(self._step_micros)

    def steps(self, count: int) -> None:
        """Take ``count`` steps, backward when negative.

        :param count: The signed number of steps.
        :raises Exception: Whatever a line raises; the steps already taken stay counted.
        """
        direction = _direction_of(count)
        for _ in range(abs(count)):
            self.step(direction)

    def release(self) -> Tuple[_Step, _Dir]:
        """Hand the lines back.

        :returns: The step line, then the direction line.
        """
        return self._step, self._direction


class Pwm(NamedTuple):
    """When in a period a PWM output goes high and low, in counts."""

    on: int
    """The count at which the output goes high."""

    off: int
    """The count at which it goes low, or the full-off flag."""


class Pca9685:
    """An NXP PCA9685 on an I2C bus, driving sixteen PWM outputs.

    The driver programs the prescale for its frequency with the oscillator asleep, because
    the part only takes a prescale then, wakes it, waits the 500 us it needs, and loads a
    channel's four registers in one transfer. Every call releases the interpreter while the
    bus is busy.

    Example::

        board = Pca9685(bus, pca9685.DEFAULT_ADDRESS, frequency_hz=50)
        board.set_channel(0, pwm.servo(1500))
    """

    __slots__ = ("_native",)

    def __init__(
        self,
        bus: I2cBus,
        address: int = 0x40,
        *,
        frequency_hz: int = 200,
        oscillator_hz: int = _INTERNAL_OSC_HZ,
        totem_pole: bool = True,
        inverted: bool = False,
        change_on_ack: bool = False,
    ) -> None:
        """Make a driver for the part at ``address`` on ``bus``. Nothing is sent until
        :meth:`init` or the first channel is loaded.

        :param bus: The bus the part is on.
        :param address: The address its A5 to A0 pins select, ``0x40`` with all six low.
        :param frequency_hz: The PWM frequency every channel shares; 50 for hobby servos.
        :param oscillator_hz: The clock the prescaler divides, for a board driving EXTCLK.
        :param totem_pole: Totem-pole outputs when true, open-drain when false.
        :param inverted: Invert the output logic, for a board with no external driver.
        :param change_on_ack: Change the outputs on each write's acknowledge, not the stop.
        """
        self._native = _NativePca9685(
            bus._native,
            address,
            frequency_hz,
            oscillator_hz,
            totem_pole,
            inverted,
            change_on_ack,
        )

    def init(self) -> None:
        """Program the prescale and the output wiring, wake the oscillator, and restart.

        :raises PamojaError: If the bus fails.
        """
        self._native.init()

    def set_channel(self, channel: int, setting: bytes) -> None:
        """Load one channel, initializing the part first if :meth:`init` has not run.

        :param channel: The output, 0 to 15.
        :param setting: The four register bytes the ``pwm`` builders make.
        :raises ValueError: For a channel past 15, or a setting that is not four bytes.
        :raises PamojaError: If the bus fails.
        """
        self._native.set_channel(channel, bytes(setting))

    def channel(self, channel: int) -> bytes:
        """Read one channel's four register bytes back from the part.

        The registers are read one at a time, so the read works whether or not MODE1's
        auto-increment bit is set, and it changes nothing on the part: a program that
        restarts can ask a running part what it holds. ``pwm.counts`` names the counts.

        :param channel: The output, 0 to 15.
        :returns: The four register bytes the channel holds.
        :raises ValueError: For a channel past 15.
        :raises PamojaError: If the bus fails.
        """
        return bytes(self._native.channel(channel))

    def set_all(self, setting: bytes) -> None:
        """Load every channel with the same four bytes in one transfer.

        :param setting: The four register bytes the ``pwm`` builders make.
        :raises PamojaError: If the bus fails.
        """
        self._native.set_all(bytes(setting))

    def sleep(self) -> None:
        """Stop the oscillator; the channels keep their settings.

        :raises PamojaError: If the bus fails.
        """
        self._native.sleep()

    def wake(self) -> None:
        """Wake the oscillator, wait the 500 us it needs, and restart the channels.

        :raises PamojaError: If the bus fails.
        """
        self._native.wake()

    def software_reset(self) -> None:
        """Send the general-call software reset to every PCA9685 on the bus.

        It goes to address ``0x00``, which nothing on a simulated bus answers.

        :raises PamojaError: If the bus fails, or nothing acknowledges the general call.
        """
        self._native.software_reset()

    @property
    def prescale(self) -> int:
        """The prescale value the driver writes for its frequency."""
        return self._native.prescale

    @property
    def frequency(self) -> float:
        """The frequency the part runs at once the prescaler has rounded, in hertz."""
        return self._native.frequency


class _Pca9685Sim:
    """A PCA9685 that is not there, keeping its datasheet's rules: PRE_SCALE takes a write
    only while the part sleeps, the register pointer moves on only with auto-increment set,
    and one write to the ALL_LED registers loads every channel."""

    __slots__ = ()

    def part(self, address: int = 0x40) -> I2cPart:
        """A simulated PCA9685 as it powers up: asleep at 200 Hz with every output off.

        :param address: The address it answers to.
        :returns: A part to put on a simulated bus.
        """
        return I2cPart._wrap(_pca9685_sim_part(address))


class _Pca9685:
    """An NXP PCA9685 16-channel PWM controller, for servos, LEDs, and valves."""

    __slots__ = ()

    #: The address it answers at with its six address pins low.
    DEFAULT_ADDRESS = 0x40
    #: The part's internal oscillator frequency, in hertz.
    INTERNAL_OSC_HZ = _INTERNAL_OSC_HZ
    #: How many channels it drives.
    CHANNELS = _CHANNELS
    #: How many counts each period is divided into.
    COUNTS = _COUNTS
    #: How long the oscillator takes to run once woken, in microseconds.
    OSCILLATOR_STARTUP_MICROS = 500
    #: Mode register 1: restart, clock, auto-increment, sleep, and the addresses answered.
    REGISTER_MODE1 = 0x00
    #: Mode register 2: how the outputs are wired and when they change.
    REGISTER_MODE2 = 0x01
    #: The first of channel 0's four registers; :meth:`channel_register` gives the rest.
    REGISTER_LED0_ON_L = 0x06
    #: The first of the four registers that load every channel at once.
    REGISTER_ALL_LED_ON_L = 0xFA
    #: The prescaler, writable only while the part sleeps.
    REGISTER_PRE_SCALE = 0xFE
    #: MODE1's RESTART bit: set when the part slept with a channel running.
    MODE1_RESTART = 0x80
    #: MODE1's EXTCLK bit: the prescaler divides the EXTCLK pin.
    MODE1_EXTCLK = 0x40
    #: MODE1's auto-increment bit: the register pointer moves on after each byte.
    MODE1_AUTO_INCREMENT = 0x20
    #: MODE1's SLEEP bit: the oscillator is off and PRE_SCALE takes a write.
    MODE1_SLEEP = 0x10
    #: MODE1 at power-up: asleep, answering the All Call address.
    MODE1_RESET = 0x11
    #: MODE2 at power-up: totem-pole outputs.
    MODE2_RESET = 0x04
    #: PRE_SCALE at power-up: 200 Hz on the internal oscillator.
    PRE_SCALE_RESET = 0x1E
    #: The smallest value the part loads into PRE_SCALE.
    PRE_SCALE_MIN = 3
    #: A PCA9685 that is not there.
    sim = _Pca9685Sim()

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

        The datasheet rules out the same count in on and off, so 0 is the full-off
        setting and 4096 or more the full-on one.

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

    def counts(self, data: bytes) -> Pwm:
        """Read a setting back from the four register bytes a channel holds.

        The inverse of the builders above, so a caller can read a channel off the bus
        and see what it is set to rather than decoding the registers by hand.

        :param data: The four channel registers.
        :returns: The counts at which the output goes high and low, named.
        :raises ValueError: If ``data`` is not four bytes.
        """
        on, off = _pwm_counts(bytes(data))
        return Pwm(on, off)

    def full_on(self) -> bytes:
        """Return the setting that holds a channel continuously high.

        :returns: The four register bytes.
        """
        return _pwm_full_on()

    def full_off(self) -> bytes:
        """Return the setting that holds a channel continuously low.

        This is the power-on state, and its flag takes precedence over the full-on flag
        when both are set.

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


class _StepperTiming:
    """Stepper motor timing, as ``pamoja_actuators::stepper`` starts a driver with."""

    __slots__ = ()

    #: The pause a driver takes after each step unless given another, in microseconds.
    DEFAULT_STEP_MICROS = _DEFAULT_STEP_MICROS
    #: How long a step and direction driver holds the direction before a step pulse, and
    #: the pulse itself, in microseconds.
    DEFAULT_PULSE_MICROS = _DEFAULT_PULSE_MICROS


#: Stepper motor timing.
stepper = _StepperTiming()

#: An NXP PCA9685 16-channel PWM controller.
pca9685 = _Pca9685()

#: The four register bytes for one PCA9685 channel.
pwm = _Pwm()
