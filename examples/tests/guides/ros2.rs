//! The ROS 2 naming guide example; see docs/guides/ros2.md.

/// The names a ROS 2 node uses and the DDS topics they become, which is what has to line
/// up before two endpoints can find each other on the wire.
#[test]
fn a_ros2_name_maps_to_the_dds_topic_and_type_the_graph_expects() {
    // ANCHOR: example
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
    let published = dds_topic("/robot1/cmd_vel", EntityKind::Topic);
    let asked = dds_topic("/robot1/add", EntityKind::ServiceRequest);
    let answered = dds_topic("/robot1/add", EntityKind::ServiceResponse);
    println!("a topic    becomes {published:?}");
    println!("a request  becomes {asked:?}");
    println!("a response becomes {answered:?}");

    // A message type maps to a DDS type name the same way, so both ends agree on what is
    // being carried before a byte is exchanged.
    println!(
        "std_msgs/msg/String becomes {:?}",
        dds_type_name("std_msgs/msg/String")
    );
    println!(
        "a malformed type name becomes {:?}",
        dds_type_name("not a type")
    );
    // ANCHOR_END: example

    assert!(is_valid_name("/robot1/camera_left/image_raw"));
    assert!(!is_valid_name("/2foo"));
    assert!(is_fully_qualified("/chatter"));
    assert!(!is_fully_qualified("chatter"));
    assert_eq!(published.as_deref(), Some("rt/robot1/cmd_vel"));
    assert_eq!(asked.as_deref(), Some("rq/robot1/add"));
    assert_eq!(answered.as_deref(), Some("rr/robot1/add"));
    assert_eq!(
        dds_type_name("std_msgs/msg/String").as_deref(),
        Some("std_msgs::msg::dds_::String_")
    );
    assert_eq!(dds_type_name("not a type"), None);
}
