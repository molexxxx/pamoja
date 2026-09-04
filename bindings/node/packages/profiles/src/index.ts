/**
 * Profiles and robotics: A node instantiated by name with its policy and schedule, and the naming and encoding rules a robot's topics follow, with no ROS 2 or Zenoh installed.
 *
 * Installing this package installs `@pamoja/profile`, `@pamoja/ros2`, `@pamoja/zenoh`, and re-exports each under its own
 * name, so a name two of them share stays unambiguous.
 *
 * @packageDocumentation
 */

export * as profile from '@pamoja/profile'
export * as ros2 from '@pamoja/ros2'
export * as zenoh from '@pamoja/zenoh'
