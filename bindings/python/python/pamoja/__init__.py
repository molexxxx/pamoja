"""Pamoja device SDK for Python.

This is the ergonomic facade over the native pamoja core: the default import most
users ever touch. It adds idiomatic ergonomics - exceptions for errors, an async
iterator over incoming messages, ``async with`` lifecycle, keyword construction,
and plain Python values instead of buffers - without adding behavior; all real
work happens in the Rust core.

Each capability also has its own module (:mod:`pamoja.mqtt`,
:mod:`pamoja.security`, :mod:`pamoja.codec`, :mod:`pamoja.kit`,
:mod:`pamoja.serial`, :mod:`pamoja.modbus`, :mod:`pamoja.can`,
:mod:`pamoja.gpio`, :mod:`pamoja.sensors`, :mod:`pamoja.actuators`,
:mod:`pamoja.lora`, :mod:`pamoja.lorawan`, :mod:`pamoja.mesh`,
:mod:`pamoja.routing`) for callers who want only one, and the generated low-level
contract remains available at :mod:`pamoja.raw`.

The field-I/O capabilities are re-exported as modules rather than flattened:
their operations are named for their protocol (``frame``, ``raw``,
``parse_frame``), which only reads unambiguously with the protocol in front of
it.
"""

from . import (
    actuators,
    can,
    gpio,
    lora,
    lorawan,
    mesh,
    modbus,
    routing,
    sensors,
    serial,
)
from ._core import PamojaError, version
from .codec import Quantizer, from_cbor, pack_samples, to_cbor, unpack_samples
from .kit import (
    WINDOW_CAPACITY,
    Anomaly,
    Boundary,
    Calibration,
    Coordinate,
    Debounce,
    Depletion,
    Geofence,
    Kalman,
    Median,
    Pid,
    Ramp,
    Smoother,
    Surge,
    Thermostat,
    Trend,
    Window,
    bearing_between,
    deadband,
    distance_between,
)
from .mqtt import MqttClient, MqttMessage, Qos
from .security import DeviceIdentity, Payload, fingerprint, verify

__all__ = [
    "Anomaly",
    "Boundary",
    "Calibration",
    "Coordinate",
    "Debounce",
    "Depletion",
    "DeviceIdentity",
    "Geofence",
    "Kalman",
    "Median",
    "MqttClient",
    "MqttMessage",
    "PamojaError",
    "Payload",
    "Pid",
    "Qos",
    "Quantizer",
    "Ramp",
    "Smoother",
    "Surge",
    "Thermostat",
    "Trend",
    "WINDOW_CAPACITY",
    "Window",
    "actuators",
    "bearing_between",
    "can",
    "deadband",
    "distance_between",
    "fingerprint",
    "from_cbor",
    "gpio",
    "lora",
    "lorawan",
    "mesh",
    "modbus",
    "pack_samples",
    "routing",
    "sensors",
    "serial",
    "to_cbor",
    "unpack_samples",
    "verify",
    "version",
]
