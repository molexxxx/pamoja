"""Field I/O: The wires a gateway actually has: framed serial packets, an RS485 request and the reply it draws, a CAN frame, and the address a chip answers on.

Installing this distribution installs ``pamoja.serial``, ``pamoja.modbus``, ``pamoja.can``, ``pamoja.gpio``, and re-exports each under its
own name, so a name two of them share stays unambiguous.
"""

from pamoja import serial, modbus, can, gpio

__all__ = ["serial", "modbus", "can", "gpio"]

