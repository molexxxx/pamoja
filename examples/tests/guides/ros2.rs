//! The ROS 2 rules guide example; see docs/guides/ros2.md.

/// The naming rules a ROS 2 node has to agree with the rest of the graph on: what a valid
/// name is, and the DDS topic and type name it maps to. Checked against the rules the ROS 2
/// design documents publish, so a bridge interoperates without a ROS 2 install to compare
/// against.
#[test]
fn a_ros2_name_maps_to_the_dds_topic_and_type_the_graph_expects() {
    // ANCHOR: example
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
    // ANCHOR_END: example
}
