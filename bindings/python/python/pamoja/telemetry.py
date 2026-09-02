"""Idiomatic telemetry facade.

A node that ships every event it produces will spend more on reporting than on
the job it was installed to do, and on a satellite link that is money. A reporter
ships what is worth its bytes, counts everything either way, and moves its own
bar as the link gets more expensive, so the aggregate picture survives even when
the detail cannot be sent.

The generated binding decides on the level alone, since that is all the core
reporter reads. The event a caller writes travels no further than this layer,
which hands it straight back when it should be shipped.
"""

from __future__ import annotations

import enum
from dataclasses import dataclass

from ._core import Reporter as _Reporter
from ._core import Snapshot
from ._core import link_cost_threshold as _link_cost_threshold

__all__ = [
    "Event",
    "Level",
    "LinkCost",
    "Reporter",
    "Snapshot",
    "link_cost_threshold",
]


class Level(str, enum.Enum):
    """How urgent an event is."""

    #: Fine-grained detail, useful only when chasing a specific problem.
    TRACE = "Trace"
    #: Diagnostic detail for development.
    DEBUG = "Debug"
    #: A normal, noteworthy event.
    INFO = "Info"
    #: Something unexpected that the node recovered from.
    WARN = "Warn"
    #: A failure that needs attention.
    ERROR = "Error"


class LinkCost(str, enum.Enum):
    """What the link back to the network currently costs."""

    #: Bytes are effectively free, such as on wired power and ethernet.
    FREE = "Free"
    #: Bytes are paid for, such as on a cellular plan.
    METERED = "Metered"
    #: Bytes are scarce, such as on a satellite or long-range radio link.
    EXPENSIVE = "Expensive"
    #: Nothing can be shipped at all.
    OFFLINE = "Offline"


@dataclass(frozen=True)
class Event:
    """A structured telemetry event.

    The code is a stable, short label such as ``battery.low`` rather than a
    free-form message, so events stay small and group cleanly into counts.
    """

    #: How urgent the event is.
    level: Level
    #: A stable, short identifier for what happened.
    code: str
    #: An optional measurement, such as the charge that triggered it.
    value: float | None = None


def link_cost_threshold(cost: LinkCost) -> Level:
    """Return the level a link cost calls for.

    :param cost: What the link currently costs.
    :returns: The lowest level still worth its bytes at that cost.
    """
    return Level(_link_cost_threshold(cost.value))


class Reporter:
    """Record events, ship the ones worth their bytes, and count them all."""

    def __init__(self, threshold: Level = Level.INFO) -> None:
        """Create a reporter that ships events at or above ``threshold``.

        :param threshold: The lowest level to ship.
        """
        self._inner = _Reporter(threshold.value)

    @property
    def threshold(self) -> Level:
        """The level this reporter is currently shipping from."""
        return Level(self._inner.threshold)

    @threshold.setter
    def threshold(self, threshold: Level) -> None:
        self._inner.threshold = threshold.value

    @property
    def total(self) -> int:
        """How many events have been seen across every level."""
        return self._inner.total

    @property
    def emitted(self) -> int:
        """How many events passed the threshold and were shipped."""
        return self._inner.emitted

    @property
    def dropped(self) -> int:
        """How many events the threshold dropped."""
        return self._inner.dropped

    def adapt_to(self, cost: LinkCost) -> None:
        """Move the threshold to match what the link now costs.

        :param cost: What the link currently costs.
        """
        self._inner.adapt_to(cost.value)

    def record(self, event: Event) -> Event | None:
        """Record an event, returning it when it should be shipped.

        :param event: The event that occurred.
        :returns: The same event when it passed the threshold, or ``None`` when
            it was counted and dropped.
        """
        return event if self._inner.record(event.level.value) else None

    def count(self, level: Level) -> int:
        """Return how many events have been seen at a level, shipped or not.

        :param level: The level to count.
        :returns: The number of events recorded at that level.
        """
        return self._inner.count(level.value)

    def snapshot(self) -> Snapshot:
        """Take a snapshot of the counters to ship in place of the stream.

        :returns: The per-level counts and the shipped and dropped totals.
        """
        return self._inner.snapshot()
