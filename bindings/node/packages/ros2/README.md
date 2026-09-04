# @pamoja/ros2

ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/ros2.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
npm install @pamoja/ros2
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/ros2.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ros2.ts):

```typescript
import assert from 'node:assert/strict'

import { EntityKind, name } from '@pamoja/ros2'

// A name is slash-separated tokens. A token may hold letters, digits, and underscores,
// and may not begin with a digit, which is the rule that catches most generated names.
assert.ok(name.isValid('/robot1/camera_left/image_raw'))
assert.ok(!name.isValid('/2foo'))
assert.ok(name.isFullyQualified('/chatter'))
assert.ok(!name.isFullyQualified('chatter'))

// On the wire a topic carries a prefix that says what kind of endpoint it is, so a
// subscription and a service request never collide in the same DDS partition.
assert.equal(name.ddsTopic('/robot1/cmd_vel', EntityKind.Topic), 'rt/robot1/cmd_vel')
assert.equal(name.ddsTopic('/robot1/add', EntityKind.ServiceRequest), 'rq/robot1/add')
assert.equal(name.ddsTopic('/robot1/add', EntityKind.ServiceResponse), 'rr/robot1/add')

// A message type maps to a DDS type name the same way, so both ends agree on what is
// being carried before a byte is exchanged.
assert.equal(name.ddsTypeName('std_msgs/msg/String'), 'std_msgs::msg::dds_::String_')
assert.equal(name.ddsTypeName('not a type'), null)
```

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-ros2`](https://crates.io/crates/pamoja-ros2) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html), [docs.rs](https://docs.rs/pamoja-ros2) |
| TypeScript | [`@pamoja/ros2`](https://www.npmjs.com/package/@pamoja/ros2) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html) |
| Python | [`pamoja-ros2`](https://pypi.org/project/pamoja-ros2/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html) |
| C# | [`Pamoja.Ros2`](https://www.nuget.org/packages/Pamoja.Ros2) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html) |

## Documentation

- [`@pamoja/ros2` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html), every class, function, and type this package exports.
- [The ROS 2 rules guide](https://pamoja.molex.cloud/docs/guides/ros2.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
