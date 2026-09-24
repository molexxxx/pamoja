using Pamoja.Ros2;

using static System.FormattableString;
using static Guides.Guide;

namespace Guides;

/// <summary>The ROS 2 naming guide example; see docs/guides/ros2.md.</summary>
public static class Ros2Guide
{
    /// <summary>Runs the example.</summary>
    public static void Run()
    {
        // ANCHOR: example
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
        // ANCHOR_END: example

        Expect(published == "rt/robot1/cmd_vel", "a topic takes the rt prefix");
        Expect(asked == "rq/robot1/add_two_intsRequest", "a request takes rq and Request");
        Expect(answered == "rr/robot1/add_two_intsReply", "a reply takes rr and Reply");

        // ANCHOR: zenoh
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
        // ANCHOR_END: zenoh

        Expect(key == $"0/chatter/std_msgs::msg::dds_::String_/{stringHash}", "the documented key");

        // ANCHOR: cdr
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
        // ANCHOR_END: cdr

        Expect(back == command, "the command survives the round trip");
    }
}
