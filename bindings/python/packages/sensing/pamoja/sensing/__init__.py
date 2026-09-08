"""Sensing and actuation: The parts wired to a board: a thermometer that checks its own bytes, a servo pulse, a stepper walking its coils, and a part of your own.

Installing this distribution installs ``pamoja.sensors``, ``pamoja.actuators``, ``pamoja.core``, and re-exports each under its
own name, so a name two of them share stays unambiguous.
"""

from pamoja import sensors, actuators, core

__all__ = ["sensors", "actuators", "core"]

