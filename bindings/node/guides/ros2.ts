// The ROS 2 rules guide example; see docs/guides/ros2.md.

// ANCHOR: example
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
// ANCHOR_END: example
