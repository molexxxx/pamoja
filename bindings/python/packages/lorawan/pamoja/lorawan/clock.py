"""Application layer clock synchronization, TS003-2.0.0, on port :data:`PORT`.

A device that has no clock of its own still needs the time: a multicast window and a scheduled
reboot both depend on it. The device says what time it believes it is, the server answers with
the difference, and a four-bit token keeps a late answer from pulling a corrected clock back.

>>> from pamoja.lorawan import clock, package_encode, package_parse
>>> sync = clock.ClockSync()
>>> asked = package_parse(clock.PORT, True, sync.request(1_000_000))
>>> answer = package_encode(
...     clock.PORT, "app_time_ans", time_correction=12, token=asked.token
... )
>>> sync.heard(answer).correction
12
"""

from __future__ import annotations

from pamoja._native import LORAWAN_CLOCK_PORT, LorawanClockHeard, LorawanClockSync

__all__ = ["PORT", "ClockHeard", "ClockSync"]

#: The port this package is spoken on.
PORT = LORAWAN_CLOCK_PORT

#: What a device made of a downlink on the clock port.
ClockHeard = LorawanClockHeard

#: The clock synchronization package running on a device.
ClockSync = LorawanClockSync
