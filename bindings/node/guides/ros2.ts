// The ROS 2 naming guide example; see docs/guides/ros2.md.

import assert from 'node:assert/strict'

// ANCHOR: example
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
// ANCHOR_END: example

assert.equal(published, 'rt/robot1/cmd_vel')
assert.equal(asked, 'rq/robot1/add_two_intsRequest')
assert.equal(answered, 'rr/robot1/add_two_intsReply')

// ANCHOR: zenoh
import { typeHash } from '@pamoja/ros2'

// A type hash pins the message definition itself, so two builds of a message that share a
// name but not a layout never talk. This is the hash rosidl publishes for
// std_msgs/msg/String: RIHS01, then a SHA-256 in hex.
const STRING_HASH = 'RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18'
const digest = typeHash.digest(STRING_HASH)
console.log(`hash      std_msgs/msg/String carries RIHS01 with a ${digest?.length}-byte SHA-256`)

// rmw_zenoh puts a topic on the key <domain>/<name>/<type>/<hash>, so a peer that builds the
// same key talks to ROS 2 nodes with no DDS in the path.
const key = typeHash.entityKey(0, '/chatter', 'std_msgs/msg/String', STRING_HASH)
console.log(`key       ${key}`)
if (typeHash.entityKey(0, 'chatter', 'std_msgs/msg/String', STRING_HASH) === null) {
  console.log('no key    a relative name has no key until the node resolves it')
}

// A hash one digit short is not a hash, and is refused rather than matched loosely.
if (typeHash.digest(STRING_HASH.slice(0, -1)) === null) {
  console.log('refused   a hash one hex digit short is not RIHS01')
}

// A liveliness token says who is on the graph, and in it rmw_zenoh writes each slash of a
// name as a percent sign.
console.log(
  `token     /robot1/chatter is written ${name.percentMangle('/robot1/chatter')} in a liveliness token`,
)
// ANCHOR_END: zenoh

assert.equal(key, `0/chatter/std_msgs::msg::dds_::String_/${STRING_HASH}`)

// ANCHOR: cdr
import { cdr } from '@pamoja/ros2'

// A command velocity: half a meter a second forward, turning at 0.2 radians a second. CDR
// opens with a four-byte header naming the byte order, then the six doubles.
const command = { linear: { x: 0.5, y: 0, z: 0 }, angular: { x: 0, y: 0, z: 0.2 } }
const bytes = cdr.twistToBytes(command)
console.log(`twist     ${bytes.length} bytes: a 4-byte header, then six 8-byte doubles`)
const back = cdr.twistFromBytes(bytes)!
console.log(
  `decoded   forward ${back.linear.x} m/s, turning ${back.angular.z} rad/s, the command that was sent`,
)

// Each value is aligned to its own size, counted from after the header, so an integer
// followed by a double takes four bytes of padding before the double.
const writer = cdr.writer()
writer.writeI32(7)
writer.writeF64(2.5)
const mixed = writer.bytes
const reader = cdr.reader(mixed)
const count = reader.readI32()
const level = reader.readF64()
console.log(
  `aligned   an i32 then an f64 take ${mixed.length} bytes, not 16, and read back as ${count} and ${level}`,
)

// A message cut short decodes as nothing rather than as a plausible command.
if (cdr.twistFromBytes(bytes.subarray(0, bytes.length - 8)) === null) {
  console.log('short     a twist missing its last double decodes as nothing')
}
// ANCHOR_END: cdr

assert.deepEqual(back, command)
