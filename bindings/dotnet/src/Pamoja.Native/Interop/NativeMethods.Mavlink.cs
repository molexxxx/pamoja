using System.Runtime.InteropServices;

namespace Pamoja.Native.Interop;

/// <summary>
/// The addressing fields a sender stamps on every frame, mirroring
/// <c>PamojaMavlinkHeader</c> in <c>pamoja.h</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaMavlinkHeader
{
    /// <summary>The sending system's id.</summary>
    public byte SystemId;

    /// <summary>The sending component's id.</summary>
    public byte ComponentId;

    /// <summary>The sender's sequence number, which wraps at 256.</summary>
    public byte Sequence;
}

/// <summary>
/// One field of a message definition, mirroring <c>PamojaMavlinkField</c> in
/// <c>pamoja.h</c>.
/// </summary>
/// <remarks>
/// The two names are UTF-8 pointers the caller owns for the duration of the
/// call, so a facade allocates them, calls, and frees them again.
/// </remarks>
[StructLayout(LayoutKind.Sequential)]
public struct PamojaMavlinkField
{
    /// <summary>The field's type name as the dialect writes it.</summary>
    public IntPtr TypeName;

    /// <summary>The field's name as the dialect writes it.</summary>
    public IntPtr FieldName;

    /// <summary>The element count for an array field, or <c>0</c> for a scalar.</summary>
    public byte ArrayLen;
}

/// <summary>
/// The P/Invoke declarations for the MAVLink wire protocol, mirroring
/// <c>pamoja.h</c> one-to-one.
/// </summary>
/// <remarks>
/// Split from the other declarations only to keep each file readable; this is the
/// same <see cref="NativeMethods"/> class and the same low-level escape hatch.
/// Every part must be updated together with the generated header.
/// </remarks>
public static partial class NativeMethods
{
    /// <summary>The largest payload a frame can carry, in bytes.</summary>
    public const int MavlinkMaxPayload = 255;

    /// <summary>The largest frame, in bytes, header, checksum and signature included.</summary>
    public const int MavlinkMaxFrame = 280;

    /// <summary>The length of a v2 signature block, in bytes.</summary>
    public const int MavlinkSignatureLen = 13;

    /// <summary>The length of a signing key, in bytes.</summary>
    public const int MavlinkKeyLen = 32;

    /// <summary>The default window a verifier accepts a timestamp within.</summary>
    public const ulong MavlinkDefaultTimestampWindow = 6_000_000;

    /// <summary>The original wire format, with a six-byte header.</summary>
    public const byte MavlinkVersionV1 = 1;

    /// <summary>The current wire format, with a 24-bit message id and signing.</summary>
    public const byte MavlinkVersionV2 = 2;

    /// <summary>Returns the CRC-16/MCRF4XX checksum of a byte string.</summary>
    [LibraryImport(Library)]
    public static partial ushort pamoja_mavlink_crc16_mcrf4xx(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen);

    /// <summary>Derives the CRC_EXTRA seed of a message from its definition.</summary>
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    public static partial PamojaStatus pamoja_mavlink_message_crc_extra(
        string name,
        ReadOnlySpan<PamojaMavlinkField> fields,
        nuint fieldCount,
        out byte outCrcExtra);

    /// <summary>Returns the CRC_EXTRA the common dialect publishes for a message id.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_known_crc_extra(
        uint msgid,
        out byte outCrcExtra);

    /// <summary>Creates an empty dialect table.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mavlink_dialect_new();

    /// <summary>Adds or replaces the seed for a message id.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_dialect_add(
        IntPtr dialect,
        uint msgid,
        byte crcExtra);

    /// <summary>Returns the seed a dialect resolves a message id to.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_dialect_crc_extra(
        IntPtr dialect,
        uint msgid,
        out byte outCrcExtra);

    /// <summary>Releases a dialect table.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_dialect_free(IntPtr dialect);

    /// <summary>Assembles a frame carrying a message.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_frame_encode(
        byte version,
        PamojaMavlinkHeader header,
        uint msgid,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        byte crcExtra,
        out IntPtr outFrame);

    /// <summary>Parses one frame against a known CRC_EXTRA.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_frame_parse(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        byte crcExtra,
        out IntPtr outFrame);

    /// <summary>Parses one frame, looking its CRC_EXTRA up as it goes.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_frame_parse_known(
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        IntPtr dialect,
        out IntPtr outFrame);

    /// <summary>Assembles a v2 frame carrying a message this build does not type.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_raw_message_to_frame(
        PamojaMavlinkHeader header,
        uint msgid,
        byte crcExtra,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        out IntPtr outFrame);

    /// <summary>Releases a frame.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_frame_free(IntPtr frame);

    /// <summary>Returns the wire format a frame uses.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mavlink_frame_version(IntPtr frame);

    /// <summary>Returns the addressing fields a frame carries.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_frame_header(
        IntPtr frame,
        out PamojaMavlinkHeader outHeader);

    /// <summary>Returns the id of the message a frame carries.</summary>
    [LibraryImport(Library)]
    public static partial uint pamoja_mavlink_frame_message_id(IntPtr frame);

    /// <summary>Returns the incompatibility flags a v2 frame declares.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mavlink_frame_incompat_flags(IntPtr frame);

    /// <summary>Reports whether a frame carries a signature.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mavlink_frame_is_signed(IntPtr frame);

    /// <summary>Returns a pointer to a frame's payload.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mavlink_frame_payload(IntPtr frame, out nuint outLen);

    /// <summary>Returns a pointer to a frame's bytes.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mavlink_frame_bytes(IntPtr frame, out nuint outLen);

    /// <summary>Copies a frame's signature block out.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_frame_signature(
        IntPtr frame,
        Span<byte> outSignature);

    /// <summary>Creates a parser with an empty buffer.</summary>
    [LibraryImport(Library)]
    public static partial IntPtr pamoja_mavlink_parser_new();

    /// <summary>Feeds bytes off a link into the parser.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_parser_push(
        IntPtr parser,
        ReadOnlySpan<byte> bytes,
        nuint bytesLen,
        IntPtr dialect);

    /// <summary>Takes the next completed frame out of the parser.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_parser_next(
        IntPtr parser,
        out IntPtr outFrame);

    /// <summary>Returns how many completed frames are waiting.</summary>
    [LibraryImport(Library)]
    public static partial nuint pamoja_mavlink_parser_pending(IntPtr parser);

    /// <summary>Releases a parser.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_parser_free(IntPtr parser);

    /// <summary>Converts Unix time into the timestamp MAVLink signing counts in.</summary>
    [LibraryImport(Library)]
    public static partial ulong pamoja_mavlink_timestamp_from_unix_micros(ulong unixMicros);

    /// <summary>Creates a signer.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_signer_new(
        ReadOnlySpan<byte> key,
        byte linkId,
        ulong timestamp,
        out IntPtr outSigner);

    /// <summary>Signs a message into a v2 frame.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_signer_sign(
        IntPtr signer,
        PamojaMavlinkHeader header,
        uint msgid,
        ReadOnlySpan<byte> payload,
        nuint payloadLen,
        byte crcExtra,
        out IntPtr outFrame);

    /// <summary>Returns the link a signer signs on.</summary>
    [LibraryImport(Library)]
    public static partial byte pamoja_mavlink_signer_link_id(IntPtr signer);

    /// <summary>Releases a signer.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_signer_free(IntPtr signer);

    /// <summary>Creates a verifier.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_verifier_new(
        ReadOnlySpan<byte> key,
        out IntPtr outVerifier);

    /// <summary>Sets how far a timestamp may run ahead of the last accepted.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_verifier_set_window(
        IntPtr verifier,
        ulong window);

    /// <summary>Checks a frame's signature and its place in the sequence.</summary>
    [LibraryImport(Library)]
    public static partial PamojaStatus pamoja_mavlink_verifier_verify(
        IntPtr verifier,
        IntPtr frame);

    /// <summary>Releases a verifier.</summary>
    [LibraryImport(Library)]
    public static partial void pamoja_mavlink_verifier_free(IntPtr verifier);
}
