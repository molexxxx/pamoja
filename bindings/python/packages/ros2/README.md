# pamoja-ros2

ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
pip install pamoja-ros2
```

```python
from pamoja import ros2
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

From [`bindings/python/guides/ros2.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ros2.py):

```python
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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-ros2`](https://crates.io/crates/pamoja-ros2) | [docs.rs](https://docs.rs/pamoja-ros2), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html) |
| TypeScript | [`@pamoja/ros2`](https://www.npmjs.com/package/@pamoja/ros2) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html) |
| Python | [`pamoja-ros2`](https://pypi.org/project/pamoja-ros2/) | [`pamoja.ros2`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html) |
| C# | [`Pamoja.Ros2`](https://www.nuget.org/packages/Pamoja.Ros2) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.Ros2.html) |

## Documentation

- [The ROS 2 rules guide](https://pamoja.molex.cloud/docs/guides/ros2.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
