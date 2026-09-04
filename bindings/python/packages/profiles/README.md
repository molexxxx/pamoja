# pamoja-profiles

A node instantiated by name with its policy and schedule, and the naming and encoding rules a robot's topics follow, with no ROS 2 or Zenoh installed.

One install for the 3 capabilities of this domain. Each is also its own
distribution, and `pamoja` is the whole framework in one.

```sh
pip install pamoja-profiles
```

```python
from pamoja.profiles import profile
```

| Capability | Module | What it covers |
| --- | --- | --- |
| [Device profiles](https://pamoja.molex.cloud/docs/guides/profile.html) | `pamoja.profile` | Named, ready-to-run device profiles from plain data or a JSON manifest |
| [ROS 2 rules](https://pamoja.molex.cloud/docs/guides/ros2.html) | `pamoja.ros2` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
| [Zenoh keys](https://pamoja.molex.cloud/docs/guides/zenoh.html) | `pamoja.zenoh` | Zenoh key expressions: validity, canonical form, and wildcard matching |

The guides, with a worked Python example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
