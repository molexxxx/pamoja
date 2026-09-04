"""The ROS 2 rules guide example; see docs/guides/ros2.md."""

# ANCHOR: example
from pamoja.ros2 import (
    EntityKind, dds_topic, dds_type_name, is_fully_qualified, is_valid_name,
)

# A name is slash-separated tokens. A token may hold letters, digits, and underscores,
# and may not begin with a digit, which is the rule that catches most generated names.
assert is_valid_name("/robot1/camera_left/image_raw")
assert not is_valid_name("/2foo")
assert is_fully_qualified("/chatter")
assert not is_fully_qualified("chatter")

# On the wire a topic carries a prefix that says what kind of endpoint it is, so a
# subscription and a service request never collide in the same DDS partition.
assert dds_topic("/robot1/cmd_vel", EntityKind.TOPIC) == "rt/robot1/cmd_vel"
assert dds_topic("/robot1/add", EntityKind.SERVICE_REQUEST) == "rq/robot1/add"
assert dds_topic("/robot1/add", EntityKind.SERVICE_RESPONSE) == "rr/robot1/add"

# A message type maps to a DDS type name the same way, so both ends agree on what is
# being carried before a byte is exchanged.
assert dds_type_name("std_msgs/msg/String") == "std_msgs::msg::dds_::String_"
assert dds_type_name("not a type") is None
# ANCHOR_END: example
