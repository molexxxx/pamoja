using Pamoja.Ros2;

using static Guides.Guide;

namespace Guides;

/// <summary>The ROS 2 naming guide example; see docs/guides/ros2.md.</summary>
public static class Ros2Guide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
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
        // ANCHOR_END: example

        Expect(Ros2.IsValidName("/robot1/camera_left/image_raw"), "a well-formed name");
        Expect(!Ros2.IsValidName("/2foo"), "a token may not start with a digit");
        Expect(Ros2.IsFullyQualified("/chatter"), "a leading slash qualifies a name");
        Expect(!Ros2.IsFullyQualified("chatter"), "and without one it is relative");
        Expect(published == "rt/robot1/cmd_vel", "a topic carries the rt prefix");
        Expect(asked == "rq/robot1/add", "a request carries rq");
        Expect(answered == "rr/robot1/add", "and a response rr");
        Expect(
            Ros2.DdsTypeName("std_msgs/msg/String") == "std_msgs::msg::dds_::String_",
            "a message type maps to its DDS name");
        Expect(Ros2.DdsTypeName("not a type") is null, "and a malformed one maps to nothing");
    }
}
