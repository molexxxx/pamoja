"""Idiomatic helper-math facade.

The helpers are named for the goal rather than the technique, with the real
algorithm one layer down: smooth a noisy reading, hold a value with a PID, warn
before a tank runs dry, and notice when a tracked point leaves its area.

They are synchronous and allocation-free in the core, so this module re-exports
most of the generated classes rather than wrapping them. What it adds is a
:class:`Coordinate` for the geo helpers, so a fix travels as one value instead of
a pair of loose floats, and the :class:`Boundary` and :class:`Edge` enums for the
crossing states a :class:`Geofence` and a :class:`Trigger` report.

A reading that is not a finite number, such as the NaN a failed sensor reports,
is ignored by every helper that keeps state, so one bad reading cannot poison an
average or an integral. :class:`Anomaly` flags such a reading instead.
"""

from __future__ import annotations

import enum
from typing import NamedTuple

from pamoja._native import Anomaly, Median, Trend, Window
from pamoja._native import window_capacity as _window_capacity
from pamoja._native import Calibration, Debounce, Depletion, Kalman, Pid, Ramp, Smoother, Surge, Thermostat
from pamoja._native import Geofence as _NativeGeofence
from pamoja._native import Trigger as _NativeTrigger
from pamoja._native import bearing_between as _bearing_between
from pamoja._native import deadband
from pamoja._native import distance_between as _distance_between

#: How many readings a windowed helper keeps.
WINDOW_CAPACITY = _window_capacity()

__all__ = [
    "Window",
    "WINDOW_CAPACITY",
    "Trend",
    "Median",
    "Anomaly",
    "Boundary",
    "Calibration",
    "Coordinate",
    "Debounce",
    "Depletion",
    "Geofence",
    "Kalman",
    "Pid",
    "Ramp",
    "Smoother",
    "Surge",
    "Thermostat",
    "Trigger",
    "Edge",
    "bearing_between",
    "deadband",
    "distance_between",
]


class Edge(str, enum.Enum):
    """What a :class:`Trigger` reports when a reading changes its state."""

    #: The reading just crossed the line: the condition became true.
    SET = "set"
    #: The reading just came back past the release band: the condition stopped holding.
    CLEARED = "cleared"


class Trigger:
    """Fires once when a reading crosses a line, and not again until it has come back.

    A trigger is a threshold with hysteresis that reports its edges: ``update``
    answers :attr:`Edge.SET` on the reading that crosses the line,
    :attr:`Edge.CLEARED` on the one that comes back past the release band, and
    ``None`` for every reading in between, so an alert is sent once rather than on
    every reading while the condition holds.

    Example::

        dry = Trigger.below(20.0, 2.0)
        dry.update(25.0)  # None
        dry.update(19.0)  # Edge.SET
        dry.update(21.0)  # None, still inside the release band
        dry.update(22.5)  # Edge.CLEARED
    """

    __slots__ = ("_native",)

    def __init__(self, native: _NativeTrigger) -> None:
        """Wrap a generated trigger; use :meth:`above` or :meth:`below` instead."""
        self._native = native

    @classmethod
    def above(cls, threshold: float, hysteresis: float) -> Trigger:
        """Create a trigger that fires when a reading rises above the line.

        :param threshold: The line to watch.
        :param hysteresis: How far below the line the reading must fall to clear.
        :returns: A trigger that has not fired.
        """
        return cls(_NativeTrigger.above(threshold, hysteresis))

    @classmethod
    def below(cls, threshold: float, hysteresis: float) -> Trigger:
        """Create a trigger that fires when a reading falls below the line.

        :param threshold: The line to watch.
        :param hysteresis: How far above the line the reading must rise to clear.
        :returns: A trigger that has not fired.
        """
        return cls(_NativeTrigger.below(threshold, hysteresis))

    def update(self, reading: float) -> Edge | None:
        """Feed a reading in and report the edge it caused.

        :param reading: The latest reading. One that is not a finite number is ignored.
        :returns: The edge, or ``None`` while nothing changed.
        """
        edge = self._native.update(reading)
        return None if edge is None else Edge(edge)

    @property
    def is_set(self) -> bool:
        """Whether the condition currently holds."""
        return self._native.is_set

    @property
    def threshold(self) -> float:
        """The line the trigger watches."""
        return self._native.threshold

    @property
    def hysteresis(self) -> float:
        """The release band on the far side of the line."""
        return self._native.hysteresis

    @property
    def watches_above(self) -> bool:
        """Whether the trigger watches a rising reading, as :meth:`above` makes it."""
        return self._native.watches_above


class Coordinate(NamedTuple):
    """A latitude and longitude in degrees."""

    #: Degrees north of the equator, negative for south.
    latitude: float
    #: Degrees east of the prime meridian, negative for west.
    longitude: float


class Boundary(str, enum.Enum):
    """Where a fix sits relative to a :class:`Geofence`, including a crossing."""

    #: The fix is inside the fence and was inside before, or is the first fix inside.
    INSIDE = "Inside"
    #: The fix is outside the fence and was outside before, or is the first fix outside.
    OUTSIDE = "Outside"
    #: The fix just crossed from inside to outside: the moment to raise a breach alert.
    EXITED = "Exited"
    #: The fix just crossed from outside back inside.
    ENTERED = "Entered"


class Geofence:
    """Keeps a tracked point inside an area, and notices when it leaves.

    A fence is a center and a radius; feeding it successive fixes reports whether
    each is inside or outside and, crucially, the single fix that crossed, so an
    alert fires once on the crossing rather than on every fix while away.

    Example::

        pen = Geofence(Coordinate(-1.2921, 36.8219), 50.0)
        pen.update(Coordinate(-1.2921, 36.8219))  # Boundary.INSIDE
        pen.update(Coordinate(-1.2930, 36.8219))  # Boundary.EXITED
    """

    __slots__ = ("_native",)

    def __init__(self, center: Coordinate, radius_m: float) -> None:
        """Create a circular fence around a center fix.

        :param center: The center of the fence.
        :param radius_m: The fence radius, in meters.
        """
        self._native = _NativeGeofence(center.latitude, center.longitude, radius_m)

    def update(self, point: Coordinate) -> Boundary:
        """Feed a fix in and report where it sits, including a single crossing.

        :param point: The latest fix.
        :returns: The boundary state for this fix.
        """
        return Boundary(self._native.update(point.latitude, point.longitude))

    def contains(self, point: Coordinate) -> bool:
        """Report whether a fix lies inside, without recording a crossing.

        :param point: The fix to test.
        :returns: ``True`` if the fix is inside the fence.
        """
        return self._native.contains(point.latitude, point.longitude)


def distance_between(origin: Coordinate, destination: Coordinate) -> float:
    """Return the great-circle distance between two coordinates, in meters.

    :param origin: The coordinate to measure from.
    :param destination: The coordinate to measure to.
    :returns: The distance in meters.
    """
    return _distance_between(
        origin.latitude, origin.longitude, destination.latitude, destination.longitude
    )


def bearing_between(origin: Coordinate, destination: Coordinate) -> float:
    """Return the initial bearing from one coordinate to another, in degrees.

    :param origin: The coordinate to measure from.
    :param destination: The coordinate to measure to.
    :returns: The bearing in degrees, clockwise from north.
    """
    return _bearing_between(
        origin.latitude, origin.longitude, destination.latitude, destination.longitude
    )
