using System.Runtime.InteropServices;
using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>Which MAVLink wire format a frame uses.</summary>
public enum MavlinkVersion
{
    /// <summary>The original six-byte-header format.</summary>
    V1 = 1,

    /// <summary>The current format: a 24-bit message id, flags, and optional signing.</summary>
    V2 = 2,
}

/// <summary>The addressing fields a sender stamps on every frame.</summary>
/// <param name="SystemId">The sending system's id.</param>
/// <param name="ComponentId">The sending component's id.</param>
/// <param name="Sequence">The sender's sequence number, which wraps at 256.</param>
/// <remarks>
/// A frame says who sent it and where it sits in that sender's stream, so a
/// receiver can tell a dropped frame from a quiet link.
/// </remarks>
public readonly record struct MavlinkHeader(byte SystemId, byte ComponentId, byte Sequence = 0)
{
    /// <summary>Converts the header into the shape that crosses the boundary.</summary>
    /// <returns>The equivalent struct.</returns>
    internal PamojaMavlinkHeader ToNative() => new()
    {
        SystemId = SystemId,
        ComponentId = ComponentId,
        Sequence = Sequence,
    };
}

/// <summary>One field of a message definition, as the CRC_EXTRA derivation reads it.</summary>
/// <param name="TypeName">The field's type name as the dialect writes it, such as <c>uint8_t</c>.</param>
/// <param name="FieldName">The field's name as the dialect writes it.</param>
/// <param name="ArrayLen">The element count for an array field, or <c>0</c> for a scalar.</param>
public readonly record struct MavlinkField(string TypeName, string FieldName, byte ArrayLen = 0);

/// <summary>The MAVLink wire protocol: checksums, seeds, and clocks.</summary>
/// <remarks>
/// MAVLink is the language drones speak, so talking to a PX4 or ArduPilot
/// autopilot means putting exactly the right bytes on the wire and trusting the
/// bytes that come back.
/// <para>
/// Nothing here is limited to the messages this build happens to know. The
/// common dialect's seeds are built in, and <see cref="MavlinkDialect"/> carries
/// any others, derived by <see cref="MessageCrcExtra"/> the way the
/// specification does.
/// </para>
/// </remarks>
public static class Mavlink
{
    /// <summary>The largest payload a frame can carry, in bytes.</summary>
    public const int MaxPayload = NativeMethods.MavlinkMaxPayload;

    /// <summary>The largest frame, in bytes, header, checksum and signature included.</summary>
    public const int MaxFrame = NativeMethods.MavlinkMaxFrame;

    /// <summary>The length of a v2 signature block, in bytes.</summary>
    public const int SignatureLength = NativeMethods.MavlinkSignatureLen;

    /// <summary>The length of a signing key, in bytes.</summary>
    public const int KeyLength = NativeMethods.MavlinkKeyLen;

    /// <summary>The default window a verifier accepts a timestamp within.</summary>
    public const ulong DefaultTimestampWindow = NativeMethods.MavlinkDefaultTimestampWindow;

    /// <summary>Returns the CRC-16/MCRF4XX checksum of a byte string.</summary>
    /// <param name="bytes">The data to checksum.</param>
    /// <returns>The checksum.</returns>
    /// <remarks>
    /// This is the checksum every MAVLink frame carries, exposed because a host
    /// that implements part of the protocol itself needs the same arithmetic.
    /// </remarks>
    public static ushort Crc16(ReadOnlySpan<byte> bytes) =>
        NativeMethods.pamoja_mavlink_crc16_mcrf4xx(bytes, (nuint)bytes.Length);

    /// <summary>Derives the CRC_EXTRA seed of a message from its definition.</summary>
    /// <param name="name">The message name, such as <c>HEARTBEAT</c>.</param>
    /// <param name="fields">The base fields in wire order.</param>
    /// <returns>The seed.</returns>
    /// <exception cref="PamojaException">A name was not valid UTF-8.</exception>
    /// <remarks>
    /// This is what makes a dialect this build has never seen usable: the seed
    /// comes out the same as the one the dialect publishes, and a frame carrying
    /// that message then checks like any other. Extension fields are excluded
    /// from the seed and must not be listed.
    /// </remarks>
    public static byte MessageCrcExtra(string name, IReadOnlyList<MavlinkField> fields)
    {
        PamojaMavlinkField[] described = new PamojaMavlinkField[fields.Count];
        List<IntPtr> owned = new(fields.Count * 2);
        try
        {
            for (int index = 0; index < fields.Count; index++)
            {
                IntPtr typeName = Marshal.StringToCoTaskMemUTF8(fields[index].TypeName);
                IntPtr fieldName = Marshal.StringToCoTaskMemUTF8(fields[index].FieldName);
                owned.Add(typeName);
                owned.Add(fieldName);
                described[index] = new PamojaMavlinkField
                {
                    TypeName = typeName,
                    FieldName = fieldName,
                    ArrayLen = fields[index].ArrayLen,
                };
            }

            PamojaCore.ThrowIfError(
                NativeMethods.pamoja_mavlink_message_crc_extra(
                    name,
                    described,
                    (nuint)described.Length,
                    out byte crcExtra));
            return crcExtra;
        }
        finally
        {
            foreach (IntPtr pointer in owned)
            {
                Marshal.FreeCoTaskMem(pointer);
            }
        }
    }

    /// <summary>Returns the CRC_EXTRA the common dialect publishes for a message id.</summary>
    /// <param name="msgid">The message id to look up.</param>
    /// <returns>
    /// The seed, or <c>null</c> for an id outside the common dialect, which is
    /// what a <see cref="MavlinkDialect"/> is for.
    /// </returns>
    public static byte? KnownCrcExtra(uint msgid) =>
        NativeMethods.pamoja_mavlink_known_crc_extra(msgid, out byte crcExtra) == PamojaStatus.Ok
            ? crcExtra
            : null;

    /// <summary>Converts Unix time into the timestamp MAVLink signing counts in.</summary>
    /// <param name="unixMicros">The time in microseconds since the Unix epoch.</param>
    /// <returns>The signing timestamp, in units of ten microseconds since 2015.</returns>
    public static ulong TimestampFromUnixMicros(ulong unixMicros) =>
        NativeMethods.pamoja_mavlink_timestamp_from_unix_micros(unixMicros);

    /// <summary>Returns a signing timestamp for now.</summary>
    /// <returns>The signing timestamp matching the current clock.</returns>
    public static ulong TimestampNow() =>
        TimestampFromUnixMicros((ulong)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() * 1000);

    /// <summary>Builds a v2 frame carrying a message the common dialect defines.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <param name="msgid">The message id.</param>
    /// <param name="payload">The message payload.</param>
    /// <returns>The frame ready to send.</returns>
    /// <exception cref="PamojaException">
    /// The id is outside the common dialect, in which case build the frame with
    /// <see cref="MavlinkFrame.Raw"/> and a seed of your own.
    /// </exception>
    /// <remarks>
    /// The seed is looked up rather than passed, which is the usual case: a
    /// sender emitting a standard message should not have to know its checksum
    /// constant.
    /// </remarks>
    public static MavlinkFrame Frame(MavlinkHeader header, uint msgid, ReadOnlySpan<byte> payload)
    {
        byte? crcExtra = KnownCrcExtra(msgid);
        if (crcExtra is null)
        {
            throw new PamojaException(
                $"message {msgid} is not in the common dialect; supply its CRC_EXTRA with MavlinkFrame.Raw");
        }

        return MavlinkFrame.EncodeV2(header, msgid, payload, crcExtra.Value);
    }
}

/// <summary>The CRC_EXTRA seeds of a dialect beyond the common one.</summary>
/// <remarks>
/// A receiver must know a message's CRC_EXTRA before it can check the frame
/// carrying it. Entries added here are consulted before the built-in
/// common-dialect registry, so a private dialect may also override an id the
/// common one defines.
/// </remarks>
public sealed class MavlinkDialect : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an empty dialect table.</summary>
    public MavlinkDialect() =>
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_mavlink_dialect_new(),
            NativeMethods.pamoja_mavlink_dialect_free,
            "dialect");

    /// <summary>The native pointer, for the calls that consult this dialect.</summary>
    internal IntPtr Handle => _handle.DangerousGetHandle();

    /// <summary>Adds or replaces the seed for a message id.</summary>
    /// <param name="msgid">The message id.</param>
    /// <param name="crcExtra">The seed.</param>
    public void Add(uint msgid, byte crcExtra) =>
        PamojaCore.ThrowIfError(NativeMethods.pamoja_mavlink_dialect_add(Handle, msgid, crcExtra));

    /// <summary>Adds a message by its definition, deriving the seed.</summary>
    /// <param name="msgid">The message id.</param>
    /// <param name="name">The message name.</param>
    /// <param name="fields">The base fields in wire order.</param>
    /// <returns>The seed that was derived and added.</returns>
    /// <remarks>
    /// This is the whole path for a vendor dialect: describe the message once,
    /// and every frame carrying it checks from then on.
    /// </remarks>
    public byte AddMessage(uint msgid, string name, IReadOnlyList<MavlinkField> fields)
    {
        byte crcExtra = Mavlink.MessageCrcExtra(name, fields);
        Add(msgid, crcExtra);
        return crcExtra;
    }

    /// <summary>Returns the seed this dialect resolves a message id to.</summary>
    /// <param name="msgid">The message id to look up.</param>
    /// <returns>
    /// The seed, or <c>null</c> if neither this table nor the common dialect
    /// knows the id.
    /// </returns>
    public byte? CrcExtra(uint msgid) =>
        NativeMethods.pamoja_mavlink_dialect_crc_extra(Handle, msgid, out byte crcExtra)
            == PamojaStatus.Ok
            ? crcExtra
            : null;

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>One MAVLink frame, assembled or received.</summary>
public sealed class MavlinkFrame : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Wraps a frame the native core produced.</summary>
    /// <param name="handle">The frame pointer.</param>
    internal MavlinkFrame(IntPtr handle) =>
        _handle = NativeHandle.Create(handle, NativeMethods.pamoja_mavlink_frame_free, "frame");

    /// <summary>Assembles a v2 frame carrying a message.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <param name="msgid">The message id.</param>
    /// <param name="payload">The message payload.</param>
    /// <param name="crcExtra">The seed for this message id.</param>
    /// <returns>The frame ready to send.</returns>
    /// <exception cref="PamojaException">The payload does not fit a frame.</exception>
    /// <remarks>This is the current wire format and what a modern autopilot expects.</remarks>
    public static MavlinkFrame EncodeV2(
        MavlinkHeader header,
        uint msgid,
        ReadOnlySpan<byte> payload,
        byte crcExtra)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_frame_encode(
                NativeMethods.MavlinkVersionV2,
                header.ToNative(),
                msgid,
                payload,
                (nuint)payload.Length,
                crcExtra,
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Assembles a v1 frame, for a peer that predates MAVLink 2.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <param name="msgid">The message id; a v1 frame only carries ids below 256.</param>
    /// <param name="payload">The message payload.</param>
    /// <param name="crcExtra">The seed for this message id.</param>
    /// <returns>The frame ready to send.</returns>
    /// <exception cref="PamojaException">The id or payload does not fit a v1 frame.</exception>
    public static MavlinkFrame EncodeV1(
        MavlinkHeader header,
        uint msgid,
        ReadOnlySpan<byte> payload,
        byte crcExtra)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_frame_encode(
                NativeMethods.MavlinkVersionV1,
                header.ToNative(),
                msgid,
                payload,
                (nuint)payload.Length,
                crcExtra,
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Parses one frame, checking it against a known CRC_EXTRA.</summary>
    /// <param name="bytes">The frame as received.</param>
    /// <param name="crcExtra">The seed for the message the frame carries.</param>
    /// <returns>The parsed frame.</returns>
    /// <exception cref="PamojaException">
    /// The bytes are not a whole frame or the checksum does not match, which is
    /// what rejects a frame mangled in transit.
    /// </exception>
    public static MavlinkFrame Parse(ReadOnlySpan<byte> bytes, byte crcExtra)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_frame_parse(
                bytes,
                (nuint)bytes.Length,
                crcExtra,
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Parses one frame, looking its CRC_EXTRA up as it goes.</summary>
    /// <param name="bytes">The frame as received.</param>
    /// <param name="dialect">The dialect to prefer, or <c>null</c> for the common one.</param>
    /// <returns>The parsed frame.</returns>
    /// <exception cref="PamojaException">
    /// The bytes are not a whole frame, the checksum does not match, or no
    /// dialect here knows the message id.
    /// </exception>
    /// <remarks>
    /// This is what a receiver holding many message types uses: the id comes out
    /// of the frame, and the seed comes from the dialect or the common registry.
    /// </remarks>
    public static MavlinkFrame ParseKnown(ReadOnlySpan<byte> bytes, MavlinkDialect? dialect = null)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_frame_parse_known(
                bytes,
                (nuint)bytes.Length,
                dialect?.Handle ?? IntPtr.Zero,
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Assembles a v2 frame carrying a message this build does not type.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <param name="msgid">The message id.</param>
    /// <param name="crcExtra">The seed for this message id.</param>
    /// <param name="payload">The message payload.</param>
    /// <returns>The frame ready to send.</returns>
    /// <exception cref="PamojaException">The payload does not fit a frame.</exception>
    /// <remarks>
    /// The escape hatch a private dialect needs: supply the id, the payload, and
    /// the seed, and the frame is built and checked like any other.
    /// </remarks>
    public static MavlinkFrame Raw(
        MavlinkHeader header,
        uint msgid,
        byte crcExtra,
        ReadOnlySpan<byte> payload)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_raw_message_to_frame(
                header.ToNative(),
                msgid,
                crcExtra,
                payload,
                (nuint)payload.Length,
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>The native pointer, for the calls that read this frame.</summary>
    internal IntPtr Handle => _handle.DangerousGetHandle();

    /// <summary>Which wire format this frame uses.</summary>
    public MavlinkVersion Version =>
        (MavlinkVersion)NativeMethods.pamoja_mavlink_frame_version(Handle);

    /// <summary>The addressing fields the frame carries.</summary>
    public MavlinkHeader Header
    {
        get
        {
            PamojaCore.ThrowIfError(
                NativeMethods.pamoja_mavlink_frame_header(
                    Handle,
                    out PamojaMavlinkHeader header));
            return new MavlinkHeader(header.SystemId, header.ComponentId, header.Sequence);
        }
    }

    /// <summary>The id of the message the frame carries.</summary>
    public uint MessageId => NativeMethods.pamoja_mavlink_frame_message_id(Handle);

    /// <summary>The incompatibility flags a v2 frame declares.</summary>
    public byte IncompatFlags => NativeMethods.pamoja_mavlink_frame_incompat_flags(Handle);

    /// <summary>Whether the frame carries a signature.</summary>
    /// <remarks>
    /// This says only that the frame was signed, not that the signature is good;
    /// <see cref="MavlinkVerifier.Verify"/> decides that.
    /// </remarks>
    public bool Signed => NativeMethods.pamoja_mavlink_frame_is_signed(Handle) != 0;

    /// <summary>The message payload.</summary>
    /// <remarks>
    /// A v2 frame drops trailing zero bytes, so a payload can arrive shorter
    /// than the message's full length; a decoder zero-extends it.
    /// </remarks>
    public byte[] Payload => Copy(NativeMethods.pamoja_mavlink_frame_payload(Handle, out nuint len), len);

    /// <summary>The whole frame, ready to put on the wire.</summary>
    public byte[] Bytes => Copy(NativeMethods.pamoja_mavlink_frame_bytes(Handle, out nuint len), len);

    /// <summary>The signature block, or <c>null</c> when the frame is not signed.</summary>
    public byte[]? Signature
    {
        get
        {
            byte[] signature = new byte[Mavlink.SignatureLength];
            return NativeMethods.pamoja_mavlink_frame_signature(Handle, signature)
                == PamojaStatus.Ok
                ? signature
                : null;
        }
    }

    /// <summary>Copies bytes the native core owns into a managed array.</summary>
    /// <param name="pointer">The native pointer.</param>
    /// <param name="length">How many bytes to copy.</param>
    /// <returns>The copied bytes.</returns>
    private static byte[] Copy(IntPtr pointer, nuint length)
    {
        if (pointer == IntPtr.Zero)
        {
            return [];
        }

        byte[] copied = new byte[(int)length];
        Marshal.Copy(pointer, copied, 0, copied.Length);
        return copied;
    }

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>A streaming frame parser, and the frames it has completed.</summary>
public sealed class MavlinkParser : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a parser with an empty buffer.</summary>
    public MavlinkParser() =>
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_mavlink_parser_new(),
            NativeMethods.pamoja_mavlink_parser_free,
            "parser");

    /// <summary>Feeds bytes off a link and returns the frames that completed.</summary>
    /// <param name="bytes">The bytes just read off the link.</param>
    /// <param name="dialect">The dialect to prefer, or <c>null</c> for the common one.</param>
    /// <returns>The frames that completed, which may be none.</returns>
    /// <remarks>
    /// Whatever a serial port or socket delivers can be pushed as it arrives,
    /// however it is split. Noise between frames is skipped rather than
    /// reported, which is what lets a parser join a stream already in progress.
    /// </remarks>
    public IReadOnlyList<MavlinkFrame> Push(
        ReadOnlySpan<byte> bytes,
        MavlinkDialect? dialect = null)
    {
        IntPtr handle = _handle.DangerousGetHandle();
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_parser_push(
                handle,
                bytes,
                (nuint)bytes.Length,
                dialect?.Handle ?? IntPtr.Zero));

        List<MavlinkFrame> found = [];
        while (true)
        {
            PamojaCore.ThrowIfError(
                NativeMethods.pamoja_mavlink_parser_next(handle, out IntPtr frame));
            if (frame == IntPtr.Zero)
            {
                return found;
            }

            found.Add(new MavlinkFrame(frame));
        }
    }

    /// <summary>How many completed frames are waiting to be taken.</summary>
    public int Pending =>
        (int)NativeMethods.pamoja_mavlink_parser_pending(_handle.DangerousGetHandle());

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>A signing key and the monotonic timestamp that goes with it.</summary>
public sealed class MavlinkSigner : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a signer.</summary>
    /// <param name="key">The shared signing key, <see cref="Mavlink.KeyLength"/> bytes.</param>
    /// <param name="linkId">Which link this sender signs on.</param>
    /// <param name="timestamp">The timestamp to start from.</param>
    /// <exception cref="ArgumentException">The key is the wrong length.</exception>
    /// <remarks>
    /// The link id separates two links from one system, so traffic on one does
    /// not look like a replay of the other.
    /// </remarks>
    public MavlinkSigner(ReadOnlySpan<byte> key, byte linkId = 0, ulong timestamp = 0)
    {
        if (key.Length != Mavlink.KeyLength)
        {
            throw new ArgumentException(
                $"a signing key is {Mavlink.KeyLength} bytes",
                nameof(key));
        }

        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_signer_new(key, linkId, timestamp, out IntPtr signer));
        _handle = NativeHandle.Create(signer, NativeMethods.pamoja_mavlink_signer_free, "signer");
    }

    /// <summary>Signs a message into a v2 frame.</summary>
    /// <param name="header">The addressing fields to stamp on the frame.</param>
    /// <param name="msgid">The message id.</param>
    /// <param name="payload">The message payload.</param>
    /// <param name="crcExtra">The seed for this message id.</param>
    /// <returns>The signed frame.</returns>
    /// <exception cref="PamojaException">The payload does not fit a frame.</exception>
    /// <remarks>
    /// Each call advances the timestamp, which is what makes a replayed frame
    /// detectable.
    /// </remarks>
    public MavlinkFrame Sign(
        MavlinkHeader header,
        uint msgid,
        ReadOnlySpan<byte> payload,
        byte crcExtra)
    {
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_signer_sign(
                _handle.DangerousGetHandle(),
                header.ToNative(),
                msgid,
                payload,
                (nuint)payload.Length,
                crcExtra,
                out IntPtr frame));
        return new MavlinkFrame(frame);
    }

    /// <summary>Which link this signer signs on.</summary>
    public byte LinkId =>
        NativeMethods.pamoja_mavlink_signer_link_id(_handle.DangerousGetHandle());

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>A signing key and the timestamps it has already accepted.</summary>
public sealed class MavlinkVerifier : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates a verifier.</summary>
    /// <param name="key">The shared signing key, <see cref="Mavlink.KeyLength"/> bytes.</param>
    /// <exception cref="ArgumentException">The key is the wrong length.</exception>
    public MavlinkVerifier(ReadOnlySpan<byte> key)
    {
        if (key.Length != Mavlink.KeyLength)
        {
            throw new ArgumentException(
                $"a signing key is {Mavlink.KeyLength} bytes",
                nameof(key));
        }

        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_verifier_new(key, out IntPtr verifier));
        _handle = NativeHandle.Create(
            verifier,
            NativeMethods.pamoja_mavlink_verifier_free,
            "verifier");
    }

    /// <summary>Sets how far a timestamp may run ahead of the last one accepted.</summary>
    /// <param name="window">The window in timestamp units, ten microseconds each.</param>
    /// <remarks>
    /// A wider window tolerates a noisier link; a narrower one narrows the chance
    /// of a replay landing inside it.
    /// </remarks>
    public void SetWindow(ulong window) =>
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_verifier_set_window(
                _handle.DangerousGetHandle(),
                window));

    /// <summary>Checks a frame's signature and its place in the timestamp sequence.</summary>
    /// <param name="frame">The frame to check.</param>
    /// <exception cref="PamojaException">
    /// The frame is unsigned, the signature does not match the key, or the
    /// timestamp has been seen before.
    /// </exception>
    public void Verify(MavlinkFrame frame) =>
        PamojaCore.ThrowIfError(
            NativeMethods.pamoja_mavlink_verifier_verify(
                _handle.DangerousGetHandle(),
                frame.Handle));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
