# ROS 2 rules

A ROS 2 graph agrees on names before it exchanges anything. A node's topic has a
name, that name maps to a DDS topic with a prefix saying what kind of endpoint it
is, and the message type maps to a DDS type name both ends must derive
identically. Over `rmw_zenoh` the same agreement is one Zenoh key built from the
name, the type, and a hash of the type's definition. And once two ends agree, what
they exchange is CDR, with each value aligned where the format says. pamoja
implements those rules and nothing else, so a bridge or a tool can speak the
graph's language with no ROS 2 installation anywhere near it.

## What the example does

It runs a camera topic and four malformed names past the name rules, shows a
relative and a private name that are valid but not yet fully qualified, and maps a
topic and both halves of a service to the DDS topics they travel on. Then it maps a
message type and a service type to their DDS type names, and refuses a type name
that is missing its namespace.

The second part builds the Zenoh key `rmw_zenoh` puts `/chatter` on, from the type
hash `rosidl` publishes for `std_msgs/msg/String`, and shows what refuses a key or
a hash. The third part encodes a command velocity as CDR, decodes it back, writes
an integer and a double to show the alignment padding between them, and decodes a
message cut short.

The wire names are derived rather than written out. Each comes from the ROS name
and the kind of endpoint carrying it, and the key from its parts, so a reader sees
which part of a wire name the library supplies.

It proves:

- A token holds letters, digits, and underscores and may not begin with a digit, and
  a name may not end in a slash, hold an empty token, or double an underscore.
- A relative name and a private name are valid, and neither is fully qualified, so
  neither reaches the wire until the node resolves it.
- A topic travels under `rt`, and a service on two topics: its request under `rq`
  with `Request` appended, and its reply under `rr` with `Reply` appended.
- `std_msgs/msg/String` becomes `std_msgs::msg::dds_::String_`, and a name that is
  not `package/namespace/Type` becomes nothing rather than something plausible.
- The `rmw_zenoh` key for `/chatter` is the one its design document publishes, a
  relative name has no key, and a hash one digit short is refused.
- A command velocity is 52 bytes of CDR and decodes to the command that was sent,
  an integer and a double take 20 bytes where their sizes add to 16, and a message
  cut short decodes as nothing.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example ros2" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example ros2</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- ros2" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- ros2</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/ros2.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/ros2.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- ros2" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- ros2</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-ros2` is `no_std` by default. `name` holds the name rules and the
DDS mapping, `typehash` the DDS type name and the `RIHS01` `TypeHash`, `key` the
`rmw_zenoh` `entity_key`, and `msg` the `CdrWriter`, `CdrReader`, and the geometry
messages. Every mapping returns an `Option`, `None` for a name, type, or buffer it
cannot use. The live bridge onto a ROS 2 graph is the `bridge` feature, which needs
a sourced ROS 2 install.

<!-- snippet: examples/guides/ros2.rs#example -->
From [`examples/guides/ros2.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/ros2.rs):

```rust
use pamoja_ros2::name::{dds_topic, is_fully_qualified, is_valid_name, EntityKind};
use pamoja_ros2::typehash::dds_type_name;

// A name is slash-separated tokens of letters, digits, and underscores. A token may not
// start with a digit, and a name may not end in a slash, hold an empty token, or double
// an underscore.
let camera = "/robot1/camera_left/image_raw";
if is_valid_name(camera) {
    println!("valid     {camera}");
}
for (name, why) in [
    ("/2foo", "a token starts with a digit"),
    ("/cmd_vel/", "it ends in a slash"),
    ("/robot1//odom", "it has an empty token"),
    ("/robot1/cmd__vel", "it doubles an underscore"),
] {
    if !is_valid_name(name) {
        println!("invalid   {name}, since {why}");
    }
}

// A name with no leading slash is relative, and one that starts with a tilde is private.
// Both are valid, and both resolve against the node before they reach the wire, so
// neither is fully qualified.
for (label, name, against) in [
    ("relative", "cmd_vel", "the node's namespace"),
    ("private", "~/setpoint", "the node's own name"),
] {
    if is_valid_name(name) && !is_fully_qualified(name) {
        println!("{label:<10}{name} is valid, and resolves against {against} first");
    }
}

// Only a fully qualified name reaches the wire. DDS puts a prefix before it that says
// what kind of endpoint it is, and a service travels on two topics, each ending in the
// suffix the middleware appends.
let published = dds_topic("/robot1/cmd_vel", EntityKind::Topic).expect("a qualified name");
let asked =
    dds_topic("/robot1/add_two_ints", EntityKind::ServiceRequest).expect("a qualified name");
let answered =
    dds_topic("/robot1/add_two_ints", EntityKind::ServiceResponse).expect("a qualified name");
println!("topic     /robot1/cmd_vel travels on {published}");
println!("request   /robot1/add_two_ints asks on {asked}");
println!("reply     and answers on {answered}");

// A message type maps to a DDS type name the same way, so both ends agree on what is
// carried before a byte is exchanged. A name that is not package/namespace/Type maps to
// nothing rather than to something plausible.
for ros_type in [
    "std_msgs/msg/String",
    "example_interfaces/srv/AddTwoInts",
    "std_msgs/String",
] {
    match dds_type_name(ros_type) {
        Some(carried) => println!("type      {ros_type} is named {carried}"),
        None => println!(
            "malformed {ros_type} is not package/namespace/Type, so it has no DDS type name"
        ),
    }
}
```
<!-- end -->

The Zenoh key and the type hash, continuing from above:

<!-- snippet: examples/guides/ros2.rs#zenoh -->
From [`examples/guides/ros2.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/ros2.rs):

```rust
use pamoja_ros2::key::entity_key;
use pamoja_ros2::name::percent_mangle;
use pamoja_ros2::typehash::TypeHash;

// A type hash pins the message definition itself, so two builds of a message that share
// a name but not a layout never talk. This is the hash rosidl publishes for
// std_msgs/msg/String: RIHS01, then a SHA-256 in hex.
const STRING_HASH: &str =
    "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18";
let hash = TypeHash::parse(STRING_HASH).expect("the published hash");
println!(
    "hash      std_msgs/msg/String carries RIHS01 with a {}-byte SHA-256",
    hash.digest().len()
);

// rmw_zenoh puts a topic on the key <domain>/<name>/<type>/<hash>, so a peer that builds
// the same key talks to ROS 2 nodes with no DDS in the path.
let key = entity_key(0, "/chatter", "std_msgs/msg/String", &hash).expect("a usable key");
println!("key       {key}");
if entity_key(0, "chatter", "std_msgs/msg/String", &hash).is_none() {
    println!("no key    a relative name has no key until the node resolves it");
}

// A hash one digit short is not a hash, and is refused rather than matched loosely.
if TypeHash::parse(&STRING_HASH[..STRING_HASH.len() - 1]).is_none() {
    println!("refused   a hash one hex digit short is not RIHS01");
}

// A liveliness token says who is on the graph, and in it rmw_zenoh writes each slash of
// a name as a percent sign.
println!(
    "token     /robot1/chatter is written {} in a liveliness token",
    percent_mangle("/robot1/chatter")
);
```
<!-- end -->

CDR, continuing from above:

<!-- snippet: examples/guides/ros2.rs#cdr -->
From [`examples/guides/ros2.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/ros2.rs):

```rust
use pamoja_ros2::msg::{CdrReader, CdrWriter, Twist, Vector3};

// A command velocity: half a meter a second forward, turning at 0.2 radians a second.
// CDR opens with a four-byte header naming the byte order, then the six doubles.
let cmd = Twist {
    linear: Vector3::new(0.5, 0.0, 0.0),
    angular: Vector3::new(0.0, 0.0, 0.2),
};
let bytes = cmd.to_cdr();
println!(
    "twist     {} bytes: a 4-byte header, then six 8-byte doubles",
    bytes.len()
);
let back = Twist::from_cdr(&bytes).expect("a whole twist");
println!(
    "decoded   forward {} m/s, turning {} rad/s, the command that was sent",
    back.linear.x, back.angular.z
);

// Each value is aligned to its own size, counted from after the header, so an integer
// followed by a double takes four bytes of padding before the double.
let mut writer = CdrWriter::new();
writer.write_i32(7);
writer.write_f64(2.5);
let mixed = writer.into_bytes();
let mut reader = CdrReader::new(&mixed).expect("a CDR header");
let count = reader.read_i32().expect("an integer");
let level = reader.read_f64().expect("a double");
println!(
    "aligned   an i32 then an f64 take {} bytes, not 16, and read back as {count} and {level}",
    mixed.len()
);

// A message cut short decodes as nothing rather than as a plausible command.
if Twist::from_cdr(&bytes[..bytes.len() - 8]).is_none() {
    println!("short     a twist missing its last double decodes as nothing");
}
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/ros2` groups the rules into `name` (`isValid`,
`isFullyQualified`, `ddsTopic`, `ddsTypeName`, `prefixFor`, `suffixFor`,
`percentMangle`), `typeHash` (`digest`, `entityKey`), and `cdr` (`twistToBytes`,
`twistFromBytes`, `writer`, `reader`). A mapping that cannot be made returns `null`,
and bytes are a `Buffer`.

<!-- snippet: bindings/node/guides/ros2.ts#example -->
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
<!-- end -->

The Zenoh key and the type hash, continuing from above:

<!-- snippet: bindings/node/guides/ros2.ts#zenoh -->
From [`bindings/node/guides/ros2.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ros2.ts):

```typescript
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
```
<!-- end -->

CDR, continuing from above:

<!-- snippet: bindings/node/guides/ros2.ts#cdr -->
From [`bindings/node/guides/ros2.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/ros2.ts):

```typescript
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
```
<!-- end -->

## Python

In Python, `pamoja.ros2` has the rules as functions, with `EntityKind` naming the
three endpoints and `CdrWriter` and `CdrReader` for CDR. A mapping that cannot be
made returns `None`, a twist crosses as two `(x, y, z)` triples, and bytes are
`bytes`.

<!-- snippet: bindings/python/guides/ros2.py#example -->
From [`bindings/python/guides/ros2.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ros2.py):

```python
from pamoja.ros2 import EntityKind, dds_topic, dds_type_name, is_fully_qualified, is_valid_name

# A name is slash-separated tokens of letters, digits, and underscores. A token may not start
# with a digit, and a name may not end in a slash, hold an empty token, or double an
# underscore.
camera = "/robot1/camera_left/image_raw"
if is_valid_name(camera):
    print(f"valid     {camera}")
for name, why in [
    ("/2foo", "a token starts with a digit"),
    ("/cmd_vel/", "it ends in a slash"),
    ("/robot1//odom", "it has an empty token"),
    ("/robot1/cmd__vel", "it doubles an underscore"),
]:
    if not is_valid_name(name):
        print(f"invalid   {name}, since {why}")

# A name with no leading slash is relative, and one that starts with a tilde is private. Both
# are valid, and both resolve against the node before they reach the wire, so neither is
# fully qualified.
for label, name, against in [
    ("relative", "cmd_vel", "the node's namespace"),
    ("private", "~/setpoint", "the node's own name"),
]:
    if is_valid_name(name) and not is_fully_qualified(name):
        print(f"{label:<10}{name} is valid, and resolves against {against} first")

# Only a fully qualified name reaches the wire. DDS puts a prefix before it that says what
# kind of endpoint it is, and a service travels on two topics, each ending in the suffix the
# middleware appends.
published = dds_topic("/robot1/cmd_vel", EntityKind.TOPIC)
asked = dds_topic("/robot1/add_two_ints", EntityKind.SERVICE_REQUEST)
answered = dds_topic("/robot1/add_two_ints", EntityKind.SERVICE_RESPONSE)
print(f"topic     /robot1/cmd_vel travels on {published}")
print(f"request   /robot1/add_two_ints asks on {asked}")
print(f"reply     and answers on {answered}")

# A message type maps to a DDS type name the same way, so both ends agree on what is carried
# before a byte is exchanged. A name that is not package/namespace/Type maps to nothing rather
# than to something plausible.
for ros_type in ["std_msgs/msg/String", "example_interfaces/srv/AddTwoInts", "std_msgs/String"]:
    carried = dds_type_name(ros_type)
    if carried is not None:
        print(f"type      {ros_type} is named {carried}")
    else:
        print(f"malformed {ros_type} is not package/namespace/Type, so it has no DDS type name")
```
<!-- end -->

The Zenoh key and the type hash, continuing from above:

<!-- snippet: bindings/python/guides/ros2.py#zenoh -->
From [`bindings/python/guides/ros2.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ros2.py):

```python
from pamoja.ros2 import entity_key, percent_mangle, type_hash_digest

# A type hash pins the message definition itself, so two builds of a message that share a
# name but not a layout never talk. This is the hash rosidl publishes for std_msgs/msg/String:
# RIHS01, then a SHA-256 in hex.
STRING_HASH = "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18"
digest = type_hash_digest(STRING_HASH)
print(f"hash      std_msgs/msg/String carries RIHS01 with a {len(digest)}-byte SHA-256")

# rmw_zenoh puts a topic on the key <domain>/<name>/<type>/<hash>, so a peer that builds the
# same key talks to ROS 2 nodes with no DDS in the path.
key = entity_key(0, "/chatter", "std_msgs/msg/String", STRING_HASH)
print(f"key       {key}")
if entity_key(0, "chatter", "std_msgs/msg/String", STRING_HASH) is None:
    print("no key    a relative name has no key until the node resolves it")

# A hash one digit short is not a hash, and is refused rather than matched loosely.
if type_hash_digest(STRING_HASH[:-1]) is None:
    print("refused   a hash one hex digit short is not RIHS01")

# A liveliness token says who is on the graph, and in it rmw_zenoh writes each slash of a name
# as a percent sign.
print(
    f"token     /robot1/chatter is written {percent_mangle('/robot1/chatter')} "
    "in a liveliness token"
)
```
<!-- end -->

CDR, continuing from above:

<!-- snippet: bindings/python/guides/ros2.py#cdr -->
From [`bindings/python/guides/ros2.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/ros2.py):

```python
from pamoja.ros2 import CdrReader, CdrWriter, twist_from_cdr, twist_to_cdr

# A command velocity: half a meter a second forward, turning at 0.2 radians a second. CDR
# opens with a four-byte header naming the byte order, then the six doubles.
linear, angular = (0.5, 0.0, 0.0), (0.0, 0.0, 0.2)
data = twist_to_cdr(linear, angular)
print(f"twist     {len(data)} bytes: a 4-byte header, then six 8-byte doubles")
back_linear, back_angular = twist_from_cdr(data)
print(
    f"decoded   forward {back_linear[0]:g} m/s, turning {back_angular[2]:g} rad/s, "
    "the command that was sent"
)

# Each value is aligned to its own size, counted from after the header, so an integer
# followed by a double takes four bytes of padding before the double.
writer = CdrWriter()
writer.write_i32(7)
writer.write_f64(2.5)
mixed = writer.bytes
reader = CdrReader(mixed)
count = reader.read_i32()
level = reader.read_f64()
print(
    f"aligned   an i32 then an f64 take {len(mixed)} bytes, not 16, "
    f"and read back as {count} and {level:g}"
)

# A message cut short decodes as nothing rather than as a plausible command.
if twist_from_cdr(data[:-8]) is None:
    print("short     a twist missing its last double decodes as nothing")
```
<!-- end -->

## C#

In C#, the static `Ros2` class holds the rules, and `CdrWriter` and `CdrReader`
hold native state and are disposed with `using`. A mapping that cannot be made
returns `null`, a twist is a `Ros2Twist` record of two `Vector3`s, and a reader
returns `null` once the bytes run out.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs):

```csharp
// A name is slash-separated tokens of letters, digits, and underscores. A token may
// not start with a digit, and a name may not end in a slash, hold an empty token, or
// double an underscore.
const string camera = "/robot1/camera_left/image_raw";
if (Ros2.IsValidName(camera))
{
    Console.WriteLine($"valid     {camera}");
}

foreach ((string name, string why) in new[]
{
    ("/2foo", "a token starts with a digit"),
    ("/cmd_vel/", "it ends in a slash"),
    ("/robot1//odom", "it has an empty token"),
    ("/robot1/cmd__vel", "it doubles an underscore"),
})
{
    if (!Ros2.IsValidName(name))
    {
        Console.WriteLine($"invalid   {name}, since {why}");
    }
}

// A name with no leading slash is relative, and one that starts with a tilde is
// private. Both are valid, and both resolve against the node before they reach the
// wire, so neither is fully qualified.
foreach ((string label, string name, string against) in new[]
{
    ("relative", "cmd_vel", "the node's namespace"),
    ("private", "~/setpoint", "the node's own name"),
})
{
    if (Ros2.IsValidName(name) && !Ros2.IsFullyQualified(name))
    {
        Console.WriteLine($"{label,-10}{name} is valid, and resolves against {against} first");
    }
}

// Only a fully qualified name reaches the wire. DDS puts a prefix before it that says
// what kind of endpoint it is, and a service travels on two topics, each ending in the
// suffix the middleware appends.
string? published = Ros2.DdsTopic("/robot1/cmd_vel", EntityKind.Topic);
string? asked = Ros2.DdsTopic("/robot1/add_two_ints", EntityKind.ServiceRequest);
string? answered = Ros2.DdsTopic("/robot1/add_two_ints", EntityKind.ServiceResponse);
Console.WriteLine($"topic     /robot1/cmd_vel travels on {published}");
Console.WriteLine($"request   /robot1/add_two_ints asks on {asked}");
Console.WriteLine($"reply     and answers on {answered}");

// A message type maps to a DDS type name the same way, so both ends agree on what is
// carried before a byte is exchanged. A name that is not package/namespace/Type maps
// to nothing rather than to something plausible.
foreach (string rosType in new[]
{
    "std_msgs/msg/String",
    "example_interfaces/srv/AddTwoInts",
    "std_msgs/String",
})
{
    Console.WriteLine(Ros2.DdsTypeName(rosType) is { } carried
        ? $"type      {rosType} is named {carried}"
        : $"malformed {rosType} is not package/namespace/Type, so it has no DDS type name");
}
```
<!-- end -->

The Zenoh key and the type hash, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs#zenoh -->
From [`bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs):

```csharp
// A type hash pins the message definition itself, so two builds of a message that
// share a name but not a layout never talk. This is the hash rosidl publishes for
// std_msgs/msg/String: RIHS01, then a SHA-256 in hex.
const string stringHash =
    "RIHS01_df668c740482bbd48fb39d76a70dfd4bd59db1288021743503259e948f6b1a18";
byte[]? digest = Ros2.TypeHashDigest(stringHash);
Console.WriteLine($"hash      std_msgs/msg/String carries RIHS01 with a {digest?.Length}-byte SHA-256");

// rmw_zenoh puts a topic on the key <domain>/<name>/<type>/<hash>, so a peer that
// builds the same key talks to ROS 2 nodes with no DDS in the path.
string? key = Ros2.EntityKey(0, "/chatter", "std_msgs/msg/String", stringHash);
Console.WriteLine($"key       {key}");
if (Ros2.EntityKey(0, "chatter", "std_msgs/msg/String", stringHash) is null)
{
    Console.WriteLine("no key    a relative name has no key until the node resolves it");
}

// A hash one digit short is not a hash, and is refused rather than matched loosely.
if (Ros2.TypeHashDigest(stringHash[..^1]) is null)
{
    Console.WriteLine("refused   a hash one hex digit short is not RIHS01");
}

// A liveliness token says who is on the graph, and in it rmw_zenoh writes each slash
// of a name as a percent sign.
Console.WriteLine(
    $"token     /robot1/chatter is written {Ros2.PercentMangle("/robot1/chatter")} in a liveliness token");
```
<!-- end -->

CDR, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs#cdr -->
From [`bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/Ros2Guide.cs):

```csharp
// A command velocity: half a meter a second forward, turning at 0.2 radians a second.
// CDR opens with a four-byte header naming the byte order, then the six doubles.
var command = new Ros2Twist(new Vector3(0.5, 0, 0), new Vector3(0, 0, 0.2));
byte[] bytes = Ros2.TwistToCdr(command);
Console.WriteLine($"twist     {bytes.Length} bytes: a 4-byte header, then six 8-byte doubles");
Ros2Twist back = Ros2.TwistFromCdr(bytes) ?? throw new InvalidOperationException("a whole twist");
Console.WriteLine(Invariant(
    $"decoded   forward {back.Linear.X} m/s, turning {back.Angular.Z} rad/s, the command that was sent"));

// Each value is aligned to its own size, counted from after the header, so an integer
// followed by a double takes four bytes of padding before the double.
using var writer = new CdrWriter();
writer.WriteInt32(7);
writer.WriteDouble(2.5);
byte[] mixed = writer.ToBytes();
using var reader = new CdrReader(mixed);
int? count = reader.ReadInt32();
double? level = reader.ReadDouble();
Console.WriteLine(Invariant(
    $"aligned   an i32 then an f64 take {mixed.Length} bytes, not 16, and read back as {count} and {level}"));

// A message cut short decodes as nothing rather than as a plausible command.
if (Ros2.TwistFromCdr(bytes.AsSpan(0, bytes.Length - 8)) is null)
{
    Console.WriteLine("short     a twist missing its last double decodes as nothing");
}
```
<!-- end -->

## Values at a glance

**What a name may be,** from the ROS 2 design for topic and service names:

| Rule | Breaks it |
| --- | --- |
| not empty | `""` |
| letters, digits, underscores, and slashes only | `/cmd vel` |
| no token starts with a digit | `/2foo` |
| no trailing slash | `/cmd_vel/` |
| no repeated slash | `/robot1//odom` |
| no repeated underscore | `/robot1/cmd__vel` |
| a tilde only at the start, and then followed by a slash | `/robot1/~odom` |
| braces balanced, holding letters, digits, and underscores | `/robot1/{ns` |

**Where a name resolves:**

| A name that | Is | Resolves against |
| --- | --- | --- |
| starts with `/` | fully qualified, when it holds no `~` or `{}` | nothing: it goes on the wire as it is |
| starts with a letter or `_` | relative | the node's namespace |
| starts with `~/` | private | the node's own name |
| holds `{...}` | a substitution | whatever is substituted at run time |

Only a fully qualified name maps to a DDS topic or a Zenoh key.

**The DDS topics,** as `rmw_fastrtps` and `rmw_cyclonedds` both build them:

| Endpoint | DDS topic | `/robot1/add_two_ints` travels on |
| --- | --- | --- |
| topic | `rt`, then the name | `rt/robot1/add_two_ints` |
| service request | `rq`, the name, then `Request` | `rq/robot1/add_two_intsRequest` |
| service reply | `rr`, the name, then `Reply` | `rr/robot1/add_two_intsReply` |

The ROS 2 design reserves `rs`, `rp`, and `ra` for services, parameters, and
actions as well; pamoja maps the three above.

**The type names:**

| Interface | DDS type name |
| --- | --- |
| `std_msgs/msg/String` | `std_msgs::msg::dds_::String_` |
| `example_interfaces/srv/AddTwoInts` | `example_interfaces::srv::dds_::AddTwoInts_` |
| anything but `package/namespace/Type` | none |

On DDS a service's request and reply carry their own types,
`AddTwoInts_Request_` and `AddTwoInts_Response_`; `rmw_zenoh` keys the service by
`AddTwoInts_`.

**The `rmw_zenoh` key** is `<domain>/<name>/<type>/<hash>`, with the name's
leading slash as the separator after the domain:

| Part | For `/chatter` in domain 0 |
| --- | --- |
| domain | `0`, the `ROS_DOMAIN_ID` |
| name | `chatter` |
| type | `std_msgs::msg::dds_::String_` |
| hash | `RIHS01_` and the 64 hex digits of a SHA-256 over the type's description |

In a liveliness token `rmw_zenoh` writes each `/` of a name as `%`, so
`/robot1/chatter` reads `%robot1%chatter`.

**CDR:**

| Part | Size |
| --- | --- |
| encapsulation header | 4 bytes: classic CDR, little-endian |
| `i32`, `u32`, `f32` | 4 bytes, aligned to 4 from after the header |
| `f64` | 8 bytes, aligned to 8 from after the header |
| `geometry_msgs/msg/Twist` | 52 bytes: the header and six `f64` |

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| check a name | `name::is_valid_name(name)`, `name::is_fully_qualified(name)` |
| map it to DDS | `name::dds_topic(fqn, EntityKind::Topic)`, `EntityKind::prefix()`, `suffix()`, `typehash::dds_type_name(ros_type)` |
| key it for `rmw_zenoh` | `typehash::TypeHash::parse(text)`, `digest()`, `key::entity_key(domain, fqn, ros_type, &hash)`, `name::percent_mangle(name)` |
| encode and decode | `msg::Twist { linear, angular }.to_cdr()`, `Twist::from_cdr(bytes)`, `CdrWriter::new()`, `write_i32`, `write_f64`, `into_bytes()`, `CdrReader::new(bytes)`, `read_i32`, `read_f64` |

### TypeScript

| To | Call |
| --- | --- |
| check a name | `name.isValid(name)`, `name.isFullyQualified(name)` |
| map it to DDS | `name.ddsTopic(fqn, EntityKind.Topic)`, `name.prefixFor(kind)`, `name.suffixFor(kind)`, `name.ddsTypeName(rosType)` |
| key it for `rmw_zenoh` | `typeHash.digest(text)`, `typeHash.entityKey(domain, fqn, rosType, hash)`, `name.percentMangle(name)` |
| encode and decode | `cdr.twistToBytes(twist)`, `cdr.twistFromBytes(bytes)`, `cdr.writer()`, `writeI32`, `writeF64`, `bytes`, `cdr.reader(bytes)`, `readI32`, `readF64` |

### Python

| To | Call |
| --- | --- |
| check a name | `is_valid_name(name)`, `is_fully_qualified(name)` |
| map it to DDS | `dds_topic(fqn, EntityKind.TOPIC)`, `prefix_for(kind)`, `suffix_for(kind)`, `dds_type_name(ros_type)` |
| key it for `rmw_zenoh` | `type_hash_digest(text)`, `entity_key(domain, fqn, ros_type, hash)`, `percent_mangle(name)` |
| encode and decode | `twist_to_cdr(linear, angular)`, `twist_from_cdr(data)`, `CdrWriter()`, `write_i32`, `write_f64`, `bytes`, `CdrReader(data)`, `read_i32`, `read_f64` |

### C#

| To | Call |
| --- | --- |
| check a name | `Ros2.IsValidName(name)`, `Ros2.IsFullyQualified(name)` |
| map it to DDS | `Ros2.DdsTopic(fqn, EntityKind.Topic)`, `Ros2.PrefixFor(kind)`, `Ros2.SuffixFor(kind)`, `Ros2.DdsTypeName(rosType)` |
| key it for `rmw_zenoh` | `Ros2.TypeHashDigest(text)`, `Ros2.EntityKey(domain, fqn, rosType, hash)`, `Ros2.PercentMangle(name)` |
| encode and decode | `Ros2.TwistToCdr(twist)`, `Ros2.TwistFromCdr(bytes)`, `new CdrWriter()`, `WriteInt32`, `WriteDouble`, `ToBytes()`, `new CdrReader(bytes)`, `ReadInt32`, `ReadDouble` |

<!-- languages end -->

## When it goes wrong

A name, a type, or a buffer the rules refuse comes back as nothing. What gets past
the rules shows up as two ends that never find each other, or read each other
wrong. The ones that cost an afternoon:

- **A service call is never answered.** A service travels on two DDS topics, and
  each name ends in its suffix: `rq/add_two_intsRequest` and
  `rr/add_two_intsReply`. A bridge that drops the suffix subscribes to topics no
  ROS 2 node publishes on.
- **A node talks to itself.** A relative name resolves against the namespace, so
  `cmd_vel` in a node under `/robot1` is `/robot1/cmd_vel`, and the same code in a
  node with no namespace is `/cmd_vel`. Resolve names before mapping them, or write
  them fully qualified.
- **An `rmw_zenoh` peer sees nothing.** Its key carries the domain and the type hash,
  so a peer on another `ROS_DOMAIN_ID`, or one built from a different version of the
  message, is on a different key. Take the hash from the peer's own `rosidl` build,
  not from another distribution.
- **Every value after the first is wrong.** CDR aligns each value to its own size
  from after the header, so a writer that packs an `i32` and an `f64` into 12 bytes
  shifts everything after them by 4. Write with `CdrWriter`, and read in the order
  the message declares its fields.
- **A message from another machine will not decode.** The reader decodes classic
  little-endian CDR, and refuses a big-endian header rather than misreading it.
- **A type name matches nothing.** The DDS type name carries the `dds_` namespace and
  a trailing underscore, and a service's request and reply carry their own types, so
  a name typed by hand misses one of the three.

## Where next

<!-- table: next ros2 -->
- [Zenoh keys](zenoh.md): Zenoh key expressions.
- [MAVLink](mavlink.md): MAVLink v1 and v2 framing, signing, named message fields and enum values, and the mission, command, and offboard protocols.
- [Rules](rules.md): Rules between nodes as a file.
- Also in Profiles and robotics: [Device profiles](profile.md), [Robot motion](motion.md).
<!-- end -->

## Reference

<!-- table: reference ros2 -->
- Rust: [`pamoja-ros2`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_ros2/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-ros2)
- TypeScript: [`@pamoja/ros2`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_ros2.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-ros2)
- Python: [`pamoja.ros2`](https://pamoja.molex.cloud/docs/reference/python/pamoja/ros2.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-ros2)
- C#: [`Pamoja.Ros2`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Ros2.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-ros2)
<!-- end -->
