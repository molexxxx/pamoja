# @pamoja/ros2

ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/ros2.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html)

## Install

```sh
npm install @pamoja/ros2
```

This pulls in `@pamoja/native`, the compiled engine. `npm install pamoja` is the whole framework in one package.

## Example

The test that runs in CI, spliced here as it ran.

From [`bindings/node/guides/ros2.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ros2.ts):

```typescript
import { EntityKind, name } from '@pamoja/ros2'

// A name is slash-separated tokens of letters, digits, and underscores. A token may not start
// with a digit, and a name may not end in a slash, hold an empty token, or double an
// underscore.
const camera = '/robot1/camera_left/image_raw'
if (name.isValid(camera)) {
  console.log(`valid     ${camera}`)
}
for (const [candidate, why] of [
  ['/2foo', 'a token starts with a digit'],
  ['/cmd_vel/', 'it ends in a slash'],
  ['/robot1//odom', 'it has an empty token'],
  ['/robot1/cmd__vel', 'it doubles an underscore'],
]) {
  if (!name.isValid(candidate)) {
    console.log(`invalid   ${candidate}, since ${why}`)
  }
}

// A name with no leading slash is relative, and one that starts with a tilde is private. Both
// are valid, and both resolve against the node before they reach the wire, so neither is
// fully qualified.
for (const [label, candidate, against] of [
  ['relative', 'cmd_vel', "the node's namespace"],
  ['private', '~/setpoint', "the node's own name"],
]) {
  if (name.isValid(candidate) && !name.isFullyQualified(candidate)) {
    console.log(`${label.padEnd(10)}${candidate} is valid, and resolves against ${against} first`)
  }
}

// Only a fully qualified name reaches the wire. DDS puts a prefix before it that says what
// kind of endpoint it is, and a service travels on two topics, each ending in the suffix the
// middleware appends.
const published = name.ddsTopic('/robot1/cmd_vel', EntityKind.Topic)
const asked = name.ddsTopic('/robot1/add_two_ints', EntityKind.ServiceRequest)
const answered = name.ddsTopic('/robot1/add_two_ints', EntityKind.ServiceResponse)
console.log(`topic     /robot1/cmd_vel travels on ${published}`)
console.log(`request   /robot1/add_two_ints asks on ${asked}`)
console.log(`reply     and answers on ${answered}`)

// A message type maps to a DDS type name the same way, so both ends agree on what is carried
// before a byte is exchanged. A name that is not package/namespace/Type maps to nothing
// rather than to something plausible.
for (const rosType of ['std_msgs/msg/String', 'example_interfaces/srv/AddTwoInts', 'std_msgs/String']) {
  const carried = name.ddsTypeName(rosType)
  if (carried !== null) {
    console.log(`type      ${rosType} is named ${carried}`)
  } else {
    console.log(`malformed ${rosType} is not package/namespace/Type, so it has no DDS type name`)
  }
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

- [`@pamoja/ros2` reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html), every class, function, and type this package exports.
- [The ROS 2 rules guide](https://pamoja.molex.cloud/docs/guides/ros2.html), with the same example in Rust, Python, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
