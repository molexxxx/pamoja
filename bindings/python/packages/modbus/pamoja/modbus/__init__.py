"""Idiomatic Modbus RTU facade.

Modbus over RS485 is what cheap industrial sensing speaks: energy meters, soil
probes, water-quality transmitters, pump controllers. Each request builder here
returns a complete frame with its CRC, ready to write to a port, and a reply comes
back through :func:`parse_frame` as an object that reads its own values.
"""

from __future__ import annotations

import enum
from typing import Sequence

from pamoja._native import ModbusFrame
from pamoja._native import modbus_crc16 as _crc16
from pamoja._native import modbus_parse_frame as _parse_frame
from pamoja._native import modbus_raw as _raw
from pamoja._native import modbus_read_coils as _read_coils
from pamoja._native import modbus_read_discrete_inputs as _read_discrete_inputs
from pamoja._native import modbus_read_holding_registers as _read_holding_registers
from pamoja._native import modbus_read_input_registers as _read_input_registers
from pamoja._native import modbus_write_multiple_coils as _write_multiple_coils
from pamoja._native import modbus_write_multiple_registers as _write_multiple_registers
from pamoja._native import modbus_write_single_coil as _write_single_coil
from pamoja._native import modbus_write_single_register as _write_single_register

__all__ = [
    "Exception_",
    "Function",
    "ModbusFrame",
    "crc16",
    "parse_frame",
    "raw",
    "read_coils",
    "read_discrete_inputs",
    "read_holding_registers",
    "read_input_registers",
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
    #: A gateway reached the target device but got no response.
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
