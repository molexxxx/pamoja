"""Idiomatic device-profile facade.

A profile is a named, pre-wired bundle: a control policy, a publish topic, and a
power schedule. Instantiate one rather than choosing algorithms and tuning
constants by hand.

A :class:`Profile` is the manifest, which loads from and saves to JSON so it
ships as a file. A :class:`Controller` is the decision logic that manifest
describes: hand it a reading and it says what the output should do and whether
the reading crossed a threshold worth raising. The presentation a dashboard
reads travels inside the manifest JSON.
"""

from __future__ import annotations

import enum

from pamoja._native import (
    AlertReport,
    ControlPolicy,
    Controller,
    PowerScheduleSpec,
    Profile,
    Reaction,
)

__all__ = [
    "AlertKind",
    "AlertReport",
    "ControlKind",
    "ControlPolicy",
    "Controller",
    "PowerScheduleSpec",
    "Profile",
    "Reaction",
]


class ControlKind(str, enum.Enum):
    """Which control policy a profile applies to each reading."""

    #: Hold a reading near a setpoint by switching an output on and off.
    SETPOINT = "Setpoint"
    #: Watch a falling level and warn before it reaches empty.
    LEVEL = "Level"
    #: Warn when a reading changes faster than a limit.
    SURGE = "Surge"
    #: Report readings only, with no output and no alerts.
    MONITOR = "Monitor"


class AlertKind(str, enum.Enum):
    """Which threshold a reading crossed."""

    #: A controlled reading drifted outside its safe band.
    OUT_OF_RANGE = "OutOfRange"
    #: A falling level will reach empty within a few more samples.
    RUNNING_OUT = "RunningOut"
    #: A reading is changing faster than its safe rate.
    CHANGING_FAST = "ChangingFast"
