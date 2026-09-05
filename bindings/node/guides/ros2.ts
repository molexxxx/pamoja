// The ROS 2 naming guide example; see docs/guides/ros2.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { EntityKind, name } from '@pamoja/ros2'

// A name is slash-separated tokens. A token may hold letters, digits, and underscores, and
// may not begin with a digit, which is the rule that catches most generated names.
for (const candidate of ['/robot1/camera_left/image_raw', '/2foo']) {
  console.log(`${candidate} is a valid name: ${name.isValid(candidate)}`)
}
console.log(`chatter is fully qualified: ${name.isFullyQualified('chatter')}`)
console.log(`/chatter is fully qualified: ${name.isFullyQualified('/chatter')}`)

// On the wire a topic carries a prefix that says what kind of endpoint it is, so a
// subscription and a service request never collide in the same DDS partition.
const published = name.ddsTopic('/robot1/cmd_vel', EntityKind.Topic)
const asked = name.ddsTopic('/robot1/add', EntityKind.ServiceRequest)
const answered = name.ddsTopic('/robot1/add', EntityKind.ServiceResponse)
console.log(`a topic    becomes ${published}`)
console.log(`a request  becomes ${asked}`)
console.log(`a response becomes ${answered}`)

// A message type maps to a DDS type name the same way, so both ends agree on what is being
// carried before a byte is exchanged.
console.log(`std_msgs/msg/String becomes ${name.ddsTypeName('std_msgs/msg/String')}`)
console.log(`a malformed type name becomes ${name.ddsTypeName('not a type')}`)
// ANCHOR_END: example

assert.ok(name.isValid('/robot1/camera_left/image_raw'))
assert.ok(!name.isValid('/2foo'))
assert.ok(name.isFullyQualified('/chatter'))
assert.ok(!name.isFullyQualified('chatter'))
assert.equal(published, 'rt/robot1/cmd_vel')
assert.equal(asked, 'rq/robot1/add')
assert.equal(answered, 'rr/robot1/add')
assert.equal(name.ddsTypeName('std_msgs/msg/String'), 'std_msgs::msg::dds_::String_')
assert.equal(name.ddsTypeName('not a type'), null)
