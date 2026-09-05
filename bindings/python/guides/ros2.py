"""The ROS 2 naming guide example; see docs/guides/ros2.md."""

# ANCHOR: example
from pamoja.ros2 import EntityKind, dds_topic, dds_type_name, is_fully_qualified, is_valid_name

# A name is slash-separated tokens. A token may hold letters, digits, and underscores, and
# may not begin with a digit, which is the rule that catches most generated names.
for name in ("/robot1/camera_left/image_raw", "/2foo"):
    print(f"{name} is a valid name: {is_valid_name(name)}")
print(f"chatter is fully qualified: {is_fully_qualified('chatter')}")
print(f"/chatter is fully qualified: {is_fully_qualified('/chatter')}")

# On the wire a topic carries a prefix that says what kind of endpoint it is, so a
# subscription and a service request never collide in the same DDS partition.
published = dds_topic("/robot1/cmd_vel", EntityKind.TOPIC)
asked = dds_topic("/robot1/add", EntityKind.SERVICE_REQUEST)
answered = dds_topic("/robot1/add", EntityKind.SERVICE_RESPONSE)
print(f"a topic    becomes {published}")
print(f"a request  becomes {asked}")
print(f"a response becomes {answered}")

# A message type maps to a DDS type name the same way, so both ends agree on what is being
# carried before a byte is exchanged.
print(f"std_msgs/msg/String becomes {dds_type_name('std_msgs/msg/String')}")
print(f"a malformed type name becomes {dds_type_name('not a type')}")
# ANCHOR_END: example

assert is_valid_name("/robot1/camera_left/image_raw")
assert not is_valid_name("/2foo")
assert is_fully_qualified("/chatter")
assert not is_fully_qualified("chatter")
assert published == "rt/robot1/cmd_vel"
assert asked == "rq/robot1/add"
assert answered == "rr/robot1/add"
assert dds_type_name("std_msgs/msg/String") == "std_msgs::msg::dds_::String_"
assert dds_type_name("not a type") is None
