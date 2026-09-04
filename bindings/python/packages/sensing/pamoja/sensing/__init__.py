"""Sensing and actuation: The parts wired to a board: a thermometer that checks its own bytes, a servo pulse, and a stepper walking its coils.

Installing this distribution installs ``pamoja.sensors``, ``pamoja.actuators``, and re-exports each under its
own name, so a name two of them share stays unambiguous.
"""

from pamoja import sensors, actuators

__all__ = ["sensors", "actuators"]

