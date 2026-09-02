using Pamoja.Core.Interop;

namespace Pamoja.Core;

/// <summary>Serial-line packet framing: SLIP and COBS.</summary>
/// <remarks>
/// A serial line is a stream of bytes with no packet boundaries, so something has
/// to mark where one message ends and the next begins. Each framing is offered
/// both as a one-shot call over a complete frame and, through
/// <see cref="SlipDecoder"/> and <see cref="CobsDecoder"/>, as a streaming decoder
/// for the arbitrary chunks a port delivers. The streaming decoder is what a real
/// read loop uses.
/// </remarks>
public static class Serial
{
    /// <summary>Frames a payload as a SLIP packet (RFC 1055).</summary>
    /// <param name="payload">The bytes to send.</param>
    /// <returns>The frame, delimiter included.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] SlipEncode(ReadOnlySpan<byte> payload)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_serial_slip_encode(
            payload, (nuint)payload.Length, out IntPtr buffer));
        return Codec.TakeBytes(buffer);
    }

    /// <summary>Reads the payload back out of a SLIP frame.</summary>
    /// <param name="frame">The frame as it arrived.</param>
    /// <returns>The payload.</returns>
    /// <exception cref="PamojaException">The frame is corrupt.</exception>
    public static byte[] SlipDecode(ReadOnlySpan<byte> frame)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_serial_slip_decode(
            frame, (nuint)frame.Length, out IntPtr buffer));
        return Codec.TakeBytes(buffer);
    }

    /// <summary>Frames a payload as a COBS packet, terminated by its zero delimiter.</summary>
    /// <param name="payload">The bytes to send.</param>
    /// <returns>The frame, delimiter included.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public static byte[] CobsEncode(ReadOnlySpan<byte> payload)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_serial_cobs_encode(
            payload, (nuint)payload.Length, out IntPtr buffer));
        return Codec.TakeBytes(buffer);
    }

    /// <summary>Reads the payload back out of a COBS frame.</summary>
    /// <param name="frame">The frame as it arrived.</param>
    /// <returns>The payload.</returns>
    /// <exception cref="PamojaException">The frame is corrupt.</exception>
    public static byte[] CobsDecode(ReadOnlySpan<byte> frame)
    {
        PamojaCore.ThrowIfError(NativeMethods.pamoja_serial_cobs_decode(
            frame, (nuint)frame.Length, out IntPtr buffer));
        return Codec.TakeBytes(buffer);
    }

    /// <summary>Returns the largest SLIP frame a payload of this length can produce.</summary>
    /// <param name="payloadLen">The payload length in bytes.</param>
    /// <returns>The worst-case frame length.</returns>
    public static int SlipMaxEncodedLen(int payloadLen) =>
        checked((int)NativeMethods.pamoja_serial_slip_max_encoded_len((nuint)payloadLen));

    /// <summary>Returns the largest COBS frame a payload of this length can produce.</summary>
    /// <param name="payloadLen">The payload length in bytes.</param>
    /// <returns>The worst-case frame length.</returns>
    public static int CobsMaxEncodedLen(int payloadLen) =>
        checked((int)NativeMethods.pamoja_serial_cobs_max_encoded_len((nuint)payloadLen));

    /// <summary>Copies every frame out of a native frame set and releases it.</summary>
    /// <param name="frames">The frame set handle a feed call produced.</param>
    /// <returns>The frames, in order.</returns>
    internal static byte[][] TakeFrames(IntPtr frames)
    {
        try
        {
            int count = checked((int)NativeMethods.pamoja_frames_count(frames));
            byte[][] collected = new byte[count][];
            for (int index = 0; index < count; index++)
            {
                int length = checked((int)NativeMethods.pamoja_frames_len(frames, (nuint)index));
                byte[] frame = new byte[length];
                if (length > 0)
                {
                    System.Runtime.InteropServices.Marshal.Copy(
                        NativeMethods.pamoja_frames_data(frames, (nuint)index), frame, 0, length);
                }

                collected[index] = frame;
            }

            return collected;
        }
        finally
        {
            NativeMethods.pamoja_frames_free(frames);
        }
    }
}

/// <summary>Reassembles whole SLIP frames from the chunks a serial port delivers.</summary>
/// <remarks>
/// A corrupt frame does not throw, because the frames around it are still good; it
/// is dropped and counted on <see cref="Discarded"/>.
/// </remarks>
/// <example>
/// <code>
/// using var decoder = new SlipDecoder();
/// foreach (byte[] frame in decoder.Feed(chunk)) Handle(frame);
/// </code>
/// </example>
public sealed class SlipDecoder : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an empty decoder, ready for the first chunk.</summary>
    /// <exception cref="PamojaException">The native decoder could not be created.</exception>
    public SlipDecoder()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_slip_decoder_new(),
            NativeMethods.pamoja_slip_decoder_free,
            "SLIP decoder");
    }

    /// <summary>How many corrupt frames this decoder has discarded.</summary>
    public ulong Discarded => _handle.Use(NativeMethods.pamoja_slip_decoder_discarded);

    /// <summary>Feeds a chunk of the stream.</summary>
    /// <param name="chunk">The bytes just read from the port.</param>
    /// <returns>Every frame this chunk completed, in order, which is often none.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[][] Feed(ReadOnlySpan<byte> chunk)
    {
        byte[] copy = chunk.ToArray();
        IntPtr frames = _handle.Use(handle =>
        {
            PamojaCore.ThrowIfError(NativeMethods.pamoja_slip_decoder_feed(
                handle, copy, (nuint)copy.Length, out IntPtr produced));
            return produced;
        });
        return Serial.TakeFrames(frames);
    }

    /// <summary>Discards any partly assembled frame.</summary>
    public void Reset() => _handle.Use(NativeMethods.pamoja_slip_decoder_reset);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}

/// <summary>Reassembles whole COBS frames from the chunks a serial port delivers.</summary>
/// <remarks>
/// The counterpart to <see cref="SlipDecoder"/>, for links where the framing
/// overhead has to stay small and predictable.
/// </remarks>
public sealed class CobsDecoder : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an empty decoder, ready for the first chunk.</summary>
    /// <exception cref="PamojaException">The native decoder could not be created.</exception>
    public CobsDecoder()
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_cobs_decoder_new(),
            NativeMethods.pamoja_cobs_decoder_free,
            "COBS decoder");
    }

    /// <summary>How many corrupt frames this decoder has discarded.</summary>
    public ulong Discarded => _handle.Use(NativeMethods.pamoja_cobs_decoder_discarded);

    /// <summary>Feeds a chunk of the stream.</summary>
    /// <param name="chunk">The bytes just read from the port.</param>
    /// <returns>Every frame this chunk completed, in order.</returns>
    /// <exception cref="PamojaException">The native call failed.</exception>
    public byte[][] Feed(ReadOnlySpan<byte> chunk)
    {
        byte[] copy = chunk.ToArray();
        IntPtr frames = _handle.Use(handle =>
        {
            PamojaCore.ThrowIfError(NativeMethods.pamoja_cobs_decoder_feed(
                handle, copy, (nuint)copy.Length, out IntPtr produced));
            return produced;
        });
        return Serial.TakeFrames(frames);
    }

    /// <summary>Discards any partly assembled frame.</summary>
    public void Reset() => _handle.Use(NativeMethods.pamoja_cobs_decoder_reset);

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
