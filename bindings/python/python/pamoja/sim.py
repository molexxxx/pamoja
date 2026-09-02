"""Idiomatic simulated-device facade.

Drive a whole node with no hardware attached: a sensor that drifts, a replay of a
real capture, an actuator that records what it was told, and a robot that moves
only in arithmetic. A lossy link is `Transport.degraded`, since it wraps a
transport rather than standing alone.
"""

from __future__ import annotations

from ._core import Pose, RecordingActuatorHandle, Replay, SimulatedRobot, SimulatedSensor

__all__ = [
    "Pose",
    "RecordingActuator",
    "Replay",
    "SimulatedRobot",
    "SimulatedSensor",
]

#: An actuator that records every command instead of acting on one.
RecordingActuator = RecordingActuatorHandle
