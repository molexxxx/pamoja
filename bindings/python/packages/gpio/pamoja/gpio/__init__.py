"""Idiomatic on-board bus facade.

Before a node reaches any network it talks to the chips wired to its own board.
Three interfaces cover almost everything cheap hardware uses, and each carries one
small piece of logic that is a classic field bug when it is wrong: the I2C address
byte, the SPI clock mode, and whether a relay is active high or active low.
"""

from __future__ import annotations

import enum

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

__all__ = ["Edge", "Level", "Polarity", "SpiClock", "i2c", "pin", "spi"]


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
