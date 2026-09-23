"""Idiomatic Modbus RTU facade.

Modbus over RS485 is what cheap industrial sensing speaks: energy meters, soil
probes, water-quality transmitters, pump controllers. A :class:`ModbusClient` runs whole
transactions over a serial port, with the timing the serial line specification sets, and a
:class:`ModbusServer` is a device a :class:`ModbusLine` puts on the far end of a simulated
one. Underneath, each request builder here returns a complete frame with its CRC, and a
reply comes back through :func:`parse_frame` as an object that reads its own values.
"""

from __future__ import annotations

import enum
from typing import Sequence

from pamoja._native import ModbusClient as _NativeClient
from pamoja._native import ModbusClientError
from pamoja._native import ModbusFrame
from pamoja._native import ModbusLine as _NativeLine
from pamoja._native import ModbusServer
from pamoja._native import modbus_crc16 as _crc16
from pamoja._native import modbus_parse_frame as _parse_frame
from pamoja._native import modbus_raw as _raw
from pamoja._native import modbus_read_coils as _read_coils
from pamoja._native import modbus_read_discrete_inputs as _read_discrete_inputs
from pamoja._native import modbus_read_holding_registers as _read_holding_registers
from pamoja._native import (
    modbus_read_holding_registers_reply as _read_holding_registers_reply,
)
from pamoja._native import modbus_read_input_registers as _read_input_registers
from pamoja._native import (
    modbus_read_input_registers_reply as _read_input_registers_reply,
)
from pamoja._native import modbus_write_multiple_coils as _write_multiple_coils
from pamoja._native import modbus_write_multiple_registers as _write_multiple_registers
from pamoja._native import modbus_write_single_coil as _write_single_coil
from pamoja._native import modbus_write_single_register as _write_single_register
from pamoja.hal import Parity, SerialPort, SerialSettings

__all__ = [
    "BROADCAST",
    "Exception_",
    "Function",
    "ModbusClient",
    "ModbusClientError",
    "ModbusFrame",
    "ModbusLine",
    "ModbusServer",
    "crc16",
    "parse_frame",
    "raw",
    "read_coils",
    "read_discrete_inputs",
    "read_holding_registers",
    "read_holding_registers_reply",
    "read_input_registers",
    "read_input_registers_reply",
    "write_multiple_coils",
    "write_multiple_registers",
    "write_single_coil",
    "write_single_register",
]


class Function(int, enum.Enum):
    """The function codes this SDK names, as they appear on the wire."""

    #: Read one or more coils (read/write bits).
    READ_COILS = 0x01
    #: Read one or more discrete inputs (read-only bits).
    READ_DISCRETE_INPUTS = 0x02
    #: Read one or more holding registers (read/write 16-bit words).
    READ_HOLDING_REGISTERS = 0x03
    #: Read one or more input registers (read-only 16-bit words).
    READ_INPUT_REGISTERS = 0x04
    #: Write a single coil.
    WRITE_SINGLE_COIL = 0x05
    #: Write a single holding register.
    WRITE_SINGLE_REGISTER = 0x06
    #: Write a contiguous block of coils.
    WRITE_MULTIPLE_COILS = 0x0F
    #: Write a contiguous block of holding registers.
    WRITE_MULTIPLE_REGISTERS = 0x10


class Exception_(int, enum.Enum):
    """The reason a device gives for refusing a request.

    Named with a trailing underscore because ``Exception`` is a Python builtin.
    """

    #: The function code is not allowed for this device.
    ILLEGAL_FUNCTION = 0x01
    #: The data address is not allowed for this device.
    ILLEGAL_DATA_ADDRESS = 0x02
    #: A value in the request is not allowed for this device.
    ILLEGAL_DATA_VALUE = 0x03
    #: The device failed while serving the request.
    SERVER_DEVICE_FAILURE = 0x04
    #: The device accepted a long-running request and is still processing it.
    ACKNOWLEDGE = 0x05
    #: The device is busy with a long-running request; retry later.
    SERVER_DEVICE_BUSY = 0x06
    #: The device detected a parity error in its memory.
    MEMORY_PARITY_ERROR = 0x08
    #: A gateway could not route the request to the target path.
    GATEWAY_PATH_UNAVAILABLE = 0x0A
    #: A gateway got no response from the target device, usually one not on the network.
    GATEWAY_TARGET_FAILED_TO_RESPOND = 0x0B


def crc16(data: bytes) -> int:
    """Compute the CRC-16/MODBUS that every RTU frame ends with.

    :param data: The frame contents, without the trailing checksum.
    :returns: The checksum.
    """
    return _crc16(bytes(data))


def read_coils(address: int, start: int, count: int) -> bytes:
    """Build a read-coils request (function ``0x01``).

    :param address: The unit address to ask.
    :param start: The address of the first coil.
    :param count: How many coils to read.
    :returns: The frame to send.
    """
    return _read_coils(address, start, count)


def read_discrete_inputs(address: int, start: int, count: int) -> bytes:
    """Build a read-discrete-inputs request (function ``0x02``).

    :param address: The unit address to ask.
    :param start: The address of the first input.
    :param count: How many inputs to read.
    :returns: The frame to send.
    """
    return _read_discrete_inputs(address, start, count)


def read_holding_registers_reply(address: int, values: list[int]) -> bytes:
    """Build the reply a device sends to a read-holding-registers request.

    This is the answering half of :func:`read_holding_registers`, so a client can be
    written and tested against what a device sends without a device on the line.

    :param address: The unit address the reply comes from.
    :param values: The register values the device reports, in address order.
    :returns: The frame the device would send.
    :raises PamojaError: If there are no values, or more than one frame can carry.
    """
    return _read_holding_registers_reply(address, values)


def read_input_registers_reply(address: int, values: list[int]) -> bytes:
    """Build the reply a device sends to a read-input-registers request.

    :param address: The unit address the reply comes from.
    :param values: The register values the device reports, in address order.
    :returns: The frame the device would send.
    :raises PamojaError: If there are no values, or more than one frame can carry.
    """
    return _read_input_registers_reply(address, values)


def read_holding_registers(address: int, start: int, count: int) -> bytes:
    """Build a read-holding-registers request (function ``0x03``).

    :param address: The unit address to ask.
    :param start: The address of the first register.
    :param count: How many registers to read.
    :returns: The frame to send.
    """
    return _read_holding_registers(address, start, count)


def read_input_registers(address: int, start: int, count: int) -> bytes:
    """Build a read-input-registers request (function ``0x04``).

    :param address: The unit address to ask.
    :param start: The address of the first register.
    :param count: How many registers to read.
    :returns: The frame to send.
    """
    return _read_input_registers(address, start, count)


def write_single_coil(address: int, coil: int, on: bool) -> bytes:
    """Build a write-single-coil request (function ``0x05``).

    :param address: The unit address to write to.
    :param coil: The coil address.
    :param on: The state to write.
    :returns: The frame to send.
    """
    return _write_single_coil(address, coil, on)


def write_single_register(address: int, register: int, value: int) -> bytes:
    """Build a write-single-register request (function ``0x06``).

    :param address: The unit address to write to.
    :param register: The register address.
    :param value: The 16-bit value to write.
    :returns: The frame to send.
    """
    return _write_single_register(address, register, value)


def write_multiple_registers(address: int, start: int, values: Sequence[int]) -> bytes:
    """Build a write-multiple-registers request (function ``0x10``).

    :param address: The unit address to write to.
    :param start: The address of the first register.
    :param values: The 16-bit values, at most 123 of them.
    :returns: The frame to send.
    :raises PamojaError: If there are no values, or more than one request carries.
    """
    return _write_multiple_registers(address, start, list(values))


def write_multiple_coils(address: int, start: int, values: Sequence[bool]) -> bytes:
    """Build a write-multiple-coils request (function ``0x0F``).

    :param address: The unit address to write to.
    :param start: The address of the first coil.
    :param values: One state per coil, at most 1968 of them.
    :returns: The frame to send.
    :raises PamojaError: If there are no values, or more than one request carries.
    """
    return _write_multiple_coils(address, start, [bool(value) for value in values])


def raw(address: int, function_code: int, data: bytes) -> bytes:
    """Build a request from a raw function code and data.

    This is the escape hatch for the function codes this SDK does not name.

    :param address: The unit address to send to.
    :param function_code: The function code byte.
    :param data: The bytes that follow it, used verbatim.
    :returns: The frame to send.
    :raises PamojaError: If the data is longer than a PDU may be.
    """
    return _raw(address, function_code, bytes(data))


def parse_frame(data: bytes) -> ModbusFrame:
    """Parse a received RTU frame, verifying its CRC.

    :param data: The frame as it came off the wire, checksum included.
    :returns: The validated frame, which reads its own registers and coils.
    :raises PamojaError: If the frame is truncated, oversized, or its CRC does not
        match its contents.
    """
    return _parse_frame(bytes(data))


#: The unit address every device acts on and none answers.
BROADCAST = 0


class ModbusLine:
    """Several devices on one simulated line, as devices share an RS485 pair.

    Every frame reaches all of them, the one it is addressed to answers, and each carries
    out a broadcast write. The line shares each :class:`ModbusServer` put on it, so the
    program's own object still reads and changes the device. :meth:`port` makes a
    :class:`~pamoja.hal.SerialPort` with the line on its far end for a :class:`ModbusClient`
    to poll; nothing on it waits, and the silences and timeouts of a real line are counted
    in its ``waited_micros``.

    >>> meter = ModbusServer(17)
    >>> meter.set_holding_registers(107, [2301, 418, 0])
    >>> line = ModbusLine().attach(meter)
    >>> client = ModbusClient(line.port(SerialSettings(19_200, Parity.EVEN)))
    >>> client.read_holding_registers(17, 107, 3)
    [2301, 418, 0]
    """

    __slots__ = ("_native",)

    def __init__(self) -> None:
        """Make a line with no devices on it."""
        self._native = _NativeLine()

    def attach(self, server: ModbusServer) -> ModbusLine:
        """Put a device on the line, which shares it.

        :param server: The device.
        :returns: This line, to put another device on.
        """
        self._native.attach(server)
        return self

    def __len__(self) -> int:
        """How many devices are on the line."""
        return len(self._native)

    def port(self, settings: SerialSettings) -> SerialPort:
        """Make a serial port with the line on its far end.

        Devices put on the line later are on the port too.

        :param settings: The speed and character format the line runs at.
        :returns: The port.
        """
        return SerialPort(
            self._native.port(settings.baud, Parity(settings.parity).value, settings.stop_bits)
        )


class ModbusClient:
    """A Modbus RTU client on a serial line: the gateway, the master in the specification's
    words, that sends each request and waits for its reply.

    Each transaction follows the Modbus over Serial Line specification. The client leaves
    the line silent for 3.5 characters, or 1.75 ms above 19200 baud; drops anything stale
    waiting in the port; writes the request; and reads the reply to the length the request
    implies, against :attr:`response_timeout`. The reply's CRC, unit, and function are
    checked before a value is read out of it, and any failure raises
    :class:`ModbusClientError`, whose ``kind`` says why. A write to :data:`BROADCAST` reaches
    every device and draws no reply, so the client waits out :attr:`turnaround` instead.
    Each call releases the interpreter while the line is busy.
    """

    __slots__ = ("_native",)

    def __init__(
        self, port: SerialPort, response_timeout: float = 1.0, turnaround: float = 0.1
    ) -> None:
        """Make a client on a port.

        :param port: The line, opened at the speed and format the devices on it use.
        :param response_timeout: How long to wait for a whole reply, in seconds.
        :param turnaround: How long to leave the line quiet after a broadcast, in seconds.
        """
        self._native = _NativeClient(port._native)
        self.response_timeout = response_timeout
        self.turnaround = turnaround

    @staticmethod
    def frame_gap_nanos(settings: SerialSettings) -> int:
        """The silence that separates two frames: 3.5 characters at the line's speed and
        format, and a fixed 1750 microseconds above 19200 baud.

        :param settings: The line's speed and character format.
        :returns: The silence, in nanoseconds, rounded up.
        """
        return _NativeClient.frame_gap_nanos(
            settings.baud, Parity(settings.parity).value, settings.stop_bits
        )

    @property
    def response_timeout(self) -> float:
        """How long the client waits for a whole reply, in seconds."""
        return self._native.response_timeout_micros / 1_000_000

    @response_timeout.setter
    def response_timeout(self, seconds: float) -> None:
        self._native.set_response_timeout_micros(_micros(seconds))

    @property
    def turnaround(self) -> float:
        """How long the client leaves the line quiet after a broadcast, in seconds."""
        return self._native.turnaround_micros / 1_000_000

    @turnaround.setter
    def turnaround(self, seconds: float) -> None:
        self._native.set_turnaround_micros(_micros(seconds))

    def read_coils(self, unit: int, start: int, quantity: int) -> list[bool]:
        """Read coils, function ``0x01``.

        :param unit: The device, 1 to 247.
        :param start: The first coil's address.
        :param quantity: How many, 1 to 2000.
        :returns: The coils' states, in address order.
        :raises ModbusClientError: When the transaction fails.
        """
        return self._native.read_coils(unit, start, quantity)

    def read_discrete_inputs(self, unit: int, start: int, quantity: int) -> list[bool]:
        """Read discrete inputs, function ``0x02``.

        :param unit: The device, 1 to 247.
        :param start: The first input's address.
        :param quantity: How many, 1 to 2000.
        :returns: The inputs' states, in address order.
        :raises ModbusClientError: When the transaction fails.
        """
        return self._native.read_discrete_inputs(unit, start, quantity)

    def read_holding_registers(self, unit: int, start: int, quantity: int) -> list[int]:
        """Read holding registers, function ``0x03``.

        :param unit: The device, 1 to 247.
        :param start: The first register's address.
        :param quantity: How many, 1 to 125.
        :returns: The registers' values, in address order.
        :raises ModbusClientError: When the transaction fails.
        """
        return self._native.read_holding_registers(unit, start, quantity)

    def read_input_registers(self, unit: int, start: int, quantity: int) -> list[int]:
        """Read input registers, function ``0x04``.

        :param unit: The device, 1 to 247.
        :param start: The first register's address.
        :param quantity: How many, 1 to 125.
        :returns: The registers' values, in address order.
        :raises ModbusClientError: When the transaction fails.
        """
        return self._native.read_input_registers(unit, start, quantity)

    def write_single_coil(self, unit: int, address: int, on: bool) -> None:
        """Write one coil, function ``0x05``.

        :param unit: The device, 1 to 247, or :data:`BROADCAST` for every device.
        :param address: The coil's address.
        :param on: The state to write.
        :raises ModbusClientError: When the transaction fails.
        """
        self._native.write_single_coil(unit, address, bool(on))

    def write_single_register(self, unit: int, address: int, value: int) -> None:
        """Write one holding register, function ``0x06``.

        :param unit: The device, 1 to 247, or :data:`BROADCAST` for every device.
        :param address: The register's address.
        :param value: The value to write.
        :raises ModbusClientError: When the transaction fails.
        """
        self._native.write_single_register(unit, address, value)

    def write_multiple_coils(self, unit: int, start: int, values: Sequence[bool]) -> None:
        """Write a run of coils, function ``0x0F``.

        :param unit: The device, 1 to 247, or :data:`BROADCAST` for every device.
        :param start: The first coil's address.
        :param values: The states to write, 1 to 1968 of them, in address order.
        :raises ModbusClientError: When the transaction fails.
        """
        self._native.write_multiple_coils(unit, start, [bool(value) for value in values])

    def write_multiple_registers(self, unit: int, start: int, values: Sequence[int]) -> None:
        """Write a run of holding registers, function ``0x10``.

        :param unit: The device, 1 to 247, or :data:`BROADCAST` for every device.
        :param start: The first register's address.
        :param values: The values to write, 1 to 123 of them, in address order.
        :raises ModbusClientError: When the transaction fails.
        """
        self._native.write_multiple_registers(unit, start, list(values))


def _micros(seconds: float) -> int:
    if not seconds >= 0:
        raise ValueError("a time must be zero or more seconds")
    return round(seconds * 1_000_000)