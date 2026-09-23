"""Idiomatic bus facade.

A driver is a conversation with a part, and a bus is what carries it. An
:class:`I2cBus` is one bus that the program and every driver on it share, with one of
three things on the other end: the kernel's adapter on a Linux board, simulated parts
that answer from their registers, or a script of the transfers a driver is expected to
make. A driver runs the same way over all three, so a program is written and tested
with nothing plugged in and then pointed at ``/dev/i2c-1``.
"""

from __future__ import annotations

import enum
import time
from typing import Iterable, List, Optional, Protocol, Union

from pamoja._native import CommandPart as _NativeCommandPart
from pamoja._native import I2cBus as _NativeBus
from pamoja._native import I2cPart as _NativePart
from pamoja._native import I2cStep as _NativeStep
from pamoja._native import WordPart as _NativeWordPart

__all__ = [
    "CommandPart",
    "Delay",
    "DelayLog",
    "I2cBus",
    "I2cBusKind",
    "I2cFault",
    "I2cPart",
    "I2cStep",
    "SimulatedPart",
    "SleepDelay",
    "WordPart",
]


class I2cBusKind(str, enum.Enum):
    """What answers on a bus."""

    #: The kernel's adapter, with real parts on real wires.
    ADAPTER = "Adapter"
    #: Simulated parts, answering from their registers.
    SIMULATED = "Simulated"
    #: A script of the transfers a driver is expected to make.
    SCRIPTED = "Scripted"


class I2cFault(str, enum.Enum):
    """How a scripted step fails the transfer that reaches it."""

    #: Nothing acknowledged the address.
    NO_ACKNOWLEDGE_ADDRESS = "NoAcknowledgeAddress"
    #: The part did not acknowledge a data byte.
    NO_ACKNOWLEDGE_DATA = "NoAcknowledgeData"
    #: A missing acknowledge, with no telling whether of the address or the data.
    NO_ACKNOWLEDGE = "NoAcknowledge"
    #: A bus error, such as a misplaced start or stop condition.
    BUS = "Bus"
    #: Another controller won the bus.
    ARBITRATION_LOSS = "ArbitrationLoss"
    #: Data arrived faster than it was taken.
    OVERRUN = "Overrun"
    #: A failure of no more particular kind.
    OTHER = "Other"


class I2cPart:
    """A part that is not there, answering from 256 registers.

    A write names a register and fills it and the ones after it; a read takes them
    back from wherever the last write left off. What a driver writes stays written, so
    :meth:`register` reads a part's configuration back once a driver is done with it.

    >>> part = I2cPart(0x76).holding(0xD0, bytes([0x60]))
    >>> hex(part.register(0xD0))
    '0x60'
    """

    __slots__ = ("_native",)

    def __init__(self, address: int) -> None:
        """Make a part answering at one address, with every register reading zero.

        :param address: The 7-bit address it answers to.
        """
        self._native = _NativePart(address)

    @classmethod
    def _wrap(cls, native: _NativePart) -> I2cPart:
        part = cls.__new__(cls)
        part._native = native
        return part

    def holding(self, first: int, data: bytes) -> I2cPart:
        """Put bytes in the part from a register on, and return the part.

        :param first: The register the bytes start at.
        :param data: What to put there. Past the last register it wraps to the first.
        :returns: This part, so calls chain.
        """
        self._native.load(first, bytes(data))
        return self

    def register(self, register: int) -> int:
        """Read what one register holds now.

        :param register: Which register.
        :returns: Its value, which is what a driver wrote if it wrote one.
        """
        return self._native.register(register)

    def read(self, first: int, length: int) -> bytes:
        """Read consecutive registers from one register on.

        :param first: The first register.
        :param length: How many registers.
        :returns: One byte per register.
        """
        return bytes(self._native.read(first, length))

    @property
    def address(self) -> int:
        """The address the part answers to."""
        return self._native.address

    @property
    def transfers(self) -> int:
        """How many transfers the part has served."""
        return self._native.transfers


class WordPart:
    """A part that is not there, answering from 256 registers sixteen bits wide.

    This is how Texas Instruments lays out parts such as the TMP117, the INA219 and
    INA226, the OPT3001, the ADS1115, and the HDC1080. A pointer byte names a register and
    a register travels most significant byte first. Bits the part sets for itself, such as
    a conversion-ready flag, are marked with :meth:`read_only` and keep the part's value
    whatever a driver writes.

    >>> part = WordPart(0x48).holding(0x0F, 0x0117)
    >>> hex(part.word(0x0F))
    '0x117'
    """

    __slots__ = ("_native",)

    def __init__(self, address: int) -> None:
        """Make a part answering at one address, with every register reading zero.

        :param address: The 7-bit address it answers to.
        """
        self._native = _NativeWordPart(address)

    @classmethod
    def _wrap(cls, native: _NativeWordPart) -> WordPart:
        part = cls.__new__(cls)
        part._native = native
        return part

    def holding(self, register: int, value: int) -> WordPart:
        """Put a value in one register, and return the part.

        :param register: The register.
        :param value: What it holds, read-only bits included.
        :returns: This part, so calls chain.
        """
        self._native.set(register, value)
        return self

    def read_only(self, register: int, mask: int) -> WordPart:
        """Mark bits of one register as the part's to set, and return the part.

        :param register: The register.
        :param mask: The bits a driver's write leaves as the part holds them.
        :returns: This part, so calls chain.
        """
        self._native.read_only(register, mask)
        return self

    def set(self, register: int, value: int) -> None:
        """Put a value in one register, read-only bits included, as the part itself would.

        :param register: The register.
        :param value: What it holds.
        """
        self._native.set(register, value)

    def word(self, register: int) -> int:
        """Read what one register holds now.

        :param register: Which register.
        :returns: Its value, which is what a driver wrote apart from the read-only bits.
        """
        return self._native.word(register)

    @property
    def address(self) -> int:
        """The address the part answers to."""
        return self._native.address

    @property
    def transfers(self) -> int:
        """How many transfers the part has served."""
        return self._native.transfers


class CommandPart:
    """A part that is not there, answering commands with the replies it was given.

    This is how Sensirion lays out parts such as the SHT3x and the SCD4x. A write sends a
    command and any arguments after it; a read takes the reply that command left, once,
    padded with ``0xFF`` the way an idle bus reads. A command given no reply leaves none,
    and a read then is not acknowledged, which is what a real part does when asked for data
    it does not have.

    >>> part = CommandPart(0x44).answering(bytes([0xF3, 0x2D]), bytes([0x80, 0x10, 0xE1]))
    >>> part.address
    68
    """

    __slots__ = ("_native",)

    def __init__(self, address: int, width: int = 2) -> None:
        """Make a part answering at one address that has been given no replies yet.

        :param address: The 7-bit address it answers to.
        :param width: How many bytes a command takes: two for Sensirion's commands.
        """
        self._native = _NativeCommandPart(address, width)

    @classmethod
    def _wrap(cls, native: _NativeCommandPart) -> CommandPart:
        part = cls.__new__(cls)
        part._native = native
        return part

    def answering(self, command: bytes, reply: bytes) -> CommandPart:
        """Answer one command with a reply from now on, and return the part.

        :param command: The command's bytes.
        :param reply: What a read after it returns, in place of any reply given before.
        :returns: This part, so calls chain.
        """
        self.answer(command, reply)
        return self

    def answer(self, command: bytes, reply: bytes) -> None:
        """Answer one command with a reply from now on.

        :param command: The command's bytes.
        :param reply: What a read after it returns, in place of any reply given before.
        """
        self._native.answer(bytes(command), bytes(reply))

    @property
    def received(self) -> List[bytes]:
        """Every write the part has received, oldest first: a command and any arguments."""
        return [bytes(write) for write in self._native.received]

    @property
    def address(self) -> int:
        """The address the part answers to."""
        return self._native.address

    @property
    def transfers(self) -> int:
        """How many transfers the part has served."""
        return self._native.transfers


#: Any simulated part: a bus takes each kind and gives each back as its own class.
SimulatedPart = Union[I2cPart, WordPart, CommandPart]


def _part_of(native: object) -> SimulatedPart:
    """Wrap a native part as the class of part it is."""
    if isinstance(native, _NativeWordPart):
        return WordPart._wrap(native)
    if isinstance(native, _NativeCommandPart):
        return CommandPart._wrap(native)
    return I2cPart._wrap(native)


class I2cStep:
    """One transfer a script expects, and what the part answers."""

    __slots__ = ("_native",)

    def __init__(self, native: _NativeStep) -> None:
        """Wrap a native step. Use :meth:`write`, :meth:`read`, :meth:`write_read`, or
        :meth:`fault`."""
        self._native = native

    @classmethod
    def write(cls, address: int, data: bytes) -> I2cStep:
        """The driver writes exactly ``data`` to the address.

        :param address: The 7-bit address the write must go to.
        :param data: The bytes the driver must send.
        :returns: The step.
        """
        return cls(_NativeStep.write(address, bytes(data)))

    @classmethod
    def read(cls, address: int, reply: bytes) -> I2cStep:
        """The driver reads from the address and receives ``reply``.

        :param address: The 7-bit address the read must come from.
        :param reply: The bytes the part answers with; the driver must ask for exactly
            this many.
        :returns: The step.
        """
        return cls(_NativeStep.read(address, bytes(reply)))

    @classmethod
    def write_read(cls, address: int, data: bytes, reply: bytes) -> I2cStep:
        """The driver writes ``data`` and then reads ``reply`` in one transaction, the
        shape of a register read.

        :param address: The 7-bit address of the part.
        :param data: The bytes the driver must send first, usually a register address.
        :param reply: The bytes the part answers with.
        :returns: The step.
        """
        return cls(_NativeStep.write_read(address, bytes(data), bytes(reply)))

    @classmethod
    def fault(cls, address: int, fault: I2cFault) -> I2cStep:
        """The next transfer to the address fails, the way a missing or busy part does.

        :param address: The 7-bit address the failing transfer must go to.
        :param fault: The failure the driver sees.
        :returns: The step.
        """
        return cls(_NativeStep.fault(address, I2cFault(fault).value))


class I2cBus:
    """One I2C bus, shared by the program and every driver built on it.

    :meth:`open` opens the kernel's adapter on a Linux board; :meth:`simulated` puts
    simulated parts of any kind on a bus, each answering at its own address;
    :meth:`scripted` plays :class:`I2cStep` s in order and refuses any other transfer. A
    failed transfer raises ``PamojaError`` with the reason: nothing answered at the address,
    the script expected something else, or the kernel's own words.

    >>> bus = I2cBus.simulated([I2cPart(0x76).holding(0xD0, bytes([0x60]))])
    >>> bus.write_read(0x76, bytes([0xD0]), 1).hex()
    '60'
    >>> bus.transfers
    1
    """

    __slots__ = ("_native",)

    def __init__(self, native: _NativeBus) -> None:
        """Wrap a native bus. Use :meth:`open`, :meth:`simulated`, or :meth:`scripted`."""
        self._native = native

    @classmethod
    def open(cls, path: str) -> I2cBus:
        """Open the kernel's I2C adapter, such as ``/dev/i2c-1`` on a Raspberry Pi.

        :param path: The adapter's device file.
        :returns: The bus, with the real parts wired to it on the other end.
        :raises PamojaError: Anywhere but Linux, and when the file cannot be opened as
            an adapter: the interface is not turned on, or the process may not use it.
        """
        return cls(_NativeBus.open(path))

    @classmethod
    def simulated(cls, parts: Iterable[SimulatedPart] = ()) -> I2cBus:
        """Make a bus of simulated parts, each answering at its own address.

        :param parts: The parts on the bus, of any kind. A later part at an address an
            earlier one holds takes its place.
        :returns: The bus. A transfer to an address no part holds raises
            ``PamojaError``, as nothing acknowledges it.
        """
        return cls(_NativeBus.simulated([part._native for part in parts]))

    @classmethod
    def scripted(cls, steps: Iterable[I2cStep]) -> I2cBus:
        """Make a bus that plays the steps in order and refuses any other transfer.

        :param steps: The transfers a driver is expected to make, and the replies.
        :returns: The bus.
        """
        return cls(_NativeBus.scripted([step._native for step in steps]))

    def attach(self, part: SimulatedPart) -> None:
        """Put a copy of a part on a simulated bus, in place of any part at its address.

        A driver keeps working across the change, which is how a test moves a reading on.

        :param part: The part, of any kind.
        :raises PamojaError: If the bus is not simulated.
        """
        self._native.attach(part._native)

    @property
    def kind(self) -> I2cBusKind:
        """What answers on the bus."""
        return I2cBusKind(self._native.kind)

    def write(self, address: int, data: bytes) -> None:
        """Write bytes to a part in one transaction: usually a register and its value.

        :param address: The part's 7-bit address.
        :param data: The bytes to write.
        :raises PamojaError: If the transfer fails.
        """
        self._native.write(address, bytes(data))

    def read(self, address: int, length: int) -> bytes:
        """Read bytes from a part in one transaction.

        :param address: The part's 7-bit address.
        :param length: How many bytes to read.
        :returns: The bytes.
        :raises PamojaError: If the transfer fails.
        """
        return bytes(self._native.read(address, length))

    def write_read(self, address: int, data: bytes, length: int) -> bytes:
        """Write bytes and then read the reply in one transaction, with a repeated start
        between them, which is how a register is read.

        :param address: The part's 7-bit address.
        :param data: What to write first, usually the register address.
        :param length: How many bytes to read.
        :returns: The reply.
        :raises PamojaError: If the transfer fails.
        """
        return bytes(self._native.write_read(address, bytes(data), length))

    def part(self, address: int) -> Optional[SimulatedPart]:
        """Copy what a simulated part holds now, with whatever drivers wrote to it.

        :param address: The part's address.
        :returns: The copy, as the class of part it is, or ``None`` when the bus is not
            simulated or no part holds the address.
        """
        native = self._native.part(address)
        return None if native is None else _part_of(native)

    @property
    def transfers(self) -> int:
        """How many transfers have been made on the bus, by the program and every driver
        on it, including any that failed."""
        return self._native.transfers

    @property
    def remaining(self) -> Optional[int]:
        """How many steps a script has left, or ``None`` when the bus is not scripted."""
        return self._native.remaining

    @property
    def waited_micros(self) -> int:
        """How long the drivers on the bus have asked to wait, in microseconds, whether
        or not the process slept through it."""
        return self._native.waited_micros


class Delay(Protocol):
    """What paces a driver that has to wait between pin changes, such as a stepper
    between steps. :class:`SleepDelay` really waits; :class:`DelayLog` counts every wait
    and waits for none, for a program run with nothing plugged in."""

    def delay_micros(self, micros: int) -> None:
        """Wait, or count the wait.

        :param micros: How long, in microseconds.
        """
        ...


class DelayLog:
    """A delay that records every wait it is asked for and sleeps through none of them,
    as ``pamoja_hal::script::DelayLog`` does in Rust.

    >>> delay = DelayLog()
    >>> delay.delay_micros(480)
    >>> delay.delay_micros(10_000)
    >>> delay.total_micros, delay.total_millis
    (10480, 10)
    """

    __slots__ = ("_total", "_waits")

    def __init__(self) -> None:
        """Create a log with nothing waited yet."""
        self._waits: List[int] = []
        self._total = 0

    @property
    def waits_micros(self) -> List[int]:
        """Every wait asked for, in microseconds, oldest first."""
        return list(self._waits)

    @property
    def total_micros(self) -> int:
        """The waits added up, in microseconds."""
        return self._total

    @property
    def total_millis(self) -> int:
        """The waits added up, in whole milliseconds, rounded down."""
        return self._total // 1_000

    def delay_micros(self, micros: int) -> None:
        """Record a wait.

        :param micros: How long, in microseconds.
        """
        self._waits.append(micros)
        self._total += micros

    def clear(self) -> None:
        """Forget every recorded wait."""
        self._waits.clear()
        self._total = 0


class SleepDelay:
    """A delay that really waits: :func:`time.sleep` for a millisecond or more, and a
    spin on :func:`time.perf_counter_ns` for a shorter wait, which the scheduler cannot
    keep. A sleep lasts at least what was asked and may run over by the scheduler's own
    latency."""

    __slots__ = ()

    def delay_micros(self, micros: int) -> None:
        """Wait.

        :param micros: How long, in microseconds.
        """
        if micros >= 1_000:
            time.sleep(micros / 1_000_000)
            return
        until = time.perf_counter_ns() + micros * 1_000
        while time.perf_counter_ns() < until:
            pass
