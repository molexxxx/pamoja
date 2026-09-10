"""Idiomatic device-profile facade.

A profile is a named, pre-wired bundle: a control policy, a publish topic, and a
power schedule. Instantiate one rather than choosing algorithms and tuning
constants by hand.

A :class:`Profile` is the manifest, which loads from and saves to JSON so it
ships as a file. A :class:`Controller` is the decision logic that manifest
describes: hand it a reading and it says what the output should do and whether
the reading crossed a threshold worth raising. The presentation a dashboard
reads travels inside the manifest JSON and is read and built here as a typed
:class:`Presentation` of :class:`ElementSpec` entries, drawn with a :class:`Viz`.
"""

from __future__ import annotations

import enum

from pamoja._native import (
    AlertReport,
    ControlPolicy,
    Controller,
    ElementSpec,
    PowerScheduleSpec,
    Presentation,
    Profile,
    Reaction,
    Theme,
)

__all__ = [
    "AlertKind",
    "AlertReport",
    "ControlKind",
    "ControlPolicy",
    "Controller",
    "ElementSpec",
    "PowerScheduleSpec",
    "Presentation",
    "Profile",
    "Reaction",
    "Theme",
    "Viz",
]


class Viz(str, enum.Enum):
    """The graphic a dashboard draws an element with, named by the instrument rather
    than the quantity. The values are the ones a manifest carries."""

    #: A rolling sparkline of recent values.
    SPARK = "spark"
    #: A 270-degree arch gauge, for a fraction or percentage.
    GAUGE = "gauge"
    #: A half-dial with a needle, for a pressure or flow reading.
    DIAL = "dial"
    #: A horizontal bar with a safe-band tick, for a level or stock.
    BAR = "bar"
    #: A thermometer, for a temperature.
    THERMOMETER = "thermometer"
    #: A liquid-filled droplet, for humidity or moisture.
    DROPLET = "droplet"
    #: A segmented battery cell, for a state of charge or voltage.
    BATTERY = "battery"
    #: An anemometer, for wind speed.
    WIND = "wind"
    #: A sun whose corona grows with the reading, for illuminance.
    SUN = "sun"
    #: An acoustic waveform, for sound level or an acoustic event.
    WAVE = "wave"
    #: A labeled state chip, lit when the state reads as on.
    SWITCH = "switch"
    #: A pipe valve, open along the flow or closed across it.
    VALVE = "valve"
    #: A row of hash-chained blocks, for a tamper-evident record count.
    CHAIN = "chain"
    #: A neighbor-mesh topology map, for a mesh node's peers.
    MESH = "mesh"
    #: A plain numeric counter, for a node or network stat.
    COUNT = "count"


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
    #: A kind the library does not ship, named by the manifest in ``custom_kind`` and
    #: decided by the program's own code from ``params``.
    CUSTOM = "Custom"


class AlertKind(str, enum.Enum):
    """Which threshold a reading crossed."""

    #: A controlled reading drifted outside its safe band.
    OUT_OF_RANGE = "OutOfRange"
    #: A falling level will reach empty within a few more samples.
    RUNNING_OUT = "RunningOut"
    #: A reading is changing faster than its safe rate.
    CHANGING_FAST = "ChangingFast"
    #: A condition a policy of the program's own raised, named by ``code``.
    CUSTOM = "Custom"
