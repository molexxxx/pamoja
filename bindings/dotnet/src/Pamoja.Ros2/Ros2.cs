using Pamoja.Native.Interop;

namespace Pamoja.Ros2;

/// <summary>The ROS 2 subsystem a name belongs to, which fixes its DDS prefix.</summary>
public enum EntityKind
{
    /// <summary>A topic, which takes the <c>rt</c> prefix.</summary>
    Topic = 0,

    /// <summary>The request side of a service, which takes the <c>rq</c> prefix.</summary>
    ServiceRequest = 1,

    /// <summary>The reply side of a service, which takes the <c>rr</c> prefix.</summary>
    ServiceResponse = 2,
}

/// <summary>A three-dimensional vector, matching <c>geometry_msgs/msg/Vector3</c>.</summary>
/// <param name="X">The x component.</param>
/// <param name="Y">The y component.</param>
/// <param name="Z">The z component.</param>
public readonly record struct Vector3(double X, double Y, double Z);

/// <summary>A body velocity command, matching <c>geometry_msgs/msg/Twist</c>.</summary>
/// <remarks>This is what a ROS 2 robot is driven by on <c>cmd_vel</c>.</remarks>
/// <param name="Linear">The linear velocity in metres per second.</param>
/// <param name="Angular">The angular velocity in radians per second.</param>
public readonly record struct Ros2Twist(Vector3 Linear, Vector3 Angular);

/// <summary>
/// The ROS 2 naming and encoding rules, none of which need a ROS installation.
/// </summary>
/// <remarks>
/// A gateway written in C# can validate a name, derive the DDS topic and the
/// Zenoh key an <c>rmw_zenoh</c> peer subscribes on, and encode a
/// <c>geometry_msgs/msg/Twist</c> with no ROS distribution anywhere near it.
/// Driving a live graph does need one, so that stays in the Rust crate.
/// </remarks>
public static class Ros2
{
    /// <summary>Reports whether a string is a valid ROS 2 topic or service name.</summary>
    /// <param name="name">The candidate name.</param>
    /// <returns><c>true</c> when the name obeys the ROS 2 rules.</returns>
    public static bool IsValidName(string name) =>
        NativeMethods.pamoja_ros2_name_is_valid(name);

    /// <summary>Reports whether a name resolves with no namespace applied.</summary>
    /// <param name="name">The candidate name.</param>
    /// <returns><c>true</c> when the name is fully qualified.</returns>
    public static bool IsFullyQualified(string name) =>
        NativeMethods.pamoja_ros2_name_is_fully_qualified(name);

    /// <summary>Returns the DDS topic prefix a subsystem uses.</summary>
    /// <param name="kind">The subsystem.</param>
    /// <returns><c>rt</c>, <c>rq</c>, or <c>rr</c>.</returns>
    public static string PrefixFor(EntityKind kind) =>
        System.Runtime.InteropServices.Marshal.PtrToStringUTF8(
            NativeMethods.pamoja_ros2_entity_kind_prefix((PamojaEntityKind)kind)) ?? string.Empty;

    /// <summary>Returns the DDS topic a fully qualified name maps onto.</summary>
    /// <param name="fqn">The fully qualified name.</param>
    /// <param name="kind">Which subsystem the name belongs to.</param>
    /// <returns>The DDS topic, or <c>null</c> if the name is not fully qualified.</returns>
    public static string? DdsTopic(string fqn, EntityKind kind) =>
        OwnedString.ReadOrNull(
            NativeMethods.pamoja_ros2_dds_topic(fqn, (PamojaEntityKind)kind));

    /// <summary>Percent-mangles a name the way a DDS partition requires.</summary>
    /// <param name="name">The name to mangle.</param>
    /// <returns>The mangled name.</returns>
    public static string PercentMangle(string name) =>
        OwnedString.Read(NativeMethods.pamoja_ros2_percent_mangle(name));

    /// <summary>Returns the DDS type name an interface type maps onto.</summary>
    /// <param name="rosType">The interface type as <c>package/namespace/Type</c>.</param>
    /// <returns>The DDS type name, or <c>null</c> if the type is not valid.</returns>
    public static string? DdsTypeName(string rosType) =>
        OwnedString.ReadOrNull(NativeMethods.pamoja_ros2_dds_type_name(rosType));

    /// <summary>Returns the 32-byte digest a RIHS01 hash string carries.</summary>
    /// <param name="text">The hash as its <c>RIHS01_</c> string.</param>
    /// <returns>The digest, or <c>null</c> if the string is malformed.</returns>
    public static byte[]? TypeHashDigest(string text) =>
        NativeMethods.pamoja_ros2_type_hash_parse(text, out PamojaTypeHash hash)
            == PamojaStatus.Ok
            ? hash.Digest.ToArray()
            : null;

    /// <summary>Builds the Zenoh key an <c>rmw_zenoh</c> peer publishes an entity on.</summary>
    /// <param name="domainId">The ROS 2 domain.</param>
    /// <param name="fqn">The fully qualified entity name.</param>
    /// <param name="rosType">The interface type as <c>package/namespace/Type</c>.</param>
    /// <param name="typeHash">The message type hash as its <c>RIHS01_</c> string.</param>
    /// <returns>The key, or <c>null</c> if the name, type, or hash is not usable.</returns>
    public static string? EntityKey(uint domainId, string fqn, string rosType, string typeHash)
    {
        if (NativeMethods.pamoja_ros2_type_hash_parse(typeHash, out PamojaTypeHash hash)
            != PamojaStatus.Ok)
        {
            return null;
        }

        return OwnedString.ReadOrNull(
            NativeMethods.pamoja_ros2_entity_key(domainId, fqn, rosType, hash));
    }

    /// <summary>Encodes a twist into its CDR representation.</summary>
    /// <param name="twist">The command to encode.</param>
    /// <returns>The encoded bytes.</returns>
    public static byte[] TwistToCdr(Ros2Twist twist)
    {
        PamojaRos2Twist native = new()
        {
            Linear = new PamojaVector3
            {
                X = twist.Linear.X,
                Y = twist.Linear.Y,
                Z = twist.Linear.Z,
            },
            Angular = new PamojaVector3
            {
                X = twist.Angular.X,
                Y = twist.Angular.Y,
                Z = twist.Angular.Z,
            },
        };
        return NativeBuffer.Read(NativeMethods.pamoja_ros2_twist_to_cdr(native));
    }

    /// <summary>Decodes a twist from its CDR representation.</summary>
    /// <param name="data">The encoded bytes.</param>
    /// <returns>The command, or <c>null</c> if the bytes are not a well-formed twist.</returns>
    public static Ros2Twist? TwistFromCdr(ReadOnlySpan<byte> data)
    {
        if (NativeMethods.pamoja_ros2_twist_from_cdr(data, (nuint)data.Length, out PamojaRos2Twist t)
            != PamojaStatus.Ok)
        {
            return null;
        }

        return new Ros2Twist(
            new Vector3(t.Linear.X, t.Linear.Y, t.Linear.Z),
            new Vector3(t.Angular.X, t.Angular.Y, t.Angular.Z));
    }
}

/// <summary>A CDR encoder, which writes primitives with the required alignment.</summary>
public sealed class CdrWriter : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an encoder with the encapsulation header already written.</summary>
    public CdrWriter() =>
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_cdr_writer_new(),
            NativeMethods.pamoja_cdr_writer_free,
            "CDR writer");

    /// <summary>Appends a 32-bit signed integer.</summary>
    /// <param name="value">The value to append.</param>
    public void WriteInt32(int value) => _handle.Use(w =>
        Status.ThrowIfError(NativeMethods.pamoja_cdr_writer_write_i32(w, value)));

    /// <summary>Appends a 32-bit unsigned integer.</summary>
    /// <param name="value">The value to append.</param>
    public void WriteUInt32(uint value) => _handle.Use(w =>
        Status.ThrowIfError(NativeMethods.pamoja_cdr_writer_write_u32(w, value)));

    /// <summary>Appends a 32-bit float.</summary>
    /// <param name="value">The value to append.</param>
    public void WriteSingle(float value) => _handle.Use(w =>
        Status.ThrowIfError(NativeMethods.pamoja_cdr_writer_write_f32(w, value)));

    /// <summary>Appends a 64-bit float.</summary>
    /// <param name="value">The value to append.</param>
    public void WriteDouble(double value) => _handle.Use(w =>
        Status.ThrowIfError(NativeMethods.pamoja_cdr_writer_write_f64(w, value)));

    /// <summary>Takes the encoded bytes, leaving the encoder spent.</summary>
    /// <remarks>
    /// The native call consumes the encoder, so this hands ownership over and the
    /// handle must not be used again.
    /// </remarks>
    /// <returns>The encoded bytes.</returns>
    /// <exception cref="PamojaException">The encoder was already spent.</exception>
    public byte[] ToBytes()
    {
        IntPtr writer = _handle.DangerousGetHandle();
        _handle.SetHandleAsInvalid();
        return NativeBuffer.Read(NativeMethods.pamoja_cdr_writer_into_bytes(writer));
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>A CDR decoder, which reads primitives back in the order written.</summary>
/// <remarks>
/// Reading past the end returns <c>null</c> rather than throwing, because a short
/// buffer is a wire condition rather than a programming error.
/// </remarks>
public sealed class CdrReader : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a decoder over encoded bytes.</summary>
    /// <param name="data">The encoded bytes.</param>
    /// <exception cref="PamojaException">
    /// The bytes carry no valid CDR encapsulation header.
    /// </exception>
    public CdrReader(ReadOnlySpan<byte> data) =>
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_cdr_reader_new(data, (nuint)data.Length),
            NativeMethods.pamoja_cdr_reader_free,
            "CDR reader");

    /// <summary>Reads the next 32-bit signed integer.</summary>
    /// <returns>The value, or <c>null</c> once exhausted.</returns>
    public int? ReadInt32() => _handle.Use(r =>
        NativeMethods.pamoja_cdr_reader_read_i32(r, out int value) == PamojaStatus.Ok
            ? value
            : (int?)null);

    /// <summary>Reads the next 32-bit unsigned integer.</summary>
    /// <returns>The value, or <c>null</c> once exhausted.</returns>
    public uint? ReadUInt32() => _handle.Use(r =>
        NativeMethods.pamoja_cdr_reader_read_u32(r, out uint value) == PamojaStatus.Ok
            ? value
            : (uint?)null);

    /// <summary>Reads the next 32-bit float.</summary>
    /// <returns>The value, or <c>null</c> once exhausted.</returns>
    public float? ReadSingle() => _handle.Use(r =>
        NativeMethods.pamoja_cdr_reader_read_f32(r, out float value) == PamojaStatus.Ok
            ? value
            : (float?)null);

    /// <summary>Reads the next 64-bit float.</summary>
    /// <returns>The value, or <c>null</c> once exhausted.</returns>
    public double? ReadDouble() => _handle.Use(r =>
        NativeMethods.pamoja_cdr_reader_read_f64(r, out double value) == PamojaStatus.Ok
            ? value
            : (double?)null);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Reads and releases the owned byte buffers the C ABI produces.</summary>
internal static class NativeBuffer
{
    /// <summary>Copies an owned buffer out and releases it.</summary>
    /// <param name="buffer">The native buffer handle.</param>
    /// <returns>The bytes.</returns>
    /// <exception cref="PamojaException">The native call produced no buffer.</exception>
    public static byte[] Read(IntPtr buffer)
    {
        if (buffer == IntPtr.Zero)
        {
            throw new PamojaException(Status.LastError() ?? "the call produced no buffer");
        }

        try
        {
            int length = checked((int)NativeMethods.pamoja_buffer_len(buffer));
            byte[] bytes = new byte[length];
            System.Runtime.InteropServices.Marshal.Copy(
                NativeMethods.pamoja_buffer_data(buffer), bytes, 0, length);
            return bytes;
        }
        finally
        {
            NativeMethods.pamoja_buffer_free(buffer);
        }
    }
}
