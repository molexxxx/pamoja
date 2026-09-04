"""The pamoja engine's surface: the runtime version, the error every native call
raises, and the transport every link shares.

This is the counterpart of the ``pamoja-core`` crate, and like it, it is small; the
compiled engine is ``pamoja-native``, which this package depends on. Each capability
is its own package (``pamoja-mqtt`` gives ``pamoja.mqtt``, and so on), and
``pamoja`` installs all of them.

A ladder rung, a fault injector, and a degraded link all take some transport.
Python has no way to say "any transport", so one class holds whichever kind was
built and dispatches to it. Composing consumes a transport, because the thing it
is composed into owns it from then on; a consumed transport is emptied rather
than left aliasing what now belongs to a ladder, so using one twice raises.
"""

from __future__ import annotations

from pamoja._native import Message, PamojaError, version
from pamoja._native import PyTransport as Transport

__all__ = ["Message", "PamojaError", "Transport", "version"]
