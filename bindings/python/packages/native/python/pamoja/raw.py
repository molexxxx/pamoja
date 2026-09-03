"""Low-level generated contract for the pamoja core (the escape hatch).

This module re-exports the native :mod:`pamoja._native` extension verbatim. It is
the Python analog of ``@pamoja/core/raw`` in the Node binding: anything the
ergonomic facade does not surface is still reachable here without leaving the SDK.
"""

from pamoja import _native
from pamoja._native import *  # noqa: F401,F403

__all__ = [name for name in dir(_native) if not name.startswith("_")]
