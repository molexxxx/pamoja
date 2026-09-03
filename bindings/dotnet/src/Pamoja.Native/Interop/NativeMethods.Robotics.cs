using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The P/Invoke declarations for the profile and robotics-logic capabilities of
/// the pamoja C ABI - device profiles, the ROS 2 naming and encoding rules, and
/// Zenoh key expressions - mirroring <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>Returns a pointer to an owned string's bytes.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_string_data(IntPtr text);

    /// <summary>Returns the length in bytes of an owned string.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_string_len(IntPtr text);

    /// <summary>Releases an owned string.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_string_free(IntPtr text);

    /// <summary>Creates a cold-chain fridge monitor.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_profile_vaccine_fridge_monitor();

    /// <summary>Creates an irrigation node.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_profile_irrigation_node();

    /// <summary>Creates a well-level monitor.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_profile_well_level();

    /// <summary>Creates a flood sensor.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_profile_flood_sensor();

    /// <summary>Loads a profile from its JSON manifest.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial IntPtr pamoja_profile_from_json(string manifest);

    /// <summary>Serializes a profile to its JSON manifest.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_profile_to_json(IntPtr profile);

    /// <summary>Returns a profile's stable name.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_profile_name(IntPtr profile);

    /// <summary>Returns the topic a profile publishes each reading to.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_profile_topic(IntPtr profile);

    /// <summary>Returns the control policy a profile applies.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_profile_control(
        IntPtr profile,
        out PamojaControlSpec control);

    /// <summary>Returns the sampling schedule a profile keeps.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_profile_power(
        IntPtr profile,
        out PamojaPowerSchedule schedule);

    /// <summary>Assembles a profile's schedule into a power governor.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_profile_power_plan(
        IntPtr profile,
        out PamojaPowerPlan plan);

    /// <summary>Builds the decision logic a profile describes.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_profile_controller(IntPtr profile);

    /// <summary>Releases a profile handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_profile_free(IntPtr profile);

    /// <summary>Creates a controller that holds a reading near a setpoint.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_controller_setpoint(
        float setpoint,
        float hysteresis,
        [MarshalAs(UnmanagedType.U1)] bool cooling,
        float safeBand);

    /// <summary>Creates a controller that warns before a level reaches empty.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_controller_level(float empty, uint warnWithin);

    /// <summary>Creates a controller that warns when a reading changes too fast.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_controller_surge(
        [MarshalAs(UnmanagedType.U1)] bool rising,
        float limit);

    /// <summary>Creates a controller that reports readings without judging them.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_controller_monitor();

    /// <summary>Decides what one reading calls for.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_controller_evaluate(
        IntPtr controller,
        float reading,
        out PamojaReaction reaction);

    /// <summary>Releases a controller handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_controller_free(IntPtr controller);

    /// <summary>Reports whether a string is a valid ROS 2 name.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_ros2_name_is_valid(string name);

    /// <summary>Reports whether a name is fully qualified.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_ros2_name_is_fully_qualified(string name);

    /// <summary>Returns the DDS topic prefix a subsystem uses.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ros2_entity_kind_prefix(PamojaEntityKind kind);

    /// <summary>Returns the DDS topic a fully qualified name maps onto.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial IntPtr pamoja_ros2_dds_topic(string fqn, PamojaEntityKind kind);

    /// <summary>Percent-mangles a name the way a DDS partition requires.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial IntPtr pamoja_ros2_percent_mangle(string name);

    /// <summary>Returns the DDS type name an interface type maps onto.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial IntPtr pamoja_ros2_dds_type_name(string rosType);

    /// <summary>Parses a RIHS01 type hash string.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_ros2_type_hash_parse(
        string text,
        out PamojaTypeHash hash);

    /// <summary>Renders a type hash back to its RIHS01 string.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ros2_type_hash_to_string(PamojaTypeHash hash);

    /// <summary>Builds the Zenoh key an entity is published on.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial IntPtr pamoja_ros2_entity_key(
        uint domainId,
        string fqn,
        string rosType,
        PamojaTypeHash hash);

    /// <summary>Encodes a twist into its CDR representation.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_ros2_twist_to_cdr(PamojaRos2Twist twist);

    /// <summary>Decodes a twist from its CDR representation.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_ros2_twist_from_cdr(
        ReadOnlySpan<byte> data,
        nuint dataLen,
        out PamojaRos2Twist twist);

    /// <summary>Creates a CDR encoder.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_cdr_writer_new();

    /// <summary>Appends a 32-bit signed integer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_cdr_writer_write_i32(IntPtr writer, int value);

    /// <summary>Appends a 32-bit unsigned integer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_cdr_writer_write_u32(IntPtr writer, uint value);

    /// <summary>Appends a 32-bit float.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_cdr_writer_write_f32(IntPtr writer, float value);

    /// <summary>Appends a 64-bit float.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_cdr_writer_write_f64(IntPtr writer, double value);

    /// <summary>Takes the encoded bytes, consuming the encoder.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_cdr_writer_into_bytes(IntPtr writer);

    /// <summary>Releases a CDR encoder handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_cdr_writer_free(IntPtr writer);

    /// <summary>Creates a CDR decoder over encoded bytes.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_cdr_reader_new(ReadOnlySpan<byte> data, nuint dataLen);

    /// <summary>Reads the next 32-bit signed integer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_cdr_reader_read_i32(IntPtr reader, out int value);

    /// <summary>Reads the next 32-bit unsigned integer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_cdr_reader_read_u32(IntPtr reader, out uint value);

    /// <summary>Reads the next 32-bit float.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_cdr_reader_read_f32(IntPtr reader, out float value);

    /// <summary>Reads the next 64-bit float.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_cdr_reader_read_f64(IntPtr reader, out double value);

    /// <summary>Releases a CDR decoder handle.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_cdr_reader_free(IntPtr reader);

    /// <summary>Reports whether a key expression is well formed.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_keyexpr_is_valid(string key);

    /// <summary>Reports whether a key expression is already canonical.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_keyexpr_is_canon(string key);

    /// <summary>Rewrites a key expression into its canonical form.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial IntPtr pamoja_keyexpr_canonize(string key);

    /// <summary>Reports whether a pattern selects a key.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [return: MarshalAs(UnmanagedType.U1)]
    public static partial bool pamoja_keyexpr_matches(string pattern, string key);
}
