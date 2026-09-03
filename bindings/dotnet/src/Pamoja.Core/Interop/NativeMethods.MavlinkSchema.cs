using System.Runtime.InteropServices;

namespace Pamoja.Core.Interop;

/// <summary>
/// One field of a message shape, mirroring <c>PamojaMavlinkFieldInfo</c> in
/// <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// Both names point into the schema they came from and stay valid until it is
/// released, so a caller reads them in place rather than freeing them.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaMavlinkFieldInfo
{
    /// <summary>The field name as the dialect writes it.</summary>
    public IntPtr Name;

    /// <summary>The field's type name as the dialect writes it.</summary>
    public IntPtr TypeName;

    /// <summary>The field's type, one of the field type codes.</summary>
    public uint FieldType;

    /// <summary>The element count for an array field, or <c>0</c> for a scalar.</summary>
    public byte ArrayLen;

    /// <summary><c>1</c> for a MAVLink 2 extension field, <c>0</c> for a base field.</summary>
    public byte Extension;

    /// <summary>The field's byte offset within the payload.</summary>
    public nuint Offset;
}

/// <summary>
/// The P/Invoke declarations for MAVLink message shapes, mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>Returns the shape of a message the engine types, by id.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_schema_for_id(
        uint msgid,
        out IntPtr outSchema);

    /// <summary>Returns the shape of a message the engine types, by name.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_schema_for_name(
        string name,
        out IntPtr outSchema);

    /// <summary>Returns how many messages this build types.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_mavlink_schema_count();

    /// <summary>Returns the shape at a position in the built-in registry.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_schema_at(
        nuint index,
        out IntPtr outSchema);

    /// <summary>Returns the id of the message a schema describes.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_mavlink_schema_id(IntPtr schema);

    /// <summary>Returns the name of the message a schema describes.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mavlink_schema_name(IntPtr schema);

    /// <summary>Returns the CRC_EXTRA seed a schema implies.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mavlink_schema_crc_extra(IntPtr schema);

    /// <summary>Returns the length of the message on the wire, in bytes.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_mavlink_schema_wire_len(IntPtr schema);

    /// <summary>Returns how many fields a message has.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_mavlink_schema_field_count(IntPtr schema);

    /// <summary>Describes one field of a message.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_schema_field(
        IntPtr schema,
        nuint index,
        out PamojaMavlinkFieldInfo outField);

    /// <summary>Releases a schema.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_schema_free(IntPtr schema);

    /// <summary>Adds a schema's message to a dialect table.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_dialect_add_schema(
        IntPtr dialect,
        IntPtr schema);

    /// <summary>Starts describing a message.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial IntPtr pamoja_mavlink_schema_builder_new(uint msgid, string name);

    /// <summary>Adds a base field, in the order the definition declares it.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_schema_builder_field(
        IntPtr builder,
        string name,
        uint fieldType,
        byte arrayLen);

    /// <summary>Adds a MAVLink 2 extension field, in declared order.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_schema_builder_extension(
        IntPtr builder,
        string name,
        uint fieldType,
        byte arrayLen);

    /// <summary>Puts the declared fields in wire order and finishes the shape.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_schema_builder_build(
        IntPtr builder,
        out IntPtr outSchema);

    /// <summary>Releases a builder that was never built.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_schema_builder_free(IntPtr builder);

    /// <summary>Creates a message with every field zero.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_message_new(
        IntPtr schema,
        out IntPtr outMessage);

    /// <summary>Reads a message out of a frame payload.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_message_decode(
        IntPtr schema,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out IntPtr outMessage);

    /// <summary>Returns a pointer to a message's payload bytes.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mavlink_message_payload(
        IntPtr message,
        out nuint outLen);

    /// <summary>Builds a v2 frame carrying a message.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_message_to_frame(
        IntPtr message,
        PamojaMavlinkHeader header,
        out IntPtr outFrame);

    /// <summary>Releases a message.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_message_free(IntPtr message);

    /// <summary>Reads a field as a signed integer.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_get_int(
        IntPtr message,
        string field,
        nuint index,
        out long outValue);

    /// <summary>Reads a field as an unsigned integer.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_get_uint(
        IntPtr message,
        string field,
        nuint index,
        out ulong outValue);

    /// <summary>Reads a floating-point field.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_get_float(
        IntPtr message,
        string field,
        nuint index,
        out double outValue);

    /// <summary>Reads a field as a double, whatever its type.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_get_number(
        IntPtr message,
        string field,
        nuint index,
        out double outValue);

    /// <summary>Writes a signed integer into a field.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_set_int(
        IntPtr message,
        string field,
        nuint index,
        long value);

    /// <summary>Writes an unsigned integer into a field.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_set_uint(
        IntPtr message,
        string field,
        nuint index,
        ulong value);

    /// <summary>Writes a floating-point field.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_set_float(
        IntPtr message,
        string field,
        nuint index,
        double value);

    /// <summary>Writes a double into a field, converting it to the field's type.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_set_number(
        IntPtr message,
        string field,
        nuint index,
        double value);

    /// <summary>Copies the raw bytes of a byte-wide array field out.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_get_bytes(
        IntPtr message,
        string field,
        Span<byte> outBytes,
        nuint outBytesLen);

    /// <summary>Writes the raw bytes of a byte-wide array field.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_set_bytes(
        IntPtr message,
        string field,
        ReadOnlySpan<byte> bytes,
        nuint bytesLen);
}
