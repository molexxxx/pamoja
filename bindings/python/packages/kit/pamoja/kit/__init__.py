"""Idiomatic helper-math facade.

The helpers are named for the goal rather than the technique, with the real
algorithm one layer down: smooth a noisy reading, hold a value with a PID, warn
before a tank runs dry, and notice when a tracked point leaves its area.

They are synchronous and allocation-free in the core, so this module re-exports
the generated classes rather than wrapping them. What it adds is a
:class:`Coordinate` for the geo helpers, so a fix travels as one value instead of
a pair of loose floats, and a :class:`Boundary` enum for the crossing states.
"""

from __future__ import annotations

import enum
from typing import NamedTuple

from pamoja._native import Anomaly, Median, Trend, Window
from pamoja._native import window_capacity as _window_capacity
from pamoja._native import Calibration, Debounce, Depletion, Kalman, Pid, Ramp, Smoother, Surge, Thermostat
from pamoja._native import Geofence as _NativeGeofence
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
    "bearing_between",
    "deadband",
    "distance_between",
]


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

    A fence is a centre and a radius; feeding it successive fixes reports whether
    each is inside or outside and, crucially, the single fix that crossed, so an
    alert fires once on the crossing rather than on every fix while away.

    Example::

        pen = Geofence(Coordinate(-1.2921, 36.8219), 50.0)
        pen.update(Coordinate(-1.2921, 36.8219))  # Boundary.INSIDE
        pen.update(Coordinate(-1.2930, 36.8219))  # Boundary.EXITED
    """

    __slots__ = ("_native",)

    def __init__(self, center: Coordinate, radius_m: float) -> None:
        """Create a circular fence around a centre fix.

        :param center: The centre of the fence.
        :param radius_m: The fence radius, in metres.
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
    """Return the great-circle distance between two coordinates, in metres.

    :param origin: The coordinate to measure from.
    :param destination: The coordinate to measure to.
    :returns: The distance in metres.
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
