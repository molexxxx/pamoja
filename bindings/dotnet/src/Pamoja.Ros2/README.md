# Pamoja.Ros2

ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/ros2.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html)

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
// A name is slash-separated tokens of letters, digits, and underscores. A token may
// not start with a digit, and a name may not end in a slash, hold an empty token, or
// double an underscore.
const string camera = "/robot1/camera_left/image_raw";
if (Ros2.IsValidName(camera))
{
    Console.WriteLine($"valid     {camera}");
}

foreach ((string name, string why) in new[]
{
    ("/2foo", "a token starts with a digit"),
    ("/cmd_vel/", "it ends in a slash"),
    ("/robot1//odom", "it has an empty token"),
    ("/robot1/cmd__vel", "it doubles an underscore"),
})
{
    if (!Ros2.IsValidName(name))
    {
        Console.WriteLine($"invalid   {name}, since {why}");
    }
}

// A name with no leading slash is relative, and one that starts with a tilde is
// private. Both are valid, and both resolve against the node before they reach the
// wire, so neither is fully qualified.
foreach ((string label, string name, string against) in new[]
{
    ("relative", "cmd_vel", "the node's namespace"),
    ("private", "~/setpoint", "the node's own name"),
})
{
    if (Ros2.IsValidName(name) && !Ros2.IsFullyQualified(name))
    {
        Console.WriteLine($"{label,-10}{name} is valid, and resolves against {against} first");
    }
}

// Only a fully qualified name reaches the wire. DDS puts a prefix before it that says
// what kind of endpoint it is, and a service travels on two topics, each ending in the
// suffix the middleware appends.
string? published = Ros2.DdsTopic("/robot1/cmd_vel", EntityKind.Topic);
string? asked = Ros2.DdsTopic("/robot1/add_two_ints", EntityKind.ServiceRequest);
string? answered = Ros2.DdsTopic("/robot1/add_two_ints", EntityKind.ServiceResponse);
Console.WriteLine($"topic     /robot1/cmd_vel travels on {published}");
Console.WriteLine($"request   /robot1/add_two_ints asks on {asked}");
Console.WriteLine($"reply     and answers on {answered}");

// A message type maps to a DDS type name the same way, so both ends agree on what is
// carried before a byte is exchanged. A name that is not package/namespace/Type maps
// to nothing rather than to something plausible.
foreach (string rosType in new[]
{
    "std_msgs/msg/String",
    "example_interfaces/srv/AddTwoInts",
    "std_msgs/String",
})
{
    Console.WriteLine(Ros2.DdsTypeName(rosType) is { } carried
        ? $"type      {rosType} is named {carried}"
        : $"malformed {rosType} is not package/namespace/Type, so it has no DDS type name");
}
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-ros2`](https://crates.io/crates/pamoja-ros2) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html), [docs.rs](https://docs.rs/pamoja-ros2), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-ros2) |
| TypeScript | [`@pamoja/ros2`](https://www.npmjs.com/package/@pamoja/ros2) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-ros2) |
| Python | [`pamoja-ros2`](https://pypi.org/project/pamoja-ros2/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-ros2) |
| C# | [`Pamoja.Ros2`](https://www.nuget.org/packages/Pamoja.Ros2) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-ros2) |

## Documentation

- [`Pamoja.Ros2` reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html), every type in this namespace.
- [The ROS 2 rules guide](https://pamoja.molex.cloud/docs/guides/ros2.html), with the same example in Rust, TypeScript, and Python.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
