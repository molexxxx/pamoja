"""Idiomatic power-scheduling facade.

A node on a battery and a panel has to decide how often to do anything at all. A
duty cycle says how the time splits between working and sleeping; a power plan
says how that split should change as the charge falls, so a node that would
otherwise go dark in a cloudy week keeps reporting, less often.
"""

from __future__ import annotations

import enum

from pamoja._native import DutyCycle, PowerPlan

__all__ = [
    "DutyCycle",
    "PowerMode",
    "PowerPlan",
    "duty_cycle",
    "power_plan",
]


class PowerMode(str, enum.Enum):
    """What a node should be doing at the current state of charge."""

    #: Full duty, because the charge is healthy.
    ACTIVE = "Active"
    #: Reduced duty, to conserve charge.
    SAVER = "Saver"
    #: Minimum duty, to stay alive as long as possible.
    CRITICAL = "Critical"


def duty_cycle(active_us: int, sleep_us: int) -> DutyCycle:
    """Split a period between working and sleeping.

    :param active_us: How long the node works each period, in microseconds.
    :param sleep_us: How long it sleeps each period, in microseconds.
    :returns: The duty cycle.
    """
    return DutyCycle(active_us, sleep_us)


def power_plan(active_us: int, saver_us: int, critical_us: int) -> PowerPlan:
    """Describe how a work interval stretches as the charge falls.

    The defaults enter :attr:`PowerMode.SAVER` below 50% charge and
    :attr:`PowerMode.CRITICAL` below 20%; move them with
    :meth:`PowerPlan.with_thresholds`.

    :param active_us: The interval at a healthy charge, in microseconds.
    :param saver_us: The longer interval used to conserve, in microseconds.
    :param critical_us: The longest interval, in microseconds.
    :returns: The power plan.
    """
    return PowerPlan(active_us, saver_us, critical_us)
