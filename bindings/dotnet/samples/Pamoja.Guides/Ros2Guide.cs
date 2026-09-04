using Pamoja.Ros2;

using static Guides.Guide;

namespace Guides;

/// <summary>The ROS 2 rules guide example; see docs/guides/ros2.md.</summary>
public static class Ros2Guide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
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
        // ANCHOR_END: example
    }
}
