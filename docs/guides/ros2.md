# ROS 2 rules

A ROS 2 graph agrees on names before it exchanges anything. A node's topic has a
name, that name maps to a DDS topic with a prefix saying what kind of endpoint it
is, and the message type maps to a DDS type name both ends must derive
identically. pamoja implements those rules and nothing else, so a bridge or a
tool can speak the graph's language with no ROS 2 installation anywhere near it.

## What the example does

It checks a node name against the token rules, maps a topic and both halves of a
service to the DDS names they carry on the wire, and maps a message type to its
DDS type name.

It proves:

- A name is slash-separated tokens that may hold letters, digits, and
  underscores and may not begin with a digit, which is the rule that catches
  most generated names.
- A leading slash is what makes a name fully qualified.
- A topic goes out under `rt`, a service request under `rq`, and its response
  under `rr`, so the three never collide in one DDS partition.
- A message type maps to the DDS type name the graph expects, and a malformed
  type has none rather than a plausible-looking one.

## Rust

<!-- snippet: examples/tests/guides/ros2.rs#example -->
From [`examples/tests/guides/ros2.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/ros2.rs):

```rust
use pamoja_ros2::name::{dds_topic, is_fully_qualified, is_valid_name, EntityKind};
use pamoja_ros2::typehash::dds_type_name;

// A name is slash-separated tokens. A token may hold letters, digits, and underscores,
// and may not begin with a digit, which is the rule that catches most generated names.
assert!(is_valid_name("/robot1/camera_left/image_raw"));
assert!(!is_valid_name("/2foo"));
assert!(is_fully_qualified("/chatter"));
assert!(!is_fully_qualified("chatter"));

// On the wire a topic carries a prefix that says what kind of endpoint it is, so a
// subscription and a service request never collide in the same DDS partition.
assert_eq!(
    dds_topic("/robot1/cmd_vel", EntityKind::Topic).as_deref(),
    Some("rt/robot1/cmd_vel")
);
assert_eq!(
    dds_topic("/robot1/add", EntityKind::ServiceRequest).as_deref(),
    Some("rq/robot1/add")
);
assert_eq!(
    dds_topic("/robot1/add", EntityKind::ServiceResponse).as_deref(),
    Some("rr/robot1/add")
);

// A message type maps to a DDS type name the same way, so both ends agree on what is
// being carried before a byte is exchanged.
assert_eq!(
    dds_type_name("std_msgs/msg/String").as_deref(),
    Some("std_msgs::msg::dds_::String_")
);
assert_eq!(dds_type_name("not a type"), None);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/ros2.ts#example -->
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
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/ros2.py#example -->
From [`bindings/python/guides/ros2.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ros2.py):

```python
from pamoja.ros2 import (
    EntityKind, dds_topic, dds_type_name, is_fully_qualified, is_valid_name,
)

# A name is slash-separated tokens. A token may hold letters, digits, and underscores,
# and may not begin with a digit, which is the rule that catches most generated names.
assert is_valid_name("/robot1/camera_left/image_raw")
assert not is_valid_name("/2foo")
assert is_fully_qualified("/chatter")
assert not is_fully_qualified("chatter")

# On the wire a topic carries a prefix that says what kind of endpoint it is, so a
# subscription and a service request never collide in the same DDS partition.
assert dds_topic("/robot1/cmd_vel", EntityKind.TOPIC) == "rt/robot1/cmd_vel"
assert dds_topic("/robot1/add", EntityKind.SERVICE_REQUEST) == "rq/robot1/add"
assert dds_topic("/robot1/add", EntityKind.SERVICE_RESPONSE) == "rr/robot1/add"

# A message type maps to a DDS type name the same way, so both ends agree on what is
# being carried before a byte is exchanged.
assert dds_type_name("std_msgs/msg/String") == "std_msgs::msg::dds_::String_"
assert dds_type_name("not a type") is None
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs):

```csharp
// A name is slash-separated tokens. A token may hold letters, digits, and
// underscores, and may not begin with a digit, which is the rule that catches most
// generated names.
Expect(Ros2.IsValidName("/robot1/camera_left/image_raw"), "a well-formed name");
Expect(!Ros2.IsValidName("/2foo"), "a token may not start with a digit");
Expect(Ros2.IsFullyQualified("/chatter"), "a leading slash qualifies a name");
Expect(!Ros2.IsFullyQualified("chatter"), "and without one it is relative");

// On the wire a topic carries a prefix that says what kind of endpoint it is, so a
// subscription and a service request never collide in the same DDS partition.
Expect(
    Ros2.DdsTopic("/robot1/cmd_vel", EntityKind.Topic) == "rt/robot1/cmd_vel",
    "a topic is published under rt");
Expect(
    Ros2.DdsTopic("/robot1/add", EntityKind.ServiceRequest) == "rq/robot1/add",
    "a service request under rq");
Expect(
    Ros2.DdsTopic("/robot1/add", EntityKind.ServiceResponse) == "rr/robot1/add",
    "and its response under rr");

// A message type maps to a DDS type name the same way, so both ends agree on what
// is being carried before a byte is exchanged.
Expect(
    Ros2.DdsTypeName("std_msgs/msg/String") == "std_msgs::msg::dds_::String_",
    "the DDS type name the graph expects");
Expect(Ros2.DdsTypeName("not a type") is null, "and a malformed type has none");
```
<!-- end -->

## Reference

<!-- table: reference ros2 -->
- Rust: [`pamoja-ros2`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html)
- TypeScript: [`@pamoja/ros2`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html)
- Python: [`pamoja.ros2`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html)
- C#: [`Pamoja.Ros2`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html)
<!-- end -->
