"""Idiomatic MAVLink wire-protocol facade.

MAVLink is the language drones speak: PX4 and ArduPilot autopilots and MAVSDK
ground stations all exchange MAVLink frames, so talking to a vehicle means
putting exactly the right bytes on the wire and trusting the bytes that come
back. This is that byte layer: v1 and v2 frames, the CRC-16/MCRF4XX checksum
every frame carries, the per-message ``CRC_EXTRA`` seed that catches a frame
whose shape does not match, and MAVLink 2 signing.

Nothing here is limited to the messages this build happens to know. The common
dialect's seeds are built in, and :class:`Dialect` carries any others, derived
from a message definition the way the specification does.

Above the bytes sits the shape: :class:`MessageSchema` names a message's fields,
so a :class:`MavlinkMessage` is filled in and read back by name rather than by
byte offset, and :class:`MessageSchemaBuilder` describes a message this build has
never heard of.

Above the messages sit the exchanges: :class:`MissionSender` and
:class:`MissionReceiver` carry a plan between a station and a vehicle,
:class:`CommandProtocol` matches a command to its acknowledgement and counts
retries, and :func:`local_position`, :func:`local_velocity`, and
:func:`global_position` build setpoints. Each takes a frame off the link and
hands back the frame to send, with no IO or timers of its own.
"""

from __future__ import annotations

import time
from enum import IntEnum, IntFlag

from ._core import (
    AckOutcome,
    CommandProtocol,
    Dialect,
    MavlinkFieldInfo,
    MavlinkFrame,
    MavlinkHeader,
    MavlinkMessage,
    MavlinkParser,
    MavlinkSigner,
    MavlinkVerifier,
    MessageSchema,
    MessageSchemaBuilder,
    MissionReceiver,
    MissionSender,
    ReceiverStep,
    SenderStep,
    mavlink_crc16_mcrf4xx,
    mavlink_known_crc_extra,
    mavlink_known_messages,
    mavlink_message_crc_extra,
    mavlink_offboard_global_position,
    mavlink_offboard_local_position,
    mavlink_offboard_local_velocity,
    mavlink_offboard_type_mask,
    mavlink_timestamp_from_unix_micros,
)

__all__ = [
    "AckOutcome",
    "CommandProtocol",
    "DEFAULT_TIMESTAMP_WINDOW",
    "Dialect",
    "FieldType",
    "KEY_LEN",
    "MAX_FRAME",
    "MAX_PAYLOAD",
    "MAX_RETRIES",
    "MavlinkFieldInfo",
    "MavlinkFrame",
    "MavlinkHeader",
    "MavlinkMessage",
    "MavlinkParser",
    "MavlinkSigner",
    "MavlinkVerifier",
    "MessageSchema",
    "MessageSchemaBuilder",
    "MissionReceiver",
    "MissionSender",
    "ReceiverStep",
    "SIGNATURE_LEN",
    "SenderStep",
    "TypeMask",
    "crc16",
    "frame",
    "from_dict",
    "global_position",
    "known_crc_extra",
    "known_messages",
    "local_position",
    "local_velocity",
    "message",
    "message_crc_extra",
    "schema_for",
    "timestamp_from_unix_micros",
    "timestamp_now",
    "to_dict",
    "type_mask",
]

#: The largest payload a frame can carry, in bytes.
MAX_PAYLOAD = 255

#: The largest frame, in bytes, header, checksum and signature included.
MAX_FRAME = 280

#: The length of a v2 signature block, in bytes.
SIGNATURE_LEN = 13

#: The length of a signing key, in bytes.
KEY_LEN = 32

#: The default window a verifier accepts a timestamp within.
DEFAULT_TIMESTAMP_WINDOW = 6_000_000

#: The number of times a request is retransmitted before a transfer is abandoned, as
#: the mission protocol recommends.
MAX_RETRIES = 5


def crc16(data: bytes) -> int:
    """Return the CRC-16/MCRF4XX checksum of a byte string.

    This is the checksum every MAVLink frame carries, exposed because a host
    that implements part of the protocol itself needs the same arithmetic.

    :param data: The data to checksum.
    :returns: The checksum.
    """
    return mavlink_crc16_mcrf4xx(data)


def message_crc_extra(name: str, fields: list[tuple[str, str, int]]) -> int:
    """Derive the ``CRC_EXTRA`` seed of a message from its definition.

    This is what makes a dialect this build has never seen usable: given a
    message's name and its base fields in wire order, the seed comes out the
    same as the one the dialect publishes, and a frame carrying that message
    then checks like any other.

    Extension fields are excluded from the seed and must not be listed, which is
    what lets a peer that predates them still check the frame.

    :param name: The message name, such as ``HEARTBEAT``.
    :param fields: The base fields in wire order, as ``(type, name, array_len)``
        triples; ``array_len`` is ``0`` for a scalar.
    :returns: The seed.

    >>> message_crc_extra("PRIVATE_STATUS", [("uint32_t", "uptime", 0)]) >= 0
    True
    """
    return mavlink_message_crc_extra(name, fields)


def known_crc_extra(msgid: int) -> int | None:
    """Return the ``CRC_EXTRA`` the common dialect publishes for a message id.

    :param msgid: The message id to look up.
    :returns: The seed, or ``None`` for an id outside the common dialect, which
        is what a :class:`Dialect` is for.

    >>> known_crc_extra(0)
    50
    >>> known_crc_extra(9999) is None
    True
    """
    return mavlink_known_crc_extra(msgid)


def timestamp_from_unix_micros(unix_micros: int) -> int:
    """Convert Unix time into the timestamp MAVLink signing counts in.

    :param unix_micros: The time in microseconds since the Unix epoch.
    :returns: The signing timestamp, in units of ten microseconds since 2015.
    """
    return mavlink_timestamp_from_unix_micros(unix_micros)


def timestamp_now() -> int:
    """Return a signing timestamp for now.

    :returns: The signing timestamp matching the current clock.
    """
    return mavlink_timestamp_from_unix_micros(int(time.time() * 1_000_000))


def frame(header: MavlinkHeader, msgid: int, payload: bytes) -> MavlinkFrame:
    """Build a v2 frame carrying a message the common dialect defines.

    The seed is looked up rather than passed, which is the usual case: a sender
    emitting a standard message should not have to know its checksum constant.

    :param header: The addressing fields to stamp on the frame.
    :param msgid: The message id.
    :param payload: The message payload.
    :returns: The frame ready to send.
    :raises ValueError: If the id is outside the common dialect, in which case
        build the frame with :meth:`MavlinkFrame.raw` and a seed of your own.

    >>> heartbeat = bytes([0, 0, 0, 0, 18, 0, 0, 4, 3])
    >>> sent = frame(MavlinkHeader(1, 1), 0, heartbeat)
    >>> sent.message_id
    0
    >>> MavlinkFrame.parse_known(sent.bytes).payload == heartbeat
    True
    """
    crc_extra = mavlink_known_crc_extra(msgid)
    if crc_extra is None:
        raise ValueError(
            f"message {msgid} is not in the common dialect; "
            "supply its CRC_EXTRA with MavlinkFrame.raw"
        )
    return MavlinkFrame.encode_v2(header, msgid, payload, crc_extra)


class FieldType(IntEnum):
    """The field types a message definition uses.

    A builder accepts either one of these or the name a dialect writes, so
    ``FieldType.UINT32`` and ``"uint32_t"`` mean the same thing.
    """

    UINT8 = 1
    INT8 = 2
    CHAR = 3
    UINT16 = 4
    INT16 = 5
    UINT32 = 6
    INT32 = 7
    UINT64 = 8
    INT64 = 9
    FLOAT = 10
    DOUBLE = 11


def schema_for(message: int | str) -> MessageSchema:
    """Return the shape of a message the engine types.

    :param message: The message id or name, such as ``33`` or
        ``"GLOBAL_POSITION_INT"``.
    :returns: The shape.
    :raises ValueError: If this build does not type that message, in which case
        describe it with :class:`MessageSchemaBuilder`.

    >>> schema_for("GLOBAL_POSITION_INT").id
    33
    >>> schema_for(0).name
    'HEARTBEAT'
    """
    if isinstance(message, int):
        return MessageSchema.for_id(message)
    return MessageSchema.for_name(message)


def known_messages() -> list[str]:
    """Return the names of every message this build types, in message-id order.

    :returns: The message names, each usable with :func:`schema_for`.

    >>> "HEARTBEAT" in known_messages()
    True
    """
    return mavlink_known_messages()


def message(shape: MessageSchema | int | str) -> MavlinkMessage:
    """Create a message with every field zero.

    :param shape: The shape to build, or the id or name of a message the engine
        types.
    :returns: The zeroed message, ready for its fields to be set.

    >>> heartbeat = message("HEARTBEAT")
    >>> heartbeat.set("type", 18)  # MAV_TYPE_ONBOARD_CONTROLLER
    >>> heartbeat.set("system_status", 4)  # MAV_STATE_ACTIVE
    >>> frame = heartbeat.to_frame(MavlinkHeader(1, 1))
    >>> frame.message_id
    0
    """
    if not isinstance(shape, MessageSchema):
        shape = schema_for(shape)
    return MavlinkMessage.empty(shape)


def to_dict(built: MavlinkMessage, shape: MessageSchema) -> dict[str, object]:
    """Read a whole message as plain values, keyed by field name.

    A scalar field comes back as a number, an array field as a list, and a
    ``char`` array as the text it carries, so a received message reads like an
    ordinary mapping.

    :param built: The message to read.
    :param shape: The shape it was built from, which names its fields.
    :returns: The fields as plain values.

    >>> status = schema_for("STATUSTEXT")
    >>> written = from_dict(status, {"severity": 4, "text": "battery low"})
    >>> to_dict(written, status)["text"]
    'battery low'
    """
    values: dict[str, object] = {}
    for field in shape.fields:
        if field.array_len == 0:
            values[field.name] = built.get(field.name)
        elif field.field_type == FieldType.CHAR:
            values[field.name] = built.get_text(field.name)
        else:
            values[field.name] = [
                built.get(field.name, index) for index in range(field.array_len)
            ]
    return values


def from_dict(shape: MessageSchema, values: dict[str, object]) -> MavlinkMessage:
    """Build a message from plain values, keyed by field name.

    A field left out stays zero, which is what a sender filling in part of a
    message wants.

    :param shape: The shape to build.
    :param values: The fields to set.
    :returns: The message.
    :raises ValueError: If a name is not a field of the message, or a value does
        not fit its field.

    >>> position = schema_for("GLOBAL_POSITION_INT")
    >>> report = from_dict(position, {"lat": -33856780, "lon": 151215300})
    >>> report.get_int("lat")
    -33856780
    """
    built = MavlinkMessage.empty(shape)
    for name, value in values.items():
        if isinstance(value, str):
            built.set_text(name, value)
        elif isinstance(value, (bytes, bytearray)):
            built.set_bytes(name, bytes(value))
        elif isinstance(value, (list, tuple)):
            for index, element in enumerate(value):
                built.set(name, float(element), index)
        else:
            built.set(name, float(value))
    return built


class TypeMask(IntFlag):
    """The fields of a setpoint the autopilot should act on.

    Combine members with ``|``; the fields left out are ignored.
    """

    POSITION = 1
    VELOCITY = 2
    ACCELERATION = 4
    YAW = 8
    YAW_RATE = 16
    FORCE = 32


def type_mask(fields: TypeMask | int) -> int:
    """Build a setpoint ``type_mask`` from the fields to use.

    :param fields: The fields the autopilot should act on.
    :returns: The mask, as the ``type_mask`` field of a setpoint carries it.

    >>> type_mask(TypeMask.VELOCITY | TypeMask.YAW_RATE) == mavlink_offboard_type_mask(2 | 16)
    True
    """
    return mavlink_offboard_type_mask(int(fields))


def local_position(
    header: MavlinkHeader,
    time_boot_ms: int,
    coordinate_frame: int,
    target_system: int,
    target_component: int,
    x: float,
    y: float,
    z: float,
) -> MavlinkFrame:
    """Build a local-frame position setpoint, ready to send.

    :param header: The addressing fields to stamp on the frame.
    :param time_boot_ms: The sender's boot timestamp, in milliseconds.
    :param coordinate_frame: The ``MAV_FRAME`` of the setpoint.
    :param target_system: The target system id.
    :param target_component: The target component id.
    :param x: The position along x, in metres in the chosen frame.
    :param y: The position along y.
    :param z: The position along z.
    :returns: The ``SET_POSITION_TARGET_LOCAL_NED`` frame.

    >>> local_position(MavlinkHeader(255, 190), 1000, 1, 1, 1, 10.0, 0.0, -5.0).message_id
    84
    """
    return mavlink_offboard_local_position(
        header, time_boot_ms, coordinate_frame, target_system, target_component, x, y, z
    )


def local_velocity(
    header: MavlinkHeader,
    time_boot_ms: int,
    coordinate_frame: int,
    target_system: int,
    target_component: int,
    vx: float,
    vy: float,
    vz: float,
) -> MavlinkFrame:
    """Build a local-frame velocity setpoint, ready to send.

    :param header: The addressing fields to stamp on the frame.
    :param time_boot_ms: The sender's boot timestamp, in milliseconds.
    :param coordinate_frame: The ``MAV_FRAME`` of the setpoint.
    :param target_system: The target system id.
    :param target_component: The target component id.
    :param vx: The velocity along x, in metres per second in the chosen frame.
    :param vy: The velocity along y.
    :param vz: The velocity along z.
    :returns: The ``SET_POSITION_TARGET_LOCAL_NED`` frame.
    """
    return mavlink_offboard_local_velocity(
        header, time_boot_ms, coordinate_frame, target_system, target_component, vx, vy, vz
    )


def global_position(
    header: MavlinkHeader,
    time_boot_ms: int,
    coordinate_frame: int,
    target_system: int,
    target_component: int,
    lat_int: int,
    lon_int: int,
    alt: float,
) -> MavlinkFrame:
    """Build a global-frame position setpoint, ready to send.

    :param header: The addressing fields to stamp on the frame.
    :param time_boot_ms: The sender's boot timestamp, in milliseconds.
    :param coordinate_frame: The ``MAV_FRAME`` of the setpoint.
    :param target_system: The target system id.
    :param target_component: The target component id.
    :param lat_int: The latitude, in degrees times ten million.
    :param lon_int: The longitude, in degrees times ten million.
    :param alt: The altitude, in metres.
    :returns: The ``SET_POSITION_TARGET_GLOBAL_INT`` frame.

    >>> global_position(MavlinkHeader(255, 190), 1000, 6, 1, 1, -338567800, 1512153000, 50.0).message_id
    86
    """
    return mavlink_offboard_global_position(
        header,
        time_boot_ms,
        coordinate_frame,
        target_system,
        target_component,
        lat_int,
        lon_int,
        alt,
    )
