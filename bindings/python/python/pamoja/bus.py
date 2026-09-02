"""Idiomatic event-bus facade.

One publisher, many subscribers, inside a single process. It is how the parts of
a gateway talk to each other without knowing about each other, so a sampler can
announce a reading and whatever cares about readings picks it up.

A subscriber only sees events published after it existed, so subscribe before
publishing anything it needs to see.
"""

from __future__ import annotations

from ._core import EventBus

__all__ = ["EventBus"]
