"""Idiomatic LoRa radio facade.

``pamoja.lora`` works out what a link costs and how far it reaches; this puts a radio chip
on the air to match. For the Semtech SX1261, SX1262, and LLCC68, :mod:`pamoja.radios.sx126x`
builds the bytes of every command and decodes every answer, and chooses the amplifier
setting a regional EIRP ceiling allows behind an antenna. For the SX1276 family,
:mod:`pamoja.radios.sx127x` gives the register values and decodes the readings.
:class:`DutyCycle` holds the radio
silent for the off time a duty-cycle limit owes after each frame. On a Linux board,
:class:`LoraRadio` opens a module of either family through the kernel's spidev and GPIO
character devices and drives it with the same calls.
"""

from __future__ import annotations

from pamoja._native import LoraRadio, LoraReception
from pamoja._native import RadioDutyCycle as DutyCycle

from . import sx126x, sx127x

__all__ = ["DutyCycle", "LoraRadio", "LoraReception", "sx126x", "sx127x"]
