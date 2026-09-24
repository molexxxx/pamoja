//! The ROS 2 naming guide example; see docs/guides/ros2.md.
//!
//! Run: `cargo run -p pamoja-examples --example ros2`

use std::error::Error;

/// The names a ROS 2 node uses and the DDS topics they become, the key an `rmw_zenoh` peer
/// finds them on, and the CDR a command velocity travels in: what has to line up before two
/// endpoints find each other on the wire and read what they exchange.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
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
    // ANCHOR_END: example

    assert_eq!(published, "rt/robot1/cmd_vel");
    assert_eq!(asked, "rq/robot1/add_two_intsRequest");
    assert_eq!(answered, "rr/robot1/add_two_intsReply");

    // ANCHOR: zenoh
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
    // ANCHOR_END: zenoh

    assert_eq!(
        key,
        format!("0/chatter/std_msgs::msg::dds_::String_/{STRING_HASH}")
    );

    // ANCHOR: cdr
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
    // ANCHOR_END: cdr

    assert_eq!(back, cmd);
    Ok(())
}
