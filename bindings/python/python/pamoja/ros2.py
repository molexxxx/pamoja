"""Idiomatic ROS 2 naming and encoding facade.

What makes a topic name legal, what it becomes on the DDS wire, the RIHS type
hash that identifies a message definition, and the CDR encoding the payload
itself is written in.

None of it needs a ROS 2 installation. A gateway written in Python can validate
a name, derive the DDS topic and the Zenoh key an ``rmw_zenoh`` peer subscribes
on, and encode a ``geometry_msgs/msg/Twist`` with no ROS distribution anywhere
near it. Driving a live graph does need one, so that stays in the Rust crate.
"""

from __future__ import annotations

import enum

from ._core import (
    CdrReader,
    CdrWriter,
    ros2_dds_topic as dds_topic,
    ros2_dds_type_name as dds_type_name,
    ros2_entity_key as entity_key,
    ros2_entity_kind_prefix as prefix_for,
    ros2_is_fully_qualified as is_fully_qualified,
    ros2_is_valid_name as is_valid_name,
    ros2_percent_mangle as percent_mangle,
    ros2_twist_from_cdr as twist_from_cdr,
    ros2_twist_to_cdr as twist_to_cdr,
    ros2_type_hash_digest as type_hash_digest,
)

__all__ = [
    "CdrReader",
    "CdrWriter",
    "EntityKind",
    "dds_topic",
    "dds_type_name",
    "entity_key",
    "is_fully_qualified",
    "is_valid_name",
    "percent_mangle",
    "prefix_for",
    "twist_from_cdr",
    "twist_to_cdr",
    "type_hash_digest",
]


class EntityKind(str, enum.Enum):
    """The ROS 2 subsystem a name belongs to, which fixes its DDS prefix."""

    #: A topic, which takes the ``rt`` prefix.
    TOPIC = "Topic"
    #: The request side of a service, which takes the ``rq`` prefix.
    SERVICE_REQUEST = "ServiceRequest"
    #: The reply side of a service, which takes the ``rr`` prefix.
    SERVICE_RESPONSE = "ServiceResponse"
