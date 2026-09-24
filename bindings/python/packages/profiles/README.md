# pamoja-profiles

A node instantiated by name with its policy and schedule, and the naming and encoding rules a robot's topics follow, with no ROS 2 or Zenoh installed.

One install for the 5 capabilities of this domain. Each is also its own
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
| [Rules](https://pamoja.molex.cloud/docs/guides/rules.html) | `pamoja.profile` | Rules between nodes as a file: a condition over one node's topic with hysteresis, and the actions that drive another node's actuator or publish, run by an engine off any link or judged reading by reading from any language |
| [Robot motion](https://pamoja.molex.cloud/docs/guides/motion.html) | `pamoja.kit` | Wheel speeds for differential, skid-steer, car-like, and mecanum chassis, a two-link arm and Denavit-Hartenberg forward kinematics, odometry, waypoint guidance, a safety gate, and servo, ESC, and encoder conversions |
| [ROS 2 rules](https://pamoja.molex.cloud/docs/guides/ros2.html) | `pamoja.ros2` | ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed |
| [Zenoh keys](https://pamoja.molex.cloud/docs/guides/zenoh.html) | `pamoja.zenoh` | Zenoh key expressions: validity, canonical form, matching, and whether two expressions share or cover keys |

The guides, with a worked Python example for each, are at [https://pamoja.molex.cloud/docs](https://pamoja.molex.cloud/docs/).

## License

MIT
