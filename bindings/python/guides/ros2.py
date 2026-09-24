"""The ROS 2 naming guide example; see docs/guides/ros2.md."""

# ANCHOR: example
from pamoja.ros2 import EntityKind, dds_topic, dds_type_name, is_fully_qualified, is_valid_name

# A name is slash-separated tokens of letters, digits, and underscores. A token may not start
# with a digit, and a name may not end in a slash, hold an empty token, or double an
# underscore.
camera = "/robot1/camera_left/image_raw"
if is_valid_name(camera):
    print(f"valid     {camera}")
for name, why in [
    ("/2foo", "a token starts with a digit"),
    ("/cmd_vel/", "it ends in a slash"),
    ("/robot1//odom", "it has an empty token"),
    ("/robot1/cmd__vel", "it doubles an underscore"),
]:
    if not is_valid_name(name):
        print(f"invalid   {name}, since {why}")

# A name with no leading slash is relative, and one that starts with a tilde is private. Both
# are valid, and both resolve against the node before they reach the wire, so neither is
# fully qualified.
for label, name, against in [
    ("relative", "cmd_vel", "the node's namespace"),
    ("private", "~/setpoint", "the node's own name"),
]:
    if is_valid_name(name) and not is_fully_qualified(name):
        print(f"{label:<10}{name} is valid, and resolves against {against} first")

# Only a fully qualified name reaches the wire. DDS puts a prefix before it that says what
# kind of endpoint it is, and a service travels on two topics, each ending in the suffix the
# middleware appends.
published = dds_topic("/robot1/cmd_vel", EntityKind.TOPIC)
asked = dds_topic("/robot1/add_two_ints", EntityKind.SERVICE_REQUEST)
answered = dds_topic("/robot1/add_two_ints", EntityKind.SERVICE_RESPONSE)
print(f"topic     /robot1/cmd_vel travels on {published}")
print(f"request   /robot1/add_two_ints asks on {asked}")
print(f"reply     and answers on {answered}")

# A message type maps to a DDS type name the same way, so both ends agree on what is carried
# before a byte is exchanged. A name that is not package/namespace/Type maps to nothing rather
# than to something plausible.
for ros_type in ["std_msgs/msg/String", "example_interfaces/srv/AddTwoInts", "std_msgs/String"]:
    carried = dds_type_name(ros_type)
    if carried is not None:
        print(f"type      {ros_type} is named {carried}")
    else:
        print(f"malformed {ros_type} is not package/namespace/Type, so it has no DDS type name")
# ANCHOR_END: example

assert published == "rt/robot1/cmd_vel"
assert asked == "rq/robot1/add_two_intsRequest"
assert answered == "rr/robot1/add_two_intsReply"

# ANCHOR: zenoh
from pamoja.ros2 import entity_key, percent_mangle, type_hash_digest

# A type hash pins the message definition itself, so two builds of a message that share a
# name but not a layout never talk. This is the hash rosidl publishes for std_msgs/msg/String:
# RIHS01, then a SHA-256 in hex.
STRING_HASH = "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18"
digest = type_hash_digest(STRING_HASH)
print(f"hash      std_msgs/msg/String carries RIHS01 with a {len(digest)}-byte SHA-256")

# rmw_zenoh puts a topic on the key <domain>/<name>/<type>/<hash>, so a peer that builds the
# same key talks to ROS 2 nodes with no DDS in the path.
key = entity_key(0, "/chatter", "std_msgs/msg/String", STRING_HASH)
print(f"key       {key}")
if entity_key(0, "chatter", "std_msgs/msg/String", STRING_HASH) is None:
    print("no key    a relative name has no key until the node resolves it")

# A hash one digit short is not a hash, and is refused rather than matched loosely.
if type_hash_digest(STRING_HASH[:-1]) is None:
    print("refused   a hash one hex digit short is not RIHS01")

# A liveliness token says who is on the graph, and in it rmw_zenoh writes each slash of a name
# as a percent sign.
print(
    f"token     /robot1/chatter is written {percent_mangle('/robot1/chatter')} "
    "in a liveliness token"
)
# ANCHOR_END: zenoh

assert key == f"0/chatter/std_msgs::msg::dds_::String_/{STRING_HASH}"

# ANCHOR: cdr
from pamoja.ros2 import CdrReader, CdrWriter, twist_from_cdr, twist_to_cdr

# A command velocity: half a meter a second forward, turning at 0.2 radians a second. CDR
# opens with a four-byte header naming the byte order, then the six doubles.
linear, angular = (0.5, 0.0, 0.0), (0.0, 0.0, 0.2)
data = twist_to_cdr(linear, angular)
print(f"twist     {len(data)} bytes: a 4-byte header, then six 8-byte doubles")
back_linear, back_angular = twist_from_cdr(data)
print(
    f"decoded   forward {back_linear[0]:g} m/s, turning {back_angular[2]:g} rad/s, "
    "the command that was sent"
)

# Each value is aligned to its own size, counted from after the header, so an integer
# followed by a double takes four bytes of padding before the double.
writer = CdrWriter()
writer.write_i32(7)
writer.write_f64(2.5)
mixed = writer.bytes
reader = CdrReader(mixed)
count = reader.read_i32()
level = reader.read_f64()
print(
    f"aligned   an i32 then an f64 take {len(mixed)} bytes, not 16, "
    f"and read back as {count} and {level:g}"
)

# A message cut short decodes as nothing rather than as a plausible command.
if twist_from_cdr(data[:-8]) is None:
    print("short     a twist missing its last double decodes as nothing")
# ANCHOR_END: cdr

assert (back_linear, back_angular) == (linear, angular)
