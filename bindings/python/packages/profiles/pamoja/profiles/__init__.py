"""Profiles and robotics: A node written down as a JSON file, rules between nodes as another, and the naming and encoding rules a robot's topics follow, with no ROS 2 or Zenoh installed.

Installing this distribution installs ``pamoja.profile``, ``pamoja.kit``, ``pamoja.ros2``, ``pamoja.zenoh``, and re-exports each under its
own name, so a name two of them share stays unambiguous.
"""

from pamoja import profile, kit, ros2, zenoh

__all__ = ["profile", "kit", "ros2", "zenoh"]

