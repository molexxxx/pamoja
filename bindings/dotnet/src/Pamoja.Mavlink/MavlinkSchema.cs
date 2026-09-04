using System.Runtime.InteropServices;
using System.Text;
using Pamoja.Native.Interop;

namespace Pamoja.Mavlink;

/// <summary>The field types a message definition uses.</summary>
public enum MavlinkFieldType : uint
{
    /// <summary><c>uint8_t</c>.</summary>
    UInt8 = 1,

    /// <summary><c>int8_t</c>.</summary>
    Int8 = 2,

    /// <summary><c>char</c>; an array of these carries text.</summary>
    Char = 3,

    /// <summary><c>uint16_t</c>.</summary>
    UInt16 = 4,

    /// <summary><c>int16_t</c>.</summary>
    Int16 = 5,

    /// <summary><c>uint32_t</c>.</summary>
    UInt32 = 6,

    /// <summary><c>int32_t</c>.</summary>
    Int32 = 7,

    /// <summary><c>uint64_t</c>.</summary>
    UInt64 = 8,

    /// <summary><c>int64_t</c>.</summary>
    Int64 = 9,

    /// <summary><c>float</c>.</summary>
    Float = 10,

    /// <summary><c>double</c>.</summary>
    Double = 11,
}

/// <summary>One field of a message shape.</summary>
/// <param name="Name">The field name as the dialect writes it.</param>
/// <param name="TypeName">The field's type name as the dialect writes it.</param>
/// <param name="FieldType">The field's type.</param>
/// <param name="ArrayLen">The element count for an array field, or <c>0</c> for a scalar.</param>
/// <param name="Extension">Whether this is a MAVLink 2 extension field.</param>
/// <param name="Offset">The field's byte offset within the payload.</param>
public readonly record struct MavlinkFieldInfo(
    string Name,
    string TypeName,
    MavlinkFieldType FieldType,
    byte ArrayLen,
    bool Extension,
    int Offset);

/// <summary>
/// The shape of one message: its id, name, seed, and fields.
/// </summary>
/// <remarks>
/// <see cref="MavlinkFrame"/> carries any message's bytes, which is enough to move
/// traffic but leaves the caller hand-packing payloads against a message definition.
/// A schema is the layer above: it states a message's fields, so a
/// <see cref="MavlinkMessage"/> works in <c>custom_mode</c> and <c>lat</c> rather than
/// byte offsets. Every message the engine types is published, and a message from
/// ArduPilot's dialect, PX4's, or a vendor's private one is described through
/// <see cref="MavlinkSchemaBuilder"/>.
/// </remarks>
public sealed class MavlinkSchema : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a schema the native core produced.</summary>
    /// <param name="handle">The schema pointer.</param>
    internal MavlinkSchema(IntPtr handle) =>
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_mavlink_schema_free, "schema");

    /// <summary>The native pointer, for the calls that consult this schema.</summary>
    internal IntPtr Handle => _handle.DangerousGetHandle();

    /// <summary>Returns the shape of a message the engine types, by id.</summary>
    /// <param name="msgid">The message id to look up.</param>
    /// <returns>The shape.</returns>
    /// <exception cref="PamojaException">
    /// This build does not type that id, which is what
    /// <see cref="MavlinkSchemaBuilder"/> is for.
    /// </exception>
    public static MavlinkSchema ForId(uint msgid)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_schema_for_id(msgid, out IntPtr schema));
        return new MavlinkSchema(schema);
    }

    /// <summary>Returns the shape of a message the engine types, by name.</summary>
    /// <param name="name">The message name, such as <c>GLOBAL_POSITION_INT</c>.</param>
    /// <returns>The shape.</returns>
    /// <exception cref="PamojaException">This build does not type that name.</exception>
    public static MavlinkSchema ForName(string name)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_schema_for_name(name, out IntPtr schema));
        return new MavlinkSchema(schema);
    }

    /// <summary>Returns the names of every message this build types, in id order.</summary>
    /// <returns>The message names, each usable with <see cref="ForName"/>.</returns>
    public static IReadOnlyList<string> KnownMessages()
    {
        int count = (int)NativeMethods.pamoja_mavlink_schema_count();
        List<string> names = new(count);
        for (int index = 0; index < count; index += 1)
        {
            Status.ThrowIfError(
                NativeMethods.pamoja_mavlink_schema_at((nuint)index, out IntPtr schema));
            using MavlinkSchema held = new(schema);
            names.Add(held.Name);
        }

        return names;
    }

    /// <summary>The id of the message this schema describes.</summary>
    public uint MessageId => NativeMethods.pamoja_mavlink_schema_id(Handle);

    /// <summary>The name of the message this schema describes.</summary>
    public string Name =>
        Marshal.PtrToStringUTF8(NativeMethods.pamoja_mavlink_schema_name(Handle)) ?? string.Empty;

    /// <summary>The seed a frame carrying this message folds into its checksum.</summary>
    public byte CrcExtra => NativeMethods.pamoja_mavlink_schema_crc_extra(Handle);

    /// <summary>The length of the message on the wire, in bytes, extensions included.</summary>
    public int WireLength => (int)NativeMethods.pamoja_mavlink_schema_wire_len(Handle);

    /// <summary>The fields in wire order: the base fields largest first, then extensions.</summary>
    public IReadOnlyList<MavlinkFieldInfo> Fields
    {
        get
        {
            int count = (int)NativeMethods.pamoja_mavlink_schema_field_count(Handle);
            List<MavlinkFieldInfo> fields = new(count);
            for (int index = 0; index < count; index += 1)
            {
                Status.ThrowIfError(
                    NativeMethods.pamoja_mavlink_schema_field(
                        Handle,
                        (nuint)index,
                        out PamojaMavlinkFieldInfo described));
                fields.Add(new MavlinkFieldInfo(
                    Marshal.PtrToStringUTF8(described.Name) ?? string.Empty,
                    Marshal.PtrToStringUTF8(described.TypeName) ?? string.Empty,
                    (MavlinkFieldType)described.FieldType,
                    described.ArrayLen,
                    described.Extension != 0,
                    (int)described.Offset));
            }

            return fields;
        }
    }

    /// <summary>Creates a message with every field zero.</summary>
    /// <returns>The zeroed message, ready for its fields to be set.</returns>
    /// <exception cref="PamojaException">The shape does not fit a MAVLink payload.</exception>
    public MavlinkMessage CreateMessage()
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_new(Handle, out IntPtr message));
        return new MavlinkMessage(message);
    }

    /// <summary>Reads a message out of a frame payload.</summary>
    /// <param name="payload">The frame payload.</param>
    /// <returns>The decoded message.</returns>
    /// <exception cref="PamojaException">
    /// The payload is longer than this shape describes.
    /// </exception>
    /// <remarks>
    /// A shorter payload is zero-extended, as MAVLink 2 truncation requires, so a frame
    /// from a peer that trimmed trailing zeros or predates an extension field decodes.
    /// </remarks>
    public MavlinkMessage Decode(ReadOnlySpan<byte> payload)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_decode(
                Handle,
                payload,
                (nuint)payload.Length,
                out IntPtr message));
        return new MavlinkMessage(message);
    }

    /// <summary>Releases the schema.</summary>
    public void Dispose() => _handle.Dispose();
}

/// <summary>
/// Describes a message this build does not type, one field at a time.
/// </summary>
/// <remarks>
/// Fields are added in the order the message definition lists them;
/// <see cref="Build"/> puts them in wire order and derives the <c>CRC_EXTRA</c> seed
/// from the result, so a caller transcribes a definition as it reads.
/// </remarks>
public sealed class MavlinkSchemaBuilder : IDisposable
{
    private NativeHandle? _handle;

    /// <summary>Starts describing a message.</summary>
    /// <param name="msgid">The message id on the wire.</param>
    /// <param name="name">
    /// The message name, which the seed derivation folds in, so it must match the
    /// dialect exactly.
    /// </param>
    public MavlinkSchemaBuilder(uint msgid, string name) =>
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_mavlink_schema_builder_new(msgid, name),
            NativeMethods.pamoja_mavlink_schema_builder_free,
            "schema builder");

    /// <summary>Adds a base field, in the order the definition declares it.</summary>
    /// <param name="name">The field name.</param>
    /// <param name="fieldType">The field's type.</param>
    /// <param name="arrayLen">The element count for an array, or <c>0</c> for a scalar.</param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The shape has already been built.</exception>
    public MavlinkSchemaBuilder Field(string name, MavlinkFieldType fieldType, byte arrayLen = 0)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_schema_builder_field(
                Live(),
                name,
                (uint)fieldType,
                arrayLen));
        return this;
    }

    /// <summary>Adds a MAVLink 2 extension field, in declared order.</summary>
    /// <param name="name">The field name.</param>
    /// <param name="fieldType">The field's type.</param>
    /// <param name="arrayLen">The element count for an array, or <c>0</c> for a scalar.</param>
    /// <returns>This builder, so calls chain.</returns>
    /// <exception cref="PamojaException">The shape has already been built.</exception>
    /// <remarks>
    /// Extensions keep their declared order, stay out of the seed, and read as zero from
    /// a frame sent by a peer that predates them.
    /// </remarks>
    public MavlinkSchemaBuilder Extension(
        string name,
        MavlinkFieldType fieldType,
        byte arrayLen = 0)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_schema_builder_extension(
                Live(),
                name,
                (uint)fieldType,
                arrayLen));
        return this;
    }

    /// <summary>Puts the declared fields in wire order and finishes the shape.</summary>
    /// <returns>The finished schema.</returns>
    /// <exception cref="PamojaException">
    /// Two fields share a name, the fields do not fit a MAVLink payload, or the shape has
    /// already been built.
    /// </exception>
    public MavlinkSchema Build()
    {
        IntPtr builder = Live();

        // The native call consumes the builder whether or not the shape is valid, so the
        // wrapper stops owning it before the status is checked.
        _handle!.SetHandleAsInvalid();
        _handle = null;

        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_schema_builder_build(builder, out IntPtr schema));
        return new MavlinkSchema(schema);
    }

    /// <summary>Releases a builder that was never built.</summary>
    public void Dispose()
    {
        _handle?.Dispose();
        _handle = null;
    }

    private IntPtr Live()
    {
        if (_handle is null)
        {
            throw new PamojaException("this builder has already been built");
        }

        return _handle.DangerousGetHandle();
    }
}

/// <summary>
/// A message read and written by field name against a <see cref="MavlinkSchema"/>.
/// </summary>
public sealed class MavlinkMessage : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a message the native core produced.</summary>
    /// <param name="handle">The message pointer.</param>
    internal MavlinkMessage(IntPtr handle) =>
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_mavlink_message_free, "message");

    private IntPtr Handle => _handle.DangerousGetHandle();

    /// <summary>The message's bytes as they go on the wire.</summary>
    public byte[] Payload
    {
        get
        {
            IntPtr bytes = NativeMethods.pamoja_mavlink_message_payload(Handle, out nuint length);
            if (bytes == IntPtr.Zero)
            {
                return [];
            }

            byte[] payload = new byte[(int)length];
            Marshal.Copy(bytes, payload, 0, payload.Length);
            return payload;
        }
    }

    /// <summary>Builds a v2 frame carrying this message.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <returns>The frame ready to send.</returns>
    /// <exception cref="PamojaException">The message does not fit a frame.</exception>
    public MavlinkFrame ToFrame(MavlinkHeader header)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_to_frame(
                Handle,
                header.ToNative(),
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Reads a field as a double, whatever its type.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="index">The element to read, or <c>0</c> for a scalar field.</param>
    /// <returns>The value.</returns>
    /// <exception cref="PamojaException">
    /// The message has no such field, or the element is past the end of an array.
    /// </exception>
    /// <remarks>
    /// An integer field wider than 53 bits can exceed what a double holds exactly, so read
    /// those with <see cref="GetInt64"/> or <see cref="GetUInt64"/> where the exact value
    /// matters.
    /// </remarks>
    public double Get(string field, int index = 0)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_get_number(
                Handle,
                field,
                (nuint)index,
                out double value));
        return value;
    }

    /// <summary>Reads an integer field exactly, whatever its width or sign.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="index">The element to read, or <c>0</c> for a scalar field.</param>
    /// <returns>The value.</returns>
    /// <exception cref="PamojaException">
    /// The message has no such field, the element is past the end of an array, the field is
    /// floating-point, or a <c>uint64_t</c> value is above the signed range.
    /// </exception>
    public long GetInt64(string field, int index = 0)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_get_int(
                Handle,
                field,
                (nuint)index,
                out long value));
        return value;
    }

    /// <summary>Reads an unsigned integer field exactly.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="index">The element to read, or <c>0</c> for a scalar field.</param>
    /// <returns>The value.</returns>
    /// <exception cref="PamojaException">
    /// The message has no such field, the element is past the end of an array, the field is
    /// floating-point, or the value is negative.
    /// </exception>
    public ulong GetUInt64(string field, int index = 0)
    {
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_get_uint(
                Handle,
                field,
                (nuint)index,
                out ulong value));
        return value;
    }

    /// <summary>Writes a double into a field, converting it to the field's type.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="value">The value to store.</param>
    /// <param name="index">The element to write, or <c>0</c> for a scalar field.</param>
    /// <exception cref="PamojaException">
    /// The message has no such field, the element is past the end of an array, or an
    /// integer field cannot hold the value exactly.
    /// </exception>
    /// <remarks>
    /// A value bound for an integer field must be a whole number within that field's range,
    /// so a fractional or oversized value is refused rather than silently truncated.
    /// </remarks>
    public void Set(string field, double value, int index = 0) =>
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_set_number(
                Handle,
                field,
                (nuint)index,
                value));

    /// <summary>Writes an integer into a field exactly, whatever its width or sign.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="value">The value to store.</param>
    /// <param name="index">The element to write, or <c>0</c> for a scalar field.</param>
    /// <exception cref="PamojaException">
    /// The message has no such field, the element is past the end of an array, the field is
    /// floating-point, or the value does not fit the field's type.
    /// </exception>
    public void SetInt64(string field, long value, int index = 0) =>
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_set_int(
                Handle,
                field,
                (nuint)index,
                value));

    /// <summary>Writes an unsigned integer into a field exactly.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="value">The value to store.</param>
    /// <param name="index">The element to write, or <c>0</c> for a scalar field.</param>
    /// <exception cref="PamojaException">
    /// The message has no such field, the element is past the end of an array, the field is
    /// floating-point, or the value does not fit the field's type.
    /// </exception>
    public void SetUInt64(string field, ulong value, int index = 0) =>
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_set_uint(
                Handle,
                field,
                (nuint)index,
                value));

    /// <summary>Copies the raw bytes of a byte-wide array field out.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="length">The field's declared length, from its schema.</param>
    /// <returns>The bytes, including the zero padding.</returns>
    /// <exception cref="PamojaException">
    /// The message has no such field, or it is not a byte-wide array.
    /// </exception>
    public byte[] GetBytes(string field, int length)
    {
        byte[] bytes = new byte[length];
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_get_bytes(
                Handle,
                field,
                bytes,
                (nuint)bytes.Length));
        return bytes;
    }

    /// <summary>Writes the raw bytes of a byte-wide array field, zero-padding the rest.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="bytes">The bytes to store, at most the field's declared length.</param>
    /// <exception cref="PamojaException">
    /// The message has no such field, it is not a byte-wide array, or the bytes are longer
    /// than the field.
    /// </exception>
    public void SetBytes(string field, ReadOnlySpan<byte> bytes) =>
        Status.ThrowIfError(
            NativeMethods.pamoja_mavlink_message_set_bytes(
                Handle,
                field,
                bytes,
                (nuint)bytes.Length));

    /// <summary>Reads a <c>char</c> array as text, stopping at the padding.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="length">The field's declared length, from its schema.</param>
    /// <returns>The text, without its padding.</returns>
    /// <exception cref="PamojaException">
    /// The message has no such field, or it is not a byte-wide array.
    /// </exception>
    /// <remarks>
    /// MAVLink carries a string in a fixed-length array, padded with zeros when the text is
    /// shorter and left unterminated when it exactly fills the field.
    /// </remarks>
    public string GetText(string field, int length)
    {
        byte[] bytes = GetBytes(field, length);
        int end = Array.IndexOf(bytes, (byte)0);
        return Encoding.UTF8.GetString(bytes, 0, end < 0 ? bytes.Length : end);
    }

    /// <summary>Writes text into a <c>char</c> array, padding the rest with zeros.</summary>
    /// <param name="field">The field name.</param>
    /// <param name="text">The text to store, at most the field's declared length.</param>
    /// <exception cref="PamojaException">
    /// The message has no such field, it is not a byte-wide array, or the text is longer
    /// than the field.
    /// </exception>
    public void SetText(string field, string text) =>
        SetBytes(field, Encoding.UTF8.GetBytes(text));

    /// <summary>Releases the message.</summary>
    public void Dispose() => _handle.Dispose();
}
