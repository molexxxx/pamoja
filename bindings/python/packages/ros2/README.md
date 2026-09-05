# pamoja-ros2

ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/ros2.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

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
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-ros2`](https://crates.io/crates/pamoja-ros2) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html), [docs.rs](https://docs.rs/pamoja-ros2) |
| TypeScript | [`@pamoja/ros2`](https://www.npmjs.com/package/@pamoja/ros2) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html) |
| Python | [`pamoja-ros2`](https://pypi.org/project/pamoja-ros2/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html) |
| C# | [`Pamoja.Ros2`](https://www.nuget.org/packages/Pamoja.Ros2) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html) |

## Documentation

- [`pamoja.ros2` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html), every class and function in this module.
- [The ROS 2 rules guide](https://pamoja.molex.cloud/docs/guides/ros2.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
