# ROS 2 rules

A ROS 2 graph agrees on names before it exchanges anything. A node's topic has a
name, that name maps to a DDS topic with a prefix saying what kind of endpoint it
is, and the message type maps to a DDS type name both ends must derive
identically. pamoja implements those rules and nothing else, so a bridge or a
tool can speak the graph's language with no ROS 2 installation anywhere near it.

## What the example does

It runs a camera topic and a malformed name past the token rules, asks whether a
name is fully qualified, maps a topic and both halves of a service to the names
they carry on the wire, then maps a message type to its DDS type name.

The DDS names are derived rather than written out. Each one comes from the ROS
name plus the kind of endpoint carrying it, so a reader sees which part of a
wire name the library supplies. The request and the response start from one
name, `/robot1/add`, and only the `EntityKind` separates them.

It proves:

- A token may hold letters, digits and underscores but may not begin with a
  digit, so `/2foo` is rejected where `/robot1/camera_left/image_raw` passes.
- A leading slash is what makes a name fully qualified, and `chatter` without
  one is relative.
- A topic goes out under `rt`, a service request under `rq` and its response
  under `rr`, so a request and its response keep one ROS name and still never
  collide in a DDS partition.
- `std_msgs/msg/String` becomes `std_msgs::msg::dds_::String_`, the `dds_`
  namespace and the trailing underscore included, since a peer matches the whole
  string.
- A malformed type name maps to nothing rather than to something that looks
  plausible.

## Rust

<!-- snippet: examples/tests/guides/ros2.rs#example -->
From [`examples/tests/guides/ros2.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/ros2.rs):

```rust
use pamoja_ros2::name::{dds_topic, is_fully_qualified, is_valid_name, EntityKind};
use pamoja_ros2::typehash::dds_type_name;

// A name is slash-separated tokens. A token may hold letters, digits, and underscores,
// and may not begin with a digit, which is the rule that catches most generated names.
for name in ["/robot1/camera_left/image_raw", "/2foo"] {
    println!("{name} is a valid name: {}", is_valid_name(name));
}
println!(
    "chatter is fully qualified: {}",
    is_fully_qualified("chatter")
);
println!(
    "/chatter is fully qualified: {}",
    is_fully_qualified("/chatter")
);

// On the wire a topic carries a prefix that says what kind of endpoint it is, so a
// subscription and a service request never collide in the same DDS partition.
let published = dds_topic("/robot1/cmd_vel", EntityKind::Topic).expect("a valid name");
let asked = dds_topic("/robot1/add", EntityKind::ServiceRequest).expect("a valid name");
let answered = dds_topic("/robot1/add", EntityKind::ServiceResponse).expect("a valid name");
println!("a topic    becomes {published}");
println!("a request  becomes {asked}");
println!("a response becomes {answered}");

// A message type maps to a DDS type name the same way, so both ends agree on what is
// being carried before a byte is exchanged. A name that is not well formed maps to
// nothing rather than to something plausible.
let carried = dds_type_name("std_msgs/msg/String").expect("a well-formed type name");
let malformed = dds_type_name("not a type");
println!("std_msgs/msg/String becomes {carried}");
println!(
    "a malformed type name becomes {}",
    malformed.as_deref().unwrap_or("nothing")
);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/ros2.ts#example -->
From [`bindings/node/guides/ros2.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ros2.ts):

```typescript
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
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/ros2.py#example -->
From [`bindings/python/guides/ros2.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ros2.py):

```python
from pamoja.ros2 import EntityKind, dds_topic, dds_type_name, is_fully_qualified, is_valid_name

# A name is slash-separated tokens. A token may hold letters, digits, and underscores, and
# may not begin with a digit, which is the rule that catches most generated names.
for name in ("/robot1/camera_left/image_raw", "/2foo"):
    print(f"{name} is a valid name: {is_valid_name(name)}")
print(f"chatter is fully qualified: {is_fully_qualified('chatter')}")
print(f"/chatter is fully qualified: {is_fully_qualified('/chatter')}")

# On the wire a topic carries a prefix that says what kind of endpoint it is, so a
# subscription and a service request never collide in the same DDS partition.
published = dds_topic("/robot1/cmd_vel", EntityKind.TOPIC)
asked = dds_topic("/robot1/add", EntityKind.SERVICE_REQUEST)
answered = dds_topic("/robot1/add", EntityKind.SERVICE_RESPONSE)
print(f"a topic    becomes {published}")
print(f"a request  becomes {asked}")
print(f"a response becomes {answered}")

# A message type maps to a DDS type name the same way, so both ends agree on what is being
# carried before a byte is exchanged.
print(f"std_msgs/msg/String becomes {dds_type_name('std_msgs/msg/String')}")
print(f"a malformed type name becomes {dds_type_name('not a type')}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs):

```csharp
// A name is slash-separated tokens. A token may hold letters, digits, and
// underscores, and may not begin with a digit, which is the rule that catches most
// generated names.
foreach (string candidate in new[] { "/robot1/camera_left/image_raw", "/2foo" })
{
    Console.WriteLine($"{candidate} is a valid name: {Ros2.IsValidName(candidate)}");
}

Console.WriteLine($"chatter is fully qualified: {Ros2.IsFullyQualified("chatter")}");
Console.WriteLine($"/chatter is fully qualified: {Ros2.IsFullyQualified("/chatter")}");

// On the wire a topic carries a prefix that says what kind of endpoint it is, so a
// subscription and a service request never collide in the same DDS partition.
string? published = Ros2.DdsTopic("/robot1/cmd_vel", EntityKind.Topic);
string? asked = Ros2.DdsTopic("/robot1/add", EntityKind.ServiceRequest);
string? answered = Ros2.DdsTopic("/robot1/add", EntityKind.ServiceResponse);
Console.WriteLine($"a topic    becomes {published}");
Console.WriteLine($"a request  becomes {asked}");
Console.WriteLine($"a response becomes {answered}");

// A message type maps to a DDS type name the same way, so both ends agree on what
// is being carried before a byte is exchanged.
Console.WriteLine(
    $"std_msgs/msg/String becomes {Ros2.DdsTypeName("std_msgs/msg/String")}");
Console.WriteLine(
    $"a malformed type name becomes {Ros2.DdsTypeName("not a type") ?? "nothing"}");
```
<!-- end -->

## Reference

<!-- table: reference ros2 -->
- Rust: [`pamoja-ros2`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html)
- TypeScript: [`@pamoja/ros2`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html)
- Python: [`pamoja.ros2`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html)
- C#: [`Pamoja.Ros2`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html)
<!-- end -->
