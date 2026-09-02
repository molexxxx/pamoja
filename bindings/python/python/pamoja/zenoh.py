"""Idiomatic Zenoh key-expression facade.

A key expression is how a Zenoh network addresses data: a slash-separated path
that may carry the ``*`` and ``**`` wildcards, so one subscriber names a whole
subtree of a fleet rather than each node in it.

Only the naming rules cross. Running a Zenoh session needs the std-only zenoh
stack, which would land in every wheel, so it stays in the Rust crate.
"""

from __future__ import annotations

from ._core import (
    keyexpr_canonize as canonize,
    keyexpr_is_canon as is_canon,
    keyexpr_is_valid as is_valid,
    keyexpr_matches as matches,
)

__all__ = [
    "canonize",
    "is_canon",
    "is_valid",
    "matches",
]
