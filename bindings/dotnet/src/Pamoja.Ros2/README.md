# Pamoja.Ros2

ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

## Install

```sh
dotnet add package Pamoja.Ros2
```

```csharp
using Pamoja.Ros2;
```

This pulls in `Pamoja.Native`, the compiled engine. `dotnet add package Pamoja` is the whole framework in one package.

## Example

The guide project's example, spliced here as it ran in CI.

From [`bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs):

```csharp
// A name is slash-separated tokens. A token may hold letters, digits, and
// underscores, and may not begin with a digit, which is the rule that catches most
// generated names.
Expect(Ros2.IsValidName("/robot1/camera_left/image_raw"), "a well-formed name");
Expect(!Ros2.IsValidName("/2foo"), "a token may not start with a digit");
Expect(Ros2.IsFullyQualified("/chatter"), "a leading slash qualifies a name");
Expect(!Ros2.IsFullyQualified("chatter"), "and without one it is relative");

// On the wire a topic carries a prefix that says what kind of endpoint it is, so a
// subscription and a service request never collide in the same DDS partition.
Expect(
    Ros2.DdsTopic("/robot1/cmd_vel", EntityKind.Topic) == "rt/robot1/cmd_vel",
    "a topic is published under rt");
Expect(
    Ros2.DdsTopic("/robot1/add", EntityKind.ServiceRequest) == "rq/robot1/add",
    "a service request under rq");
Expect(
    Ros2.DdsTopic("/robot1/add", EntityKind.ServiceResponse) == "rr/robot1/add",
    "and its response under rr");

// A message type maps to a DDS type name the same way, so both ends agree on what
// is being carried before a byte is exchanged.
Expect(
    Ros2.DdsTypeName("std_msgs/msg/String") == "std_msgs::msg::dds_::String_",
    "the DDS type name the graph expects");
Expect(Ros2.DdsTypeName("not a type") is null, "and a malformed type has none");
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-ros2`](https://crates.io/crates/pamoja-ros2) | [docs.rs](https://docs.rs/pamoja-ros2), [site](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html) |
| TypeScript | [`@pamoja/ros2`](https://www.npmjs.com/package/@pamoja/ros2) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html) |
| Python | [`pamoja-ros2`](https://pypi.org/project/pamoja-ros2/) | [`pamoja.ros2`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html) |
| C# | [`Pamoja.Ros2`](https://www.nuget.org/packages/Pamoja.Ros2) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.Ros2.html) |

## Documentation

- [The ROS 2 rules guide](https://pamoja.molex.cloud/docs/guides/ros2.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
