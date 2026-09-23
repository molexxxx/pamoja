"""Idiomatic on-board bus facade.

Before a node reaches any network it talks to the chips wired to its own board.
Three interfaces cover almost everything cheap hardware uses, and each carries one
small piece of logic that is a classic field bug when it is wrong: the I2C address
byte, the SPI clock mode, and whether a relay is active high or active low.
"""

from __future__ import annotations

import enum
from collections import deque
from typing import Generic, Iterable, Protocol, TypeVar

from pamoja._native import GpioLine as _NativeLine
from pamoja._native import I2C_RESERVED_BELOW as _RESERVED_BELOW
from pamoja._native import I2C_RESERVED_FROM as _RESERVED_FROM
from pamoja._native import SpiClock
from pamoja._native import i2c_address_frame as _address_frame
from pamoja._native import i2c_address_frame_len as _address_frame_len
from pamoja._native import i2c_address_is_general_call as _is_general_call
from pamoja._native import i2c_address_is_reserved as _is_reserved
from pamoja._native import pin_edge_triggered_by as _edge_triggered_by
from pamoja._native import pin_level_from_bool as _level_from_bool
from pamoja._native import pin_level_inverted as _level_inverted
from pamoja._native import pin_polarity_is_asserted as _polarity_is_asserted
from pamoja._native import pin_polarity_level as _polarity_level
from pamoja._native import spi_mode_clock as _mode_clock
from pamoja._native import spi_mode_from_clock as _mode_from_clock

__all__ = [
    "Contact",
    "Edge",
    "GpioLine",
    "InputLine",
    "Level",
    "OutputLine",
    "PinScript",
    "Polarity",
    "SpiClock",
    "Switch",
    "i2c",
    "pin",
    "spi",
]


class Level(str, enum.Enum):
    """The physical voltage level on a pin."""

    #: A low level, near ground.
    LOW = "Low"
    #: A high level, near the supply voltage.
    HIGH = "High"


class Edge(str, enum.Enum):
    """The signal transition that triggers a pin interrupt."""

    #: A low-to-high transition.
    RISING = "Rising"
    #: A high-to-low transition.
    FALLING = "Falling"
    #: Either transition.
    BOTH = "Both"


class Polarity(str, enum.Enum):
    """Whether a signal is asserted by a high or a low physical level."""

    #: A high level means asserted.
    ACTIVE_HIGH = "ActiveHigh"
    #: A low level means asserted, the wiring of most buttons and relay boards.
    ACTIVE_LOW = "ActiveLow"


class _I2c:
    """I2C addressing per the NXP I2C-bus specification (UM10204)."""

    __slots__ = ()

    #: The lowest 7-bit address the specification keeps for itself.
    RESERVED_FROM = _RESERVED_FROM

    #: The first 7-bit address above the reserved block at the bottom of the range.
    RESERVED_BELOW = _RESERVED_BELOW

    def address_frame(
        self, address: int, *, read: bool = False, ten_bit: bool = False
    ) -> bytes:
        """Return the address bytes a controller puts on the bus for a transfer.

        A 7-bit address frames as the single byte ``(address << 1) | r/w``; a
        10-bit one frames as two, the reserved ``11110`` prefix carrying the top
        two bits and the read/write bit, then the low eight.

        :param address: The device address.
        :param read: Whether the transfer reads rather than writes.
        :param ten_bit: Whether this is a 10-bit address.
        :returns: One byte for a 7-bit address, two for a 10-bit one.
        :raises PamojaError: If the address is outside its width's range.
        """
        return _address_frame(address, ten_bit, read)

    def frame_len(self, address: int, ten_bit: bool = False) -> int:
        """Return how many bytes an address frame occupies.

        :param address: The device address.
        :param ten_bit: Whether this is a 10-bit address.
        :returns: ``1`` for a 7-bit address, ``2`` for a 10-bit one.
        :raises PamojaError: If the address is outside its width's range.
        """
        return _address_frame_len(address, ten_bit)

    def is_reserved(self, address: int, ten_bit: bool = False) -> bool:
        """Report whether an address falls in a range the specification reserves.

        UM10204 reserves ``0x00..=0x07`` and ``0x78..=0x7F``, leaving
        ``0x08..=0x77`` for ordinary devices.

        :param address: The device address.
        :param ten_bit: Whether this is a 10-bit address, never reserved in this
            sense.
        :returns: Whether the address is reserved.
        :raises PamojaError: If the address is outside its width's range.
        """
        return _is_reserved(address, ten_bit)

    def is_general_call(self, address: int, ten_bit: bool = False) -> bool:
        """Report whether an address is the general call address ``0x00``.

        :param address: The device address.
        :param ten_bit: Whether this is a 10-bit address.
        :returns: Whether this is the broadcast every device listens to.
        :raises PamojaError: If the address is outside its width's range.
        """
        return _is_general_call(address, ten_bit)


class _Spi:
    """The four SPI clock modes, as the ``(CPOL, CPHA)`` pair datasheets quote."""

    __slots__ = ()

    def clock_for(self, mode: int) -> SpiClock:
        """Return the clock polarity and phase a mode number names.

        :param mode: The mode number, 0 to 3.
        :returns: The pair.
        :raises ValueError: If the mode number is above 3.
        """
        return _mode_clock(mode)

    def mode_for(self, cpol: bool, cpha: bool) -> int:
        """Return the mode number a clock polarity and phase name.

        :param cpol: Whether the clock idles high.
        :param cpha: Whether data is sampled on the trailing edge.
        :returns: The mode number, 0 to 3. Every pair names a mode.
        """
        return _mode_from_clock(cpol, cpha)


class _Pin:
    """The GPIO pin model: levels, interrupt edges, and active polarity."""

    __slots__ = ()

    def level_from(self, high: bool) -> Level:
        """Return the level a boolean names.

        :param high: ``True`` for high, ``False`` for low.
        :returns: The level.
        """
        return Level(_level_from_bool(high))

    def invert(self, level: Level) -> Level:
        """Return the opposite level.

        :param level: The level to invert.
        :returns: The other level.
        """
        return Level(_level_inverted(Level(level).value))

    def triggers(self, edge: Edge, from_level: Level, to_level: Level) -> bool:
        """Report whether a transition fires an interrupt trigger.

        :param edge: The trigger configured on the pin.
        :param from_level: The level before the change.
        :param to_level: The level after it.
        :returns: Whether the trigger fires.
        """
        return _edge_triggered_by(
            Edge(edge).value, Level(from_level).value, Level(to_level).value
        )

    def level_for(self, polarity: Polarity, asserted: bool) -> Level:
        """Return the physical level that represents a logical state.

        :param polarity: How the signal is wired.
        :param asserted: Whether the signal should be asserted.
        :returns: The level to drive, inverted for active-low wiring.
        """
        return Level(_polarity_level(Polarity(polarity).value, asserted))

    def is_asserted(self, polarity: Polarity, level: Level) -> bool:
        """Report whether a physical level means the signal is asserted.

        :param polarity: How the signal is wired.
        :param level: The level read on the pin.
        :returns: Whether the signal is asserted.
        """
        return _polarity_is_asserted(Polarity(polarity).value, Level(level).value)


#: I2C addressing, validated before anything reaches the bus.
i2c = _I2c()

#: SPI clock modes, as a checked value rather than two transposable booleans.
spi = _Spi()

#: The GPIO pin model, so an active-low relay is handled by the type.
pin = _Pin()


class OutputLine(Protocol):
    """A line a pin library drives: ``gpiozero`` or ``lgpio`` on a Raspberry Pi, a
    vendor SDK on a microcontroller, or a :class:`PinScript` in a test. Anything with
    this one method can sit under a :class:`Switch`."""

    def drive(self, level: Level) -> None:
        """Drive the line to a level.

        :param level: The level to drive.
        """
        ...


class InputLine(Protocol):
    """A line a pin library reads, which a :class:`Contact` sits over."""

    def read(self) -> Level:
        """Read the line's level now.

        :returns: The level on the line.
        """
        ...


_Out = TypeVar("_Out", bound=OutputLine)
_In = TypeVar("_In", bound=InputLine)


class Switch(Generic[_Out]):
    """A two-state output over any line, with its polarity said once: a relay, an
    LED, a solenoid valve, a buzzer. ``set(True)`` asserts it, which drives the line
    low for an active-low part, so no call site inverts a level by hand.

    >>> pump = Switch.active_low(PinScript())
    >>> pump.set(True)
    >>> pump.is_asserted, pump.release().driven
    (True, [<Level.LOW: 'Low'>])
    """

    __slots__ = ("_asserted", "_line", "_polarity")

    def __init__(self, line: _Out, polarity: Polarity) -> None:
        """Wrap a line, starting deasserted. Nothing is driven until :meth:`set`.

        :param line: The line the part is wired to.
        :param polarity: How the part is wired.
        """
        self._line = line
        self._polarity = Polarity(polarity)
        self._asserted = False

    @classmethod
    def active_high(cls, line: _Out) -> Switch[_Out]:
        """A switch whose part is asserted by a high level.

        :param line: The line the part is wired to.
        :returns: The switch.
        """
        return cls(line, Polarity.ACTIVE_HIGH)

    @classmethod
    def active_low(cls, line: _Out) -> Switch[_Out]:
        """A switch whose part is asserted by a low level, the wiring of most relay
        boards.

        :param line: The line the part is wired to.
        :returns: The switch.
        """
        return cls(line, Polarity.ACTIVE_LOW)

    @property
    def polarity(self) -> Polarity:
        """How the part is wired."""
        return self._polarity

    @property
    def is_asserted(self) -> bool:
        """Whether the part was last set on."""
        return self._asserted

    def set(self, asserted: bool) -> None:
        """Turn the part on or off, driving whichever level that means for its wiring.

        :param asserted: ``True`` to turn it on.
        :raises Exception: Whatever the line raises when it cannot be driven.
        """
        self._line.drive(pin.level_for(self._polarity, asserted))
        self._asserted = asserted

    def release(self) -> _Out:
        """Hand the line back, for a test to read what was driven or a program to
        reuse it.

        :returns: The line.
        """
        return self._line


class Contact(Generic[_In]):
    """A two-state input over any line, with its polarity said once: a button, a
    float switch, a reed switch, a limit switch. :meth:`is_asserted` answers whether
    it is closed, pressed, or tripped, whatever level that takes on the wire.

    >>> float_switch = Contact.active_low(PinScript([Level.LOW]))
    >>> float_switch.is_asserted()
    True
    """

    __slots__ = ("_line", "_polarity")

    def __init__(self, line: _In, polarity: Polarity) -> None:
        """Wrap a line.

        :param line: The line the part is wired to.
        :param polarity: How the part is wired.
        """
        self._line = line
        self._polarity = Polarity(polarity)

    @classmethod
    def active_high(cls, line: _In) -> Contact[_In]:
        """A contact that reads high when asserted.

        :param line: The line the part is wired to.
        :returns: The contact.
        """
        return cls(line, Polarity.ACTIVE_HIGH)

    @classmethod
    def active_low(cls, line: _In) -> Contact[_In]:
        """A contact that reads low when asserted, the wiring of a switch to ground
        with a pull-up.

        :param line: The line the part is wired to.
        :returns: The contact.
        """
        return cls(line, Polarity.ACTIVE_LOW)

    @property
    def polarity(self) -> Polarity:
        """How the part is wired."""
        return self._polarity

    def level(self) -> Level:
        """Read the raw level on the line.

        :returns: The level.
        :raises Exception: Whatever the line raises when it cannot be read.
        """
        return Level(self._line.read())

    def is_asserted(self) -> bool:
        """Read the line and report whether the part is asserted.

        :returns: Whether it is closed, pressed, or tripped.
        :raises Exception: Whatever the line raises when it cannot be read.
        """
        return pin.is_asserted(self._polarity, self.level())

    def release(self) -> _In:
        """Hand the line back.

        :returns: The line.
        """
        return self._line


class GpioLine:
    """A GPIO line opened on a Linux board, through the kernel's GPIO character device:
    the line a :class:`Switch` or a :class:`Contact` sits over on a Raspberry Pi or any
    Linux board.

    :meth:`open_output` drives its initial level from the moment the line is taken, so an
    active-low relay is opened ``Level.HIGH`` and stays off; :meth:`open_input` opens one
    to read. The chip is a device file such as ``/dev/gpiochip0``, and the line is the GPIO
    or BCM number a Raspberry Pi pinout gives. A line is held by one process at a time;
    :meth:`close`, or leaving a ``with`` block, hands it back. On Raspberry Pi OS a user in
    the ``gpio`` group opens lines without root. Opening raises ``PamojaError`` on any
    platform but Linux, and names the chip and the line when either cannot be opened.
    """

    __slots__ = ("_native",)

    def __init__(self, native: _NativeLine) -> None:
        """Wrap an opened native line. Use :meth:`open_output` or :meth:`open_input`."""
        self._native = native

    @classmethod
    def open_output(cls, chip: str, line: int, initial: Level) -> GpioLine:
        """Open a line as an output, driving ``initial`` from the moment it is taken.

        :param chip: The GPIO chip's device file, ``/dev/gpiochip0`` on most boards.
        :param line: The line's number on that chip, the GPIO or BCM number on a
            Raspberry Pi.
        :param initial: The level to drive as soon as the line is taken.
        :returns: The line.
        :raises PamojaError: If the platform is not Linux, or the chip or the line
            cannot be opened.
        """
        return cls(_NativeLine.open_output(chip, line, Level(initial).value))

    @classmethod
    def open_input(cls, chip: str, line: int) -> GpioLine:
        """Open a line as an input.

        :param chip: The GPIO chip's device file.
        :param line: The line's number on that chip.
        :returns: The line.
        :raises PamojaError: If the platform is not Linux, or the chip or the line
            cannot be opened.
        """
        return cls(_NativeLine.open_input(chip, line))

    @property
    def chip(self) -> str:
        """The GPIO chip's device file."""
        return self._native.chip

    @property
    def offset(self) -> int:
        """The line's number on its chip."""
        return self._native.offset

    def drive(self, level: Level) -> None:
        """Drive the line to a level. The line must have been opened as an output.

        :param level: The level to drive.
        :raises PamojaError: If the kernel refuses the write, or the line is closed.
        """
        self._native.drive(Level(level).value)

    def read(self) -> Level:
        """Read the level on the line now.

        :returns: The level.
        :raises PamojaError: If the kernel refuses the read, or the line is closed.
        """
        return Level(self._native.read())

    def close(self) -> None:
        """Hand the line back to the kernel. Calls after this raise ``PamojaError``."""
        self._native.close()

    def __enter__(self) -> GpioLine:
        """Return the line, so it can be opened in a ``with`` block."""
        return self

    def __exit__(self, *exception: object) -> None:
        """Close the line at the end of a ``with`` block, letting any exception through."""
        self.close()


class PinScript:
    """A line for running with nothing plugged in: it answers the reads it was given,
    in order, and records every level it is driven to. It is what the examples and
    tests put under a :class:`Switch` or a :class:`Contact`, and the one thing a real
    node replaces with its board's pin library. It behaves as
    ``pamoja_hal::script::PinScript`` does in Rust: it starts released, high, and once
    its reads run out a read answers the level it was last driven to.

    >>> line = PinScript([Level.LOW])
    >>> line.read(), line.read()
    (<Level.LOW: 'Low'>, <Level.HIGH: 'High'>)
    """

    __slots__ = ("_driven", "_inputs", "_level")

    def __init__(self, inputs: Iterable[Level] = ()) -> None:
        """Create a released line.

        :param inputs: The levels to answer reads with, in order.
        """
        self._inputs: deque[Level] = deque(Level(level) for level in inputs)
        self._driven: list[Level] = []
        self._level = Level.HIGH

    @property
    def driven(self) -> list[Level]:
        """Every level the line was driven to, oldest first."""
        return list(self._driven)

    @property
    def level(self) -> Level:
        """The level the line was last driven to, high while it has never been
        driven."""
        return self._level

    @property
    def remaining(self) -> int:
        """How many scripted reads are left."""
        return len(self._inputs)

    def drive(self, level: Level) -> None:
        """Record a driven level.

        :param level: The level driven.
        """
        self._level = Level(level)
        self._driven.append(self._level)

    def read(self) -> Level:
        """Answer the next scripted level, or the driven level once the script runs
        out.

        :returns: The level.
        """
        return self._inputs.popleft() if self._inputs else self._level
