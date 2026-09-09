"""Field I/O: The wires a gateway actually has: framed serial packets, an RS485 request and the reply it draws, a CAN frame, the address a chip answers on, and the bus that carries a driver to the part.

Installing this distribution installs ``pamoja.serial``, ``pamoja.modbus``, ``pamoja.can``, ``pamoja.gpio``, ``pamoja.core``, and re-exports each under its
own name, so a name two of them share stays unambiguous.
"""

from pamoja import serial, modbus, can, gpio, core

__all__ = ["serial", "modbus", "can", "gpio", "core"]

