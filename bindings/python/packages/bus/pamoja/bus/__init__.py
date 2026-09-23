"""Idiomatic event-bus facade.

One publisher, many subscribers, inside a single process. It is how the parts of
a gateway talk to each other without knowing about each other, so a sampler can
announce a reading and whatever cares about readings picks it up.

A subscriber only sees events published after it existed, so subscribe before
publishing anything it needs to see. An ``EventBus`` endpoint publishes and
receives; an ``EventPublisher`` only publishes, so a part that announces and never
reads holds one of those.
"""

from __future__ import annotations

from pamoja._native import EventBus, EventPublisher

__all__ = ["EventBus", "EventPublisher"]
