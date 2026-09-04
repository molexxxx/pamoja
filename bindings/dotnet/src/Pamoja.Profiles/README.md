# Pamoja.Profiles

A node instantiated by name with its policy and schedule, and the naming and encoding rules a robot's topics follow, with no ROS 2 or Zenoh installed.

One reference for the 3 capabilities of this domain. Each is also its own package,
and `Pamoja` is the whole framework in one.

```sh
dotnet add package Pamoja.Profiles
```

This package ships no assembly: it brings in the packages below, and each keeps its own
namespace, so a type is named the way it is when the package is referenced directly.

| Capability | Package | What it covers |
| --- | --- | --- |
| [Device profiles](https://pamoja.molex.cloud/docs/guides/profile.html) | `Pamoja.Profile` | Named, ready-to-run device profiles from plain data or a JSON manifest |
| [ROS 2 rules](https://pamoja.molex.cloud/docs/guides/ros2.html) | `Pamoja.Ros2` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
| [Zenoh keys](https://pamoja.molex.cloud/docs/guides/zenoh.html) | `Pamoja.Zenoh` | Zenoh key expressions: validity, canonical form, and wildcard matching |

The guides, with a worked C# example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
